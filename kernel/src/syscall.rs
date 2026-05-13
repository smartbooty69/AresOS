//! Minimal syscall surface for Phase 6+ bring-up.

use alloc::{string::String, vec::Vec};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum SyscallId {
    GetTickCount = 1,
    GetProcessCount = 2,
    GetTotalPreemptions = 3,
    StorageMounted = 4,
    StorageFileCount = 5,
    StorageFormat = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallError {
    InvalidSyscall,
    InvalidArgument,
    Storage,
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
        x if x == SyscallId::StorageMounted as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::storage::is_mounted() as u64)
        }
        x if x == SyscallId::StorageFileCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            crate::storage::info()
                .map(|info| info.file_count as u64)
                .map_err(|_| SyscallError::Storage)
        }
        x if x == SyscallId::StorageFormat as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            crate::storage::format()
                .map(|_| 0)
                .map_err(|_| SyscallError::Storage)
        }
        _ => Err(SyscallError::InvalidSyscall),
    }
}

pub fn storage_list_files() -> Result<Vec<String>, SyscallError> {
    crate::storage::list_files().map_err(|_| SyscallError::Storage)
}

pub fn storage_read_file(path: &str) -> Result<Option<String>, SyscallError> {
    crate::storage::read_file(path).map_err(|_| SyscallError::Storage)
}

pub fn storage_write_file(path: &str, contents: &str) -> Result<(), SyscallError> {
    crate::storage::write_file(path, contents).map_err(|_| SyscallError::Storage)
}

pub fn storage_delete_file(path: &str) -> Result<(), SyscallError> {
    crate::storage::delete_file(path).map_err(|_| SyscallError::Storage)
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

    #[test_case]
    fn storage_syscalls_report_status() {
        crate::storage::init();
        assert_eq!(invoke_raw(SyscallId::StorageMounted as u64, 0), Ok(1));
        assert!(invoke_raw(SyscallId::StorageFileCount as u64, 0).unwrap() > 0);
    }

    #[test_case]
    fn storage_wrappers_validate_paths() {
        crate::storage::init();
        assert_eq!(
            storage_write_file("relative-path", "bad"),
            Err(SyscallError::Storage)
        );
    }
}
