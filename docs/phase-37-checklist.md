# Phase 37 Checklist: Manifest-Discovered ELF Load

## Scope

- [x] Discover `elf64-image` manifests from storage.
- [x] Gated execution via `EXECUTION_ALLOWLIST` and `execute_manifest_elf_gated`.
- [x] Seed `/bin/tickprobe` fixture.
- [x] Emit `Phase37-ManifestElf` boot smoke output.

## Validation

- [x] `cargo check -p kernel`
- [x] `cargo test -p kernel --features preemption --test preemption_integration`
- [x] `python scripts/phase37_manifest_elf_check.py --timeout 120`

## Deferred

- [ ] Unsigned arbitrary ELF execution.
