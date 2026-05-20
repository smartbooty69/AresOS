# Phase 7 Checklist (Persistent Storage Bring-Up)

**Date**: 2026-05-13  
**Status**: Complete

## 1. Storage Architecture Boundary

- [x] `BlockDevice` trait for sector-oriented storage
- [x] `MemoryBlockDevice` implementation for boot/runtime validation
- [x] `SimpleFs` path-level filesystem abstraction
- [x] Expanded `StorageError` coverage for expected failure modes

## 2. Simple Persistent Filesystem

- [x] Fixed magic/version filesystem header
- [x] Fixed-size directory table
- [x] One-sector file data slots
- [x] Format, mount, unmount, and remount support
- [x] Create, read, overwrite, delete, list, and fs status operations
- [x] Persistence validated across remount on the same device image

## 3. Shell & Syscall Surface

- [x] Shell commands: `touch`, `write`, `rm`, `mount`, `format`, `fsinfo`
- [x] Existing `ls` and `cat` commands backed by `SimpleFs`
- [x] Storage status syscalls
- [x] Storage list/read/write/delete syscall wrappers with negative coverage
- [x] `fsinfo` user utility reports filesystem status through syscalls

## 4. Validation

- [x] Boot-time `Phase7-Storage` smoke line
- [x] `scripts/phase7-storage-check` for QEMU-backed validation
- [x] `scripts/validation_matrix.py` includes `phase7-storage-check`
- [x] Integration tests cover remount persistence and syscall file lifecycle

## Validation Commands

```bash
cargo check -p kernel
cargo test -p kernel --test preemption_integration
python scripts/phase7_storage_check.py --timeout 120
python scripts/validation_matrix.py --soak-duration 20 --latency-duration 20
```

## Known Limits

- Files are limited to one 512-byte sector each.
- The filesystem supports at most 16 files.
- Storage is backed by an in-memory block device until a hardware disk driver exists.
- There is no journaling, permissions model, or crash recovery.
