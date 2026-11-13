# Changelog

All notable changes to CPU ZeeZ are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/). Each release's notes on GitHub are
generated from its section below.

## [Unreleased]

## [1.1.0-beta.3] - 2026-11-13

The third 1.1 beta (M4 of the [1.1 roadmap](docs/v1.1/ROADMAP.md)): code moves with no visible change. OS calls leave the app for the platform crate (61 fewer `unsafe` outside it) and the interface's main file is split by job. Nothing should look or behave differently; anything that does is a bug. **Known, as in beta.2:** the overlay stays at about 3 % of one core and its private memory grows about 1.5 MB per minute in a 30-minute test; that is the last item before 1.1.0.

### Changed

- **Code:** the timers, firmware tables, TSC and add-chain clock, macOS CPU load and sysctl, Ctrl+C and file-path calls moved into the platform crate as safe functions; then the Windows sensors (PDH, battery), the macOS SMC, PCI enumeration and DXCore NPUs. 61 fewer `unsafe` outside it. Nothing looks different.
- **Code:** `gui/mod.rs` is split by job (page state, frame-loop chores, navigation, page helpers, translations, start-up), from 1618 lines to 386. Nothing looks different.

## [1.1.0-beta.2] - 2026-11-13

The second 1.1 beta (M3 of the [1.1 roadmap](docs/v1.1/ROADMAP.md)): the interface moves to egui 0.37, which touches every page, so look at spacing and icons. Nothing was done about the overlay's CPU: it stays at about 3 % of one core in a 30-minute test, over the 2 % goal. **Known:** in that same test the overlay's private memory grows about 1.5 MB per minute; it is on this version and the one before it, and the cause is not found.

### Changed

- **CI:** a ratchet on `unsafe` outside `crates/platform` (`.github/scripts/unsafe_ratchet.py`): a new `unsafe` block, or more in a file, fails the build until it moves to `platform`.
- **Build:** eframe, egui and egui_kittest 0.37.
- **Accessibility:** every combo box, number field, slider and checkbox has a name a screen reader reads: the label beside it, or a short one where nothing visible names it. egui_kittest 0.37 fails a test on any control without one.
- **Build:** the Phosphor icon font ships in the repository (`assets/fonts/Phosphor.ttf`, MIT) with the 104 icons the app uses; the `egui-phosphor` crate is gone, so the next egui release no longer waits for it. Nothing looks different.
- **Soak:** `soak.py` lists the busiest threads' CPU in the report on Windows.

### Fixed

- Icons in monospace text and the ● in *REC* and the sensor legend drew as empty boxes where the font lacked them; *REC* now uses the record icon and the sensor marks a bullet that every font has.
- **Settings at 200 %:** a row's controls move below its label when they do not fit beside it.

## [1.1.0-beta.1] - 2026-11-12

The first 1.1 beta (M2 of the [1.1 roadmap](docs/v1.1/ROADMAP.md)): every page tested, side panels that fit and resize, ZeeZ Live that fits the window.

### Changed

- **Packages:** a pre-release of a minor version (`1.1.0-beta.1`) gets package versions that upgrade in order: MSI and macOS `1.0.60001`, between `1.0.2` and `1.1.0`.
- **System ▸ Services and Processes: the side panel can be resized.** Drag the line between the table and the inspector (or the details panel), or focus it with Tab and use ← and →; a double-click puts it back. The width is kept per panel, between 280 points and 60 % of the window.
- **Docs:** the README's table of what needs administrator or root is generated from the code, in English and Portuguese, and a test fails when they differ (`python .github/scripts/permissions_map.py --write` regenerates it).
- **Screenshots:** `-screenshots` shows ZeeZ Live with a recorded game's frames (FPS, lows, GPU Busy) instead of whatever app was in front, and never starts PresentMon.
- **Tests:** each overlay style has a drawing budget (shapes and text per frame) that CI checks, and the soak report shows the overlay's drawing time as a trend.
- **Tests:** every page and tab is drawn and driven in a test, from fixed data that reads no hardware: controls have names, and copy, pin, the detail level and the tabs work. A new page or tab without a test fails CI.

