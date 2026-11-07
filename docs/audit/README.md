# Audit guide

For whoever audits CPU ZeeZ 1.0, human or AI. It says what the project claims, where to look, how to reproduce the evidence, and what is known to be weak. It does not hide limits: a finding that matches a known limit is still a finding, but it is not news.

Audit scope: **code, architecture, design, UI, UX, CI, CD, pipelines, vulnerabilities, safety.**

## 1. What the project is

A cross-platform (Windows, Linux, macOS) hardware and system monitor in Rust: CPU-Z-style inspection, a Task Manager under a *System* hub, and *ZeeZ Live* (overlay, FPS, capture, a bottleneck detector). One binary: GUI by default, CLI with flags (`-txt`, `-json`, `-sensors`, `-log`, …).

The plan and its rules are in [docs/v1/README.md](../v1/README.md) (principles **P1–P8**), the work list in [docs/v1/ROADMAP.md](../v1/ROADMAP.md), the decisions in [docs/adr/](../adr/README.md).

## 2. Map of the repository

| Path | What it holds | Audit it for |
|---|---|---|
| `crates/core/` | Pure logic, no OS calls: parsers (SMBIOS, SPD, PCI, EDID, PresentMon CSV, MangoHud), the bottleneck detector (`bottleneck.rs`), frame statistics (`frames.rs`), sessions, reports, units, actions (`actions.rs`) | Correctness, panics on hostile input, numeric edge cases |
| `crates/platform/` | Per-OS code behind one API: processes, services, elevation, overlay window, displays, PDH, `/proc`, `sysctl`, nettop | `unsafe`, FFI, privilege use, per-OS parity |
| `src/` | The binary: data collection, telemetry, history (SQLite), updater, CLI | Wiring, I/O, the one network call (`update.rs`) |
| `src/gui/` | egui/eframe app: pages, tables, plots, overlay, mini monitor | UI, UX, accessibility (`a11y.rs`), thread discipline |
| `fuzz/` | cargo-fuzz targets, one per parser | Coverage of untrusted input |
| `.github/workflows/` | `ci.yml`, `fuzz.yml`, `nightly.yml`, `release.yml` | Pipeline security, least privilege, pinning |
| `.github/scripts/` | `soak.py`, `pkgver.py`, `check_no_crt.py`, `shotdiff.py`, `fetch_pawnio.sh` | Script safety and honesty of the checks |
| `docs/` | Plan, ADRs, threat model, testing notes | Claims that code does not back |

Layering rule ([ADR 0011](../adr/0011-workspace-split-by-layer.md)): `core` knows nothing of the OS or the GUI; `platform` knows nothing of the GUI.

## 3. Claims to check

Each claim names where the evidence is. A claim without evidence is a finding.

| Claim | Where to look |
|---|---|
| **No network except the update check, off until opted in** (P1) | `src/update.rs`; grep the tree for `curl`, `http`, `TcpStream`; [threat model §3](../security/threat-model.md) |
| **Nothing is recorded without opt-in** (P2) | `src/settings.rs` defaults; history, capture and log gates |
| **Elevation is per action, through a closed enum, validated** (P3) | `crates/core/src/actions.rs`, `crates/platform/src/elevation.rs`, the `--elevated-op` decoding |
| **No code injection; other processes opened with limited rights only** (P4) | [ADR 0012](../adr/0012-no-code-injection-ever.md); grep `OpenProcess`, `PROCESS_`, `WriteProcessMemory`, `CreateRemoteThread` |
| **Destructive actions confirm and warn on critical targets** (P4) | `src/gui/pages/`, the critical-process list in `crates/core/src/procs.rs` |
| **Parsers survive hostile input** | `crates/core/src/robustness.rs`, `crates/platform/src/robustness.rs`, `src/robustness.rs`, `fuzz/fuzz_targets/` |
| **The detector explains itself** (evidence, thresholds, lower confidence on fallback) | `crates/core/src/bottleneck.rs` and its tests; [bottleneck-detector.md](../v1/bottleneck-detector.md) |
| **The UI thread never probes the OS** | the `Cached<T>` pattern in `src/gui/`; `Sampler.want()` |
| **No leaks over hours** | [docs/testing/soak.md](../testing/soak.md) |
| **Accessible: contrast, scale, keyboard** | [docs/accessibility.md](../accessibility.md), `src/gui/a11y.rs` |
| **Supply chain: pinned actions, least privilege, dependency policy** | `.github/workflows/*.yml`, `.github/dependabot.yml`, [SECURITY.md](../../SECURITY.md) |

