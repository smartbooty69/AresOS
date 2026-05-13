# Phase 8 Checklist (Device & Block Driver Bring-Up)

**Date**: 2026-05-13  
**Status**: Complete

## 1. Device Layer Foundation

- [x] `DeviceId`, `DeviceKind`, `DeviceState`, `DeviceInfo`, and `DeviceError`
- [x] Global device registry
- [x] Device register/list/query helpers
- [x] Device summary counts for runtime diagnostics

## 2. PCI Discovery Skeleton

- [x] QEMU-safe PCI config-space scanner
- [x] Vendor/device/class/subclass discovery
- [x] PCI devices registered into the device registry
- [x] Empty-scan fallback represented without panic

## 3. Block Device Manager

- [x] `BlockDeviceId`, backend metadata, and block registry
- [x] Active block-device selection
- [x] Sector read/write through active backend
- [x] Storage reports active backend and driver-backed status

## 4. QEMU-Friendly Backend

- [x] Simulated QEMU-style driver-backed block backend
- [x] Phase 7 `SimpleFs` mounted through managed block-device backend
- [x] Read/write/remount smoke check through driver-backed path

## 5. Shell, Syscalls, and Observability

- [x] Shell commands: `devices`, `blk list`, `blk info <id>`, `mount <block-id>`
- [x] Device/block count syscalls
- [x] `fsinfo` reports block-device count
- [x] Boot-time `Phase8-Devices` smoke line

## 6. Validation

- [x] `scripts/phase8-device-check` for QEMU-backed validation
- [x] `scripts/validation_matrix.py` includes `phase8-device-check`
- [x] Integration tests cover device registry, block registry, and storage-through-manager behavior

## Validation Commands

```bash
cargo check -p kernel
cargo test -p kernel --test preemption_integration
python scripts/phase8_device_check.py --timeout 20
python scripts/validation_matrix.py --soak-duration 20 --latency-duration 20
```

## Known Limits

- The shipped block backend is simulated and driver-plumbed, not a full AHCI/NVMe/virtio implementation.
- PCI scanning is read-only enumeration for observability and future driver binding.
- Block I/O is synchronous and polling-style.
- DMA, MSI/MSI-X, interrupt-driven I/O, and production hardware drivers are deferred.
