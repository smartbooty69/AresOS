# Storage Design (Phase 7)

AresOS Phase 7 introduced a small persistent storage stack on top of a block-device boundary. Phase 8 mounts that filesystem through a managed block backend so the same filesystem API can run on driver-plumbed storage.

## Layers

```mermaid
flowchart TD
Shell[Shell Commands] --> StorageApi[storage API]
Syscalls[Storage Syscalls] --> StorageApi
Userspace[User Utilities] --> Syscalls
StorageApi --> SimpleFs[SimpleFs]
SimpleFs --> BlockDevice[BlockDevice]
BlockDevice --> MemoryBlockDevice[MemoryBlockDevice]
BlockDevice --> ManagedBlockDevice[ManagedBlockDevice]
ManagedBlockDevice --> BlockManager[Block Manager]
```

## Filesystem Format

- Sector size: 512 bytes
- Header sector: magic, version, file count
- Directory table: fixed-size entries
- File data: one sector per file
- Maximum files: 16
- Maximum file size: 512 bytes
- Maximum path length: 48 bytes

Each write updates file data and flushes the directory/header metadata to the backing block device. Remount validation proves data survives unmount/mount cycles on the same device instance.

## Runtime API

Primary kernel APIs live in `kernel/src/storage.rs`:

- `init()`
- `format()`
- `remount()`
- `list_files()`
- `read_file(path)`
- `create_file(path)`
- `write_file(path, contents)`
- `delete_file(path)`
- `info()`
- `phase7_smoke_check()`
- `phase8_smoke_check()`

## Shell Commands

- `ls`
- `cat <path>`
- `touch <path>`
- `write <path> <text>`
- `rm <path>`
- `mount`
- `format`
- `fsinfo`

## Validation

```bash
python scripts/phase7_storage_check.py --timeout 20
python scripts/validation_matrix.py --soak-duration 20 --latency-duration 20
```

The kernel emits:

```text
Phase7-Storage: mounted=true, persistent_rw_ok=true
```

## Phase 8 Backend

By default, runtime storage uses `ManagedBlockDevice`, which delegates sector I/O to the active block backend. Phase 8 registers `qemu-sim-block0` through the block manager as a deterministic driver-backed backend for QEMU validation.

`MemoryBlockDevice` remains available for focused filesystem tests.

## Deferred Work

- Real AHCI/NVMe/virtio block drivers
- FAT/ext-style filesystem compatibility
- Journaling and crash consistency
- File permissions and ownership
- Loading executable program images from storage
