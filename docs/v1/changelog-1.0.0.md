# CHANGELOG entry for `1.0.0`, consolidated (draft)

> To paste into [CHANGELOG.md](../../CHANGELOG.md) when `1.0.0` is tagged (S3-08), above the pre-release entries, which stay below it. It tells someone coming from `0.3.0` everything 1.0.0 brings, by area, not release by release: what is new, what improved, what was fixed and what is known not to work yet. Built from every pre-release (`1.0.0-beta.1` to `beta.10`, `1.0.0-snapshot.1`) and the [ROADMAP](ROADMAP.md).
> Lines marked ⚠ must be checked against what the snapshots and release candidates settle before tagging. Keep the CHANGELOG's style: plain words, what the user sees first, the technical detail after.

## [1.0.0] - YYYY-MM-DD

CPU ZeeZ 1.0 is the CPU-Z feature set grown into a complete tool for the PC on Windows, Linux and macOS: a hardware monitor, a Task Manager, a game overlay with FPS and capture, a bottleneck detector, a benchmark suite and a stability test, all local, with nothing sent anywhere. It was built in the open over ten betas and the snapshots after them; every pre-release is listed below this entry.

### Highlights

- **ZeeZ Live**: an overlay above games in its own window (nothing injected), FPS and frame times with 1% and 0.1% lows and GPU Busy, session capture, reports and comparison.
- **What limits my FPS?**: a bottleneck detector that measures, explains its verdict with the numbers behind it, and never guesses from part rankings.
- **System**: a Task Manager for every OS: processes, performance, app history, startup, users and services.
- **Every page rebuilt**: an Overview with build health and alerts, deeper Processor, Mainboard, Memory, Storage and Devices pages, and a Sensors table.
- **Benchmarks**: a CPU suite, memory, GPU, storage, core-to-core latency and a monitored stability test, each with history and comparison.
- **Private by design**: no telemetry, no account; recording and the update check are off until you allow them; rights are asked per action.
- **A new name and icon: CPU ZeeZ.**

### Added

#### ZeeZ Live: overlay, FPS and capture

- **A new ZeeZ Live entry** in the sidebar with four tabs: *Live*, *Overlay*, *Sessions* and *Bottleneck*.
- **The overlay** is CPU ZeeZ's own window above the game: transparent, always on top, never takes focus, lets every click through. Nothing is injected into any game.
  - Five styles (*Minimal*, *Compact*, *Detailed*, *Benchmark*, *Overclocker*) and a layout editor for your own.
  - Nine places on the screen plus an offset, or *Move with the mouse*; it goes on the **monitor the game is on** (or the one you pick), and dropping it on another monitor moves it there.
  - **Throttling pills** name what throttles (CPU or GPU, thermal or power), with an icon as well as a colour.
  - A real **outline** on the text, so it reads over bright and dark scenes.
  - A small **"GPU ▲" / "CPU ▲"** pill from the bottleneck detector (can be turned off).
  - On Windows it can be hidden from screenshots, recordings and streams.
  - A warning when a game runs in true exclusive fullscreen, where no window can be drawn on top.
  - First use takes three steps: pick a style, confirm the hotkey, *Try it now*.
