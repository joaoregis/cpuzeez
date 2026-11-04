# System hub: a cross-platform Task Manager

> Part of the [1.0 plan](README.md). Status: design. Delivered in stage **S2a**.
> Goal: **100% of the Windows Task Manager's functions** (and of `services.msc`) on Windows, Linux and macOS, with better UX. Where an OS doesn't allow something, the UI says why.

## 1. Evidence: where the Windows Task Manager gets each value

This is static, *clean-room* analysis: we record **what** it uses (imports and strings), not **how** its code works, in the same spirit as [FEATURES.md](../../FEATURES.md).

- **Binary:** `C:\Windows\System32\Taskmgr.exe` 10.0.26100.9549 (Windows 10.0.26200), 5.87 MB.
- **Scripts:** [`evidence/pe_imports.py`](evidence/pe_imports.py) and [`evidence/pe_strings.py`](evidence/pe_strings.py) (Python, stdlib only).

```bash
python -I docs/v1/evidence/pe_imports.py C:/Windows/System32/Taskmgr.exe
python -I docs/v1/evidence/pe_strings.py C:/Windows/System32/Taskmgr.exe "StartupApproved|FwPOSTTime|GPU Engine"
```

| Tab / function | Evidence in the binary | Conclusion |
|---|---|---|
| Processes / Details | `ntdll!NtQuerySystemInformation`, `PcwCreateQuery/PcwCollectData`, `K32GetProcessMemoryInfo`, `QueryFullProcessImageNameW`, `GetProcessTimes`, `GetProcessorSystemCycleTime` | One kernel snapshot of every process per refresh, plus performance counters |
| Per-process GPU | Strings `\GPU Engine(*)\Utilization Percentage`, `\GPU Engine(*)\Running Time`, `\GPU Process Memory(*)\Dedicated Usage`/`Shared Usage`/`Total Committed`; `pdh.dll`; `D3DKMTQueryAdapterInfo`; `DXCoreCreateAdapterFactory` | Per-process GPU load and VRAM come from **public PDH counters**. We can do the same |
| Publisher | `TmGetPublisherFromVersionInfo`, `ResolveImagePublisher_Desktop/_Immersive` | "Publisher" is the **CompanyName from the version info** (or the package, for UWP apps), **not the digital signature**. It's cheap. Real signature verification is an extra of ours (§3.3) |
| Efficiency mode | `SetProcessInformation`, strings `Efficiency Mode`, `EfficiencyModeDenyList` | EcoQoS (`ProcessPowerThrottling`) + low priority, with a deny-list of processes it never applies to |
| Create dump file | `dbghelp!MiniDumpWriteDump` | Process memory dump |
| Analyze wait chain | `OpenThreadWaitChainSession` | Wait Chain Traversal API |
| UAC virtualization | `ToggleProcessVirtualization`, `UAC virtualization` | Per-process toggle |
| App history | `WdcAppHistoryMonitor::_RegisterForSrumData`, `_ProcessNetworkSrumRecord`, `_RetireOldUsageData` | The data comes from **SRUM** (System Resource Usage Monitor), the usage database Windows itself records |
| Startup apps | `Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved`; WinRT `StartupTask` | Enable/disable = a value under `StartupApproved`. Packaged apps use `StartupTask` |
| Startup impact | `CStartupImpactHelper::*`, `%WINDIR%\system32\WDI\LogFiles\StartupInfo\%s_StartupInfo%d.xml`, column `StartupDiskIO` | Impact is computed from **XML files Windows writes at every boot** (CPU time and disk I/O per item) |
| Last BIOS time | `FwPOSTTime`, `BIOSPostTimeLabel` | A registry value written at boot (firmware POST time) |
| Users | `WTSEnumerateSessionsW`, `WTSQuerySessionInformationW`, `WTSDisconnectSession`, `WTSLogoffSession`, `WTSSendMessageW`, `WinStationConnectW`, `CredUIPromptForCredentialsW` | Sessions, disconnect, sign out, send message, connect (asks for credentials) |
| Services | `SubscribeServiceChangeNotifications` + the SCM APIs in `advapi32` | Listing and state-change notifications |
| Network (performance) | `IPHLPAPI!GetIfEntry2`, `GetAdaptersAddresses`, `NSI!NsiGetAllParameters` | Per-adapter counters |
| Settings | Strings `AlwaysOnTop`, `DefaultStartPage`, `MinimizeOnUse`, `HideWhenMinimized`, `UpdateSpeed`, `SelectedUpdateSpeedIndex` | The Settings tab items |

