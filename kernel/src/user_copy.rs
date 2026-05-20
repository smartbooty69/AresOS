//! Validated user pointer copies (Phase 26).

use core::sync::atomic::{AtomicU64, Ordering};

use crate::user_paging;

static COPY_SUCCESS: AtomicU64 = AtomicU64::new(0);
static COPY_REJECTED: AtomicU64 = AtomicU64::new(0);

pub const MAX_USER_COPY_LEN: usize = 64;

pub fn status() -> (u64, u64) {
    (
        COPY_SUCCESS.load(Ordering::Relaxed),
        COPY_REJECTED.load(Ordering::Relaxed),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserCopyError {
    NullPointer,
    KernelPointer,
    TooLarge,
    NotMapped,
    NotWritable,
}

pub fn copy_from_user(user_src: u64, dst: &mut [u8]) -> Result<usize, UserCopyError> {
    if user_src == 0 {
        COPY_REJECTED.fetch_add(1, Ordering::Relaxed);
        return Err(UserCopyError::NullPointer);
    }
    if dst.len() > MAX_USER_COPY_LEN {
        COPY_REJECTED.fetch_add(1, Ordering::Relaxed);
        return Err(UserCopyError::TooLarge);
    }
    if user_src >= 0xffff_8000_0000_0000 {
        COPY_REJECTED.fetch_add(1, Ordering::Relaxed);
        return Err(UserCopyError::KernelPointer);
    }
    for (idx, byte) in dst.iter_mut().enumerate() {
        let addr = user_src.saturating_add(idx as u64);
        let phys = user_paging::verify_active_translation(addr).ok_or_else(|| {
            COPY_REJECTED.fetch_add(1, Ordering::Relaxed);
            UserCopyError::NotMapped
        })?;
        let virt = user_paging::phys_to_virt(x86_64::PhysAddr::new(phys));
        *byte = unsafe { *(virt.as_ptr() as *const u8) };
    }
    COPY_SUCCESS.fetch_add(1, Ordering::Relaxed);
    Ok(dst.len())
}

pub fn copy_to_user(src: &[u8], user_dst: u64) -> Result<usize, UserCopyError> {
    if user_dst == 0 {
        COPY_REJECTED.fetch_add(1, Ordering::Relaxed);
        return Err(UserCopyError::NullPointer);
    }
    if src.len() > MAX_USER_COPY_LEN {
        COPY_REJECTED.fetch_add(1, Ordering::Relaxed);
        return Err(UserCopyError::TooLarge);
    }
    if user_dst >= 0xffff_8000_0000_0000 {
        COPY_REJECTED.fetch_add(1, Ordering::Relaxed);
        return Err(UserCopyError::KernelPointer);
    }
    for (idx, byte) in src.iter().enumerate() {
        let addr = user_dst.saturating_add(idx as u64);
        let _phys = user_paging::verify_active_translation(addr).ok_or_else(|| {
            COPY_REJECTED.fetch_add(1, Ordering::Relaxed);
            UserCopyError::NotMapped
        })?;
        let virt = user_paging::phys_to_virt(x86_64::PhysAddr::new(
            user_paging::verify_active_translation(addr).unwrap(),
        ));
        unsafe {
            *(virt.as_mut_ptr() as *mut u8) = *byte;
        }
    }
    COPY_SUCCESS.fetch_add(1, Ordering::Relaxed);
    Ok(src.len())
}

pub fn probe_round_trip(user_buf: u64) -> bool {
    let sample = b"ares-copyin-ok";
    copy_to_user(sample, user_buf).is_ok()
        && {
            let mut buf = [0u8; 14];
            copy_from_user(user_buf, &mut buf).is_ok() && &buf[..sample.len()] == sample
        }
}
