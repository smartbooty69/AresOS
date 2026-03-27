#!/usr/bin/env python3

import argparse
import os
import re
import select
import signal
import subprocess
import sys
import time
from dataclasses import dataclass

LATENCY_RE = re.compile(r"Phase5-Latency:\s+(.*)")
KV_RE = re.compile(
    r"(ticks|quantum|req|pts|backlog|max_backlog|est_ms|max_est_ms)=([0-9]+(?:\.[0-9]+)?)"
)


@dataclass
class Sample:
    ticks: int
    quantum: int
    req: int
    pts: int
    backlog: int
    max_backlog: int
    est_ms: int
    max_est_ms: int


def parse_sample(line: str) -> Sample | None:
    match = LATENCY_RE.search(line)
    if not match:
        return None

    values = {k: v for k, v in KV_RE.findall(match.group(1))}
    required = {"ticks", "quantum", "req", "pts", "backlog", "max_backlog", "est_ms", "max_est_ms"}
    if not required.issubset(values):
        return None

    return Sample(
        ticks=int(float(values["ticks"])),
        quantum=int(float(values["quantum"])),
        req=int(float(values["req"])),
        pts=int(float(values["pts"])),
        backlog=int(float(values["backlog"])),
        max_backlog=int(float(values["max_backlog"])),
        est_ms=int(float(values["est_ms"])),
        max_est_ms=int(float(values["max_est_ms"])),
    )


def validate(samples: list[Sample], max_latency_ms: int) -> tuple[bool, list[str]]:
    errors: list[str] = []
    if len(samples) < 2:
        return False, ["not enough latency samples captured"]

    first = samples[0]
    last = samples[-1]

    if last.req <= first.req:
        errors.append("reschedule request counter did not advance")

    if last.pts <= first.pts:
        errors.append("reschedule point counter did not advance")

    observed_max = max(sample.max_est_ms for sample in samples)
    if observed_max > max_latency_ms:
        errors.append(f"estimated preemption latency too high: {observed_max}ms > {max_latency_ms}ms")

    return len(errors) == 0, errors


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Run Phase 5 latency validation and enforce estimated preemption latency SLA."
    )
    parser.add_argument("--duration", type=int, default=120, help="Validation duration in seconds")
    parser.add_argument("--min-samples", type=int, default=5, help="Minimum latency samples")
    parser.add_argument("--max-latency-ms", type=int, default=100, help="Maximum allowed estimated preemption latency in milliseconds")
    args = parser.parse_args()

    cmd = [
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

    print(f"Starting Phase 5 latency check for {args.duration}s")
    print("Command:", " ".join(cmd))

    samples: list[Sample] = []
    output_tail: list[str] = []
    process = None
    try:
        process = subprocess.Popen(
            cmd,
            env=os.environ.copy(),
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=False,
            bufsize=0,
            start_new_session=True,
        )

        deadline = time.time() + args.duration
        pending = ""
        while process.stdout is not None and time.time() < deadline:
            ready, _, _ = select.select([process.stdout], [], [], 0.2)
            if not ready:
                if process.poll() is not None:
                    break
                continue

            chunk = os.read(process.stdout.fileno(), 4096)
            if not chunk:
                if process.poll() is not None:
                    break
                continue

            pending += chunk.decode(errors="replace")
            while "\n" in pending:
                line, pending = pending.split("\n", 1)
                output_tail.append(line.rstrip("\r"))
                if len(output_tail) > 40:
                    output_tail.pop(0)

                sample = parse_sample(line)
                if sample is not None:
                    samples.append(sample)

        if pending:
            output_tail.append(pending.rstrip("\r"))
            if len(output_tail) > 40:
                output_tail.pop(0)

            sample = parse_sample(pending)
            if sample is not None:
                samples.append(sample)

        if process.poll() is None:
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=3)
    except KeyboardInterrupt:
        if process is not None and process.poll() is None:
            os.killpg(process.pid, signal.SIGTERM)
        print("Interrupted by user")
        return 130

    if len(samples) < args.min_samples:
        print(
            f"FAIL: captured only {len(samples)} latency samples, need at least {args.min_samples}"
        )
        if output_tail:
            print("Last captured output lines:")
            for line in output_tail[-20:]:
                print(f"  {line}")
        return 1

    ok, errors = validate(samples, args.max_latency_ms)
    first = samples[0]
    last = samples[-1]
    observed_max_est_ms = max(sample.max_est_ms for sample in samples)

    print(
        "Summary: "
        f"samples={len(samples)}, "
        f"req={first.req}->{last.req}, "
        f"pts={first.pts}->{last.pts}, "
        f"backlog={first.backlog}->{last.backlog}, "
        f"est_ms={first.est_ms}->{last.est_ms}, "
        f"max_est_ms={observed_max_est_ms}"
    )

    if not ok:
        print("FAIL:")
        for error in errors:
            print(f"  - {error}")
        return 1

    print(f"PASS: Phase 5 latency check passed (max_est_ms <= {args.max_latency_ms})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
