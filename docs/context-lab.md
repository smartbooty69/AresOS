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
- demo tasks increment counters and call `yield_now()` cooperatively
- log output includes lines like:
  - `ContextLab A=..., B=...`

## Notes

- This mode is intended for scheduler experimentation.
- Default boot path remains unchanged when `context-lab` is not enabled.