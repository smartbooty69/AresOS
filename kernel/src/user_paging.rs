//! Hardware user page tables, CR3 activation, and per-process switching (Phases 21-22, 30).

use core::sync::atomic::{AtomicU64, Ordering};
use bootloader::bootinfo::MemoryMap;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        page_table::PageTableEntry, FrameAllocator, Mapper, OffsetPageTable, Page, PageTable,
        PageTableFlags, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

use crate::memory;

use crate::{
    frame_ownership::OwnedFrameToken,
    load_plan::LoadPermissions,
    user_memory::{InactiveUserPageTable, UserPageTableId},
};

static PHYS_MEM_OFFSET: AtomicU64 = AtomicU64::new(0);
static HW_BUILT: AtomicU64 = AtomicU64::new(0);
static HW_VERIFIED: AtomicU64 = AtomicU64::new(0);
static HW_REJECTED: AtomicU64 = AtomicU64::new(0);
static CR3_ACTIVATIONS: AtomicU64 = AtomicU64::new(0);
static CR3_RESTORES: AtomicU64 = AtomicU64::new(0);
static CR3_SWITCHES: AtomicU64 = AtomicU64::new(0);
static CR3_ISOLATION_CHECKS: AtomicU64 = AtomicU64::new(0);
static BRINGUP_USER_CR3: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HwPageTableHandle {
    pub inactive_id: UserPageTableId,
    pub cr3_phys: u64,
    pub pml4_token: OwnedFrameToken,
    pub mapped_pages: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UserPagingError {
    NotInitialized,
    EmptyTable,
    FrameUnavailable,
    MapFailed,
    VerifyFailed,
    AlreadyActive,
}

struct KernelCr3Backup {
    frame: PhysFrame<Size4KiB>,
    flags: x86_64::registers::control::Cr3Flags,
}

lazy_static! {
    static ref KERNEL_CR3: Mutex<Option<KernelCr3Backup>> = Mutex::new(None);
    static ref ACTIVE_USER_CR3: Mutex<Option<u64>> = Mutex::new(None);
    static ref PAGE_TABLE_FRAME_ALLOCATOR: Mutex<Option<crate::memory::BootInfoFrameAllocator>> =
        Mutex::new(None);
}

pub unsafe fn set_boot_frame_allocator(memory_map: &'static MemoryMap, skip_frames: usize) {
    *PAGE_TABLE_FRAME_ALLOCATOR.lock() = Some(crate::memory::BootInfoFrameAllocator::init_from_index(
        memory_map,
        skip_frames,
    ));
}

pub fn init(physical_memory_offset: VirtAddr) {
    PHYS_MEM_OFFSET.store(physical_memory_offset.as_u64(), Ordering::Relaxed);
}

pub fn phys_mem_offset() -> VirtAddr {
    VirtAddr::new(PHYS_MEM_OFFSET.load(Ordering::Relaxed))
}

pub fn phys_to_virt(phys: PhysAddr) -> VirtAddr {
    phys_mem_offset() + phys.as_u64()
}

pub fn status() -> (u64, u64, u64, u64, u64, u64, u64) {
    (
        HW_BUILT.load(Ordering::Relaxed),
        HW_VERIFIED.load(Ordering::Relaxed),
        HW_REJECTED.load(Ordering::Relaxed),
        CR3_ACTIVATIONS.load(Ordering::Relaxed),
        CR3_RESTORES.load(Ordering::Relaxed),
        CR3_SWITCHES.load(Ordering::Relaxed),
        CR3_ISOLATION_CHECKS.load(Ordering::Relaxed),
    )
}

pub fn write_phys_bytes(phys: u64, offset: usize, bytes: &[u8]) {
    let addr = phys.saturating_add(offset as u64);
    let virt = phys_to_virt(PhysAddr::new(addr));
    unsafe {
        core::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            virt.as_mut_ptr(),
            bytes.len(),
        );
    }
}

