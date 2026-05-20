#!/usr/bin/env python3

import argparse
import os
import re
import signal
import subprocess
import sys

PHASE9_LOADER_RE = re.compile(
    r"Phase9-Loader:\s+programs=(\d+),\s+launch_ok=(true|false),\s+storage_backed=(true|false),\s+launches=(\d+),\s+failed_launches=(\d+)"
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


def phase9_loader_ok(output: str) -> bool:
    for line in output.splitlines():
        match = PHASE9_LOADER_RE.search(line)
        if not match:
            continue
        programs, launch_ok, storage_backed, launches, _failed = match.groups()
        return (
            int(programs) >= 4
            and launch_ok == "true"
            and storage_backed == "true"
            and int(launches) > 0
        )
    return False


def main() -> int:
    parser = argparse.ArgumentParser(description="Run Phase 9 stored program loader smoke check.")
    parser.add_argument("--timeout", type=int, default=120)
    args = parser.parse_args()

    cleanup_qemu_processes()
    code, output = run_kernel(args.timeout)
    print(output[-4000:])

    if not phase9_loader_ok(output):
        print("FAIL: Phase9-Loader line missing or indicates false flags")
        return 1
    if code not in (0, 124):
        print(f"FAIL: kernel exited with {code}")
        return code

    print("PASS: Phase 9 stored program loader smoke check passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
