# Phase 38 Checklist: Demand-Zero Page Faults

## Scope

- [x] `#PF` handler delegates to `demand_paging`.
- [x] `map_demand_zero_page` for user growth region.
- [x] Emit `Phase38-DemandZero` boot smoke output.

## Validation

- [x] `cargo check -p kernel`
- [x] `cargo test -p kernel --features preemption --test preemption_integration`
- [x] `python scripts/phase38_demand_zero_check.py --timeout 120`

## Deferred

- [ ] File-backed demand read.
- [ ] SMP TLB shootdown.
