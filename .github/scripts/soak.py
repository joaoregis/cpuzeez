"""Soak test (S0-T10, S3-02): run for hours and watch for leaks.

Two modes.

  sampler (default, nightly on Linux): `cpuzeez -log=...` reads every sensor
  with the CSV hook on; memory and file descriptors must plateau.

      python soak.py path/to/cpuzeez MINUTES

  gui (`--gui`): the GUI built with `--features soak` runs on its own: a
  synthetic game feeds the frame hub, the overlay is shown, app history
  records, and a tour visits every page and tab, records captures and runs
  the bottleneck analysis. Private memory, handles and threads must plateau.
  Works on Windows, Linux and macOS; settings live in a throw-away profile,
  never the user's.

      cargo build --release --features soak
      python soak.py --gui target/release/cpuzeez.exe 480 --report soak.md

  `--overlay-only` (with `--gui`): the synthetic game and the overlay only,
  main window minimised, no tour: the case the 2 % CPU budget is for.

  `--selftest` checks the analysis itself on synthetic series (CI runs it).
"""
import argparse
import json
import math
import os
import subprocess
import sys
import tempfile
import time

# ---------------------------------------------------------------- analysis


def slope(xs, ys):
    """Least-squares slope of ys over xs (units of y per unit of x)."""
    n = len(xs)
    if n < 2:
        return 0.0
    mx, my = sum(xs) / n, sum(ys) / n
    den = sum((x - mx) ** 2 for x in xs)
    return sum((x - mx) * (y - my) for x, y in zip(xs, ys)) / den if den else 0.0


def analyze(samples, minutes, max_cpu=None, max_mem_mb=None):
    """samples: dicts with minute, private_mb, handles, threads, cpu_pct.
    Returns (problems, summary). Growth is judged after a warm-up: history
    windows, caches and thread pools fill up first."""
    warm = max(10, minutes * 0.15) if minutes >= 40 else minutes * 0.25
    body = [s for s in samples if s["minute"] >= warm] or samples
    hours = [s["minute"] / 60 for s in body]
    mem = [s["private_mb"] for s in body]
    handles = [s["handles"] for s in body]
    threads = [s["threads"] for s in body]
    cpu = sorted(s["cpu_pct"] for s in body if s.get("cpu_pct") is not None)
    sm = {
        "samples": len(samples),
        "warmup_minutes": round(warm, 1),
        "private_mb_first": round(mem[0], 1),
        "private_mb_last": round(mem[-1], 1),
        "private_mb_peak": round(max(s["private_mb"] for s in samples), 1),
        "private_mb_per_hour": round(slope(hours, mem), 2),
        "handles_first": handles[0],
        "handles_last": handles[-1],
        "handles_per_hour": round(slope(hours, handles), 2),
        "threads_first": threads[0],
        "threads_last": threads[-1],
        "cpu_pct_mean": round(sum(cpu) / len(cpu), 2) if cpu else None,
        "cpu_pct_p95": round(cpu[min(len(cpu) - 1, math.ceil(len(cpu) * 0.95) - 1)], 2) if cpu else None,
    }
    problems = []
    growth = mem[-1] - mem[0]
    # Noise-proof: it has to grow steadily AND by a real amount.
    if sm["private_mb_per_hour"] > 10 and growth > 20:
        problems.append(f"private memory grows {sm['private_mb_per_hour']} MB/hour ({growth:+.1f} MB after warm-up): possible leak")
    if mem[-1] > mem[0] * 1.25 + 20:
        problems.append(f"private memory went {mem[0]:.0f} -> {mem[-1]:.0f} MB after warm-up")
    if handles[-1] > handles[0] + 16 and sm["handles_per_hour"] > 2:
        problems.append(f"handles keep growing: {handles[0]} -> {handles[-1]}")
    if threads[-1] > threads[0] + 4:
        problems.append(f"threads keep growing: {threads[0]} -> {threads[-1]}")
    if max_mem_mb is not None and sm["private_mb_peak"] > max_mem_mb:
        problems.append(f"peak private memory {sm['private_mb_peak']} MB is over the {max_mem_mb} MB budget")
    if max_cpu is not None and sm["cpu_pct_mean"] is not None and sm["cpu_pct_mean"] > max_cpu:
        problems.append(f"mean CPU {sm['cpu_pct_mean']} % of one core is over the {max_cpu} % budget")
    return problems, sm


