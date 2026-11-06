# Changelog

All notable changes to CpuZeeZ are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/). Each release's notes on GitHub are
generated from its section below.

## [Unreleased]

## [1.0.0-beta.7] - 2026-11-06

The bottleneck detector: what limits your FPS, measured, with the evidence.

### Added

- **ZeeZ Live › Bottleneck** answers "what is limiting my frame rate right now?" while a game runs, and over a whole session. It measures; there is no ranking of parts and no guess.
  - **The "Now" card**: a verdict (*GPU at its limit*, *CPU at its limit (core 4)*, *Held back on purpose*, *Thermal throttling*, *Video memory is full*, *The system is paging*, *Waiting on the disk*, *Another program is competing*, or *Nothing at its limit*), how sure it is, and a plain explanation. *Why?* lists the numbers behind it.
  - **How it decides**:
    - the GPU is the limit when *GPU Busy* (the time the GPU actually worked) fills the frame;
    - the CPU is the limit when the game's heaviest thread is saturated and the GPU has room, which a low *total* CPU load can hide on many-core CPUs;
    - a limiter is told from a limit: frames locked to the refresh rate (V-Sync, or VRR at its ceiling), steady at a round cap or a Reflex-style cap, or a game that isn't in front;
    - throttling, full video memory or RAM with paging, disk reads while frames stutter, and another program using the CPU are checked next.
  - **Without GPU Busy** (Linux, macOS, or Windows without administrator) it falls back to GPU utilization and the game's thread, and says so: lower confidence and "estimated" on screen.
  - **Live readings**: GPU Busy, GPU load, CPU load, busiest core, the game's busiest thread, video memory, RAM and temperatures. Each bar has a mark where its rule fires, so you can see why.
  - **Session analysis** for 1, 3 or 5 minutes, or until you stop: share of time per cause, a second-by-second timeline, the strongest evidence, and recommendations. Each recommendation shows the measurement behind it; a session with nothing wrong gets none.
  - The same analysis appears in each **session report** (and in its HTML export), computed from `frames.csv`, `hw.csv` and the throttling spans.
- A small **"GPU ▲" / "CPU ▲" pill on the overlay** (turn it off in the Overlay tab).
- The Overview has a **What limits my FPS?** shortcut next to *Build health*, which already covers the configuration causes (memory profile off, single-channel memory, reduced PCIe link, Resizable BAR, refresh rate, power plan).
- A *Hard page faults* reading (pages read from disk to resolve faults, per second) on Windows and Linux, for the paging check.

### Changed

- While the Bottleneck tab is open, an analysis runs or the overlay shows the pill, CpuZeeZ reads the process list once a second (one kernel call) and the system counters. It does nothing otherwise.
- Captured sessions now note the display's refresh rate and the kind of each throttling span (CPU or GPU, thermal or power).

### Not included

- The "GPU limits ↔ balanced ↔ CPU limits" bar from the mockup. Calibrating it honestly needs measured reference pairs that CpuZeeZ doesn't have, and a number from internet rankings is what this feature exists to avoid. See [ADR 0014](docs/adr/0014-no-hardware-balance-bar-in-1-0.md).
- Shared (spilled) video memory isn't read yet: the video-memory rule uses the game's dedicated usage and stutters, and says so at medium confidence.
- A game on a second GPU: readings use the first adapter.

## [1.0.0-beta.6] - 2026-11-05

ZeeZ Live: the overlay, FPS, capture and reports. Test it in your games; the play-session checks are listed in the ROADMAP's S2b status.

### Added

