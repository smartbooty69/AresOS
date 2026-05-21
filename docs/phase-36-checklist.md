# Phase 36 Checklist: Storage Syscalls With Copyin

## Scope

- [x] `ReadFileProbe` / `WriteFileProbe` syscalls via `invoke_raw`.
- [x] `storage_read_probe` using validated `copy_to_user`.
- [x] Emit `Phase36-StorageCopyin` boot smoke output.

## Validation

- [x] `cargo check -p kernel`
- [x] `cargo test -p kernel --features preemption --test preemption_integration`
- [x] `python scripts/phase36_storage_copyin_check.py --timeout 120`

## Deferred

- [ ] Arbitrary path strings from user space.
