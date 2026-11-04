# Dependencies and install friction

> Part of the [1.0 plan](README.md). Principle **P8: zero install friction.** Whatever can be compiled into the binary is compiled in. Whatever must live outside it is offered by the installer or an in-app button, with consent. Nothing is installed silently, and the user is never left to hunt for anything.

## 1. Two kinds of dependency (and why "feature flag" doesn't mean "install")

| Kind | What it is | Does the user install anything? | What it costs us |
|---|---|---|---|
| **Build-time (Rust crates)** | Code compiled **into** `cpuzeez.exe` / `cpuzeez` | **No.** It's inside the single binary | Binary size, build time, supply-chain trust (we run their code), license obligations |
| **Runtime components** | Things **outside** our binary that must exist on the machine: kernel drivers, OS tools, GPU-vendor libraries, third-party programs | Possibly. This is where friction comes from | Detection, packaging, license terms, "what if it's missing" UX |

A **Cargo feature flag** (for example `wgpu` behind `gpu-bench`) is a **compile-time** switch for *us*:
- **official release builds turn it on**, so the user gets the feature with nothing to install;
- it keeps day-to-day dev builds fast;
- it lets distro packagers drop a heavy part if their policy requires it.

So "behind a feature flag" never means "the user has to install something".

## 2. Build-time dependencies

### 2.1 Today (0.3.0)

| Crate | Why | License | Notes |
|---|---|---|---|
| `serde`, `serde_json` | Reports, settings, history | MIT/Apache-2.0 | |
| `libc` (unix) | POSIX/Linux/macOS syscalls | MIT/Apache-2.0 | |
| `eframe`/`egui` 0.36 (glow) | GUI | MIT/Apache-2.0 | Renders through **OpenGL** (see §3, CRT and OpenGL) |
| `egui-phosphor` | Icons | MIT (icons and crate) | |
| `png` | F5 screenshots, icons | MIT/Apache-2.0 | |
| `tray-icon` (Win/mac), `ksni` (Linux) | Tray | MIT/Apache-2.0 | `ksni` is pure Rust D-Bus, with no system libraries |
| `winresource` (build only) | Exe icon and version info | MIT | Not shipped |

The release binary is **10.7 MB** (Windows x64, thin LTO, stripped).

### 2.2 Proposed for 1.0

| Crate | Why | User friction | Our cost | Recommendation |
|---|---|---|---|---|
| **`windows-sys`** | Official Microsoft bindings, generated from Windows metadata. The Task Manager alone needs 100+ new Win32 APIs (SCM, WTS, PDH, NtQSI, Shell, WinTrust…). Hand-written `extern` blocks get types wrong silently | **None.** Bindings only; no runtime, no DLL | Slightly longer builds. Only the APIs we use end up in the binary | ✅ Adopt in `platform/windows` only. Migrate old `extern`s when touched |
| **`rusqlite`** with the `bundled` feature | App history and boot history: queries by day, app and metric, plus retention | **None.** SQLite (public domain) is compiled into the binary, with no system SQLite needed | About +1 MB; needs a C compiler at **build** time (CI has one) | ✅ Adopt. The alternative is append-only CSV/JSON plus hand-written aggregation, which is more code and slower queries |
| **`wgpu`** | GPU benchmark (compute: FP32/FP16, VRAM bandwidth, PCIe host↔GPU). It talks to DX12/Vulkan/Metal, which ship with every GPU driver | **None** | Several MB of binary and noticeably longer builds | ✅ Adopt behind `gpu-bench` (on in releases). **Spike:** also use it as eframe's renderer (§3) |
| **`global-hotkey`** | ZeeZ Live hotkeys on Win/mac/X11 | None | Small | ✅ |
| **`objc2` + framework crates** (macOS) | Overlay window level and Spaces behaviour, ScreenCaptureKit (experimental FPS) | None | Small, macOS only | ✅ when needed |
| `ferrisetw` | Native ETW consumer (to replace the PresentMon sidecar later) | None | Medium | ⏳ After 1.0 |
| `egui_kittest`, `cargo-fuzz`, `criterion`-style benches | Tests only | **Not shipped** | Dev time | ✅ dev-dependencies |

**Rules for adding a crate** (enforced by `cargo-deny` and an ADR):
- **Licenses we link:** MIT, Apache-2.0, BSD, ISC, Zlib, Unicode and MPL-2.0 (file-level copyleft). **No GPL or LGPL** inside our binary, so the project stays MIT OR Apache-2.0.
- **Justification:** the problem it solves, the std alternative, size, maintenance and security track record.
- **Pinning:** versions are pinned by `Cargo.lock`, and advisories are checked in CI.

