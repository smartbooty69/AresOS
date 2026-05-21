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
    parser.add_argument("--soak-duration", type=int, default=60)
    parser.add_argument("--latency-duration", type=int, default=60)
    parser.add_argument("--max-latency-ms", type=int, default=300)
    parser.add_argument("--max-fairness-score", type=float, default=1.10)
    parser.add_argument("--smoke-timeout", type=int, default=120)
    parser.add_argument("--boot-wait", type=int, default=150, help="Seconds to wait for Phase5 telemetry after boot")
    parser.add_argument(
        "--from-check",
        type=str,
        default="",
        help="Skip checks before this name (e.g. phase14-frame-check)",
    )
    args = parser.parse_args()

    checks: list[tuple[str, list[str], int | None]] = [
        ("cargo-check", ["cargo", "check", "-p", "kernel"], None),
        (
            "preemption-integration",
            [
                "cargo",
                "test",
                "-p",
                "kernel",
                "--features",
                "preemption",
                "--test",
                "preemption_integration",
            ],
            900,
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
            "phase15-frame-backing-check",
            [
                "python",
                "scripts/phase15_frame_backing_check.py",
                "--timeout",
                str(args.smoke_timeout),
            ],
            None,
        ),
        (
            "phase16-page-table-check",
            [
                "python",
                "scripts/phase16_page_table_check.py",
                "--timeout",
                str(args.smoke_timeout),
            ],
            None,
        ),
        (
            "phase17-user-context-check",
            [
                "python",
                "scripts/phase17_user_context_check.py",
                "--timeout",
                str(args.smoke_timeout),
            ],
            None,
        ),
        (
            "phase18-ring3-check",
            [
                "python",
                "scripts/phase18_ring3_check.py",
                "--timeout",
                str(args.smoke_timeout),
            ],
            None,
        ),
        (
            "phase19-syscall-return-check",
            [
                "python",
                "scripts/phase19_syscall_return_check.py",
                "--timeout",
                str(args.smoke_timeout),
            ],
            None,
        ),
        (
            "phase20-user-elf-check",
            [
                "python",
                "scripts/phase20_user_elf_check.py",
                "--timeout",
                str(args.smoke_timeout),
            ],
            None,
        ),
        (
            "phase21-hw-page-table-check",
            ["python", "scripts/phase21_hw_page_table_check.py", "--timeout", str(args.smoke_timeout)],
            None,
        ),
        (
            "phase22-cr3-check",
            ["python", "scripts/phase22_cr3_check.py", "--timeout", str(args.smoke_timeout)],
            None,
        ),
        (
            "phase23-iretq-check",
            ["python", "scripts/phase23_iretq_check.py", "--timeout", str(args.smoke_timeout)],
            None,
        ),
        (
            "phase24-user-trap-check",
            ["python", "scripts/phase24_user_trap_check.py", "--timeout", str(args.smoke_timeout)],
            None,
        ),
        (
            "phase25-syscall-hw-check",
            ["python", "scripts/phase25_syscall_hw_check.py", "--timeout", str(args.smoke_timeout)],
            None,
        ),
        (
            "phase26-copyin-check",
            ["python", "scripts/phase26_copyin_check.py", "--timeout", str(args.smoke_timeout)],
            None,
        ),
        (
            "phase27-reloc-check",
            ["python", "scripts/phase27_reloc_check.py", "--timeout", str(args.smoke_timeout)],
            None,
        ),
        (
            "phase28-hw-hello-check",
            ["python", "scripts/phase28_hw_hello_check.py", "--timeout", str(args.smoke_timeout)],
            None,
        ),
        (
            "phase29-allowlist-check",
            ["python", "scripts/phase29_allowlist_check.py", "--timeout", str(args.smoke_timeout)],
            None,
        ),
        (
            "phase30-cr3-switch-check",
            ["python", "scripts/phase30_cr3_switch_check.py", "--timeout", str(args.smoke_timeout)],
            None,
        ),
        (
            "phase31-sched-cr3-check",
            ["python", "scripts/phase31_sched_cr3_check.py", "--timeout", str(args.smoke_timeout)],
            None,
        ),
        (
            "phase32-user-frame-check",
            ["python", "scripts/phase32_user_frame_check.py", "--timeout", str(args.smoke_timeout)],
            None,
        ),
        (
            "phase33-multi-elf-check",
            ["python", "scripts/phase33_multi_elf_check.py", "--timeout", str(args.smoke_timeout)],
            None,
        ),
        (
            "phase34-exit-wait-check",
            ["python", "scripts/phase34_exit_wait_check.py", "--timeout", str(args.smoke_timeout)],
            None,
        ),
        (
            "phase35-syscall-table-check",
            ["python", "scripts/phase35_syscall_table_check.py", "--timeout", str(args.smoke_timeout)],
            None,
        ),
        (
            "phase36-storage-copyin-check",
            ["python", "scripts/phase36_storage_copyin_check.py", "--timeout", str(args.smoke_timeout)],
            None,
        ),
        (
            "phase37-manifest-elf-check",
            ["python", "scripts/phase37_manifest_elf_check.py", "--timeout", str(args.smoke_timeout)],
            None,
        ),
        (
            "phase38-demand-zero-check",
            ["python", "scripts/phase38_demand_zero_check.py", "--timeout", str(args.smoke_timeout)],
            None,
        ),
        (
            "phase39-dynamic-check",
            ["python", "scripts/phase39_dynamic_check.py", "--timeout", str(args.smoke_timeout)],
            None,
        ),
        (
            "phase40-integration-check",
            ["python", "scripts/phase40_integration_check.py", "--timeout", str(max(args.smoke_timeout, 180))],
            None,
        ),
        (
            "phase5-soak-check",
            [
                "python",
                "scripts/phase5_soak_check.py",
                "--duration",
                str(args.soak_duration),
                "--boot-wait",
                str(args.boot_wait),
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
                "--boot-wait",
                str(args.boot_wait),
                "--min-samples",
                "2",
                "--max-latency-ms",
                str(args.max_latency_ms),
            ],
            None,
        ),
    ]

    if args.from_check:
        names = [name for name, _, _ in checks]
        if args.from_check not in names:
            print(f"Unknown --from-check name: {args.from_check}")
            print("Known checks:", ", ".join(names))
            return 2
        start_idx = names.index(args.from_check)
        checks = checks[start_idx:]
        print(f"Resuming from {args.from_check} ({len(checks)} checks)")

    phase5_timeout = args.boot_wait + max(args.soak_duration, args.latency_duration) + 180

    any_failed = False
    cleanup_qemu_processes()
    print("Validation matrix start")
    for name, cmd, timeout in checks:
        if name in ("phase5-soak-check", "phase5-latency-check"):
            timeout = phase5_timeout
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
