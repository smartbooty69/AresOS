#!/usr/bin/env python3

import argparse
import os
import re
import signal
import subprocess
import sys

PHASE12_LOAD_PLAN_RE = re.compile(
    r"Phase12-LoadPlan:\s+prepared=(\d+),\s+rejected=(\d+),\s+pages=(\d+),\s+exec_blocked_ok=(true|false)"
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


def phase12_load_plan_ok(output: str) -> bool:
    for line in output.splitlines():
        match = PHASE12_LOAD_PLAN_RE.search(line)
        if not match:
            continue
        prepared, _rejected, pages, exec_blocked_ok = match.groups()
        return int(prepared) >= 1 and int(pages) >= 1 and exec_blocked_ok == "true"
    return False


def main() -> int:
    parser = argparse.ArgumentParser(description="Run Phase 12 executable load-plan smoke check.")
    parser.add_argument("--timeout", type=int, default=120)
    args = parser.parse_args()

    cleanup_qemu_processes()
    code, output = run_kernel(args.timeout)
    print(output[-4000:])

    if not phase12_load_plan_ok(output):
        print("FAIL: Phase12-LoadPlan line missing or indicates false flags")
        return 1
    if code not in (0, 124):
        print(f"FAIL: kernel exited with {code}")
        return code

    print("PASS: Phase 12 executable load-plan smoke check passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
