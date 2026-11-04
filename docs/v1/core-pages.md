# Existing pages, tray and mini monitor

> Part of the [1.0 plan](README.md). Status: design. Delivered in stage **S1**.
> Philosophy: **show 100% of what the OS and the hardware expose**, organized so beginners and experts both find what they need.

## 1. Patterns shared by every page

Decided once and reused everywhere ([engineering §4](engineering.md)):

| Pattern | What it is |
|---|---|
| **Detail level** per page: *Basic · Advanced · Expert* | The same data at three depths. Basic is what a gamer wants. Expert adds registers, raw values and hex. Nobody drowns, and nothing is hidden |
| **Global search** (`Ctrl+K`, a command palette) | Searches every page ("tRFC", "PCIe", "Spooler", "cs2") and also runs actions ("Start capture", "Save report") |
| **Capability banner** (P3) | At the top of each page: what's missing and why, with the elevation button |
| Copy and pin | Every row and card can be copied, copied as report text, or **pinned to the Overview** |
| Units | °C/°F, MHz/GHz, MB/MiB, decimal places. One global setting |
| Raw values | A "show raw" toggle (hex/register) at the Expert level |
| One file per page | `gui/pages/<page>.rs`, out of `gui/mod.rs` ([engineering §9](engineering.md)) |

## 2. Overview: more information, condensed and configurable

Today it has 4 sparkline tiles, health pills and per-component cards (`gui/overview.rs`). Proposal:

- **Identity strip**: one dense line, where every item is clickable. Example: `Ryzen 9 7900X · RTX 4060 8 GB · X870-P WIFI · 32 GB DDR5-6000 CL30 (2×16, dual) · 2 TB NVMe · Windows 11 24H2`.
- **Health strip**, as pills:
  - temperatures against their limits;
  - throttling;
  - SMART;
  - battery wear;
  - **"Build health"** (the [bottleneck detector §2.5](bottleneck-detector.md) configuration checks: XMP off, single channel, reduced PCIe link…).
- **Configurable live tiles**, on the Widget Engine:
  - the user picks the metrics, the order (drag), the size and the density;
  - presets *Gamer*, *Overclocker*, *Laptop*, *Workstation*;
  - **every tile is a shortcut**: clicking opens the detailed page at the right spot (CPU temperature → Sensors, filtered; disk load → System › Performance › Disk).
- **Alerts** (opt-in, local): rules like "CPU > 90 °C for 30 s", "SMART warning", "disk under 10% free". They show on the Overview and in the tray, and are never sent anywhere.
- **Quick actions:** ZeeZ Live overlay, Start capture, Bottleneck detector, Stress test, Open System, Save report.
- **"Customize" mode** to edit all of the above. One click restores the defaults.

## 3. Processor

What exists: CPUID, topology, caches, instructions, P-states, clocks, SMU/MSR (`cpu/*`, `sensors/cpu.rs`). **New information:**

| Information | Windows | Linux | macOS |
|---|---|---|---|
| **Per-core table**: type (P/E/LP), CCD/CCX/cluster, effective and active clock, ratio, temperature, VID/voltage, power, load, **C-state residency**, preferred-core rank (CPPC) | 🧩 | ✅/🔑 (`cpuidle`, `acpi_cppc`) | ⚠ (IOReport: per-cluster residency and frequency 🔬) |
| A visual **topology diagram** (socket → CCD → CCX → core → thread, with caches drawn in) | ✅ | ✅ | ✅ (clusters) |
| Limits: PL1/PL2/PL4, Tau, PPT/TDC/EDC, PBO/Curve Optimizer (read), boost limits | 🧩 | 🔑 | ❌ |
| **Vulnerabilities and mitigations** (Spectre, Meltdown, Retbleed, Downfall, Inception…) | ✅ `NtQuerySystemInformation` (*SpeculationControl*) | ✅ `/sys/devices/system/cpu/vulnerabilities` | ⚠ |
| Virtualization: VT-x/AMD-V enabled, hypervisor present, **VBS/HVCI** (Windows) | ✅ | ✅ | ✅ |
| OS power: active plan / governor / **EPP** / Low Power Mode | ✅ `powercfg` | ✅ `cpufreq` | ✅ |
| Microcode and AGESA | ✅ (exists) | ✅ | n/a |
| Instructions grouped (SIMD, crypto, virtualization, security), searchable and briefly explained | ✅ | ✅ | ✅ |

