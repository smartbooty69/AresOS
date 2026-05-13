//! Phase 16 inactive user page-table descriptors.

use alloc::vec::Vec;

use crate::{
    address_space::AddressSpaceId,
    frame_backing::FrameBackedImage,
    frame_ownership::OwnedFrameToken,
    load_plan::{LoadPermissions, PAGE_SIZE},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserPageTableId(u64);

impl UserPageTableId {
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserPageMapping {
    pub virtual_address: u64,
    pub physical_address: u64,
    pub frame: OwnedFrameToken,
    pub permissions: LoadPermissions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InactiveUserPageTable {
    pub id: UserPageTableId,
    pub address_space_id: AddressSpaceId,
    pub mappings: Vec<UserPageMapping>,
    pub mapped_pages: usize,
    pub executable_pages: usize,
    pub writable_pages: usize,
    pub read_only_pages: usize,
    pub kernel_shared: bool,
    pub cr3_switch_ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserMemoryError {
    EmptyBacking,
    DuplicateVirtualPage,
    UnalignedVirtualAddress,
}

pub fn build_inactive_page_table(
    id: UserPageTableId,
    backed: &FrameBackedImage,
) -> Result<InactiveUserPageTable, UserMemoryError> {
    if backed.total_pages == 0 || backed.regions.is_empty() {
        return Err(UserMemoryError::EmptyBacking);
    }

    let mut mappings = Vec::new();
    for region in &backed.regions {
        for page in &region.pages {
            if page.virtual_address % PAGE_SIZE as u64 != 0 {
                return Err(UserMemoryError::UnalignedVirtualAddress);
            }
            if mappings
                .iter()
                .any(|mapping: &UserPageMapping| mapping.virtual_address == page.virtual_address)
            {
                return Err(UserMemoryError::DuplicateVirtualPage);
            }
            mappings.push(UserPageMapping {
                virtual_address: page.virtual_address,
                physical_address: page.frame.start_address,
                frame: page.frame.token,
                permissions: page.permissions,
            });
        }
    }

    let executable_pages = mappings
        .iter()
        .filter(|mapping| mapping.permissions.executable())
        .count();
    let writable_pages = mappings
        .iter()
        .filter(|mapping| mapping.permissions.writable())
        .count();
    let read_only_pages = mappings
        .len()
        .saturating_sub(executable_pages)
        .saturating_sub(writable_pages);

    Ok(InactiveUserPageTable {
        id,
        address_space_id: backed.address_space_id,
        mapped_pages: mappings.len(),
        mappings,
        executable_pages,
        writable_pages,
        read_only_pages,
        kernel_shared: true,
        cr3_switch_ready: false,
    })
}

pub fn translate(table: &InactiveUserPageTable, virtual_address: u64) -> Option<u64> {
    let page_base = virtual_address & !((PAGE_SIZE as u64) - 1);
    let offset = virtual_address.saturating_sub(page_base);
    table
        .mappings
        .iter()
        .find(|mapping| mapping.virtual_address == page_base)
        .map(|mapping| mapping.physical_address.saturating_add(offset))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn translation_misses_empty_table() {
        let table = InactiveUserPageTable {
            id: UserPageTableId::from_raw(1),
            address_space_id: AddressSpaceId::from_raw(1),
            mappings: Vec::new(),
            mapped_pages: 0,
            executable_pages: 0,
            writable_pages: 0,
            read_only_pages: 0,
            kernel_shared: true,
            cr3_switch_ready: false,
        };
        assert_eq!(translate(&table, 0x400000), None);
    }
}
