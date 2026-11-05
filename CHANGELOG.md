# Changelog

All notable changes to CpuZeeZ are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/). Each release's notes on GitHub are
generated from its section below.

## [Unreleased]

### Added

- **1.0 plan** in [`docs/v1/`](docs/v1/README.md). Nothing is implemented yet. It covers:
  - Principles: local-first data, opt-in recording, transparent privileges, no injection, zero install friction.
  - A diagnosis of 0.3.0.
  - The target architecture, engineering practices and per-stage quality gates.
  - A dependency and bundling policy.
  - A permissions map and the roadmap (one 1.0.0 release, built in four internal stages).
  - Designs for **ZeeZ Live** (overlay, capture, reports), the bottleneck detector and the cross-platform System hub (Task Manager).
  - Improvements to the existing pages, the tray icon and the mini monitor.
  - A detailed roadmap ([`docs/v1/ROADMAP.md`](docs/v1/ROADMAP.md)) listing every fix, improvement and feature by stage, with IDs and acceptance criteria.
- Static-analysis evidence scripts for the Task Manager design (`docs/v1/evidence/`).
- **PawnIO ships with the Windows packages** (S0-P01–P03). The installer (2.2.0) and the official modules (0.2.11) are pinned with their SHA-256 in `packaging/pins.txt` and checked when a release is built.
  - The MSI's *Hardware access* feature installs the driver. It's ticked by default, explained in *Custom Setup*, and can be unticked. Portable users get *Install hardware access* in the app's banner. The app re-checks the installer's hash before running it, and logs the action in the local audit log.
  - The modules are installed next to the app, so nothing is left to download by hand.
  - [THIRD_PARTY.md](THIRD_PARTY.md) and *About › Third-party components* list what is bundled and under which license.
