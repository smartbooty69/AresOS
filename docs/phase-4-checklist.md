# Phase 4 Completion Checklist (Processes / Async Executor)

Date: 2026-03-17

## Scope
Phase 4 roadmap goals:
- async cooperative task executor
- timer / sleep futures
- keyboard async task
- context switching groundwork

## Completion Criteria
- [x] `TaskId` / `Task` wrappers around `Pin<Box<dyn Future>>`
- [x] `SimpleExecutor` → replaced by `Executor` with `BTreeMap` task queue
- [x] Waker implementation via `TaskWaker` (alloc-based, wake-by-value)
- [x] Timer sleep future (`TimerFuture` / `sleep_ticks`) polled via waker
- [x] Asynchronous keyboard task (`task::keyboard`) driven by scancode queue
- [x] Per-task round-robin fairness: `FAIRNESS_CHECK_INTERVAL_TICKS = 10` — executor
  breaks the inner run-ready loop when a fairness period elapses and
  `!task_queue.is_empty()`
- [x] Scheduler signal primitives (`SchedulerSignal`, `notify_scheduler`,
  `poll_scheduler_signal`)
- [x] Context switching infrastructure: `CpuContext`, `KernelStack`,
  `RunnableContext`, `switch_context` (assembly, CLI guard)
- [x] Context-lab demo behind `context-lab` feature flag — round-robin
  switch between three tasks with stall detection and slot-based handoff
  tokens; starvation risk mitigated via `cli` in `switch_context` and
  reduced spin threshold
- [x] Deferred IRQ handoff tokens (`IrqHandoffToken`) to avoid spinlock
  deadlock at interrupt boundaries
- [x] Performance metrics module (`kernel::performance`)

## Caveat
Context switching is exercised through the `context-lab` feature only.
The default kernel main loop remains cooperative async (no preemption).
Full preemptive scheduling with process isolation is a Phase 5 / 6 target.

## Validation Snapshot
- Last full validation command: `cargo test -p kernel`
- Result: all 27 tests pass (16 unit + 11 integration)

## Phase 4 Exit Gate (Wrapper-Mode Soak)
- Command: `./scripts/phase4-soak-check`
- Purpose: guard against regression of the wrapper-mode IRQ handoff stall.
- Pass criteria:
  - Captures at least the script minimum number of `ContextLab` samples
  - `ticks`, `switches`, and both task counters (`A`, `B`) advance
  - `handoff_q` and `handoff_c` both advance and remain equal at end of run
  - `misses` does not increase during the soak window
- Optional tuning:
  - `./scripts/phase4-soak-check --duration 120 --min-samples 10`
  - Increase duration for pre-merge or release-candidate verification

## Notes
- `irq-exit-preempt-experimental` and `irq-exit-wrapper-experimental` feature
  flags exist for future IRQ-exit preemption work; disabled by default.
- Starvation in wrapper mode has been mitigated; full verification under
  sustained load is a Phase 5 item.
