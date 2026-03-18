# Phase 5 Checklist (Preemptive Scheduling & Process Foundation)

**Date**: 2026-03-17  
**Status**: Complete (core Phase 5 scope) ✅

## Scope

Phase 5 builds on Phase 4's context switching infrastructure to establish:
- Full preemptive scheduling as the default kernel mode
- Process abstraction and isolation groundwork
- Multi-task fairness and observability across 4+ concurrent tasks
- System-wide preemption policy (quantum-based + fairness)
- Foundation for user-mode process execution

## Strategic Goals

1. **Stabilize Preemption**: Graduate from experimental `context-lab` to production preemptive scheduler ✅
2. **Process Isolation**: Introduce `Process` abstraction with isolated address spaces
3. **Multi-task Fairness**: Extend round-robin scheduling beyond 2 demo tasks ✅
4. **Observability**: Real-time preemption metrics and kernel telemetry
5. **System Calls**: Foundation for user-mode interaction (prep for Phase 6)

## Completion Criteria

### 1. Preemptive Scheduler Core
- [x] Consolidate `TaskScheduler` for production use (migrate out of experimental flags)
- [x] Replace `context-lab` feature flag with `preemption` (non-experimental)
- [x] Establish scheduler quantum configuration (default SCHED_QUANTUM_TICKS = 5)
- [x] Per-task time-slice accounting and preemption tracking (TaskMetrics struct)
- [x] CPU affinity tags (single-core prep for multi-core)

### 2. Process Abstraction
- [x] `Process` struct with lifecycle/metrics metadata (kernel stack isolation pending)
- [x] Process ID (`PID`) allocation and lifecycle management
- [x] Process state machine: `New` → `Ready` → `Running` → `Blocked` → `Terminated`
- [x] Process registry (kernel-global process table, max 256 processes)
- [x] Exit code handling and reaping strategy

### 3. Multi-task & Fairness
- [x] Scheduler supports 4+ concurrent kernel tasks simultaneously (kernel_task_1..4)
- [x] Round-robin queue with fair time-slice distribution (existing scheduler supports N tasks)
- [x] Preemption counters per task (TaskMetrics: switches, preemption_successes)
- [x] Starvation detection and logging for imbalanced workloads
- [x] Fairness metrics exported to performance counters

### 4. Observability & Telemetry
- [x] Process telemetry module (`kernel::performance::process_metrics`)
  - per-process context switches ✅ (in TaskMetrics)
  - per-process CPU time (in ticks) ✅ (in `Process` + fairness snapshots)
  - per-process preemption attempts/successes ✅ (in TaskMetrics)
- [x] Kernel event log (in-memory ring buffer, ~10KB)
  - timestamps, event type, process/task ID
  - e.g. "PREEMPT: PID 2 forced after 10 ticks"
- [x] `PerformanceCounters` extended with:
  - total preemptions
  - scheduler lock contention
  - fairness violations (task exceeding quantum)

### 5. Scheduler Configuration & Tuning
- [x] Runtime configuration support for scheduler parameters (API-based)
  - `SCHEDULER_QUANTUM_TICKS` (default 5)
  - `MAX_PROCESSES` (default 256)
  - `FAIRNESS_CHECK_INTERVAL_TICKS` (from Phase 4, verify still working)
- [ ] Runtime parameter adjustment via kernel console (future work)

### 6. Integration & Testing
- [x] Spawn 4 independent long-running kernel tasks
- [x] Each task increments its own counter and periodically yields
- [x] Verify all 4 tasks advance roughly equally (fairness test)
- [ ] No single task starves others; preemption latency < 100ms (deferred to soak/hardening)
- [ ] 10-minute soak test: no stalls, all metrics advance
- [x] Extend `tests/` with preemption-specific integration tests
  - `fairness_test.rs` - 4 tasks, measure quanta distribution
  - `preemption_latency_test.rs` - measure switch delay
  - `process_isolation_test.rs` - verify process table operations
  - `scripts/phase5-soak-check` - runtime fairness/progress soak checker

