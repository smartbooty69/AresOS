//! Phase 11 descriptor-only address-space model.

use alloc::vec::Vec;

use crate::exec_image::{ExecutableImage, ExecutableFormat, SegmentFlags};

const USER_MIN: u64 = 0x1000;
const USER_MAX: u64 = 0x0000_8000_0000_0000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddressSpaceId(u64);

impl AddressSpaceId {
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionKind {
    Code,
    Data,
    Stack,
    Heap,
    KernelShared,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualRegion {
    pub start: u64,
    pub size: usize,
    pub kind: RegionKind,
    pub flags: SegmentFlags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingState {
    Planned,
    Reserved,
    MappedStub,
    FrameBacked,
    PageTableMapped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameReservationSummary {
    pub user_pages: usize,
    pub stack_pages: usize,
    pub executable_pages: usize,
    pub writable_pages: usize,
    pub read_only_pages: usize,
    pub mapping_state: MappingState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressSpaceDescriptor {
    pub id: AddressSpaceId,
    pub regions: Vec<VirtualRegion>,
    pub reservation: FrameReservationSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressSpaceError {
    InvalidRegion,
    Overlap,
    WritableExecutable,
    KernelRange,
}

pub fn descriptor_for_image(
    id: AddressSpaceId,
    image: &ExecutableImage,
) -> Result<AddressSpaceDescriptor, AddressSpaceError> {
    let mut regions = Vec::new();
    if image.format == ExecutableFormat::BuiltinAlias {
        return Ok(AddressSpaceDescriptor {
            id,
            regions,
            reservation: empty_reservation(),
        });
    }

    for segment in &image.segments {
        regions.push(VirtualRegion {
            start: segment.virtual_address,
            size: segment.memory_size,
            kind: kind_for_flags(segment.flags),
            flags: segment.flags,
        });
    }
    validate_regions(&regions)?;
    let reservation = reservation_for_regions(&regions, crate::load_plan::STACK_RESERVATION_PAGES);
    Ok(AddressSpaceDescriptor {
        id,
        regions,
        reservation,
    })
}

pub fn descriptor_for_load_plan(
    id: AddressSpaceId,
    plan: &crate::load_plan::LoadPlan,
) -> Result<AddressSpaceDescriptor, AddressSpaceError> {
    let regions = plan
        .regions
        .iter()
        .map(|region| VirtualRegion {
            start: region.start,
            size: region.size,
            kind: kind_for_load_permissions(region.permissions),
            flags: segment_flags_for_load_permissions(region.permissions),
        })
        .collect::<Vec<_>>();
    validate_regions(&regions)?;
    Ok(AddressSpaceDescriptor {
        id,
        reservation: reservation_for_regions(&regions, plan.stack_pages),
        regions,
    })
}

pub fn descriptor_for_mapped_image(mapped: &crate::mapping_stub::MappedImage) -> AddressSpaceDescriptor {
    let regions = mapped
        .regions
        .iter()
        .map(|region| VirtualRegion {
            start: region.start,
            size: region.size,
            kind: kind_for_load_permissions(region.permissions),
            flags: segment_flags_for_load_permissions(region.permissions),
        })
        .collect::<Vec<_>>();
    let mut reservation = reservation_for_regions(&regions, 0);
    reservation.mapping_state = MappingState::MappedStub;
    AddressSpaceDescriptor {
        id: mapped.address_space_id,
        regions,
        reservation,
    }
}

pub fn validate_regions(regions: &[VirtualRegion]) -> Result<(), AddressSpaceError> {
    for (index, region) in regions.iter().enumerate() {
        if region.start < USER_MIN || region.size == 0 {
            return Err(AddressSpaceError::InvalidRegion);
        }
        let end = region
            .start
            .checked_add(region.size as u64)
            .ok_or(AddressSpaceError::InvalidRegion)?;
        if end > USER_MAX && region.kind != RegionKind::KernelShared {
            return Err(AddressSpaceError::KernelRange);
        }
        if region.flags.writable() && region.flags.executable() {
            return Err(AddressSpaceError::WritableExecutable);
        }
        for other in regions.iter().skip(index + 1) {
            let other_end = other
                .start
                .checked_add(other.size as u64)
                .ok_or(AddressSpaceError::InvalidRegion)?;
            if region.start < other_end && other.start < end {
                return Err(AddressSpaceError::Overlap);
            }
        }
    }
    Ok(())
}

fn kind_for_flags(flags: SegmentFlags) -> RegionKind {
    if flags.executable() {
        RegionKind::Code
    } else if flags.writable() {
        RegionKind::Data
    } else {
        RegionKind::Data
    }
}

fn kind_for_load_permissions(permissions: crate::load_plan::LoadPermissions) -> RegionKind {
    if permissions.executable() {
        RegionKind::Code
    } else if permissions.writable() {
        RegionKind::Data
    } else {
        RegionKind::Data
    }
}

fn segment_flags_for_load_permissions(permissions: crate::load_plan::LoadPermissions) -> SegmentFlags {
    SegmentFlags::from_bits(
        ((permissions.readable() as u8) * SegmentFlags::READ)
            | ((permissions.writable() as u8) * SegmentFlags::WRITE)
            | ((permissions.executable() as u8) * SegmentFlags::EXECUTE),
    )
}

fn reservation_for_regions(regions: &[VirtualRegion], stack_pages: usize) -> FrameReservationSummary {
    let mut summary = FrameReservationSummary {
        user_pages: stack_pages,
        stack_pages,
        executable_pages: 0,
        writable_pages: 0,
        read_only_pages: 0,
        mapping_state: MappingState::Planned,
    };
    for region in regions {
        let pages = (region.size + crate::load_plan::PAGE_SIZE - 1) / crate::load_plan::PAGE_SIZE;
        summary.user_pages += pages;
        if region.flags.executable() {
            summary.executable_pages += pages;
        } else if region.flags.writable() {
            summary.writable_pages += pages;
        } else {
            summary.read_only_pages += pages;
        }
    }
    summary
}

fn empty_reservation() -> FrameReservationSummary {
    FrameReservationSummary {
        user_pages: 0,
        stack_pages: 0,
        executable_pages: 0,
        writable_pages: 0,
        read_only_pages: 0,
        mapping_state: MappingState::Planned,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exec_image::{ImageSegment, SegmentFlags};

    #[test_case]
    fn overlapping_regions_are_rejected() {
        let flags = SegmentFlags::from_bits(SegmentFlags::READ);
        let regions = [
            VirtualRegion {
                start: 0x400000,
                size: 0x1000,
                kind: RegionKind::Code,
                flags,
            },
            VirtualRegion {
                start: 0x400800,
                size: 0x1000,
                kind: RegionKind::Data,
                flags,
            },
        ];
        assert_eq!(validate_regions(&regions), Err(AddressSpaceError::Overlap));
    }

    #[test_case]
    fn writable_executable_regions_are_rejected() {
        let regions = [VirtualRegion {
            start: 0x400000,
            size: 0x1000,
            kind: RegionKind::Code,
            flags: SegmentFlags::from_bits(SegmentFlags::WRITE | SegmentFlags::EXECUTE),
        }];
        assert_eq!(
            validate_regions(&regions),
            Err(AddressSpaceError::WritableExecutable)
        );
    }

    #[test_case]
    fn image_descriptor_uses_segment_regions() {
        let image = ExecutableImage {
            name: "test".into(),
            source_path: "/bin/test.elf".into(),
            format: ExecutableFormat::Elf64,
            entry_point: 0x400000,
            image_size: 128,
            trust: crate::task::program_loader::ProgramTrust::User,
            owner: crate::security::UserId::from_raw(100),
            segments: alloc::vec![ImageSegment {
                virtual_address: 0x400000,
                file_offset: 120,
                file_size: 4,
                memory_size: 0x1000,
                flags: SegmentFlags::from_bits(SegmentFlags::READ | SegmentFlags::EXECUTE),
            }],
        };
        let descriptor = descriptor_for_image(AddressSpaceId::from_raw(1), &image)
            .expect("descriptor should be valid");
        assert_eq!(descriptor.regions.len(), 1);
        assert_eq!(descriptor.regions[0].kind, RegionKind::Code);
        assert_eq!(descriptor.reservation.executable_pages, 1);
    }
}
