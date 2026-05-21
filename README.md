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

### Phase 21 — Hardware User Page Tables

* real x86_64 page tables from inactive descriptors
* descriptor vs hardware translation verification
* blocked `HwPageTableReady` process metadata

Checklist: `docs/phase-21-checklist.md`

### Phase 22 — Controlled CR3 Activation

* activate and restore user CR3 without execution
* translation verification under switched page tables
* blocked `Cr3Activated` process metadata

Checklist: `docs/phase-22-checklist.md`

### Phase 23 — Real iretq User Entry

* CPU Ring 3 entry via `iretq` to a controlled stub
* return through invalid-opcode trap during bring-up
* blocked `UserEnteredHw` process metadata

Checklist: `docs/phase-23-checklist.md`

### Phase 24 — Hardware User Trap Return

* IDT vector `0x80` handler for cooperative user return
* blocked `UserHwTrapped` process metadata

Checklist: `docs/phase-24-checklist.md`

### Phase 25 — CPU syscall / sysret Path

* `syscall`/`sysret` MSRs and entry stub
* hardware tick-probe syscall path
* blocked `UserHwSyscallReturned` process metadata

Checklist: `docs/phase-25-checklist.md`

### Phase 26 — Validated User Copyin

* bounded `copy_from_user` / `copy_to_user`
* copy-probe syscall round-trip

Checklist: `docs/phase-26-checklist.md`

### Phase 27 — Static ELF Relocations

* `R_X86_64_RELATIVE` / `R_X86_64_64` for seeded images
* relocation accounting during frame backing

Checklist: `docs/phase-27-checklist.md`

### Phase 28 — Hardware Hello Execution

* `run hello` through hardware Ring 3 + syscall path
* blocked `UserHwElfExited` process metadata

Checklist: `docs/phase-28-checklist.md`

### Phase 29 — Allowlisted ELF Programs

* allowlisted `hello` and `exit42` ELF programs
* seeded manifests and images

Checklist: `docs/phase-29-checklist.md`

### Phase 30 — Per-Process CR3 Switching

* save/restore distinct user CR3 values
* isolation verification across switches

Checklist: `docs/phase-30-checklist.md`

### Phase 31 — Scheduler CR3 Binding

* CR3 binding on process records and preemptive context switch
* `Phase31-SchedCr3` boot smoke

Checklist: `docs/phase-31-checklist.md`

### Phase 32 — User Trap Frame Persistence

* saved `UserHwFrame` across scheduler yield
* `Phase32-UserFrame` boot smoke

Checklist: `docs/phase-32-checklist.md`

### Phase 33 — Concurrent Allowlisted ELFs

* `hello` and `exit42` under distinct hardware page tables
* `Phase33-MultiElf` boot smoke

Checklist: `docs/phase-33-checklist.md`

### Phase 34 — Exit and Wait Syscalls

* `ExitProcess` / `WaitProcess` syscalls
* `Phase34-ExitWait` boot smoke

Checklist: `docs/phase-34-checklist.md`

### Phase 35 — Hardware Syscall Dispatch Table

* allowlisted hardware syscall IDs
* `Phase35-SyscallTable` boot smoke

Checklist: `docs/phase-35-checklist.md`

### Phase 36 — Storage Syscalls With Copyin

* storage probe syscalls with validated user copies
* `Phase36-StorageCopyin` boot smoke

Checklist: `docs/phase-36-checklist.md`

### Phase 37 — Manifest-Discovered ELF Load

* discover `elf64-image` manifests; gated execution including `tickprobe`
* `Phase37-ManifestElf` boot smoke

Checklist: `docs/phase-37-checklist.md`

### Phase 38 — Demand-Zero Page Growth

* user `#PF` handler and demand-zero mapping
* `Phase38-DemandZero` boot smoke

Checklist: `docs/phase-38-checklist.md`

### Phase 39 — Dynamic Linking Groundwork

* `DT_NEEDED` detection for ARES seed ELFs
* `Phase39-Dynamic` boot smoke

Checklist: `docs/phase-39-checklist.md`

### Phase 40 — Integration Milestone

* end-to-end validation of phases 31–39
* `Phase40-Integration` boot smoke

Checklist: `docs/phase-40-checklist.md`

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

Phase 21 hardware page-table check:

```
python scripts/phase21_hw_page_table_check.py --timeout 20
```

Phase 22 CR3 activation check:

```
python scripts/phase22_cr3_check.py --timeout 20
```

Phase 23 iretq entry check:

```
python scripts/phase23_iretq_check.py --timeout 20
```

Phase 24 user trap check:

```
python scripts/phase24_user_trap_check.py --timeout 20
```

Phase 25 hardware syscall check:

```
python scripts/phase25_syscall_hw_check.py --timeout 20
```

Phase 26 user copyin check:

```
python scripts/phase26_copyin_check.py --timeout 20
```

Phase 27 relocation check:

```
python scripts/phase27_reloc_check.py --timeout 20
```

Phase 28 hardware hello check:

```
python scripts/phase28_hw_hello_check.py --timeout 20
```

Phase 29 allowlist check:

```
python scripts/phase29_allowlist_check.py --timeout 20
```

Phase 30 CR3 switch check:

```
python scripts/phase30_cr3_switch_check.py --timeout 20
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