pub fn build_hw_page_table(
    inactive: &InactiveUserPageTable,
) -> Result<HwPageTableHandle, UserPagingError> {
    if PHYS_MEM_OFFSET.load(Ordering::Relaxed) == 0 {
        return Err(UserPagingError::NotInitialized);
    }
    if inactive.mapped_pages == 0 {
        return Err(UserPagingError::EmptyTable);
    }

    let mut frame_alloc = OwnershipFrameAllocator::default();
    let pml4_phys = frame_alloc
        .allocate_frame()
        .ok_or(UserPagingError::FrameUnavailable)?
        .start_address()
        .as_u64();
    zero_page_table(pml4_phys);
    copy_kernel_pml4_entries(pml4_phys)?;

    let mut mapper = unsafe { mapper_for_phys(pml4_phys) };
    for mapping in &inactive.mappings {
        let page: Page<Size4KiB> = Page::containing_address(VirtAddr::new(mapping.virtual_address));
        let frame = PhysFrame::from_start_address(PhysAddr::new(mapping.physical_address))
            .map_err(|_| UserPagingError::MapFailed)?;
        let flags = flags_for_permissions(mapping.permissions);
        unsafe {
            if mapper.translate_page(page).is_ok() {
                let (_frame, flush) = mapper.unmap(page).map_err(|_| UserPagingError::MapFailed)?;
                flush.flush();
            }
            mapper
                .map_to(page, frame, flags, &mut frame_alloc)
                .map_err(|_| UserPagingError::MapFailed)?
                .flush();
        }
    }

    for mapping in &inactive.mappings {
        let virt = VirtAddr::new(mapping.virtual_address);
        let hw = translate_hw(pml4_phys, virt).ok_or(UserPagingError::VerifyFailed)?;
        let desc = crate::user_memory::translate(inactive, mapping.virtual_address)
            .ok_or(UserPagingError::VerifyFailed)?;
        if hw.as_u64() != desc {
            return Err(UserPagingError::VerifyFailed);
        }
    }

    HW_BUILT.fetch_add(1, Ordering::Relaxed);
    HW_VERIFIED.fetch_add(1, Ordering::Relaxed);

    let mut mapped_pages = inactive.mapped_pages;
    map_default_user_stack(pml4_phys, &mut frame_alloc, &mut mapped_pages)?;

    Ok(HwPageTableHandle {
        inactive_id: inactive.id,
        cr3_phys: pml4_phys,
        pml4_token: OwnedFrameToken::from_raw(pml4_phys),
        mapped_pages,
    })
}

fn map_default_user_stack(
    pml4_phys: u64,
    frame_alloc: &mut OwnershipFrameAllocator,
    mapped_pages: &mut usize,
) -> Result<(), UserPagingError> {
    use crate::user_context::{DEFAULT_USER_STACK_SIZE, DEFAULT_USER_STACK_TOP};
    let mut mapper = unsafe { mapper_for_phys(pml4_phys) };
    let stack_bottom = DEFAULT_USER_STACK_TOP.saturating_sub(DEFAULT_USER_STACK_SIZE as u64);
    let mut addr = stack_bottom;
    while addr < DEFAULT_USER_STACK_TOP {
        let frame = frame_alloc
            .allocate_frame()
            .ok_or(UserPagingError::FrameUnavailable)?;
        let page: Page<Size4KiB> = Page::containing_address(VirtAddr::new(addr));
        let phys = frame;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;
        unsafe {
            mapper
                .map_to(page, phys, flags, frame_alloc)
                .map_err(|_| UserPagingError::MapFailed)?
                .flush();
        }
        addr = addr.saturating_add(4096);
        *mapped_pages += 1;
    }
    Ok(())
}

pub fn activate_user_page_table(handle: &HwPageTableHandle) -> Result<(), UserPagingError> {
    x86_64::instructions::interrupts::without_interrupts(|| activate_user_page_table_inner(handle))
}

pub fn restore_kernel_page_table() -> Result<(), UserPagingError> {
    x86_64::instructions::interrupts::without_interrupts(|| restore_kernel_page_table_inner())
}

/// Activate `handle`, run `f` with interrupts disabled, then restore kernel CR3.
pub fn with_user_page_table<R>(
    handle: &HwPageTableHandle,
    f: impl FnOnce() -> R,
) -> Result<R, UserPagingError> {
    x86_64::instructions::interrupts::without_interrupts(|| activate_user_page_table_inner(handle))?;
    let result = f();
    x86_64::instructions::interrupts::without_interrupts(|| restore_kernel_page_table_inner())?;
    Ok(result)
}

fn activate_user_page_table_inner(handle: &HwPageTableHandle) -> Result<(), UserPagingError> {
    backup_kernel_cr3()?;
    let frame = PhysFrame::from_start_address(PhysAddr::new(handle.cr3_phys))
        .map_err(|_| UserPagingError::MapFailed)?;
    unsafe {
        Cr3::write(frame, Cr3::read().1);
    }
    *ACTIVE_USER_CR3.lock() = Some(handle.cr3_phys);
    BRINGUP_USER_CR3.store(handle.cr3_phys, Ordering::Relaxed);
    CR3_ACTIVATIONS.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

pub fn activate_bringup_user_cr3() -> Result<(), UserPagingError> {
    let cr3 = BRINGUP_USER_CR3.load(Ordering::Relaxed);
    if cr3 == 0 {
        return Err(UserPagingError::NotInitialized);
    }
    activate_for_process(cr3)
}

fn restore_kernel_page_table_inner() -> Result<(), UserPagingError> {
    let Some(backup) = KERNEL_CR3.lock().take() else {
        *ACTIVE_USER_CR3.lock() = None;
        return Ok(());
    };
    unsafe {
        Cr3::write(backup.frame, backup.flags);
    }
    *ACTIVE_USER_CR3.lock() = None;
    CR3_RESTORES.fetch_add(1, Ordering::Relaxed);
    Ok(())
}

pub fn verify_active_translation(virtual_address: u64) -> Option<u64> {
    if (*ACTIVE_USER_CR3.lock()).is_none() {
        return None;
    }
    memory::translate_addr(VirtAddr::new(virtual_address), phys_mem_offset())
        .map(|addr| addr.as_u64())
}

pub fn activate_for_process(cr3_phys: u64) -> Result<(), UserPagingError> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        if KERNEL_CR3.lock().is_none() {
            backup_kernel_cr3()?;
        }
        let frame =
            PhysFrame::from_start_address(PhysAddr::new(cr3_phys)).map_err(|_| UserPagingError::MapFailed)?;
        unsafe {
            Cr3::write(frame, Cr3::read().1);
        }
        *ACTIVE_USER_CR3.lock() = Some(cr3_phys);
        CR3_SWITCHES.fetch_add(1, Ordering::Relaxed);
        Ok(())
    })
}

