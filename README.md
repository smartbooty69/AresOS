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

### Phase 2 — Hardware

* interrupt descriptor table
* keyboard driver
* timer interrupts

### Phase 3 — Memory

* paging implementation
* frame allocator
* heap allocation

### Phase 4 — Processes

* multitasking scheduler
* context switching
* task management

### Phase 5 — Storage

* filesystem support
* disk access drivers

### Phase 6 — User Space

* command shell
* system utilities
* basic programs

---

# Project Structure

```
aresos
│
├── kernel        core kernel code
├── bootloader    bootloader configuration
├── drivers       hardware drivers
├── memory        memory manager
├── scheduler     task scheduler
├── filesystem    storage system
└── userland      shell and applications
```

---

# Building

Install dependencies:

```
rustup component add llvm-tools-preview
cargo install bootimage
```

Build the OS:

```
cargo bootimage
```

---

# Running

Run AresOS using QEMU:

```
qemu-system-x86_64 \
-drive format=raw,file=target/x86_64-aresos/debug/bootimage-aresos.bin
```

---

# Vision

AresOS is not intended to replace existing operating systems.

Instead, it exists to answer a question:

**What happens when you build a system entirely on your own terms?**



