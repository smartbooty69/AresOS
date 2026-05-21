# Phase 34 Checklist: Exit and Wait Syscalls

## Scope

- [x] `SyscallId::ExitProcess` and `WaitProcess`.
- [x] Kernel exit/wait accounting smoke.
- [x] Emit `Phase34-ExitWait` boot smoke output.

## Validation

- [x] `cargo check -p kernel`
- [x] `cargo test -p kernel --features preemption --test preemption_integration`
- [x] `python scripts/phase34_exit_wait_check.py --timeout 120`

## Deferred

- [ ] Per-PID wait queues and parent/child linkage.