### Fixed

- **System ▸ Services and Processes on a narrow window.** The inspector and the details panel open over the table when both do not fit side by side; the inspector used to disappear below 700 points.
- **System ▸ Services: the inspector fits its panel** in every language and zoom: long values wrap or are cut with the full text on hover, and the combo boxes no longer push it past the window.
- **ZeeZ Live ▸ Live fits the window.** The frame tiles (FPS, 1% low, 0.1% low, GPU Busy) ran past the right edge of a 1120-pixel window; they now take as many columns as fit. The overlay preview turns into a column when its one-line strip is wider than the page.
- **Tabs wrap** onto a second line when they do not fit (Tools and System at a large zoom or in a long translation).
- **System ▸ Processes:** at 200 % the network note covered the *Problems* filters; it now has its own line.
- **CI: the fuzz workflow runs again.** Its job timeout used arithmetic, which workflow expressions do not have, so GitHub rejected the file and the nightly fuzzing had not run since 2026-11-10.
- **Windows: the taskbar icon after an update.** A pinned CPU ZeeZ turned into a blank page after an update and a restart. The shortcut's icon was a copy kept by Windows Installer for that one version, which the update deletes; it now comes from `cpuzeez.exe` itself. A pin made before this version needs to be pinned again once.

## [1.0.2] - 2026-11-10

### Changed

- **CI:** tests run with `cargo nextest`. A test running past 60 s is reported and killed at 120 s, with its name in the log. A test that passes only on retry is reported as flaky, and a JUnit report is kept when tests fail.
- **CI:** the fuzz workflow takes a duration (`minutes`, for the long campaign of plan item 5). It writes each target's executions in the run summary and keeps the corpus it grew.

### Fixed

- **System ▸ Services and Startup: a refused change.** Windows refusing a change (error 5 on `CodexSandboxService`, whose own permissions refuse administrators) left three problems behind:
  - an *Undo* button for a change that never happened;
  - the editor still showing the refused value;
  - a raw `OpenServiceW(…) failed (error 5)`.

  Now the undo appears only after the change works, the editor shows the service as it is, and common Windows errors read as a sentence with their code (en, pt-BR, es). Before this, a copy not running as administrator said only "the elevated copy exited with code 1"; the error code now comes back from the elevated copy.
- **System ▸ Services: said before the click.** The inspector reads the service's permissions, which needs no elevation. When they refuse an administrator, *Apply*, *Start*, *Stop* and *Restart* are disabled, with the reason.
- **System ▸ Services:** *Stopped* is translated.
- **Settings ▸ Privacy & data** lists *Exports* (`exports/` in the data folder), so they can be seen and deleted like the other categories.
- **Windows:** the per-process TCP and UDP tables are read into 4-byte words. They were read into a byte buffer that the language does not promise to align (2026-11-09 security review).
- **Windows: an update closes CPU ZeeZ and opens it again.** *Download & install* used to end the app abruptly. In administrator mode that left the PresentMon sidecar running, and it kept the installed files locked. Now the app quits normally, and a helper waits for it, installs with only a progress bar, and reopens CPU ZeeZ with the same rights. If the install is cancelled or fails, the old version reopens and says why.
- **Windows: one running copy.** Opening CPU ZeeZ while it is already running, even hidden in the tray, brings that copy to the front instead of starting a second one next to it. A copy left in the tray also kept the installer from updating the files.
- **Linux and macOS:** ZeeZ Live said frames came from "the graphics kernel's events", which is the Windows source. It now names MangoHud's logs on Linux and the Metal HUD on macOS.
- **Builds:** the Linux and macOS release builds no longer print dead-code warnings. CI no longer allows dead code, so a warning that only one OS sees now fails CI.

## [1.0.1] - 2026-11-09

### Changed

- Dependencies: `tray-icon` 0.24 → 0.26 (the Windows and macOS tray icon; five indirect dependencies are gone), `libc` 0.2.191.
- CI: `actions/checkout` v7, `actions/upload-artifact` v7, `actions/download-artifact` v8 and a newer `dtolnay/rust-toolchain` pin for the fuzz job. Dependabot now proposes the egui crates together in one pull request, and the CI actions in one weekly pull request.

