# Phase 17 Checklist: User Context And Entry Frames

## Scope

- [x] Add user code and data selectors to the GDT.
- [x] Expose user selector descriptors for validation.
- [x] Build initial user entry frames with RIP, RSP, RFLAGS, CS, and SS.
- [x] Add user stack descriptors.
- [x] Add blocked `UserContextReady` process metadata.
- [x] Expose user-context status through shell and syscall surfaces.
- [x] Emit `Phase17-UserContext` boot smoke output.

## Validation

- [x] `cargo check -p kernel`
- [x] `cargo test -p kernel --test preemption_integration`
- [x] `python scripts/phase17_user_context_check.py --timeout 120`

## Deferred

- [ ] Execute the interrupt-return transition to Ring 3.
- [ ] Switch CR3 to user page tables.
- [ ] Run ELF entry points.