## 4. Mainboard

| Information | Windows | Linux | macOS |
|---|---|---|---|
| A **full SMBIOS browser**: every type, including OEM and raw, as a tree | ✅ | ✅/🔑 | ✅ Intel · ⚠ ARM |
| **PCIe tree** with the **current × maximum** link of every device, and **alerts** ("GPU at x8 Gen 4", "NVMe at x2") | ✅ | ✅ | ⚠ |
| Firmware: UEFI/CSM, **Secure Boot**, **TPM** (version and state), **Resizable BAR**, Above 4G | ✅ | ✅ (`efivars`, `/sys/class/tpm`) | ⚠ Secure Boot (T2/Apple Silicon) |
| ACPI tables (list, headers, dump at Expert level) | ✅ `GetSystemFirmwareTable('ACPI')` | 🔑 `/sys/firmware/acpi/tables` | ❌ |
| Super I/O: chip and sensors **with the board map's names** (exists for 150+ boards), fans | 🧩 | ✅ hwmon | ❌ (SMC) |
| Slots, ports and onboard devices (SMBIOS 8/9/41) | ✅ | ✅ | ⚠ |

## 5. Memory (+ SPD)

| Information | Windows | Linux | macOS |
|---|---|---|---|
| **Full timings**: primary, secondary and tertiary (tRRD_S/L, tFAW, tWR, tRTP, tWTR_S/L, tREFI, tRFC1/2/sb, tCWL, tCKE, GDM, Power-Down, Gear, UCLK/MCLK/FCLK ratio) | 🧩 (AMD UMC / Intel MCHBAR; partial today) | 🔑 | ❌ |
| Topology: channels, sub-channels (DDR5), ranks, bank groups, which stick sits on which channel | 🧩/✅ | 🔑/✅ | ⚠ |
| **Active profile**: current settings vs JEDEC and XMP/EXPO ("running EXPO 1 with a tuned tRFC") | 🧩 | 🔑 | ❌ |
| ECC: type (SMBIOS 16) + **error counters** | ⚠ (WHEA 🔬) | ✅ EDAC | ❌ |
| DIMM temperature, PMIC (DDR5) | 🧩 | 🔑 | ❌ |
| OS memory composition (in use, standby, compressed, *wired*), paging/swap | ✅ | ✅ | ✅ |
| Theoretical vs measured bandwidth (link to the memory benchmark) | ✅ | ✅ | ✅ |

## 6. Storage and Devices

### 6.1 Per drive: "nothing left out"

| Information | Windows | Linux | macOS |
|---|---|---|---|
| Identity: model, firmware, serial (masked in screenshots by default, as today) | ✅ | ✅ | ✅ |
| Complete **NVMe Identify** (controller and namespace): MDTS, queues, **power states** with latencies, APST, LBA formats, volatile write cache, sanitize/format, firmware slots, HMB | ✅ `IOCTL_STORAGE_QUERY_PROPERTY` (protocol-specific) | ✅ 🔑 NVMe ioctl | ⚠ IOKit |
| **ATA IDENTIFY**: NCQ, TRIM, DEVSLP, APM/AAM, security, logical/physical sectors, rotation, form factor | 🔑 | 🔑 | ⚠ |
| Full SMART with thresholds and per-vendor names; **thermal-throttle count and time** (NVMe); error log; self-test log | ✅/🔑 (partially exists) | ✅/🔑 | ⚠ |
| Link: PCIe gen × lanes current/max; SATA 1.5/3/6 Gb/s; USB (enclosures) | ✅ | ✅ | ⚠ |
| **Partitions and volumes**: GPT/MBR, type and flags, **alignment** (offset % 1 MiB), file system, cluster size, encryption (**BitLocker / LUKS / FileVault**), mount points, page-file location | ✅ | ✅ | ✅ |
| **TRIM enabled in the OS** (`DisableDeleteNotify` / `discard`, `fstrim.timer` / APFS TRIM) and write-cache policy | ✅ | ✅ | ✅ |
| Live: read/write, IOPS, queue depth, latency, active time | ✅ | ✅ | ✅ |