## 3. Runtime components: what we bundle, what we offer, and what's already there

| Component | Needed for | Today | 1.0 proposal | Why this way |
|---|---|---|---|---|
| **Visual C++ runtime** (`VCRUNTIME140.dll`) | **The app starting at all** on Windows | ⚠ **Required but not shipped.** The 0.3.0 release exe imports it. It fails to start on a clean Windows install or in Windows Sandbox without the VC++ redistributable | ✅ **Link the CRT statically** (`-C target-feature=+crt-static` for `*-windows-msvc`). One line in `.cargo/config.toml`, about 100 KB | Removes a whole class of "it doesn't open" reports. Verify in CI by running on a clean Windows image |
| **OpenGL driver** | The GUI (eframe's glow renderer) | Comes with every GPU driver; Mesa on Linux | 🔬 **Spike:** switch eframe to its **wgpu renderer**, which we're adding anyway | glow fails inside VMs, Remote Desktop and machines without a GPU driver (Windows exposes only OpenGL 1.1 there). wgpu has DX12/Vulkan/Metal and **WARP software rendering** on Windows. Apple also deprecated OpenGL on macOS. One dependency solving two problems. Decide on overlay transparency support and startup cost |
| **PawnIO driver** (Windows) | Sensors, SPD, timings, MSRs | ⚠ The user installs it manually (pawnio.eu or winget) | ✅ **Bundle the official signed installer** (redistribution of the installer is allowed, per [windows-pawnio.md](../windows-pawnio.md)). The MSI shows an **optional, explained checkbox**, "Hardware access (PawnIO)", that runs it with `-install -silent`. The portable zip gets an in-app **"Install hardware access"** button running the same bundled installer (UAC prompt) | The driver itself is proprietary-but-redistributable as an installer, and it's shared with LibreHardwareMonitor and FanControl, so **uninstalling CpuZeeZ never removes it**. We say so and link to its own uninstaller |
| **PawnIO modules** (LGPL-2.1 blobs) | Same | ⚠ The user copies `.bin` files into `pawnio-modules\` | ✅ **The release workflow** downloads the pinned PawnIO.Modules release, **verifies its SHA-256**, and ships the modules inside the MSI and zip, together with `COPYING` and a note naming the source tag | LGPL allows redistribution with the license and a pointer to the source. They stay **out of the repo**, which stays single-licensed, and **in the packages**, so the user doesn't have to copy anything |
| **PresentMon** (Intel, MIT) | ZeeZ Live FPS on Windows | — | ✅ **Bundle** the pinned, Intel-signed console executable plus its license, SHA-256-checked in CI | MIT, small, maintained by Intel. It runs as a separate process |
| **curl** | Update check and download | Ships with Windows 10 1803+, macOS and most desktop Linux | ✅ Keep it. If it's missing, the update banner says so | A Rust HTTP+TLS stack would add 2–3 MB and certificate handling to our binary for a feature that is opt-in |
| **GPU vendor libraries** (NVML, NVAPI, ADL) | GPU sensors | Installed with the GPU driver | ✅ Nothing to do | If you have the GPU, you have its driver. These libraries are not redistributable anyway |
| **smartmontools** (`smartctl`, GPL-2.0) | Optional SMART fallback: USB bridges, macOS | Used if installed | Not bundled. Linux packages list it as `Suggests`/`optdepends`; on macOS we mention Homebrew. Native NVMe and ATA paths cover internal drives | GPL is fine as a separate program, but bundling it brings source-offer obligations for little gain |
| **MangoHud** | ZeeZ Live FPS on Linux | — | Not bundled. `.deb` `Recommends`, PKGBUILD `optdepends`, plus **in-app detection** with an **"Install" button**. It runs the distro's package manager through `pkexec` after confirmation (`update.rs` already uses `pkexec` the same way). For **Flatpak Steam**, the button installs the `org.freedesktop.Platform.VulkanLayer.MangoHud` extension instead. Enabling it is covered in [zeez-live §3.1](zeez-live.md) | A Vulkan layer has to be registered system-wide, with 32- and 64-bit libraries that match the distro. Only the distro package (or the Flatpak extension) does that correctly |
| **Linux kernel modules** `msr`, `i2c-dev` | MSRs, SPD | In-tree, but not always loaded | In-app **"Enable"** button (`pkexec modprobe …`) with an explanation, and an optional "keep enabled after reboot" (writes `/etc/modules-load.d/cpuzeez.conf`, opt-in, removable) | Loading kernel modules is a system change. It needs consent, it's never silent, and it's reversible |
| **`ryzen_smu`** (out-of-tree, GPL) | AMD SMU table on Linux | Optional | Not bundled. Documented, with links to the distro/AUR/DKMS packages | It's a kernel module that must match the running kernel |
| `vulkaninfo`, `glxinfo`, `iw` (Linux) | Extra details only | Used if installed | `Suggests` in packages | Enrichment only, never required |
| macOS system tools (`ioreg`, `system_profiler`, `sysctl`, `networksetup`…) | Inventory, sensors | Part of macOS | ✅ Nothing to do | — |

### 3.1 The install friction that isn't a dependency

These aren't libraries, but they stop people from installing the app, so they belong here:

| Friction | Today | Proposal |
|---|---|---|
| **Windows SmartScreen** ("Windows protected your PC") on unsigned binaries | Unsigned | ❌ **No paid signing for now** (Q9: no budget). See the zero-cost mitigations below |
| **macOS Gatekeeper** on ad-hoc-signed apps | Ad-hoc signed, with manual "Open Anyway" steps (README) | ❌ **No Developer ID / notarization for now** (Q9). See the zero-cost mitigations below |
| Admin prompt at every launch for full sensors | Manual "run as administrator" | Restart-as-admin button (1.0, A); the optional helper service after 1.0 (C) |

**Zero-cost mitigations for unsigned builds (Q9):**
- **Explain it up front.** Both READMEs and the release notes say why the builds are unsigned (a paid certificate, which the project can't fund yet), what the SmartScreen/Gatekeeper warning means, and the exact steps to continue.
- **Let users verify.** Every release already publishes `SHA256SUMS.txt`. The README shows how to check a download with `Get-FileHash` / `shasum -a 256`, and the in-app updater already verifies it ([update.rs](../../src/update.rs)).
- **Reproducible-from-source CI.** The release workflow builds everything on GitHub's runners from the tagged commit, and the run's logs are public, so anyone can see that a binary came from the code.
- **Free signing for open-source projects (evaluate).** The SignPath Foundation offers free Authenticode signing to OSS projects that meet its criteria. 🔬 Check eligibility. If accepted, the SmartScreen warning fades as the certificate builds reputation. There's no free equivalent for Apple notarization.
- Package managers that don't require signing: winget manifests, Scoop, the AUR and Homebrew casks. 🔬 Check each one's current policy for unsigned binaries before promising it.

## 4. Rules for every runtime component

1. **Prefer compiled-in.** A runtime component is acceptable only if it **must** live outside our process (a kernel driver, a system-registered layer, an OS tool), or if its license forbids linking it (GPL).
2. **Bundled third-party binaries** must be:
   - pinned to a version;
   - **SHA-256-verified in CI**;
   - shipped with their license text;
   - listed in *About › Third-party components* and in `THIRD_PARTY.md`.
3. **Never silent.** Installation happens through an installer checkbox or an in-app button. The prompt explains what the component is, why we need it, who makes it, and how to remove it, and the OS consent prompt (UAC, polkit, macOS authorization) follows.
4. **Absence is explained.** When a component is missing, the [availability model](engineering.md) shows exactly what's unavailable, with a one-click fix where one exists (P3).
5. **Uninstall is clean.** Our uninstaller removes whatever we created (tasks, rules, module-load entries) but **not shared components** (PawnIO), and it says so.

## 5. Decisions

All decided. The rule (Q6): pick whatever solves the problem best **while removing friction for the user**.

| # | Question | Decision |
|---|---|---|
| Q6a | `windows-sys`, `rusqlite` (bundled) and `wgpu` (behind `gpu-bench`, on in releases) | ✅ Adopt all three |
| Q6b | Static CRT on Windows | ✅ S0. This one is a bug fix, not a choice |
| Q6c | Spike: eframe on wgpu instead of glow | ✅ Run it in S0. If transparency and the overlay work on all three OSes, switch |
| Q6d | Bundle the PawnIO installer and modules, and PresentMon | ✅ Yes, as described in §3 |
| Q9 | Code signing (Windows) and notarization (macOS) | ✅ **Not for now** (no budget). The zero-cost mitigations in §3.1 apply, including the README notice |
