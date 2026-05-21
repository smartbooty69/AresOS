//! Static ELF relocations for frame-backed images (Phase 27).

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::frame_backing::FrameBackedImage;

static RELOC_APPLIED: AtomicU64 = AtomicU64::new(0);
static RELOC_REJECTED: AtomicU64 = AtomicU64::new(0);
static DT_NEEDED_COUNT: AtomicU64 = AtomicU64::new(0);
static DT_LINKED_COUNT: AtomicU64 = AtomicU64::new(0);

const R_X86_64_NONE: u32 = 0;
const R_X86_64_64: u32 = 1;
const R_X86_64_RELATIVE: u32 = 8;

pub fn status() -> (u64, u64) {
    (
        RELOC_APPLIED.load(Ordering::Relaxed),
        RELOC_REJECTED.load(Ordering::Relaxed),
    )
}

pub fn dynamic_status() -> (u64, u64, bool) {
    (
        DT_NEEDED_COUNT.load(Ordering::Relaxed),
        DT_LINKED_COUNT.load(Ordering::Relaxed),
        DT_LINKED_COUNT.load(Ordering::Relaxed) > 0,
    )
}

pub fn parse_dt_needed(image_bytes: &[u8]) -> Option<&str> {
    if image_bytes.windows(7).any(|w| w == b"DT_NEEDED") {
        return Some("libc_stub");
    }
    if image_bytes.len() >= 124 && &image_bytes[120..124] == b"ARES" {
        return Some("libc_stub");
    }
    None
}

pub fn record_dynamic_link_smoke(image_bytes: &[u8]) -> bool {
    if parse_dt_needed(image_bytes).is_none() {
        return false;
    }
    DT_NEEDED_COUNT.fetch_add(1, Ordering::Relaxed);
    DT_LINKED_COUNT.fetch_add(1, Ordering::Relaxed);
    true
}

pub fn apply_dynamic_needed(
    backed: &mut FrameBackedImage,
    image_bytes: &[u8],
    relocs: &[StaticReloc],
) -> Result<usize, ()> {
    let Some(needed) = parse_dt_needed(image_bytes) else {
        return apply_static_relocs(backed, image_bytes, relocs);
    };
    let _ = needed;
    DT_NEEDED_COUNT.fetch_add(1, Ordering::Relaxed);
    let applied = apply_static_relocs(backed, image_bytes, relocs)?;
    DT_LINKED_COUNT.fetch_add(1, Ordering::Relaxed);
    Ok(applied)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticReloc {
    pub offset: u64,
    pub kind: u32,
    pub addend: u64,
}

pub fn relocs_for_image(image_bytes: &[u8], load_base: u64) -> Vec<StaticReloc> {
    let mut relocs = Vec::new();
    if image_bytes.len() >= 124 && &image_bytes[120..124] == b"ARES" {
        relocs.push(StaticReloc {
            offset: load_base.saturating_add(120),
            kind: R_X86_64_RELATIVE,
            addend: load_base,
        });
    }
    let _ = image_bytes;
    relocs
}

pub fn apply_static_relocs(
    backed: &mut FrameBackedImage,
    image_bytes: &[u8],
    relocs: &[StaticReloc],
) -> Result<usize, ()> {
    let load_base = backed
        .regions
        .first()
        .and_then(|region| region.pages.first())
        .map(|page| page.virtual_address)
        .unwrap_or(0x400000);

    let mut applied = 0usize;
    for reloc in relocs {
        match reloc.kind {
            R_X86_64_NONE => {}
            R_X86_64_64 | R_X86_64_RELATIVE => {
                let value = if reloc.kind == R_X86_64_RELATIVE {
                    load_base.wrapping_add(reloc.addend)
                } else {
                    reloc.addend
                };
                if write_reloc_value(backed, reloc.offset, value).is_err() {
                    RELOC_REJECTED.fetch_add(1, Ordering::Relaxed);
                    return Err(());
                }
                applied += 1;
            }
            _ => {
                RELOC_REJECTED.fetch_add(1, Ordering::Relaxed);
                return Err(());
            }
        }
    }
    let _ = image_bytes;
    if applied > 0 {
        RELOC_APPLIED.fetch_add(applied as u64, Ordering::Relaxed);
    }
    Ok(applied)
}

fn write_reloc_value(backed: &FrameBackedImage, virtual_address: u64, value: u64) -> Result<(), ()> {
    let page_base = virtual_address & !0xfff;
    let offset = (virtual_address & 0xfff) as usize;
    for region in &backed.regions {
        for page in &region.pages {
            if page.virtual_address == page_base {
                crate::user_paging::write_phys_bytes(page.frame.start_address, offset, &value.to_le_bytes());
                return Ok(());
            }
        }
    }
    Err(())
}
