# CpuZeeZ 1.0: the plan

> **Status:** design approved; all decisions recorded in §8. Not implemented yet.
> **Baseline:** 0.3.0 (`26d9f0e`) · **Date:** 2026-11-04.
> This is the hub. Each area has its own document.

| Document | Contents |
|---|---|
| [ROADMAP.md](ROADMAP.md) | **The detailed work list:** every fix, foundation piece, improvement, feature, spike, packaging task and doc change, by stage, with IDs and "done when" |
| [zeez-live.md](zeez-live.md) | **New flagship feature, ZeeZ Live:** in-game/in-app overlay, session capture and reports |
| [bottleneck-detector.md](bottleneck-detector.md) | **New feature:** bottleneck detector (CPU, GPU, memory, thermals, frame caps, PC configuration) |
| [task-manager.md](task-manager.md) | **New feature:** cross-platform Task Manager under a "System" hub (processes, performance, app history, startup, users, details, services, settings) |
| [core-pages.md](core-pages.md) | Improvements to the existing pages (Overview, Processor, Mainboard, Memory, Storage/Devices, Benchmarks/Tools) and to the tray icon and mini monitor |
| [engineering.md](engineering.md) | **Applies to everything:** layers and workspace, per-OS contracts, shared building blocks, concurrency, errors, `unsafe`, testing and quality gates, ADRs |
| [dependencies.md](dependencies.md) | What we compile in, what must stay outside the binary, and how we remove install friction for users |
| [evidence/](evidence/) | Static-analysis scripts used as evidence (imports and strings of binaries) |

Legend used in every document (the same as [FEATURES.md](../../FEATURES.md)):
✅ user mode · 🔑 needs admin/root · 🧩 needs a kernel driver (Windows: PawnIO) · ⚠ partial or equivalent data · ❌ impossible on that OS (with the reason) · 🔬 needs a *spike* (prove it before promising it).

---

## 1. Goal

1.0 is a **release-ready** version. It must be stable and trustworthy for real use by benchmarkers, gamers, enthusiasts, professionals and PC builders, on **Windows, Linux and macOS**. Features are left out of an OS only when the limit cannot be worked around, and every such limit is documented together with its reason.

**Everything ships together in 1.0.0.** The stages in §6 are internal milestones, each closed by an automated quality gate. They are not public releases.

## 2. Principles (non-negotiable)

**P1. Local-first: users own their data.**
- No data leaves the machine. There is no telemetry, no account, no cloud and no upload.
- Everything lives in the data folder: `%APPDATA%\CpuZeeZ`, `~/Library/Application Support/CpuZeeZ` or `$XDG_CONFIG_HOME/cpuzeez`.
- *Settings › Privacy & data* shows **what is stored and how much space it uses**. From there the user can open the folder, export, delete by category or delete everything, and set **retention** (for example, sessions for 90 days and app history for 30 days).
- The **only network call** is the update check, an anonymous `GET` to GitHub's releases API. Today it is **on by default** (`CheckUpdates=1`, `src/update.rs`). In 1.0 it becomes **opt-in**: the app asks on first run, and it can be switched off at any time.
- External links ("Search online", "Open in VirusTotal") are opened in the browser only when the user clicks them. The app never calls them on its own.

**P2. Consent: nothing is recorded without opt-in.**
- **Reading and displaying** live data needs no consent: the value is shown and then discarded.
- Anything that **records activity over time** starts disabled and is enabled only by an explicit opt-in. That covers app history, session capture, CSV logs, automatic per-game capture and startup history.
- While anything is recording, a **visible indicator** (●) shows in the window title, the tray and the overlay, and one click stops it.
- First run shows a short **Privacy & permissions** screen. It explains what the app does and does not do, and holds the update opt-in, the recording opt-ins and the explanation of admin/root.

