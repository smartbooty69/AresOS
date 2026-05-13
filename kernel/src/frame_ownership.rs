//! Phase 14 persistent frame ownership bookkeeping.

use alloc::vec::Vec;
use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use lazy_static::lazy_static;
use spin::Mutex;

pub const MAX_TRACKED_FRAMES: usize = 512;
const PAGE_SIZE: u64 = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnedFrameToken(u64);

impl OwnedFrameToken {
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameOwner {
    Kernel,
    Image,
    PageTable,
    Test,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnedFrame {
    pub token: OwnedFrameToken,
    pub start_address: u64,
    pub owner: FrameOwner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameOwnershipError {
    AlreadyInitialized,
    NotInitialized,
    Exhausted,
    UnknownFrame,
    AlreadyReleased,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameOwnershipStatus {
    pub initialized: bool,
    pub tracked_frames: usize,
    pub available_frames: usize,
    pub allocated_frames: usize,
    pub allocation_count: u64,
    pub release_count: u64,
    pub failed_allocation_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FrameRecord {
    token: OwnedFrameToken,
    start_address: u64,
    owner: Option<FrameOwner>,
}

struct FrameRegistry {
    initialized: bool,
    records: Vec<FrameRecord>,
    next_token: u64,
    allocation_count: u64,
    release_count: u64,
    failed_allocation_count: u64,
}

impl FrameRegistry {
    fn new() -> Self {
        Self {
            initialized: false,
            records: Vec::new(),
            next_token: 1,
            allocation_count: 0,
            release_count: 0,
            failed_allocation_count: 0,
        }
    }

    fn init_from_addresses(&mut self, addresses: &[u64]) -> Result<(), FrameOwnershipError> {
        if self.initialized {
            return Err(FrameOwnershipError::AlreadyInitialized);
        }
        self.records.clear();
        for address in addresses.iter().copied().take(MAX_TRACKED_FRAMES) {
            let token = OwnedFrameToken::from_raw(self.next_token);
            self.next_token = self.next_token.saturating_add(1);
            self.records.push(FrameRecord {
                token,
                start_address: address,
                owner: None,
            });
        }
        self.initialized = true;
        Ok(())
    }

    fn allocate(&mut self, owner: FrameOwner) -> Result<OwnedFrame, FrameOwnershipError> {
        if !self.initialized {
            return Err(FrameOwnershipError::NotInitialized);
        }
        let Some(record) = self.records.iter_mut().find(|record| record.owner.is_none()) else {
            self.failed_allocation_count = self.failed_allocation_count.saturating_add(1);
            return Err(FrameOwnershipError::Exhausted);
        };
        record.owner = Some(owner);
        self.allocation_count = self.allocation_count.saturating_add(1);
        Ok(OwnedFrame {
            token: record.token,
            start_address: record.start_address,
            owner,
        })
    }

    fn release(&mut self, token: OwnedFrameToken) -> Result<(), FrameOwnershipError> {
        if !self.initialized {
            return Err(FrameOwnershipError::NotInitialized);
        }
        let Some(record) = self.records.iter_mut().find(|record| record.token == token) else {
            return Err(FrameOwnershipError::UnknownFrame);
        };
        if record.owner.is_none() {
            return Err(FrameOwnershipError::AlreadyReleased);
        }
        record.owner = None;
        self.release_count = self.release_count.saturating_add(1);
        Ok(())
    }

    fn status(&self) -> FrameOwnershipStatus {
        let allocated_frames = self
            .records
            .iter()
            .filter(|record| record.owner.is_some())
            .count();
        FrameOwnershipStatus {
            initialized: self.initialized,
            tracked_frames: self.records.len(),
            available_frames: self.records.len().saturating_sub(allocated_frames),
            allocated_frames,
            allocation_count: self.allocation_count,
            release_count: self.release_count,
            failed_allocation_count: self.failed_allocation_count,
        }
    }
}

lazy_static! {
    static ref REGISTRY: Mutex<FrameRegistry> = Mutex::new(FrameRegistry::new());
}

pub fn init_from_memory_map(
    memory_map: &'static MemoryMap,
    skip_allocated_frames: usize,
) -> Result<(), FrameOwnershipError> {
    let mut addresses = Vec::new();
    for region in memory_map.iter().filter(|region| region.region_type == MemoryRegionType::Usable) {
        for address in (region.range.start_addr()..region.range.end_addr()).step_by(PAGE_SIZE as usize) {
            if addresses.len() >= skip_allocated_frames.saturating_add(MAX_TRACKED_FRAMES) {
                break;
            }
            addresses.push(address);
        }
        if addresses.len() >= skip_allocated_frames.saturating_add(MAX_TRACKED_FRAMES) {
            break;
        }
    }
    let tracked = addresses
        .into_iter()
        .skip(skip_allocated_frames)
        .collect::<Vec<_>>();
    REGISTRY.lock().init_from_addresses(&tracked)
}

pub fn allocate_frame(owner: FrameOwner) -> Result<OwnedFrame, FrameOwnershipError> {
    REGISTRY.lock().allocate(owner)
}

pub fn release_frame(token: OwnedFrameToken) -> Result<(), FrameOwnershipError> {
    REGISTRY.lock().release(token)
}

pub fn status() -> FrameOwnershipStatus {
    REGISTRY.lock().status()
}

pub fn phase14_smoke_check() -> bool {
    let before = status();
    if !before.initialized || before.available_frames == 0 {
        return false;
    }
    let Ok(frame) = allocate_frame(FrameOwner::Test) else {
        return false;
    };
    let allocated = status();
    let released = release_frame(frame.token).is_ok();
    let after = status();
    released
        && frame.start_address % PAGE_SIZE == 0
        && allocated.allocated_frames == before.allocated_frames.saturating_add(1)
        && after.allocated_frames == before.allocated_frames
        && after.release_count > before.release_count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn registry_allocates_and_releases_frames() {
        let mut registry = FrameRegistry::new();
        registry
            .init_from_addresses(&[0x1000, 0x2000])
            .expect("registry should initialize");
        let frame = registry
            .allocate(FrameOwner::Image)
            .expect("frame should allocate");
        assert_eq!(frame.start_address, 0x1000);
        assert_eq!(registry.status().allocated_frames, 1);
        registry.release(frame.token).expect("frame should release");
        assert_eq!(registry.status().allocated_frames, 0);
    }

    #[test_case]
    fn registry_reports_exhaustion_without_corruption() {
        let mut registry = FrameRegistry::new();
        registry
            .init_from_addresses(&[0x1000])
            .expect("registry should initialize");
        let _frame = registry
            .allocate(FrameOwner::Image)
            .expect("first frame should allocate");
        assert_eq!(registry.allocate(FrameOwner::Image), Err(FrameOwnershipError::Exhausted));
        let status = registry.status();
        assert_eq!(status.allocated_frames, 1);
        assert_eq!(status.failed_allocation_count, 1);
    }
}
