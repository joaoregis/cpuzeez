# CpuZeeZ 1.0: detailed roadmap

> Part of the [1.0 plan](README.md). This document is the **work list**: every fix, foundation piece, improvement, new feature, spike, packaging task and doc change that came out of the planning, with an ID, its source, its dependencies and a testable "done when".
> **Status:** not started · **Baseline:** 0.3.0 (`26d9f0e`) · **Date:** 2026-11-04.

## How to read this

- **One public release: `1.0.0`.** The four stages are internal. A stage closes only when its [quality gate](README.md) passes (README §6.2), and each closed stage produces a **pre-release** for testers (§6).
- **Item types:**

| Icon | Type |
|---|---|
| 🐞 | Fix |
| 🧱 | Foundation |
| ✨ | Improvement to an existing feature |
| 🆕 | New feature |
| 🔬 | Spike (prove before building) |
| 📦 | Packaging |
| 🧪 | Tests/CI |
| 📝 | Docs |

- **Every item's definition of done** also includes the general rules in [engineering §9](engineering.md), on top of its own "done when":
  - tests at every applicable layer;
  - per-OS availability declared;
  - actions declare their risk and privilege;
  - i18n complete;
  - docs and the CHANGELOG `Unreleased` updated;
  - the performance budget holds.
- **Spikes run early.** The risky research for later stages (ZeeZ Live, System hub) runs **during S0–S1**, in parallel, so S2 never stalls on an unknown. A spike's output is a short ADR with the result: go, no-go, or alternative.

```
S0 Foundation ──► S1 Existing features ──► S2a System ──► S2b ZeeZ Live ──► S2c Bottleneck ──► S3 Hardening ──► 1.0.0
   │                 │ beta.1                 │ beta.2       │ beta.3          │ beta.4            rc.N
   └─ spikes ────────┴───────────────────────────── feed S2 ───────────────────┘
```

---

## S0. Foundation

Goal: fix what's broken, build the shared building blocks, and set up the tests that will guard everything after.

### S0.1 Fixes found during planning

