//! Phase 19 user syscall entry/return ABI descriptors.

use core::sync::atomic::{AtomicU64, Ordering};

use crate::syscall::{self, SyscallError, SyscallId};

static LAST_HW_RETURN: AtomicU64 = AtomicU64::new(0);
static LAST_HW_ERROR: AtomicU64 = AtomicU64::new(0);
static LAST_HW_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserRegisterFrame {
    pub syscall_id: u64,
    pub arg0: u64,
    pub return_value: u64,
    pub error: Option<SyscallError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserSyscallReturn {
    pub syscall_id: u64,
    pub arg0: u64,
    pub return_value: u64,
    pub error: Option<SyscallError>,
    pub returned_to_user: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserSyscallAbiError {
    UserPointerRejected,
}

pub fn store_hw_syscall_return(ret: UserSyscallReturn) {
    LAST_HW_ID.store(ret.syscall_id, Ordering::Relaxed);
    LAST_HW_RETURN.store(ret.return_value, Ordering::Relaxed);
    LAST_HW_ERROR.store(ret.error.map(|e| e as u64).unwrap_or(0), Ordering::Relaxed);
}

pub fn last_hw_syscall_return() -> Option<UserSyscallReturn> {
    if LAST_HW_ID.load(Ordering::Relaxed) == 0 {
        return None;
    }
    let error_code = LAST_HW_ERROR.load(Ordering::Relaxed);
    Some(UserSyscallReturn {
        syscall_id: LAST_HW_ID.load(Ordering::Relaxed),
        arg0: 0,
        return_value: LAST_HW_RETURN.load(Ordering::Relaxed),
        error: if error_code == 0 {
            None
        } else {
            Some(SyscallError::InvalidArgument)
        },
        returned_to_user: true,
    })
}

pub fn dispatch_from_user(
    mut frame: UserRegisterFrame,
) -> Result<UserSyscallReturn, UserSyscallAbiError> {
    validate_user_argument(frame.syscall_id, frame.arg0)?;
    if frame.syscall_id == SyscallId::UserCopyProbe as u64 {
        return dispatch_copy_probe(frame);
    }
    match syscall::invoke_raw(frame.syscall_id, frame.arg0) {
        Ok(value) => {
            frame.return_value = value;
            frame.error = None;
        }
        Err(err) => {
            frame.return_value = 0;
            frame.error = Some(err);
        }
    }
    Ok(UserSyscallReturn {
        syscall_id: frame.syscall_id,
        arg0: frame.arg0,
        return_value: frame.return_value,
        error: frame.error,
        returned_to_user: true,
    })
}

pub fn tick_probe_frame() -> UserRegisterFrame {
    UserRegisterFrame {
        syscall_id: SyscallId::GetTickCount as u64,
        arg0: 0,
        return_value: 0,
        error: None,
    }
}

fn dispatch_copy_probe(frame: UserRegisterFrame) -> Result<UserSyscallReturn, UserSyscallAbiError> {
    let ok = crate::user_copy::probe_round_trip(frame.arg0);
    Ok(UserSyscallReturn {
        syscall_id: frame.syscall_id,
        arg0: frame.arg0,
        return_value: if ok { 1 } else { 0 },
        error: None,
        returned_to_user: true,
    })
}

fn validate_user_argument(syscall_id: u64, arg0: u64) -> Result<(), UserSyscallAbiError> {
    if syscall_id == 0 && arg0 == 0 {
        return Err(UserSyscallAbiError::UserPointerRejected);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn tick_probe_uses_zero_arg() {
        let frame = tick_probe_frame();
        assert_eq!(frame.syscall_id, SyscallId::GetTickCount as u64);
        assert_eq!(frame.arg0, 0);
    }
}
