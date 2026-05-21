# Phase 32 Checklist: User Trap Frame Persistence

## Scope

- [x] `UserHwFrame` save/resume registry for scheduler preemption bring-up.
- [x] Smoke saves frame, yields scheduler, resumes saved frame.
- [x] Emit `Phase32-UserFrame` boot smoke output.

## Validation

- [x] `cargo check -p kernel`
- [x] `cargo test -p kernel --features preemption --test preemption_integration`
- [x] `python scripts/phase32_user_frame_check.py --timeout 120`

## Deferred

- [ ] Full Ring 3 GPR save on timer interrupt.
- [ ] FPU/SSE state.
