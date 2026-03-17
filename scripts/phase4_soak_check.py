#!/usr/bin/env python3

import argparse
import re
import subprocess
import sys
import time
from dataclasses import dataclass


CONTEXT_RE = re.compile(r"ContextLab\s+(.*)")
KV_RE = re.compile(r"([a-zA-Z0-9_]+)=([0-9]+)")


@dataclass
class Sample:
    a: int
    b: int
    ticks: int
    switches: int
    irq_forced_ok: int
    irq_forced_blocked: int
    handoff_q: int
    handoff_c: int
    misses: int
    timer_stall_fallbacks: int


def parse_sample(line: str) -> Sample | None:
    m = CONTEXT_RE.search(line)
    if not m:
        return None

    values = {k: int(v) for k, v in KV_RE.findall(m.group(1))}
    required = {
        "A",
        "B",
        "ticks",
        "switches",
        "irq_forced_ok",
        "irq_forced_blocked",
        "handoff_q",
        "handoff_c",
        "misses",
        "timer_stall_fallbacks",
    }
    if not required.issubset(values):
        return None

    return Sample(
        a=values["A"],
        b=values["B"],
        ticks=values["ticks"],
        switches=values["switches"],
        irq_forced_ok=values["irq_forced_ok"],
        irq_forced_blocked=values["irq_forced_blocked"],
        handoff_q=values["handoff_q"],
        handoff_c=values["handoff_c"],
        misses=values["misses"],
        timer_stall_fallbacks=values["timer_stall_fallbacks"],
    )


def validate(samples: list[Sample]) -> tuple[bool, list[str]]:
    errors: list[str] = []

    if len(samples) < 2:
        errors.append("not enough ContextLab samples captured")
        return False, errors

    first = samples[0]
    last = samples[-1]

    if last.ticks <= first.ticks:
        errors.append("ticks did not advance")
    if last.switches <= first.switches:
        errors.append("switches did not advance")
    if last.a <= first.a:
        errors.append("ContextLab A counter did not advance")
    if last.b <= first.b:
        errors.append("ContextLab B counter did not advance")

    if last.handoff_q <= first.handoff_q:
        errors.append("handoff_q did not advance")
    if last.handoff_c <= first.handoff_c:
        errors.append("handoff_c did not advance")

    if last.handoff_q != last.handoff_c:
        errors.append(
            f"final handoff mismatch: handoff_q={last.handoff_q}, handoff_c={last.handoff_c}"
        )

    first_misses = first.misses
    if any(s.misses != first_misses for s in samples):
        errors.append("misses changed during soak")

    blocked_start = first.irq_forced_blocked
    blocked_end = last.irq_forced_blocked
    if blocked_end < blocked_start:
        errors.append("irq_forced_blocked decreased unexpectedly")

    return len(errors) == 0, errors


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Run wrapper-mode preemption soak and validate ContextLab telemetry."
    )
    parser.add_argument(
        "--duration",
        type=int,
        default=120,
        help="Soak duration in seconds (default: 120)",
    )
    parser.add_argument(
        "--min-samples",
        type=int,
        default=10,
        help="Minimum number of ContextLab samples required (default: 10)",
    )
    args = parser.parse_args()

    cmd = [
        "cargo",
        "run",
        "-p",
        "kernel",
        "--features",
        "irq-exit-wrapper-experimental",
        "--",
        "-serial",
        "stdio",
        "-display",
        "none",
        "-no-reboot",
    ]

    print(f"Starting soak run for {args.duration}s")
    print("Command:", " ".join(cmd))

    proc = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,
    )

    samples: list[Sample] = []
    deadline = time.time() + args.duration

    try:
        assert proc.stdout is not None
        while time.time() < deadline:
            line = proc.stdout.readline()
            if line == "":
                if proc.poll() is not None:
                    break
                continue

            sample = parse_sample(line)
            if sample is not None:
                samples.append(sample)
    finally:
        if proc.poll() is None:
            proc.terminate()
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait(timeout=5)

    if len(samples) < args.min_samples:
        print(
            f"FAIL: captured only {len(samples)} ContextLab samples, need at least {args.min_samples}"
        )
        return 1

    ok, errors = validate(samples)
    first = samples[0]
    last = samples[-1]

    print(
        "Summary: "
        f"samples={len(samples)}, "
        f"ticks={first.ticks}->{last.ticks} (Δ{last.ticks - first.ticks}), "
        f"switches={first.switches}->{last.switches} (Δ{last.switches - first.switches}), "
        f"handoff_q={first.handoff_q}->{last.handoff_q} (Δ{last.handoff_q - first.handoff_q}), "
        f"handoff_c={first.handoff_c}->{last.handoff_c} (Δ{last.handoff_c - first.handoff_c}), "
        f"misses={first.misses}->{last.misses}, "
        f"irq_forced_blocked={first.irq_forced_blocked}->{last.irq_forced_blocked}"
    )

    if not ok:
        print("FAIL:")
        for error in errors:
            print(f"  - {error}")
        return 1

    print("PASS: wrapper-mode preemption soak checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