def selftest():
    def series(f, minutes=240, step=1):
        return [dict(minute=m, **f(m)) for m in range(0, minutes + 1, step)]

    flat = series(lambda m: dict(private_mb=90 + min(m, 8) * 2 + (m % 3) * 0.2, handles=300, threads=40, cpu_pct=1.2))
    assert not analyze(flat, 240)[0], analyze(flat, 240)
    leak = series(lambda m: dict(private_mb=90 + m * 0.5, handles=300, threads=40, cpu_pct=1.0))
    assert any("memory" in p for p in analyze(leak, 240)[0]), "a steady leak must fail"
    hand = series(lambda m: dict(private_mb=95, handles=300 + m // 2, threads=40, cpu_pct=1.0))
    assert any("handles" in p for p in analyze(hand, 240)[0]), "growing handles must fail"
    thr = series(lambda m: dict(private_mb=95, handles=300, threads=40 + m // 30, cpu_pct=1.0))
    assert any("threads" in p for p in analyze(thr, 240)[0]), "growing threads must fail"
    # Warm-up growth is not a leak.
    warm = series(lambda m: dict(private_mb=60 + min(m, 30) * 2, handles=300, threads=40, cpu_pct=1.0))
    assert not analyze(warm, 240)[0], analyze(warm, 240)
    # Budgets only apply when asked for.
    assert any("budget" in p for p in analyze(flat, 240, max_cpu=1.0)[0])
    assert any("budget" in p for p in analyze(flat, 240, max_mem_mb=50)[0])
    assert not analyze(flat, 240, max_cpu=2.0, max_mem_mb=150)[0]
    short = series(lambda m: dict(private_mb=100, handles=300, threads=40, cpu_pct=1.0), 12)
    assert not analyze(short, 12)[0]
    assert abs(slope([0, 1, 2], [0, 2, 4]) - 2) < 1e-9
    lines = ["noise"] + [f"soak-frame overlay_ms={0.5 + i * 0.01:.4f} frames=240" for i in range(8)]
    t = frame_trend(lines)
    assert t["overlay_frame_ms_first_quarter"] < t["overlay_frame_ms_last_quarter"], t
    assert abs(t["overlay_frame_ms_per_hour"] - 0.6) < 1e-6, t
    assert frame_trend(["soak-frame overlay_ms=1 frames=1"]) == {}, "too few minutes: no trend"
    print("soak selftest: ok")


# ---------------------------------------------------------------- sampling


def _windows_sampler():
    import ctypes
    from ctypes import wintypes as w

    k32, psapi = ctypes.WinDLL("kernel32", use_last_error=True), ctypes.WinDLL("psapi", use_last_error=True)

    class MEM(ctypes.Structure):
        _fields_ = [("cb", w.DWORD), ("PageFaultCount", w.DWORD)] + [
            (n, ctypes.c_size_t)
            for n in (
                "PeakWorkingSetSize", "WorkingSetSize", "QuotaPeakPagedPoolUsage", "QuotaPagedPoolUsage",
                "QuotaPeakNonPagedPoolUsage", "QuotaNonPagedPoolUsage", "PagefileUsage", "PeakPagefileUsage", "PrivateUsage",
            )
        ]

    class FT(ctypes.Structure):
        _fields_ = [("lo", w.DWORD), ("hi", w.DWORD)]

    class TH(ctypes.Structure):
        _fields_ = [("dwSize", w.DWORD), ("cntUsage", w.DWORD), ("th32ThreadID", w.DWORD), ("th32OwnerProcessID", w.DWORD),
                    ("tpBasePri", w.LONG), ("tpDeltaPri", w.LONG), ("dwFlags", w.DWORD)]

    k32.OpenProcess.restype = w.HANDLE
    k32.CreateToolhelp32Snapshot.restype = w.HANDLE
    k32.CreateToolhelp32Snapshot.argtypes = [w.DWORD, w.DWORD]
    k32.Thread32First.argtypes = k32.Thread32Next.argtypes = [w.HANDLE, ctypes.POINTER(TH)]
    k32.GetProcessHandleCount.argtypes = [w.HANDLE, ctypes.POINTER(w.DWORD)]
    k32.GetProcessTimes.argtypes = [w.HANDLE] + [ctypes.POINTER(FT)] * 4
    psapi.GetProcessMemoryInfo.argtypes = [w.HANDLE, ctypes.POINTER(MEM), w.DWORD]
    k32.CloseHandle.argtypes = [w.HANDLE]

    def sample(pid):
        h = k32.OpenProcess(0x1000, False, pid)  # PROCESS_QUERY_LIMITED_INFORMATION
        if not h:
            raise OSError("cannot open the process")
        try:
            m = MEM()
            m.cb = ctypes.sizeof(m)
            psapi.GetProcessMemoryInfo(h, ctypes.byref(m), m.cb)
            hc = w.DWORD()
            k32.GetProcessHandleCount(h, ctypes.byref(hc))
            c, e, kt, ut = FT(), FT(), FT(), FT()
            k32.GetProcessTimes(h, *map(ctypes.byref, (c, e, kt, ut)))
            cpu_s = sum((x.hi << 32 | x.lo) for x in (kt, ut)) / 1e7
        finally:
            k32.CloseHandle(h)
        snap = k32.CreateToolhelp32Snapshot(4, 0)  # TH32CS_SNAPTHREAD
        threads = 0
        t = TH()
        t.dwSize = ctypes.sizeof(t)
        ok = k32.Thread32First(snap, ctypes.byref(t))
        while ok:
            threads += t.th32OwnerProcessID == pid
            ok = k32.Thread32Next(snap, ctypes.byref(t))
        k32.CloseHandle(snap)
        return dict(private_mb=m.PrivateUsage / 1048576, ws_mb=m.WorkingSetSize / 1048576, handles=hc.value, threads=threads, cpu_s=cpu_s)

    return sample


def _linux_sampler():
    tck = os.sysconf("SC_CLK_TCK")

    def sample(pid):
        st = dict(l.split(":", 1) for l in open(f"/proc/{pid}/status") if ":" in l)
        f = open(f"/proc/{pid}/stat").read().rsplit(")", 1)[1].split()
        rss = int(st["VmRSS"].split()[0]) / 1024
        priv = rss
        try:  # RssAnon is the private part
            priv = int(st["RssAnon"].split()[0]) / 1024
        except KeyError:
            pass
        return dict(private_mb=priv, ws_mb=rss, handles=len(os.listdir(f"/proc/{pid}/fd")), threads=int(st["Threads"]),
                    cpu_s=(int(f[11]) + int(f[12])) / tck)

    return sample


def _macos_sampler():
    def sample(pid):
        out = subprocess.run(["ps", "-o", "rss=,cputime=", "-p", str(pid)], capture_output=True, text=True).stdout.split()
        rss = int(out[0]) / 1024
        h, m, s = out[1].split(":") if out[1].count(":") == 2 else ("0", *out[1].split(":"))
        threads = max(0, len(subprocess.run(["ps", "-M", "-p", str(pid)], capture_output=True, text=True).stdout.splitlines()) - 1)
        fds = max(0, len(subprocess.run(["lsof", "-p", str(pid)], capture_output=True, text=True).stdout.splitlines()) - 1)
        return dict(private_mb=rss, ws_mb=rss, handles=fds, threads=threads, cpu_s=int(h) * 3600 + int(m) * 60 + float(s))

    return sample


def make_sampler():
    if sys.platform == "win32":
        return _windows_sampler()
    return _macos_sampler() if sys.platform == "darwin" else _linux_sampler()


# ---------------------------------------------------------------- the profile and the run


def profile(root, overlay_only=False):
    """A throw-away profile: where the app keeps its data, and what it opens with."""
    if sys.platform == "win32":
        env, data = {"APPDATA": root}, os.path.join(root, "CpuZeeZ")
    elif sys.platform == "darwin":
        env, data = {"HOME": root}, os.path.join(root, "Library", "Application Support", "CpuZeeZ")
    else:
        env, data = {"HOME": root, "XDG_CONFIG_HOME": root}, os.path.join(root, "cpuzeez")
    os.makedirs(data, exist_ok=True)
    settings = {
        "privacy_seen": 1000,  # no welcome screen
        "default_start_page": "Overview",
        "app_history_enabled": True,
        "overlay": {"shown": True, "first_use_done": True},
        "minis": [{"open": True}],
        "mini": True,
    }
    if overlay_only:  # the overlay next to a game: no mini monitor, no history
        settings.update(minis=[], mini=False, app_history_enabled=False)
    open(os.path.join(data, "settings.json"), "w").write(json.dumps(settings))
    env["CPUZEEZ_SOAK"] = "overlay" if overlay_only else "1"
    return env, data


def frame_trend(log_lines):
    """The overlay's drawing time from the soak build's stderr
    (`soak-frame overlay_ms=… frames=…`, once a minute): first and last
    quarter averages and the slope per hour. Reported, not judged: drawing
    time depends on the machine; a steady climb is what to look for."""
    ms = []
    for line in log_lines:
        if line.startswith("soak-frame "):
            fields = dict(f.split("=", 1) for f in line.split()[1:] if "=" in f)
            try:
                ms.append(float(fields["overlay_ms"]))
            except (KeyError, ValueError):
                pass
    if len(ms) < 4:
        return {}
    q = max(1, len(ms) // 4)
    avg = lambda v: round(sum(v) / len(v), 4)
    return {
        "overlay_frame_ms_first_quarter": avg(ms[:q]),
        "overlay_frame_ms_last_quarter": avg(ms[-q:]),
        "overlay_frame_ms_per_hour": round(slope(list(range(len(ms))), ms) * 60, 4),
    }


def report(path, mode, minutes, problems, sm, extra):
    lines = [f"# Soak: {mode}, {minutes} min", "", f"Result: **{'FAILED' if problems else 'ok'}**", ""]
    lines += [f"- {k}: {v}" for k, v in {**sm, **extra}.items()]
    lines += [""] + [f"- PROBLEM: {p}" for p in problems]
    text = "\n".join(lines) + "\n"
    print(text)
    if path:
        open(path, "w").write(text)
        open(os.path.splitext(path)[0] + ".json", "w").write(json.dumps({"problems": problems, **sm, **extra}, indent=2))


def run_gui(exe, minutes, interval, out, max_cpu, max_mem, overlay_only=False):
    exe = os.path.abspath(exe)
    if not os.path.isfile(exe):
        sys.exit(f"no such executable: {exe}")
    root = tempfile.mkdtemp(prefix="cz-soak-")
    env, data = profile(root, overlay_only)
    log = open(os.path.join(root, "stderr.log"), "w")
    p = subprocess.Popen([exe], env={**os.environ, **env}, stderr=log, stdout=log)
    sample, samples = make_sampler(), []
    last = None
    start = time.time()
    try:
        time.sleep(20)  # startup
        while (time.time() - start) / 60 < minutes:
            if p.poll() is not None:
                break
            s = sample(p.pid)
            now = time.time()
            cpu = None if last is None else (s["cpu_s"] - last[1]) / (now - last[0]) * 100
            last = (now, s["cpu_s"])
            samples.append(dict(minute=(now - start) / 60, cpu_pct=cpu, **s))
            print(f"{samples[-1]['minute']:7.1f} min  private {s['private_mb']:7.1f} MB  handles {s['handles']:5}  threads {s['threads']:3}"
                  + (f"  cpu {cpu:4.1f} %" if cpu is not None else ""), flush=True)
            time.sleep(interval)
    finally:
        crashed = p.poll() is not None
        code = p.returncode
        if not crashed:
            p.terminate()
            try:
                p.wait(20)
            except subprocess.TimeoutExpired:
                p.kill()
    problems, sm = ([], {})
    if crashed:
        problems.append(f"the app exited early with code {code}")
    if len(samples) >= 3:
        pr, sm = analyze(samples, minutes, max_cpu, max_mem)
        problems += pr
    elif not crashed:
        problems.append("too few samples to judge")
    # It has to have done the work, or flat numbers prove nothing.
    sessions = len([d for d in os.listdir(os.path.join(data, "sessions"))]) if os.path.isdir(os.path.join(data, "sessions")) else 0
    rows = 0
    db = os.path.join(data, "app_history.sqlite")
    if os.path.exists(db):
        import sqlite3

        rows = sqlite3.connect(db).execute("select count(*) from app_day").fetchone()[0]
    if overlay_only:
        pass  # no tour: nothing is recorded on purpose
    elif minutes >= 12 and sessions < 1:
        problems.append("no capture was recorded: the tour didn't run")
    if not overlay_only and minutes >= 5 and rows < 1:
        problems.append("app history recorded nothing")
    log.close()
    trend = frame_trend(open(os.path.join(root, "stderr.log"), errors="replace").read().splitlines())
    report(out, "overlay" if overlay_only else "gui", minutes, problems, sm, {"captures_recorded": sessions, "history_rows": rows, **trend})
    if problems and os.path.exists(os.path.join(root, "stderr.log")):
        print("--- last log lines ---")
        print("".join(open(os.path.join(root, "stderr.log")).readlines()[-15:]))
    return 1 if problems else 0


def run_sampler(exe, minutes):
    exe = os.path.abspath(exe)
    log = os.path.join(tempfile.mkdtemp(), "soak.csv")
    p = subprocess.Popen([exe, f"-log={log},{minutes * 60 + 30}"], stderr=subprocess.DEVNULL)
    sample, samples = make_sampler(), []
    try:
        for m in range(minutes):
            time.sleep(60)
            if p.poll() is not None:
                sys.exit(f"cpuzeez exited early with code {p.returncode}")
            s = sample(p.pid)
            samples.append(dict(minute=m + 1, cpu_pct=None, **s))
            print(f"{m + 1:4} min  private {s['private_mb']:7.1f} MB  handles {s['handles']}  threads {s['threads']}", flush=True)
    finally:
        p.terminate()
    problems, sm = analyze(samples, minutes)
    rows = sum(1 for line in open(log) if not line.startswith(("time,", "#")))
    if rows < minutes * 50:
        problems.append(f"only {rows} samples in {minutes} min: the sampler stalled")
    report(None, "sampler", minutes, problems, sm, {"csv_rows": rows})
    return 1 if problems else 0


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("exe", nargs="?")
    ap.add_argument("minutes", nargs="?", type=int)
    ap.add_argument("--gui", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--overlay-only", action="store_true", help="with --gui: no tour, main window minimised")
    ap.add_argument("--interval", type=int, default=60, help="seconds between samples")
    ap.add_argument("--report", help="write a Markdown (and .json) report here")
    ap.add_argument("--max-cpu", type=float, help="fail above this mean CPU, %% of one core")
    ap.add_argument("--max-mem-mb", type=float, help="fail above this peak private memory")
    a = ap.parse_args()
    if a.selftest:
        selftest()
        sys.exit(0)
    if not a.exe or not a.minutes:
        ap.error("usage: soak.py [--gui] EXE MINUTES")
    sys.exit(run_gui(a.exe, a.minutes, a.interval, a.report, a.max_cpu, a.max_mem_mb, a.overlay_only) if a.gui else run_sampler(a.exe, a.minutes))
