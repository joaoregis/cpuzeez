# Audit guide

For whoever audits CPU ZeeZ 1.0, human or AI. It says what the project claims, where to look, how to reproduce the evidence, what is known to be weak, and what cannot be checked from the repository at all. It does not hide limits: a finding that matches a known gap is still a finding, but it is not news.

Audit scope: **code, architecture, design, UI, UX, CI, CD, pipelines, vulnerabilities, safety.**

Snapshot: written against `1.0.0-beta.10` (branch `s0-foundation`). When the code moves on, trust the code and the tests over this page, and tell us what drifted.

## 1. What the project is

A cross-platform (Windows, Linux, macOS) hardware and system monitor in Rust: CPU-Z-style inspection, a Task Manager under a *System* hub, and *ZeeZ Live* (overlay, FPS, capture, a bottleneck detector). One binary: GUI by default, CLI with flags (`-txt`, `-json`, `-sensors`, `-log`, `-checkupdate`, …). The product is named **CPU ZeeZ**; the command, files, install folder and data folder keep the old spelling `cpuzeez` / `CpuZeeZ` on purpose (see §3, naming).

The plan and its rules are in [docs/v1/README.md](../v1/README.md) (principles **P1–P8**), the work list in [docs/v1/ROADMAP.md](../v1/ROADMAP.md), the decisions in [docs/adr/](../adr/README.md), what is left in [docs/v1.1/](../v1.1/README.md).

## 2. Map of the repository

| Path | What it holds | Audit it for |
|---|---|---|
| `crates/core/` | Pure logic, no OS calls: parsers (SMBIOS, SPD, PCI, EDID, PresentMon CSV, MangoHud), the bottleneck detector (`bottleneck.rs`), frame statistics (`frames.rs`), sessions, reports, layouts, monitor geometry (`screens.rs`), actions (`actions.rs`), a test RNG (`testkit.rs`) | Correctness, panics on hostile input, numeric edge cases |
| `crates/platform/` | Per-OS code behind one API: processes, services, elevation, overlay window, displays (`display.rs`; KMS in `display_drm.rs`), PDH, `/proc`, `sysctl`, nettop, the console and handle handling (`os.rs`) | `unsafe`, FFI, privilege use, per-OS parity |
| `src/` | The binary: data collection, telemetry, history (SQLite), updater (`update.rs`), CLI, soak hooks (`soak.rs`, feature `soak`) | Wiring, I/O, the one network call, the `unsafe` that is still here (§7) |
| `src/gui/` | egui/eframe app: pages, tables, plots, overlay, mini monitors, hotkeys (`hotkeys.rs`; Wayland portal in `hotkeys_portal.rs`), screens (`screens.rs`) | UI, UX, accessibility (`a11y.rs`), thread discipline; `forbid(unsafe_code)` |
| `fuzz/` | cargo-fuzz targets (11), a committed starting corpus in `fuzz/corpus/` | Coverage of untrusted input |
| `.github/workflows/` | `ci.yml`, `fuzz.yml`, `nightly.yml`, `release.yml` | Pipeline security, least privilege, pinning, jobs that really test what they claim |
| `.github/scripts/` | `soak.py`, `pkgver.py`, `check_no_crt.py`, `shotdiff.py`, `lint_workflows.py`, `fetch_pawnio.sh` | Script safety and honesty of the checks |
| `packaging/` | WiX (MSI), macOS `Info.plist`, `.desktop`, Arch `PKGBUILD`, the pinned third-party binaries (`pins.txt`), icon build (`icons/`) | Install, upgrade and removal behaviour; hashes; generated binary assets |
| `docs/` | Plan, ADRs, threat model, accessibility, performance, testing notes, this guide | Claims that code does not back |

Layering rule ([ADR 0011](../adr/0011-workspace-split-by-layer.md)): `core` knows nothing of the OS or the GUI; `platform` knows nothing of the GUI. `core` and `gui` forbid `unsafe`.

## 3. Claims to check

Each claim names where the evidence is. A claim without evidence is a finding.

