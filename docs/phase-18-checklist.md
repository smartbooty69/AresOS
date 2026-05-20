# Phase 18 Checklist: Controlled Ring 3 Trampoline

## Scope

- [x] Add a controlled user trap vector.
- [x] Add Ring 3 trampoline result and error records.
- [x] Model controlled entry and trap-back behavior from prepared user contexts.
- [x] Add blocked `UserTrapped` process metadata.
- [x] Expose Ring 3 trampoline counters through shell and syscalls.
- [x] Emit `Phase18-Ring3` boot smoke output.

## Validation

- [x] `cargo check -p kernel`
- [x] `cargo test -p kernel --test preemption_integration`
- [x] `python scripts/phase18_ring3_check.py --timeout 120`

## Deferred

- [ ] Execute a hardware `iretq` transition.
- [ ] Run arbitrary ELF entry points.
- [ ] Implement user syscall return for real user code.
