# Bottleneck detector

> Part of the [1.0 plan](README.md). Status: built in `1.0.0-beta.7` (S2c); the hardware-balance bar (§4) is cut by [ADR 0014](../adr/0014-no-hardware-balance-bar-in-1-0.md).
> It answers one question: **what is limiting my FPS right now: the GPU, the CPU, memory, temperature, a frame cap, or the PC's configuration?**

## 1. Why most "bottleneck calculators" are wrong

- "Bottleneck calculator" websites compare a CPU's *ranking* with a GPU's and output a percentage. **That doesn't measure anything.**
- The bottleneck changes with:
  - the **game**: strategy titles lean on the CPU, ray-traced shooters on the GPU;
  - the **scene**: a city full of NPCs loads the CPU, an empty desert loads the GPU;
  - **resolution and settings**: at 4K the GPU limits, at 1080p low the CPU limits;
  - **limiters**: V-Sync, G-Sync/FreeSync, in-game FPS caps, Reflex, paused menus.
- So the detector **measures what is happening** and explains it with evidence. A static estimate exists only as secondary context and is labelled as such (§4).

## 2. How to tell who limits, for real

### 2.1 The main signal: GPU Busy vs frame time (Windows)

PresentMon ([zeez-live §3.1](zeez-live.md)) measures, for every frame:
- the **frame time**, the frame's total duration;
- **GPU Busy**, the time the GPU actually worked on it.

The intuition is simple:

```
a 10 ms frame
|██████████| GPU Busy 9.8 ms   → the GPU worked the whole frame  → GPU-bound
|███-------| GPU Busy 3 ms     → the GPU waited 7 ms             → something before it limits (CPU, a cap)
```

- **GPU-bound:** GPU Busy ≥ ~90–95% of frame time, sustained.
- **Not the GPU:** GPU Busy well below frame time. The cause comes from the next steps.

Intel created the GPU Busy metric in PresentMon for exactly this. It's the most reliable signal available without injection.

### 2.2 CPU: the classic "total CPU usage" mistake

- A CPU-bound game almost never shows 100% total CPU. The usual pattern is **one or two threads** (the main thread and the render thread) saturated while the rest idle. On a 16-thread CPU, one thread at 100% shows as only ~6% total.
- So the detector looks at:
  - the **game's busiest thread**, from per-thread CPU time in the System hub (S2a);
  - the **busiest logical core**.
- **CPU-bound** = the GPU has headroom (§2.1) **and** the game's heaviest thread is near 100% of one core.

### 2.3 Limiters (the "nothing is at its limit" case)

When neither the CPU nor the GPU is at its limit, something is holding FPS back on purpose:

