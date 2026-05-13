# Phase 10 Checklist

Status: Complete

- [x] Add static user, role, credential, access-kind, file-mode, and security-error primitives.
- [x] Add file owner/mode metadata to the simple filesystem directory records.
- [x] Route shell and syscall file operations through checked read/write/delete paths.
- [x] Protect `/bin/*` as system-owned readable/executable files.
- [x] Extend executable manifests with `requires`, `trust`, and `owner` fields.
- [x] Require execute permission before loader dispatch.
- [x] Add process ownership metadata and checked kill policy.
- [x] Add shell observability commands: `whoami`, `su`, `stat`, and `chmod +x|-x`.
- [x] Add syscall counters for current identity and denied access.
- [x] Emit `Phase10-Security` boot smoke output.
- [x] Add QEMU-backed `phase10-security-check` validation and matrix coverage.

Exit gate:

- [x] User cannot overwrite `/bin/echo`.
- [x] User can create, read, write, and delete own regular files.
- [x] Loader denies launch when execute permission is missing.
- [x] User cannot terminate admin-owned processes.
- [x] Phase 10 smoke output is machine-validated.
