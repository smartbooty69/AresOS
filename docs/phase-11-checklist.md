# Phase 11 Checklist

Status: Complete

- [x] Add executable image, segment, format, flag, and image-load error types.
- [x] Add conservative ELF64 header and load-segment validation.
- [x] Extend `ares-exec-v1` with `kind=elf64-image` and `image=<path>`.
- [x] Seed a small `/bin/hello` image manifest and `/bin/hello.elf` validation fixture.
- [x] Require execute permission on both image manifests and referenced image files.
- [x] Reject actual ELF image execution with a clear unsupported-execution result.
- [x] Add process image metadata for loader-created process records.
- [x] Add descriptor-only address-space and virtual-region validation.
- [x] Expose `bin validate <program>` and richer `bin info` image fields.
- [x] Add image status syscalls and `Phase11-Images` boot smoke output.
- [x] Add Phase 11 QEMU validation and validation matrix coverage.

Exit gate:

- [x] Built-in aliases still launch.
- [x] Valid image manifests are discoverable and validate cleanly.
- [x] Malformed or non-executable image records do not panic.
- [x] No Phase 11 path executes arbitrary binary code.
