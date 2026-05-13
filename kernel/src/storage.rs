//! Phase 7 storage stack with a block-device boundary and a tiny filesystem.

use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;

pub const SECTOR_SIZE: usize = 512;
pub const DEFAULT_SECTOR_COUNT: usize = 64;
const MAGIC: &[u8; 8] = b"ARESFS1\0";
const VERSION: u32 = 1;
const HEADER_SECTOR: usize = 0;
const DIRECTORY_START_SECTOR: usize = 1;
const DIRECTORY_SECTORS: usize = 2;
const DATA_START_SECTOR: usize = DIRECTORY_START_SECTOR + DIRECTORY_SECTORS;
const MAX_FILES: usize = 16;
const MAX_PATH_LEN: usize = 48;
const MAX_FILE_SIZE: usize = SECTOR_SIZE;
const DIR_ENTRY_SIZE: usize = 64;

lazy_static! {
    static ref STORAGE: Mutex<SimpleFs<MemoryBlockDevice>> =
        Mutex::new(SimpleFs::new(MemoryBlockDevice::new(DEFAULT_SECTOR_COUNT)));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageError {
    NotMounted,
    InvalidPath,
    NotFound,
    AlreadyExists,
    NoSpace,
    FileTooLarge,
    InvalidImage,
    Io,
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::NotMounted => write!(f, "storage not mounted"),
            StorageError::InvalidPath => write!(f, "invalid path"),
            StorageError::NotFound => write!(f, "file not found"),
            StorageError::AlreadyExists => write!(f, "file already exists"),
            StorageError::NoSpace => write!(f, "no storage space available"),
            StorageError::FileTooLarge => write!(f, "file too large"),
            StorageError::InvalidImage => write!(f, "invalid filesystem image"),
            StorageError::Io => write!(f, "storage I/O error"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StorageInfo {
    pub mounted: bool,
    pub file_count: usize,
    pub max_files: usize,
    pub free_slots: usize,
    pub capacity_bytes: usize,
    pub max_file_size: usize,
}

pub trait BlockDevice {
    fn sector_count(&self) -> usize;
    fn read_sector(
        &self,
        sector: usize,
        buffer: &mut [u8; SECTOR_SIZE],
    ) -> Result<(), StorageError>;
    fn write_sector(
        &mut self,
        sector: usize,
        buffer: &[u8; SECTOR_SIZE],
    ) -> Result<(), StorageError>;
}

#[derive(Clone)]
pub struct MemoryBlockDevice {
    sectors: Vec<[u8; SECTOR_SIZE]>,
}

impl MemoryBlockDevice {
    pub fn new(sector_count: usize) -> Self {
        Self {
            sectors: vec![[0; SECTOR_SIZE]; sector_count],
        }
    }
}

impl BlockDevice for MemoryBlockDevice {
    fn sector_count(&self) -> usize {
        self.sectors.len()
    }

    fn read_sector(
        &self,
        sector: usize,
        buffer: &mut [u8; SECTOR_SIZE],
    ) -> Result<(), StorageError> {
        let source = self.sectors.get(sector).ok_or(StorageError::Io)?;
        buffer.copy_from_slice(source);
        Ok(())
    }

    fn write_sector(
        &mut self,
        sector: usize,
        buffer: &[u8; SECTOR_SIZE],
    ) -> Result<(), StorageError> {
        let target = self.sectors.get_mut(sector).ok_or(StorageError::Io)?;
        target.copy_from_slice(buffer);
        Ok(())
    }
}

#[derive(Clone)]
struct DirectoryEntry {
    path: String,
    len: usize,
    data_sector: usize,
}

pub struct SimpleFs<D: BlockDevice> {
    device: D,
    mounted: bool,
}

impl<D: BlockDevice> SimpleFs<D> {
    pub fn new(device: D) -> Self {
        Self {
            device,
            mounted: false,
        }
    }

    pub fn format(&mut self) -> Result<(), StorageError> {
        for sector in 0..self.device.sector_count() {
            self.device.write_sector(sector, &[0; SECTOR_SIZE])?;
        }
        self.write_header(0)?;
        self.mounted = true;
        Ok(())
    }

    pub fn mount(&mut self) -> Result<(), StorageError> {
        let count = self.read_header()?;
        if count > MAX_FILES {
            return Err(StorageError::InvalidImage);
        }
        self.mounted = true;
        Ok(())
    }

    pub fn unmount(&mut self) {
        self.mounted = false;
    }

    pub fn is_mounted(&self) -> bool {
        self.mounted
    }

    pub fn list_files(&self) -> Result<Vec<String>, StorageError> {
        Ok(self
            .read_directory()?
            .into_iter()
            .filter_map(|entry| entry.map(|entry| entry.path))
            .collect())
    }

    pub fn read_file(&self, path: &str) -> Result<Option<String>, StorageError> {
        validate_path(path)?;
        let entries = self.read_directory()?;
        let Some(entry) = entries
            .into_iter()
            .flatten()
            .find(|entry| entry.path == path)
        else {
            return Ok(None);
        };

        let mut sector = [0; SECTOR_SIZE];
        self.device.read_sector(entry.data_sector, &mut sector)?;
        let bytes = &sector[..entry.len];
        core::str::from_utf8(bytes)
            .map(|text| Some(text.to_string()))
            .map_err(|_| StorageError::InvalidImage)
    }

    pub fn create_file(&mut self, path: &str) -> Result<(), StorageError> {
        self.write_file_internal(path, "", false)
    }

    pub fn write_file(&mut self, path: &str, contents: &str) -> Result<(), StorageError> {
        self.write_file_internal(path, contents, true)
    }

    pub fn delete_file(&mut self, path: &str) -> Result<(), StorageError> {
        validate_path(path)?;
        let mut entries = self.read_directory()?;
        let Some(index) = entries
            .iter()
            .position(|entry| matches!(entry, Some(entry) if entry.path == path))
        else {
            return Err(StorageError::NotFound);
        };

        if let Some(entry) = entries[index].take() {
            self.device.write_sector(entry.data_sector, &[0; SECTOR_SIZE])?;
        }
        self.write_directory(&entries)
    }

    pub fn info(&self) -> Result<StorageInfo, StorageError> {
        if !self.mounted {
            return Ok(StorageInfo {
                mounted: false,
                file_count: 0,
                max_files: MAX_FILES,
                free_slots: MAX_FILES,
                capacity_bytes: MAX_FILES * MAX_FILE_SIZE,
                max_file_size: MAX_FILE_SIZE,
            });
        }
        let count = self.read_directory()?.iter().filter(|entry| entry.is_some()).count();
        Ok(StorageInfo {
            mounted: true,
            file_count: count,
            max_files: MAX_FILES,
            free_slots: MAX_FILES - count,
            capacity_bytes: MAX_FILES * MAX_FILE_SIZE,
            max_file_size: MAX_FILE_SIZE,
        })
    }

    fn write_file_internal(
        &mut self,
        path: &str,
        contents: &str,
        overwrite: bool,
    ) -> Result<(), StorageError> {
        validate_path(path)?;
        let bytes = contents.as_bytes();
        if bytes.len() > MAX_FILE_SIZE {
            return Err(StorageError::FileTooLarge);
        }

        let mut entries = self.read_directory()?;
        if let Some(index) = entries
            .iter()
            .position(|entry| matches!(entry, Some(entry) if entry.path == path))
        {
            if !overwrite {
                return Err(StorageError::AlreadyExists);
            }
            let data_sector = entries[index].as_ref().ok_or(StorageError::InvalidImage)?.data_sector;
            self.write_data_sector(data_sector, bytes)?;
            entries[index] = Some(DirectoryEntry {
                path: path.to_string(),
                len: bytes.len(),
                data_sector,
            });
            return self.write_directory(&entries);
        }

        let Some(index) = entries.iter().position(|entry| entry.is_none()) else {
            return Err(StorageError::NoSpace);
        };
        let data_sector = DATA_START_SECTOR + index;
        if data_sector >= self.device.sector_count() {
            return Err(StorageError::NoSpace);
        }

        self.write_data_sector(data_sector, bytes)?;
        entries[index] = Some(DirectoryEntry {
            path: path.to_string(),
            len: bytes.len(),
            data_sector,
        });
        self.write_directory(&entries)
    }

    fn write_header(&mut self, file_count: usize) -> Result<(), StorageError> {
        let mut sector = [0; SECTOR_SIZE];
        sector[..MAGIC.len()].copy_from_slice(MAGIC);
        sector[8..12].copy_from_slice(&VERSION.to_le_bytes());
        sector[12..16].copy_from_slice(&(file_count as u32).to_le_bytes());
        self.device.write_sector(HEADER_SECTOR, &sector)
    }

    fn read_header(&self) -> Result<usize, StorageError> {
        let mut sector = [0; SECTOR_SIZE];
        self.device.read_sector(HEADER_SECTOR, &mut sector)?;
        if &sector[..MAGIC.len()] != MAGIC {
            return Err(StorageError::InvalidImage);
        }
        if u32::from_le_bytes([sector[8], sector[9], sector[10], sector[11]]) != VERSION {
            return Err(StorageError::InvalidImage);
        }
        Ok(u32::from_le_bytes([sector[12], sector[13], sector[14], sector[15]]) as usize)
    }

    fn read_directory(&self) -> Result<Vec<Option<DirectoryEntry>>, StorageError> {
        if !self.mounted {
            return Err(StorageError::NotMounted);
        }
        self.read_header()?;
        let mut bytes = [0; DIRECTORY_SECTORS * SECTOR_SIZE];
        for sector_index in 0..DIRECTORY_SECTORS {
            let mut sector = [0; SECTOR_SIZE];
            self.device
                .read_sector(DIRECTORY_START_SECTOR + sector_index, &mut sector)?;
            let offset = sector_index * SECTOR_SIZE;
            bytes[offset..offset + SECTOR_SIZE].copy_from_slice(&sector);
        }

        let mut entries = Vec::new();
        for index in 0..MAX_FILES {
            let start = index * DIR_ENTRY_SIZE;
            entries.push(decode_entry(&bytes[start..start + DIR_ENTRY_SIZE])?);
        }
        Ok(entries)
    }

    fn write_directory(&mut self, entries: &[Option<DirectoryEntry>]) -> Result<(), StorageError> {
        if !self.mounted {
            return Err(StorageError::NotMounted);
        }
        let mut bytes = [0; DIRECTORY_SECTORS * SECTOR_SIZE];
        for (index, entry) in entries.iter().enumerate() {
            let start = index * DIR_ENTRY_SIZE;
            encode_entry(entry, &mut bytes[start..start + DIR_ENTRY_SIZE])?;
        }
        for sector_index in 0..DIRECTORY_SECTORS {
            let offset = sector_index * SECTOR_SIZE;
            let mut sector = [0; SECTOR_SIZE];
            sector.copy_from_slice(&bytes[offset..offset + SECTOR_SIZE]);
            self.device
                .write_sector(DIRECTORY_START_SECTOR + sector_index, &sector)?;
        }
        let count = entries.iter().filter(|entry| entry.is_some()).count();
        self.write_header(count)
    }

    fn write_data_sector(&mut self, data_sector: usize, contents: &[u8]) -> Result<(), StorageError> {
        let mut sector = [0; SECTOR_SIZE];
        sector[..contents.len()].copy_from_slice(contents);
        self.device.write_sector(data_sector, &sector)
    }
}

pub fn init() {
    let mut fs = STORAGE.lock();
    if fs.mount().is_err() {
        let _ = fs.format();
    }
    let _ = seed_bootstrap_files(&mut fs);
}

pub fn format() -> Result<(), StorageError> {
    let mut fs = STORAGE.lock();
    fs.format()?;
    seed_bootstrap_files(&mut fs)
}

pub fn remount() -> Result<(), StorageError> {
    let mut fs = STORAGE.lock();
    fs.unmount();
    fs.mount()
}

pub fn unmount() {
    STORAGE.lock().unmount();
}

pub fn is_mounted() -> bool {
    STORAGE.lock().is_mounted()
}

pub fn list_files() -> Result<Vec<String>, StorageError> {
    STORAGE.lock().list_files()
}

pub fn read_file(path: &str) -> Result<Option<String>, StorageError> {
    STORAGE.lock().read_file(path)
}

pub fn create_file(path: &str) -> Result<(), StorageError> {
    STORAGE.lock().create_file(path)
}

pub fn write_file(path: &str, contents: &str) -> Result<(), StorageError> {
    STORAGE.lock().write_file(path, contents)
}

pub fn delete_file(path: &str) -> Result<(), StorageError> {
    STORAGE.lock().delete_file(path)
}

pub fn info() -> Result<StorageInfo, StorageError> {
    STORAGE.lock().info()
}

pub fn phase7_smoke_check() -> bool {
    let path = "/phase7-smoke.txt";
    if write_file(path, "persistent-ok").is_err() {
        return false;
    }
    if remount().is_err() {
        return false;
    }
    matches!(read_file(path), Ok(Some(contents)) if contents == "persistent-ok")
}

fn seed_bootstrap_files<D: BlockDevice>(fs: &mut SimpleFs<D>) -> Result<(), StorageError> {
    for (path, contents) in [
        ("/README.txt", "AresOS persistent storage"),
        ("/bin/echo", "builtin: echo"),
        ("/bin/time", "builtin: time"),
        ("/bin/sysinfo", "builtin: sysinfo"),
        ("/bin/fsinfo", "builtin: fsinfo"),
    ] {
        match fs.write_file(path, contents) {
            Ok(()) | Err(StorageError::AlreadyExists) => {}
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

fn validate_path(path: &str) -> Result<(), StorageError> {
    if !path.starts_with('/') || path.len() > MAX_PATH_LEN || path.is_empty() {
        return Err(StorageError::InvalidPath);
    }
    if path.as_bytes().iter().any(|byte| *byte == 0) {
        return Err(StorageError::InvalidPath);
    }
    Ok(())
}

fn encode_entry(entry: &Option<DirectoryEntry>, out: &mut [u8]) -> Result<(), StorageError> {
    out.fill(0);
    let Some(entry) = entry else {
        return Ok(());
    };
    let path = entry.path.as_bytes();
    if path.len() > MAX_PATH_LEN || entry.len > MAX_FILE_SIZE {
        return Err(StorageError::InvalidImage);
    }
    out[0] = 1;
    out[1] = path.len() as u8;
    out[2..4].copy_from_slice(&(entry.len as u16).to_le_bytes());
    out[4..6].copy_from_slice(&(entry.data_sector as u16).to_le_bytes());
    out[8..8 + path.len()].copy_from_slice(path);
    Ok(())
}

fn decode_entry(input: &[u8]) -> Result<Option<DirectoryEntry>, StorageError> {
    if input[0] == 0 {
        return Ok(None);
    }
    let path_len = input[1] as usize;
    let len = u16::from_le_bytes([input[2], input[3]]) as usize;
    let data_sector = u16::from_le_bytes([input[4], input[5]]) as usize;
    if path_len == 0 || path_len > MAX_PATH_LEN || len > MAX_FILE_SIZE {
        return Err(StorageError::InvalidImage);
    }
    if data_sector < DATA_START_SECTOR || data_sector >= DATA_START_SECTOR + MAX_FILES {
        return Err(StorageError::InvalidImage);
    }
    let path = core::str::from_utf8(&input[8..8 + path_len])
        .map_err(|_| StorageError::InvalidImage)?
        .to_string();
    validate_path(&path)?;
    Ok(Some(DirectoryEntry {
        path,
        len,
        data_sector,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn memory_block_device_reads_and_writes_sectors() {
        let mut device = MemoryBlockDevice::new(4);
        let mut write = [0; SECTOR_SIZE];
        write[0] = 42;
        device.write_sector(2, &write).expect("sector write should succeed");

        let mut read = [0; SECTOR_SIZE];
        device.read_sector(2, &mut read).expect("sector read should succeed");
        assert_eq!(read[0], 42);
    }

    #[test_case]
    fn filesystem_has_default_files() {
        init();
        let files = list_files().expect("storage should be mounted");
        assert!(files.iter().any(|f| f == "/README.txt"));
    }

    #[test_case]
    fn unmounted_storage_rejects_access() {
        unmount();
        assert_eq!(list_files(), Err(StorageError::NotMounted));
        init();
    }

    #[test_case]
    fn simple_fs_persists_across_remount() {
        let mut fs = SimpleFs::new(MemoryBlockDevice::new(DEFAULT_SECTOR_COUNT));
        fs.format().expect("format should succeed");
        fs.write_file("/persist.txt", "hello").expect("write should succeed");
        fs.unmount();
        fs.mount().expect("remount should succeed");
        assert_eq!(
            fs.read_file("/persist.txt").expect("read should succeed"),
            Some("hello".to_string())
        );
    }

    #[test_case]
    fn invalid_image_is_rejected() {
        let mut fs = SimpleFs::new(MemoryBlockDevice::new(DEFAULT_SECTOR_COUNT));
        assert_eq!(fs.mount(), Err(StorageError::InvalidImage));
    }

    #[test_case]
    fn file_lifecycle_create_write_delete() {
        let mut fs = SimpleFs::new(MemoryBlockDevice::new(DEFAULT_SECTOR_COUNT));
        fs.format().expect("format should succeed");
        fs.create_file("/tmp.txt").expect("touch should succeed");
        assert_eq!(
            fs.read_file("/tmp.txt").expect("read should succeed"),
            Some(String::new())
        );
        fs.write_file("/tmp.txt", "updated").expect("write should succeed");
        assert_eq!(
            fs.read_file("/tmp.txt").expect("read should succeed"),
            Some("updated".to_string())
        );
        fs.delete_file("/tmp.txt").expect("delete should succeed");
        assert_eq!(fs.read_file("/tmp.txt").expect("read should succeed"), None);
    }
}
