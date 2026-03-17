# Scheduler Design (Phase 5)

AresOS Phase 5 uses a preemptive, round-robin context scheduler for kernel tasks.

## Core Policy

- Quantum: `SCHED_QUANTUM_TICKS = 5`
- Trigger: timer tick sets reschedule flag each quantum boundary
- Switch path: deferred preemption checkpoint in task loops (`preempt_if_irq_pending`)
- Selection: round-robin over runnable context tasks

## Context Switch Flow

1. Timer IRQ increments scheduler ticks (`on_timer_tick`)
2. Quantum expiry sets `NEED_RESCHEDULE` and IRQ preempt pending flag
3. Running task reaches checkpoint (`preempt_if_requested` / `preempt_if_irq_pending`)
4. Scheduler selects next runnable task pair
5. `switch_context` saves current CPU context and restores next

## Fairness & Telemetry

- Per-task metrics (`TaskMetrics`): switches, cpu ticks, preemption attempts/successes
- Kernel fairness counters: `KERNEL_TASK_1..4_COUNT`
- Runtime fairness monitor: `log_preemption_fairness`
- Fairness violation threshold: score > `1.10`

Fairness score is computed as:

$$
\text{fairness\_score} = \frac{\max(\text{task counters})}{\min(\text{task counters})}
$$

A score close to `1.0` indicates balanced scheduling.

## Observability

Phase 5 observability components:

- Global counters:
  - process creations / terminations
  - total preemptions
  - fairness violations
  - scheduler lock contention
- Event ring buffer (`EVENT_LOG_CAPACITY = 256`):
  - event type
  - tick timestamp
  - pid/task id
- Performance snapshot (`PerformanceCounters::read`) includes:
  - timer ticks
  - total preemptions
  - scheduler lock contention
  - fairness violations

## Public Scheduler API

Main public APIs in `task::scheduler`:

- `on_timer_tick()`
- `try_context_reschedule()`
- `preempt_if_requested()`
- `preempt_if_irq_pending()`
- `spawn_context_task(name, entry)`
- `spawn_kernel_tasks_phase5()`
- `stats()` and `context_stats()`
- `get_task_metrics(id)` and `get_all_task_metrics()`
- `scheduler_lock_contention()`

## Running

Preemptive mode:

```bash
cargo run -p kernel --features preemption
```

Integration validation:

```bash
cargo test -p kernel --test preemption_integration
```
