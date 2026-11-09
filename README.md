<div align="center">

<img src="assets/wordmark.png" width="440" alt="CPU ZeeZ">

**A clean-room, open-source, cross-platform reimplementation of CPU-Z's feature set, plus a hardware monitor, written in Rust.**

Processor, caches, mainboard, memory and SPD, graphics, sensors, drives, network and batteries, benchmarks and full TXT/HTML/JSON reports, on **Windows, Linux (Debian, Ubuntu, Arch, …) and macOS (Intel and Apple Silicon)**.

[![CI](https://github.com/joaoregis/cpuzeez/actions/workflows/ci.yml/badge.svg)](https://github.com/joaoregis/cpuzeez/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/joaoregis/cpuzeez)](https://github.com/joaoregis/cpuzeez/releases)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)

[Português](README.pt-BR.md) · [Download](#install) · [Screenshots](#screenshots) · [Changelog](CHANGELOG.md) · [Feature coverage](#feature-coverage) · [Clean-room process](#clean-room-process)

</div>

---

> CPU ZeeZ (`cpuzeez`) is **not affiliated with CPUID**. CPU-Z is a trademark of CPUID. No CPU-Z code, data tables or text were used. See [Clean-room process](#clean-room-process).

## Contents

- [Highlights](#highlights)
- [Screenshots](#screenshots)
- [Install](#install)
- [Updates](#updates)
- [Privileged hardware access](#privileged-hardware-access)
- [Quality, accessibility and security](#quality-accessibility-and-security)
- [Performance budget](#performance-budget)
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
  - Sidebar navigation and a live **Overview**: a one-line summary of the build, health pills, tiles you choose (with presets), quick actions and opt-in local alerts.
  - **Build health:** common setup mistakes and how to fix them (single memory module, XMP/EXPO off, GPU on fewer lanes, Resizable BAR off, power-saving plan, monitor below its refresh rate or plugged into the motherboard).
  - A **command palette** (`Ctrl+K`) that finds any field, page or action; **Basic / Advanced / Expert** detail levels; copy or **pin** any row to the Overview; units of your choice (°C/°F, MHz/GHz, MB/MiB).
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
  - **Tray icons**, one per metric you pick, as a number, bar, mini graph or ring.
  - **Mini monitors:** as many small always-on-top windows as you like, each with its own layout (list, bar, grid, one big number or your own), lockable (click-through), hidden while a full-screen game runs.
  - Sensor sources:
    - **CPU:** AMD SMU per-core temperatures, rails and PPT/TDC/EDC; Intel and AMD RAPL; per-core effective clocks.
    - **GPU:** temperature, hot spot, memory junction, fans, power, load.
    - **Mainboard:** Super I/O voltages, temperatures and fans, with maps for more than 150 board models.
    - **Chipset, DIMMs and drives.**
- **ZeeZ Live: an overlay, FPS and capture, without touching the game.**
  - **Overlay:** a transparent, always-on-top, click-through window of its own. Nothing is injected into the game. Five styles, nine places plus an offset (or drag it), on the monitor the game is on, with a real outline on the text. **Throttling pills** name what throttles (CPU or GPU, thermal or power).
  - **FPS:** PresentMon on Windows (bundled; needs administrator), MangoHud on Linux, Metal HUD on macOS (experimental). FPS, frame time, 1% and 0.1% lows, GPU Busy.
  - **Capture** a session by hand, on a timer, from a hotkey or per game; it is saved as `frames.csv` (PresentMon's columns, so CapFrameX opens it), `hw.csv` and `session.json`. **Reports** with charts, comparison between sessions and HTML export.
  - **Global hotkeys** (on Wayland through the desktop's shortcuts portal), per-game profiles, and an **anti-cheat safe mode** that stops all driver reads while a chosen game runs.
- **What limits your frame rate, measured.** The bottleneck detector says *GPU at its limit*, *CPU at its limit (core 4)*, *held back on purpose* (V-Sync, a cap), *thermal throttling*, *video memory full*, *paging*, *disk* or *another program*, with the numbers behind it, live and over a whole session. It never ranks parts and never guesses.
- **A Task Manager under *System*.** Processes (grouped, with network, power, GPU, command line, integrity and more), Performance, **App history** (opt-in, kept locally), Startup with its boot timeline, Users and Services (recovery, triggers, accounts). Ending or changing anything asks first and warns when the target is critical to the system; a change that needs rights asks for them for that one action.
- **Benchmarks** (our own workloads; 1000 points = the reference Ryzen 9 7900X / RTX 4060):
  - **CPU suite:** compression, n-body, SIMD matrix math, AES with and without the AES instructions, SHA-256, a ray tracer and sorting, single- and multi-threaded; thread scaling, P- vs E-cores, and a sustained run that shows throttling.
  - **Memory:** latency and bandwidth per cache level, STREAM triad, GUPS, the cost of TLB misses, latency per CCD.
  - **GPU** (DX12/Vulkan/Metal): FP32/FP16, VRAM bandwidth, the PCIe link both ways, a raster scene.
  - **Storage:** sequential and 4K random, uncached, plus a sustained write that finds the SSD's cache size.
  - **Core-to-core latency** heat map, and a **stability test** (CPU light/heavy/mixed, a memory test, GPU stress, or all at once) that stops itself at temperature limits.
  - Every result records its conditions and can be saved to a local **history** and compared sub-test by sub-test.
- **Devices.**
  - **Drives:** full **SMART**, SMART self-tests, a read-only surface scan, written data against the rated endurance, partition alignment, TRIM, encryption, and the decoded Identify data.
  - **Displays** (refresh rate, HDR, refresh range), **USB** as a tree with negotiated speed, **Thunderbolt/USB4**, **audio** devices with their format, **Bluetooth** battery levels.
  - **Battery health** (wear, cycles) and **network adapters** (link, IPs, DNS, driver, Wi-Fi).
- **Tools.**
  - **Compare reports**: what changed after a hardware or BIOS change.
  - Live clocks of every core and GPU, timers, and TXT/HTML/JSON export.
- **CPU-Z's command line**:
  - CPU-Z's own flags: `-txt=`, `-html=`, `-json=`, `-console`, `-bench`, `-core=`, `-coremask=`.
  - New flags: `-sensors`, `-log=`, `-membench`, `-diff=`, `-checkupdate[=stable|snapshot|beta|alpha]`.
  - A `cpuzeez.ini` with the same keys as `cpuz.ini`.
- **One-click updates**, on the channel you choose (Stable, Snapshot, Beta or Alpha). The app finds new releases and installs the right package for your OS, after verifying its SHA-256.
- **A single native binary**, about 20 MB (most of it the GPU renderer and the GPU benchmark), with no runtime dependencies.

## Screenshots

Captured with `cpuzeez -screenshots=DIR`, which renders every page and saves a PNG. The Windows set comes from an AMD Ryzen 9 7900X, MSI PRO X870-P WIFI, 2×16 GB DDR5-6000 and an RTX 4060, running elevated with PawnIO. Serial numbers, MACs and IPs are masked in this mode.

### Windows 11

| System (Task Manager) | What limits my FPS? | Benchmarks |
|---|---|---|
| ![System: performance](docs/screenshots/windows/system-performance.png) | ![The bottleneck detector](docs/screenshots/windows/bottleneck.png) | ![Benchmarks](docs/screenshots/windows/bench.png) |

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
| **System (Task Manager)** | **What limits my FPS?** | **Processor** |
| ![Linux System: performance](docs/screenshots/linux/system-performance.png) | ![Linux bottleneck detector](docs/screenshots/linux/bottleneck.png) | ![Linux Processor](docs/screenshots/linux/processor.png) |

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

### Why Windows or macOS shows a security warning

**CPU ZeeZ is not signed with a publisher certificate.** On Windows, removing the warning needs a code-signing certificate. On macOS it needs a paid Apple Developer membership (US$ 99/year) for notarization. Both cost money every year, and CPU ZeeZ is a free project with no budget for that yet. macOS executables are ad-hoc signed, which keeps them runnable but neither identifies the developer nor counts as notarization.

**What the warning means.** Windows and macOS can't tell who published the file. **It doesn't mean malware was found.** SmartScreen is reputation-based and warns about unsigned downloads that few people have run yet.

**How to check that your download is genuine:**
1. Every release ships `SHA256SUMS.txt`. Compare your file's hash with the one listed:
   - Windows (PowerShell): `Get-FileHash .\cpuzeez-<ver>-windows-x64.msi -Algorithm SHA256`
   - macOS: `shasum -a 256 cpuzeez-<ver>-macos-universal.dmg`
   - Linux: `sha256sum cpuzeez_<ver>_amd64.deb`
2. Every release is built by the public [release workflow](.github/workflows/release.yml) on GitHub's servers, from the tagged commit of this open-source code. The build logs are public.
3. The in-app updater checks the SHA-256 for you before it installs anything ([Updates](#updates)).

**How to continue:**
- **Windows:** when SmartScreen says "Windows protected your PC", click *More info → Run anyway*.
- **macOS:** Gatekeeper may block the app or the `.pkg` because Apple cannot verify the developer.
  1. Check your download against `SHA256SUMS.txt` (above).
  2. Try to open it once and dismiss the alert.
  3. Go to *System Settings → Privacy & Security → Open Anyway* and confirm *Open*.

  For the `.pkg`, finish installing and repeat for the app if prompted. On macOS 11–12 this setting is under *System Preferences → Security & Privacy → General*. Right-click → *Open* no longer bypasses this check on macOS 15 and later; see [Apple's instructions](https://support.apple.com/en-us/102445).
- **Linux:** packages installed with `apt`, `pacman` or from the tarball show no such warning.

With Rust installed you can also run `cargo install --git https://github.com/joaoregis/cpuzeez`.

## Updates

**Update notice.** At startup CPU ZeeZ asks GitHub's releases API whether a newer version exists. That is one anonymous `GET`; nothing about your machine is sent.
- If there is one, a bar offers **Download & install**. It fetches the package for your OS and CPU, verifies its SHA-256 against the release's `SHA256SUMS.txt`, then launches it:
  - **Windows:** the MSI.
  - **macOS:** the DMG or PKG.
  - **Debian/Ubuntu and Arch:** `pkexec` with `apt` or `pacman`.
  - **Anywhere else:** it opens the download folder.
- **What's new** opens the release notes.
- You can also check from *About ▸ Check for updates*, or with `cpuzeez -checkupdate`.
- **Update channels.** *Settings ▸ Updates ▸ Update channel* picks what the check may offer:
  - **Stable:** releases only (the default for a release build).
  - **Snapshot:** also snapshots, previews with every new feature (the default when you run a snapshot).
  - **Beta:** also betas, pieces of the new features while they are built (the default when you run a beta).
  - **Alpha:** everything that is published, including unstable code.
  The startup check follows the channel you chose, once you have allowed it. `cpuzeez -checkupdate=beta` shows what a channel would offer without changing anything. Pre-releases can be unfinished; a *Beta report* issue template is there for what you find.
- To turn the check off, set `CheckUpdates=0` in `cpuzeez.ini`.
- It uses the system `curl`, which ships with Windows 10+, macOS and desktop Linux distros.

**Upgrading by hand** means installing the new version over the old one:

| Platform | How |
|---|---|
| Windows | Run the new `.msi`. It upgrades in place: same folder, a single entry in *Installed apps*, PATH and shortcut kept. Installing an older version over a newer one is refused. CI checks install → upgrade → downgrade block → uninstall on every push. |
| macOS | Drag the new `CpuZeeZ.app` over the old one, or run the new `.pkg` |
| Debian / Ubuntu | `sudo apt install ./cpuzeez_<ver>_amd64.deb` |
| Arch | `sudo pacman -U cpuzeez-bin-<ver>-1-x86_64.pkg.tar.zst` |

## Uninstalling

CPU ZeeZ has no uninstaller program of its own: it is removed the way each platform removes things. CI checks the Windows removal on every push (the folder, the Start menu shortcut and the PATH entry go; your settings stay).

| Platform | How |
|---|---|
| Windows (MSI) | *Settings ▸ Apps ▸ Installed apps ▸ CPU ZeeZ ▸ Uninstall*, or `msiexec /x` on the `.msi`. The PawnIO driver is **kept**: other programs (LibreHardwareMonitor, FanControl) share it. Remove it from *Installed apps* too if nothing else uses it. |
| Windows (portable zip) | Delete the folder. |
| Debian / Ubuntu | `sudo apt remove cpuzeez` |
| Arch | `sudo pacman -R cpuzeez-bin` |
| macOS | Drag `CpuZeeZ.app` to the Trash (and `/usr/local/bin/cpuzeez` if you used the `.pkg`). |

**Your data** (settings, sessions, history, logs) is never deleted by an uninstall. It lives in `%APPDATA%\CpuZeeZ`, `~/Library/Application Support/CpuZeeZ` or `$XDG_CONFIG_HOME/cpuzeez`; delete it by hand, or first use *Settings ▸ Privacy & data ▸ Delete everything*.

## Privileged hardware access

Like CPU-Z, some information can only come from hardware registers: MSRs, the SMBus where the SPD EEPROMs live, the Super I/O sensor chip, the memory controller and the AMD SMU. CPU-Z installs its own kernel driver for this. **CPU ZeeZ has no driver of its own.** On Windows it uses [PawnIO](#windows-pawnio), a signed open-source driver that the packages include as an optional component; elsewhere it uses what the OS already offers. Whatever it can't read, it simply leaves out.

| | Without privileges | With privileges |
|---|---|---|
| **Windows** | CPU, caches, topology, DMI, BIOS, PCI list and links, chipset names, memory type/size/channels, NVMe SMART, network, batteries, GPUs (NVML/NVAPI/ADL) with temperatures and clocks, monitors, OS, timers, benchmarks | **+ PawnIO** (run as administrator): real core clocks (APERF/MPERF), P-states, preferred cores, SPD, memory timings, Super I/O voltages/temperatures/fans, CPU temperatures and power, AMD SMU, Intel PCH temperature, DIMM temperatures, ATA SMART thresholds |
| **Linux** | All of the above, plus hwmon sensors (`k10temp`, `coretemp`, `nct6775`, `it87`, `amdgpu`…), cpufreq clocks, RAPL power, and SPD when `ee1004`/`spd5118` are bound | **+ root**: MSRs (`modprobe msr`), full PCI config space, SPD over `i2c-dev`, Super I/O via `/dev/port`, AMD SMN/UMC, Intel MCHBAR, SMART via ioctls, and the SMU table via the optional `ryzen_smu` module |
| **macOS** | CPU (sysctl / CPUID), caches, clusters, SMBIOS (Intel) or IODeviceTree (Apple Silicon), memory size and type, PCI (IOKit), GPU and Metal, displays (EDID), storage, USB, network, SMC temperatures/fans/power, battery health, benchmarks | Nothing more: macOS doesn't allow third-party register access, and Apple Silicon has no SPD, MSRs or Super I/O |

### What else needs administrator or root

Most of CPU ZeeZ runs as a normal user. These are the exceptions, and each screen says so when it applies:

| Feature | Windows | Linux | macOS |
|---|---|---|---|
| **FPS and GPU Busy** (ZeeZ Live) | administrator (PresentMon reads the graphics kernel's events); the Live tab offers the restart | MangoHud, no root | Metal HUD, no root; no frame times from other apps |
| **Network per process** | administrator | TCP only; other users' processes need root | `nettop`, no root |
| **Ending, suspending or changing a process or service** | one UAC prompt for that action | `pkexec` for that action | authorization for that action |
| **Startup entries, service recovery and accounts** | one UAC prompt for that action | `pkexec`; recovery is a systemd drop-in you can delete | authorization for that action |
| **Hardware registers** (see above) | PawnIO + administrator | root | not possible |

An action that needs rights runs in a one-shot elevated copy of CPU ZeeZ that accepts only a fixed list of operations ([threat model](docs/security/threat-model.md)). Everything it does is written to the local audit log.

### Windows: PawnIO

[PawnIO](https://pawnio.eu) is a signed, open-source, scriptable kernel driver, also used by LibreHardwareMonitor and FanControl.

The Windows packages ship it, pinned and SHA-256-checked ([`packaging/pins.txt`](packaging/pins.txt)):

1. Install it:
   - **MSI:** *Hardware access (PawnIO driver)* is ticked by default under *Custom Setup*. Untick it to skip the driver. For a silent install without it, use `msiexec /i cpuzeez-….msi ADDLOCAL=Main`.
   - **In the app:** press *Install hardware access* in the yellow banner. CPU ZeeZ checks the bundled installer against the pinned hash, then Windows asks for permission.
   - **Yourself:** `winget install namazso.PawnIO`, or the installer from [pawnio.eu](https://pawnio.eu).
2. The official modules ([PawnIO.Modules](https://github.com/namazso/PawnIO.Modules/releases), LGPL-2.1) come in the `pawnio-modules` folder next to `cpuzeez.exe`. For a build from source, extract them there yourself.
3. Run CPU ZeeZ **as administrator**. *About* (and the report's *Binaries* section) then shows `Hardware access: PawnIO 2.2.0 (modules: …)`.

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

## Quality, accessibility and security

- **Tested without a person.** Parsers and the detector run on random and corrupt input on every push, and 11 fuzz targets run on a schedule (`fuzz/`). A synthetic game drives a long **soak** run that checks memory, handles and threads stay flat ([how](docs/testing/soak.md)). Installers are checked in CI: install, upgrade from every published release, downgrade refused, optional driver, clean removal.
- **Accessible.** Text reaches WCAG AA contrast in both themes, chart colours reach 3:1, nothing relies on colour alone (warnings carry an icon), pages hold together from 75% to 200% scale, tables and tabs work from the keyboard, and painted widgets and charts expose names and values to assistive technology. What is not covered yet is listed in [docs/accessibility.md](docs/accessibility.md).
- **Security.** [Threat model](docs/security/threat-model.md), [SECURITY.md](SECURITY.md) to report a problem, actions pinned by commit, read-only workflow tokens, `cargo-deny` and Dependabot. Releases are unsigned for now ([ADR 0009](docs/adr/0009-releases-stay-unsigned-for-now.md)); each comes with `SHA256SUMS.txt`.
- **An audit guide** for anyone reviewing the project: [docs/audit/](docs/audit/README.md).

## Performance budget

CPU ZeeZ runs next to your game, so it is held to a budget: at most 2% of one core for the sampler at 1 s with the overlay at 4 Hz, at most 150 MB of memory, and no frame-time spikes caused by sampling. Measured numbers and how they are measured are in [docs/performance.md](docs/performance.md).

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
| `-checkupdate[=channel]` | Checks GitHub for a newer release (`stable`, `snapshot`, `beta` or `alpha`; without a value, the channel in Settings) |
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
- language, theme, units, tray icons, close to tray, mini monitors and their layouts;
- sensor sampling period, graph history, warning temperature, local alerts.

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
| CPU suite, memory, storage and core-to-core latency benchmarks, history, stability test | ✅ | ✅ | ✅ (CI; latency as a statistical spread) |
| GPU benchmark (wgpu) | ✅ (DX12, Vulkan) | ☑️ (Vulkan) | ☑️ (Metal) |
| Drive health: SMART self-tests, surface scan, endurance | ✅🔑 | ☑️🔑 | ❌ self-tests · ☑️🔑 scan |
| Displays HDR / refresh range, audio formats, Thunderbolt/USB4, Bluetooth battery | ✅ | ☑️ | ☑️ |
| Reports (TXT/HTML/JSON), compare reports, F5/F6/F7 | ✅ | ✅ | ✅ (CI) |

¹ No PawnIO module offers generic PCI configuration access on Windows. Device IDs, names and links still come from the OS.

### Deliberately not included

- **Online validation** (valid.x86.fr, Validator Statistics). It is CPUID's service, so CPU ZeeZ doesn't submit to it. Share the HTML/JSON report instead.
- **CPU-Z's benchmark algorithm and reference scores.** They are CPUID's work. CPU ZeeZ's benchmarks are its own, and their scores are not comparable with CPU-Z's.
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

CPU ZeeZ was built so that no expression of CPU-Z (its code, tables or text) could end up in this project:

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

A Cargo workspace ([ADR 0011](docs/adr/README.md)):

```
crates/core/       pure logic, no OS calls, unit-tested everywhere:
                   report tree, Metric Engine, Widget Engine layouts (overview, tray, mini monitors),
                   SPD/EDID/SMART/Identify decoders, build health, alerts, units, detail levels
crates/platform/   per-OS adapters: CPUID, privileged access (PawnIO / Linux kernel interfaces),
                   disk/network/memory/GPU-engine counters, displays, firmware, partitions,
                   C-states, uncached I/O, large pages, encryption
src/               the app:
├── main.rs        CLI (CPU-Z options + -sensors/-log/-membench/-diff), Ctx, section order
├── cpu/  gpu/  pci/  spd/  sensors/  chipset.rs  smbios.rs  storage.rs  usb.rs  …   probes
├── telemetry.rs   sampler on the Metric Engine: series, CSV, throttling detection
├── benchkit.rs    benchmark principles (conditions, N runs, 1000-point score), background jobs
├── bench.rs  cpusuite.rs  membench.rs  memplus.rs  gpubench.rs  diskbench.rs  latency.rs
├── memtest.rs  drivetest.rs  history.rs  diff.rs  update.rs  executor.rs (audited actions)
└── gui/           egui front-end: pages, benchmark views, tray, mini monitors, layout editor,
                   command palette, i18n (en / pt-BR / es)
```

Every probe returns report sections. The GUI, the TXT/HTML/JSON writers and `-console` all read that same tree, so the app and the report always agree. The 1.0 plan and its roadmap are in [docs/v1/](docs/v1/README.md).

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

PawnIO and its modules are separate projects under their own licenses. The Windows packages redistribute them unmodified; see [THIRD_PARTY.md](THIRD_PARTY.md).
