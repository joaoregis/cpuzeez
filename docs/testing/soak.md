# Soak test (S0-T10, S3-02)

A soak test runs the app for hours and checks that nothing grows: private memory, handles (file descriptors on Linux/macOS) and threads must reach a plateau. It catches leaks that no unit test can.

## Two modes

| Mode | Runs | Where | Command |
|---|---|---|---|
| **sampler** | `cpuzeez -log=…` reads every sensor with the CSV hook on | nightly CI, Linux | `python .github/scripts/soak.py target/release/cpuzeez 60` |
| **gui** | The real GUI with a synthetic game | by hand, on a machine nobody is using | see below |

The GUI mode needs a window, so it never runs in CI. CI runs only the parts that need no window: `cargo test --bin cpuzeez --features soak soak` (the synthetic game produces the scenes it claims) and `python .github/scripts/soak.py --selftest` (the analysis catches a leak and ignores a plateau).

## GUI mode

```bash
cargo build --release --features soak --target-dir target/soak
python .github/scripts/soak.py --gui target/soak/release/cpuzeez.exe 480 --report soak.md
```

The `soak` feature is in no release. It also needs `CPUZEEZ_SOAK=1`, which the script sets; a soak build started by mistake behaves like the normal app. The script uses a throw-away settings folder and never touches the real profile.

**Overlay only.** `--overlay-only` (with `--gui`) sets `CPUZEEZ_SOAK=overlay`: the same synthetic game and the overlay, the main window minimised, no tour, no mini monitor, no app history, no captures. That is the case the 2 % CPU budget is for ([performance](../performance.md)); the 8-hour run with the tour measures something heavier.

**What runs.**
- A synthetic game (`soak-game.exe`, pid 4242) feeds the frame hub every 100 ms, in five scenes of 40 s: GPU-bound, CPU-bound, V-Sync at 60 Hz, a 120 FPS cap, and a loading stretch with uneven frames. One frame in 20 s is a 60 ms hitch.
- The overlay is shown and hidden every 30 minutes; app history records.
- A tour (`src/gui/soak.rs`) changes page every 15 s, cycling every page and tab (System tabs, Live tabs).
- Every 9 minutes a 60 s capture; every 11 minutes a bottleneck analysis.

**What is judged** (after a warm-up of 15% of the run, at least 10 minutes, because history windows, caches and thread pools fill first):

| Signal | Fails when |
|---|---|
| Private memory | slope > 10 MB/h **and** growth > 20 MB, or last > first × 1.25 + 20 MB |
| Handles | growth > 16, or a trend above 2 per hour |
| Threads | growth > 4 |
| Captures | none in a run of 12 minutes or more |
| History | no row in a run of 5 minutes or more |
| CPU, memory budget | optional: `--max-cpu 2 --max-mem-mb 150` (the P6 budgets) |

The report (`--report`) lists the numbers and the verdict, and the script exits non-zero on any failure.

## Checking the analysis itself

```bash
python .github/scripts/soak.py --selftest
```

feeds the analysis synthetic series and checks the verdicts: a plateau passes; a steady memory leak, growing handles and growing threads each fail; warm-up growth is not a leak; the CPU and memory budgets apply only when asked for.

## Results

A run is evidence only if its report is kept. Add each run's numbers to the release notes of the version it validated; the 8-hour run is scheduled with the person who owns the machine, because it opens a window.
