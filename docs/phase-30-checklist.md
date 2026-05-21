# Phase 30 Checklist: Per-Process CR3 Switching

## Scope

- [x] Switch between distinct user CR3 values and restore kernel CR3.
- [x] Verify distinct translations after switches.
- [x] Emit `Phase30-Cr3Switch` boot smoke output.

## Validation

- [x] `cargo check -p kernel`
- [x] `cargo test -p kernel --test preemption_integration`
- [x] `python scripts/phase30_cr3_switch_check.py --timeout 120`

## Deferred

- [x] Scheduler-integrated CR3 switching on every context switch (Phase 31).
- [ ] Demand paging and SMP TLB shootdown (demand-zero slice in Phase 38).
