# AresOS ⚔️

<p align="center">
	<img src="logo.png" alt="AresOS logo" width="420" />
</p>

**AresOS** is an experimental operating system written in **Rust**, built from the ground up to explore modern kernel architecture, low-level hardware control, and safe systems programming.

Named after Ares, the project represents **strength, control, and raw system power** — the philosophy that a developer should fully understand and command the machine they use.

AresOS is both a **learning platform and a long-term experimental system**, focused on transparency, performance, and deep system knowledge.

---

# Philosophy

AresOS follows a simple belief:

> The best way to understand a computer is to build the system that runs it.

Modern operating systems hide enormous complexity behind layers of abstraction. AresOS instead embraces that complexity and exposes how systems truly work.

The project focuses on:

* **Understanding the machine**
* **Writing software close to the hardware**
* **Designing systems intentionally rather than inheriting legacy design**

Rust provides the safety guarantees needed to build such a system without sacrificing performance.

---

# Inspiration

AresOS draws inspiration from several legendary operating system projects.

One of the strongest influences is TempleOS, created entirely by Terry A. Davis.

TempleOS demonstrated what a single determined developer could achieve by building a complete operating system from scratch. Its bold philosophy and uncompromising approach to system design helped inspire many modern hobby OS projects.

While AresOS follows a different technical path—using Rust and modern system architecture—it shares the same spirit of **deep curiosity, independence, and exploration of computing at the lowest level**.

Other inspirations include:

* Linux
* Redox OS
* Minix

---

# Goals

AresOS aims to become a small but powerful experimental operating system that demonstrates:

* modern kernel design
* memory-safe systems programming
* transparent system behavior
* efficient hardware interaction

The project also serves as a **long-term exploration of operating system engineering**.

---

# Planned Features

### Kernel Core

* Rust bare-metal kernel
* interrupt handling
* memory management
* virtual memory and paging

### Hardware Interaction

* keyboard input
* timer interrupts
* device driver framework

### System Architecture

* modular kernel design
* multitasking scheduler
* kernel logging and debugging

### Storage

* filesystem support
* disk drivers
* persistent storage

### User Environment

* terminal shell
* system utilities
* process management tools

---

# Roadmap

### Phase 1 — Boot

* freestanding Rust kernel
* bootloader integration
* basic screen output

Status: ✅ Complete (validated 2026-03-17)

Checklist: `docs/phase-1-checklist.md`

### Phase 2 — Hardware

* interrupt descriptor table
* keyboard driver
* timer interrupts

Status: ✅ Complete (validated 2026-03-17)

Checklist: `docs/phase-2-checklist.md`

### Phase 3 — Memory

* paging implementation
* frame allocator
* heap allocation

Status: ✅ Complete (validated 2026-03-17)

Checklist: `docs/phase-3-checklist.md`

### Phase 4 — Processes

* multitasking scheduler
* context switching
* task management

Status: ✅ Complete (validated 2026-03-17, cooperative async; context switching in `context-lab` mode)

Checklist: `docs/phase-4-checklist.md`

### Phase 5 — Preemptive Scheduling & Process Foundation

* preemptive scheduler mode (`preemption` feature)
* process abstraction + PID allocator
* fairness telemetry and preemption observability

Status: ✅ Complete (validated 2026-05-06)

Checklist: `docs/phase-5-checklist.md`

Scheduler deep dive: `docs/SCHEDULER.md`

### Phase 6 — User Space

* command shell
* system utilities
* basic programs

Status: ✅ Complete (validated 2026-05-06; shell + utilities + syscall/storage baseline)

Checklist: `docs/phase-6-checklist.md`

### Phase 7 — Persistent Storage

* block-device storage boundary
* simple persistent filesystem format
* shell and syscall file operations

Status: ✅ Complete (validated 2026-05-13; remount persistence + QEMU storage smoke)

Checklist: `docs/phase-7-checklist.md`

Storage deep dive: `docs/STORAGE.md`

### Phase 8 — Device & Block Driver Bring-Up

* device registry and PCI discovery skeleton
* block-device manager
* QEMU-friendly driver-backed storage path

Status: ✅ Complete (validated 2026-05-13; device/block smoke + storage-through-manager)

Checklist: `docs/phase-8-checklist.md`

Device deep dive: `docs/DEVICES.md`

### Phase 9 — Stored Program Loader

* executable manifest format
* `/bin/*` program discovery
* file-backed launch path for built-in program entries

Status: ✅ Complete (validated 2026-05-13; stored manifests + loader smoke)

