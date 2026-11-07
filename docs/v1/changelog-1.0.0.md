# CHANGELOG entry for `1.0.0`, consolidated (draft)

> To paste into [CHANGELOG.md](../../CHANGELOG.md) when `1.0.0` is tagged (S3-08), above the beta entries, which stay below it. It says what 1.0.0 is for someone coming from `0.3.0`, not release by release. Edit the lines marked ⚠ against what the release candidates settle.

## [1.0.0] - YYYY-MM-DD

CpuZeeZ 1.0: the CPU-Z feature set, a hardware monitor, a Task Manager, a game overlay and a bottleneck detector, on Windows, Linux and macOS. Built in the open over `1.0.0-beta.1` to `beta.8` and the release candidates.

### New since 0.3.0

- **ZeeZ Live**: an overlay in its own window (nothing injected into any game), FPS and frame times (PresentMon on Windows, MangoHud on Linux, Metal HUD on macOS, experimental), 1% and 0.1% lows, GPU Busy, capture to `frames.csv` / `hw.csv` / `session.json`, reports, comparison, global hotkeys (Wayland through the desktop's portal), per-game profiles, an anti-cheat safe mode, per-monitor placement, throttling pills.
- **Bottleneck detector**: what limits the frame rate, live and per session, with the numbers behind each verdict; no rankings, no guesses.
- **System**, a Task Manager: Processes, Performance, App history (opt-in), Startup, Users and Services, with per-process network, power and GPU, reversible changes, and a warning on anything critical to the system.
- **Benchmarks**: CPU suite, memory (latency, bandwidth, GUPS, TLB), GPU (DX12/Vulkan/Metal), storage, core-to-core latency, a monitored stability test, reproducible runs with history and comparison.
- **Devices and health**: SMART with thresholds and self-tests, a read-only surface scan, drive endurance, partition alignment, TRIM, encryption, USB/Thunderbolt/audio/Bluetooth, battery health, network adapters.
- **A new interface**: Overview with build health and alerts, a command palette, detail levels, pins, units, tray icons, mini monitors, light and dark themes, English, Português (Brasil) and Español.
- **Install and update**: MSI with an optional PawnIO driver (upgrade in place, downgrade refused, clean removal), `.deb`, Arch package, macOS DMG/PKG, portable zip; one-click verified updates on a channel you choose (Stable, Beta, Alpha).

### Principles it keeps

- **Local only.** No telemetry, no account. The single network call is the update check, off until you allow it; recording is off until you allow it.
- **Privileges per action.** The app starts as a normal user; what needs rights asks for them for that one action, in a one-shot copy that accepts only a fixed list of operations.
- **No injection.** Games are never hooked or read; kernel-level access is PawnIO only, and stops while an anti-cheat game you chose is running.

### Quality

- Parsers and the detector run on random and corrupt input on every push; 11 fuzz targets run on a schedule; a synthetic-game soak checks memory, handles and threads over hours.
- Accessibility checked by test: AA contrast, scale 75–200%, keyboard, no colour-only signals. A threat model, a security policy and an audit guide are in `docs/`.
- ⚠ Performance budget (2% of a core, 150 MB, ≤ 1% FPS cost): the measured numbers are in [docs/performance.md](../performance.md); copy the final ones here.

### Known limits

- ⚠ Releases are unsigned; Windows and macOS warn on first run ([ADR 0009](../adr/0009-releases-stay-unsigned-for-now.md)). Every release carries `SHA256SUMS.txt`.
- ⚠ Games in true exclusive fullscreen hide the overlay (capture still works). Linux and macOS desktop behaviour was checked by CI and testers, not exhaustively. See [FEATURES.md](../../FEATURES.md) for the per-OS tables.
