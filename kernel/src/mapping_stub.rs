//! Phase 13 deterministic mapping stubs for executable load plans.

use alloc::{
    string::String,
    vec::Vec,
};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::{
    address_space::{AddressSpaceId, MappingState},
    load_plan::{LoadAction, LoadPermissions, LoadPlan, PAGE_SIZE},
    security::Credentials,
};

const MAX_MAPPED_IMAGES: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MappingId(u64);

impl MappingId {
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameToken(u64);

impl FrameToken {
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedPage {
    pub virtual_address: u64,
    pub frame: FrameToken,
    pub permissions: LoadPermissions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappingActionResult {
    pub target_address: u64,
    pub len: usize,
    pub kind: MappingActionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingActionKind {
    Copy,
    ZeroFill,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedRegion {
    pub start: u64,
    pub size: usize,
    pub permissions: LoadPermissions,
    pub pages: Vec<MappedPage>,
    pub actions: Vec<MappingActionResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedImage {
    pub id: MappingId,
    pub image_name: String,
    pub source_path: String,
    pub address_space_id: AddressSpaceId,
    pub regions: Vec<MappedRegion>,
    pub total_pages: usize,
    pub executable_pages: usize,
    pub writable_pages: usize,
    pub read_only_pages: usize,
    pub copied_bytes: usize,
    pub zero_filled_bytes: usize,
    pub owner: Credentials,
    pub state: MappingState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingStubError {
    EmptyPlan,
    PageBudgetExceeded,
    RegistryFull,
    CopyOutOfBounds,
    UnsafePermissions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MappingRegistryStatus {
    pub mapped_count: usize,
    pub total_pages: usize,
    pub copied_bytes: usize,
    pub zero_filled_bytes: usize,
}

struct MappingRegistry {
    next_id: u64,
    mappings: Vec<MappedImage>,
}

impl MappingRegistry {
    fn new() -> Self {
        Self {
            next_id: 1,
            mappings: Vec::new(),
        }
    }

    fn insert(
        &mut self,
        owner: Credentials,
        address_space_id: AddressSpaceId,
        plan: &LoadPlan,
    ) -> Result<MappedImage, MappingStubError> {
        if self.mappings.len() >= MAX_MAPPED_IMAGES {
            return Err(MappingStubError::RegistryFull);
        }
        let id = MappingId::from_raw(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        let mapped = map_load_plan(owner, id, address_space_id, plan)?;
        self.mappings.push(mapped.clone());
        Ok(mapped)
    }

    fn list(&self) -> Vec<MappedImage> {
        self.mappings.clone()
    }

    fn get(&self, id: MappingId) -> Option<MappedImage> {
        self.mappings.iter().find(|mapping| mapping.id == id).cloned()
    }

    fn status(&self) -> MappingRegistryStatus {
        MappingRegistryStatus {
            mapped_count: self.mappings.len(),
            total_pages: self.mappings.iter().map(|mapping| mapping.total_pages).sum(),
            copied_bytes: self.mappings.iter().map(|mapping| mapping.copied_bytes).sum(),
            zero_filled_bytes: self
                .mappings
                .iter()
                .map(|mapping| mapping.zero_filled_bytes)
                .sum(),
        }
    }
}

lazy_static! {
    static ref REGISTRY: Mutex<MappingRegistry> = Mutex::new(MappingRegistry::new());
}

pub fn register_mapping(
    owner: Credentials,
    address_space_id: AddressSpaceId,
    plan: &LoadPlan,
) -> Result<MappedImage, MappingStubError> {
    REGISTRY.lock().insert(owner, address_space_id, plan)
}

pub fn list_mappings() -> Vec<MappedImage> {
    REGISTRY.lock().list()
}

pub fn get_mapping(id: MappingId) -> Option<MappedImage> {
    REGISTRY.lock().get(id)
}

pub fn status() -> MappingRegistryStatus {
    REGISTRY.lock().status()
}

pub fn map_load_plan(
    owner: Credentials,
    id: MappingId,
    address_space_id: AddressSpaceId,
    plan: &LoadPlan,
) -> Result<MappedImage, MappingStubError> {
    if plan.regions.is_empty() || plan.total_pages == 0 {
        return Err(MappingStubError::EmptyPlan);
    }
    if plan.total_pages + plan.stack_pages > crate::load_plan::MAX_IMAGE_PAGES {
        return Err(MappingStubError::PageBudgetExceeded);
    }

    let mut next_frame = id.as_u64().saturating_mul(10_000);
    let mut regions = Vec::new();
    let mut total_pages = 0usize;
    let mut executable_pages = 0usize;
    let mut writable_pages = 0usize;
    let mut read_only_pages = 0usize;
    let mut copied_bytes = 0usize;
    let mut zero_filled_bytes = 0usize;

    for region in &plan.regions {
        if region.permissions.writable() && region.permissions.executable() {
            return Err(MappingStubError::UnsafePermissions);
        }
        let mut pages = Vec::new();
        for page_index in 0..region.page_count {
            pages.push(MappedPage {
                virtual_address: region.start + (page_index * PAGE_SIZE) as u64,
                frame: FrameToken::from_raw(next_frame),
                permissions: region.permissions,
            });
            next_frame = next_frame.saturating_add(1);
        }

        let mut actions = Vec::new();
        for action in &region.actions {
            match *action {
                LoadAction::Copy {
                    target_address,
                    len,
                    ..
                } => {
                    validate_action(region.start, region.size, target_address, len)?;
                    copied_bytes = copied_bytes.saturating_add(len);
                    actions.push(MappingActionResult {
                        target_address,
                        len,
                        kind: MappingActionKind::Copy,
                    });
                }
                LoadAction::ZeroFill {
                    target_address,
                    len,
                } => {
                    validate_action(region.start, region.size, target_address, len)?;
                    zero_filled_bytes = zero_filled_bytes.saturating_add(len);
                    actions.push(MappingActionResult {
                        target_address,
                        len,
                        kind: MappingActionKind::ZeroFill,
                    });
                }
            }
        }

        total_pages = total_pages.saturating_add(region.page_count);
        if region.permissions.executable() {
            executable_pages = executable_pages.saturating_add(region.page_count);
        } else if region.permissions.writable() {
            writable_pages = writable_pages.saturating_add(region.page_count);
        } else {
            read_only_pages = read_only_pages.saturating_add(region.page_count);
        }

        regions.push(MappedRegion {
            start: region.start,
            size: region.size,
            permissions: region.permissions,
            pages,
            actions,
        });
    }

    Ok(MappedImage {
        id,
        image_name: plan.image_name.clone(),
        source_path: plan.source_path.clone(),
        address_space_id,
        regions,
        total_pages,
        executable_pages,
        writable_pages,
        read_only_pages,
        copied_bytes,
        zero_filled_bytes,
        owner,
        state: MappingState::MappedStub,
    })
}

fn validate_action(
    region_start: u64,
    region_size: usize,
    target_address: u64,
    len: usize,
) -> Result<(), MappingStubError> {
    let region_end = region_start
        .checked_add(region_size as u64)
        .ok_or(MappingStubError::CopyOutOfBounds)?;
    let action_end = target_address
        .checked_add(len as u64)
        .ok_or(MappingStubError::CopyOutOfBounds)?;
    if target_address < region_start || action_end > region_end {
        return Err(MappingStubError::CopyOutOfBounds);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_plan() -> LoadPlan {
        let image = crate::exec_image::parse_elf64_image(
            "hello",
            "/bin/hello.elf",
            crate::storage::phase11_sample_elf_image().as_bytes(),
            crate::task::program_loader::ProgramTrust::User,
            crate::security::Credentials::shell_user().user,
        )
        .expect("sample image should parse");
        crate::load_plan::build_load_plan(&image).expect("load plan should build")
    }

    #[test_case]
    fn maps_one_frame_token_per_page() {
        let mapped = map_load_plan(
            crate::security::Credentials::shell_user(),
            MappingId::from_raw(1),
            AddressSpaceId::from_raw(1),
            &sample_plan(),
        )
        .expect("mapping should succeed");
        assert_eq!(mapped.total_pages, 1);
        assert_eq!(mapped.regions[0].pages.len(), 1);
        assert_eq!(mapped.regions[0].pages[0].frame.as_u64(), 10_000);
    }

    #[test_case]
    fn copy_and_zero_fill_are_accounted() {
        let mapped = map_load_plan(
            crate::security::Credentials::shell_user(),
            MappingId::from_raw(2),
            AddressSpaceId::from_raw(2),
            &sample_plan(),
        )
        .expect("mapping should succeed");
        assert_eq!(mapped.copied_bytes, 4);
        assert_eq!(mapped.zero_filled_bytes, 4092);
    }
}
