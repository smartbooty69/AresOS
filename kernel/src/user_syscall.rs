//! Phase 19 user syscall entry/return ABI descriptors.

use crate::syscall::{self, SyscallError, SyscallId};

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

pub fn dispatch_from_user(
    mut frame: UserRegisterFrame,
) -> Result<UserSyscallReturn, UserSyscallAbiError> {
    validate_user_argument(frame.syscall_id, frame.arg0)?;
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