pub fn switch_between_user_tables(first: u64, second: u64) -> Result<bool, UserPagingError> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        activate_for_process(first)?;
        let first_trans = verify_active_translation(0x400000);
        activate_for_process(second)?;
        let second_trans = verify_active_translation(0x400000);
        restore_kernel_page_table_inner()?;
        CR3_ISOLATION_CHECKS.fetch_add(1, Ordering::Relaxed);
        Ok(
            first != second
                && first_trans.is_some()
                && second_trans.is_some(),
        )
    })
}

fn backup_kernel_cr3() -> Result<(), UserPagingError> {
    if KERNEL_CR3.lock().is_some() {
        return Ok(());
    }
    let (frame, flags) = Cr3::read();
    *KERNEL_CR3.lock() = Some(KernelCr3Backup { frame, flags });
    Ok(())
}

fn zero_page_table(phys: u64) {
    let virt = phys_to_virt(PhysAddr::new(phys));
    let table: &mut PageTable = unsafe { &mut *virt.as_mut_ptr() };
    for entry in table.iter_mut() {
        *entry = PageTableEntry::new();
    }
}

/// Share all present kernel PML4 entries so Ring 0 keeps working after CR3 switch.
fn copy_kernel_pml4_entries(pml4_phys: u64) -> Result<(), UserPagingError> {
    let offset = phys_mem_offset();
    let (kernel_frame, _) = Cr3::read();
    let kernel_virt = offset + kernel_frame.start_address().as_u64();
    let user_virt = offset + pml4_phys;
    let kernel_pml4: &PageTable = unsafe { &*(kernel_virt.as_ptr()) };
    let user_pml4: &mut PageTable = unsafe { &mut *(user_virt.as_mut_ptr()) };
    for index in 0..512 {
        if let Ok(frame) = kernel_pml4[index].frame() {
            user_pml4[index].set_frame(frame, kernel_pml4[index].flags());
        }
    }
    Ok(())
}

unsafe fn mapper_for_phys(pml4_phys: u64) -> OffsetPageTable<'static> {
    let virt = phys_to_virt(PhysAddr::new(pml4_phys));
    let table: &mut PageTable = &mut *virt.as_mut_ptr();
    OffsetPageTable::new(table, phys_mem_offset())
}

pub fn translate_hw_page(pml4_phys: u64, virtual_address: u64) -> Option<u64> {
    translate_hw(pml4_phys, VirtAddr::new(virtual_address)).map(|a| a.as_u64())
}

fn translate_hw(pml4_phys: u64, addr: VirtAddr) -> Option<PhysAddr> {
    let offset = phys_mem_offset();
    let frame = PhysFrame::from_start_address(PhysAddr::new(pml4_phys)).ok()?;
    let table_indexes = [addr.p4_index(), addr.p3_index(), addr.p2_index(), addr.p1_index()];
    let mut current = frame;
    for &index in &table_indexes {
        let virt = offset + current.start_address().as_u64();
        let table: &PageTable = unsafe { &*(virt.as_ptr()) };
        let entry = &table[index];
        current = entry.frame().ok()?;
    }
    Some(current.start_address() + u64::from(addr.page_offset()))
}

fn flags_for_permissions(permissions: LoadPermissions) -> PageTableFlags {
    let mut flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
    if permissions.writable() {
        flags |= PageTableFlags::WRITABLE;
    }
    if !permissions.executable() {
        flags |= PageTableFlags::NO_EXECUTE;
    }
    flags
}

#[derive(Default)]
struct OwnershipFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for OwnershipFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        PAGE_TABLE_FRAME_ALLOCATOR
            .lock()
            .as_mut()
            .and_then(|allocator| allocator.allocate_frame())
    }
}