- **ZeeZ Live**, a new sidebar entry with three tabs: *Live*, *Overlay* and *Sessions*.
  - **Overlay**: a HUD above games and apps. CpuZeeZ draws it in its own window: transparent, always on top, it never takes focus and lets every click through. Nothing is injected into the game.
    - Five styles: *Minimal*, *Compact*, *Detailed*, *Benchmark*, *Overclocker*. *Customize* opens the layout editor.
    - Nine places on the screen plus an offset, or *Move with the mouse*.
    - On Windows it can be hidden from screenshots, recordings and streams.
    - First use takes three steps: pick a style, confirm the hotkey, *Try it now*.
  - **FPS on Windows** through Intel's PresentMon 2.6.0, now in the installer and the portable zip.
    - It reads the graphics kernel's events, so nothing touches the game.
    - It needs CpuZeeZ running as administrator; the Live tab offers the restart.
    - It runs only while ZeeZ Live shows frames, and CpuZeeZ checks its SHA-256 before every start.
    - Readings: FPS, frame time, 1% and 0.1% lows, GPU Busy, and the present mode. A warning shows when a game is in true exclusive fullscreen, where no window can be drawn on top.
  - **FPS on Linux** through MangoHud:
    - CpuZeeZ detects it and installs it with your package manager after you confirm;
    - it can enable it for every game, or copy the Steam launch option `mangohud %command%`;
    - its config is written from your overlay layout, and its logs are read for FPS.
  - **macOS (experimental)**: open a game with Apple's Metal Performance HUD. macOS gives no app another app's frame times.
  - **Live** tab: the game being measured (automatic or picked from a list), FPS, lows, GPU Busy and a frame-time chart.
  - **Capture**:
    - manual, or timed (60 s by default, after a 3 s countdown), or from its hotkey;
    - markers during a capture, and throttling recorded as time spans;
    - each session is saved in `sessions/<date>_<game>/` as `frames.csv` (PresentMon's columns, so CapFrameX opens it), `hw.csv` and `session.json`;
    - **● REC** shows on the overlay and in the window title while recording.
  - **Sessions**:
    - **Report**: average, 1% and 0.1% lows (the convention is always stated) and stutters; the frame-time chart with markers and throttling; load and temperature charts; the frame-time percentile curve and an FPS histogram.
    - **Compare** several sessions, with warnings when they aren't comparable (different game, resolution or driver) and when repeats vary more than 5 % (the median is shown).
    - Export as offline HTML or PNG; import PresentMon, CapFrameX or MangoHud CSVs.
  - **Global hotkeys**: show/hide (`Ctrl+Shift+O`), next style, start/stop capture, marker, edit mode. All of them can be changed, and conflicts are shown, with each other and with other apps. On Wayland they only work while CpuZeeZ has focus.
  - **Per-game profiles**: a game's own style and place; show the overlay when it starts; capture while it runs (off by default).
  - **Anti-cheat safe mode**, per game: while it runs, nothing is read through the PawnIO driver. Temperatures and clocks that need the driver go blank; GPU and OS readings stay.
- A *Busiest core load* reading (the busiest logical CPU) for the Benchmark style and the bottleneck detector.
- *Privacy & data* lists ZeeZ Live sessions, and can delete them.

### Fixed

- Locked (click-through) mini monitors were invisible on Windows: glow draws nothing in a layered window until it gets its alpha (ADR 0013).

## [1.0.0-beta.5] - 2026-11-05

The System hub finished: what beta.3 left out, and the UI problems found testing beta.4.

### Added

- **More process columns** (right-click the header to pick them):
  - *Network*, per process. Windows needs CpuZeeZ running as administrator for it (it reads the kernel's network events); Linux counts TCP only (UDP has no per-socket counters), and other users' processes need root; macOS reads it from `nettop`.
  - *Power usage* and *Power usage trend*: an estimate from CPU, GPU, disk and network use (the OS has no per-process watts outside Windows' own estimator).
  - *Command line*, *Architecture*, *Elevated*, *Integrity* and *UAC virtualization* (the last two are Windows-only).
  - *CPU time*, *I/O reads* and *I/O writes*.
  - *GDI objects* and *USER objects* (Windows only).
- The *Essential* and *Gaming* presets now show *Network* and *Power usage*; *Security* shows elevation, integrity, architecture and the command line. The details panel's *Security* tab shows them too.
- **Restart** on Explorer's right-click menu restarts the Windows shell. **Close window** in the *Window* submenu closes the app's window as its X button would, so the app can ask to save.
- **Services**: change the log-on account to Local System, Local Service or Network Service; set what happens on the first, second and later failures (restart the service, restart the computer, nothing), the restart delay and the reset period; start a stopped service with parameters. The inspector also shows the service's triggers, its SID type and the privileges it requires. Accounts with a password are left to Windows' own Services tool: CpuZeeZ never handles passwords.
- On Linux, recovery is written as a systemd drop-in (`/etc/systemd/system/<unit>.d/50-cpuzeez-recovery.conf`); delete it to undo.
- **Tables**:
  - drag a column's edge to resize it, double-click the edge to fit the content;
  - drag a column's header to move the column;
  - widths and order are kept; the header's right-click menu resets them.
  - Tables scroll sideways when the columns are wider than the window, and the header scrolls with them.
  - Cut-off text ends in "…" and shows in full on hover.
- Processes: double-click a row, or press Enter, to open the details panel.
- Users: each user's processes are a table you can sort and filter.

### Changed

- **App history** is kept in a SQLite file (`app_history.sqlite`, next to the settings). The beta.3/beta.4 JSON file is imported once, then deleted.
- The retention setting is now applied: days older than it are deleted.
- History is recorded whenever CpuZeeZ is running and recording is on, whatever page is open. Before, it only recorded with *System* on screen.
- *Delete usage history* asks first.
- A Windows recovery action that restarts the computer is set with the shutdown privilege the service manager requires.
- Ending a process that the OS can't live without is flagged as critical by name in the elevated helper too, not only by PID.

### Fixed

- **System › Services listed nothing**: the table was laid out sideways and its rows had no height.
- **System › Performance**: the device tiles were drawn in one overlapping row and the detail pane was empty.
- **The right-click menu of every table** opened as a narrow column with one word per line.
- System › Users: the session line and the usage figures overlapped.
- App history counted an app's run time once per process: an app with 25 processes got 25 seconds per second.
- Computing a file's SHA-256 in the details panel froze the window on large executables; it runs in the background now.
- macOS: restarting, enabling and disabling a service now passes the `domain/label` target that `launchctl` requires for those commands. Not yet tried on a Mac.
- Some new System texts were missing their pt-BR and es translations. A test now fails when a text has no translation.

## [1.0.0-beta.4] - 2026-11-05

Fixes from the review of 1.0.0-beta.3: test the System hub on this one.

### Security

- **Process actions check they still have the right process.** A process ID can be reused after its process exits, and an administrator prompt can stay open for a while; ending, suspending or changing a process now first checks that the ID still belongs to the process you picked (same start time), and refuses otherwise.
- **Names in service and startup actions are checked** before anything runs, in the app and in the elevated helper. On Linux a service name could have been read by `systemctl` as an option, and a startup entry's name could point outside the autostart folder.
- **No memory dumps of anti-cheat-protected processes** (games with Vanguard, EasyAntiCheat, BattlEye, FACEIT…). *Create dump file* is greyed out for them and the helper refuses too, as the anti-cheat rule requires: CpuZeeZ never reads their memory.

### Fixed

- The process details panel and *System › Performance › Memory* read the system on the interface thread (a full process and thread snapshot every 2 s, memory counters every frame); both are read in the background now.
- *Pause* on a Linux or macOS service stopped it instead; systemd and launchd have no pause, so it now says so.
- Disabling a Windows *RunOnce* startup entry reported success but changed nothing (Windows has no switch for those); it now explains that they run once and can only be removed.

## [1.0.0-beta.3] - 2026-11-05

The **System hub**, a Task Manager for Windows, Linux and macOS. ZeeZ Live and the Bottleneck detector come in later betas.

### Added

- **System › Processes:** every process grouped as apps, background and system processes (or as one detailed list, or as a parent → child tree), with CPU, memory, disk, GPU and video-memory use, a 60-second CPU graph per row, heat-map colours, pinning, column presets (*Essential*, *Gaming*, *Leak hunting*, *Security*, *Everything*) and *Problems* filters (not responding, memory that keeps growing, high CPU for over a minute, running from a temporary folder). CpuZeeZ reads processes the way the Task Manager does, without reading their memory, so it's safe next to anti-cheats.
- **Process details panel:** threads, network connections, windows (switch to, minimize, maximize), the account, the command line and an on-demand SHA-256 of the file.
- **Process actions:** end task or the whole tree (with a strong warning for processes the system can't live without), suspend and resume, priority, CPU affinity, Efficiency mode, memory dump, *Analyze wait chain*, UAC virtualization, open file location, and *Run new task* (optionally as administrator). Every action is written to the local audit log. Export to CSV/JSON and compare two snapshots (what started, exited or grew).
- **System › Performance:** CPU (per-thread grid, speed, processes/threads/handles, up time, idle states, caches), memory (what it's used for, page file), each disk, each network adapter, each GPU (per engine, video memory, temperature, power, link) and NPUs, with 1 min / 10 min / 1 h / session graphs, a compact summary view and CSV export.
- **System › Services:** every service with its state, startup type, account, path, description, dependencies both ways (the stop confirmation lists what else stops) and recent logs; start, stop, pause/resume (Windows), restart and startup type, with filters (*Running*, *Automatic but stopped*, *Disabled*, *Third-party*, *Failed recently*), a lock on services the system protects, and *Undo*. The recovery settings are shown read-only.
- **System › Startup:** apps that start with Windows (registry and Startup folders), the desktop (XDG autostart) or macOS (LaunchAgents); enable or disable them the same reversible way the Task Manager does, with the date an item was disabled, its startup impact where Windows measured it, the firmware's last boot time, a boot timeline and *Undo*.
- **System › Users:** signed-in sessions with each user's CPU, memory, disk and GPU use and their processes; disconnect, sign out, send a message and a shortcut to the account settings.
- **System › App history** (off until you turn it on): CPU time, GPU time, disk and memory per app per day, kept on this computer for the days you choose, with what used the most. It only covers the time CpuZeeZ was running, and says so.
- **Settings › System:** start page, always on top, minimize on use, hide when minimized, full account names, Efficiency-mode confirmation and how long app history is kept.

## [1.0.0-beta.2] - 2026-11-05

A fix release for testers of 1.0.0-beta.1.

### Fixed

- **The interface felt sluggish** (reported on 1.0.0-beta.1). The Overview re-read every drive's SMART data several times per frame, about 150 ms per frame, so the whole app lagged; it now takes under 2 ms. Drives, batteries, firmware details, the OS's CPU settings and the GPU list are read on a background thread and refreshed from time to time, never while a frame is drawn, and a test keeps it that way. `CZ_FRAME_LOG=1` prints any part of a frame that takes longer than 4 ms, for reports like this one.

## [1.0.0-beta.1] - 2026-11-05

First pre-release of 1.0, for testers: the existing features, improved (stage S1 of the [roadmap](docs/v1/ROADMAP.md)). ZeeZ Live, the bottleneck detector and the System hub come in the next betas. What to try first is in [docs/testing/beta1.md](docs/testing/beta1.md).

### Added

- **1.0 plan** in [`docs/v1/`](docs/v1/README.md). It covers:
  - Principles: local-first data, opt-in recording, transparent privileges, no injection, zero install friction.
  - A diagnosis of 0.3.0.
  - The target architecture, engineering practices and per-stage quality gates.
  - A dependency and bundling policy.
  - A permissions map and the roadmap (one 1.0.0 release, built in four internal stages).
  - Designs for **ZeeZ Live** (overlay, capture, reports), the bottleneck detector and the cross-platform System hub (Task Manager).
  - Improvements to the existing pages, the tray icon and the mini monitor.
  - A detailed roadmap ([`docs/v1/ROADMAP.md`](docs/v1/ROADMAP.md)) listing every fix, improvement and feature by stage, with IDs and acceptance criteria.
- **README:** the Portuguese README is now a full translation of the English one (contents, the per-OS coverage table, `cpuzeez.ini`, CLI-only build, architecture), and both describe the 1.0 features and that the Windows packages include PawnIO as an optional component.
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
- **Mini monitors** (*Settings › Mini monitors*): as many small always-on-top windows as you like, each with its own layout (list, bar, grid, one big number, or your own), text size, background opacity, corners, outline and theme. They snap to the screen edges, remember where they were, can be locked so clicks go through them, hide while a full-screen game or video runs (Windows), show graphs while the pointer is over them, and grow with Ctrl+scroll. Right-click one to change its layout, edit it, lock it or add another. The editor picks readings by name (CPU temperature, GPU load…), so a layout works on any computer, and layouts can be copied and pasted as JSON.
- **Tray icons** (*Settings › Tray icons*): up to eight icons, one per metric you choose (CPU or GPU temperature, load, clock, power, RAM…), each as a number, bar, mini graph or ring drawn at the icon's real size with the app's font. Colours follow the thresholds and always come with a shape (a corner notch for a warning, a frame for danger). The tooltip lists every icon's value and any alert; the menu shows or hides the app, toggles the mini monitor, pauses monitoring and sets the update speed, in the current language; the left click opens the app or the mini monitor. macOS shows every metric as text in the menu bar.
- **Drive health** (*Tools › Drive health*): life left, data written against the drive's rated endurance (you type the TBW from its datasheet), temperature over time, error counters and warnings; SMART self-tests (short, extended and, on SATA, conveyance) with progress and history on Windows and Linux; and a read-only surface scan (a quick sample or the whole drive) that times every 1 MiB block and lists unreadable ones. Self-tests and the scan need administrator rights; macOS can't send SMART commands without a third-party driver.
- **Stability test** (*Benchmarks › Stress test*): CPU stress with a light, heavy (widest SIMD) or mixed load, a memory test over 1–16 GB (clearly not a MemTest86 replacement), GPU stress, or all of them at once to check the power supply and cooling. It stops itself at the CPU or GPU temperature limit, on the first memory error, or if the GPU stress fails.
- **GPU benchmark** (*Benchmarks › Graphics*): FP32 and FP16 throughput, video-memory bandwidth, upload and download over the bus (which confirms the real PCIe link) and a 1080p raster scene, on any GPU through DX12, Vulkan or Metal; 1000 points = the reference RTX 4060.
- **Sustained write** (*Benchmarks › Storage*): writes 8, 32 or 64 GB in a row and draws the speed over time, showing where a fast SSD's cache runs out. It says up front how much it writes, only offers sizes that fit in 80 % of the free space, and deletes its file.
- **Core-to-core latency** (*Benchmarks › Latency*): a heat map of how long each core takes to see another core's writes, which shows the CCD/CCX layout at a glance. On macOS, which can't pin threads, it measures two free threads many times and shows the spread.
- **Devices page**: USB devices with the speed they run at and the USB version they support (with a warning when a USB 3 device runs at USB 2 speed), and each display's refresh rate against what it supports. USB devices show as a tree under their hubs; monitors show HDR (supported / on), colour depth and their refresh range (a wide range means variable refresh); new sections list Thunderbolt/USB4 controllers and devices, Bluetooth devices' battery, and every audio input and output with the format Windows mixes it at.
- **Drives**: partition alignment, the OS's TRIM setting and NVMe thermal throttling (how often and for how long) on each drive's card. Each volume says whether it's encrypted (BitLocker, LUKS, FileVault), and at Expert level the drive's whole Identify data is shown decoded (supported commands, power states, temperature limits…) and as hex.
- **Memory page**: which profile is running (XMP/EXPO, or "tuned tRAS"), secondary and tertiary timings on AMD (tRRD, tFAW, tWR, tRTP, tWTR, tCWL, tREFI, tRFC2), the FCLK, a map of which slot holds which module, ECC, module temperatures, a bar of what the OS uses RAM for, and the theoretical bandwidth next to a link to measure it.
- **Mainboard page**: firmware (UEFI or legacy boot, Secure Boot, TPM), every PCI Express device's link next to its maximum (with a warning when a device runs narrower), mainboard sensors by name, slots and ports, a browsable SMBIOS tree and, at Expert level, the ACPI tables.
- **Processor page**: a live per-core table (clock, temperature, load, CCD/CCX), a topology diagram, power limits, speculative-execution vulnerabilities and their mitigations, virtualization (VT-x/AMD-V, VBS, Memory integrity, KVM), the power plan or governor, and instruction sets grouped by what they do. The per-core table also shows how long each core sleeps in each idle state (C-states) and which cores the firmware prefers (CPPC rank), and Intel CPUs show their PL1/PL2 power limits.
- **A new Overview**: a one-line, clickable summary of the build; health pills (build checks, drive health, battery wear); tiles you choose (presets *Gamer*, *Overclocker*, *Laptop*, *Workstation*) that open the Sensors page on their metric; quick actions; *Customize* to add, order and remove tiles.
- **Local alerts** (opt-in, *Settings › Alerts*): rules such as "CPU above 90 °C for 30 s" show on the Overview and in the tray tooltip. They're checked on this computer and never sent anywhere.
- **Build health**: CpuZeeZ spots common setup mistakes and says how to fix them: a single memory module, XMP/EXPO not enabled, the graphics card on fewer PCIe lanes than it supports, the power-saving plan, a monitor below its refresh rate, or the monitor plugged into the motherboard while a graphics card is installed. It also notices when Resizable BAR is off for the graphics card, and the Mainboard page shows Resizable BAR and Above 4G decoding.
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

[Unreleased]: https://github.com/joaoregis/cpuzeez/compare/v1.0.0-beta.7...HEAD
[1.0.0-beta.7]: https://github.com/joaoregis/cpuzeez/compare/v1.0.0-beta.6...v1.0.0-beta.7
[1.0.0-beta.6]: https://github.com/joaoregis/cpuzeez/compare/v1.0.0-beta.5...v1.0.0-beta.6
[1.0.0-beta.5]: https://github.com/joaoregis/cpuzeez/compare/v1.0.0-beta.4...v1.0.0-beta.5
[1.0.0-beta.4]: https://github.com/joaoregis/cpuzeez/compare/v1.0.0-beta.3...v1.0.0-beta.4
[1.0.0-beta.3]: https://github.com/joaoregis/cpuzeez/compare/v1.0.0-beta.2...v1.0.0-beta.3
[1.0.0-beta.2]: https://github.com/joaoregis/cpuzeez/compare/v1.0.0-beta.1...v1.0.0-beta.2
[1.0.0-beta.1]: https://github.com/joaoregis/cpuzeez/compare/v0.3.0...v1.0.0-beta.1
[0.3.0]: https://github.com/joaoregis/cpuzeez/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/joaoregis/cpuzeez/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/joaoregis/cpuzeez/releases/tag/v0.1.0