Checklist: `docs/phase-9-checklist.md`

Program loader deep dive: `docs/PROGRAMS.md`

### Phase 10 — Permissions & Process Isolation Groundwork

* static users, roles, and credential model
* file owner/mode metadata with checked shell/syscall operations
* executable trust fields and process ownership policy

Status: ✅ Complete (validated 2026-05-13; permission denial + process ownership smoke)

Checklist: `docs/phase-10-checklist.md`

Security deep dive: `docs/SECURITY.md`

### Phase 11 — Executable Image & Address-Space Groundwork

* conservative ELF64 image validation
* descriptor-only address-space and virtual-region model
* image manifest discovery without unsafe binary execution

Status: ✅ Complete (validated 2026-05-13; image validation + unsupported execution smoke)

Checklist: `docs/phase-11-checklist.md`

Executable image deep dive: `docs/EXECUTABLE_IMAGES.md`

### Phase 12 — Executable Load Plans & Mapping Groundwork

* page-aligned executable load plans
* copy and zero-fill action accounting
* frame/page reservation metadata without page-table mutation

Status: ✅ Complete (validated 2026-05-13; load-plan preparation + execution-block smoke)

Checklist: `docs/phase-12-checklist.md`

Load-plan deep dive: `docs/LOAD_PLANS.md`

### Phase 13 — Frame-Backed Mapping Stubs

* deterministic mapping-stub records for prepared load plans
* frame-token, copy-byte, and zero-fill accounting
* mapped-stub process metadata without executable scheduling

Status: ✅ Complete (validated 2026-05-13; mapping-stub smoke + execution-block preservation)

Checklist: `docs/phase-13-checklist.md`

Mapping-stub deep dive: `docs/MAPPING_STUBS.md`

### Phase 14 — Frame Ownership Service

* persistent frame ownership registry
* bounded physical-frame accounting after heap initialization
* frame allocation/release counters for future executable backing

Status: ✅ Complete (validated 2026-05-13; frame ownership smoke)

Checklist: `docs/phase-14-checklist.md`

Frame ownership deep dive: `docs/FRAME_OWNERSHIP.md`

### Phase 15 — Real Backing Frames For Load Plans

* frame-backed image records for mapped executable pages
* owned-frame consumption from the Phase 14 registry
* copy and zero-fill accounting attached to backed pages

Status: ✅ Complete (validated 2026-05-13; frame-backed image smoke)

Checklist: `docs/phase-15-checklist.md`

Frame-backed image deep dive: `docs/FRAME_BACKED_IMAGES.md`

### Phase 16 — Inactive User Page Tables

* inactive user page-table descriptors for frame-backed images
* virtual-to-physical translation validation
* blocked `PageTableReady` process metadata without CR3 switching

Status: ✅ Complete (validated 2026-05-13; inactive page-table smoke)

Checklist: `docs/phase-16-checklist.md`

User page-table deep dive: `docs/USER_PAGE_TABLES.md`

### Phase 17 — User Context And Entry Frames

* GDT user code/data selectors
* initial user entry frame and stack descriptors
* blocked `UserContextReady` process metadata without Ring 3 entry

Status: ✅ Complete (validated 2026-05-13; user-context smoke)

Checklist: `docs/phase-17-checklist.md`

User context deep dive: `docs/USER_CONTEXT.md`

### Phase 18 — Controlled Ring 3 Trampoline

* controlled user-entry/trap result records
* reserved user trap vector metadata
* blocked `UserTrapped` process metadata

Status: ✅ Complete (validated 2026-05-13; controlled Ring 3 trampoline smoke)

Checklist: `docs/phase-18-checklist.md`

Ring 3 trampoline deep dive: `docs/RING3_TRAMPOLINE.md`

### Phase 19 — Syscall Entry And Return ABI

* user syscall register-frame ABI
* syscall dispatch return metadata
* blocked `UserSyscallReturned` process metadata

Status: ✅ Complete (validated 2026-05-13; syscall return smoke)

Checklist: `docs/phase-19-checklist.md`

User syscall deep dive: `docs/USER_SYSCALLS.md`

### Phase 20 — Minimal ELF Execution MVP

* guarded `/bin/hello` ELF execution path
* deterministic output and exit status for `run hello`
* blocked `UserElfExited` process metadata

Status: ✅ Complete (validated 2026-05-13; user ELF smoke)

Checklist: `docs/phase-20-checklist.md`

User ELF MVP deep dive: `docs/USER_ELF_MVP.md`

---

# Project Structure

