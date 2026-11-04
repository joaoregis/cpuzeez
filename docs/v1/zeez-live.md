# ZeeZ Live: overlay, capture and reports

> Part of the [1.0 plan](README.md). Status: design. 🔬 items must be proven before they're in scope.

## 1. The brand and its parts

**ZeeZ Live** is the brand for everything about watching performance *while you use the PC*: games, renders, exports, compiles. A brand is only worth something if people understand it in seconds. So the name is fixed, and **the parts inside it use plain, universal words**:

| Part | What it is | Works without the others? |
|---|---|---|
| **Overlay** | The HUD above a game or app: FPS, temperatures, clocks, whatever the user picks | Yes |
| **Capture** | Records a session (frames + hardware): manual, timed, or automatic per game | Yes. You can record with no overlay, e.g. with the game on another monitor |
| **Reports** | Analyses and compares sessions: average, 1%/0.1% lows, frame-time curve, throttling events | Yes, including CSV imported from other tools |

**In the app:** a "ZeeZ Live" sidebar entry with three tabs, **Live** (dashboard), **Overlay** (styles, editor, hotkeys) and **Sessions** (capture library, reports, compare). Internally the module is `live`.

### 1.1 Brand consistency
- The name is always written **ZeeZ Live**. It is not translated in any language; the parts' names are.
- The indicators are always the same:
  - **● LIVE**: overlay showing;
  - **● REC**: capturing, in red, everywhere: overlay, tray, window title.
- The icon is a pulse line, with the brand violet as accent, reused from the design tokens in `theme.rs`.

### 1.2 UX for instant understanding
- **First use in 3 steps:**
  1. Pick a **style** (§5.2), shown as live previews.
  2. Confirm the **hotkey**.
  3. Press **"Try it now"**: the overlay appears over the desktop so you see it before launching a game.
  
  That's it, and it works with defaults.
- **Defaults that just work:** *Compact* style, top-left corner, `Ctrl+Shift+O`, auto-target the foreground game.
- **Progressive disclosure:** the styles cover most people. The full layout editor sits behind **"Customize"**.
- **Every metric explains itself.** A one-line, plain-language tooltip, e.g. "**1% low**: how smooth the worst moments feel. Higher is better." There's also a "Learn more" link to an in-app glossary (offline).
- **Empty states teach.** "No game detected yet. Start a game, or pick an app from the list. FPS needs administrator rights on Windows. [Restart as administrator]"
- **The overlay never surprises.** It never takes focus, never captures clicks (outside edit mode), and always disappears with the same hotkey.

## 2. Who it's for

| Persona | Main use | Needs |
|---|---|---|
| Gamer | See FPS and whether the PC is "struggling" | A minimal, discreet style, hotkey on/off, never steals focus |
| Benchmarker / reviewer | Run the same pass N times and compare | Timed capture, 1%/0.1% lows, frame times, CSV/HTML export, comparison |
| Overclocker / enthusiast | Validate stability and limits | Effective clocks, voltages, power, throttling flags, peaks |
| Professional | Understand a render or export | No FPS: per-process CPU/GPU, VRAM, I/O, duration |
| PC builder | Burn-in and a report for the customer | A long-session report with temperatures, throttling and stability |

## 3. How it works (and where the limits are)

The overlay solves **three independent problems**. Each one has different technical limits on each OS.

### 3.1 Measuring the game's frames

**How a game makes frames.** For every frame the game asks the system to show the image:
- `Present` (DirectX);
- `vkQueuePresentKHR` (Vulkan);
- `SwapBuffers` (OpenGL);
- `present` (Metal).

FPS is how many of those requests happen per second. Frame time is the gap, in milliseconds, between one request and the next.

**Why average FPS misleads.** 60 FPS with steady 16.7 ms frames feels smooth. 60 FPS made of 10 ms frames with 50 ms spikes in between feels choppy. So we measure:
- **frame time** for every frame;
- the **1% low** and **0.1% low**, which describe the worst moments;
- **stutters**, frames far above the median.

**Definition of 1% low.** The industry uses two conventions:
- the 99th-percentile frame time, converted to FPS;
- the average of the slowest 1% of frames.

Mixing them confuses comparisons. We use the **percentile** by default, offer the other as an option, and **always state which one a report uses**.

