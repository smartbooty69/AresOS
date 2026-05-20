#!/usr/bin/env python3

import argparse
import os
import re
import signal
import subprocess
import sys

PHASE17_CONTEXT_RE = re.compile(
    r"Phase17-UserContext:\s+contexts=(\d+),\s+rejected=(\d+),\s+user_code=(\d+),\s+user_data=(\d+),\s+entry_ok=(true|false),\s+ring3_entered=(true|false)"
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


def phase17_user_context_ok(output: str) -> bool:
    for line in output.splitlines():
        match = PHASE17_CONTEXT_RE.search(line)
        if not match:
            continue
        contexts, _rejected, user_code, user_data, entry_ok, ring3_entered = match.groups()
        return (
            int(contexts) >= 1
            and int(user_code) > 0
            and int(user_data) > 0
            and entry_ok == "true"
            and ring3_entered == "false"
        )
    return False


def main() -> int:
    parser = argparse.ArgumentParser(description="Run Phase 17 user-context smoke check.")
    parser.add_argument("--timeout", type=int, default=120)
    args = parser.parse_args()

    cleanup_qemu_processes()
    code, output = run_kernel(args.timeout)
    print(output[-4000:])

    if not phase17_user_context_ok(output):
        print("FAIL: Phase17-UserContext line missing or indicates false flags")
        return 1
    if code not in (0, 124):
        print(f"FAIL: kernel exited with {code}")
        return code

    print("PASS: Phase 17 user-context smoke check passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
