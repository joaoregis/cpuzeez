# ADR 0014: No "hardware balance" bar in 1.0

- **Status:** accepted (2026-11-06)
- **Decision ref:** S2c-06 · details in [docs/v1/bottleneck-detector.md](../v1/bottleneck-detector.md#4-hardware-balance-static-estimate)

## Context
The detector's mockup had a "GPU limits ↔ balanced ↔ CPU limits" bar for a given CPU and GPU. The plan allowed it only if it could be calibrated honestly from CpuZeeZ's own benchmarks (CPU single-thread score, GPU score), with no database of rankings, and said to cut it otherwise: "no number beats an invented one".

## Decision
It is cut from 1.0. The detector reports only what it measures: GPU Busy against frame time, the game's busiest thread, limiter, throttling, memory and disk signals, each with its evidence.

## Why
- A calibration needs reference pairs: many CPU+GPU combinations, each with its benchmark scores **and** measured behaviour in real games at a stated resolution. CpuZeeZ has the benchmark scores of the machines it was built on, and one real capture. That is not a calibration; any bar drawn from it would be an invented number.
- What limits FPS depends on the game, the scene, the resolution and the settings. The live detector already answers it per game, and that answer is the one worth trusting.
- The configuration checks that do hold (single-channel memory, memory profile off, reduced PCIe link, Resizable BAR off, refresh rate, power plan) already show as *Build health* on the Overview, because each is a fact about this PC, not an estimate.

## Alternatives considered
- A bar from internet CPU/GPU rankings: rejected, it is the "bottleneck calculator" the plan calls wrong (bottleneck-detector §1).
- A bar from our benchmarks with a wide "can't tell" middle: rejected for now: without reference data the middle would swallow every case.

## Consequences
- No static estimate in the app. If CpuZeeZ ever gathers enough opt-in, local, measured pairs to validate one, this ADR is revisited; until then, nothing is shown.