| Claim | Where to look |
|---|---|
| **No network except the update check, off until opted in** (P1) | `src/update.rs`; the test `update_is_the_only_network_path`; grep the tree for `curl`, `http`, `TcpStream`; the Linux CI job that runs with no network (`unshare -n`); [threat model §3](../security/threat-model.md) |
| **The update channel only filters what is offered** (Stable, Beta, Alpha; default follows the running build) | `update::newest`, `Channel`; tests `each_channel_gets_its_newest`, `channels_by_label`; the download is verified against `SHA256SUMS.txt` whatever the channel |
| **Nothing is recorded without opt-in** (P2) | `src/settings.rs` defaults; history, capture and log gates |
| **Elevation is per action, through a closed enum, validated** (P3) | `crates/core/src/actions.rs`, `crates/platform/src/elevation.rs`, the `--elevated-op` decoding |
| **No code injection; other processes opened with limited rights only** (P4) | [ADR 0012](../adr/0012-no-code-injection-ever.md); grep `OpenProcess`, `PROCESS_`, `WriteProcessMemory`, `CreateRemoteThread` |
| **Destructive actions confirm and warn on critical targets** (P4) | `src/gui/pages/`, the critical-process list in `crates/core/src/procs.rs` |
| **Third-party binaries are what we pinned** | `packaging/pins.txt`; the release job checks SHA-256; the app re-checks PawnIO and PresentMon before running them; [THIRD_PARTY.md](../../THIRD_PARTY.md) |
| **Paths a privileged process opens are private** | `crates/platform/src/tempdir.rs` and its tests; the updater, the handoff marker and the PresentMon hold ([threat model §5](../security/threat-model.md)) |
| **Child processes start in every launch mode** | `crates/platform/src/os.rs` (`hide_own_console`, `nul_std_handles`) and its test, which starts a real child under a console of its own; `update::command` never inherits stdin. Probe every other `Command::…spawn()` / `.status()` |
| **Parsers survive hostile input** | `crates/core/src/robustness.rs`, `crates/platform/src/robustness.rs`, `src/robustness.rs`, `fuzz/fuzz_targets/` and the corpus |
| **The detector explains itself** (evidence, thresholds, lower confidence on fallback) | `crates/core/src/bottleneck.rs` and its tests; [bottleneck-detector.md](../v1/bottleneck-detector.md) |
| **Overlay and mini monitors land on the right monitor** | `crates/core/src/screens.rs` tests (negative origins, portrait monitors, round trip); the Windows enumeration in `platform::display::screens` |
| **Wayland shortcuts go through the desktop's portal** | [ADR 0015](../adr/0015-wayland-shortcuts-through-the-portal.md); `src/gui/hotkeys_portal.rs`. Only the pure parts are tested; the exchange with a real portal is not (§8) |
| **Linux refresh and monitors through KMS, no root** | `crates/platform/src/display_drm.rs`: the byte decoding is tested everywhere on fixtures and random bytes; the ioctls are Linux-only and `unsafe` |
| **The UI thread never probes the OS** | the `Cached<T>` pattern in `src/gui/`; `Sampler.want()` |
| **No leaks over hours** | [docs/testing/soak.md](../testing/soak.md): the harness and its analysis are tested; a long run has **not** been made yet |
| **Performance budget** | [docs/performance.md](../performance.md): says what is measured and what is not |
| **Accessible: contrast, scale, keyboard** | [docs/accessibility.md](../accessibility.md), `src/gui/a11y.rs` |
| **Naming: the rename broke nothing** | `cpuzeez_core::DATA_DIR_NAME` and the test `the_data_folder_keeps_its_old_name`; ETW session names, install folder, registry key and `cpuzeez.ini` unchanged; CI's MSI jobs accept both product names |
| **Supply chain: pinned actions, least privilege, dependency policy, declared MSRV** | `.github/workflows/*.yml`, `lint_workflows.py`, `.github/dependabot.yml`, `deny.toml`, the `msrv` job, [SECURITY.md](../../SECURITY.md) |
| **Installers: upgrade, downgrade refused, optional driver, clean removal** | CI jobs `msi-upgrade` and `msi-releases` (upgrade through every published MSI), `.deb` job, distro containers; [README, Uninstalling](../../README.md#uninstalling) |
| **Docs say what the code does** | spot-check README, FEATURES.md and the ROADMAP marks against the code (§9 lists the usual drift) |

## 4. Reproducing the evidence

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --bin cpuzeez --features soak soak
python .github/scripts/soak.py --selftest
python .github/scripts/lint_workflows.py        # needs PyYAML
```

Cross-target lints (clippy does not link, so no C cross-compiler is needed; SQLite's C is not built):

```bash
export LIBSQLITE3_SYS_USE_PKG_CONFIG=1 SQLITE3_LIB_DIR=/nonexistent
cargo clippy --target x86_64-unknown-linux-gnu --workspace --all-targets -- -D warnings
cargo clippy --target aarch64-apple-darwin --workspace --all-targets -- -D warnings
```

The declared minimum Rust (`rust-version` in `Cargo.toml`, currently 1.95; CI's `msrv` job does this):

```bash
rustup toolchain install 1.95 --profile minimal
cargo +1.95 check --workspace --locked
```

Fuzzing (nightly toolchain and `cargo install cargo-fuzz`); the corpus is committed and `write_fuzz_seeds` regenerates it:

```bash
cd fuzz && cargo fuzz list
cargo fuzz run detector -- -max_total_time=60
cargo test -p cpuzeez-core --lib write_fuzz_seeds -- --ignored
```

Dependencies: `cargo deny check` (the CI job `deny` runs it).

Icons, with or without `resvg`: `python packaging/icons/make_icons.py` (uses `resvg` when present, the standard-library renderer `packaging/icons/raster.py` otherwise).

Verifying a release: download an asset and `SHA256SUMS.txt`, compare the hash, check the version string in *About*, and on Windows run `python .github/scripts/check_no_crt.py cpuzeez.exe` against the unpacked binary. The release workflow builds from the tag; compare the tag's commit with the one CI passed on.

What opens a window or takes hours, so ask the owner first: the GUI soak ([soak.md](../testing/soak.md)), `-screenshots`, anything that starts the app.

## 5. Per-area checklists

**Code.** Every `unsafe` block says why it is sound. No `unwrap`/`expect` on data from outside the process. Arithmetic on sizes and times read from files is checked or saturating. Casts on lengths are bounded. Parsers return `Option`/`Result` and never panic. Every external process is started with its three standard handles accounted for.

**Architecture.** The layer rule holds (§2). Per-OS code sits behind one function with the same signature and a documented fallback. Long work runs off the UI thread. ADRs match what the code does (check ADR 0012, 0013 and 0015 against their code).

**Design and UI.** Spacing, radius and colour come from tokens (`src/gui/theme.rs`); pages hard-code no colours. Light and dark are both readable. Layout survives 75–200% scale (§8 lists the pages not yet in that test). The icon and wordmark in `assets/` match `icon.svg`.

**UX.** Every empty value says why (OS limit, missing permission, not yet sampled). Destructive actions confirm. Capability banners say exactly what is missing. Text exists in English, pt-BR and es with the same placeholders (the i18n parity tests). Links open something that exists (a link to a branch file that is not merged is a 404: the About links use the version's tag).

**CI / CD / pipelines.** Actions are pinned by commit SHA. `permissions:` is read-only at the top; write only on the publishing job. No secret is reachable from a fork or printed in a log. A workflow file GitHub would reject must fail CI before a tag exists (`lint_workflows.py`). Release artifacts are built from the tagged commit; pre-releases are flagged from the tag. Third-party binaries (PresentMon, PawnIO) are pinned by version and hash. Each CI job tests what its name says: read what it asserts, not only that it is green. Unsigned releases are a recorded decision ([ADR 0009](../adr/0009-releases-stay-unsigned-for-now.md)).

**Vulnerabilities.** Follow the entry-point table in the [threat model](../security/threat-model.md): each row names the control and the test or fuzz target behind it. Try to break the control, not only to read it. Add: the update path (list parsing, channel filter, download, installer launch), the Wayland portal replies, and the KMS ioctl structures.

**Safety.** Anti-cheat: nothing hooks, injects or reads game memory (P4). The overlay is a separate window, and a safe mode turns the extras off in games with anti-cheat. The kernel driver is PawnIO only, never a vulnerable driver; uninstalling the app keeps the shared driver on purpose.

**Privacy and data.** The data folder holds what the README says and nothing else; uninstalling never deletes it; *Privacy & data* can delete each category. Diagnostics bundles are made locally and never uploaded.

## 6. What the repository cannot prove

Say so in the report instead of inferring:

- **Real desktops.** Wayland (the shortcuts portal, overlay stacking), KDE and GNOME, macOS Spaces and Gatekeeper, a clean Windows machine's SmartScreen. CI has headless renders and builds.
- **Real hardware readings.** Sensor values, SPD, SMU and Super I/O on boards other than the author's; the board maps are community data.
- **Games.** Anti-cheat behaviour in a match, the overlay over exclusive fullscreen, FPS cost, sampler-induced stutter, PresentMon on other drivers.
- **Long runs.** The GUI soak, a long fuzz campaign, the overlay's CPU and memory under load.
- **CI history.** Read the Actions runs themselves; a green badge is not the evidence.
- **Translations.** Parity is tested; quality was not reviewed by a native speaker.

## 7. Known gaps

Everything below is tracked; none is hidden. IDs are ROADMAP rows; the plan for each is in [docs/v1.1/](../v1.1/README.md).

| Gap | Area | State |
|---|---|---|
| The overlay costs about 3.2–3.6 % of one core next to a game, over the 2 % goal | performance | measured in [performance.md](../performance.md); no experiment shipped in 1.1 (M3-04/05, owner's decision) |
| The overlay's private memory grows about 1.5 MB/min in the 30-minute soak on Windows; the 1.0.0 run was flat | performance | headless probes of the app's own parts are flat; likely the renderer or the system ([performance.md](../performance.md#the-overlays-memory-growth-m5-07), M5-07) |
| `unsafe` left outside `platform`: the four benchmark kernels (by design) and five large FFI files (`gpu/win`, `gpu/nvml`, `gpu/dl`, `storage`, `network`) | code, safety | a CI ratchet only lets it go down; the five move in 1.2 (engineering §7) |
| Screen readers cannot read table rows or chart internals | accessibility | [accessibility.md](../accessibility.md) |
| Wayland portal, KMS ioctls, per-monitor placement on mixed-DPI setups: pure parts tested, not on real systems | code | §6 |
| Releases unsigned | CD | [ADR 0009](../adr/0009-releases-stay-unsigned-for-now.md) |
| Windows monitor positions are divided by one scale factor, so other monitors' rectangles are approximate with mixed DPI | design | noted in `src/gui/screens.rs` |

**Closed in 1.1.0:** `gui/mod.rs` is 386 lines (M4-05 to M4-08); the README permissions tables are generated from `core::availability` and checked (M2-08); every page and tab has an interaction test and a scale test (M2-02 to M2-04); the overlay has a drawing budget per style and its CPU and memory are measured (M2-09, M3-03); a 60-minute fuzz run per target is recorded in [fuzz-results.md](../security/fuzz-results.md) (M5-01); the Linux and macOS README screenshots come from CI (M5-03).

## 8. Suggested prompts

- **Code:** "Read `crates/core/src/<file>.rs` and list inputs that panic, overflow or loop forever. For each, give a failing input."
- **Process spawning:** "List every `Command::new` in `src/` and `crates/`. For each, say which standard handles it inherits and whether it can run after `FreeConsole` or from a service. Report any that inherit stdin."
- **Architecture:** "Check that `crates/core` imports nothing from `crates/platform` or `src/gui`, and that every `cfg(target_os)` block has a counterpart or a documented fallback."
- **Updater:** "Read `src/update.rs`. Try a hostile GitHub answer (huge list, odd tags, a pre-release on the Stable channel, an asset with a path in its name, a missing checksum). Does anything run unverified?"
- **Vulnerabilities:** "Walk the entry-point table of the threat model. For each row, open the named control and try to bypass it. Report the rows with no test."
- **Unsafe:** "For each `unsafe` block, state the invariant it relies on and whether the surrounding code guarantees it, including the KMS ioctl structures and `SetStdHandle`."
- **CI/CD:** "Read `.github/workflows/*.yml`. Report any unpinned action, any `pull_request_target`, any write permission outside the publish job, any secret reachable from a fork, and any job whose assertions do not match its name."
- **Release:** "Starting from a tag, follow `release.yml` to the published assets. Could a tag on an unreviewed commit publish? What stops a version that does not match `Cargo.toml`?"
- **Docs truth:** "Pick ten statements from the README, FEATURES.md and the ROADMAP marks, and find the code or test behind each. List the ones with none."
- **UI/UX:** "Open every page at 75% and 200% scale, light and dark. List overlaps, truncated text, low contrast and empty values without an explanation."
- **Naming:** "Search for the old spelling `CpuZeeZ`. Is each hit an identifier that must not change (folder, registry key, ETW session) or a visible name that should have changed?"

## 9. Bug classes that already escaped once

Each was found late, so each deserves a second look elsewhere:

- **Inherited standard handles** after the console is dropped: a child process failed with "The handle is invalid" (the one-click update). Look for the same in any new spawn.
- **A workflow GitHub refuses as a whole** (a duplicate `with:` key from a mechanical edit) that no local tool flagged. Look for mechanical edits across many files.
- **A declared version nobody tested** (`rust-version` said 1.89; the GUI toolkit needs 1.95). Look for other declared-but-unchecked claims.
- **Links to files only on an unmerged branch** (404 in the app). Look for URLs built from `main`.
- **Layout bugs at large text sizes** (a title running into a button) that screenshots at 100% hid.
- **Paths handed to a more privileged process** (joaoregis/cpuzeez#4, #5, found by a contributor): a predictable `/tmp` folder for the updater's package, a predictable root-written marker, a sidecar checked once and run many times by path. Look for any path prepared by one process and opened by a more privileged one; the rule is in the [threat model §8](../security/threat-model.md).
- **Hostile input** found by deterministic tests before any fuzzer ran: an overflow in a histogram, a quadratic loop in session windows, and an allocation sized by a timestamp.

## 10. How to report

For each finding: area, severity (critical, high, medium, low, note), file and line, how to reproduce, and the smallest fix. Mark findings that match a known gap (§7) or a limit (§6). State what you could not check and why. Security findings follow [SECURITY.md](../../SECURITY.md); do not post exploitable details in a public issue.

A suggested severity rubric: **critical** is privilege escalation, code execution from input, or data loss; **high** is a control that can be bypassed or a release that cannot be built or updated; **medium** is a wrong result, a leak, or a lie in the docs about a safety property; **low** is polish, a missing test or a rough edge; **note** is an observation.