**Windows: ETW and PresentMon.**
- Windows logs every `Present` from every process to ETW providers (`Microsoft-Windows-DxgKrnl`, `-DXGI`, `-D3D9`, `-Win32k`).
- **PresentMon** (Intel, MIT license) consumes those events. It is the engine behind Intel PresentMon, CapFrameX and similar tools.
- Per frame it gives:
  - time between presents and between display changes;
  - **GPU Busy**, the time the GPU actually worked. It is essential for the [bottleneck detector](bottleneck-detector.md);
  - display latency;
  - present mode (*independent flip* vs *composed*);
  - dropped frames;
  - frame type, which separates **generated** frames (DLSS/FSR Frame Generation) from rendered ones.
- It covers DX9–DX12, Vulkan and OpenGL without touching the game process, because the events come from the graphics kernel. **That isn't injection**, so it's anti-cheat safe.
- **Limit:** a real-time ETW session requires **administrator** (or the "Performance Log Users" group). That is why privilege option A now and the helper C later exist ([README §4.3](README.md)).
- **Q3, decided:**
  1. For 1.0, bundle PresentMon's console executable as a *sidecar* (MIT, Intel-maintained and Intel-signed, read through `stdout`; see [dependencies.md](dependencies.md)). The user installs nothing.
  2. Later, evaluate a native Rust ETW consumer (`ferrisetw`) to drop the extra binary.
- 🔬 **SP1:** measure PresentMon's overhead; validate capture on DX11, DX12, Vulkan and OpenGL, with and without Frame Generation.

**Linux: there is no "ETW".**
- Neither X11 nor Wayland exposes another process's frame timing. That's why the reference tool, **MangoHud** (MIT), runs as a **Vulkan layer** plus an **OpenGL preload**, enabled with `mangohud %command%` in Steam or with `MANGOHUD=1`.
- **Q4, decided for 1.0**, with every step aimed at less friction:
  1. **Detect** MangoHud (distro package or the Flatpak Steam extension). If it's missing, a one-click **"Install"** runs the right package manager after confirmation ([dependencies.md §3](dependencies.md)).
  2. **Enable it without editing every game.** One switch, "Enable for all games", writes `MANGOHUD=1` to `~/.config/environment.d/cpuzeez-mangohud.conf`. It's opt-in, applies from the next login, and a switch removes it. Per game, the alternative is a **"Copy launch option"** button (`mangohud %command%`), with the Steam steps shown. We don't edit Steam's own config files, because Steam overwrites them while it runs.
  3. **Generate its config from the layout chosen in CpuZeeZ**, so the in-game HUD follows the user's design.
  4. **Ingest its logs** (`output_folder`, `log_interval`) for Capture and Reports.
  5. Show it all on one **status line**: "MangoHud: installed ✓ · enabled for all games ✓ · last game seen: cs2".
- **After 1.0:** our own minimal Vulkan layer (`VK_LAYER_CPUZEEZ_frametime`). It only timestamps `vkQueuePresentKHR` into a shared-memory ring and draws nothing.
- Anti-cheat under Proton generally tolerates layers like MangoHud, but **with no guarantee**, and we document that.
- 🔬 **SP5:** can MangoHud data be read live, not only at the end, and with what latency?

**macOS: the most restricted.**
- There is no public API for another app's frame timing.
- **Metal Performance HUD**: Apple's own tool (macOS 13+), enabled by an environment variable or a developer setting. Apple draws it, not us. We can offer "Launch game with the Metal HUD". 🔬 **SP6:** is there a log we can consume?
- **Displayed FPS by screen capture (experimental):**
  - ScreenCaptureKit reports when the game window's content changes. Counting those changes gives the **FPS delivered to the screen** (not the rendered FPS).
  - It works with any API.
  - It needs the "Screen Recording" permission, and the request explains why.
  - It has a cost we must measure.
  - The same technique is a fallback on Linux (PipeWire) and Windows (Windows.Graphics.Capture).
- In short, macOS 1.0 gets the full **hardware** overlay, FPS labelled "experimental", and a Metal HUD shortcut.

### 3.2 Drawing on top of the game

