# Phase 9 Checklist (Stored Program Loader)

**Date**: 2026-05-13  
**Status**: Complete

## 1. Executable Manifest Format

- [x] `ares-exec-v1` text manifest format
- [x] `name`, `kind`, `entry`, and `description` fields
- [x] `builtin-alias` executable kind
- [x] Parser rejects invalid version, missing fields, invalid fields, and unsupported kinds

## 2. Program Registry and Discovery

- [x] `/bin/*` discovery through the storage API
- [x] Program metadata includes name, source path, kind, entry, and description
- [x] Default utilities seeded as executable manifests
- [x] Malformed `/bin` records are skipped without panics

## 3. File-Backed Run Path

- [x] `run_program()` resolves stored programs before dispatch
- [x] Built-in entry dispatch preserved for `echo`, `time`, `sysinfo`, and `fsinfo`
- [x] Launch success/failure accounting
- [x] Program launches create/terminate process records through existing process metadata

## 4. Shell, Syscalls, and Observability

- [x] Shell commands: `programs`, `bin list`, `bin info <program>`
- [x] Program count, launch count, and failed launch count syscalls
- [x] `fsinfo` reports program count
- [x] Boot-time `Phase9-Loader` smoke line

## 5. Validation

- [x] `scripts/phase9-loader-check` for QEMU-backed validation
- [x] `scripts/validation_matrix.py` includes `phase9-loader-check`
- [x] Integration tests cover parser, discovery, run path, malformed files, and loader syscalls

## Validation Commands

```bash
cargo check -p kernel
cargo test -p kernel --test preemption_integration
python scripts/phase9_loader_check.py --timeout 20
python scripts/validation_matrix.py --soak-duration 20 --latency-duration 20
```

## Known Limits

- Phase 9 manifests map stored program files to existing built-in entry targets.
- Real ELF parsing, relocation, paging isolation, and raw binary execution are deferred.
- Program permissions, signatures, ownership, and executable memory protections are deferred.
