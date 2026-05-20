# Phase 26 Checklist: Validated User Copyin

## Scope

- [x] Add bounded `copy_from_user` and `copy_to_user`.
- [x] Prove a user-buffer round-trip under active page tables.
- [x] Emit `Phase26-Copyin` boot smoke output.

## Validation

- [x] `cargo check -p kernel`
- [x] `cargo test -p kernel --test preemption_integration`
- [x] `python scripts/phase26_copyin_check.py --timeout 120`

## Deferred

- [ ] Storage syscalls with user pointers.
