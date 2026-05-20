#!/usr/bin/env python3

import argparse
import os
import re
import signal
import subprocess
import sys

PHASE14_FRAMES_RE = re.compile(
    r"Phase14-Frames:\s+initialized=(true|false),\s+tracked=(\d+),\s+available=(\d+),\s+allocated=(\d+),\s+allocations=(\d+),\s+releases=(\d+),\s+failures=(\d+),\s+smoke_ok=(true|false)"
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


def phase14_frames_ok(output: str) -> bool:
    for line in output.splitlines():
        match = PHASE14_FRAMES_RE.search(line)
        if not match:
            continue
        initialized, tracked, available, _allocated, allocations, releases, _failures, smoke_ok = (
            match.groups()
        )
        return (
            initialized == "true"
            and int(tracked) > 0
            and int(available) > 0
            and int(allocations) > 0
            and int(releases) > 0
            and smoke_ok == "true"
        )
    return False


def main() -> int:
    parser = argparse.ArgumentParser(description="Run Phase 14 frame ownership smoke check.")
    parser.add_argument("--timeout", type=int, default=120)
    args = parser.parse_args()

    cleanup_qemu_processes()
    code, output = run_kernel(args.timeout)
    print(output[-4000:])

    if not phase14_frames_ok(output):
        print("FAIL: Phase14-Frames line missing or indicates false flags")
        return 1
    if code not in (0, 124):
        print(f"FAIL: kernel exited with {code}")
        return code

    print("PASS: Phase 14 frame ownership smoke check passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