**P3. Transparent privileges.**
- The app always starts **unprivileged**.
- When a screen needs more, it shows a **capability banner**, for example: "Without administrator rights: per-core temperatures, SPD, memory timings and game FPS are not available. [Restart as administrator] [Why?]".
- One-off actions ask for elevation **for that action only**, through UAC, polkit/pkexec or the macOS authorization prompt.
- Every banner says **exactly what is missing** without the permission. The same map is in §5 and goes into the README.

**P4. Safety first.**
- **Never** inject code into other processes. It risks anti-cheat bans and crashes.
- Never use vulnerable drivers. Windows keeps using PawnIO, never WinRing0.
- Never read another process's memory without need. Game processes are opened only with `PROCESS_QUERY_LIMITED_INFORMATION`.
- Destructive actions (ending a process, disabling a service, changing startup items) always ask for confirmation and warn when the target is critical to the system.
- Any privileged component has the smallest possible surface (§4.3).

**P5. Honest cross-platform parity.** Every feature has a Windows/Linux/macOS table with the data source and the privilege it needs. When something is ⚠ or ❌, the UI explains why instead of showing an empty field.

**P6. Minimal overhead, because the app runs next to the game.** 1.0 targets, measured in CI and on real hardware:

| Metric | Target |
|---|---|
| FPS impact with overlay + capture on | ≤ 1% (mean of 3 runs, measured with PresentMon) |
| CpuZeeZ CPU use, sampler at 1 s and overlay at 4 Hz | ≤ 2% of **one** logical core |
| Resident memory, overlay + capture | ≤ 150 MB |
| Sampler-induced stutter | No frame-time spikes correlated with sampler ticks |

**P7. Transparent documentation.** A feature is "done" only when README, README.pt-BR, FEATURES.md, CHANGELOG and the in-app text explain:
- where each value comes from;
- which permission it needs, and what is lost without it;
- what is recorded and where;
- its per-OS limits.

**P8. Zero install friction.** Whatever can be compiled into the binary is compiled in. Whatever must live outside it (a kernel driver, for example) is offered by the installer or by an in-app button. It is never installed silently and never left for the user to hunt down. See [dependencies.md](dependencies.md).

## 3. Diagnosis of 0.3.0: what to fix before building on it

Items marked ⛔ block the overlay or the Task Manager.

| # | Issue | Where | Why it matters |
|---|---|---|---|
| D1 ⛔ | The effective clock via APERF/MPERF **busy-spins 30 ms per core, one core after another** | `src/cpu/clocks.rs:76-81` | About **480 ms of CPU per sample** on 16 cores, taken from the game. There is also an 80 ms-per-CPU fallback loop (`clocks.rs:64`) |
| D2 ⛔ | On Windows every `rdmsr` spawns a thread to pin to the target CPU | `src/os.rs:45`, `src/hw/windows.rs:385` | Cost and jitter on every read |
| D3 | The SMART probe of every drive runs **inside the sampler** every 10 s | `src/telemetry.rs:310` | I/O latency stalls the sample |
| D4 | macOS spawns `ioreg` twice **per sample** (battery) | `src/battery.rs:393`, `src/sensors/macos.rs:208` | A process spawn every second |
| D5 | `series()` clones the whole history on every call, and the GUI runs a **second, separate sampler** | `telemetry.rs:672,748`, `gui/mod.rs:259` | Cost grows with history; two sources of truth |
| D6 ⛔ | No reusable table (sort, filter, columns) and no virtualization (`show_rows`) | `gui/*` | The Task Manager has thousands of rows |
| D7 ⛔ | No per-process, per-disk I/O or per-adapter throughput data | — | It is the basis of the Task Manager, ZeeZ Live and the bottleneck detector |
| D8 ⛔ | No elevation flow (ask for admin, restart as admin) | — | P3 |
| D9 | Changing `sample_ms`/`graph_seconds` in Settings does not restart the sampler, and the mini monitor is not persisted | `gui/settings_ui.rs`, `gui/sensors.rs:281` | UX bugs |
| D10 | FFI is hand-written (`extern` blocks) throughout | — | The Task Manager needs 100+ new Win32 APIs. See [dependencies.md](dependencies.md) |

