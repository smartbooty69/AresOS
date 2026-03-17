//! CPU context primitives for preemptive task switching groundwork.

use alloc::{boxed::Box, vec};
use core::arch::asm;

/// Default per-task kernel stack size for context-switched tasks.
pub const DEFAULT_KERNEL_STACK_SIZE: usize = 16 * 1024;

/// Saved CPU register state for a schedulable execution context.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CpuContext {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub rip: u64,
    pub rflags: u64,
}

impl CpuContext {
    pub const fn empty() -> Self {
        Self {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbx: 0,
            rbp: 0,
            rsp: 0,
            rip: 0,
            rflags: 0,
        }
    }

    /// Capture a snapshot of the current CPU context.
    pub fn capture() -> Self {
        let mut context = Self::empty();

        unsafe {
            asm!(
                "mov {r15}, r15",
                "mov {r14}, r14",
                "mov {r13}, r13",
                "mov {r12}, r12",
                "mov {rbx}, rbx",
                "mov {rbp}, rbp",
                "mov {rsp}, rsp",
                "lea {rip}, [rip]",
                "pushfq",
                "pop {rflags}",
                r15 = out(reg) context.r15,
                r14 = out(reg) context.r14,
                r13 = out(reg) context.r13,
                r12 = out(reg) context.r12,
                rbx = out(reg) context.rbx,
                rbp = out(reg) context.rbp,
                rsp = out(reg) context.rsp,
                rip = out(reg) context.rip,
                rflags = out(reg) context.rflags,
                options(nostack, preserves_flags)
            );
        }

        context
    }
}

/// A kernel stack owned by a schedulable context.
pub struct KernelStack {
    bytes: Box<[u8]>,
}

impl KernelStack {
    pub fn new(size: usize) -> Self {
        assert!(size >= 1024, "kernel stack too small");
        Self {
            bytes: vec![0u8; size].into_boxed_slice(),
        }
    }

    pub fn top(&self) -> u64 {
        (self.bytes.as_ptr() as u64).saturating_add(self.bytes.len() as u64)
    }

    pub fn size(&self) -> usize {
        self.bytes.len()
    }
}

impl Default for KernelStack {
    fn default() -> Self {
        Self::new(DEFAULT_KERNEL_STACK_SIZE)
    }
}

/// A bootstrapped runnable context with its own kernel stack.
pub struct RunnableContext {
    pub context: CpuContext,
    pub stack: KernelStack,
}

impl RunnableContext {
    pub fn new(entry: extern "C" fn() -> !) -> Self {
        let stack = KernelStack::default();
        let mut context = CpuContext::empty();
        context.rsp = stack.top();
        context.rip = entry as usize as u64;
        context.rflags = 0x202;
        Self { context, stack }
    }
}

/// Save current registers and restore `next`, then continue at `next.rip`.
///
/// This function returns when another context switches back into `current`.
pub unsafe fn switch_context(current: &mut CpuContext, next: &CpuContext) {
    asm!(
        "mov [rdi + 0x00], r15",
        "mov [rdi + 0x08], r14",
        "mov [rdi + 0x10], r13",
        "mov [rdi + 0x18], r12",
        "mov [rdi + 0x20], rbx",
        "mov [rdi + 0x28], rbp",
        "mov [rdi + 0x30], rsp",
        "lea rax, [rip + 2f]",
        "mov [rdi + 0x38], rax",
        "pushfq",
        "pop rax",
        "or rax, 0x200",
        "mov [rdi + 0x40], rax",
        "mov r15, [rsi + 0x00]",
        "mov r14, [rsi + 0x08]",
        "mov r13, [rsi + 0x10]",
        "mov r12, [rsi + 0x18]",
        "mov rbx, [rsi + 0x20]",
        "mov rbp, [rsi + 0x28]",
        "mov rsp, [rsi + 0x30]",
        "mov rax, [rsi + 0x40]",
        "push rax",
        "popfq",
        "mov rax, [rsi + 0x38]",
        "push rax",
        "ret",
        "2:",
        in("rdi") current as *mut CpuContext,
        in("rsi") next as *const CpuContext,
        out("rax") _,
        options(preserves_flags)
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn cpu_context_empty_is_zeroed() {
        let context = CpuContext::empty();
        assert_eq!(context.r15, 0);
        assert_eq!(context.r14, 0);
        assert_eq!(context.r13, 0);
        assert_eq!(context.r12, 0);
        assert_eq!(context.rbx, 0);
        assert_eq!(context.rbp, 0);
        assert_eq!(context.rsp, 0);
        assert_eq!(context.rip, 0);
        assert_eq!(context.rflags, 0);
    }

    #[test_case]
    fn cpu_context_capture_sets_rsp_and_rflags() {
        let context = CpuContext::capture();
        assert!(context.rsp != 0);
        assert!(context.rflags != 0);
    }
}
