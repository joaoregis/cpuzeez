# Changelog

All notable changes to CpuZeeZ are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/). Each release's notes on GitHub are
generated from its section below.

## [0.3.0] - 2026-10-14

A redesigned interface, a hardware monitor, memory and cache benchmarks,
device health, and closing most of the remaining gaps with CPU-Z.

### Added

- **New interface.**
  - Sidebar navigation with full pages: Overview, Processor, Mainboard, Memory, Graphics, Devices, Sensors, Benchmarks, Tools, Settings and About.
  - Card-based layout and Phosphor icons.
  - The operating system's own UI font (Segoe UI, SF Pro, Cantarell, Noto, Inter…).
  - Light and dark themes with the brand violet accent.
- **Overview dashboard.** The system at a glance: CPU, GPU and chipset temperatures and the CPU clock with live sparklines, a health and throttling status, and one card per component.
- **Translations.** The interface is available in English, Brazilian Portuguese and Spanish, picked from the OS language or chosen in Settings. Reports stay in English, like CPU-Z's.
- **Settings page.** Language, theme, tray icon, close to tray, sampling period, graph history, warning temperature and data folder. Settings are saved per user.
- **Sensors page.** An HWiNFO-style monitor:
  - Every sensor, grouped by device (CPU, GPU, mainboard, chipset, memory, storage, battery), with current, minimum, maximum and average values and search.
  - Live charts with hover values.
  - Temperature colouring against Tjmax.
  - CSV logging.
- **Monitored stress test.** Live temperature, effective clock, package power and GPU temperature. It warns when thermal, power or current throttling is detected and stops automatically at a temperature limit.
- **Throttling detection.**
  - Intel: thermal status and limit-reason MSRs.
  - AMD: Tjmax and PPT/TDC/EDC limits.
  - Linux: the kernel's throttle counters.
  - GPUs: their throttle reasons.
- **Tray / menu-bar mini monitor.** The tray icon shows the CPU temperature. There is also an always-on-top mini monitor window on every OS.
- **Memory and cache benchmark.**
  - Latency from 4 KiB to 1 GiB, which reveals L1, L2, L3 and DRAM.
  - Read, write and copy bandwidth per cache level and for DRAM, single- and multi-threaded (`-membench`).
- **Benchmark history.** CPU and memory results are saved locally with a label, with charts over time and comparison of two runs.
- **Compare reports.** A diff between two JSON reports, or between the current system and a saved report, that ignores live values unless asked (`-diff=OLD.json,NEW.json|now`).
- **Devices page.**
  - Full SMART for NVMe and SATA: health, attributes, temperature, wear, hours and data written.
  - Battery health: design and full capacity, wear and cycles.
  - Network adapters: link, MAC, IPs, gateway, DNS, driver and Wi-Fi details.
- **AMD SMU decoding**, Zen 2 to Zen 5:
  - Per-core, L3 and SoC temperatures.
  - VDDCR_VDD, VDDCR_SOC and VDD_MISC rails.
  - PPT, TDC and EDC.
  - FCLK, UCLK and MCLK, Fmax, and per-core max effective ratio.
- **Intel memory timings** from the memory controller, Skylake to Arrow Lake: CL, tRCD, tRP, tRAS, tRFC, command rate, gear and the memory controller clock.
- **Mainboard sensor maps** for more than 150 ASUS, Gigabyte, MSI and ASRock board models, plus generic maps per Super I/O chip. The `CPUZEEZ_EC_DUMP` helper and an issue template make it easy to contribute new boards.
- **Chipset temperature.**
  - Intel: the PCH sensor (PawnIO `IntelPCHThermal`, Linux `intel_pch_thermal`).
  - AMD boards: when the board routes it to the Super I/O.
- **GPU sensors.**
  - Hot spot and memory junction temperatures, fan RPM and duty, power, load and throttle reasons.
  - NVIDIA memory vendor, via NVAPI.
- **Uncore and fabric clocks** (AMD FCLK/UCLK, Intel ring), the memory controller frequency and the PCIe "Bus Specification" on Windows.
- **One-click update.** The update banner downloads the right installer for your OS and CPU and verifies its SHA-256 against the release's `SHA256SUMS.txt` before launching it.
- **New command-line options:** `-sensors`, `-log=FILE[,SECONDS]`, `-membench`, `-diff=…`.

### Changed

- The reference clock is measured from the TSC and the base ratio, so memory and uncore frequencies follow the real BCLK.
- ITE Super I/O voltage steps match the published board tables.
- Reports gained Batteries and Network sections, SMART groups under Storage, and Sensors groups under each display adapter.

### Fixed

- **macOS**, thanks to [@vinicius741](https://github.com/vinicius741):
  - The standalone universal CLI (in the `.pkg` and `.tar.gz`) is now ad-hoc signed for both architectures, and the release workflow verifies every shipped signature. Updated Gatekeeper instructions for macOS 15+ ([#1](https://github.com/joaoregis/cpuzeez/pull/1)).
  - Memory capacity and type on Apple Silicon, which has no SMBIOS memory records ([#2](https://github.com/joaoregis/cpuzeez/pull/2)).
- Links and the "Download" buttons now open the browser.
- Overview cards no longer show empty separators when a value is unknown.

## [0.2.0] - 2026-10-14

### Added

- New name and logo: **CpuZeeZ**. The icon is used for the executable (with version info), the window, the Start-menu shortcut, *Installed apps*, the macOS `.app` and the Linux desktop entry.
- Update notice: the app checks GitHub releases at startup (`CheckUpdates` in `cpuzeez.ini`, `-checkupdate`).
- CI tests MSI install, in-place upgrade, downgrade blocking and uninstall.

### Fixed

- Windows: no console window behind the GUI when it is started from Explorer or the Start menu.

## [0.1.0] - 2026-10-14

First release: a clean-room, cross-platform reimplementation of CPU-Z's
features.

- CPU (CPUID topology, caches, instruction sets, P-states, clocks, names database), SMBIOS/DMI, BIOS, chipset and AMD memory timings.
- SPD decoders from DDR to DDR5 with XMP, EXPO and EPP.
- Super I/O and CPU sensors; PCI devices with capability decoding.
- Storage, USB, GPUs, monitors (EDID), NPUs and timers.
- Benchmark and stress test.
- TXT, HTML and JSON reports with CPU-Z's command line.
- GUI with CPU-Z's tabs.
- Installers for Windows (MSI), macOS (DMG/PKG), Debian/Ubuntu (DEB) and Arch.

[0.3.0]: https://github.com/joaoregis/cpuzeez/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/joaoregis/cpuzeez/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/joaoregis/cpuzeez/releases/tag/v0.1.0
