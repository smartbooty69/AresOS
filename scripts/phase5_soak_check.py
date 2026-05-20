#!/usr/bin/env python3

import argparse
import re
import sys
import time
from dataclasses import dataclass
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from phase5_telemetry import collect_samples

FAIRNESS_RE = re.compile(r"Phase5-Fairness:\s+(.*)")
KV_RE = re.compile(r"(T[1-4]|score)=([0-9]+(?:\.[0-9]+)?)")


@dataclass
class Sample:
    t1: int
    t2: int
    t3: int
    t4: int
    score: float


def parse_sample(line: str) -> Sample | None:
    match = FAIRNESS_RE.search(line)
    if not match:
        return None

    values = {k: v for k, v in KV_RE.findall(match.group(1))}
    required = {"T1", "T2", "T3", "T4", "score"}
    if not required.issubset(values):
        return None

    return Sample(
        t1=int(float(values["T1"])),
        t2=int(float(values["T2"])),
        t3=int(float(values["T3"])),
        t4=int(float(values["T4"])),
        score=float(values["score"]),
    )


def validate(samples: list[Sample], max_score: float) -> tuple[bool, list[str]]:
    errors: list[str] = []
    if len(samples) < 2:
        return False, ["not enough fairness samples captured"]

    first = samples[0]
    last = samples[-1]

    deltas = [
        last.t1 - first.t1,
        last.t2 - first.t2,
        last.t3 - first.t3,
        last.t4 - first.t4,
    ]

    for index, delta in enumerate(deltas, start=1):
        if delta <= 0:
            errors.append(f"task T{index} did not advance")

    if last.score > max_score:
        errors.append(f"fairness score too high: {last.score:.3f} > {max_score:.3f}")

    return len(errors) == 0, errors


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Run Phase 5 preemption soak and validate fairness telemetry."
    )
    parser.add_argument("--boot-wait", type=int, default=150, help="Seconds to wait for first fairness sample")
    parser.add_argument("--duration", type=int, default=120, help="Soak duration after first sample")
    parser.add_argument("--min-samples", type=int, default=3, help="Minimum fairness samples")
    parser.add_argument("--max-score", type=float, default=1.10, help="Max allowed fairness score")
    args = parser.parse_args()

    print(
        f"Starting Phase 5 soak (boot_wait={args.boot_wait}s, duration={args.duration}s after first sample)"
    )

    try:
        samples, output_tail = collect_samples(
            parse_sample,
            boot_wait=args.boot_wait,
            duration=args.duration,
        )
    except KeyboardInterrupt:
        print("Interrupted by user")
        return 130

    if len(samples) < args.min_samples:
        print(
            f"FAIL: captured only {len(samples)} fairness samples, need at least {args.min_samples}"
        )
        if output_tail:
            print("Last captured output lines:")
            for line in output_tail[-20:]:
                print(f"  {line}")
        return 1

    ok, errors = validate(samples, args.max_score)
    first = samples[0]
    last = samples[-1]

    print(
        "Summary: "
        f"samples={len(samples)}, "
        f"T1={first.t1}->{last.t1}, "
        f"T2={first.t2}->{last.t2}, "
        f"T3={first.t3}->{last.t3}, "
        f"T4={first.t4}->{last.t4}, "
        f"score={first.score:.3f}->{last.score:.3f}"
    )

    if not ok:
        print("FAIL:")
        for error in errors:
            print(f"  - {error}")
        return 1

    print("PASS: Phase 5 fairness soak checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