| Technique | How | Pros | Cons |
|---|---|---|---|
| **External window**: transparent, always on top, click-through (**proposed**) | Our own window above the game's | **No injection, anti-cheat safe**, works with any API | Not visible in **true exclusive fullscreen** (below) |
| Injection / `Present` hook (RTSS, Steam, Discord) | A DLL inside the game draws the HUD | Works in every display mode | **Ban risk** (EAC, BattlEye, Vanguard, FACEIT), crashes, per-API code. **Rejected (P4)** |
| Xbox Game Bar widget (Windows only) | A UWP widget inside the system overlay | Safe, visible in fullscreen | Windows only, a separate UWP project. Evaluate after 1.0 |

**What "exclusive fullscreen" is, and why it matters.**
- In the past, a fullscreen game owned the whole screen and no other window could show on top.
- Today most DX11/DX12 games use the *flip model* with "fullscreen optimizations" (FSO). The game **looks** exclusive, but the Windows compositor (DWM) can still put windows on top. The external overlay works there, and in **borderless windowed**.
- It does not work in "true" exclusive mode (old DX9, FSO turned off, some Vulkan titles). We detect that from PresentMon's present mode and show: "This game is in exclusive fullscreen; switch to *borderless window* to see the overlay. Capture still works."
- A window on top may drop the game out of *independent flip* (slightly more latency). 🔬 **SP2:**
  - measure latency and FPS with and without the overlay on DX11 exclusive/FSO, DX12 borderless and Vulkan;
  - check whether MPO planes avoid the loss.

**Per-OS details.**

| | How to build the window | Limits |
|---|---|---|
| **Windows** | `WS_EX_LAYERED \| WS_EX_TRANSPARENT \| WS_EX_TOPMOST \| WS_EX_NOACTIVATE \| WS_EX_TOOLWINDOW`: transparent, no clicks, no focus, not in Alt-Tab | Exclusive fullscreen (above). An option **"Hide from recordings/streams"** (`SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)`), or the reverse for streamers who want the HUD on stream |
| **macOS** | `NSWindow` above normal level, `ignoresMouseEvents`, `collectionBehavior = canJoinAllSpaces \| fullScreenAuxiliary \| stationary` | Fullscreen apps live in their own *Space*. winit's `AlwaysOnTop` isn't enough, so the native window needs tuning via `objc2`. 🔬 **SP3** |
| **Linux X11** | `_NET_WM_STATE_ABOVE` + an empty input region (click-through) | Works well |
| **Linux Wayland** | **No app can force itself above another or pick a global position.** That is a deliberate security choice of the protocol. `wlr-layer-shell` exists (KDE, Sway, Hyprland), but **GNOME doesn't support it** | External overlay only where layer-shell exists 🔬 **SP4**. Elsewhere, the path is **MangoHud's HUD** with our layout (§3.1) |

