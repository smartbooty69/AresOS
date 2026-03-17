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

## Behavior

- async executor tasks are skipped in lab mode
- boot enters context-task scheduler directly
- demo tasks increment counters and call `preempt_if_requested()`
- timer IRQ requests reschedule every scheduler quantum
- context handoff occurs when a pending preemption request is observed
- log output includes lines like:
  - `ContextLab A=..., B=...`
  - `Preemptive-groundwork: ... misses=..., watchdog_trips=...`

## Notes

- This mode is intended for scheduler experimentation.
- A watchdog panics if no context switch progress is observed for an extended tick window.
- Default boot path remains unchanged when `context-lab` is not enabled.