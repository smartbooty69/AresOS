//! Kernel heap allocator.
//!
//! Maps `HEAP_SIZE` bytes of virtual address space and installs a
//! `linked_list_allocator::LockedHeap` as the global allocator.

use linked_list_allocator::LockedHeap;
use x86_64::{
    structures::paging::{
        mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
    },
    VirtAddr,
};

/// Virtual address at which the kernel heap begins.
pub const HEAP_START: usize = 0x_4444_4444_0000;
/// Size of the kernel heap (1 MiB).
pub const HEAP_SIZE: usize = 1024 * 1024;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Map the heap pages and initialise the allocator.
pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end   = heap_start + HEAP_SIZE as u64 - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page   = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
    }

    unsafe {
        ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE);
    }

    Ok(())
}

// ─────────────────────────────────── tests ───────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{boxed::Box, vec, vec::Vec};
    use core::mem;

    #[test_case]
    fn test_simple_allocation() {
        let heap_value_1 = Box::new(41);
        let heap_value_2 = Box::new(13);
        assert_eq!(*heap_value_1, 41);
        assert_eq!(*heap_value_2, 13);
    }

    #[test_case]
    fn test_large_vec() {
        let n = 1000;
        let mut vec = Vec::new();
        for i in 0..n {
            vec.push(i);
        }
        assert_eq!(vec.iter().sum::<u64>(), (n - 1) * n / 2);
    }

    #[test_case]
    fn test_many_boxes() {
        for i in 0..super::HEAP_SIZE {
            let x = Box::new(i);
            assert_eq!(*x, i);
        }
    }

    #[test_case]
    fn test_many_boxes_long_lived() {
        let long_lived = Box::new(1);
        for i in 0..super::HEAP_SIZE {
            let x = Box::new(i);
            assert_eq!(*x, i);
        }
        assert_eq!(*long_lived, 1);
    }

    #[test_case]
    fn test_box_size() {
        let b = Box::new(0u64);
        assert_eq!(mem::size_of_val(&*b), mem::size_of::<u64>());
    }
}