### Fixed

- A test of the background cache waited a fixed 100 ms for a refresh and could fail on a busy CI runner; it now waits for the refresh with a deadline.

### Documentation

- The 1.1.0 [roadmap](docs/v1.1/ROADMAP.md): corrections and debt only, in six milestones (`1.0.1`, `1.0.2`, three `1.1.0` betas, `1.1.0`), one row per pull request with its gate. The [plan](docs/v1.1/README.md) gains two items (the security review's notes, the macOS and Arch installers in CI) and the counts checked against the code.

## [1.0.0] - 2026-11-09

CPU ZeeZ 1.0 is the CPU-Z feature set grown into a complete tool for the PC on Windows, Linux and macOS: a hardware monitor, a Task Manager, a game overlay with FPS and capture, a bottleneck detector, a benchmark suite and a stability test, all local, with nothing sent anywhere. It was built in the open over ten betas and two snapshots; every pre-release is listed below this entry.

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
- **No memory is ever read from anti-cheat-protected games**, and no code is ever injected into any process ([ADR 0012](docs/adr/0012-no-code-injection-ever.md)).
- **Paths a privileged process opens are private**: the updater's package, the "Restart as root" marker and the PresentMon executable can no longer be swapped or redirected by another local user or program, and the `-screenshots` report is masked and kept in its own folder. Four of these were reported and fixed by Vinicius de Avila Moreira ([@vinicius741](https://github.com/vinicius741), joaoregis/cpuzeez#4 and #5).
- **From a review of the whole tree** before the release ([review](docs/security/review-2026-11-09.md)): an elevated memory dump can no longer overwrite or follow a planted file; the update check accepts only `https://` URLs and keeps `curl` on HTTPS; exports and copied rows neutralise spreadsheet formulas; exports go to `exports/` in the data folder, never the current folder; `notify-send` cannot read a message as an option.
- **Third-party binaries** (PawnIO, PresentMon) are pinned by SHA-256 at build time and checked again before they run.
- **Supply chain**: GitHub Actions pinned by commit, read-only workflow tokens, `cargo-deny` on licenses and advisories, Dependabot.
- A [threat model](docs/security/threat-model.md), a [security policy](SECURITY.md) and an [audit guide](docs/audit/README.md).

### Quality and engineering

- **Architecture**: a workspace split by layer (`core` pure logic with no OS calls and no `unsafe`, `platform` per-OS code, the app), a metric engine, a widget engine shared by the overlay, tray and mini monitors, and a virtualized table. 15 decision records in [docs/adr/](docs/adr/README.md).
- **Tests on every change**: unit and GUI interaction tests (egui_kittest), deterministic robustness tests for every parser and the detector, 11 scheduled fuzz targets with a committed corpus, WCAG contrast and scale tests, a test that the update check is the only network path (and a Linux job with no network), a soak harness that drives a synthetic game for hours (8 hours on Windows: memory flat at ~143 MB, no handle or thread growth), and a randomized campaign over every parser and the detector (1 564 seeds × 500 000 rounds, no failure).
- **CI on Windows, Ubuntu (x64, arm64, X11 and Wayland), macOS (Intel, Apple Silicon), Debian, Ubuntu 22.04 and Arch**, with visual regression between a change and its base, the declared minimum Rust, `cargo-deny`, workflow linting, and installer jobs.
- **Docs**: README and README.pt-BR describe every feature and the permissions each needs; [FEATURES.md](FEATURES.md) has per-OS tables; [docs/accessibility.md](docs/accessibility.md) and [docs/performance.md](docs/performance.md) say what is checked and what is not.

### Not included in 1.0

- A "GPU ↔ balanced ↔ CPU" hardware balance bar: it cannot be calibrated honestly without measured reference data ([ADR 0014](docs/adr/0014-no-hardware-balance-bar-in-1-0.md)).
- Shared (spilled) video memory is not read; a game on a second GPU is measured with the first one's readings.
- macOS: no frame times from other apps (Apple's HUD only), no overlay over fullscreen Spaces, no SMART self-tests.
- Screen readers cannot read the inside of tables and charts yet.
- Partly done work planned for 1.1.0 is listed in [docs/v1.1/](docs/v1.1/README.md).

### Known limits

- Releases are not code-signed: Windows SmartScreen and macOS Gatekeeper warn on first run ([ADR 0009](docs/adr/0009-releases-stay-unsigned-for-now.md)). Every release has `SHA256SUMS.txt`.
- Games in true exclusive fullscreen hide the overlay (capture still works).
- Wayland shortcuts, KMS refresh and Linux/macOS desktop behaviour were checked by CI and testers, not on every desktop; see [FEATURES.md](FEATURES.md).
- The overlay next to a game uses about 3.5 % of one CPU core (measured with a synthetic game, the main window minimised) against a 2 % goal; memory stays around 120 MB with no growth. Bringing CPU down to 2 % is planned for 1.1.0 ([docs/performance.md](docs/performance.md)).

### Thanks

To Vinicius de Avila Moreira ([@vinicius741](https://github.com/vinicius741)) for the macOS fixes in 0.3.0 and the security review and fixes in 1.0, and to everyone who tested the betas and snapshots.

## [1.0.0-snapshot.2] - 2026-11-09

The second snapshot of 1.0: the fixes from a security review of the whole tree, and the results of the long overnight runs ([docs/performance.md](docs/performance.md)).

### Security

From a review of the whole tree after snapshot.1 ([docs/security/review-2026-11-09.md](docs/security/review-2026-11-09.md)):

- *Create dump file* (Windows, run elevated) wrote to a predictable name in your temp folder and overwrote whatever was there, following a planted link; the name is now unpredictable and the file is created only if nothing has that name.
- The update check only accepts `https://` URLs from GitHub's answer, and `curl` stays on HTTPS, redirects included.
- Exports and copied rows no longer carry spreadsheet formulas: text from other programs that Excel would run as a formula is prefixed with `'`.
- Process and performance exports go to `exports/` in the data folder, with the full path shown, instead of the current folder.
- Linux: a message sent to another session can no longer be read by `notify-send` as an option.

### Changed

- Testing: an 8-hour run of the interface (every page and tab, captures, analysis, overlay) showed no memory, handle or thread growth, and a randomized campaign over every parser and the detector passed 1 564 seeds. The soak has an overlay-only mode (`--overlay-only`) for the CPU budget next to a game; its first result, 3.5 % of one core against a 2 % goal, is open work for the final release.

## [1.0.0-snapshot.1] - 2026-11-09

The first snapshot of 1.0: every 1.0 feature together, with the security fixes from the contributor review. From here on, a snapshot is the preview before the release.

### Changed

- **A release ladder with snapshots.** From steadiest to earliest: **release** (production), **snapshot** (a preview with every new feature), **beta** (pieces of the new features while they are built), **alpha** (early, unstable code). Tags `vX.0.0-snapshot.N` build like the others (MSI `X-1.99.(100+N)`, above every beta and release candidate; `.deb` `X.0.0~snapshot.N`; Arch `X.0.0snapshotN`), and *Settings ▸ Updates* has a **Snapshot** channel. The Beta channel also offers snapshots, so beta testers move up to them.

### Security

Three local privilege problems and one information leak, reported and fixed by Vinicius de Avila Moreira ([@vinicius741](https://github.com/vinicius741)) in joaoregis/cpuzeez#4 and #5, reviewed and integrated with the follow-ups below. All need someone already running code on the machine.

- **Updater (Linux, High).** The installer was downloaded to a predictable folder in the shared `/tmp`. Another local user could create it first and swap the verified package while the password prompt was open, so `apt`/`pacman` installed theirs as root. The download now goes to a fresh private folder (unpredictable name, created exclusively, `0700`, owner and every parent folder checked).
- **"Restart as root" (Linux, High).** The root copy wrote its start-up marker to a predictable `/tmp` path and followed a symlink planted there, so another user could make root overwrite a file of their choice. The marker now lives in a private folder, root refuses to follow links or write outside it, and checks the folder belongs to the user who asked.
- **PresentMon (Windows, High in a writable install folder).** It was checked once and then run several times by path, and the teardown run was not checked at all; a program able to write the folder could swap it in between and run as administrator. The checked file is now held open so it cannot be changed, and every run uses that file.
- **`-screenshots` (Medium).** The full unmasked report (serials, MAC and IP addresses) was written to a fixed, world-readable file in `/tmp`. It is now masked and written to the run's own output folder.
- Follow-ups from the review: the private-folder code of both fixes is one shared module (`cpuzeez_platform::tempdir`), its mode is pinned to `0700` whatever the umask, and folders left behind by earlier updates are swept. "Restart as administrator/root" no longer forwards any of the original command-line arguments to the elevated copy.

## [1.0.0-beta.10] - 2026-11-08

### Fixed

- **The one-click update failed** with "curl: The handle is invalid. (os error 6)" when the app was opened by double-click. Windows hands such an app a console that CpuZeeZ then drops, and every program it started inherited the dead standard handles. The standard handles now point at `NUL` once the console is dropped, and the update's own helpers never inherit stdin. The same problem affected *Open file location* and *Open folder*. A test starts a real child process under those conditions. **If you are on beta.8 or beta.9, install this one by hand once** (the MSI upgrades in place); after that the in-app update works.

## [1.0.0-beta.9] - 2026-11-07

The new name and icon, and fixes to what beta.8 showed in *About*.

### Fixed

- *About ▸ Third-party components* now lists PresentMon (and, on Linux, says MangoHud is optional and not bundled).
- *About ▸ Full list and licenses* and the PawnIO guide linked to a file on the `main` branch that does not exist until the branch is merged (a 404). They open the document at the tag of the version you run.

### Changed

- **A new name and icon: CPU ZeeZ.** The window, About, tray, installer, shortcut, reports and docs say "CPU ZeeZ". The icon is new: a gauge arc, an area chart and a two-tone Z, in the app's violet. What stays as it was so nothing breaks: the `cpuzeez` command and file names, the install folder (`C:\Program Files\CpuZeeZ`), the data folder (`%APPDATA%\CpuZeeZ`, so your settings, history and sessions are found), the `cpuzeez.ini` file and the repository name. Windows shows the new name in *Installed apps* after the next upgrade.
- The minimum Rust version is now `1.95` and CI builds with it (it said `1.89`, which the GUI toolkit does not build on).
- A starting corpus for the fuzz targets is committed (`fuzz/corpus/`).
- [docs/v1.1/README.md](docs/v1.1/README.md) plans the 1.1.0 corrections that 1.0.0 leaves partly done.

## [1.0.0-beta.8] - 2026-11-07

Hardening: tests, security, accessibility, installers and the leftovers of ZeeZ Live.

### Added

- **Update channels.** *Settings ▸ Updates ▸ Update channel*: **Stable** (releases only), **Beta** (also betas and release candidates) or **Alpha** (everything). The default follows the build you run: a beta is told about newer betas. The startup check, *About ▸ Check for updates* and `cpuzeez -checkupdate[=stable|beta|alpha]` use it. The check now reads the releases list instead of `/releases/latest`, which GitHub keeps pre-releases out of. A *Beta report* issue template is included.
- **Throttling pills** in the overlay and mini monitors (a new widget kind, in the *Benchmark* and *Overclocker* styles; the layout editor picks CPU, GPU or both): one pill per throttle that is on, with a warning icon besides the colour, or a quiet "no throttling".
- **Overlay and mini monitors per monitor.** The overlay goes on the monitor the game is on (or the one you choose in *Position ▸ Monitor*) and dropping it on another monitor moves it there. Mini monitors snap to the edges of the monitor they are on. (Windows lists the monitors; elsewhere the screen is the one the window reports, as before.)
- **A real outline on overlay text**: the glyphs are stroked, not drawn on a dark plate.
- **Wayland global shortcuts** through the desktop's GlobalShortcuts portal ([ADR 0015](docs/adr/0015-wayland-shortcuts-through-the-portal.md)): the five ZeeZ Live shortcuts are asked of the desktop, which shows them and lets you pick the keys. Where there is no portal the card says so and shortcuts work while CpuZeeZ has focus. The exchange with a real portal is not tested in CI (no Wayland desktop there).
- **Linux refresh rate through KMS**: current and best refresh, monitor names (from the EDID) and the connector, without root. V-Sync detection and *Build health* now see the refresh rate on Linux.
- **Accessibility**: AA contrast for text in both themes and 3:1 for chart colours (several colours were darkened), icons with every warning mark, a visible keyboard focus, names and values for painted widgets, charts and bars, and pages that hold together from 75% to 200% scale. Checked by tests ([docs/accessibility.md](docs/accessibility.md)).
- **Security**: a [threat model](docs/security/threat-model.md), [SECURITY.md](SECURITY.md), five more fuzz targets (11 in all), deterministic robustness tests for every parser and the detector, GitHub Actions pinned by commit with read-only tokens, Dependabot.
- **Soak test** (`--features soak`, `soak.py --gui`): a synthetic game drives the overlay, capture, detector and a tour of every page for hours while memory, handles and threads are checked ([docs/testing/soak.md](docs/testing/soak.md)). Not part of any release.
- **Installer checks in CI**: install without the optional driver, clean removal (folder, shortcut, PATH entry; your data stays), and upgrade through **every published MSI** to the current build.
- [docs/audit/](docs/audit/README.md): a guide for whoever audits the project; [docs/performance.md](docs/performance.md): the P6 budget and what is measured.

### Fixed

- Found by the new tests: a frame-time histogram could overflow on absurd values; building a session's windows was quadratic in its length and allocated by the timestamp's size (a corrupt `session.json` could hang the app).
- Found by the scale test: the *System* page header ran the title into *Run new task* at large text sizes; the Users session row overlapped at 150%.
- `release.yml` had two `with:` keys on one step after the actions were pinned, which GitHub refuses as a whole: no tag would have built a release. A CI check now looks for it.
- `unsafe` is gone from the GUI (`forbid(unsafe_code)`): its two OS calls moved to `cpuzeez_platform::os`.

### Changed

- The ROADMAP marks every S0 row with its outcome, and the places where a row is only partly met.

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

[Unreleased]: https://github.com/joaoregis/cpuzeez/compare/v1.1.0-beta.3...HEAD
[1.1.0-beta.3]: https://github.com/joaoregis/cpuzeez/compare/v1.1.0-beta.2...v1.1.0-beta.3
[1.1.0-beta.2]: https://github.com/joaoregis/cpuzeez/compare/v1.1.0-beta.1...v1.1.0-beta.2
[1.1.0-beta.1]: https://github.com/joaoregis/cpuzeez/compare/v1.0.2...v1.1.0-beta.1
[1.0.2]: https://github.com/joaoregis/cpuzeez/compare/v1.0.1...v1.0.2
[1.0.1]: https://github.com/joaoregis/cpuzeez/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/joaoregis/cpuzeez/compare/v0.3.0...v1.0.0
[1.0.0-snapshot.2]: https://github.com/joaoregis/cpuzeez/compare/v1.0.0-snapshot.1...v1.0.0-snapshot.2
[1.0.0-snapshot.1]: https://github.com/joaoregis/cpuzeez/compare/v1.0.0-beta.10...v1.0.0-snapshot.1
[1.0.0-beta.10]: https://github.com/joaoregis/cpuzeez/compare/v1.0.0-beta.9...v1.0.0-beta.10
[1.0.0-beta.9]: https://github.com/joaoregis/cpuzeez/compare/v1.0.0-beta.8...v1.0.0-beta.9
[1.0.0-beta.8]: https://github.com/joaoregis/cpuzeez/compare/v1.0.0-beta.7...v1.0.0-beta.8
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
