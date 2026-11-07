# Performance budget (P6, S3-01)

CPU ZeeZ runs next to a game, so it keeps to a budget ([plan §2, P6](v1/README.md)):

| Metric | Budget | How it is checked |
|---|---|---|
| CPU used by CPU ZeeZ, sampler at 1 s and overlay at 4 Hz | ≤ 2% of **one** logical core | `soak.py --gui … --max-cpu 2` (by hand, see below); the engine's own per-tick work is a test (`tick_overhead_budget`) |
| Memory, overlay and capture on | ≤ 150 MB private | `soak.py --gui … --max-mem-mb 150` |
| Frame-rate cost of the overlay and capture | ≤ 1% (mean of 3 runs, PresentMon) | by hand, with a game: it needs one |
| Sampling spikes in frame time | none correlated with sampler ticks | by hand, with a game |

## What has been measured

Machine: Ryzen 9 7900X, Windows 11, release build of the branch before `1.0.0-beta.8`, not elevated, nothing else of ours running.

**The sampler alone** (`cpuzeez -log=…`, no window), 110 s after a 10 s warm-up, CPU time of the process over wall time:

| What | Result |
|---|---|
| Every source read each second (231 sensors) | **3.4%** of one core, 42 MB private, 17 threads, 309 handles |

That run reads *everything*, which the app does not: a page or the overlay *leases* only the sources it shows, and slow ones (storage, SMART) are read every few seconds. Per-source mean read times from the same run (`-log` prints them at the end):

| Source | Mean read | Read |
|---|---|---|
| board (Super I/O, EC) | 10.7 ms | every second |
| dimms | 9.1 ms | every second |
| memory | 7.1 ms | every second |
| cpu | 3.5 ms | every second |
| gpu | 1.9 ms | every second |
| gpu-engines | 1.6 ms | every second |
| net | 1.4 ms | every second |
| frames | 0.2 ms | every second |
| storage | 31.8 ms | every 10 s |

The overlay in its default styles leases `cpu`, `gpu`, `memory` and `frames`: about 12.7 ms per second, i.e. **≈ 1.3% of a core** for sampling. That is an estimate from the table, not a measurement of the running overlay.

## Not measured yet

- **The GUI with the overlay at 4 Hz** (drawing included) against the 2% and 150 MB budgets. It needs a window, so it is a run of the soak by hand:

  ```bash
  cargo build --release --features soak --target-dir target/soak
  python .github/scripts/soak.py --gui target/soak/release/cpuzeez.exe 30 --report soak.md --max-cpu 2 --max-mem-mb 150
  ```

  The report lists CPU, memory, handles and threads, and fails above the budgets.
- **The cost to a game's frame rate** and **sampler-induced stutter**: both need a game and PresentMon.

Until those runs are made and added here, only the numbers above are claimed.