```
AresOS
├── Cargo.toml                 workspace manifest
├── kernel/
│   ├── Cargo.toml             kernel crate manifest
│   ├── x86_64-unknown-none.json
│   ├── src/
│   │   ├── main.rs            kernel entry point
│   │   ├── device.rs          device registry + PCI discovery skeleton
│   │   ├── ring3_trampoline.rs controlled user-entry trap records
│   │   ├── block.rs           block-device manager
│   │   ├── security.rs        identity + permission policy primitives
│   │   ├── exec_image.rs      executable image parser and validation
│   │   ├── frame_backing.rs   frame-backed executable image records
│   │   ├── frame_ownership.rs persistent frame ownership bookkeeping
│   │   ├── address_space.rs   descriptor-only process address spaces
│   │   ├── load_plan.rs       executable load-plan accounting
│   │   ├── mapping_stub.rs    deterministic executable mapping stubs
│   │   ├── user_context.rs    user entry-frame descriptors
│   │   ├── user_memory.rs     inactive user page-table descriptors
│   │   ├── user_syscall.rs    user syscall ABI descriptors
│   │   ├── lib.rs             shared kernel modules
│   │   ├── interrupts.rs      IDT + IRQ handlers
│   │   ├── memory.rs          paging + frame allocator
│   │   ├── allocator.rs       heap allocator
│   │   ├── task/              async executor + keyboard
│   │   └── performance/       metrics + profiler
│   └── tests/                 boot/integration tests
└── .cargo/config.toml         target + runner configuration
```

---

# Building

Install dependencies:

```
rustup component add llvm-tools-preview
cargo install bootimage
rustup component add rust-src
```

Install QEMU (example on Ubuntu/Debian):

```
sudo apt install qemu-system-x86
```

Install QEMU on Windows (winget):

```
winget install --id SoftwareFreedomConservancy.QEMU --accept-package-agreements --accept-source-agreements
```

Build the OS:

```
cargo build -p kernel
```

---

# Running

Run AresOS using QEMU:

```
cargo run -p kernel
```

Run Phase 5 preemption mode:

```
cargo run -p kernel --features preemption
```

Phase 5 integration checks:

```
cargo test -p kernel --test preemption_integration
```

Phase 5 soak check (fairness/progress):

```
./scripts/phase5-soak-check --duration 120 --min-samples 3
```

Phase 5 latency check (<100ms estimated preemption latency):

```
./scripts/phase5-latency-check --duration 120 --min-samples 5 --max-latency-ms 100
```

Phase 6 smoke check:

```
./scripts/phase6-smoke-check
```

Phase 7 persistent storage check:

```
./scripts/phase7-storage-check --timeout 20
```

Phase 8 device/block check:

```
./scripts/phase8-device-check --timeout 20
```

Phase 9 stored program loader check:

```
./scripts/phase9-loader-check --timeout 20
```

Phase 10 security policy check:

```
./scripts/phase10-security-check --timeout 20
```

Phase 11 executable image check:

```
./scripts/phase11-image-check --timeout 20
```

Phase 12 executable load-plan check:

```
./scripts/phase12-load-plan-check --timeout 20
```

Phase 13 mapping-stub check:

```
./scripts/phase13-mapping-stub-check --timeout 20
```

Phase 14 frame ownership check:

```
./scripts/phase14-frame-check --timeout 20
```

Phase 15 frame-backed image check:

```
./scripts/phase15-frame-backing-check --timeout 20
```

Phase 16 inactive page-table check:

```
./scripts/phase16-page-table-check --timeout 20
```

Phase 17 user-context check:

```
./scripts/phase17-user-context-check --timeout 20
```

Phase 18 controlled Ring 3 check:

```
./scripts/phase18-ring3-check --timeout 20
```

Phase 19 syscall return check:

```
./scripts/phase19-syscall-return-check --timeout 20
```

Phase 20 user ELF check:

```
./scripts/phase20-user-elf-check --timeout 20
```

Full validation matrix (QEMU-backed):

```
python scripts/validation_matrix.py --soak-duration 20 --latency-duration 20
```

Run tests (unit + integration under QEMU):

```
cargo test -p kernel
```

Run Phase 4 wrapper-mode preemption soak check:

```
./scripts/phase4-soak-check
```

---

# Vision

AresOS is not intended to replace existing operating systems.

Instead, it exists to answer a question:

**What happens when you build a system entirely on your own terms?**

---

# License

Licensed under the Apache License, Version 2.0.

See [LICENSE](LICENSE) for the full text.