- **FPS on Windows** through Intel's PresentMon 2.6, bundled with the installer and the portable zip: it reads the graphics kernel's events, so nothing touches the game. FPS, frame time, 1% and 0.1% lows, GPU Busy and the present mode. Needs CPU ZeeZ running as administrator (the Live tab offers the restart); runs only while ZeeZ Live shows frames.
- **FPS on Linux** through MangoHud: CPU ZeeZ detects it, installs it with your package manager after you confirm, enables it for every game or copies the Steam launch option, writes its config from your overlay layout and reads its logs.
- **macOS (experimental)**: opens a game with Apple's Metal Performance HUD (macOS gives no app another app's frame times).
- **Live tab**: the game being measured (automatic or picked), FPS, lows, GPU Busy and a frame-time chart.
- **Capture**: manual, timed (60 s by default, after a 3 s countdown), from a hotkey, or automatically per game; markers during a capture; throttling recorded as time spans; the display's refresh rate noted. Each session is saved as `frames.csv` (PresentMon's columns, so CapFrameX opens it), `hw.csv` and `session.json`. **● REC** shows on the overlay and in the window title while recording.
- **Session reports**: average, 1% and 0.1% lows (the convention is always stated) and stutters; frame-time chart with markers and throttling; load and temperature charts; frame-time percentile curve; FPS histogram; the bottleneck analysis. **Compare** sessions, with warnings when they are not comparable or vary more than 5 %. Export to offline HTML or PNG; import PresentMon, CapFrameX and MangoHud CSVs.
- **Global hotkeys**: show/hide the overlay (`Ctrl+Shift+O`), next style, start/stop capture, marker, edit mode; all can be changed, and conflicts are shown. On Wayland they go through the desktop's GlobalShortcuts portal, which lets you pick the keys.
- **Per-game profiles**: a game's own style and place, show the overlay when it starts, capture while it runs.
- **Anti-cheat safe mode**, per game: while it runs, nothing is read through the PawnIO driver.

#### What limits my FPS: the bottleneck detector

- **The "Now" card** answers what limits the frame rate right now: *GPU at its limit*, *CPU at its limit (core N)*, *Held back on purpose* (V-Sync, VRR ceiling, a frame cap, a game not in front), *Thermal throttling*, *Video memory is full*, *The system is paging*, *Waiting on the disk*, *Another program is competing*, or *Nothing at its limit*, with how sure it is and *Why?* listing the numbers.
- It measures: GPU Busy against the frame time, the game's heaviest thread (a low total CPU load can hide a saturated thread), throttling flags, video memory, paging and disk reads during stutters. Without GPU Busy (Linux, macOS, or Windows without administrator) it falls back to GPU utilization, at lower confidence, and says "estimated".
- **Live readings** with a mark where each rule fires: GPU Busy, GPU load, CPU load, busiest core, the game's busiest thread, video memory, RAM, temperatures.
- **Session analysis** for 1, 3 or 5 minutes or until stopped: time per cause, a second-by-second timeline, the strongest evidence and recommendations, each tied to a measurement (a session with nothing wrong gets none). The same analysis is in every session report.
- A **What limits my FPS?** shortcut on the Overview, next to *Build health*.

#### System: a Task Manager for Windows, Linux and macOS

