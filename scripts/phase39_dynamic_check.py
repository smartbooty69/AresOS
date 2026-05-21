#!/usr/bin/env python3
import argparse, os, re, signal, subprocess, sys
PHASE_RE = re.compile(
    r"Phase39-Dynamic:\s+needed=(\d+),\s+linked=(\d+),\s+reloc_ok=(true|false)"
)
def cleanup():
    if os.name == "nt":
        subprocess.run(["taskkill", "/IM", "qemu-system-x86_64.exe", "/F"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
def run_kernel(timeout):
    p = subprocess.Popen(["cargo", "run", "-p", "kernel", "--features", "preemption", "--", "-serial", "stdio", "-display", "none", "-no-reboot"], stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, env=os.environ.copy())
    try:
        o, _ = p.communicate(timeout=timeout); return p.returncode or 0, o
    except subprocess.TimeoutExpired:
        p.send_signal(signal.SIGTERM)
        try: o, _ = p.communicate(timeout=5)
        except subprocess.TimeoutExpired: p.kill(); o, _ = p.communicate(timeout=5)
        cleanup(); return 124, o
def ok(output):
    for line in output.splitlines():
        m = PHASE_RE.search(line)
        if m:
            needed, linked, flag = m.groups()
            return int(needed) >= 1 and int(linked) >= 1 and flag == "true"
    return False
def main():
    parser = argparse.ArgumentParser(); parser.add_argument("--timeout", type=int, default=120); args = parser.parse_args()
    cleanup(); code, output = run_kernel(args.timeout); print(output[-5000:])
    return 0 if ok(output) else 1
if __name__ == "__main__": sys.exit(main())
