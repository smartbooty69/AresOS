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
    DeviceCount = 7,
    BlockDeviceCount = 8,
    ProgramCount = 9,
    ProgramLaunchCount = 10,
    ProgramFailedLaunchCount = 11,
    CurrentUser = 12,
    CurrentRole = 13,
    DeniedAccessCount = 14,
    DeniedExecuteCount = 15,
    ImageCount = 16,
    ValidImageCount = 17,
    InvalidImageCount = 18,
    UnsupportedExecutionCount = 19,
    PreparedImageCount = 20,
    RejectedLoadPlanCount = 21,
    TotalPlannedPages = 22,
    ExecutionBlockedCount = 23,
    MappedImageCount = 24,
    RejectedMappingCount = 25,
    TotalMappedPages = 26,
    MappedCopiedBytes = 27,
    MappedZeroFilledBytes = 28,
    FrameTrackedCount = 29,
    FrameAvailableCount = 30,
    FrameAllocatedCount = 31,
    FrameAllocationCount = 32,
    FrameReleaseCount = 33,
    FrameFailedAllocationCount = 34,
    FrameBackedImageCount = 35,
    RejectedFrameBackingCount = 36,
    TotalFrameBackedPages = 37,
    UserPageTableCount = 38,
    RejectedUserPageTableCount = 39,
    TotalUserPageTablePages = 40,
    UserContextCount = 41,
    RejectedUserContextCount = 42,
    Ring3EntryCount = 43,
    Ring3TrapCount = 44,
    RejectedRing3Count = 45,
    UserSyscallCount = 46,
    UserSyscallReturnCount = 47,
    RejectedUserSyscallCount = 48,
    UserElfExecutionCount = 49,
    UserElfExitCount = 50,
    RejectedUserElfCount = 51,
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
        x if x == SyscallId::DeviceCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::device::summary().total as u64)
        }
        x if x == SyscallId::BlockDeviceCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::block::list_block_devices().len() as u64)
        }
        x if x == SyscallId::ProgramCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().program_count as u64)
        }
        x if x == SyscallId::ProgramLaunchCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().launch_count)
        }
        x if x == SyscallId::ProgramFailedLaunchCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().failed_launch_count)
        }
        x if x == SyscallId::CurrentUser as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::security::current_credentials().user.as_u64())
        }
        x if x == SyscallId::CurrentRole as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::security::current_credentials().role.as_u64())
        }
        x if x == SyscallId::DeniedAccessCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::security::denied_access_count())
        }
        x if x == SyscallId::DeniedExecuteCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::security::denied_execute_count())
        }
        x if x == SyscallId::ImageCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().image_count as u64)
        }
        x if x == SyscallId::ValidImageCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().valid_image_count as u64)
        }
        x if x == SyscallId::InvalidImageCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().invalid_image_count as u64)
        }
        x if x == SyscallId::UnsupportedExecutionCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().unsupported_execution_count)
        }
        x if x == SyscallId::PreparedImageCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().prepared_image_count)
        }
        x if x == SyscallId::RejectedLoadPlanCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().rejected_load_plan_count)
        }
        x if x == SyscallId::TotalPlannedPages as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().total_planned_pages)
        }
        x if x == SyscallId::ExecutionBlockedCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().unsupported_execution_count)
        }
        x if x == SyscallId::MappedImageCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().mapped_image_count)
        }
        x if x == SyscallId::RejectedMappingCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().rejected_mapping_count)
        }
        x if x == SyscallId::TotalMappedPages as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().total_mapped_pages)
        }
        x if x == SyscallId::MappedCopiedBytes as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().copied_bytes)
        }
        x if x == SyscallId::MappedZeroFilledBytes as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().zero_filled_bytes)
        }
        x if x == SyscallId::FrameTrackedCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::frame_ownership::status().tracked_frames as u64)
        }
        x if x == SyscallId::FrameAvailableCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::frame_ownership::status().available_frames as u64)
        }
        x if x == SyscallId::FrameAllocatedCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::frame_ownership::status().allocated_frames as u64)
        }
        x if x == SyscallId::FrameAllocationCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::frame_ownership::status().allocation_count)
        }
        x if x == SyscallId::FrameReleaseCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::frame_ownership::status().release_count)
        }
        x if x == SyscallId::FrameFailedAllocationCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::frame_ownership::status().failed_allocation_count)
        }
        x if x == SyscallId::FrameBackedImageCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().frame_backed_image_count)
        }
        x if x == SyscallId::RejectedFrameBackingCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().rejected_frame_backing_count)
        }
        x if x == SyscallId::TotalFrameBackedPages as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().total_frame_backed_pages)
        }
        x if x == SyscallId::UserPageTableCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().user_page_table_count)
        }
        x if x == SyscallId::RejectedUserPageTableCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().rejected_user_page_table_count)
        }
        x if x == SyscallId::TotalUserPageTablePages as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().total_user_page_table_pages)
        }
        x if x == SyscallId::UserContextCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().user_context_count)
        }
        x if x == SyscallId::RejectedUserContextCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().rejected_user_context_count)
        }
        x if x == SyscallId::Ring3EntryCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().ring3_entry_count)
        }
        x if x == SyscallId::Ring3TrapCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().ring3_trap_count)
        }
        x if x == SyscallId::RejectedRing3Count as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().rejected_ring3_count)
        }
        x if x == SyscallId::UserSyscallCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().user_syscall_count)
        }
        x if x == SyscallId::UserSyscallReturnCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().user_syscall_return_count)
        }
        x if x == SyscallId::RejectedUserSyscallCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().rejected_user_syscall_count)
        }
        x if x == SyscallId::UserElfExecutionCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().user_elf_execution_count)
        }
        x if x == SyscallId::UserElfExitCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().user_elf_exit_count)
        }
        x if x == SyscallId::RejectedUserElfCount as u64 => {
            if arg0 != 0 {
                return Err(SyscallError::InvalidArgument);
            }
            Ok(crate::task::program_loader::status().rejected_user_elf_count)
        }
        _ => Err(SyscallError::InvalidSyscall),
    }
}

pub fn storage_list_files() -> Result<Vec<String>, SyscallError> {
    crate::storage::list_files().map_err(|_| SyscallError::Storage)
}

pub fn storage_read_file(path: &str) -> Result<Option<String>, SyscallError> {
    crate::storage::read_file_checked(crate::security::current_credentials(), path)
        .map_err(|_| SyscallError::Storage)
}

pub fn storage_write_file(path: &str, contents: &str) -> Result<(), SyscallError> {
    crate::storage::write_file_checked(crate::security::current_credentials(), path, contents)
        .map_err(|_| SyscallError::Storage)
}

pub fn storage_delete_file(path: &str) -> Result<(), SyscallError> {
    crate::storage::delete_file_checked(crate::security::current_credentials(), path)
        .map_err(|_| SyscallError::Storage)
}

pub fn device_summary() -> crate::device::DeviceSummary {
    crate::device::summary()
}

pub fn block_devices() -> Vec<crate::block::BlockDeviceInfo> {
    crate::block::list_block_devices()
}

pub fn loader_status() -> crate::task::program_loader::LoaderStatus {
    crate::task::program_loader::status()
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
        assert!(invoke_raw(SyscallId::DeviceCount as u64, 0).unwrap() > 0);
        assert!(invoke_raw(SyscallId::BlockDeviceCount as u64, 0).unwrap() > 0);
        assert!(invoke_raw(SyscallId::ProgramCount as u64, 0).unwrap() > 0);
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
