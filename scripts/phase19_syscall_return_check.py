#!/usr/bin/env python3

import argparse
import os
import re
import signal
import subprocess
import sys

PHASE19_SYSCALL_RE = re.compile(
    r"Phase19-SyscallReturn:\s+syscalls=(\d+),\s+returns=(\d+),\s+rejected=(\d+),\s+abi_ok=(true|false),\s+returned_ok=(true|false)"
)


def cleanup_qemu_processes() -> None:
    if os.name != "nt":
        return
    subprocess.run(
        ["taskkill", "/IM", "qemu-system-x86_64.exe", "/F"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )


def run_kernel(timeout: int) -> tuple[int, str]:
    process = subprocess.Popen(
        [
            "cargo",
            "run",
            "-p",
            "kernel",
            "--features",
            "preemption",
            "--",
            "-serial",
            "stdio",
            "-display",
            "none",
            "-no-reboot",
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        env=os.environ.copy(),
    )
    try:
        output, _ = process.communicate(timeout=timeout)
        return process.returncode or 0, output
    except subprocess.TimeoutExpired:
        process.send_signal(signal.SIGTERM)
        try:
            output, _ = process.communicate(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            output, _ = process.communicate(timeout=5)
        cleanup_qemu_processes()
        return 124, output


def phase19_syscall_return_ok(output: str) -> bool:
    for line in output.splitlines():
        match = PHASE19_SYSCALL_RE.search(line)
        if not match:
            continue
        syscalls, returns, _rejected, abi_ok, returned_ok = match.groups()
        return int(syscalls) >= 1 and int(returns) >= 1 and abi_ok == "true" and returned_ok == "true"
    return False


def main() -> int:
    parser = argparse.ArgumentParser(description="Run Phase 19 syscall return smoke check.")
    parser.add_argument("--timeout", type=int, default=20)
    args = parser.parse_args()

    cleanup_qemu_processes()
    code, output = run_kernel(args.timeout)
    print(output[-4000:])

    if not phase19_syscall_return_ok(output):
        print("FAIL: Phase19-SyscallReturn line missing or indicates false flags")
        return 1
    if code not in (0, 124):
        print(f"FAIL: kernel exited with {code}")
        return code

    print("PASS: Phase 19 syscall return smoke check passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
