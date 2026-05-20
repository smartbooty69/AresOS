#!/usr/bin/env python3
"""Collect Phase 5 serial telemetry lines after kernel boot."""

from __future__ import annotations

import os
import signal
import subprocess
import threading
import time
from collections.abc import Callable
from queue import Empty, Queue
from typing import TypeVar

from smoke_qemu import cleanup_qemu_processes

T = TypeVar("T")

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


def terminate_process_tree(process: subprocess.Popen[bytes]) -> None:
    if process.poll() is not None:
        return
    if os.name == "nt":
        process.terminate()
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=3)
        return

    os.killpg(process.pid, signal.SIGTERM)
    try:
        process.wait(timeout=3)
    except subprocess.TimeoutExpired:
        os.killpg(process.pid, signal.SIGKILL)
        process.wait(timeout=3)


def collect_samples(
    parse_line: Callable[[str], T | None],
    *,
    boot_wait: int,
    duration: int,
) -> tuple[list[T], list[str]]:
    """Wait up to `boot_wait` seconds for the first sample, then collect for `duration` seconds."""
    cleanup_qemu_processes()
    process = subprocess.Popen(
        KERNEL_CMD,
        env=os.environ.copy(),
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=False,
        bufsize=0,
        start_new_session=True,
    )

    queue: Queue[str | None] = Queue()
    output_tail: list[str] = []

    def stream_reader() -> None:
        assert process.stdout is not None
        pending = ""
        while True:
            chunk = os.read(process.stdout.fileno(), 4096)
            if not chunk:
                if pending:
                    queue.put(pending)
                queue.put(None)
                return
            pending += chunk.decode(errors="replace")
            while "\n" in pending:
                line, pending = pending.split("\n", 1)
                queue.put(line.rstrip("\r"))

    reader = threading.Thread(target=stream_reader, daemon=True)
    reader.start()

    samples: list[T] = []
    boot_deadline = time.time() + boot_wait
    collect_deadline: float | None = None

    try:
        while True:
            now = time.time()
            if collect_deadline is not None and now >= collect_deadline:
                break
            if collect_deadline is None and now >= boot_deadline and not samples:
                break

            try:
                line = queue.get(timeout=0.2)
            except Empty:
                if process.poll() is not None:
                    if collect_deadline is None and now < boot_deadline:
                        continue
                    break
                continue

            if line is None:
                break

            output_tail.append(line)
            if len(output_tail) > 40:
                output_tail.pop(0)

            sample = parse_line(line)
            if sample is not None:
                samples.append(sample)
                if collect_deadline is None:
                    collect_deadline = time.time() + duration
    finally:
        if process.poll() is None:
            terminate_process_tree(process)
        cleanup_qemu_processes()

    return samples, output_tail
