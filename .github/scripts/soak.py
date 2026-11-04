"""Soak test (S0-T10): run the sampler for a long time and watch for leaks.

Starts `cpuzeez -log=...` (every sensor, CSV hook on), samples the process's
resident memory and open file descriptors every minute on Linux, and fails
if, after a warm-up, either keeps growing beyond a tolerance.

usage: python soak.py path/to/cpuzeez MINUTES
"""
import os
import subprocess
import sys
import tempfile
import time

exe, minutes = sys.argv[1], int(sys.argv[2])
log = os.path.join(tempfile.mkdtemp(), "soak.csv")
p = subprocess.Popen([exe, f"-log={log},{minutes * 60 + 30}"], stderr=subprocess.DEVNULL)


def rss_kb(pid):
    for line in open(f"/proc/{pid}/status"):
        if line.startswith("VmRSS:"):
            return int(line.split()[1])
    return 0


def fds(pid):
    return len(os.listdir(f"/proc/{pid}/fd"))


samples = []
try:
    for m in range(minutes):
        time.sleep(60)
        if p.poll() is not None:
            sys.exit(f"cpuzeez exited early with code {p.returncode}")
        samples.append((m + 1, rss_kb(p.pid), fds(p.pid)))
        print(f"{m + 1:4} min  RSS {samples[-1][1] / 1024:7.1f} MB  fds {samples[-1][2]}", flush=True)
finally:
    p.terminate()

warm = [s for s in samples if s[0] >= 5] or samples
first, last = warm[0], warm[-1]
growth_mb = (last[1] - first[1]) / 1024
rows = sum(1 for line in open(log) if not line.startswith(("time,", "#")))
print(f"after warm-up: RSS {first[1] / 1024:.1f} -> {last[1] / 1024:.1f} MB ({growth_mb:+.1f}), fds {first[2]} -> {last[2]}, {rows} CSV rows")
# Tolerance: history is bounded (graph window), so memory must plateau.
if growth_mb > 20 or last[1] > first[1] * 1.25:
    sys.exit("memory keeps growing: possible leak")
if last[2] > first[2] + 8:
    sys.exit("file descriptors keep growing: possible leak")
if rows < minutes * 50:
    sys.exit(f"only {rows} samples in {minutes} min: the sampler stalled")
print("soak: ok")