- **The GUI opens on machines without a working OpenGL driver**, such as VMs, Remote Desktop and missing GPU drivers. If OpenGL fails, CpuZeeZ restarts its window on wgpu (DX12/WARP, Vulkan or Metal). `CZ_RENDERER=wgpu` forces it ([ADR 0013](docs/adr/0013-gui-renders-with-glow-and-falls-back-to-wgpu.md)).
- **Per-disk activity** in the Sensors list and CSV logs: read/write speed, IOPS, latency, active time and queue for every physical disk, on Windows, Linux and macOS, without administrator rights.
- **Per-adapter network traffic** the same way: download/upload in Mbps, packets per second, errors and drops.
- **GPU engine load**: 3D, copy, video encode and decode, compute and the rest per GPU on Windows (as the Task Manager counts them); the busy figure on Linux (AMD) and Device/Renderer/Tiler on macOS.
- **Storage benchmark** (*Benchmarks › Storage*): sequential and 4K random read/write with 1 and 8 requests in flight, bypassing the OS cache, on a temporary file that is always deleted.
- **CPU suite** (*Benchmarks › Processor*): eight workloads (compression, n-body physics, SIMD matrix math, AES with and without the CPU's AES instructions, SHA-256, a ray tracer and sorting), each on one core and on every thread, scored so that 1000 = the reference Ryzen 9 7900X. Also: thread scaling from 1 thread to all, P-core vs E-core on hybrid CPUs, and a sustained run (1, 10 or 30 minutes) that shows how much the score drops when the CPU throttles.
- **More memory tests** (*Benchmarks › Memory*): STREAM triad bandwidth, GUPS random updates, a random walk on 4 KiB and on large pages (the cost of TLB misses) and DRAM latency from each CCD or NUMA node. Large pages on Windows need the *Lock pages in memory* right; the test says so when it's missing.
- **Benchmarks are reproducible**: each result records the app version, power plan and starting CPU temperature; the suite can run 3 or 5 times and report the median and the spread.
- **History for every benchmark**: the CPU suite, storage and core-to-core latency results can be saved too, and comparing two results lists every sub-test side by side.
- **Core-to-core latency** (*Benchmarks › Latency*): a heat map of how long each core takes to see another core's writes, which shows the CCD/CCX layout at a glance.
- **Devices page**: USB devices with the speed they run at and the USB version they support (with a warning when a USB 3 device runs at USB 2 speed), and each display's refresh rate against what it supports.
- **Drives**: partition alignment, the OS's TRIM setting and NVMe thermal throttling (how often and for how long) on each drive's card.
- **Memory page**: which profile is running (XMP/EXPO, or "tuned tRAS"), secondary and tertiary timings on AMD (tRRD, tFAW, tWR, tRTP, tWTR, tCWL, tREFI, tRFC2), the FCLK, a map of which slot holds which module, ECC, module temperatures, a bar of what the OS uses RAM for, and the theoretical bandwidth next to a link to measure it.
- **Mainboard page**: firmware (UEFI or legacy boot, Secure Boot, TPM), every PCI Express device's link next to its maximum (with a warning when a device runs narrower), mainboard sensors by name, slots and ports, a browsable SMBIOS tree and, at Expert level, the ACPI tables.
- **Processor page**: a live per-core table (clock, temperature, load, CCD/CCX), a topology diagram, power limits, speculative-execution vulnerabilities and their mitigations, virtualization (VT-x/AMD-V, VBS, Memory integrity, KVM), the power plan or governor, and instruction sets grouped by what they do.
- **A new Overview**: a one-line, clickable summary of the build; health pills (build checks, drive health, battery wear); tiles you choose (presets *Gamer*, *Overclocker*, *Laptop*, *Workstation*) that open the Sensors page on their metric; quick actions; *Customize* to add, order and remove tiles.
- **Local alerts** (opt-in, *Settings › Alerts*): rules such as "CPU above 90 °C for 30 s" show on the Overview and in the tray tooltip. They're checked on this computer and never sent anywhere.
- **Build health**: CpuZeeZ spots common setup mistakes and says how to fix them: a single memory module, XMP/EXPO not enabled, the graphics card on fewer PCIe lanes than it supports, the power-saving plan, a monitor below its refresh rate, or the monitor plugged into the motherboard while a graphics card is installed.
- **Sensors page on a real table**: sort any column, filter with the search syntax (`cur>80`, `unit:°`, `id:disk`), one-click presets (temperatures, clocks, power…), select several rows to plot, pin or copy them, and *Export view* to CSV.
- **Command palette** (`Ctrl+K`): search any value CpuZeeZ reads ("tRFC", "PCIe", "BIOS"), any page or an action such as saving the report, then press Enter to go there.
- **Copy and pin**: right-click any row to copy its value, the row or the whole card as text, or pin it to the Overview. Sensors pinned there stay live.
- **Detail levels** on the Processor, Mainboard, Memory and Graphics pages: *Basic* (the essentials), *Advanced* (everything, as before) and *Expert* (plus raw CPUID leaves and MSRs). Each page remembers its choice.
- **Units** in *Settings › Units*: °C or °F, MHz or GHz, MB as Windows counts, MiB/GiB or decimal MB, and a fixed number of decimals. Only the display changes; logs and reports keep °C, MHz and MiB.
- **What memory is used for**: standby/cached, modified, compressed, kernel pools (wired), commit charge and page file or swap use, each where the OS reports it.

### Changed

- README (English and pt-BR): the install section now explains why the builds are unsigned (a paid certificate the project can't fund yet), what the SmartScreen/Gatekeeper warning means, and how to verify a download with `SHA256SUMS.txt`.

### Fixed

- Tests that switched the interface language could change the labels other tests were clicking (now per test thread).
- On Windows and Linux, Super I/O readings labelled "CPU …" were filed under the CPU and missing from leases that asked for the mainboard; they stay with the mainboard now (macOS's SMC still files them under the CPU).
- With PawnIO installed but its modules missing, the banner said "PawnIO has no module for this CPU". It now says the `pawnio-modules` folder is missing.

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

[Unreleased]: https://github.com/joaoregis/cpuzeez/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/joaoregis/cpuzeez/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/joaoregis/cpuzeez/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/joaoregis/cpuzeez/releases/tag/v0.1.0