## 4. Reproducing the evidence

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings -A dead_code
cargo test --workspace --all-targets
cargo test --bin cpuzeez --features soak soak
python .github/scripts/soak.py --selftest
```

Cross-target lints (clippy does not link, so no C cross-compiler is needed):

```bash
cargo clippy --target x86_64-unknown-linux-gnu --workspace --all-targets -- -D warnings -A dead_code
cargo clippy --target aarch64-apple-darwin --workspace --all-targets -- -D warnings -A dead_code
```

Fuzzing (nightly toolchain and `cargo install cargo-fuzz`):

```bash
cd fuzz && cargo fuzz list
cargo fuzz run detector -- -max_total_time=60
```

Dependencies: `cargo deny check` (the CI job `deny` runs it).

The soak in GUI mode opens a window and runs for hours: see [soak.md](../testing/soak.md). Ask the owner before running it.

## 5. Per-area checklists

**Code.** Every `unsafe` block says why it is sound. No `unwrap`/`expect` on data from outside the process. Arithmetic on sizes and times read from files is checked or saturating. Casts on lengths are bounded. Parsers return `Option`/`Result` and never panic.

**Architecture.** The layer rule holds (§2). Per-OS code sits behind one function with the same signature and a documented fallback. Long work runs off the UI thread. ADRs match what the code does.

**Design and UI.** Spacing, radius and colour come from tokens (`src/gui/theme.rs`); pages hard-code no colours. Light and dark are both readable. Layout survives 75–200% scale.

**UX.** Every empty value says why (OS limit, missing permission, not yet sampled). Destructive actions confirm. Capability banners say exactly what is missing. Text exists in English, pt-BR and es with the same placeholders (the i18n parity tests).

**CI / CD / pipelines.** Actions are pinned by commit SHA. `permissions:` is read-only at the top; write only on the publishing job. No secret is reachable from a fork or printed in a log. Release artifacts are built from the tagged commit. Third-party binaries (PresentMon, PawnIO) are pinned by version and hash. Unsigned releases are a recorded decision ([ADR 0009](../adr/0009-releases-stay-unsigned-for-now.md)).

**Vulnerabilities.** Follow the entry-point table in the [threat model](../security/threat-model.md): each row names the control and the test or fuzz target behind it. Try to break the control, not only to read it.

**Safety.** Anti-cheat: nothing hooks, injects or reads game memory (P4). The overlay is a separate window, and a safe mode turns the extras off in games with anti-cheat. The kernel driver is PawnIO only, never a vulnerable driver.

## 6. Known limits

- Desktop behaviour on macOS and Linux is checked by CI builds and tests, not on real desktops by the author.
- Screen-reader support for painted tables and charts is missing ([accessibility.md](../accessibility.md)).
- The FPS-impact budget (≤ 1%) needs a game and PresentMon; it is measured by hand.
- Releases are unsigned.
- Wayland has no global hotkeys without the GlobalShortcuts portal.
- Known partial items (a large `gui/mod.rs`, `unsafe` in the older hardware modules, tests missing on some pages, no long fuzz campaign yet) are listed with a plan in [docs/v1.1/](../v1.1/README.md).

## 7. Suggested prompts

- **Code:** "Read `crates/core/src/<file>.rs` and list inputs that panic, overflow or loop forever. For each, give a failing input."
- **Architecture:** "Check that `crates/core` imports nothing from `crates/platform` or `src/gui`, and that every `cfg(target_os)` block has a counterpart or a documented fallback."
- **Vulnerabilities:** "Walk the entry-point table of the threat model. For each row, open the named control and try to bypass it. Report the rows with no test."
- **CI/CD:** "Read `.github/workflows/*.yml`. Report any unpinned action, any `pull_request_target`, any write permission outside the publish job, any secret reachable from a fork."
- **UI/UX:** "Open every page at 75% and 200% scale, light and dark. List overlaps, truncated text, low contrast and empty values without an explanation."

## 8. How to report

For each finding: area, severity (critical, high, medium, low, note), file and line, how to reproduce, and the smallest fix. Mark findings that match a known limit (§6). Security findings follow [SECURITY.md](../../SECURITY.md).
