# Phase 19 Checklist: Syscall Entry And Return ABI

## Scope

- [x] Add user register-frame syscall ABI records.
- [x] Dispatch user syscall frames through the existing syscall table.
- [x] Record return values and syscall errors for user-mode return.
- [x] Add a user syscall probe path for validated image programs.
- [x] Add blocked `UserSyscallReturned` process metadata.
- [x] Expose user syscall counters through shell and syscalls.
- [x] Emit `Phase19-SyscallReturn` boot smoke output.

## Validation

- [x] `cargo check -p kernel`
- [x] `cargo test -p kernel --test preemption_integration`
- [x] `python scripts/phase19_syscall_return_check.py --timeout 20`

## Deferred

- [ ] Use CPU syscall/sysret instructions.
- [ ] Copy buffers through validated user pointers.
- [ ] Execute arbitrary ELF syscall instructions.
