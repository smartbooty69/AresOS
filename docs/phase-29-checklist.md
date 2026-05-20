# Phase 29 Checklist: Allowlisted ELF Programs

## Scope

- [x] Allowlist `hello` and `exit42`.
- [x] Seed `/bin/exit42` manifest and ELF fixture.
- [x] Emit `Phase29-Allowlist` boot smoke output.

## Validation

- [x] `cargo check -p kernel`
- [x] `cargo test -p kernel --test preemption_integration`
- [x] `python scripts/phase29_allowlist_check.py --timeout 120`

## Deferred

- [ ] Manifest-discovered arbitrary ELFs.