### 7. Documentation & Examples
- [x] README: section on preemption, quantum tuning, fairness guarantees
- [x] `SCHEDULER.md`: deep-dive on scheduler design, round-robin policy
- [x] Example: concurrent task fairness demo (built-in, similar to context-lab)
- [x] Scheduler API documentation (`task::scheduler` public interface)

### 8. Clean-up & Consolidation
- [x] Remove `context-lab` feature flag (replaced by `preemption`)
- [x] Remove `irq-exit-preempt-experimental` and `irq-exit-wrapper-experimental`
- [x] Consolidate IRQ preemption path (single, stable code path)
- [x] No compiler warnings in preemptive builds

## Deferred Hardening (Phase 6 Candidate)

- Runtime scheduler parameterization (CLI/console)
- Long-duration (10-minute) preemption soak + latency SLA validation

## Implementation Order (Recommended)

1. **Stabilize preemption** (days 1–2)
   - Consolidate `TaskScheduler` from experimental
   - Extend time-slice tracking
   - Add per-task preemption counters

2. **Process abstraction** (days 3–4)
   - Define `Process` struct and `PID` allocator
   - Implement process registry kernel-globally
   - Process state machine

3. **Multi-task support** (days 4–5)
   - Spawn 4+ kernel tasks in main
   - Test round-robin fairness
   - Add fairness metrics

4. **Observability** (days 5–6)
   - Event log ring buffer
   - Process telemetry module
   - Export metrics

5. **Testing & soak** (days 6–7)
   - Write integration tests
   - 10-minute soak under load
   - Validate metrics convergence

6. **Documentation** (days 7–8)
   - Update README, add SCHEDULER.md
   - Example code and tuning guide

## Exit Gate

**Preemptive Kernel Soak Test**:
```bash
cargo run -p kernel --features preemption
# (built-in 10-minute demo, prints metrics every 1s)
```

Pass criteria:
- All 4+ tasks advance without stall
- Context switches occur every ~10 ticks (quantum boundary)
- Fairness score (ratio of max/min CPU time) ≤ 1.05 (within 5%)
- No scheduler deadlock (watchdog passes)
- All tests pass: `cargo test -p kernel --features preemption`

## Risks & Mitigation

| Risk | Mitigation |
|------|-----------|
| Scheduler lock contention under high load | Profile & optimize lock hold times; consider lock-free structures if needed |
| Timer IRQ starvation in preemptive mode | Validate interrupt handling under sustained preemption; fallback strategy for missed ticks |
| Process table overflow | Implement LRU eviction or panic with telemetry |
| Fairness violations (one task starves others) | Aggressive preemption on quantum expiry; watchdog detection |

## Notes

- Single-core focus; multi-core support deferred to Phase 6
- User-mode processes & address spaces deferred to Phase 6 (only kernel tasks in Phase 5)
- System calls API design begins in Phase 5; implementation in Phase 6
- Phase 4's `context-lab` feature is the foundation; replaces it with stable `preemption` flag
- **Day 1 Completed**: 
  - ✅ Added `preemption` feature flag (non-experimental)
  - ✅ Per-task metrics tracking in `TaskMetrics` struct
  - ✅ Spawned 4 independent kernel tasks (kernel_task_1..4)
  - ✅ Counter tracking for fairness testing (`KERNEL_TASK_*_COUNT`)
  - ✅ Logging infrastructure for fairness monitoring
  - ✅ All Phase 4 tests passing (27/27)

## Related PRs / Commits

- Day 1: Stabilize preemption and add per-task metrics (in progress)

## Validation Snapshot

- All Phase 4 tests must continue passing ✅ (27 tests passing)
- New soak test baseline: *pending (multi-task fairness test ready to run)*
- Performance baseline: *pending*

---

**Next Step**: Build Process abstraction and observability module.
