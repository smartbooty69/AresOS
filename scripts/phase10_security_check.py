#!/usr/bin/env python3

import argparse
import os
import re
import signal
import subprocess
import sys

PHASE10_SECURITY_RE = re.compile(
    r"Phase10-Security:\s+user=(\d+),\s+role=([a-z]+),\s+policy_ok=(true|false),\s+denied_ok=(true|false),\s+denied_access=(\d+),\s+denied_execute=(\d+)"
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


def phase10_security_ok(output: str) -> bool:
    for line in output.splitlines():
        match = PHASE10_SECURITY_RE.search(line)
        if not match:
            continue
        user, role, policy_ok, denied_ok, denied_access, _denied_execute = match.groups()
        return (
            int(user) == 100
            and role == "user"
            and policy_ok == "true"
            and denied_ok == "true"
            and int(denied_access) > 0
        )
    return False


def main() -> int:
    parser = argparse.ArgumentParser(description="Run Phase 10 security smoke check.")
    parser.add_argument("--timeout", type=int, default=120)
    args = parser.parse_args()

    cleanup_qemu_processes()
    code, output = run_kernel(args.timeout)
    print(output[-4000:])

    if not phase10_security_ok(output):
        print("FAIL: Phase10-Security line missing or indicates false flags")
        return 1
    if code not in (0, 124):
        print(f"FAIL: kernel exited with {code}")
        return code

    print("PASS: Phase 10 security smoke check passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
