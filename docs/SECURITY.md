# Security Model

Phase 10 adds policy groundwork, not hardware-enforced isolation. The kernel now has static credentials, file permissions, executable trust metadata, and process ownership checks that future ELF loading and address-space isolation can build on.

## Identity

The security module defines four roles:

- `Kernel`: boot and internal kernel ownership.
- `Admin`: privileged management identity.
- `User`: default shell identity.
- `Guest`: read-biased low-trust identity for validation and future sessions.

The default shell starts as `UserId(100)` with role `User`. `su admin`, `su user`, and `su guest` are static role switches for validation only; there is no password or multi-session login layer in this phase.

## File Policy

Each filesystem directory entry stores an owner and a three-bit mode:

- read
- write
- execute

Bootstrapped files use conservative defaults:

- `/bin/*` is admin-owned and readable/executable.
- `/README.txt` is kernel-owned and read-only.
- user-created files are owned by the current user and readable/writable.

Shell and syscall read/write/delete paths use checked storage APIs. Legacy unchecked storage helpers remain for bootstrapping and older internal tests, but user-facing paths enforce policy.

## Program Trust

`ares-exec-v1` manifests now accept:

- `requires=execute`
- `trust=system` or `trust=user`
- `owner=<name>`

The loader rejects unsupported trust or requirement values without panicking. Before dispatching a stored built-in alias, the loader checks execute permission on the manifest file and records denied launches separately from normal failed launches.

Phase 11 extends this policy to executable image manifests. `kind=elf64-image` records require execute permission on both the manifest and referenced image file before validation. The image can be parsed and described, but execution is blocked until a future phase adds executable mappings and privilege separation.

## Process Ownership

Process records now capture creator credentials. Process display includes owner role, and checked termination follows these rules:

- admin/kernel can terminate any process
- users can terminate their own processes
- users cannot terminate admin/kernel-owned processes

## Observability

The shell exposes:

- `whoami`
- `stat <path>`
- `chmod +x|-x <path>`
- `su admin|user|guest`

Syscalls expose current user, current role, denied access count, and denied execute count. Boot emits:

```text
Phase10-Security: user=100, role=user, policy_ok=true, denied_ok=true, denied_access=..., denied_execute=...
```

## Limits

Phase 10 intentionally does not provide CPU privilege separation, page-table isolation, real executable memory protections, cryptographic program signatures, groups, ACLs, or capabilities. Those are deferred until the kernel has raw ELF loading and per-process address spaces.

Phase 11 adds descriptor-only address spaces and ELF64 validation, but still does not switch page tables or run arbitrary stored code.