## 4. Target architecture (what the features share)

The central idea: **ZeeZ Live, the mini monitor, the tray, the Overview, the Task Manager and the bottleneck detector are all *consumers* of the same parts.** We build those parts once.

```
             ┌──────────────────── Sources (per OS) ─────────────────────┐
             │ sensors (PawnIO/hwmon/SMC) · GPU (NVML/ADL/sysfs/IOKit)    │
             │ processes (NtQSI/procfs/libproc) · disk · network          │
             │ frames (PresentMon/MangoHud/Metal) · services · startup    │
             └──────────────────────────────┬────────────────────────────┘
                                            ▼
             ┌──────────────── Metric Engine (new) ──────────────────────┐
             │ catalog of metrics with stable IDs · per-source cost       │
             │ subscriptions: collect only what someone shows or records  │
             │ history rings · statistics · cheap snapshots               │
             └───────┬──────────────┬──────────────┬─────────────┬───────┘
                     ▼              ▼              ▼             ▼
               Widget Engine    Recorder      Bottleneck     Pages (Sensors,
               (JSON layouts)  (sessions,     detector       System, Overview…)
                ┌──┴──┬────┐   history)       (rules)
            Overlay Mini Tray
```

### 4.1 Metric Engine (replaces today's `telemetry` and the GUI sampler)

- **Catalog:** keeps the existing ID format (`<device>/<kind>/<slug>`, `telemetry.rs:7-12`) and adds `proc/<pid>/…`, `disk/<n>/…`, `net/<if>/…` and `frame/<pid>/…`.
- **Per-metric metadata:** unit, group, source, **cost class** (cheap, medium or expensive), required **privilege**, and minimum and maximum rate.
- **Subscriptions:** each consumer declares what it needs and at which rate, and the engine polls only metrics that have a subscriber. With the overlay showing "FPS + GPU temp", for example, nothing reads SMART or the Super I/O.
- **Rates by class:**

| Class | Rate | Examples |
|---|---|---|
| Event | as events arrive | frames, from a stream |
| Fast | 250 ms to 1 s | sensors, load, clocks |
| Slow | 10 to 60 s | SMART, battery |
| On demand | when asked | inventory |

  Slow sources run on their own worker and never block the fast path. This fixes D3 and D4.
- **Effective clock without busy-spinning (fixes D1):** APERF, MPERF and the TSC are free-running counters. Read all three **once per core per tick** and compute the **difference from the previous tick**:
  - *active clock* = f_TSC × ΔAPERF/ΔMPERF;
  - *effective clock* (HWiNFO-style, idle time included) = ΔAPERF/Δt.
  
  The cost drops from 30 ms per core to a few microseconds. To pin, one sampler thread changes its own affinity instead of spawning threads (fixes D2).
- **Cheap snapshots (fixes D5):** consumers read the current value in O(1) and fetch series incrementally (`since(t)`). Nothing clones the whole history.

### 4.2 Widget Engine (shared layouts)

- One model, `Layout { widgets: [Widget { metric, kind, format, thresholds, style }] }`, serialized as JSON and edited with **a single visual editor**.
- Four surfaces use it: the **ZeeZ Live overlay** (above the game, click-through), the **mini monitor** (a desktop window), the **tray** (icon and tooltip) and the **Overview tiles**.
- The benefit: improve one widget, for example the frame-time graph, and the overlay, the mini monitor and the Overview all get it at once.

### 4.3 Privileges: getting admin/root (decided, Q2)

