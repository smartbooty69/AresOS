# Phase 27 Checklist: Static ELF Relocations

## Scope

- [x] Apply static `R_X86_64_RELATIVE` fixups for seeded images.
- [x] Write image bytes into frame-backed pages during backing.
- [x] Emit `Phase27-Reloc` boot smoke output.

## Validation

- [x] `cargo check -p kernel`
- [x] `cargo test -p kernel --test preemption_integration`
- [x] `python scripts/phase27_reloc_check.py --timeout 120`

## Deferred

- [ ] Dynamic linking and `DT_NEEDED`.
