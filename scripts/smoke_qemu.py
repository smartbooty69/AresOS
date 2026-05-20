#!/usr/bin/env python3
"""Shared helpers for Phase N QEMU serial smoke checks."""

from __future__ import annotations

import os
import signal
import subprocess

DEFAULT_SMOKE_TIMEOUT = 120

KERNEL_CMD = [
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
]


def cleanup_qemu_processes() -> None:
    if os.name != "nt":
        return
    subprocess.run(
        ["taskkill", "/IM", "qemu-system-x86_64.exe", "/F"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )


def run_kernel(timeout: int = DEFAULT_SMOKE_TIMEOUT) -> tuple[int, str]:
    cleanup_qemu_processes()
    process = subprocess.Popen(
        KERNEL_CMD,
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


def kernel_exit_ok(code: int) -> bool:
    return code in (0, 124)
