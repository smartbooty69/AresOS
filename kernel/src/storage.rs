//! In-memory storage baseline for shell file operations.

use alloc::{collections::BTreeMap, vec::Vec};
use core::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

lazy_static! {
    static ref FS: Mutex<BTreeMap<&'static str, &'static str>> = {
        let mut files = BTreeMap::new();
        files.insert("/README.txt", "AresOS in-memory storage");
        files.insert("/bin/echo", "builtin: echo");
        files.insert("/bin/time", "builtin: time");
        files.insert("/bin/sysinfo", "builtin: sysinfo");
        Mutex::new(files)
    };
}

static STORAGE_MOUNTED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageError {
    NotMounted,
}

pub fn init() {
    STORAGE_MOUNTED.store(true, Ordering::Relaxed);
}

pub fn is_mounted() -> bool {
    STORAGE_MOUNTED.load(Ordering::Relaxed)
}

pub fn list_files() -> Result<Vec<&'static str>, StorageError> {
    if !is_mounted() {
        return Err(StorageError::NotMounted);
    }
    Ok(FS.lock().keys().copied().collect())
}

pub fn read_file(path: &str) -> Result<Option<&'static str>, StorageError> {
    if !is_mounted() {
        return Err(StorageError::NotMounted);
    }
    Ok(FS.lock().get(path).copied())
}

#[cfg(test)]
mod tests {
    #[test_case]
    fn filesystem_has_default_files() {
        super::init();
        let files = super::list_files().expect("storage should be mounted");
        assert!(files.iter().any(|f| *f == "/README.txt"));
    }

    #[test_case]
    fn unmounted_storage_rejects_access() {
        super::STORAGE_MOUNTED.store(false, core::sync::atomic::Ordering::Relaxed);
        assert_eq!(super::list_files(), Err(super::StorageError::NotMounted));
    }
}