| Signal | Likely diagnosis |
|---|---|
| Very regular frame time equal to 1/refresh (16.67 ms at 60 Hz, 6.94 ms at 144 Hz) | **V-Sync**, or **VRR** at its ceiling |
| FPS steady at a round number (60, 120, 141, 237) | **FPS cap** (in-game, driver, Reflex or RTSS) |
| Game in a menu, paused, or unfocused (FPS drops to 30/60) | Background or menu limiter |
| *Composed* present mode | The **compositor** is in the way (another tool's overlay, HDR, windowed mode) |

### 2.4 Other real limits (that players rarely see)

| Bottleneck | How we detect it | Source |
|---|---|---|
| **VRAM overflow** | Dedicated VRAM near 100% **and** shared GPU memory rising (textures spilling into RAM) **and** frame-time spikes | PDH `GPU Process Memory` (Windows), `fdinfo` (Linux) |
| **RAM and paging** | Low available RAM **and** high hard page faults/s, correlated with stutters | OS counters |
| **Disk (asset streaming, shader compilation)** | Game read bursts that line up with stutters; lots of stutters in the first minutes that then fade (shader cache) | Per-process I/O |
| **Thermal throttling** | Clock dropping as temperature hits its limit (Tjmax/hot spot), followed by an FPS drop | `telemetry` (exists) |
| **Power/current limits** | CPU: PPT/TDC/EDC or PL1/PL2 at the ceiling. GPU: *power cap* throttle reason (NVML) | Exists |
| **Background contention** | Another process using CPU/GPU/disk during the session (antivirus, a browser playing video, another game's shader cache) | System hub (S2a) |

### 2.5 Configuration bottlenecks (static, but real and common)

These are inventory checks. Much of the data is already in the report and only needs to become an alert:

| Check | Example alert | Source |
|---|---|---|
| **Single-channel memory** | "One 16 GB stick: the CPU gets half the bandwidth. Use two sticks." | SMBIOS/SPD |
| **XMP/EXPO off** | "Memory runs at 4800 MT/s, but the sticks support 6000 (EXPO). Enable it in the BIOS." | SPD vs current speed |
| **Reduced GPU PCIe link** | "The GPU runs at x4 Gen 3 (max x16 Gen 4): wrong slot, riser, or BIOS setting." | PCI (exists) |
| Resizable BAR off | "ReBAR is off: some games lose 5–10%." | PCI capabilities |
| Power plan / governor | "The 'Power saver' plan is active." / "governor = powersave" | OS |
| Monitor below its supported refresh rate | "Your monitor runs at 60 Hz; it supports 165 Hz." | EDID vs current mode |
| Monitor cable plugged into the motherboard (iGPU) while a dedicated GPU is present | A classic builder mistake | Display topology 🔬 |

## 3. Algorithm: classify each time window

1. Split the session into **1 s windows**.
2. For each window, apply the rules in priority order:
   1. limiter (§2.3);
   2. GPU-bound (§2.1);
   3. CPU-bound (§2.2);
   4. throttling (§2.4);
   5. memory/VRAM/disk on stutters;
   6. "inconclusive".
3. Each rule returns:
   - a **class**;
   - a **confidence** (high, medium or low);
   - **numeric evidence**, e.g. "GPU Busy 97% of frame time, 58 s of 60 s".
4. The session result is the **share of time per class** + a **timeline** + the **top 3 pieces of evidence** + **recommendations**.

**Without GPU Busy** (Linux, macOS, or Windows without admin), the detector falls back to heuristics with **lower confidence**, and **the app shows that on screen**:
- GPU utilization (NVML/amdgpu) ≥ 97% → probably GPU-bound;
- the game's thread saturated while the GPU is < 85% → probably CPU-bound.

**Rules as data, not code scattered around.**
- Each rule is a pure function `fn(&Window) -> Option<Finding>` in `core::bottleneck`, with no OS or GUI dependency.
- Rules are tested against **recorded sessions** (CSV) used as fixtures: "this cs2 session with V-Sync must classify as *limiter*".
- Thresholds live in one documented table, so anyone can audit why the app reached a conclusion ([engineering.md](engineering.md)).

## 4. Hardware balance (static estimate)

The mockup has a "GPU limits ↔ balanced ↔ CPU limits" bar. It's useful as **context**, but it's the easiest part to get wrong. Proposal:
- Base it on **our own measurements**, not internet rankings:
  - the CPU benchmark's *single-thread* score;
  - the GPU benchmark's score (wgpu, [core-pages §7](core-pages.md)), normalized against reference pairs.
- If the benchmarks haven't run, offer "Run the quick tests (≈ 1 min)". Never fall back to a database guess.
- **Always** show the caption: "Estimate for typical games at 1440p. The live detector shows what really happens in each game."
- 🔬 If we can't reach an honest calibration during S2c, the bar is **cut** from 1.0. No number beats an invented one.

## 5. UX (from the mockup)

The mockup is very good: a big verdict, live readings and session analysis. We keep that structure and add the following.

**"Now" card**
- A big verdict, keeping the mockup's tone: "**GPU at its limit**", "**CPU at its limit (core 4)**", "**Nothing at its limit**".
- A context line: `cs2 · 597 FPS · 1% low 154`.
- A plain-language explanation.
- **Confidence and evidence** in an expandable "Why?": "GPU Busy 41% of frame time; cs2's main thread at 98%".

**Live readings**
- Keeps the mockup's bars: GPU load, total CPU load, busiest core, VRAM, RAM, temperatures.
- Adds:
  - **GPU Busy %** (when available);
  - **the game's heaviest thread**;
  - a mini frame-time graph.
- Each bar marks the threshold where its rule fires, so users *see* the why.

**Session analysis**
- Durations of 1, 3 and 5 min (as in the mockup) + "until I stop".
- At the end it shows:
  - a stacked bar of time per class (e.g. "GPU 72% · CPU 18% · Limiter 6% · Inconclusive 4%");
  - a coloured timeline over the FPS chart;
  - the strongest evidence;
  - **recommendations**.

**Recommendations**: always tied to measured evidence, never generic.

| Situation | Recommendation |
|---|---|
| GPU-bound | "Lower settings or resolution, or enable upscaling (DLSS/FSR/XeSS)." If the CPU has headroom: "You can raise CPU-heavy settings (view distance, crowd density) without losing FPS." |
| CPU-bound | "You can raise resolution or graphics 'for free' (the GPU has headroom)." "Close [process X], which used 14% CPU." If XMP is off: the §2.5 alert |
| Thermal | "The CPU reached 95 °C and its clock dropped 18%: check the cooler, thermal paste and airflow." |

- **No snake oil.** No "disable Windows Defender", no "clean the registry".

**Integrations**
- A bottleneck pill on the **ZeeZ Live overlay** ("GPU ▲").
- A section in the **session report**.
- A shortcut from the **Overview**.
- The configuration checks (§2.5) also appear on the Overview as "Build health".

## 6. Support per OS

| | Windows | Linux | macOS |
|---|---|---|---|
| GPU/CPU classification with GPU Busy | 🔑 (PresentMon) | ❌ (no GPU Busy) | ❌ |
| Heuristic from GPU load + game thread | ✅ | ✅ (NVML/amdgpu; i915 ⚠) | ⚠ (GPU load 🔬) |
| Limiters (frame time, cap, V-Sync) | 🔑 | ✅ with MangoHud | ⚠ experimental |
| VRAM / RAM / disk / thermal / power | ✅/🧩 | ✅/🔑 | ⚠ |
| Configuration checks (§2.5) | ✅ | ✅ | ⚠ (Apple Silicon: unified memory, no XMP/PCIe) |

## 7. Delivery (inside stage S2c)

1. Configuration checks (§2.5). They don't depend on frames, so they can land early, as part of the Overview work in S1.
2. Live classification + "Now" card + readings.
3. Session analysis + recommendations + overlay and report integration.
4. *(Conditional)* static balance (§4), if the calibration passes its criterion.