| Option | How it works | Pros | Cons |
|---|---|---|---|
| **A. Restart as admin** | A button relaunches the app elevated: Windows `ShellExecuteEx` with the `runas` verb, Linux `pkexec`, macOS Authorization Services | Simple, no new component | One UAC prompt per launch. On Linux, running the whole GUI as root is discouraged (Wayland/X11) |
| **B. Per-action elevation** | Each privileged action (stop a service, end an elevated process, change a machine-wide startup item) runs an elevated `cpuzeez` subprocess with one specific command, e.g. `cpuzeez --elevated-op=service-stop:Spooler`. On Linux, systemd and logind already ask polkit over D-Bus | The main app never runs elevated | One prompt per action. Not suitable for continuous reads (sensors, FPS) |
| **C. Optional read-only helper service**, installed with consent | A Windows service, systemd unit or LaunchDaemon. **Read-only**: PawnIO, ETW/PresentMon, SMART. It serves the GUI over a local channel (named pipe or socket with an ACL) | Full sensors and FPS **without a UAC prompt at each launch** | Another component to maintain. A service running as SYSTEM is a privilege-escalation target, so its surface must be minimal and audited |

**Decision:**
- **1.0 = A + B.** The app never runs elevated unless the user asks, and every write goes through B.
- **C comes after 1.0.** It addresses the daily-gamer pain (FPS and sensors without UAC), under strict rules:
  - **read-only**;
  - no MSR or port writes;
  - the IPC accepts local interactive users only;
  - commands come from an allow-list;
  - it can be uninstalled at any time.

### 4.4 Local storage

| Data | Format | Why |
|---|---|---|
| Settings and layouts | JSON (as today, `settings.json`) and `layouts/*.json` | Human-readable, can be versioned |
| Capture sessions | `sessions/<date>_<app>/frames.csv` (**PresentMon-compatible** columns) + `hw.csv` + `session.json` (summary) | Interoperability: CapFrameX and other tools import PresentMon CSV |
| App and boot history | Local SQLite ([dependencies.md](dependencies.md)) | Queries by day, app and metric, with retention |
| Benchmark history | JSON (as today, `history.json`) | Small volume |

### 4.5 Code organization

Details are in [engineering.md](engineering.md). In short:
- A workspace split by **layer**:
  - `core`: domain and pure logic, with no OS and no GUI;
  - `platform`: per-OS adapters, where all `unsafe` lives;
  - `gui`: never calls the OS;
  - the `cpuzeez` binary: composition.
- Platform *traits* exist **only** where there are 3 real implementations plus a test fake, and each comes with contract tests.
- **Five reusable building blocks:**
  - the Metric Engine;
  - the Widget Engine;
  - the virtualized table;
  - the availability model, which feeds the permission banners;
  - actions + executor, which unifies normal and elevated execution and keeps the local audit log.

No other internal framework without a proven second use.

## 5. Permissions map (goes into the README)

A summary of what each privilege level unlocks. Details are in each document.

| Capability | Windows | Linux | macOS |
|---|---|---|---|
| Inventory (CPU, cache, SMBIOS, GPU, drives, network), load, memory, the user's own processes | ✅ | ✅ | ✅ |
| Per-core temperatures, clocks and power; SPD; memory timings; Super I/O | 🧩 PawnIO + admin | 🔑 root (or ✅ through in-kernel hwmon drivers) | ⚠ SMC/IOReport only, no admin |
| **Game FPS/frame times (ZeeZ Live)** | 🔑 admin (ETW); later the helper (C) | ✅ through MangoHud (a Vulkan/OpenGL layer the user enables) | ⚠ 🔬 (see zeez-live) |
| End, re-prioritize or suspend **other users' or elevated** processes | 🔑 | 🔑 | 🔑 |
| Start, stop and configure services | 🔑 (UAC per action) | 🔑 polkit per action | 🔑 system domain · ✅ user domain |
| The user's startup items | ✅ | ✅ | ⚠ (third-party Login Items have no public API) |
| Machine-wide startup items and "startup impact" | 🔑 | 🔑 | ❌ impact |
| Disconnect or sign out another user | 🔑 | 🔑 polkit | 🔑 |
| Per-process network | 🔑 (ETW) | 🔑 (eBPF/conntrack) 🔬 | ⚠ (`nettop`) 🔬 |
| SMART self-test, manual TRIM, defragmentation | 🔑 | 🔑 | ⚠/❌ |

