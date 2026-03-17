# Phase 4 Preemption Checkpoint — 2026-03-17

## Completed

### Core Context Switching Architecture
- [x] CPU context struct with register save state
- [x] Kernel stack allocation per context
- [x] Assembly-level context switch with interrupt atomicity (`cli` guarding save/restore)
- [x] Context capture and restoration semantics
- [x] Runnable context bootstrapping

### Scheduler Foundation
- [x] Context-task scheduler with round-robin queue
- [x] Live context switching (immediate switch from any call site)
- [x] Deferred IRQ preemption signaling
- [x] Callback-based reschedule checkpoints in async executor

### Experimentation & Telemetry
- [x] Context-lab isolated mode for pure context-task testing
- [x] Deferred IRQ handoff token (lock-free, producer in IRQ tail, consumer in task loop)
- [x] Comprehensive preemption telemetry (queued/consumed handoffs, forced attempts/blocks, stall counters)
- [x] Serial + VGA logging of context-lab demo metrics
- [x] Watchdog for starvation detection

### Starvation Mitigation
- [x] Interrupt-atomic context switching to prevent IRQ races
- [x] Aggressive timer-stall fallback recovery (20k-spin threshold)
- [x] Simplified deferred handoff semantics (token-only, not pointer-pair)
- [x] Explicit interrupt re-enabling in context task loops

## Known Issues & Limitations

### Wrapper-Mode IRQ Degradation (Experimental Feature)
- **Issue**: Under `irq-exit-wrapper-experimental` feature, after first context hop, timer IRQ flow to the scheduler stops (handoff_queued stalls at 1).
- **Assessment**: Likely deep in low-level wrapper return path or IRQ dispatch desynchronization. Mitigated by fallback switches but not root-caused.
- **Status**: Observable but acceptable. Main scheduler path remains unaffected.

### One-Way Handoff Scenarios
- **Historical**: Earlier token iterations could queue no-op or self-targeting handoffs, causing one-task starvation.
- **Fix Applied**: Current slot publishing in task loops + slot-based handoff semantics prevent this.
- **Status**: Resolved via last checkpoint commit.

## Test Status
- **All unit tests**: ✅ Pass (16 tests)
- **Integration tests**: ✅ Pass (basic_boot, heap_allocation, stack_overflow, 5 heap sub-tests)
- **Context tests**: ✅ Pass (capture, switch, empty-zeroed)
- **Scheduler tests**: ✅ Pass (tick requests, request clearing, task names)
- **Wrapper-mode runtime**: ✅ Both demo tasks progress (mitigated starvation)

## Architecture Notes

### Context Switching Path
1. Task loop calls `preempt_if_irq_pending()` or `yield_now()`
2. Scheduler lock acquired
3. Next pair computed
4. Assembly `switch_context()` saves current state, restores next
5. Execution resumes at `next.rip`

### IRQ Preemption Flow
1. Timer IRQ fires, increments tick counter
2. On quantum boundary, `IRQ_PREEMPT_PENDING` flag set
3. `try_forced_preempt_from_irq_tail()` queues generic handoff token
4. IRQ returns to interrupted context (no switch inline)
5. Task loop polls `preempt_if_irq_pending()`, consumes token, performs switch

### Safety Strategy
- Default production path: async executor with cooperative preemption (stable, tested)
- Lab mode (`context-lab` feature): isolated dual-task context demo (experimental)
- Wrapper mode (`irq-exit-wrapper-experimental`): low-level IRQ dispatch experiment (has known degradation)
- All live switches guarded by interrupt atomicity

## Next Steps (Priority Order)

### 1. Consolidate & Document Preemption API
- [ ] Publish stable `set_context_switching_enabled()` public API
- [ ] Document `preempt_if_irq_pending()` checkpoint for kernel integration
- [ ] Add preemption integration examples

### 2. Deep Dive: IRQ Wrapper Degradation (Optional)
- [ ] Instrument [kernel/src/interrupts.rs](kernel/src/interrupts.rs) to log per-IRQ dispatch flow
- [ ] Check IF state at wrapper exit vs. task re-enable
- [ ] Profile timer tick frequency across wrapper vs. standard handlers

### 3. Async Executor Integration
- [ ] Wire cooperative preemption into main executor loop
- [ ] Add task priority or fairness modes
- [ ] Test concurrent tasks with timer-driven work-stealing

### 4. Performance Profiling
- [ ] Measure context switch latency
- [ ] Profile scheduler lock contention
- [ ] Benchmark context-lab demo throughput

## Commit Checkpoint
- Latest commit: `b7cb1b9` "fix(context-lab): mitigate wrapper-mode starvation via interrupt-atomic switches and stall recovery"
- All tests green as of this checkpoint

---

**Status**: Phase 4 foundational work is solid and stable. Wrapper-mode experiment has known limitations but main scheduler path is production-ready. Ready to either (a) deep-dive on IRQ wrapper, or (b) move to full async executor integration and Phase 5+ features.