| ID | Item | Type | Source | Done when |
|---|---|---|---|---|
| S0-F01 | **Static CRT on Windows**: the release exe imports `VCRUNTIME140.dll`, which isn't shipped | 🐞 | [dependencies §3](dependencies.md) | `+crt-static` for `*-windows-msvc`; CI starts the exe on a clean Windows image with no VC++ redistributable |
| S0-F02 | **Clock busy-spin**: 30 ms per core, in sequence (≈480 ms per sample on 16 cores), plus the 80 ms fallback loop | 🐞 | README D1, `cpu/clocks.rs:64,76-81` | Delta-based APERF/MPERF/TSC per tick; CI bench: tick cost < 1 ms on 16 cores (fake MSR source) |
| S0-F03 | `rdmsr` spawns a thread per read on Windows | 🐞 | D2, `os.rs:45`, `hw/windows.rs:385` | One sampler thread re-pins itself; no thread creation in the sampling path (test counts threads) |
| S0-F04 | SMART probe of every drive inside the sampler | 🐞 | D3, `telemetry.rs:310` | SMART on the slow worker; a slow fake drive doesn't delay fast metrics (test) |
| S0-F05 | macOS spawns `ioreg` twice per sample | 🐞 | D4, `battery.rs:393`, `sensors/macos.rs:208` | Battery on the slow class (or native IOKit); no process spawn per fast tick |
| S0-F06 | `series()` clones the whole history; a duplicate sampler runs in the GUI | 🐞 | D5, `telemetry.rs:672,748`, `gui/mod.rs:259` | Replaced by the Metric Engine (S0-B01); the GUI sampler is removed |
| S0-F07 | Changing `sample_ms`/`graph_seconds` doesn't restart the sampler; the mini monitor isn't persisted | 🐞 | D9 | Settings apply live (GUI test); mini monitor state survives a restart |
| S0-F08 | `update::newer()` treats every pre-release as equal (`beta.2` isn't newer than `beta.1`) | 🐞 | README §6.1, `update.rs:95-101` | SemVer pre-release ordering with unit tests (`beta.2>beta.1`, `rc.1>beta.9`, `1.0.0>rc.N`) |
| S0-F09 | MSI versions must be numeric; a `-beta.N` tag would break the MSI build | 🐞 | README §6.1, `release.yml:35` | Per-format mapping (MSI `0.99.N`, deb `1.0.0~beta.N`, Arch/macOS equivalents); upgrade 0.3.0 → beta → rc → 1.0.0 passes in CI |
| S0-F10 | Update check is on by default | 🐞 | P1, `update.rs` | Off until opted in on first run (S0-B07); test: zero network calls without consent |
| S0-F11 | 20 compiler warnings in the release build | 🐞 | engineering §1 | `clippy -D warnings` green |
| S0-F12 | Dead code: `Monitor::menu`, `autoshot`, `autoshot_next` | 🐞 | GUI map, `gui/sensors.rs:176,316,339` | Removed |

### S0.2 Engineering foundation

| ID | Item | Type | Source | Depends | Done when |
|---|---|---|---|---|---|
| S0-E01 | **ADRs** for Q1–Q10 in `docs/adr/` | 📝 | README §8 | — | One ADR per decision: context, decision, alternatives, consequences |
| S0-E02 | **Workspace migration** step 1: pure decoders → `core` | 🧱 | [engineering §10](engineering.md) | — | Existing tests pass unchanged; `core` builds with no OS API |
| S0-E03 | Step 2: per-OS sources → `platform` behind the traits (`trait Hw` included) | 🧱 | engineering §3, §10 | E02 | Contract suites run per OS in CI |
| S0-E04 | Step 3: extract the pages out of `gui/mod.rs`; separate the Super I/O data from logic | 🧱 | engineering §9 | E02 | Visual regression shows no difference; `gui/mod.rs` holds only navigation and composition |
| S0-E05 | `windows-sys` in `platform/windows` (new code; old `extern`s migrate when touched) | 🧱 | dependencies §2.2 | E03 | New Win32 calls use it |
| S0-E06 | `unsafe` confined to `platform/*/sys/`; `forbid(unsafe_code)` in `core` and `gui`; `// SAFETY:` comments | 🧱 | engineering §7 | E03 | Lint enforces it in CI |

### S0.3 Test infrastructure and CI

| ID | Item | Type | Done when |
|---|---|---|---|
| S0-T01 | CI matrix: Windows, Ubuntu X11 + Wayland (headless compositor), macOS Intel + ARM | 🧪 | Every job is required for merge. ✅ Wayland: headless Weston 13 renders all 17 screens on x64 and arm64 |
| S0-T02 | `rustfmt`, `clippy -D warnings` (agreed pedantic subset), MSRV, **`cargo-deny`** (licenses, advisories, duplicates) | 🧪 | Required checks |
| S0-T03 | **Fakes** for every source trait + the **contract test** harness | 🧪 | Every trait has a suite; every platform runs it. ✅ `metrics::check_contract`; every real source checked on each CI OS (`every_source_keeps_the_contract`) |
| S0-T04 | **Fixture** framework (recorded `/proc`, SMART/NVMe dumps, SMU tables, PresentMon/MangoHud CSV) | 🧪 | Fixture tests in CI on every OS. ✅ `crates/core/fixtures/` (convention + privacy rule in its README); first: `/proc/diskstats`. Each later parser adds its own |
| S0-T05 | **GUI interaction tests** with `egui_kittest` | 🧪 | At least one test per existing page, as the template |
| S0-T06 | **Visual regression** from the `-screenshots` set (light and dark, with tolerance) | 🧪 | Diffs fail CI |
| S0-T07 | **Fuzz** targets: SMBIOS, SPD, EDID, CSV import, session/report JSON, elevated-op parser | 🧪 | Scheduled runs; corpus committed |
| S0-T08 | **Performance benches** with thresholds: engine tick, overlay frame, 10k-row table frame. Per-source read time is already recorded (`Engine::costs`, printed after `-log`) | 🧪 | Regressions fail CI |
| S0-T09 | **Security tests**: no network without opt-in (unit test + a Linux job in a network namespace with no network), executor allow-list, data-folder permissions | 🧪 | Required checks |
| S0-T10 | **Soak** runner (nightly): sampler + overlay + capture for N hours | 🧪 | Flat memory, handles and fds |
| S0-T11 | **Packaging tests** extended: clean-Windows start, PawnIO checkbox, `.deb`/Arch install, pre-release version mapping | 🧪 | Required on release-touching PRs |

### S0.4 Shared building blocks

| ID | Item | Type | Source | Depends | Done when |
|---|---|---|---|---|---|
| S0-B01 | **Metric Engine**: catalog with stable IDs, cost classes, subscriptions, fast/slow/event/on-demand workers, snapshots, `since(t)` | 🧱 | README §4.1 | E02, F02–F06 | All existing consumers (Sensors, Overview, Stress, tray, mini monitor, `-sensors`, `-log`) run on it; P6 benches pass |
| S0-B02 | **Widget Engine**: `Layout` model (JSON, versioned), widget renderer, threshold styles; the editor core | 🧱 | README §4.2 | B01 | The tray and mini monitor can render a layout (used in S1) |
| S0-B03 | **Virtualized table**: columns and presets, sort, filter syntax, grouping, multi-select, context menu from `Action`s, CSV export | 🧱 | engineering §4.3 | — | 10k rows within the frame budget (bench); kittest coverage |
| S0-B04 | **Availability model** + the single `capability_banner` | 🧱 | engineering §4.1, P3 | E03 | Every existing page shows a banner when data is missing; the README permissions map is generated or checked from it |
| S0-B05 | **Actions + Executor + audit log**, elevated subprocess with allow-list (**option B**) | 🧱 | engineering §4.2, README §4.3 | E03 | Unknown/malformed ops rejected (tests + fuzz); every action logged locally |
| S0-B06 | **Restart as administrator** (**option A**): Windows `runas`, Linux `pkexec`, macOS authorization | 🧱 | README §4.3 | B04 | Button in banners works on all OSes; manual test matrix |
| S0-B07 | **First run: Privacy & permissions**; *Settings › Privacy & data* (storage view, export, delete by category, retention); the shared **● REC** indicator | 🧱 | P1, P2 | B08 | Update check and recordings off until opted in (tests) |
| S0-B08 | **Local storage**: data-folder layout, schema versions + migrations, retention jobs. ✅ in S0: settings `schema` + read-only when newer, atomic saves, unreadable files kept as `.bad`, history refuses entries from a newer app, layouts versioned, log retention. **SQLite moves to S2a-10** with its first consumer (app history): no dependency without a user (ADR 0006) | 🧱 | README §4.4, dependencies | E02 | A newer file never gets overwritten by an older app (test) |
| S0-B09 | **Local logs** with rotation, plus a manual diagnostics bundle (logs + report, never uploaded) | 🧱 | engineering §9 | — | Bundle created from Settings; no network involved |
| S0-B10 | Probe isolation: `catch_unwind`, timeouts, cancellation on workers | 🧱 | engineering §5–6 | B01 | A panicking fake probe shows "unavailable" and the GUI stays up (test) |

### S0.5 Packaging and distribution

| ID | Item | Type | Source | Done when |
|---|---|---|---|---|
| S0-P01 | Bundle the **official PawnIO installer**: MSI optional checkbox + in-app "Install hardware access" (portable). ✅ Feature ticked by default (owner's decision), can be unticked; the in-app action re-checks the pinned SHA-256 with the file locked; CI installs it and checks uninstall keeps it | 📦 | dependencies §3 | Checkbox installs PawnIO; uninstalling CpuZeeZ leaves it, and says so |
| S0-P02 | Release workflow downloads pinned **PawnIO modules**, verifies SHA-256, and ships them with `COPYING` and the source tag. ✅ `packaging/pins.txt` + `fetch_pawnio.sh`, MSI and portable zip | 📦 | dependencies §3 | Modules load from a fresh MSI install with no manual copy |
| S0-P03 | `THIRD_PARTY.md` + *About › Third-party components* ✅ | 📦📝 | dependencies §4 | Every bundled binary listed with version, license and hash |
| S0-P04 | **Pre-release pipeline**: prerelease flag for `-beta`/`-rc` tags, opt-in **Beta channel** in Settings, "Beta report" issue template | 📦 | README §6.1 | A test tag publishes a pre-release that stable users don't see |
| S0-P05 | Zero-cost signing mitigations: README notice ✅ (done), release notes text ✅, hash-check steps ✅ | 📝 | dependencies §3.1, Q9 | Release notes template includes the notice |

### S0.6 Early spikes (de-risk S1–S2)

| ID | Question | For | Done when |
|---|---|---|---|
| S0-X01 | **eframe on wgpu** instead of glow: transparency, overlay, startup cost, VM/RDP (WARP) | Q6c, ZeeZ Live | ADR: switch or stay on glow. ✅ [ADR 0013](../adr/0013-gui-renders-with-glow-and-falls-back-to-wgpu.md): glow stays, automatic wgpu fallback (WARP verified) |
| S0-X02 | **SP1** PresentMon sidecar: overhead and API coverage, FG | ZeeZ Live | < 0.5% CPU; DX11/DX12/Vulkan/OpenGL covered. 🟡 2.6.0: 0.04 % of the machine (~1 % of one core), 15 MB, DX11 ✅; DX12/Vulkan/OpenGL/FG and 240+ FPS still to do (zeez-live §3) |
| S0-X03 | **SP2** external overlay over games on Windows (display modes × latency) | ZeeZ Live | Matrix documented |
| S0-X04 | **SP3** macOS overlay over fullscreen Spaces | ZeeZ Live | Proof of concept or documented limit |
| S0-X05 | **SP4** Wayland layer-shell with winit/eframe | ZeeZ Live | PoC on KDE/Hyprland, or a "MangoHud only" ADR |
| S0-X06 | **SP5** MangoHud live data latency | ZeeZ Live | < 1 s, or a "post-session only" ADR |
| S0-X07 | **SP6** Metal HUD log; ScreenCaptureKit FPS cost | ZeeZ Live | Feasibility + cost |
| S0-X08 | **SP7** PawnIO × kernel anti-cheats (Vanguard, FACEIT…) | ZeeZ Live | Compatibility list. 🟡 Started in zeez-live §5: Vanguard + PawnIO coexist outside a match; in-match and FACEIT/EAC/BattlEye need a player (see the S0 gate note) |
| S0-X09 | **SP8** overlay GPU cost | ZeeZ Live | < 0.3 ms per game frame |
| S0-X10 | Per-process network: ETW Kernel-Network (Win), eBPF/sock_diag (Linux), `nettop` (macOS) | System | Feasibility per OS. ✅ task-manager §3: ETW (admin), sock_diag TCP_INFO (no eBPF), `nettop` sidecar. macOS needs a check on the Mac (docs/testing/macos.md) |
| S0-X11 | Per-process GPU on macOS (IOAccelerator `AppUsage`); "not responding" on macOS | System | Feasibility |
| S0-X12 | Task Manager clean-room confirmations: startup-impact thresholds, `StartupApproved` format | System | Documented in task-manager §1. ✅ Layout and thresholds confirmed; WDI files found stale, so impact shows its date and falls back to the recorder |
| S0-X13 | NVMe Device Self-test through Windows' protocol command path | S1 tools | Works, or a documented limit. ✅ Supported (1903+, admin) with a ~10 min throttle, per core-pages §7.3. The first real run happens in S1-K08 with the user's consent |
| S0-X14 | SignPath Foundation eligibility; package-manager policies for unsigned builds (winget, Scoop, AUR, Homebrew) | Distribution | Decision per channel. ✅ dependencies §3.1: winget, Scoop and AUR OK; Homebrew needs our own tap; SignPath: ask about dual licensing first |

**S0 gate:** every 🐞 closed; the building blocks have tests and benches; existing features behave exactly as in 0.3.0 (visual regression), but run on the new architecture; every spike has an ADR. *No pre-release: nothing new for users yet.*

**S0 gate status (2026-11-05): closed for S1, with spikes carried.**
- Every 🐞 is closed, every building block has tests, and CI is green on Windows, Linux (x64, arm64, Debian, Ubuntu 22.04, Arch) and macOS (Intel, ARM).
- The spikes that need hardware, games or another OS stay open. None of them blocks S1, which improves the existing pages; each must close before the gate of the stage it de-risks:

  | Spike | Waiting on | Blocks |
  |---|---|---|
  | X02 (rest): DX12/Vulkan/OpenGL/FG, 240+ FPS | Games with those APIs | S2b gate |
  | X03, X09: overlay over games × display modes; overlay GPU cost | A play session with the overlay prototype (S2b) | S2b gate |
  | X04, X07, X11: macOS overlay over fullscreen, Metal HUD, per-process GPU | Mac results ([docs/testing/macos.md](../testing/macos.md)) | S2a/S2b gates |
  | X05, X06: Wayland layer-shell, MangoHud | A Linux desktop (KDE/Hyprland) | S2b gate |
  | X08 (rest): in-match with sensors on; FACEIT/EAC/BattlEye | The account owner, at their own risk | S2b gate |

---

## S1. Existing features, improved

Goal: everything CpuZeeZ already does becomes complete, configurable and polished. New data sources built here are reused in S2.

### S1.1 New platform data sources

| ID | Item | Type | Source | Done when |
|---|---|---|---|---|
| S1-D01 | Per-disk I/O (bytes, IOPS, queue, latency, active time) | 🧱 | task-manager §4 | Win/Linux/macOS sources + contract tests. ✅ `disks` source (`disk<N>/…`): `IOCTL_DISK_PERFORMANCE`, `/proc/diskstats` (fixture-tested), IOKit statistics (macOS: active time estimated, no queue); no privileges |
| S1-D02 | Per-adapter network throughput, errors and drops | 🧱 | task-manager §4 | Same. ✅ `net` source (`net<N>/…`, Mbps, packets/s, errors, drops): `GetIfTable2` (hardware, up), `/proc/net/dev` (fixture-tested), routing-socket `if_data64`; plumbing interfaces left out |
| S1-D03 | Memory composition (commit, pools, standby, compressed, *wired*), page file/swap | 🧱 | task-manager §4 | Same. ✅ `core::memory::Composition` (missing ≠ 0); `GetPerformanceInfo` + memory-list PDH counters, `/proc/meminfo` (fixture-tested), `vm_statistics64` + `vm.swapusage`; `mem/size/*`, `mem/load/{commit,swap}` |
| S1-D04 | GPU per-engine utilization (adapter level) | 🧱 | zeez-live §4 | Same. ✅ `gpu-engines` source (`gpu<N>/engine/*`): `GPU Engine` counters aggregated like the Task Manager (sum per engine, busiest engine per type; unit-tested), amdgpu `gpu_busy_percent`, IOAccelerator Device/Renderer/Tiler |
| S1-D05 | **Split the `cpu+board` source by device** (CPU, Super I/O, DIMMs, SMU, chipset) so a lease reads only what it needs, and find the ~170 ms of waiting per read (measured 2026-11-04 on the reference machine, elevated: 193 ms mean latency but ~19 ms of CPU) | 🐞 | P6, measurement below | Overlay lease "cpu temp + clock" reads < 5 ms per tick. ✅ Sources `cpu`, `board`, `dimms`; the "waiting" was the first read averaged in (now reported apart); typical cpu tick ~3 ms under game load. 🟡 Rare ~30 ms spikes on busy cores: a per-core pinned reader pool is the next step if ZeeZ Live's SP8 budget needs it |

### S1.2 Shared page patterns

| ID | Item | Type | Source | Done when |
|---|---|---|---|---|
| S1-U01 | Detail levels: Basic / Advanced / Expert | ✨ | [core-pages §1](core-pages.md) | On every page; kittest. ✅ Switch in the Processor, Mainboard, Memory and Graphics headers, remembered per page; Advanced (default) = the pre-1.0 view, Basic = essentials, Expert adds thread 0's CPUID leaves and MSRs (`core::detail`, kittest `detail_levels_filter_cards`). Devices/Sensors get it with their S1-G07/G08 rebuilds |
| S1-U02 | **Command palette** (`Ctrl+K`): search every page, run actions | ✨ | core-pages §1 | Finds fields and actions across pages. ✅ Every report field (with its value and path), every page and six actions; every word must match, title-prefix first; Enter opens the page that shows it (tests + kittest) |
| S1-U03 | Copy / copy as report text / **pin to Overview** on every row and card | ✨ | core-pages §1 | Everywhere. ✅ Right-click any card row (`widgets::kv_list`, so every page): copy value, row, card as text, pin. Sensors rows pin their metric (live on the Overview's *Pinned* card); pins persist in settings; kittest `a_row_pins_under_its_card` |
| S1-U04 | Global units (°C/°F, MHz/GHz, MB/MiB, decimals) and a raw-value toggle | ✨ | core-pages §1 | Everywhere. ✅ `core::units` (tested) + *Settings › Units*; every live value, tile, tray/mini text and page clock goes through it; stored values stay °C/MHz/MiB. Raw values come with Expert detail (U01) |

### S1.3 Pages

| ID | Item | Type | Source | Done when |
|---|---|---|---|---|
| S1-G01 | **Overview**: identity strip, health strip, configurable tiles (Widget Engine) with deep links, presets, opt-in local alerts, quick actions, Customize mode | ✨ | core-pages §2 | Every tile deep-links; layouts persist. ✅ Clickable identity strip; health pills (build, drives, battery wear); tiles from `layout::presets::overview` (Default = pre-1.0, Gamer, Overclocker, Laptop, Workstation) open Sensors filtered; Customize mode (kittest); quick actions; pinned card; opt-in local alerts (`core::alerts`, whole-window rule, tested; Overview banner + tray tooltip; *Settings › Alerts*). SMART is in the health strip |
| S1-G02 | **Build health** checks on the Overview: single channel, XMP/EXPO off, reduced PCIe link, ReBAR, power plan/governor, monitor refresh, iGPU cable | 🆕 | [bottleneck §2.5](bottleneck-detector.md) | Each check has fixtures and a plain-language fix. ✅ `core::health` (rules unit-tested per mistake; unknown facts skip their rule) fed by `src/health.rs` (SPD/SMBIOS, Chipset, adapters' links, `platform::display` modes + power plan/governor); translated fixes. 🟡 ReBAR: no reliable source yet (left out, not guessed); Linux refresh needs KMS |
| S1-G03 | **Processor**: per-core table (type, CCD/CCX, clocks, ratio, temp, VID, power, load, C-states, CPPC rank), topology diagram, limits, vulnerabilities/mitigations, virtualization/VBS, OS power/EPP, grouped instructions | ✨ | core-pages §3 | ✅ Per-core table (live clock/temp/load), topology (package → CCD → CCX → cores, L3), SMU limits, vulnerabilities (Windows speculation-control/KVA classes, Linux sysfs), VT-x/AMD-V + VBS/HVCI + KVM, power plan/governor/EPP (`platform::cpu_os`), instructions grouped and explained. 🟡 C-state residency, CPPC rank, Intel PL1/PL2 next. Per-OS table in FEATURES.md updated |
| S1-G04 | **Mainboard**: full SMBIOS browser, PCIe tree with link alerts, firmware security (Secure Boot, TPM, ReBAR, Above 4G, CSM), ACPI tables, Super I/O with board names, slots/ports | ✨ | core-pages §4 | ✅ Firmware (UEFI/CSM, Secure Boot, TPM via `platform::firmware`), PCIe links table (current vs max; narrower = warning, slower = idle), Super I/O readings by board-map name, slots/ports (SMBIOS 8/9), SMBIOS tree browser, ACPI table headers at Expert (`core::acpi`). 🟡 ReBAR/Above 4G: no reliable OS source yet |
| S1-G05 | **Memory**: full timings (primary/secondary/tertiary), topology, active profile vs JEDEC/XMP/EXPO, ECC + EDAC, DIMM temp/PMIC, OS composition, theoretical vs measured bandwidth | ✨ | core-pages §5 | ✅ AMD UMC secondary/tertiary (tRCDWR, tRRD_S/L, tFAW, tWR, tRTP, tCWL, tWTR_S/L, tREFI, DDR5 tRFC1/2; decoded from registers read on a 7900X, unit-tested), active profile (exact match or "tuned …"), SMBIOS slot map, ECC type + Linux EDAC counters, DIMM temps, OS composition bar, theoretical bandwidth → memory benchmark. 🟡 Intel secondaries, DDR5 GDM/2T/power-down (unverified fields left out) |
| S1-G06 | **Storage**: complete NVMe Identify / ATA IDENTIFY, full SMART with thresholds and throttle counts, link current/max, partitions/volumes with alignment and encryption, OS TRIM status, live I/O | ✨ | core-pages §6.1 | 🟡 Done: full SMART tables (ATA thresholds, whole NVMe health log), NVMe thermal-throttle count/time, link, partition alignment (`platform::partitions`, no admin), OS TRIM (NTFS setting / Linux discard); live I/O per disk is the Sensors *Disks* preset (D01). Open: full Identify dumps (Expert), volume encryption (BitLocker needs admin/WMI; LUKS via dm) |
| S1-G07 | **Devices**: USB tree with negotiated vs supported speed, Thunderbolt/USB4, Bluetooth peripheral battery, monitor current mode/HDR/VRR, audio formats, live network | ✨ | core-pages §6.2 | Same |
| S1-G08 | **Sensors** on the virtualized table (search syntax, presets, export) | ✨ | engineering §4.3 | Same features as today, plus table features. ✅ Grouped table (sort, filter syntax incl. `cur>80`/`unit:°`/`id:disk`, multi-select, Ctrl+C), hot values tinted, presets, *Export view* (CSV of the filtered rows), plot/pin from the selection. The release build has 0 warnings again |

### S1.4 Benchmarks and tools

| ID | Item | Type | Source | Done when |
|---|---|---|---|---|
| S1-K01 | Benchmark principles: test-file only, reproducibility (conditions recorded, N runs + median), documented score (1000 = reference machine) | ✨ | core-pages §7.1 | Every benchmark follows them |
| S1-K02 | **CPU suite**: integer, FP, SIMD GEMM, crypto, ray tracer, branchy, thread scaling, P × E, 10-min sustained | ✨ | core-pages §7.2 | Scores stable within tolerance across 3 runs on the reference machine |
| S1-K03 | **Core-to-core latency** heat map | 🆕 | core-pages §7.2 | Win/Linux; macOS statistical |
| S1-K04 | **Memory** additions: GUPS, STREAM triad, huge pages, per-CCD/NUMA latency | ✨ | core-pages §7.2 | Same |
| S1-K05 | **GPU benchmark** (`wgpu`, feature `gpu-bench`): FP32/FP16, VRAM bandwidth, PCIe host↔GPU, raster scene | 🆕 | core-pages §7.2 | DX12/Vulkan/Metal |
| S1-K06 | **Storage benchmark** (CrystalDiskMark-style + sustained write with a warning) | 🆕 | core-pages §7.2 | Uncached I/O verified; cleans up its test file |
| S1-K07 | **Stability**: CPU stress variants, user-mode memory test, GPU stress, combined | ✨ | core-pages §7.2 | Auto-stop limits tested |
| S1-K08 | **SSD/HDD health**: SMART self-tests, health panel (life, TBW, temperature history, alerts), read-only surface scan | 🆕 | core-pages §7.3 | Per-OS availability declared |
| S1-K09 | History and compare for every benchmark type | ✨ | core-pages §7.2 | Same UI as today's history |

### S1.5 Tray and mini monitor

| ID | Item | Type | Source | Done when |
|---|---|---|---|---|
| S1-M01 | **Tray**: several icons (one per metric), styles (number, bar, mini graph, ring), DPI-sized text, colour + shape thresholds, rich tooltip, dynamic re-translatable menu, configurable click, macOS multi-metric menu-bar text, honest limits shown | ✨ | core-pages §8 | Win/mac/Linux SNI; kittest for the settings UI |
| S1-M02 | **Mini monitor** on the Widget Engine: layouts, resizable, appearance, lock (click-through), snap and remember, several instances, hide on fullscreen, expand on hover, context menu, persistence | ✨ | core-pages §9 | Shares layouts with the overlay editor |

**S1 gate → pre-release `1.0.0-beta.1`.**

---

## S2a. System hub (Task Manager)

| ID | Item | Type | Source | Depends | Done when |
|---|---|---|---|---|---|
| S2a-01 | `procs` sources per OS: snapshot with threads, no strong handles | 🆕 | [task-manager §3.1](task-manager.md) | S0-E03, X10–X11 | Contract tests; anti-cheat rule enforced (handle-rights test) |
| S2a-02 | **Processes**, Grouped and Detailed modes: every column, search syntax, presets, tree, heat map, 60 s sparklines, pinning | 🆕 | task-manager §2–3 | S0-B03 | Every Task Manager column present or marked per OS |
| S2a-03 | **Process actions** through the Executor: end/tree, restart, suspend/resume, Eco mode, priority, affinity, dump, wait chain, UAC virtualization, run new task (+ admin), window actions | 🆕 | task-manager §3.2 | S0-B05 | Critical-process warnings; kittest flows |
| S2a-04 | **Details panel**: threads, modules, connections, open files, services, windows, GPU, security (signature, integrity, on-demand SHA-256) | 🆕 | task-manager §3.3 | 01 | Protected/anti-cheat processes excluded from on-demand reads |
| S2a-05 | Problems filters, multi-select batch actions, export, snapshot compare | 🆕 | task-manager §3.3 | 02 | Same |
| S2a-06 | **Performance** tab: per-device views (CPU grid, memory, disks, network, GPU engines, NPU), summary view, time ranges, export | 🆕 | task-manager §4 | S1-D01–D04 | Same |
| S2a-07 | **Services**: SCM / systemd D-Bus (polkit) / launchd; start/stop/pause/restart, startup type, log-on account, recovery, dependencies (graph), logs, health, filters, protected list, undo | 🆕 | task-manager §8 | S0-B05 | Every `services.msc` function present or marked per OS |
| S2a-08 | **Startup**: all sources, enable/disable, impact (WDI / systemd / recorder), last BIOS time (FwPOSTTime / FPDT), boot timeline, read-only advanced view, undo | 🆕 | task-manager §6 | X12 | Same |
| S2a-09 | **Users**: sessions, per-user resources, disconnect/sign out/message/connect, manage accounts | 🆕 | task-manager §7 | S0-B05 | Same |
| S2a-10 | **App history** recorder (opt-in, SQLite, retention) + UI | 🆕 | task-manager §5 | S0-B07–B08 | Off by default; covers-only-while-running notice shown |
| S2a-11 | *Settings › System* (every Task Manager setting + new ones) | 🆕 | task-manager §9 | — | Same |

**S2a gate → `1.0.0-beta.2`.**

## S2b. ZeeZ Live

| ID | Item | Type | Source | Depends | Done when |
|---|---|---|---|---|---|
| S2b-01 | **Overlay window** per OS (click-through, no focus, deferred viewport, capture-exclusion option, exclusive-fullscreen detection) | 🆕 | [zeez-live §3.2](zeez-live.md) | X01, X03–X05, X09 | P6 FPS impact ≤ 1% |
| S2b-02 | **Brand and first use**: ZeeZ Live entry with Live / Overlay / Sessions tabs, ● LIVE / ● REC, 3-step first use, glossary, teaching empty states | 🆕 | zeez-live §1 | 01 | Usability check: a new user shows the overlay in under 1 minute |
| S2b-03 | **Styles** (Minimal, Compact, Detailed, Benchmark, Overclocker) and the **layout editor** (WYSIWYG, widgets, styles, anchors/drag, rates) | 🆕 | zeez-live §5.2–5.3 | S0-B02 | Layouts shared with the mini monitor |
| S2b-04 | **Global hotkeys** (Win/mac/X11 + Wayland portal), conflict detection | 🆕 | zeez-live §5.5 | — | Per-OS tests |
| S2b-05 | **Frame source, Windows**: bundled PresentMon sidecar | 🆕 | zeez-live §3.1, Q3 | X02 | Frames, GPU Busy, FG and present mode parsed (fixtures) |
| S2b-06 | **Frame source, Linux**: MangoHud detect, install, enable for all games, copy launch option, config generator, log ingest, status line | 🆕 | zeez-live §3.1, Q4 | X06 | Works with distro and Flatpak Steam |
| S2b-07 | **Frame source, macOS (experimental)**: Metal HUD launcher, ScreenCaptureKit displayed FPS | 🆕 | zeez-live §3.1 | X07 | Labelled experimental |
| S2b-08 | **Live** dashboard tab (target, FPS, graphs, capture button) | 🆕 | zeez-live §5.1 | 05–07 | Same |
| S2b-09 | **Capture**: manual, timed, automatic (opt-in); `frames.csv` (PresentMon columns), `hw.csv`, `session.json` | 🆕 | zeez-live §6 | S0-B07–B08 | Files open in CapFrameX (manual check) |
| S2b-10 | **Reports**: summary (1% convention stated), aligned charts, percentiles, histogram, comparison, validity warnings, HTML/PNG export, CSV import | 🆕 | zeez-live §6 | 09 | Fixture sessions produce known numbers |
| S2b-11 | **Per-game profiles** and automation | 🆕 | zeez-live §5.4 | 03 | Same |
| S2b-12 | **Anti-cheat safety**: "no PawnIO" mode where needed, the compatibility list from X08 | 🆕 | zeez-live §7 | X08 | Documented |

**S2b gate → `1.0.0-beta.3`.**

## S2c. Bottleneck detector

| ID | Item | Type | Source | Depends | Done when |
|---|---|---|---|---|---|
| S2c-01 | `core::bottleneck` rules (limiter, GPU, CPU-thread, throttling, VRAM/RAM/disk, contention) as pure functions + threshold table | 🆕 | [bottleneck §2–3](bottleneck-detector.md) | S2a-01, S2b-05/06 | Recorded-session fixtures classify correctly |
| S2c-02 | Heuristic fallback without GPU Busy, with lowered confidence shown | 🆕 | bottleneck §3 | 01 | Same |
| S2c-03 | **"Now" card** + live readings with rule thresholds | 🆕 | bottleneck §5 | 01 | kittest |
| S2c-04 | **Session analysis** (1/3/5 min, until stop) + evidence-tied recommendations | 🆕 | bottleneck §5 | 01 | No recommendation without evidence (test) |
| S2c-05 | Integrations: overlay pill, report section, Overview shortcut | 🆕 | bottleneck §5 | S2b | Same |
| S2c-06 | *(Conditional)* static hardware balance from our own benchmarks | 🆕 | bottleneck §4 | S1-K02, K05 | Calibration passes, or the ADR cuts it |

**S2c gate → `1.0.0-beta.4`.**

---

## S3. Release hardening → 1.0.0

| ID | Item | Type | Done when |
|---|---|---|---|
| S3-01 | P6 budgets measured on the real-hardware matrix and **published** | 🧪📝 | Numbers in README |
| S3-02 | 8-hour soak on real hardware | 🧪 | Flat memory, no leaks |
| S3-03 | Accessibility audit: keyboard, AA contrast, 75–200% scaling, colour redundancy | ✨ | Checklist passes on all pages |
| S3-04 | i18n: 100% pt-BR and es | 📝 | Parity tests green; strings reviewed |
| S3-05 | Security review: threat model (executor, elevation, bundled binaries, file formats), fuzz campaign, `cargo-deny` clean | 🧪 | Review doc in `docs/` |
| S3-06 | Installers: clean uninstall, optional components, upgrade path from 0.3.0 and from every beta/rc | 📦 | CI packaging matrix green |
| S3-07 | Docs: README + README.pt-BR (features, permissions map, privacy, unsigned notice), FEATURES.md per-OS tables, refreshed screenshots on every OS, CHANGELOG `1.0.0` consolidated | 📝 | P7 checklist complete |
| S3-08 | `1.0.0-rc.N` until no release-blocking issue remains, then tag **`1.0.0`** | 📦 | Release published |

---

## After 1.0 (backlog, already decided to wait)

| Item | Source |
|---|---|
| Read-only helper service (privilege option C): FPS and sensors without UAC at every launch | README §4.3 |
| Native Rust ETW consumer (`ferrisetw`) replacing the PresentMon sidecar | zeez-live §3.1 |
| Our own Vulkan frame-time layer on Linux | zeez-live §3.1 |
| Xbox Game Bar widget | zeez-live §3.2 |
| Import Windows history (SRUM) | task-manager §5 |
| Startup "Advanced" view with enable/disable | task-manager §6 |
| Drive optimization: defrag/ReTrim/fstrim through the OS engines | core-pages §7.4 |
| Code signing and notarization, when funded or through a free OSS program | dependencies §3.1, Q9 |

## Measurements

| Date | Machine | What | Result |
|---|---|---|---|
| 2026-11-04 | Ryzen 9 7900X, Windows 11, elevated, PawnIO 2.2.0 (AMDFamily17, LpcIO, SmbusPIIX4, RyzenSMU) | Sampler CPU after warm-up, every sensor (168) read at 1 Hz (`-log`), 40 s window | 0.3.0: 3.24% of one core · S0: **2.42%** |
| 2026-11-04 | same | Mean read time per source (S0) | cpu+board 193 ms (≈19 ms CPU), storage 61 ms every 10 s, gpu 1.5 ms, memory and battery ≈0 |
| 2026-11-04 | same | First pass (`-sensors`), one-time initialization | SMBus detection 1.1 s, OS counters 0.34 s, CPU monitor 0.2–0.5 s |
| 2026-11-05 | same, with VALORANT and NVIDIA FrameView's busy PresentMon loading the CPU (96 °C) | S1-D05. The "193 ms" was the first read's 1.7 s of discovery averaged in; steady state was ~22 ms in one pass (Super I/O 11, DIMMs 8, CPU 3). After the split, with the first read reported apart | cpu 7.5–19 ms mean (typical tick ~3 ms; rare ~30 ms spikes on the busy cores), board 11 ms, dimms 9 ms, gpu 1.7 ms. Per-core clocks: 3.5–32 ms → **0.4–0.6 ms** typical (one move per core for APERF+MPERF, time-critical priority while pinned, MSR probe cached) |

