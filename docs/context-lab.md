# Context Lab Mode

`context-lab` is an isolated scheduler lab mode for exercising real context switches.

## Purpose

- run two dedicated context tasks (`ctx-demo-a`, `ctx-demo-b`)
- perform real `switch_context` handoffs
- observe per-task progress counters in logs

## Run

```bash
cargo run -p kernel --features context-lab
```

Run with low-level timer IRQ wrapper experiment:

```bash
cargo run -p kernel --features irq-exit-wrapper-experimental
```

## Behavior

- async executor tasks are skipped in lab mode
- boot enters context-task scheduler directly
- demo tasks increment counters and call `preempt_if_requested()`
- timer IRQ requests reschedule every scheduler quantum
- timer IRQ records interrupted RIP/RSP for scheduler telemetry
- context handoff occurs at deferred checkpoints when pending IRQ preemption is observed
- timer IRQ tail preempt hook records forced-preempt attempts/blocks (telemetry-only at this stage)
- optional low-level IRQ wrapper path can replace the timer interrupt entry/return sequence (experimental)
- log output includes lines like:
  - `ContextLab A=..., B=...`
  - `Preemptive-groundwork: ... misses=..., watchdog_trips=..., irq_req=..., irq_ckpt=..., irq_forced_attempts=..., irq_forced_blocked=...`

## Notes

- This mode is intended for scheduler experimentation.
- A watchdog panics if no context switch progress is observed for an extended tick window.
- Default boot path remains unchanged when `context-lab` is not enabled.