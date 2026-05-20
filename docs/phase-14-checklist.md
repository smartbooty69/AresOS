# Phase 14 Checklist: Frame Ownership Service

## Scope

- [x] Add a persistent frame ownership registry initialized from the bootloader memory map.
- [x] Track bounded frame records, owners, allocations, releases, and failed allocation attempts.
- [x] Preserve Phase 13 deterministic mapping stubs without consuming owned frames.
- [x] Expose frame ownership status through shell and syscall surfaces.
- [x] Emit `Phase14-Frames` boot smoke output.

## Validation

- [x] `cargo check -p kernel`
- [x] `cargo test -p kernel --test preemption_integration`
- [x] `python scripts/phase14_frame_check.py --timeout 120`

## Deferred

- [ ] Use owned frames as backing storage for executable load plans.
- [ ] Install owned frames into inactive user page tables.
- [ ] Reclaim frames from terminated user processes.