### 6.2 Other devices

| Information | Windows | Linux | macOS |
|---|---|---|---|
| **USB tree** with **negotiated × supported** speed ("external SSD on a USB 2.0 port"), Thunderbolt/USB4 | ✅ | ✅ | ✅ |
| Battery level of Bluetooth peripherals (mouse, headset) | ⚠ WinRT GATT 🔬 | ✅ BlueZ `Battery1` | ⚠ IOBluetooth 🔬 |
| Monitors: EDID (exists) + **current mode** (resolution, Hz), HDR, VRR, colour depth | ✅ | ✅/⚠ Wayland | ✅ |
| Audio: devices, current sample rate and bit depth | ✅ | ✅ | ✅ |
| Network: what exists + live throughput and connections (links to System) | ✅ | ✅ | ✅ |

## 7. Benchmarks and tools

### 7.1 Principles
- **Safe by default:** no test writes anywhere except a **test file** in a folder the user picks, and no test changes settings. Stress tests auto-stop on temperature (this exists).
- **Reproducible:** each result records version, configuration and conditions (starting temperature, power plan). A "run N times" mode reports the median and the spread.
- **Our own documented score:** 1000 points = a public reference machine (the Ryzen 9 7900X in `reference/`). Every score shows its sub-tests. As FEATURES.md says, it isn't comparable with CPU-Z.

### 7.2 New tests

| Component | Tests | Per OS |
|---|---|---|
| **CPU** | Integer (compression), floating point (n-body), **SIMD** (GEMM GFLOPS with SSE/AVX2/AVX-512/NEON), crypto (AES and SHA, with and without hardware instructions), ray tracer, branch-heavy sorting, **1…N thread scaling**, **P × E** on hybrids, **10-minute sustained** (score over time = throttling drop) | ✅ all (core selection ❌ on macOS) |
| **Core-to-core latency** (core × core heat map, atomic ping-pong) | Shows the cost across CCDs and clusters, a figure enthusiasts love | ✅ Win/Linux · ⚠ macOS (no affinity: statistical) |
| **Memory** | What exists + random access (GUPS), STREAM triad, huge-page effect, per-CCD/NUMA latency | ✅ |
| **GPU** (`wgpu`, see [dependencies.md](dependencies.md)) | FP32/FP16 GFLOPS, VRAM bandwidth, **PCIe host↔GPU bandwidth** (confirms the real link), a raster scene | ✅ Vulkan/DX12/Metal |
| **Storage** | CrystalDiskMark-style: SEQ1M Q8T1/Q1T1, RND4K Q32T16/Q1T1, read and write, configurable size and passes, uncached I/O (`FILE_FLAG_NO_BUFFERING` / `O_DIRECT` / `F_NOCACHE`). **Sustained write** (SLC cache exhaustion curve), with a warning stating how many GB it writes | ✅ |
| **Stability** | CPU stress (light SSE, heavy AVX2/AVX-512, mixed), **user-mode memory test** (patterns over the chosen GB, clearly labelled "not a replacement for MemTest86/TestMem5"), GPU stress, combined (PSU and thermals) | ✅ |

### 7.3 SSD and HDD health

