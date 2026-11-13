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

**The GUI soak, 8 hours** (2026-11-09, Windows 11, `soak.py --gui … 480`): the main window stays open and a tour visits every page and tab, records captures and runs the analysis, with the overlay shown and a synthetic game feeding it.

| | Result |
|---|---|
| Private memory after warm-up | 143.5 → 143.3 MB, peak 147.9 MB (+0.48 MB/h): within 150 MB |
| Handles / threads | 498 → 492 / 23 → 23: no leak |
| CPU, whole run | mean 3.17 %, p95 4.79 % of one core |

The CPU figure is the main window touring every page, which is heavier than the 2 % budget's case (sampler plus overlay next to a game), so it is not a measurement of that budget.

**The overlay alone, 30 minutes** (2026-11-09, Windows 11, `soak.py --gui --overlay-only … 30`): the synthetic game, the overlay at its default rate, the main window minimised, no mini monitor, no app history.

| | Result | Budget |
|---|---|---|
| Private memory after warm-up | 118.0 → 119.5 MB, peak 122.2 MB | ≤ 150 MB: **met** |
| Handles / threads | 490 → 493 / 24 → 24 | no leak |
| CPU | mean **3.51 %**, p95 4.58 % of one core (includes the synthetic game's thread) | ≤ 2 %: **not met** |

Where it goes (per-thread CPU over 30 s): the interface thread about 1–1.9 %, two worker threads about 1 % each. With the overlay up, eframe keeps running the main window's interface while it is minimised (the overlay is its child viewport); not building the hidden page was tried and saved little (≈ 3 % instead of 3.5 %), so it was left out. 1.0.0 ships with this number; reaching 2 % is a 1.1.0 goal ([plan, item 9](v1.1/README.md#9-overlay-cpu-down-to-the-2--goal)).

## The overlay alone, per thread (M3-03)

2026-11-13, Windows 11, `soak.py --gui --overlay-only … 30 --interval 30`, the same machine and settings for both builds; the new report lists the busiest threads as a share of one core. `main` is the interface thread (the overlay is its child viewport); `metrics-*` are the sampler's workers.

| Thread | egui 0.36 (`main`, `706b541`) | egui 0.37 (M3-02) |
|---|---|---|
| `main` (interface) | 1.46 % | 0.99 % |
| `cz-procs` | 0.71 % | 0.72 % |
| `metrics-memory` | 0.46 % | 0.44 % |
| unnamed worker | 0.31 % | 0.38 % |
| `metrics-gpu`, `metrics-gpu-engines` | 0.15 %, 0.14 % | 0.13 %, 0.13 % |
| `metrics-frames`, `cz-soak-frames` | 0.08 %, 0.03 % | 0.07 %, 0.03 % |
| **Whole process, mean / p95** | **3.62 % / 5.41 %** | **3.16 % / 4.16 %** |
| Private memory, first → last | 142.8 → 176.9 MB | 155.2 → 189.0 MB |

- The bump to 0.37 does not cost CPU. The interface thread is the largest share in both, and the three workers that sample memory, processes and the GPU come next; the 2 % goal is not met by either.
- **Memory is not flat, on either version.** Private memory grows about 1.5 MB/min (≈ 90 MB/h) from minute 8 to the end, with handles and threads steady, and peaks over the 150 MB budget. The 2026-11-09 run of 1.0.0 on the same machine stayed at 118 → 120 MB. So the growth came in after 1.0.0 and is not from egui 0.37. Not yet bisected; M3-05 cannot pass until it is.
- The per-thread numbers are means over the run, from samples every 30 s: a thread that lives for the whole run is measured well, one that ends early loses its last stretch.

## Left open in 1.1 (M3-04, M3-05)

No further soak was run: the owner closed M3 without one (2026-11-13). Nothing was changed to lower the overlay's CPU, so the numbers above are the 1.1 numbers: **3.2–3.6 % of one core, over the 2 % goal**, with the memory growth noted above.

- **Repaint only when minimised (a):** read on the code, not built. The overlay is drawn from the main window's `ui()` as an immediate child viewport and asks for its own repaint every `1/text_hz` s, so the main window's 500 ms request does not set the pace while the overlay is up.
- **Own viewport, repaint on change, lease fewer sources (b, c, d):** not tried. Per thread, the interface thread is about a third of the total and the rest is sampling workers, so no single change gets from 3 % to 2 %; moving the frame source in-process (2.0) is the larger lever.

## Not measured yet

- **Repeating the overlay run** after work on the 2 % budget:

  ```bash
  cargo build --release --features soak --target-dir target/soak
  python .github/scripts/soak.py --gui --overlay-only target/soak/release/cpuzeez.exe 30 --interval 30 --report soak.md --max-cpu 2 --max-mem-mb 150
  ```

  The report lists CPU, memory, handles and threads, and fails above the budgets.
- **The cost to a game's frame rate** and **sampler-induced stutter**: both need a game and PresentMon.

Until those runs are made and added here, only the numbers above are claimed.