**Feasibility with egui/eframe.**
- eframe 0.36 already supports undecorated, transparent, click-through, always-on-top windows (`ViewportBuilder::with_transparent / with_mouse_passthrough / with_window_level / with_decorations`).
- The overlay is a **deferred viewport** with its own repaint cycle. It keeps working with the main window hidden in the tray (today's mini monitor is *immediate* and depends on the main window).
- It repaints **only when a value changes**: text at 1–10 Hz, the frame-time graph at up to 30 Hz (configurable).
- Renderer: glow, falling back to wgpu ([ADR 0013](../adr/0013-gui-renders-with-glow-and-falls-back-to-wgpu.md)). On Windows, glow plus click-through needs one `SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA)` call after the window opens. Without it the window is invisible.

### 3.3 Collecting hardware data without hurting the game

- Everything comes from the **Metric Engine** ([README §4.1](README.md)) through subscriptions, so the overlay asks only for the metrics it shows.
- **Prerequisite (S0):** remove the clock busy-spin (D1). Today it would take about 0.5 s of CPU per sample during the game.
- The game's **per-process** data (CPU, threads, memory, VRAM, I/O) comes from the System hub (S2a), read **without strong handles** on the game:
  - `NtQuerySystemInformation` and PDH counters;
  - at most `PROCESS_QUERY_LIMITED_INFORMATION`.

  That matters for anti-cheats that watch who opens their process.
- **Overlay target:**
  - automatic: the process that owns the foreground window and is presenting frames;
  - manual: picked from a list.

## 4. Metric catalog

| Category | Metric | Windows | Linux | macOS |
|---|---|---|---|---|
| **Frames** | FPS, frame time, 1%/0.1% low, frame-time graph | 🔑 PresentMon | ✅ MangoHud | ⚠ 🔬 |
| | GPU Busy / GPU Wait, display latency, present mode, generated frames (FG), dropped frames | 🔑 PresentMon | ⚠ (some via MangoHud) | ❌ |
| **GPU** | Overall load; per-engine load (3D, copy, encode, decode) | ✅ PDH `GPU Engine` | ⚠ amdgpu/i915/xe/NVML | ⚠ IOAccelerator 🔬 |
| | Temperature, hot spot, memory junction, clocks, power, fan, throttle reasons | ✅ NVML/NVAPI/ADL (exists) | ✅ NVML/sysfs (exists) | ⚠ SMC |
| | VRAM total/used **and the game's** | ✅ PDH `GPU Process Memory` | ⚠ `fdinfo` (`drm-memory-*`) / NVML | ⚠ 🔬 |
| **CPU** | Total load, **busiest core**, game process load and its heaviest thread | ✅ | ✅ | ✅ (no affinity) |
| | Effective clock (avg/max), temperature, package power, throttling | 🧩 | ✅/🔑 | ⚠ |
| **Memory** | RAM used/available, game working set, hard faults/s (paging) | ✅ | ✅ | ✅ |
| **Disk/Network** | Game I/O (MB/s), adapter throughput | ✅ | ✅ | ✅ |
| **System** | Clock, session duration, battery/power source, ● REC indicator | ✅ | ✅ | ✅ |

**Server ping:** out of scope, because it would mean sending packets (P1). If it ever ships, it is opt-in and the user picks the destination.

## 5. UX and UI

### 5.1 Surfaces
1. **Overlay:** the HUD itself.
2. **ZeeZ Live › Live:** an in-app dashboard with target, FPS, graphs and a capture button. Useful with the game on a second monitor.
3. **› Overlay:** styles, the layout editor (§5.3) and hotkeys.
4. **› Sessions:** capture library, reports and comparison (§6).

### 5.2 Built-in styles (all editable)

| Style | Contents | Shape |
|---|---|---|
| **Minimal** | FPS | One number in a corner |
| **Compact** (default) | FPS · 1% low · GPU °C/% · CPU °C/% · RAM | One horizontal line |
| **Detailed** | Frames / GPU / CPU / Memory groups with values and bars | Vertical column |
| **Benchmark** | FPS, 1%/0.1% low, frame-time graph, GPU Busy, busiest core, ● REC + timer | Vertical, with graph |
| **Overclocker** | Effective clocks, voltages, power (PPT/PL), temperatures, throttling pills | Grid |

### 5.3 Layout editor
- **WYSIWYG:** live preview on sample backgrounds (light, dark and a sample game scene), because legibility depends heavily on what's behind the text.
- **Widgets:**
  - value;
  - label + value;
  - bar;
  - sparkline;
  - **frame-time graph** with spikes highlighted;
  - histogram;
  - status pill (throttling, bottleneck);
  - group header;
  - separator;
  - clock/timer.
- **Style:**
  - scale and font, with **tabular** figures so numbers don't jitter;
  - colour per metric and **per value range** (e.g. temperature green/amber/red relative to Tjmax), always paired with an icon or text;
  - background colour and opacity, corner radius, padding;
  - **text outline/shadow** for legibility over any scene;
  - horizontal or vertical.
- **Position:**
  - 9 anchors + offset, or drag in **edit mode**: a hotkey makes the overlay interactive, and Esc returns it to click-through;
  - per monitor, DPI-aware.
- **Refresh rate:** separate for text and for graphs.

### 5.4 Per-game profiles and automation (opt-in, P2)
- A profile per executable, with its own layout and position.
- "Show the overlay automatically when this game starts."
- "Capture automatically": off by default, with the ● REC indicator.

### 5.5 Global hotkeys

| Action | Suggested default |
|---|---|
| Show/hide overlay | `Ctrl+Shift+O` |
| Cycle style | `Ctrl+Shift+L` |
| Start/stop capture | `Ctrl+Shift+R` |
| Marker during capture ("it stuttered here") | `Ctrl+Shift+M` |
| Overlay edit mode | `Ctrl+Shift+E` |

- All of them are configurable, and the app detects conflicts.
- **Per OS:**
  - Windows: `RegisterHotKey`.
  - macOS: `RegisterEventHotKey`, which doesn't need the Accessibility permission.
  - X11: `XGrabKey`.
  - **Wayland:** only through the *GlobalShortcuts* portal (KDE; GNOME 48+). Elsewhere, the fallback is the tray menu.

## 6. Capture and reports

**Capture modes:**
- **manual** (hotkey or button);
- **timed** (e.g. 60 s with a 3 s countdown, the usual benchmark practice);
- **automatic per game** (opt-in).

**What is recorded** (local, P1), in `sessions/<yyyy-mm-dd_hhmm>_<game>/`:

| File | Contents |
|---|---|
| `frames.csv` | One frame per row, **with PresentMon's columns**. It opens in CapFrameX and other tools |
| `hw.csv` | Hardware metrics, in the existing `telemetry` CSV format |
| `session.json` | Game, layout, system configuration (CPU/GPU/RAM/driver/resolution when available), markers, summary |

**The report** opens when capture stops, and can be exported as self-contained offline HTML or as PNG:
- **Summary:** average FPS, 1%/0.1% lows (convention stated), min/max, stutter count, % of time per [bottleneck](bottleneck-detector.md) class.
- **Time-aligned charts:** frame time, FPS, GPU/CPU load and temperature, clocks, power. Markers and throttling events appear as bands.
- **Frame-time percentile curve** and **FPS histogram**.
- **Comparison** of up to N sessions (e.g. before/after an overclock or a driver update): bars for average/1%/0.1% and overlaid curves.
- **Validity:**
  - a warning when sessions aren't comparable (different game, resolution or driver);
  - a warning when variance between repeats is high: "run it 3× and use the median".

## 7. Safety, anti-cheat and privacy

- No injection, no reading the game's memory, handles only with `PROCESS_QUERY_LIMITED_INFORMATION`.
- ETW/PresentMon is what official tools use (Intel PresentMon, NVIDIA FrameView), and it never touches the process.
- PawnIO is a signed driver and isn't on Microsoft's vulnerable-driver blocklist. 🔬 **SP7:** check with common kernel anti-cheats (Vanguard, FACEIT) whether a loaded PawnIO causes blocks. If it does, the overlay must run in "no PawnIO" mode (GPU/OS data only) and say so.
- Nothing is recorded without opt-in, and ● REC is always visible on the overlay while capturing.

## 8. Per-OS limits (honest summary)

| | Windows | Linux | macOS |
|---|---|---|---|
| Hardware overlay | ✅ | ✅ X11 · ⚠ Wayland (layer-shell or MangoHud) | ✅ 🔬 fullscreen Spaces |
| FPS/frame times | 🔑 PresentMon | ✅ with MangoHud enabled | ⚠ experimental (capture) / Metal HUD |
| GPU Busy / FG / latency | 🔑 | ⚠ | ❌ |
| Visible in true exclusive fullscreen | ❌ (warning + borderless suggestion; capture still works) | ✅ X11 | n/a |
| Global hotkeys | ✅ | ✅ X11 · ⚠ Wayland (portal) | ✅ |

## 9. Spikes before implementing

| # | Question | Success criterion |
|---|---|---|
| SP1 | PresentMon *sidecar*: overhead and coverage | < 0.5% CPU; correct frames on DX11/DX12/Vulkan/OpenGL; FG identified |
| SP2 | External window over games on Windows | Display-mode × visibility × latency-impact matrix |
| SP3 | macOS over fullscreen apps | Overlay visible in the game's Space, without stealing focus |
| SP4 | Wayland layer-shell with winit/eframe | Proof of concept on KDE and Hyprland, or a decision to use MangoHud only |
| SP5 | MangoHud live data | Data latency < 1 s |
| SP6 | Metal HUD log | Does it exist, and can we consume it? |
| SP7 | PawnIO × kernel anti-cheats | A documented compatibility list |
| SP8 | GPU cost of the overlay (glow vs wgpu) | < 0.3 ms of GPU per game frame |

## 10. Delivery (inside stage S2b)

1. Overlay window + styles + hotkeys + first-use flow, with hardware metrics on all OSes.
2. Frame sources: PresentMon (Windows), MangoHud (Linux), experimental macOS.
3. Capture + reports + comparison.
4. Per-game profiles and automation; bottleneck integration.
