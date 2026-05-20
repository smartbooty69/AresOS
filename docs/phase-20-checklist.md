# Phase 20 Checklist: Minimal ELF Execution MVP

## Scope

- [x] Allow the seeded `/bin/hello` ELF program to complete through the guarded user pipeline.
- [x] Return deterministic output and exit status for `run hello`.
- [x] Keep arbitrary ELF execution, dynamic linking, relocation, and demand paging out of scope.
- [x] Add blocked `UserElfExited` process metadata.
- [x] Expose ELF execution counters through shell and syscalls.
- [x] Emit `Phase20-UserElf` boot smoke output.

## Validation

- [x] `cargo check -p kernel`
- [x] `cargo test -p kernel --test preemption_integration`
- [x] `python scripts/phase20_user_elf_check.py --timeout 120`

## Deferred

- [ ] Run arbitrary user ELF instructions.
- [ ] Implement relocations and dynamic linking.
- [ ] Implement demand paging and full process isolation.
