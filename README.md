<div align="center">

<img src="assets/wordmark.png" width="440" alt="CpuZeeZ">

**A clean-room, open-source, cross-platform reimplementation of CPU-Z's feature set, plus a hardware monitor, written in Rust.**

Processor, caches, mainboard, memory and SPD, graphics, sensors, drives, network and batteries, benchmarks and full TXT/HTML/JSON reports, on **Windows, Linux (Debian, Ubuntu, Arch, …) and macOS (Intel and Apple Silicon)**.

[![CI](https://github.com/joaoregis/cpuzeez/actions/workflows/ci.yml/badge.svg)](https://github.com/joaoregis/cpuzeez/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/joaoregis/cpuzeez)](https://github.com/joaoregis/cpuzeez/releases)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

[Português](README.pt-BR.md) · [Download](#install) · [Screenshots](#screenshots) · [Changelog](CHANGELOG.md) · [Feature coverage](#feature-coverage) · [Clean-room process](#clean-room-process)

</div>

---

> CpuZeeZ (`cpuzeez`) is **not affiliated with CPUID**. CPU-Z is a trademark of CPUID. No CPU-Z code, data tables or text were used. See [Clean-room process](#clean-room-process).

## Contents

- [Highlights](#highlights)
- [Screenshots](#screenshots)
- [Install](#install)
- [Updates](#updates)
- [Privileged hardware access](#privileged-hardware-access)
- [Usage](#usage)
- [Feature coverage](#feature-coverage)
- [Accuracy: side by side with CPU-Z](#accuracy-side-by-side-with-cpu-z)
- [Clean-room process](#clean-room-process)
- [Building from source](#building-from-source)
- [Architecture](#architecture)
- [Contributing](#contributing)
- [License](#license)

## Highlights

- **A modern interface.**
  - Sidebar navigation and a live **Overview** dashboard: CPU, GPU and chipset temperatures with sparklines, plus health and throttling status.
  - Card-based pages in light and dark themes, using the OS's own UI font.
  - Available in English, Português (Brasil) and Español.
- **Everything CPU-Z reports.**
  - CPU and topology (CCD/CCX, P/E/LP cores).
  - Caches, mainboard, chipset and BIOS.
  - Memory timings: AMD from the UMC, Intel from MCHBAR, Skylake through Arrow Lake.
  - SPD: DDR to DDR5 and LPDDR4/5, with XMP, EXPO and EPP.
  - Graphics, PCI, DMI, timers and register dumps.
  - The same 23 report sections as CPU-Z's `-txt`, in the same order.
- **A hardware monitor.**
  - Every sensor with min/max/average, live charts and CSV logging.
  - Thermal, power and current **throttling detection**.
  - A **monitored stress test** that stops automatically at a temperature limit.
  - A tray / menu-bar icon with the CPU temperature, and an always-on-top mini monitor.
  - Sensor sources:
    - **CPU:** AMD SMU per-core temperatures, rails and PPT/TDC/EDC; Intel and AMD RAPL; per-core effective clocks.
    - **GPU:** temperature, hot spot, memory junction, fans, power, load.
    - **Mainboard:** Super I/O voltages, temperatures and fans, with maps for more than 150 board models.
    - **Chipset, DIMMs and drives.**
- **Benchmarks.**
  - CPU single- and multi-threaded, in Scalar, AVX2, AVX-512 and NEON variants.
  - **Memory and cache latency and bandwidth.**
  - A local **benchmark history** to compare before and after an overclock or a BIOS update.
- **Devices.** Full **SMART** for NVMe and SATA, **battery health** (wear, cycles), and **network adapters** (link, IPs, DNS, driver, Wi-Fi).
- **Tools.**
  - **Compare reports**: what changed after a hardware or BIOS change.
  - Live clocks of every core and GPU, timers, and TXT/HTML/JSON export.
- **CPU-Z's command line**:
  - CPU-Z's own flags: `-txt=`, `-html=`, `-json=`, `-console`, `-bench`, `-core=`, `-coremask=`.
  - New flags: `-sensors`, `-log=`, `-membench`, `-diff=`, `-checkupdate`.
  - A `cpuzeez.ini` with the same keys as `cpuz.ini`.
- **One-click updates.** The app finds new releases and installs the right package for your OS, after verifying its SHA-256.
- **A single native binary**, about 8 MB, with no runtime dependencies.

## Screenshots

Captured with `cpuzeez -screenshots=DIR`, which renders every page and saves a PNG. The Windows set comes from an AMD Ryzen 9 7900X, MSI PRO X870-P WIFI, 2×16 GB DDR5-6000 and an RTX 4060, running elevated with PawnIO. Serial numbers, MACs and IPs are masked in this mode.

### Windows 11

| Overview | Processor | Memory |
|---|---|---|
| ![Overview](docs/screenshots/windows/overview.png) | ![Processor](docs/screenshots/windows/processor.png) | ![Memory and SPD](docs/screenshots/windows/memory.png) |
| **Sensors** | **Stress test** | **Memory benchmark** |
| ![Sensors](docs/screenshots/windows/sensors.png) | ![Monitored stress test](docs/screenshots/windows/stress.png) | ![Memory and cache benchmark](docs/screenshots/windows/membench.png) |
| **Devices** | **Graphics** | **Mainboard** |
| ![Devices](docs/screenshots/windows/devices.png) | ![Graphics](docs/screenshots/windows/graphics.png) | ![Mainboard](docs/screenshots/windows/mainboard.png) |
| **Benchmark history** | **Compare reports** | **Settings** |
| ![History](docs/screenshots/windows/history.png) | ![Compare reports](docs/screenshots/windows/compare.png) | ![Settings](docs/screenshots/windows/settings.png) |
| **Light theme, Português** | **About, Português** | **Clocks** |
| ![Overview, light theme in Portuguese](docs/screenshots/windows/overview-light.png) | ![About in Portuguese](docs/screenshots/windows/about-pt.png) | ![Clocks](docs/screenshots/windows/clocks.png) |

### Linux

Ubuntu 24.04 (WSLg, Wayland) on the same machine, with NVIDIA through NVML:

| Overview | Sensors | Devices |
|---|---|---|
| ![Linux Overview](docs/screenshots/linux/overview.png) | ![Linux Sensors](docs/screenshots/linux/sensors.png) | ![Linux Devices](docs/screenshots/linux/devices.png) |

### macOS

Captured by CI on GitHub's macOS runners. These are virtual machines, so some fields show what the hypervisor exposes:

| Apple Silicon: Overview | Apple Silicon: Processor | Intel Mac: Overview |
|---|---|---|
| ![macOS ARM Overview](docs/screenshots/macos/overview.png) | ![macOS ARM Processor](docs/screenshots/macos/processor.png) | ![macOS Intel Overview](docs/screenshots/macos-intel/overview.png) |

## Install

Every [release](https://github.com/joaoregis/cpuzeez/releases) ships an installer per platform, plus `SHA256SUMS.txt` and the [changelog](CHANGELOG.md) section for that version:

| Platform | File | Notes |
|---|---|---|
| Windows 10/11 x64 | `cpuzeez-<ver>-windows-x64.msi` | Per-machine install, Start-menu shortcut, adds `cpuzeez` to `PATH` |
| Windows (portable) | `cpuzeez-<ver>-windows-x64-portable.zip` | Just `cpuzeez.exe` |
| macOS 11+ (Intel and Apple Silicon) | `cpuzeez-<ver>-macos-universal.dmg` | Drag `CpuZeeZ.app` to Applications |
| macOS (installer) | `cpuzeez-<ver>-macos-universal.pkg` | Installs the app and `/usr/local/bin/cpuzeez` |
| Debian 12+, Ubuntu 22.04+ | `cpuzeez_<ver>_amd64.deb` / `_arm64.deb` | `sudo apt install ./cpuzeez_<ver>_amd64.deb` |
| Arch Linux | `cpuzeez-bin-<ver>-1-x86_64.pkg.tar.zst` | `sudo pacman -U cpuzeez-bin-*.pkg.tar.zst` |
| Any Linux (glibc 2.35+) | `cpuzeez-<ver>-linux-<arch>.tar.gz` | Binary, `.desktop` file and icons |

> **No publisher certificate.** The installers are not signed with a publisher certificate, because that needs a paid certificate. macOS executables use ad-hoc signatures, which neither identify the developer nor provide Apple notarization.
> - **Windows:** SmartScreen may say "Windows protected your PC". Click *More info → Run anyway*.
> - **macOS:** Gatekeeper may block the app or the `.pkg` because Apple cannot verify the developer.
>   1. Check your download against `SHA256SUMS.txt`.
>   2. Try to open it once and dismiss the alert.
>   3. Go to *System Settings → Privacy & Security → Open Anyway* and confirm *Open*.
>
>   For the `.pkg`, finish installing and repeat for the app if prompted. On macOS 11–12 this setting is under *System Preferences → Security & Privacy → General*. Right-click → *Open* no longer bypasses this check on macOS 15 and later; see [Apple's instructions](https://support.apple.com/en-us/102445).

With Rust installed you can also run `cargo install --git https://github.com/joaoregis/cpuzeez`.

## Updates

**Update notice.** At startup CpuZeeZ asks GitHub's releases API whether a newer version exists. That is one anonymous `GET`; nothing about your machine is sent.
- If there is one, a bar offers **Download & install**. It fetches the package for your OS and CPU, verifies its SHA-256 against the release's `SHA256SUMS.txt`, then launches it:
  - **Windows:** the MSI.
  - **macOS:** the DMG or PKG.
  - **Debian/Ubuntu and Arch:** `pkexec` with `apt` or `pacman`.
  - **Anywhere else:** it opens the download folder.
- **What's new** opens the release notes.
- You can also check from *About ▸ Check for updates*, or with `cpuzeez -checkupdate`.
- To turn the check off, set `CheckUpdates=0` in `cpuzeez.ini`.
- It uses the system `curl`, which ships with Windows 10+, macOS and desktop Linux distros.

**Upgrading by hand** means installing the new version over the old one:

| Platform | How |
|---|---|
| Windows | Run the new `.msi`. It upgrades in place: same folder, a single entry in *Installed apps*, PATH and shortcut kept. Installing an older version over a newer one is refused. CI checks install → upgrade → downgrade block → uninstall on every push. |
| macOS | Drag the new `CpuZeeZ.app` over the old one, or run the new `.pkg` |
| Debian / Ubuntu | `sudo apt install ./cpuzeez_<ver>_amd64.deb` |
| Arch | `sudo pacman -U cpuzeez-bin-<ver>-1-x86_64.pkg.tar.zst` |

## Privileged hardware access

Like CPU-Z, some information can only come from hardware registers: MSRs, the SMBus where the SPD EEPROMs live, the Super I/O sensor chip, the memory controller and the AMD SMU. CPU-Z installs its own kernel driver for this. **CpuZeeZ never ships a driver.** It uses what each OS already offers, and simply leaves out what it can't read.

| | Without privileges | With privileges |
|---|---|---|
| **Windows** | CPU, caches, topology, DMI, BIOS, PCI list and links, chipset names, memory type/size/channels, NVMe SMART, network, batteries, GPUs (NVML/NVAPI/ADL) with temperatures and clocks, monitors, OS, timers, benchmarks | **+ PawnIO** (run as administrator): real core clocks (APERF/MPERF), P-states, preferred cores, SPD, memory timings, Super I/O voltages/temperatures/fans, CPU temperatures and power, AMD SMU, Intel PCH temperature, DIMM temperatures, ATA SMART thresholds |
| **Linux** | All of the above, plus hwmon sensors (`k10temp`, `coretemp`, `nct6775`, `it87`, `amdgpu`…), cpufreq clocks, RAPL power, and SPD when `ee1004`/`spd5118` are bound | **+ root**: MSRs (`modprobe msr`), full PCI config space, SPD over `i2c-dev`, Super I/O via `/dev/port`, AMD SMN/UMC, Intel MCHBAR, SMART via ioctls, and the SMU table via the optional `ryzen_smu` module |
| **macOS** | CPU (sysctl / CPUID), caches, clusters, SMBIOS (Intel) or IODeviceTree (Apple Silicon), memory size and type, PCI (IOKit), GPU and Metal, displays (EDID), storage, USB, network, SMC temperatures/fans/power, battery health, benchmarks | Nothing more: macOS doesn't allow third-party register access, and Apple Silicon has no SPD, MSRs or Super I/O |

### Windows: PawnIO

[PawnIO](https://pawnio.eu) is a signed, open-source, scriptable kernel driver, also used by LibreHardwareMonitor and FanControl.

1. Install it: `winget install namazso.PawnIO`, or get the installer from [pawnio.eu](https://pawnio.eu).
2. Download the official modules ([PawnIO.Modules releases](https://github.com/namazso/PawnIO.Modules/releases), LGPL-2.1). Extract the `.bin` files into a `pawnio-modules` folder next to `cpuzeez.exe`, or into the PawnIO install folder.
3. Run CpuZeeZ **as administrator**. *About* (and the report's *Binaries* section) then shows `Hardware access: PawnIO 2.2.0 (modules: …)`.

Details are in [docs/windows-pawnio.md](docs/windows-pawnio.md).

### Linux: kernel interfaces

```bash
sudo modprobe msr i2c-dev   # MSRs and SMBus
```
```bash
sudo modprobe spd5118       # DDR5 SPD (kernel 6.11+), or ee1004 for DDR4
```
```bash
sudo cpuzeez                # full detail; without sudo you still get most of it
```

## Usage

Run `cpuzeez` with no arguments to open the app. All of CPU-Z's command-line options work, plus a few new ones:

| Option | What it does |
|---|---|
| `-txt=FILE` | Full TXT report (CPU-Z's register-dump layout). `.txt` is added if missing |
| `-html=FILE` | HTML report (light/dark aware) |
| `-json=FILE` / `-jsontxt=FILE` | JSON report (the same tree as TXT) |
| `-console` | Print the TXT report to stdout |
| `-bench` | CPU benchmark (single and multi thread), saved as `<hostname>.txt` and to the history |
| `-membench` | Memory and cache latency and bandwidth (`-membench=quick` for a short run) |
| `-sensors` | Print every sensor once, with its id |
| `-log=FILE[,SECONDS]` | Log every sensor to CSV each second until SECONDS or Ctrl+C |
| `-diff=OLD.json,NEW.json` | Compare two reports (`NEW` can be `now`; add `,all` to include live values) |
| `-core=N` | Print core #N's clock and exit |
| `-coremask=HEX` | Restrict `-bench` to these logical CPUs |
| `-checkupdate` | Check GitHub for a newer release |
| `-screenshots=DIR` | Render every page to PNG and exit (docs/CI) |

GUI keys, as in CPU-Z:
- **F5** saves the current page as `.bmp`.
- **F6** copies it to the clipboard.
- **F7** saves a `.cvf` file (our JSON report).

### `cpuzeez.ini` and settings

`cpuzeez.ini` sits next to the executable and holds the hardware-probe switches. It uses the same keys as `cpuz.ini`, and CPU-Z's own UTF-16 file is read as is:

```ini
[CPU-Z]
PCI=1          ; 0 also disables chipset, SPD and SMBus
DMI=1
Sensor=1
SMBus=1
SPD=1
LPCIO=1
Display=1
UseDisplayAPI=1 ; NVML / NVAPI / ADL
SMART=1
CheckUpdates=1  ; update notice at startup
```

Interface preferences live in the per-user data folder, editable on the **Settings** page:
- language, theme, tray icon, close to tray;
- sensor sampling period, graph history, warning temperature.

The folder is `%APPDATA%\CpuZeeZ`, `~/Library/Application Support/CpuZeeZ` or `~/.config/cpuzeez`. It also holds the benchmark history (`history.json`) and the sensor logs.

Mapping a new mainboard's sensors: `CPUZEEZ_EC_DUMP=1 cpuzeez -txt=board` adds the raw Super I/O registers and the unmapped inputs to the report. See [docs/boards.md](docs/boards.md).

## Feature coverage

The full spec, built from CPU-Z's own report and readme, is [FEATURES.md](FEATURES.md). What changed in each version is in [CHANGELOG.md](CHANGELOG.md).

Legend:
- ✅ implemented and verified on the reference machine (Windows natively; Linux under WSL2, which only exposes what a VM can see)
- ☑️ implemented and compile/CI-tested on that OS, not yet checked on real hardware
- 🔑 needs admin/root (see above)
- ❌ not possible on that platform

| Feature | Windows | Linux | macOS |
|---|---|---|---|
| Processor name, vendor, family/model/stepping, brand, instruction sets | ✅ | ✅ | ✅ (CI; ARM via sysctl / MIDR) |
| Codename, package, process, stepping name, TDP, Tjmax | ✅ | ✅ | ✅ (CI) |
| Topology tree, CCD/CCX, hybrid P/E/LP cores, CPU groups (>64 threads) | ✅ | ✅ | ☑️ (perf levels) |
| Caches: size, ways, line, instances, sharing | ✅ | ✅ | ✅ (CI) |
| Thread dumps: raw CPUID and MSRs per thread | ✅ (MSR 🔑) | ✅ (MSR 🔑) | ☑️ (x86) |
| P-states, preferred cores (CPPC), measured BCLK | ✅🔑 | ☑️🔑 | ❌ |
| Core clock: APERF/MPERF, or OS, or measured | ✅🔑 / measured | ✅ measured / ☑️ cpufreq | ✅ measured (CI) |
| AMD SMU: per-core/L3/SoC temperatures, VDDCR rails, PPT/TDC/EDC, FCLK/UCLK/MCLK, Fmax, per-core max ratio | ✅🔑 (Zen 4 verified) | ☑️🔑 (`ryzen_smu`) | ❌ |
| CPU temperatures, VID, package/core power (RAPL), per-core clocks | ✅🔑 | ☑️ hwmon / 🔑 | ☑️ SMC |
| Timers: ACPI, perf, system | ✅ | ✅ | ☑️ |
| BIOS: UEFI, date, AGESA / microcode | ✅ | ✅ | ☑️ (iBoot) |
| DMI / SMBIOS: every type CPU-Z shows | ✅ | ☑️🔑 (partial without root) | ☑️ Intel / synthesized on ARM |
| Chipset: northbridge/southbridge + revision, bus specification, graphic interface link | ✅ | ☑️ | ☑️ |
| Memory type, size, channels | ✅ | ✅ | ☑️ (incl. Apple Silicon) |
| Memory frequency, CL/tRCD/tRP/tRAS/tRC/tRFC, CR, uncore/fabric | ✅🔑 AMD · ☑️🔑 Intel | ☑️🔑 | ❌ |
| SPD: all fields, JEDEC/XMP/EXPO tables, hex dump | ✅🔑 | ☑️ (`spd5118`/`ee1004`) / 🔑 | ❌ (soldered memory) |
| DIMM temperatures | ✅🔑 | ☑️ | ❌ |
| LPCIO: Super I/O vendor, model, IDs, config registers | ✅🔑 | ☑️🔑 | ❌ |
| Mainboard sensors: voltages, temperatures, fans | ✅🔑 (NCT6687 verified) | ☑️ hwmon | ☑️ SMC |
| Chipset temperature | ✅🔑 (AMD via Super I/O) · ☑️ Intel PCH | ☑️ | ❌ |
| GPU: specs, clocks, power/thermal limits, VRAM and vendor, link, driver | ✅ (NVML, NVAPI, ADL, DXGI) | ✅ (DRM, NVML) | ☑️ (Metal) |
| GPU sensors: temperature, hot spot, memory junction, fans, power, load, throttling | ✅ | ☑️ | ❌ |
| Monitors (EDID) | ✅ | ✅ | ☑️ |
| PCI device list, IDs, names, links | ✅ | ✅ | ☑️ |
| PCI capabilities decode and config hex dump | ❌¹ | ✅ (full with root) | ❌ |
| Storage: model, serial, firmware, capacity, bus, link, volumes | ✅ | ✅ | ☑️ |
| SMART: health, attributes, temperature, wear, hours, data read/written | ✅ (ATA thresholds 🔑) | ☑️🔑 | ☑️ (`smartctl` if installed) |
| Battery health: design vs full capacity, wear, cycles | ☑️ | ☑️ | ☑️ |
| Network adapters: link, MAC, IPs, gateway, DNS, driver, Wi-Fi | ✅ | ✅ | ☑️ |
| USB devices, NPU / AI devices | ✅ | ✅ | ☑️ |
| Software: OS, edition, build, install date, DirectX / Vulkan / Metal | ✅ | ✅ | ☑️ |
| Sensors page, CSV log, throttling detection, tray / mini monitor | ✅ | ✅ (mini monitor; tray needs a StatusNotifier host) | ☑️ |
| CPU benchmark, memory/cache benchmark, history, stress test | ✅ | ✅ | ✅ (CI) |
| Reports (TXT/HTML/JSON), compare reports, F5/F6/F7 | ✅ | ✅ | ✅ (CI) |

¹ No PawnIO module offers generic PCI configuration access on Windows. Device IDs, names and links still come from the OS.

### Deliberately not included

- **Online validation** (valid.x86.fr, Validator Statistics). It is CPUID's service, so CpuZeeZ doesn't submit to it. Share the HTML/JSON report instead.
- **CPU-Z's benchmark algorithm and reference scores.** They are CPUID's work. CpuZeeZ's benchmarks are its own, and their scores are not comparable with CPU-Z's.
- **Obsolete targets:** Itanium/IA-64, Windows 9x, and CPUs older than i686. None of today's target OSes run on them.

## Accuracy: side by side with CPU-Z

On the reference machine (Ryzen 9 7900X, MSI PRO X870-P WIFI, Windows 11 25H2), `cpuzeez -txt` was compared section by section with `cpuz.exe -txt`.

**Same output as CPU-Z:**
- **CPU and topology:**
  - APIC tree, instruction sets, caches, microcode.
  - Codename, package and stepping (`Raphael`, `Socket AM5 (LGA1718)`, `RPL-B2`).
  - P-states (`47.00x - 1.256 V`), preferred cores.
- **AMD SMU:**
  - Version `84.79.225`, `PMTV 00540004`, PPT/TDC/EDC 230 W / 160 A / 225 A, `Max Frequency 5700 MHz`.
  - VDDCR_VDD/SOC/MISC rails and the per-core / L3 / SoC temperature layout.
- **Chipset and memory:**
  - `Bus Specification PCI-Express 5.0 (32.0 GT/s)`.
  - DDR5 timings `30-36-36-76`, tRC 112, ratio 1:30, `Memory Max Frequency 2800.0 MHz`.
- **SPD:** the 2 × 1024 bytes read live are byte-identical to CPU-Z's, and so is every decoded field.
- **Super I/O:** the NCT6687 registers, voltages, temperatures and fans (`Fan 15 SYS 2`, `Fan 16 SYS 1`).
- **DMI, storage, USB, GPUs and software:**
  - RTX 4060 `AD107-400`, 3072 cores, memory vendor `Samsung`, limits and clocks.
  - The monitor EDID and the OS line.
- **Timers.**

**Intentional differences:**
- **BIOS ROM size:** we print `32768 KB`, the SMBIOS extended ROM-size field; CPU-Z ignores it and shows `16384 KB`.
- **RTX 4060 TMUs:** we print `96`, the public spec (24 SMs × 4); CPU-Z prints 192.
- **Live values** depend on the moment: temperatures, clocks and per-core max ratios. Ratios are measured the way CPU-Z describes them (each core loaded alone), so they vary with temperature.
- **Reference clock:** measured from TSC ÷ base ratio. It gives `Memory Frequency 2999.9 MHz` where CPU-Z shows `2998.8 MHz`.

**Not decoded:**
- The fused limits CPU-Z gets from an SMU mailbox call (`fused: …`) and the PBO scalar.
- PCI capability decoding on Windows.

## Clean-room process

CpuZeeZ was built so that no expression of CPU-Z (its code, tables or text) could end up in this project:

1. **Black-box observation.** CPU-Z's documented `-txt` option produced its own report of a test machine. That report shows what CPU-Z reports and in which format, not how it does it.
2. **Feature inventory.** The readme and the exported string and RTTI class names of `cpuz.exe`, inspected with [REA](https://github.com/morluto/rea) and Ghidra, list which features and hardware families exist. **No function was decompiled or translated.**
3. **Spec.** Both were condensed into [FEATURES.md](FEATURES.md).
4. **Implementation from public sources only:**
   - CPU and firmware: Intel SDM and datasheets, AMD APM/PPR, DMTF SMBIOS.
   - Memory: JEDEC SPD annexes, JEP106, Intel XMP and AMD EXPO layouts.
   - Buses and displays: PCI/PCIe specs, VESA EDID.
   - Drives: NVMe and ATA SMART.
   - Chips and GPUs: chip datasheets, NVML/ADL/DXGI/IOKit docs, Linux sysfs ABIs.
   - Community documentation for undocumented-but-public interfaces (AMD PM table cell layouts, NVAPI function IDs, board sensor labels), used as facts and rewritten in our own code.
5. **Verification against behaviour.** The output was compared with CPU-Z's report on the same hardware. Where they differ, we follow the public spec.

## Building from source

Requires Rust 1.89+ (`rustup`). There are no system libraries to install, on Linux included: the GUI loads X11/Wayland/OpenGL at runtime, and the Linux tray icon talks D-Bus in pure Rust.

```bash
git clone https://github.com/joaoregis/cpuzeez
```
```bash
cargo build --release
```
```bash
cargo test
```
```bash
cargo build --release --no-default-features
```

The last command builds the CLI only, without the GUI.

CI ([`ci.yml`](.github/workflows/ci.yml)):
- **Native runners:** Windows, Ubuntu x64/arm64 and macOS arm64/x64. Each job runs rustfmt, clippy and the tests, then the binary itself (every report format, `-core`, `-bench`, GUI screenshots).
- **Containers:** Debian, Ubuntu 22.04 and Arch Linux.
- **MSI test:** install, upgrade in place, downgrade block and uninstall.

Release ([`release.yml`](.github/workflows/release.yml)): pushing a `v*` tag builds the MSI (WiX), the DMG/PKG (universal binary, signatures verified), the `.deb` (cargo-deb) and the Arch package (makepkg). It attaches them to a GitHub release whose notes are this version's [CHANGELOG](CHANGELOG.md) section.

## Architecture

```
src/
├── main.rs        CLI (CPU-Z options + -sensors/-log/-membench/-diff), Ctx, section order
├── report.rs      report tree (Field / Group / Hex) → TXT, HTML, JSON (and back, for diffs)
├── config.rs      cpuzeez.ini              settings.rs   per-user GUI preferences + data folder
├── hw/            privileged access: trait Hw { rdmsr, pci_read, io, smn, smbus, smu, mchbar, pch }
├── cpu/           CPUID (x86.rs), ARM (arm.rs), clocks, names database (db.rs)
├── smbios.rs  bios.rs  chipset.rs (+ chipset/intel.rs, chipset/macos.rs)  pci/  spd/
├── sensors/       Super I/O + board maps, AMD SMU (amd_pm.rs), RAPL, hwmon, SMC, PDH
├── telemetry.rs   sampler: series, min/max/avg, CSV, throttling detection
├── gpu/  npu.rs  storage.rs (+ storage/smart.rs)  battery.rs  network.rs  usb.rs  software.rs
├── bench.rs  membench.rs  history.rs  diff.rs  timers.rs  update.rs
└── gui/           egui front-end:
                   theme.rs + widgets.rs  design system
                   pages: overview, sensors, stress, devices, tools, settings_ui
                   tray.rs, plot.rs, i18n.rs (en / pt-BR / es)
```

Every probe returns report sections. The GUI, the TXT/HTML/JSON writers and `-console` all read that same tree, so the app and the report always agree.

## Contributing

Issues and PRs are welcome, especially:
- **New mainboards:** use the [board sensors issue template](.github/ISSUE_TEMPLATE/board-sensors.yml) with `CPUZEEZ_EC_DUMP=1 cpuzeez -txt=board` output.
- **CPUs and GPUs missing from the tables** (`src/cpu/db.rs`, `src/gpu/db.rs`): cite a public source.
- **Reports from real macOS hardware and from Intel systems**, especially for the Intel memory-timing decoders.
- **Translations** (`PT_BR` / `ES` tables at the end of each `src/gui/*.rs` file; adding a language is one table per file).

Please keep the clean-room rule: **don't contribute anything derived from CPU-Z's binary**. Work from public documentation.

Thanks to [@vinicius741](https://github.com/vinicius741) for the macOS signing and Apple Silicon memory fixes ([#1](https://github.com/joaoregis/cpuzeez/pull/1), [#2](https://github.com/joaoregis/cpuzeez/pull/2)).

## License

Licensed under either of [Apache License 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option.

PawnIO and its modules are separate projects under their own licenses. They are not bundled.
