//! CPU syscall/sysret user entry (Phase 25+).

use core::sync::atomic::{AtomicU64, Ordering};

use x86_64::{
    registers::model_specific::{Efer, EferFlags, LStar, SFMask, Star},
    VirtAddr,
};

use crate::{
    gdt::UserSelectors,
    syscall::SyscallId,
    user_context::UserEntryFrame,
    user_entry::{self, UserEntryError},
    user_paging::HwPageTableHandle,
    user_syscall::{self, UserRegisterFrame, UserSyscallReturn},
};

static HW_SYSCALLS: AtomicU64 = AtomicU64::new(0);
static HW_SYSRETS: AtomicU64 = AtomicU64::new(0);
static HW_SYSCALL_READY: AtomicU64 = AtomicU64::new(0);
pub static HW_SYSCALL_ALLOWED: AtomicU64 = AtomicU64::new(0);
pub static HW_SYSCALL_REJECTED: AtomicU64 = AtomicU64::new(0);

pub const ALLOWED_HW_SYSCALLS: &[SyscallId] = &[
    SyscallId::GetTickCount,
    SyscallId::UserCopyProbe,
    SyscallId::ExitProcess,
    SyscallId::WaitProcess,
    SyscallId::ReadFileProbe,
    SyscallId::WriteFileProbe,
];

pub fn status() -> (u64, u64) {
    (
        HW_SYSCALLS.load(Ordering::Relaxed),
        HW_SYSRETS.load(Ordering::Relaxed),
    )
}

pub fn mark_dispatch_table_ready() {
    HW_SYSCALL_READY.store(1, Ordering::Relaxed);
}

pub fn dispatch_table_status() -> (u64, u64, bool) {
    (
        HW_SYSCALL_ALLOWED.load(Ordering::Relaxed),
        HW_SYSCALL_REJECTED.load(Ordering::Relaxed),
        HW_SYSCALL_READY.load(Ordering::Relaxed) != 0,
    )
}

pub fn record_hw_syscall_completed() {
    HW_SYSCALLS.fetch_add(1, Ordering::Relaxed);
    HW_SYSRETS.fetch_add(1, Ordering::Relaxed);
}

pub fn init_syscall_msrs() {
    if HW_SYSCALL_READY.load(Ordering::Relaxed) != 0 {
        return;
    }
    let syscall_entry = syscall_entry_trampoline as *const () as u64;
    unsafe {
        let user = crate::gdt::user_selectors();
        Star::write(
            user.code,
            user.data,
            crate::gdt::kernel_code_selector(),
            crate::gdt::kernel_data_selector(),
        )
        .expect("STAR write failed");
        LStar::write(VirtAddr::new(syscall_entry));
        SFMask::write(x86_64::registers::rflags::RFlags::INTERRUPT_FLAG);
        Efer::write(Efer::read() | EferFlags::SYSTEM_CALL_EXTENSIONS);
    }
    HW_SYSCALL_READY.store(1, Ordering::Relaxed);
}

pub fn is_allowed_hw_syscall(id: u64) -> bool {
    ALLOWED_HW_SYSCALLS
        .iter()
        .any(|syscall| *syscall as u64 == id)
}

pub fn run_hw_tick_syscall(
    hw: &HwPageTableHandle,
    entry: &UserEntryFrame,
    selectors: UserSelectors,
) -> Result<UserSyscallReturn, UserEntryError> {
    user_entry::write_user_stub_int80_syscall(hw, entry.rip, SyscallId::GetTickCount as u64)?;

    user_entry::set_hw_syscall_bringup_flag();

    let before = HW_SYSCALLS.load(Ordering::Relaxed);
    user_entry::enter_user_syscall_hw(hw, entry, selectors)?;
    let syscall_ok = HW_SYSCALLS.load(Ordering::Relaxed) > before;

    if !syscall_ok {
        return Err(UserEntryError::NoTrap);
    }

    user_syscall::last_hw_syscall_return().ok_or(UserEntryError::NoTrap)
}

extern "C" fn syscall_entry_trampoline() {
    let _ = crate::user_paging::restore_kernel_page_table();
    let (syscall_id, arg0) = unsafe { read_syscall_args() };
    if !is_allowed_hw_syscall(syscall_id) {
        HW_SYSCALL_REJECTED.fetch_add(1, Ordering::Relaxed);
        user_syscall::store_hw_syscall_return(UserSyscallReturn {
            syscall_id,
            arg0,
            return_value: 0,
            error: Some(crate::syscall::SyscallError::InvalidSyscall),
            returned_to_user: true,
        });
        HW_SYSCALLS.fetch_add(1, Ordering::Relaxed);
        let _ = crate::user_paging::activate_bringup_user_cr3();
        unsafe {
            core::arch::asm!("sysret", options(noreturn));
        }
    }
    HW_SYSCALL_ALLOWED.fetch_add(1, Ordering::Relaxed);
    let frame = UserRegisterFrame {
        syscall_id,
        arg0,
        return_value: 0,
        error: None,
    };
    let result = user_syscall::dispatch_from_user(frame).unwrap_or_else(|_| UserSyscallReturn {
        syscall_id,
        arg0,
        return_value: 0,
        error: Some(crate::syscall::SyscallError::InvalidArgument),
        returned_to_user: true,
    });
    user_syscall::store_hw_syscall_return(result);
    HW_SYSCALLS.fetch_add(1, Ordering::Relaxed);
    HW_SYSRETS.fetch_add(1, Ordering::Relaxed);
    let _ = crate::user_paging::activate_bringup_user_cr3();
    unsafe {
        core::arch::asm!("sysret", options(noreturn));
    }
}

unsafe fn read_syscall_args() -> (u64, u64) {
    let id: u64;
    let arg0: u64;
    core::arch::asm!(
        "mov {0}, rax",
        "mov {1}, rdi",
        out(reg) id,
        out(reg) arg0,
        options(nomem, nostack)
    );
    (id, arg0)
}
