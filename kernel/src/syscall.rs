//! Minimal syscall surface for Phase 6 bring-up.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum SyscallId {
    GetTickCount = 1,
    GetProcessCount = 2,
    GetTotalPreemptions = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallError {
    InvalidSyscall,
    InvalidArgument,
}

pub fn invoke_raw(id: u64, arg0: u64) -> Result<u64, SyscallError> {
    match id {
        x if x == SyscallId::GetTickCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::performance::metrics::TICK_COUNTER.load(core::sync::atomic::Ordering::Relaxed))
        }
        x if x == SyscallId::GetProcessCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::process::process_count() as u64)
        }
        x if x == SyscallId::GetTotalPreemptions as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            let (_, _, preemptions, _) =
                crate::performance::process_metrics::ProcessMetricsGlobal::global_snapshot();
            Ok(preemptions)
        }
        _ => Err(SyscallError::InvalidSyscall),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn invalid_syscall_is_rejected() {
        let result = invoke_raw(999, 0);
        assert_eq!(result, Err(SyscallError::InvalidSyscall));
    }

    #[test_case]
    fn invalid_arguments_are_rejected() {
        let result = invoke_raw(SyscallId::GetTickCount as u64, 1);
        assert_eq!(result, Err(SyscallError::InvalidArgument));
    }
}