## 6. Roadmap (decided, Q7)

**One public release, 1.0.0.** The item-by-item breakdown is in [ROADMAP.md](ROADMAP.md). Work moves through four internal stages. Each stage closes only when its **quality gate** passes (§6.2). `main` is always green, and nothing half-done ships.

| Stage | Contents | Depends on |
|---|---|---|
| **S0. Foundation** | ADRs for every decision in §8; workspace migration (engineering §10, no behaviour change); test infrastructure (engineering §8); Metric Engine (D1–D5); virtualized table (D6); **Widget Engine**; availability model + elevation A/B + banners (D8); actions + executor + audit log; Privacy & permissions first-run + update opt-in (P1/P2); Settings fixes (D9); `windows-sys` (D10); bundling of the PawnIO installer, modules and CRT ([dependencies.md](dependencies.md)) | — |
| **S1. Existing features, improved** | Overview, Processor, Mainboard, Memory, Storage/Devices, Benchmarks/Tools (including SSD/HDD health), Sensors on the new table, **tray** and **mini monitor** on the Widget Engine ([core-pages.md](core-pages.md)). The new data sources built here (per-disk I/O, adapter throughput, memory composition) are reused in S2 | S0 |
| **S2. New features** | In order: (a) **System / Task Manager**, since ZeeZ Live and the detector need its per-process and per-thread data; (b) **ZeeZ Live** (overlay, capture, reports); (c) **bottleneck detector**. Parts of (a) and (b) can run in parallel | S1 |
| **S3. Release hardening** | The 1.0 criteria (§7) → tag **1.0.0** | everything |

### 6.1 Beta releases (decided, Q10)

1.0.0 is still the one feature release. Before it, testers with varied hardware get **pre-releases**, because CI cannot cover the real world of boards, GPUs and drivers.

- **When:** after each closed stage gate (`1.0.0-beta.N` after S1, S2a, S2b and S2c), then `1.0.0-rc.N` during S3, then `1.0.0`.
- **Who gets them:**
  - They're published as GitHub **pre-releases**. GitHub's `releases/latest` endpoint, which today's update check uses, skips pre-releases, so **stable users never see a beta**.
  - The app gains an opt-in **"Beta channel"** in *Settings › Updates*. It's off by default and only available when the update check itself is opted in (P1). It lists all releases and picks the newest, pre-releases included.
- **Fixes needed first (S0):**
  - `update::newer()` (`src/update.rs:95-101`) treats every pre-release as equal, so `beta.2` would not count as newer than `beta.1`. It must compare pre-release identifiers per SemVer (`beta.2 > beta.1`, `rc.1 > beta.9`, `1.0.0 > rc.N`), with tests.
  - Package versions must be mapped per format, because not every format accepts `-beta.N`:
    - **MSI** requires a numeric `major.minor.build` and ignores a 4th field for upgrades. Today the tag goes straight into `-d Version="$VER"` (`release.yml:35`), which would break. Map `beta.N` → `0.99.N` and `rc.N` → `0.99.(50+N)`, so the 0.3.0 → beta → rc → 1.0.0 upgrades work.
    - **`.deb`** uses `1.0.0~beta.N` (the `~` sorts before `1.0.0`).
    - **Arch** and **macOS** get the equivalent.
  
  The displayed version stays `1.0.0-beta.N`, and the existing install/upgrade/downgrade CI tests verify the mapping.
- **CHANGELOG:** each pre-release gets its own `## [1.0.0-beta.N] - date` section, since the release workflow requires one per tag. The final `1.0.0` section consolidates everything.
- **Feedback:** a "Beta report" issue template asking for the hardware, what went wrong, and an optional diagnostics bundle **attached manually by the tester** (P1: nothing is uploaded automatically). Known issues are listed in each pre-release's notes.

