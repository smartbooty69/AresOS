# Executable Image Groundwork

Phase 11 adds executable-image recognition and address-space descriptors. It does not execute arbitrary machine code yet.

## Image Manifest

Image programs use the existing `ares-exec-v1` envelope:

```text
ares-exec-v1
name=hello
kind=elf64-image
entry=0x400000
image=/bin/hello.elf
requires=execute
trust=user
owner=user
description=ELF image validation fixture
```

The loader still supports `kind=builtin-alias` for current stored programs. `kind=elf64-image` is discoverable and validatable, but `run hello` returns an unsupported-execution error until a future phase adds executable mappings and privilege transitions.

## ELF64 Validation

The image parser accepts a deliberately small subset:

- ELF64 little-endian images
- x86_64 machine type
- loadable program headers
- bounded image and segment counts
- non-overlapping segments
- no writable+executable segments

The parser rejects invalid magic, unsupported architecture, invalid header layout, malformed segments, oversized images, and unsupported execution attempts with typed errors.

## Address-Space Descriptors

Phase 11 introduces descriptor-only address spaces:

- `AddressSpaceId`
- `VirtualRegion`
- `RegionKind`
- mapping flags derived from image segment flags

Descriptors validate user ranges, overlap, empty regions, and writable+executable mappings. They do not switch CR3 or create per-process page tables.

## Observability

The shell exposes:

- `bin validate <program>`
- richer `bin info <program>` output
- `ps` image/source display for loader-created process records

Boot emits:

```text
Phase11-Images: images=..., valid=..., rejected=..., exec_blocked_ok=true
```

## Deferred Work

- actual ELF relocation and executable memory mapping
- per-process page tables and CR3 switching
- Ring 3 entry and syscall return paths
- demand paging and memory-mapped executable files
- dynamic linking and cryptographic signatures