- **Processes**: grouped as apps, background and system processes, as one detailed list, or as a tree; CPU, memory, disk, GPU, video memory, **network** and **power usage** per process; a 60-second graph per row, heat-map colours, pinning, column presets (*Essential*, *Gaming*, *Leak hunting*, *Security*, *Everything*), columns you can resize, move and pick (command line, architecture, elevated, integrity, UAC virtualization, CPU time, I/O, GDI and USER objects…), and *Problems* filters (not responding, growing memory, sustained high CPU, running from a temporary folder). Processes are read the way the Task Manager reads them, never their memory.
- **Details panel**: threads, network connections, windows (switch to, minimize, maximize, close), account, command line, security (elevation, integrity, architecture) and an on-demand SHA-256 of the file.
- **Actions**: end task or tree (with a strong warning for processes the system needs), restart (Explorer), suspend and resume, priority, CPU affinity, Efficiency mode, memory dump, *Analyze wait chain*, UAC virtualization, open file location, *Run new task* (optionally as administrator), export to CSV/JSON and compare two snapshots. Every action is written to a local audit log.
- **Performance**: CPU (per-thread grid, speed, processes/threads/handles, up time, idle states, caches), memory (what it is used for, page file), each disk, each network adapter, each GPU (per engine, video memory, temperature, power, link) and NPUs; 1 min / 10 min / 1 h / session graphs, a compact view and CSV export.
- **Services**: state, startup type, account, path, description, dependencies both ways and recent logs; start, stop, pause/resume (Windows), restart, startup type, log-on account (built-in accounts; passwords are left to the OS's own tool), recovery actions, start with parameters, triggers, SID type and required privileges; filters, a lock on protected services, and *Undo*. Windows Service Control Manager, systemd (recovery as a drop-in you can delete) and launchd.
- **Startup**: Windows registry and Startup folders, XDG autostart, macOS LaunchAgents; enable or disable the reversible way the Task Manager does, with the date an item was disabled, its startup impact, the firmware's last boot time, a boot timeline and *Undo*.
- **Users**: signed-in sessions with each user's resources and processes; disconnect, sign out, send a message.
- **App history** (off until you turn it on): CPU time, GPU time, disk and memory per app per day, kept locally in SQLite for the days you choose.
- **Settings ▸ System**: start page, always on top, minimize on use, hide when minimized, full account names, Efficiency-mode confirmation, history retention.

#### Hardware pages

- **Overview**: a one-line summary of the build, health pills (build checks, drive health, battery wear), tiles you choose (presets *Gamer*, *Overclocker*, *Laptop*, *Workstation*) that open the Sensors page on their metric, quick actions and *Customize*.
- **Build health**: single memory module, XMP/EXPO off, graphics card on fewer PCIe lanes, Resizable BAR off, power-saving plan, monitor below its refresh rate, monitor plugged into the motherboard while a graphics card is installed; each with how to fix it.
- **Local alerts** (opt-in): rules such as "CPU above 90 °C for 30 s" on the Overview and in the tray tooltip, checked locally.
- **Processor**: a live per-core table (clock, temperature, load, CCD/CCX, idle-state residency, CPPC preferred cores), a topology diagram, power limits (PL1/PL2 on Intel), speculative-execution vulnerabilities and mitigations, virtualization (VT-x/AMD-V, VBS, Memory integrity, KVM), the power plan or governor, instruction sets by purpose.
- **Mainboard**: firmware (UEFI or legacy, Secure Boot, TPM), every PCIe device's link against its maximum (with a warning when narrower), Resizable BAR and Above 4G decoding, mainboard sensors by name, slots and ports, a browsable SMBIOS tree, ACPI tables (Expert).
- **Memory**: the running profile (XMP/EXPO or tuned), secondary and tertiary timings on AMD, FCLK, a slot map, ECC, module temperatures, what the OS uses RAM for (standby, modified, compressed, kernel pools, commit, page file or swap), theoretical bandwidth.
- **Storage**: full SMART with thresholds, NVMe thermal-throttle counts, link speed, partition alignment, TRIM, volume encryption (BitLocker, LUKS, FileVault), and the decoded Identify data (Expert).
- **Devices**: USB as a tree with negotiated against supported speed (a warning when a USB 3 device runs at USB 2), Thunderbolt/USB4, Bluetooth battery levels, audio devices with their mix format, and displays with refresh rate against what they support, HDR and colour depth (on Linux, current and best refresh and monitor names through KMS, without root).
- **Sensors** on a real table: sort, filter (`cur>80`, `unit:°`, `id:disk`), presets, multi-select to plot, pin or copy, export the view to CSV.

#### Monitoring

- **New readings**: per-disk activity (speed, IOPS, latency, active time, queue), per-adapter network traffic (throughput, packets, errors, drops), GPU engine load (3D, copy, video encode/decode, compute), memory composition, hard page faults, busiest core load; without administrator rights.
- **Tray icons**: up to eight, one per metric, as a number, bar, mini graph or ring at the icon's real size, with colour and shape thresholds, a tooltip with every value and alert, and a menu in your language. macOS shows them as text in the menu bar.
- **Mini monitors**: as many small always-on-top windows as you like, each with its own layout, size, opacity, corners, outline and theme; snap to the edges of the monitor they are on, remember their place, lock so clicks pass through, hide during full-screen games, show graphs on hover, grow with Ctrl+scroll. Layouts pick readings by name, so they work on any computer, and can be copied as JSON.

#### Benchmarks and tools

- **CPU suite**: eight workloads (compression, n-body, SIMD matrix math, AES with and without AES instructions, SHA-256, a ray tracer, sorting), single- and multi-threaded, 1000 = the reference Ryzen 9 7900X; thread scaling, P-core vs E-core, and a sustained run that shows throttling.
- **Memory**: STREAM triad, GUPS, the cost of TLB misses on 4 KiB and large pages, latency from each CCD or NUMA node, on top of the 0.3.0 latency and bandwidth tests.
- **GPU** (DX12, Vulkan, Metal): FP32/FP16, video-memory bandwidth, upload and download over the bus (which confirms the real PCIe link), a 1080p raster scene; 1000 = the reference RTX 4060.
- **Storage**: sequential and 4K random with 1 and 8 requests in flight, bypassing the cache, plus a **sustained write** that finds where an SSD's cache runs out (sizes that fit in 80 % of the free space; the file is always deleted).
- **Core-to-core latency** heat map (on macOS, a statistical spread).
- **Stability test**: CPU light/heavy/mixed, a memory test, GPU stress, or all at once; stops itself at the temperature limit, on a memory error or if the GPU stress fails.
- **Drive health**: life left, data written against the rated endurance, temperature over time, error counters, SMART self-tests with progress and history (Windows, Linux), and a read-only surface scan.
- **Reproducible results**: each records the app version, power plan and starting temperature; 3 or 5 runs with median and spread; every benchmark has history and side-by-side comparison.

#### Interface

- **Command palette** (`Ctrl+K`): find any value, page or action.
- **Detail levels** (*Basic*, *Advanced*, *Expert*) on the hardware pages.
- **Copy and pin** any row or card; pinned sensors stay live on the Overview.
- **Units**: °C/°F, MHz/GHz, MB/MiB, fixed decimals (display only; logs and reports keep their units).
- **Tables** with resizable and movable columns, sideways scrolling, full text on hover, and keyboard control (arrows, Enter, Ctrl+A, Esc, Ctrl+C).
- **Accessibility**: WCAG AA contrast for text in both themes and 3:1 for chart colours, an icon with every warning colour, a visible keyboard focus, names and values for painted widgets and charts for assistive technology, and pages that hold together from 75 % to 200 % scale.
- **English, Português (Brasil) and Español** throughout, with a test that fails when a text has no translation.
- **The window opens without a working OpenGL driver** (VMs, Remote Desktop, missing drivers): it falls back to wgpu (DX12/WARP, Vulkan, Metal).

#### Privacy and permissions

- **A first-run screen** that explains what CPU ZeeZ does and does not do, and asks before the update check.
- **Settings ▸ Privacy & data**: what is stored and how much space it uses, open the folder, export, delete by category or everything, retention.
- **Recording is opt-in** (app history, captures, CSV logs, automatic capture), and **● REC** shows whenever anything records.
- **Rights per action**: the app starts as a normal user; an action that needs more asks for it once (UAC, polkit, macOS authorization) in a one-shot copy that runs only a fixed list of operations, and *Restart as administrator/root* is offered where a whole page needs it.
- **Capability banners** say exactly what is missing without the rights, and offer the fix.
- **Local logs** with rotation, and a **diagnostics bundle** you make and share yourself; nothing is uploaded.

#### Install and update

- **PawnIO** (the signed, open-source driver for hardware registers) ships with the Windows packages as an optional *Hardware access* component, pinned and checked; portable users get *Install hardware access* in the app. Uninstalling CPU ZeeZ keeps the shared driver.
- **PresentMon** ships with the Windows packages, pinned and checked before every run.
- **Update channels**: *Stable*, *Snapshot*, *Beta* or *Alpha* (Settings ▸ Updates, or `-checkupdate=<channel>`); the default follows the build you run.
- **One-click update** that downloads the package for your OS and CPU into a private folder, verifies it against `SHA256SUMS.txt` and hands it to the OS installer.
- **Uninstalling** is documented for every platform; your data is never deleted by an uninstall.
- Installers checked in CI on every change: install, upgrade in place from every published release, downgrade refused, optional driver left out, clean removal.

### Changed

- **The name is CPU ZeeZ**, with a new icon (a gauge arc, an area chart and a two-tone Z). The `cpuzeez` command and files, the install folder, the data folder (`%APPDATA%\CpuZeeZ`, `~/Library/Application Support/CpuZeeZ`, `$XDG_CONFIG_HOME/cpuzeez`) and `cpuzeez.ini` keep their names, so nothing breaks on upgrade.
- **The update check is off until you allow it** (it was on by default in 0.3.0), and pre-releases are only offered on the channels that include them.
- **A release ladder**: release, then snapshot (a preview with every new feature), beta and alpha; installers order them correctly for upgrades.
- **Monitoring costs less**: sources are read only while a page, the overlay, a tray icon or a mini monitor shows them; slow sources (SMART, batteries) are read every few seconds, never on the interface thread; the sampler's CPU use fell (3.2 % → 2.4 % of one core reading every sensor in the S0 measurement).
- **App history** is kept in SQLite with the retention setting applied.
- **Mainboard sensors** labelled "CPU …" stay with the mainboard on Windows and Linux.
- **Session reports** record the refresh rate and the kind of each throttling span.
- **Requires Rust 1.95** to build from source.

### Fixed

#### Since 0.3.0

- The Windows executable needed the Visual C++ runtime, which was not shipped: the C runtime is now built in.
- Reading core clocks spun for about 30 ms per core in sequence (about 480 ms per sample on 16 cores): clocks are now computed from per-tick counters.
- Each MSR read started a thread on Windows; the sampler now pins one thread.
- Every drive's SMART was read inside the sampler, and macOS ran `ioreg` twice per sample: both moved to the slow cadence.
- The interface kept a second sampler and copied whole histories every frame; a single metric engine serves everything.
- Changing the sampling period or the graph history needed a restart; the mini monitor was not remembered.
- The update check treated every pre-release as equal; versions now follow SemVer, and package versions map correctly for MSI, Debian, Arch and macOS.
- The interface could lag badly while drives were read; nothing is read while a frame is drawn, and a test keeps it that way.
- Locked (click-through) mini monitors were invisible on Windows.
- *Open file location*, *Open folder* and the one-click update failed when the app was opened by double-click on Windows.
- With PawnIO installed but its modules missing, the banner named the wrong cause.
- 20 compiler warnings and dead code removed.

#### During the pre-releases

Problems that appeared in the betas and were fixed before 1.0, for testers who saw them:

- System: an empty Services list, overlapping Performance tiles, table menus one word wide, overlapping user rows, app history counting time once per process, a freeze while hashing large files, Linux and macOS services stopped instead of paused, Windows *RunOnce* entries that could not be disabled, process details read on the interface thread.
- Layout at large text sizes (a title running into a button, overlapping rows), found by the new scale test.
- *About* missed PresentMon, and its links opened a page that did not exist yet.
- Inputs that could crash or hang the app: an overflow in the frame-time histogram, and corrupt `session.json` files that made session analysis quadratic.
- Release tooling: a workflow file GitHub refused (no release could build), a minimum Rust version that was never tested, missing translations.

### Security

- **Per-action elevation** through a closed list of operations, validated in the app and again in the elevated copy; malformed requests are refused.
- **Process actions** check that a process ID still belongs to the process you picked before acting, since IDs are reused.
- **Service and startup names** are validated so they cannot be read as options or point outside their folder.
- **No memory is ever read from anti-cheat-protected games**, and no code is ever injected into any process ([ADR 0012](../adr/0012-no-code-injection-ever.md)).
- **Paths a privileged process opens are private**: the updater's package, the "Restart as root" marker and the PresentMon executable can no longer be swapped or redirected by another local user or program, and the `-screenshots` report is masked and kept in its own folder. Four of these were reported and fixed by Vinicius de Avila Moreira ([@vinicius741](https://github.com/vinicius741), joaoregis/cpuzeez#4 and #5).
- **Third-party binaries** (PawnIO, PresentMon) are pinned by SHA-256 at build time and checked again before they run.
- **Supply chain**: GitHub Actions pinned by commit, read-only workflow tokens, `cargo-deny` on licenses and advisories, Dependabot.
- A [threat model](../security/threat-model.md), a [security policy](../../SECURITY.md) and an [audit guide](../audit/README.md).

### Quality and engineering

- **Architecture**: a workspace split by layer (`core` pure logic with no OS calls and no `unsafe`, `platform` per-OS code, the app), a metric engine, a widget engine shared by the overlay, tray and mini monitors, and a virtualized table. 15 decision records in [docs/adr/](../adr/README.md).
- **Tests on every change**: unit and GUI interaction tests (egui_kittest), deterministic robustness tests for every parser and the detector, 11 scheduled fuzz targets with a committed corpus, WCAG contrast and scale tests, a test that the update check is the only network path (and a Linux job with no network), a soak harness that drives a synthetic game for hours (8 hours on Windows: memory flat at ~143 MB, no handle or thread growth), and a randomized campaign over every parser and the detector (1 564 seeds × 500 000 rounds, no failure).
- **CI on Windows, Ubuntu (x64, arm64, X11 and Wayland), macOS (Intel, Apple Silicon), Debian, Ubuntu 22.04 and Arch**, with visual regression between a change and its base, the declared minimum Rust, `cargo-deny`, workflow linting, and installer jobs.
- **Docs**: README and README.pt-BR describe every feature and the permissions each needs; [FEATURES.md](../../FEATURES.md) has per-OS tables; [docs/accessibility.md](../accessibility.md) and [docs/performance.md](../performance.md) say what is checked and what is not.

### Not included in 1.0

- A "GPU ↔ balanced ↔ CPU" hardware balance bar: it cannot be calibrated honestly without measured reference data ([ADR 0014](../adr/0014-no-hardware-balance-bar-in-1-0.md)).
- Shared (spilled) video memory is not read; a game on a second GPU is measured with the first one's readings.
- macOS: no frame times from other apps (Apple's HUD only), no overlay over fullscreen Spaces, no SMART self-tests.
- Screen readers cannot read the inside of tables and charts yet.
- Partly done work planned for 1.1.0 is listed in [docs/v1.1/](../v1.1/README.md).

### Known limits

- ⚠ Releases are not code-signed: Windows SmartScreen and macOS Gatekeeper warn on first run ([ADR 0009](../adr/0009-releases-stay-unsigned-for-now.md)). Every release has `SHA256SUMS.txt`.
- ⚠ Games in true exclusive fullscreen hide the overlay (capture still works).
- ⚠ Wayland shortcuts, KMS refresh and Linux/macOS desktop behaviour were checked by CI and testers, not on every desktop; see [FEATURES.md](../../FEATURES.md).
- The overlay next to a game uses about 3.5 % of one CPU core (measured with a synthetic game, the main window minimised) against a 2 % goal; memory stays around 120 MB with no growth. Bringing CPU down to 2 % is planned for 1.1.0 ([docs/performance.md](../performance.md)).

### Thanks

To Vinicius de Avila Moreira ([@vinicius741](https://github.com/vinicius741)) for the macOS fixes in 0.3.0 and the security review and fixes in 1.0, and to everyone who tested the betas and snapshots.