### 6.2 Quality gates (every stage)

A stage is closed only when **all** of these pass in CI on Windows, Linux and macOS. The details of each test type are in [engineering §8](engineering.md).

| Gate | Checks |
|---|---|
| **Functionality** | Unit, fixture and contract tests for everything new; GUI interaction tests (`egui_kittest`) for each new or changed page; visual regression of the screenshot set |
| **Stability** | No panics in the fuzzing corpus; a soak test (sampler + overlay for N hours on a test runner) with flat memory; every probe failure degrades to "unavailable" (tested with fakes that fail) |
| **Security** | `cargo-deny` (licenses, advisories); executor allow-list tests (unknown or malformed `--elevated-op` is rejected); a test that **no network call happens without opt-in**; data-folder file permissions; no `unsafe` outside `platform` (lint) |
| **Performance** | The P6 budgets as CI benchmarks that fail on regression |
| **Docs** | P7 checklist for every feature in the stage; CHANGELOG `Unreleased` updated |
| **Real hardware** (manual, before closing the stage) | The hardware matrix in engineering §8 |

## 7. 1.0 release criteria

- **Stability**
  - No `panic` brings the GUI down. Probes are isolated, and a failed probe shows "unavailable".
  - An 8-hour session with overlay and sampler running shows no leaks (memory stays flat).
  - Smoke tests run in CI on Windows, Linux (X11 and Wayland) and macOS (Intel and ARM).
- **Performance:** the P6 targets are measured and published.
- **Privacy:** P1/P2 are verified, including a test that no network connection happens without opt-in.
- **Documentation:** P7 holds for every feature, and FEATURES.md is updated with the new per-OS tables.
- **i18n:** 100% of keys in pt-BR and es (the existing `i18n.rs` tests already check parity).
- **Accessibility:**
  - full keyboard navigation;
  - AA contrast in both themes;
  - UI scaling from 75% to 200%;
  - colour is always paired with an icon or text, never colour alone, for colour-blind users.
- **Installers:**
  - optional components are clear (PawnIO, and later the helper service);
  - uninstall is clean and removes any services, tasks and rules the app created;
  - nothing is installed without consent (P8).

## 8. Decisions

| # | Question | Decision |
|---|---|---|
| Q1 | Name of the monitoring feature | ✅ **ZeeZ Live**: a brand, with plain names for its parts (Overlay, Capture, Reports). See [zeez-live.md §1](zeez-live.md) |
| Q2 | Privilege strategy | ✅ A + B in 1.0; read-only helper C after 1.0 (§4.3) |
| Q3 | FPS source on Windows | ✅ PresentMon (Intel, MIT), **bundled** so users install nothing; native ETW in Rust evaluated after 1.0 |
| Q4 | FPS on Linux | ✅ MangoHud, with every friction reducer in [zeez-live §3.1](zeez-live.md); our own Vulkan layer after 1.0 |
| Q5 | Task Manager navigation | ✅ One "System" sidebar entry with tabs; "Details" is a *view mode* of the process list, with no column or action lost |
| Q6 | New dependencies | ✅ As in [dependencies.md §5](dependencies.md), with one rule: pick whatever solves the problem best **while removing friction for the user** |
| Q7 | Order | ✅ S0 foundation → S1 existing features → S2 new features → S3 hardening; **everything ships in 1.0.0**, with a quality gate per stage |
| Q8 | Language of these documents | ✅ English |
| Q9 | Code signing (Windows Authenticode, Apple notarization) | ✅ **Not for now** (no budget). Zero-cost mitigations are in [dependencies.md §3.1](dependencies.md) |
| Q10 | Pre-releases for testers | ✅ Yes: `1.0.0-beta.N` / `rc.N`, with an opt-in beta channel (§6.1) |