| Tool | Windows | Linux | macOS |
|---|---|---|---|
| **SMART self-test** (short, extended, conveyance) with progress and history | 🔑 ATA pass-through · NVMe 🔬 (Windows filters *admin* commands; validate *Device Self-test*) | 🔑 ✅ | ❌ (no ATA pass-through without a third-party driver) |
| Health panel: life left, TBW written vs rated TBW (entered by the user, since drives don't report it), temperature over time, error counts, alerts | ✅ | ✅ | ⚠ |
| **Read-only surface scan** (slow and unreadable sectors) | 🔑 | 🔑 | 🔑 ⚠ |

### 7.4 Optimization (future, after 1.0)

**Principle:** **don't reimplement** defragmentation or TRIM. Use the OS's own engine, which is correct and data-safe, and show its progress and result.

| | Windows | Linux | macOS |
|---|---|---|---|
| HDD: analyze and defragment | `defrag /A` and `defrag /O` (the "Optimize Drives" engine) 🔑 | `e4defrag` (ext4), `btrfs filesystem defragment` (with a snapshot warning) 🔑 | n/a (APFS) |
| SSD: manual TRIM | `defrag /L` (ReTrim) 🔑 | `fstrim` 🔑 | n/a (automatic; `trimforce` only with a warning) |
| Checks | TRIM on, alignment, AHCI/NVMe, write cache, firmware | same + I/O scheduler | TRIM on |

- **No "magic optimizers"** (disabling indexing or prefetch, "cleaning the registry"). Myths stay out of the product.

## 8. Tray icon

Today (`gui/tray.rs`): a 32×32 icon with the CPU temperature in 3×5 digits, a one-line tooltip and a static menu (Show, Sensors, Quit).

| Improvement | Detail |
|---|---|
| **Several icons**, one per metric (like HWiNFO) | The user picks them: CPU °C, GPU °C, load, clock, game FPS, RAM… |
| **Icon styles** | Number, bar, **mini graph**, ring. Text rendered at the icon's real size (16/20/24/32 px per DPI), not today's 3×5 font |
| **Colour by range** | Reuses the Widget Engine thresholds, always paired with a shape cue for colour-blind users |
| **Rich tooltip** | Several lines with the chosen metrics |
| **Dynamic menu** | Show/hide app, Mini monitor, **ZeeZ Live overlay**, **Start/stop capture** (with ● REC), Bottleneck detector, Open System, **Pause monitoring**, update speed, Quit. Labels re-translated when the language changes |
| Configurable left click | Open the app / mini monitor / overlay |
| **macOS** | Menu-bar text with several user-chosen metrics (`CPU 62° · GPU 55°`) |
| **Honest limits** | Windows 11 hides new icons in the overflow menu. That can't be forced, so we show how to pin it. GNOME needs the AppIndicator extension; without it the app falls back to the mini monitor (as today) and explains why |

## 9. Mini monitor

Today (`gui/sensors.rs:272`): a fixed 230×96 always-on-top window with temperatures, average MHz and W. There are no options and it isn't persisted.

**Proposal:** the mini monitor becomes **a Widget Engine surface in "window mode"**. It shares layouts, widgets and the editor with the ZeeZ Live overlay, with no duplicated code.

| Improvement | Detail |
|---|---|
| **Layouts** | Vertical list, horizontal bar, grid, "one big number", and any overlay layout |
| Size and scale | Resizable, with font scaling |
| **Appearance** | Opacity, theme (follow the app / dark / light / custom), corner radius, outline |
| **Lock** | A click-through mode (it becomes a desktop overlay), toggled by hotkey or from the tray |
| Position | Snaps to edges and corners; remembers position and monitor; **several mini monitors** (e.g. one per screen) |
| Behaviour | Hide while an app is fullscreen (or keep showing); expand on hover (compact → detailed) |
| Context menu | Change layout, edit, lock, close |
| **Persistence** | Every mini monitor and its position are saved (fixes D9) |
