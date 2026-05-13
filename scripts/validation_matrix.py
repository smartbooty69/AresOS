#!/usr/bin/env python3

import argparse
import os
import re
import signal
import subprocess
import sys
import time

PHASE6_SMOKE_RE = re.compile(
    r"Phase6-Smoke:\s+mounted=(true|false),\s+list_ok=(true|false),\s+cat_ok=(true|false),\s+run_ok=(true|false)"
)


def run_command(cmd: list[str], timeout: int | None = None) -> tuple[int, str]:
    process = subprocess.Popen(
        cmd,
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


def cleanup_qemu_processes() -> None:
    if os.name != "nt":
        return
    subprocess.run(
        ["taskkill", "/IM", "qemu-system-x86_64.exe", "/F"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )


def check_phase6_smoke_output(text: str) -> bool:
    for line in text.splitlines():
        match = PHASE6_SMOKE_RE.search(line)
        if not match:
            continue
        mounted, list_ok, cat_ok, run_ok = match.groups()
        return all(v == "true" for v in (mounted, list_ok, cat_ok, run_ok))
    return False


def main() -> int:
    parser = argparse.ArgumentParser(description="Run AresOS validation matrix with practical thresholds.")
    parser.add_argument("--soak-duration", type=int, default=45)
    parser.add_argument("--latency-duration", type=int, default=45)
    parser.add_argument("--max-latency-ms", type=int, default=100)
    parser.add_argument("--max-fairness-score", type=float, default=1.10)
    parser.add_argument("--smoke-timeout", type=int, default=20)
    args = parser.parse_args()

    checks: list[tuple[str, list[str], int | None]] = [
        ("cargo-check", ["cargo", "check", "-p", "kernel"], None),
        (
            "preemption-integration",
            ["cargo", "test", "-p", "kernel", "--test", "preemption_integration"],
            None,
        ),
        (
            "phase6-smoke-qemu",
            ["cargo", "run", "-p", "kernel", "--features", "preemption", "--", "-serial", "stdio", "-display", "none", "-no-reboot"],
            args.smoke_timeout,
        ),
        (
            "phase7-storage-check",
            [
                "python",
                "scripts/phase7_storage_check.py",
                "--timeout",
                str(args.smoke_timeout),
            ],
            None,
        ),
        (
            "phase8-device-check",
            [
                "python",
                "scripts/phase8_device_check.py",
                "--timeout",
                str(args.smoke_timeout),
            ],
            None,
        ),
        (
            "phase9-loader-check",
            [
                "python",
                "scripts/phase9_loader_check.py",
                "--timeout",
                str(args.smoke_timeout),
            ],
            None,
        ),
        (
            "phase10-security-check",
            [
                "python",
                "scripts/phase10_security_check.py",
                "--timeout",
                str(args.smoke_timeout),
            ],
            None,
        ),
        (
            "phase11-image-check",
            [
                "python",
                "scripts/phase11_image_check.py",
                "--timeout",
                str(args.smoke_timeout),
            ],
            None,
        ),
        (
            "phase12-load-plan-check",
            [
                "python",
                "scripts/phase12_load_plan_check.py",
                "--timeout",
                str(args.smoke_timeout),
            ],
            None,
        ),
        (
            "phase13-mapping-stub-check",
            [
                "python",
                "scripts/phase13_mapping_stub_check.py",
                "--timeout",
                str(args.smoke_timeout),
            ],
            None,
        ),
        (
            "phase14-frame-check",
            [
                "python",
                "scripts/phase14_frame_check.py",
                "--timeout",
                str(args.smoke_timeout),
            ],
            None,
        ),
        (
            "phase5-soak-check",
            [
                "python",
                "scripts/phase5_soak_check.py",
                "--duration",
                str(args.soak_duration),
                "--min-samples",
                "2",
                "--max-score",
                str(args.max_fairness_score),
            ],
            None,
        ),
        (
            "phase5-latency-check",
            [
                "python",
                "scripts/phase5_latency_check.py",
                "--duration",
                str(args.latency_duration),
                "--min-samples",
                "2",
                "--max-latency-ms",
                str(args.max_latency_ms),
            ],
            None,
        ),
    ]

    any_failed = False
    cleanup_qemu_processes()
    print("Validation matrix start")
    for name, cmd, timeout in checks:
        print(f"\n=== {name} ===")
        print("Command:", " ".join(cmd))
        start = time.time()
        code, output = run_command(cmd, timeout=timeout)
        elapsed = time.time() - start
        print(output[-4000:])
        cleanup_qemu_processes()

        if name == "phase6-smoke-qemu":
            smoke_ok = check_phase6_smoke_output(output)
            if not smoke_ok:
                print("FAIL: Phase6-Smoke line missing or indicates false flags")
                any_failed = True
                continue
            print("PASS: Phase6-Smoke runtime path validated")

        if code != 0 and not (name == "phase6-smoke-qemu" and code == 124):
            print(f"FAIL: {name} exited with {code} in {elapsed:.1f}s")
            any_failed = True
        elif code == 124 and name == "phase6-smoke-qemu":
            # Expected timeout for non-terminating kernel run once smoke line is observed.
            print(f"PASS: {name} reached steady-state timeout in {elapsed:.1f}s")
        else:
            print(f"PASS: {name} in {elapsed:.1f}s")

    if any_failed:
        print("\nValidation matrix: FAIL")
        return 1
    print("\nValidation matrix: PASS")
    return 0


if __name__ == "__main__":
    sys.exit(main())
