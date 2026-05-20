# Phase 15 Checklist: Frame-Backed Images

## Scope

- [x] Add frame-backed image, region, and page records.
- [x] Consume owned frames from the Phase 14 frame ownership service.
- [x] Preserve Phase 13 mapping stub records and unsupported execution behavior.
- [x] Account copy and zero-fill bytes per backed page.
- [x] Add loader counters and blocked process metadata for `FrameBacked` records.
- [x] Expose frame-backed status through shell and syscall surfaces.
- [x] Emit `Phase15-FrameBackedImage` boot smoke output.

## Validation

- [x] `cargo check -p kernel`
- [x] `cargo test -p kernel --test preemption_integration`
- [x] `python scripts/phase15_frame_backing_check.py --timeout 120`

## Deferred

- [ ] Install frame-backed pages into inactive user page tables.
- [ ] Copy bytes into executable virtual mappings.
- [ ] Enter Ring 3 or jump to ELF entry points.