✅ **Confirmed (S0-X12, 2026-11-04, Windows 11 build 26200).** Observed on a real machine and checked against Microsoft's documentation, never against Microsoft's code:
- **Startup-impact thresholds** (Microsoft's documentation): *High* = more than 1 s of CPU or more than 3 MB of disk I/O; *Medium* = 300 ms–1 s or 300 KB–3 MB; *Low* = less than 300 ms and less than 300 KB. One Microsoft support page gives 292 KB for the Low disk boundary, so we use 300 KB, following the developer documentation. Items without data show *Not measured*, as the Task Manager does.
- **`StartupApproved` value**: 12 bytes (`REG_BINARY`) under `HKCU|HKLM\…\Explorer\StartupApproved\{Run, Run32, StartupFolder}`, named like the `Run` value or the `.lnk` file:
  - bytes 0–3, a little-endian DWORD: bit 0 set = **disabled** (`03`); clear = **enabled** (`02`, or `06` for some system entries such as `SecurityHealth`). A missing value means enabled.
  - bytes 4–11, a `FILETIME` (UTC) of **when it was disabled**; zero while enabled. Decoded values match the dates the entries were disabled, which gives the *date disabled* column for free.
  - To disable, we write `03 00 00 00` + the current `FILETIME`. To enable, we write `02 00 00 00` + 8 zero bytes. Both are reversible and never touch the `Run` entry itself.
- **WDI `StartupInfo` XML** (`%WINDIR%\System32\WDI\LogFiles\StartupInfo\<SID>_StartupInfo<N>.xml`, UTF-16, admin-only): `<Process Name PID>` with `<DiskUsage Units="bytes">`, `<CpuUsage Units="us">`, `<CommandLine>` and the parent. ⚠ **On the test machine the newest file was 15 months old.** Windows doesn't write these on every boot, so impact is often stale or missing. We show the file's date next to the impact and fall back to our own opt-in recorder (S2a), exactly as on Linux and macOS.

## 2. Navigation (decided, Q5)

- One **"System"** sidebar entry with internal tabs, using the existing `segmented` control:

  `Processes · Performance · App history · Startup · Users · Services`

- **Details** is a **view mode** of the Processes tab, sharing the same table:

| Mode | What it shows |
|---|---|
| **Grouped** | Processes-tab style: Apps / Background processes / Windows (or system) processes, with children |
| **Detailed** | Details-tab style: flat list, every advanced column |

- Nothing is lost: every column and action from both tabs is still there.
- The benefit: one place for filters, selection and actions.
- The Task Manager's **Settings** move into *Settings › System*, as a section. They share the update speed with the Metric Engine.
- **How it relates to existing pages**, to avoid duplication:
  - **Performance** shows per-device graphs, Task Manager style;
  - **Sensors** stays the raw sensor table, HWiNFO style;
  - the **Overview** links to both.

## 3. Processes (+ Detailed mode)

### 3.1 Columns: all of the Task Manager's, and more

| Column | Windows | Linux | macOS |
|---|---|---|---|
| Name, PID, parent PID, image name, path, command line | ✅ NtQSI + `QueryFullProcessImageNameW` | ✅ `/proc/<pid>/{stat,cmdline,exe}` | ✅ `libproc` (`proc_pidpath`, `KERN_PROCARGS2`) |
| Type (App / Background / System) | ✅ app = has a visible window; UWP by package | ⚠ app = has a `.desktop`/window (X11) or an app cgroup (`app-*.scope`) | ✅ `NSRunningApplication.activationPolicy` |
| Status: Suspended, **Efficiency mode**, Not responding | ✅ (`IsHungAppWindow`) | ⚠ Suspended = state `T`; "not responding" ❌ (no such concept) | ⚠ Suspended; "not responding" ⚠ 🔬 |
| Publisher / description | ✅ version info | ⚠ owning package (`dpkg -S`, `rpm -qf`, `pacman -Qo`), on demand and cached | ✅ signing team (`SecCode`) + `CFBundleName` |
| User, session | ✅ | ✅ | ✅ |
| CPU %, CPU time (user/kernel), cycles | ✅ | ✅ | ✅ (`proc_pid_rusage`) |
| Memory: working set, private, shared, commit, peak, paged/non-paged pool, page faults | ✅ | ✅ `/proc/<pid>/status`, `smaps_rollup` (other users 🔑) | ⚠ `phys_footprint`, `resident`; pools n/a |
| Disk: read/write B/s, operations | ✅ NtQSI I/O counters | ✅ `/proc/<pid>/io` (other users 🔑) | ✅ `rusage.ri_diskio_bytes*` |
| **Per-process network** | 🔑 ETW `Microsoft-Windows-Kernel-Network` 🔬 | 🔑 eBPF / `sock_diag` 🔬 · ✅ connection count | ⚠ `nettop` (system CLI) 🔬 |
| **GPU % and engine; dedicated/shared VRAM** | ✅ PDH (§1) | ⚠ `fdinfo` `drm-engine-*`/`drm-memory-*` (amdgpu, i915, xe); NVML for NVIDIA | ⚠ IOAccelerator `AppUsage` 🔬 |
| Power usage / trend | ⚠ Windows uses an internal estimator (E3) 🔬. Otherwise our own estimate = share of CPU and GPU time × package power, **labelled "estimate"** | ⚠ estimate (same formula, RAPL) | ✅/⚠ `rusage` energy 🔬 |
| Handles, threads, USER and GDI objects | ✅ | ✅ fds, threads; USER/GDI n/a | ✅ threads, ports; USER/GDI n/a |
| Priority, affinity, architecture (x64/ARM64/emulated), elevated, integrity level, UAC virtualization, DEP, stack protection (CET), OS context | ✅ | ✅ nice, affinity, architecture; the rest n/a | ✅ architecture (Rosetta) and *sandbox*; affinity ❌ (macOS offers apps no affinity API) |

**Anti-cheat and privacy rule (P4):**
- Columns come from the kernel snapshot (NtQSI/procfs/libproc), **without opening the process**.
- When a handle is unavoidable, it is opened with `PROCESS_QUERY_LIMITED_INFORMATION`.
- Nothing ever reads another process's memory. Environment variables and loaded modules are shown **only on demand**, in the details panel, and **never** for protected processes or for games with a known anti-cheat.

### 3.2 Actions (every one of the Task Manager's)

| Action | Windows | Linux | macOS | Privilege |
|---|---|---|---|---|
| End task / end process tree | `TerminateProcess` (tree via parent PID) | `SIGTERM` → wait → `SIGKILL` (configurable) | same | 🔑 if another user's or elevated |
| Restart (Explorer, shell) | ✅ | ✅ (restarting `plasmashell`/`gnome-shell` ⚠) | ✅ Dock/Finder | ✅ |
| Suspend / resume | `NtSuspendProcess`/`NtResumeProcess` | `SIGSTOP`/`SIGCONT` | `SIGSTOP`/`SIGCONT` | 🔑 same |
| **Efficiency / "Eco" mode** | EcoQoS + *Idle* priority | `SCHED_IDLE` + nice 19 | `taskpolicy -b` (*background* QoS, runs on Apple Silicon E-cores) | ✅/🔑 |
| Set priority | `SetPriorityClass` | `setpriority` (raising it 🔑) | `setpriority` | ✅/🔑 |
| Set affinity | `SetProcessAffinityMask`, CPU groups | `sched_setaffinity` | ❌ not available on macOS | ✅/🔑 |
| Create dump file | `MiniDumpWriteDump` | `gcore` if installed ⚠ | ❌ no `task_for_pid` (needs an entitlement) | 🔑 |
| Analyze wait chain | WCT | ⚠ state `D` + `wchan` | ❌ | ✅ |
| UAC virtualization | ✅ | n/a | n/a | 🔑 |
| Open file location / Properties / Search online | ✅ | ✅ (file manager via portal) | ✅ (Finder) | ✅. Search online opens the browser only on click (P1) |
| Go to details / services / window (switch to, bring to front, minimize, maximize) | ✅ | ✅ X11 · ⚠ Wayland (no control over other apps' windows) | ✅ | ✅ |
| **Run new task** (with "create with administrative privileges") | `ShellExecuteEx` (`runas` verb) | A command through the user's shell; "as root" via `pkexec` | `open`/spawn; admin via the authorization prompt | ✅/🔑 |

- Every action goes through the `Executor` ([engineering §4.2](engineering.md)), with confirmation by risk.
- **Critical processes** get a strong, explicit warning:
  - Windows: `csrss`, `wininit`, `smss`, `lsass`, `services`, `System`. Ending them forces a blue screen or a reboot.
  - Linux: PID 1, the display server and the compositor.
  - macOS: `launchd`, `WindowServer`, `kernel_task`.

### 3.3 UX improvements (beyond the Task Manager)

- **Search syntax:** `chrome`, `pid:1234`, `user:SYSTEM`, `cpu>10`, `mem>1GB`, `publisher:"Microsoft"`, `status:suspended`. It autocompletes and works on any column.
- **Tree view** parent → children. It keeps the lineage even when the parent is gone ("parent: 1234 (exited)").
- **Column presets:** *Essential*, *Gaming* (CPU, GPU, VRAM, heaviest thread, priority), *Leak hunting* (private, commit, delta, trend), *Security* (publisher, signature, integrity, elevated, path), *Everything*. Users can save their own.
- **Heat-map** cells (like the Task Manager), a **60 s sparkline per row**, and **pinned processes** on top.
- **Smart "Problems" filters**, always explaining why and never alarmist:
  - not responding;
  - memory growing steadily over the last N minutes (trend);
  - sustained high CPU;
  - processes running from temp folders;
  - executables without a valid signature (checked on demand).
- **Side details panel**, without leaving the page:
  - Overview;
  - Threads (with per-thread CPU, key for the bottleneck detector);
  - Modules (on demand);
  - **Network connections** (`GetExtendedTcpTable`/`sock_diag`/`proc_pidfdinfo`: local/remote address and state; no admin needed on Windows for your own processes);
  - Open files (on demand);
  - Hosted services;
  - Windows;
  - GPU;
  - Security: signature (`WinVerifyTrust`, `codesign`), integrity and an **on-demand SHA-256**. "Open in VirusTotal" is only a link that opens on click; the hash is never sent by the app.
- **Multi-select** with batch actions ("End 5 processes"), one confirmation and a list of the targets.
- **Export** (CSV/JSON) and **compare two snapshots** ("what appeared since boot").
- **Update speed**, shared (High 0.5 s / Normal 1 s / Low 4 s / Paused), with **"Pause"** on a key, so you can click a row that keeps moving.

## 4. Performance

The Task Manager shows CPU, Memory, Disk N, Ethernet/Wi‑Fi, GPU N and NPU. We cover all of that and more:

| Device | Task Manager | + CpuZeeZ |
|---|---|---|
| **CPU** | Overall or per-logical-processor graph; utilization, speed, processes, threads, handles, up time, base speed, sockets, cores, LPs, virtualization, L1–L3; "show kernel times" | **Per-core grid** with heat colours: load, effective clock, temperature and type (P/E/LP), grouped by **CCD/CCX/cluster**; package power; throttling flags; C-state residency (🧩/🔑); preferred cores |
| **Memory** | In use, available, committed, cached, pools, speed, slots, form factor, hardware reserved, composition bar | Detailed composition (in use / modified / standby / free; **compressed**; *wired* on macOS); page file/swap; current timings (link to Memory); DIMM temperatures |
| **Disk** (per physical disk) | Active time, average response time, read/write speed, capacity, formatted, system disk, page file, type | **IOPS, queue depth, latency with a histogram**, current vs maximum PCIe/SATA link, temperature, % life (SMART), volumes |
| **Network** (per adapter) | Send/receive, adapter, SSID, IPs, signal | Session totals, errors and drops, link and duplex, Wi‑Fi (band, channel, RSSI; this exists) |
| **GPU** (per adapter) | Selectable engines (3D, copy, video encode/decode…), dedicated/shared memory, driver, DirectX, physical location | Clocks, temperatures (hot spot, memory), power, fan, **throttle reasons**, current vs maximum PCIe link |
| **NPU** | Utilization, memory (Windows 11 24H2) | The same, wherever an API exists (`npu.rs` already detects NPUs) |

- **Layout:** a device list on the left with mini graphs (like the Task Manager), details on the right, and a compact **"summary view"**.
- **Graphs:**
  - selectable time window (1 min / 10 min / 1 h / session);
  - hover values;
  - PNG/CSV export;
  - optional temperature overlaid on load (secondary axis).

**New sources this needs** (none exist today, D7). They're built in **S1** and reused here:

| Data | Windows | Linux | macOS |
|---|---|---|---|
| Per-disk I/O | `IOCTL_DISK_PERFORMANCE` or PDH `PhysicalDisk` | `/proc/diskstats` | IOKit `IOBlockStorageDriver` *Statistics* |
| Bytes per adapter | `GetIfTable2` | `/proc/net/dev` | `sysctl NET_RT_IFLIST2` (64-bit counters) |
| Commit, pools, cache | `GetPerformanceInfo` / NtQSI | `/proc/meminfo` | `host_statistics64` (compressed, *wired*) |
| GPU engines | PDH `GPU Engine` | `fdinfo`, `gpu_busy_percent` | IOAccelerator `PerformanceStatistics` 🔬 |
| Totals: threads, handles, up time | NtQSI | `/proc/stat`, `/proc/loadavg` | `sysctl`, `libproc` |

## 5. App history

- **Windows:** the Task Manager reads SRUM (§1). SRUM keeps roughly 30–60 days of per-app CPU time, network and metered network, including from **before** CpuZeeZ was installed. Reading it, however:
  - needs admin;
  - the database (`%WINDIR%\System32\sru\SRUDB.dat`, ESE format) is **locked by a service** and has to be copied through VSS;
  - the format is complex.
- **Linux and macOS** have no accessible equivalent.

**Proposal:**
1. **1.0: our own history, cross-platform and opt-in (P2).**
   - The Metric Engine aggregates per app and per day: CPU time, GPU time, disk and network bytes (when available), run time and memory peaks.
   - It's written to local SQLite, with a default retention of 30 days (configurable).
   - **An honest limit, shown on screen:** "Covers only the time CpuZeeZ (or its tray icon) was running."
2. **After 1.0, Windows only: "Import Windows history (SRUM)"**, on demand, 🔑, read-only. 🔬 Depends on an ESE reader in Rust.
3. The UI follows the Task Manager's (a table per app with the metrics, plus "Delete usage history"). It adds:
   - a **per-day chart**;
   - a **period filter**;
   - **top consumers** (e.g. "what used the most battery this week" on laptops).

## 6. Startup apps

| Source | Windows | Linux | macOS |
|---|---|---|---|
| Per user | `HKCU\…\Run`, the user's Startup folder, packaged apps (`StartupTask`) | `~/.config/autostart/*.desktop`, enabled `systemd --user` units | `~/Library/LaunchAgents`, Login Items |
| Machine-wide | `HKLM\…\Run` (+ `Wow6432Node`), the common Startup folder | `/etc/xdg/autostart` | `/Library/LaunchAgents`, `/Library/LaunchDaemons` |
| **Enable / disable** | `…\Explorer\StartupApproved\{Run,StartupFolder}` (the Task Manager's own reversible mechanism). Machine-wide 🔑 | User: a copy in `~/.config/autostart` with `Hidden=true` (the mechanism the XDG spec provides). `systemctl --user disable` | LaunchAgents: `launchctl disable gui/<uid>/<label>` (user ✅, system 🔑). **Other apps' Login Items: ❌, no public API since macOS 13.** We show the list and a shortcut to *System Settings › General › Login Items* |
| **Impact** (High/Medium/Low) | 🔑 WDI XML files (§1) + the documented thresholds | ⚠ units: activation time over systemd's D-Bus (like `systemd-analyze blame`); XDG apps: measured by our recorder, if enabled | ❌ no data. Our recorder (opt-in) can measure CPU and disk in the first minutes after login |
| **Last BIOS time** | ✅ `FwPOSTTime` | ✅/🔑 systemd `FirmwareTimestampMonotonic` (if the bootloader reports it) or the ACPI **FPDT** table (`/sys/firmware/acpi/fpdt`) | ❌ macOS doesn't expose it |

**Improvements:**
- Extra columns: exact location (with "open in registry editor / folder"), command, publisher and signature, **date disabled**, and *impact measured over the last N boots* with its trend.
- **Boot timeline:** BIOS → kernel → login → desktop ready.
  - Windows 🔑: the `Diagnostics-Performance` log, events 100 and 101.
  - Linux: `systemd-analyze`.
  - The timeline shows who delayed the boot.
- **"Advanced" view (read-only in 1.0):** logon scheduled tasks, automatic services and shell extensions, in the spirit of Autoruns. Enabling and disabling those comes later, because of the risk.
- **Undo:** every change goes to the audit log with a "Revert" button.

## 7. Users

| Function | Windows | Linux | macOS |
|---|---|---|---|
| Session list (user, session, state, remote client) | ✅ WTS | ✅ logind (`org.freedesktop.login1`) | ✅ `utmpx` |
| CPU / memory / disk / network / GPU per user (sum of their processes) | ✅ | ✅ | ✅ |
| Disconnect / sign out | 🔑 for other users | 🔑 polkit (`TerminateSession`, `LockSession`) | 🔑 (`launchctl bootout user/<uid>`) |
| Send message | ✅ `WTSSendMessage` | ⚠ notification or `wall` | ❌ |
| Connect (switch to the session) | ✅ (credentials go to the system prompt, never to ours) | ⚠ VT switch / `loginctl activate` | ⚠ fast user switching |
| Manage accounts | Opens the system panel | Opens the desktop's settings | Opens *System Settings › Users* |

- Expanding a user shows their processes, reusing the same table.

## 8. Services (all of `services.msc` + the Services tab)

| Function | Windows (SCM) | Linux (systemd over D-Bus) | macOS (launchd) |
|---|---|---|---|
| List: name, display name, description, status, startup type, log on as, PID, group, path | ✅ | ✅ (unit, description, `ActiveState`/`SubState`, `UnitFileState`, `User=`, `MainPID`, `ExecStart`) | ✅ (`launchctl print`, plist) |
| Start (with parameters), stop, pause/continue, **restart** | 🔑 per action | 🔑 polkit **per action, straight over D-Bus** (systemd asks for authorization itself, with no app restart as root) | user domain ✅ · system 🔑 (`kickstart`, `bootout`) |
| Startup type | Automatic / **Automatic (delayed)** / **Automatic (trigger)** / Manual / Disabled | enabled / disabled / static / **masked** / indirect / *linked* | RunAtLoad / KeepAlive / disabled |
| Log on account (with password) | 🔑. The user types the password into our dialog, it goes straight to the SCM, and it's never stored | `User=` (read-only in 1.0) | read-only |
| **Recovery** (first/second/subsequent failures: restart the service / run a program / restart the computer; reset the count after N days) | 🔑 `SERVICE_CONFIG_FAILURE_ACTIONS` | read `Restart=`, `RestartSec=`, `StartLimit*`, `NRestarts` | `KeepAlive`, `ThrottleInterval` (read) |
| Dependencies (both directions) | ✅ | ✅ (`Requires`/`Wants`/`After`/`RequiredBy`) | ⚠ |
| Triggers, service SID, required privileges | ✅ read | n/a | n/a |
| **Service logs** | Event log (7031, 7034: failures) | **`journalctl -u`** built in | `log show --predicate` 🔬 |
| Live state-change notifications | ✅ `SubscribeServiceChangeNotifications` | ✅ D-Bus `PropertiesChanged` signals | ⚠ polling |

**Improvements over `services.msc`:**
- **Service health:** how often it failed or restarted recently, and its last error.
- An interactive **dependency graph** ("stopping this also stops…"). The confirmation lists the dependents **before** stopping.
- Quick filters: *Running*, *Stopped but should be running (automatic and stopped)*, *Disabled*, *Third-party* (not Microsoft/Apple/distro), *Failed recently*.
- **Protected services:**
  - critical services (a list per OS) and system-protected ones (PPL on Windows, SIP on macOS) show a lock, with an explanation of why they can't be changed;
  - we do **not** give "safe to disable" tips, because those tips break systems.
- **Undo** through the audit log (e.g. restore the previous startup type).
- Linux without systemd (OpenRC, runit): ⚠ read-only in 1.0, documented.

## 9. Settings (the Task Manager tab → *Settings › System*)

| Option (Task Manager) | In CpuZeeZ |
|---|---|
| Default start page | The **app's** start page (any page, including System › Processes) |
| Real-time update speed (High/Normal/Low/Paused) | Global, in the Metric Engine (fixes D9) |
| Always on top | Main window always on top |
| Minimize on use ("Run new task", "Switch to") | Yes |
| Hide when minimized | Integrates with the existing tray |
| Show full account name | Yes |
| Show history for all processes | A recorder option (§5) |
| Group by type / expand all | Persisted per table |
| Efficiency mode confirmation | Part of the risk-based confirmations |
| Show kernel times | In the CPU graph |
| **+ New** | Column presets, search syntax, SIGTERM→SIGKILL interval, protected-process list (editable), history retention, audit log |

## 10. Delivery (inside stage S2a)

1. `procs/` sources + Processes (Grouped/Detailed) + basic actions on the Executor + per-action elevation (B).
2. Performance (reusing the S1 sources: disk, network, GPU engines, memory composition) + details panel.
3. Services (SCM, systemd, launchd) + Startup (impact and BIOS time).
4. Users + App history (opt-in recorder) + Settings.
