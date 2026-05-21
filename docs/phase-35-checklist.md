# Phase 35 Checklist: Hardware Syscall Dispatch Table

## Scope

- [x] `ALLOWED_HW_SYSCALLS` allowlist in `user_syscall_hw`.
- [x] Reject unknown syscall IDs with accounting.
- [x] Emit `Phase35-SyscallTable` boot smoke output.

## Validation

- [x] `cargo check -p kernel`
- [x] `cargo test -p kernel --features preemption --test preemption_integration`
- [x] `python scripts/phase35_syscall_table_check.py --timeout 120`

## Deferred

- [ ] Unbounded syscall IDs from user programs.
- [ ] User buffer arguments without validated copyin.
