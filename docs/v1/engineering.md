# Engineering: architecture, organization and practices

> Part of the [1.0 plan](README.md). It applies to **every** 1.0 feature.
> Goal: 1.0 must stay easy to understand, test and change at twice today's size (~31k lines).

## 1. Starting point

| Fact (0.3.0) | Consequence |
|---|---|
| A single crate, with the GUI as a feature (`gui`) | Nothing stops a collection module from importing `egui`, or the GUI from calling OS APIs |
| `gui/mod.rs` is 1,738 lines: Processor/Mainboard/Memory/Graphics pages inline, plus navigation, screenshots and update | One file at the centre of everything. Every change touches it |
| `sensors/superio.rs` is 2,043 lines | Data (per-chip and per-board maps) mixed with logic |
| Hand-written FFI (`extern` blocks) everywhere | `unsafe` is scattered, with no obvious place to audit |
| Platform selection by `#[cfg]` with same-named per-OS functions | It works, but there's no explicit contract, so a platform can silently drift from the others |
| The release build emits **20 compiler warnings** | CI doesn't enforce `-D warnings` yet |
| The release exe depends on `VCRUNTIME140.dll`, which isn't shipped | A start-up failure on clean Windows ([dependencies.md §3](dependencies.md)) |
| Pure decoders (CPUID, SMBIOS, SPD, PCI, EDID, SMART) with tests | **The best part of the code, and the model to follow** |

## 2. Layers and dependency direction

Proposal: a **workspace** whose boundaries the compiler enforces, not just convention.

```
crates/
  core/        domain + pure logic         (no OS, no GUI)
  platform/    per-OS adapters             (all unsafe and FFI live here)
  gui/         egui pages and widgets      (no OS calls)
  cpuzeez/     binary: composition, CLI    (wires platform → core → gui)
  [helper/]    after 1.0, privilege option C (reuses core + platform, no GUI)
```

```
cpuzeez (bin) ──► gui ──► core ◄── platform
      └─────────────────────────────┘
```

| Crate | May depend on | May **not** depend on | Contains |
|---|---|---|---|
| `core` | `std`, `serde` | `egui`, `platform`, any OS API | Models (`Reading`, `Process`, `Service`, `Frame`, `Session`), pure decoders (CPUID, SMBIOS, SPD, PCI, EDID, SMART), the Metric Engine, bottleneck rules, frame statistics, file formats (CSV, sessions), **the source traits** |
| `platform` | `core`, `libc`, `windows-sys`, `objc2`… | `egui` | One implementation of the `core` traits per OS, in `platform/{windows,linux,macos}/`. All `unsafe` sits in `sys/` behind safe wrappers |
| `gui` | `core`, `eframe`, `egui-phosphor` | `platform`, OS APIs | Pages, widgets, the Widget Engine, theme, i18n |
| `cpuzeez` | all | — | `main`: picks the platform, builds the services, opens the GUI or the CLI |

**Why a workspace now (and not earlier):**
1. **An enforced boundary.** `core` builds and tests on any OS without any OS API. The bottleneck detector, the Metric Engine and the reports are testable with fake data in CI on every platform.
2. **The helper (C) and the CLI reuse collection** without pulling in egui.
3. **Build times.** Touching the GUI doesn't recompile the decoders.
4. **The migration means moving modules, not rewriting.** The decoders are already pure.

**What *not* to do:** one crate per feature, or a "plugin system". Those are abstractions without a second use. The split is by **layer**.

## 3. Platform contracts (traits where 3+ real implementations exist)

A trait exists only when there's **more than one real implementation**: the three platforms + a test fake. Today's `trait Hw` (`hw/mod.rs:46`) already follows that pattern. It's extended to:

```rust
// core/src/sources.rs: contracts (sketch, names to be refined)
pub trait ProcessSource  { fn snapshot(&mut self) -> Result<ProcSnapshot, SourceError>; }
pub trait ServiceManager { fn list(&self) -> Result<Vec<Service>, SourceError>;
                           fn plan(&self, a: &ServiceAction) -> Result<ActionPlan, SourceError>; }
pub trait StartupSource  { /* list + toggle */ }
pub trait SessionSource  { /* users / sessions */ }
pub trait FrameSource    { fn subscribe(&mut self, target: Target) -> Result<FrameStream, SourceError>; }
pub trait SensorSource   { /* today's sensors::monitors + gpu::live + os::memory */ }
```

- Each trait has a **contract test suite** (`core::sources::contract::*`) that every implementation runs. For example: "two snapshots never lose PID 1/4", "an unknown action returns a typed error". That's what stops a platform from drifting silently.
- **Fakes** (`core::sources::fake`) feed the tests of the Metric Engine, the detector and the GUI.

## 4. Shared building blocks (deliberate reuse)

Several features use the same building blocks. Building them once is what keeps the code small.

### 4.1 Availability model (powers P3: capability banners)

```rust
pub enum Availability {
    Ok,
    NeedsElevation { what: Capability },     // "Restart as administrator"
    NeedsDriver   { driver: &'static str },  // PawnIO
    NeedsComponent { what: Component },      // MangoHud, kernel module msr… (dependencies.md)
    NeedsPermission { kind: OsPermission },  // macOS Screen Recording, Linux i2c group…
    Unsupported   { reason: &'static str },  // ❌ with the reason
    Failed        { error: String },
}
```

- **Every source** returns its availability together with its data.
- The GUI has **one** `capability_banner(&[Availability])` component. Every page's banner, the README's permissions map and the "what you lose without admin" text all come from **the same source**, so they can't drift apart.

### 4.2 Actions as data + executor (end a process, services, startup, users)

```rust
pub struct Action { kind: ActionKind, target: TargetRef, requires: Privilege, risk: Risk }
// ActionKind::{ProcessKill, ProcessSetPriority, ServiceStop, ServiceSetStartType, StartupToggle, SessionLogoff, …}
```

- The GUI **describes** an action, and an `Executor` runs it:
  - **in-process**, when the app already has the privilege;
  - **in an elevated subprocess** (option B: the action is serialized into an argument), when it doesn't.
- **The same code path serves both cases.**
- `risk` decides the confirmation: normal, "critical system process", or "irreversible".
- Every executed action goes into a **local audit log** (P1: local only, visible in Settings): "15:02 Stopped service *Spooler* (elevated)".
- The elevated entry point accepts **only** serialized `Action`s from an allow-list. Anything else is rejected, which is tested and fuzzed (§8).

### 4.3 Virtualized table (`gui::table`)

- One implementation for Processes, Details, Services, Startup, Users, Sensors, SMART and History:
  - configurable columns, with saved presets;
  - stable sorting;
  - filtering with a syntax (`name:chrome cpu>10`);
  - grouping;
  - multi-select;
  - a context menu fed by `Action`s;
  - copy and CSV export;
  - virtualization (`ScrollArea::show_rows`) for tens of thousands of rows.
- Today's `egui::Grid` tables migrate to it as they're touched.

Together with the **Metric Engine** and the **Widget Engine** ([README §4](README.md)), that makes five building blocks. **No other internal framework** without a proven second use.

## 5. Concurrency

- **The UI thread never blocks on I/O or on an OS call.** Every probe runs off it, and the UI reads snapshots.
- The Metric Engine **owns** its state and publishes immutable snapshots (an `Arc<Snapshot>` swapped atomically). That replaces one big `Mutex` locked by both the UI and the sampler (fixes D5).
- Slow or blocking sources (SMART, `ioreg`, `smartctl`, services) run on their own workers, with **timeouts** and **cancellation**. A stuck probe never freezes the others.
- No async runtime (as today): `std::thread` + channels are enough at this scale. Revisit only if the helper (C) ever has to serve many clients.

## 6. Errors, failure and robustness

- **Typed** errors at the boundaries (`SourceError` variants map to `Availability`). No generic `String`s.
- A failing probe **degrades**: its field shows "unavailable (reason)" and the rest of the page carries on.
- A probe that panics is isolated (`catch_unwind` in the worker). The error goes to the local log, and the GUI stays up (1.0 criterion).
- User files (settings, layouts, sessions) carry a **schema version** and **migrations**. A newer file never silently breaks an older app: the app warns and doesn't overwrite it.

## 7. `unsafe` and FFI

- All `unsafe` lives in `platform/*/sys/` behind safe functions. `#![deny(unsafe_op_in_unsafe_fn)]` everywhere, and `#![forbid(unsafe_code)]` in `core` and `gui`.
- Every `unsafe` block has a `// SAFETY:` comment stating its invariant.
- `windows-sys` replaces hand-written Win32 declarations ([dependencies.md §2.2](dependencies.md)). It's used only in `platform/windows`, and existing `extern`s migrate when touched, not in a big bang.

## 8. Testing strategy and quality gates

**Every stage (S0–S3) closes only when its gate passes** ([README §6.2](README.md)). CI runs on GitHub-hosted Windows, Ubuntu (X11 and Wayland via a headless compositor) and macOS (Intel and ARM) runners. Those runners have no real sensors or GPUs, which is why fakes and fixtures carry most of the load. Real hardware is the last layer.

| Layer | What | Runs where | Gate |
|---|---|---|---|
| **Unit** | Decoders, frame statistics, bottleneck rules, parsers, Metric Engine logic | CI, all OSes | Every stage |
| **Recorded fixtures** | `/proc/*`, SMART/NVMe dumps, SMU tables, **PresentMon/MangoHud CSV**, real game sessions → expected result ("this cs2 V-Sync session classifies as *limiter*") | CI, all OSes | Every stage |
| **Contract** | Every `platform` implementation against its trait's suite (§3) | CI, on the matching OS | Every stage |
| **GUI interaction** | `egui_kittest`, egui's official test harness: it drives pages through the **accessibility tree** ("click 'End task', the confirmation appears, confirm, the executor received `ProcessKill(1234)`"), with fake sources | CI, all OSes | Every new or changed page |
| **Visual regression** | The existing `-screenshots` set compared with stored references (with a tolerance), light and dark | CI | Every stage |
| **Fuzz** | Parsers of untrusted input: SMBIOS, SPD, EDID, imported CSV, session/report JSON, **the elevated-op argument parser** (`cargo-fuzz`) | Scheduled + before each gate | No crashes in the corpus |
| **Security** | `cargo-deny` (licenses, advisories, duplicates); executor allow-list (unknown or malformed ops rejected); **no network call without opt-in** (a test asserts the update module is the only network path and that it checks consent; a Linux CI job runs the app inside a network namespace with no network and expects zero connection attempts); data-folder permissions (user-only); a lint that rejects `unsafe` outside `platform` | CI | Every stage |
| **Performance** | Cost of one Metric Engine tick, of one overlay frame, and of a 10k-row table frame, as CI benchmarks with thresholds (P6) that **fail the build** on regression | CI + real hardware | Every stage |
| **Soak** | Sampler + overlay + capture for N hours on a runner, asserting flat memory and no growth in handles/fds | Nightly | S2 and S3 |
| **Packaging** | Install, upgrade, downgrade blocking and uninstall (this exists for the MSI), extended to: **clean-Windows start** (catches CRT-style breakage), PawnIO component checkbox, `.deb`/Arch install, macOS signature checks | CI | Every stage that touches packaging |
| **Real hardware** (manual) | Matrix: AMD Zen 4 + NVIDIA (the reference machine), Intel hybrid (P/E) + AMD GPU, Intel iGPU laptop with battery, Apple Silicon, Intel Mac, Linux AMD GPU (Wayland GNOME and KDE). ZeeZ Live spikes SP1–SP8 run here | Before closing a stage | S1, S2, S3 |

## 9. Continuous quality

- **CI:** `rustfmt`; `clippy -D warnings` with an agreed `pedantic` subset (today's 20 warnings are cleared in S0); tests; `cargo-deny`; MSRV check.
- **Dependency policy:** see [dependencies.md](dependencies.md). Every new crate needs a short ADR.
- **ADRs** live in `docs/adr/NNNN-title.md`: one per decision in [README §8](README.md), and one for each future decision, with context, decision, alternatives and consequences. A year from now, nobody has to rediscover why the overlay doesn't inject.
- **Module size:** each GUI page gets its own file, and `gui/mod.rs` keeps only navigation and composition. Large data sets (Super I/O chip and board maps, CPU/GPU databases) are separated from the logic, preferably as declarative tables.
- **Definition of done** for every feature:
  1. tests (§8) at every applicable layer;
  2. per-OS availability declared (§4.1);
  3. actions with risk and privilege declared (§4.2);
  4. i18n complete;
  5. docs (P7) and the CHANGELOG `Unreleased` updated;
  6. performance budget verified (P6).
- **Local logs** with levels and rotation, and no personal data. A diagnostics bundle (logs + report) **leaves the machine only if the user attaches it manually** to an issue (P1).
- **Reviews:** small PRs; a change that moves code never also changes behaviour; every PR has a green CI.

## 10. Migration (part of S0, incremental)

1. ✅ Create the workspace and move the pure decoders into `core`: report, config, SPD, EDID, CPU/GPU names, SMU PM tables, SMART parsers, the SMBIOS parser and the PCI model and decoder. Existing tests pass unchanged.
2. ✅ Create `platform` with the shared per-OS foundation: `os`, `hw` (`trait Hw` + PawnIO/Linux backends) and CPUID.
3. Extract the pages out of `gui/mod.rs` into their own files, with no behaviour change (the screenshot regression is the safety net).
4. Build the Metric Engine in `core`, plug the GUI into it, and remove the duplicate sampler.
5. Only then start the new features, already built on the contracts.

**The rest of the per-OS collection moves by strangling, not big-bang.** Most existing modules (`cpu`, `gpu`, `sensors`, `storage`, `network`…) mix OS calls with building report sections through `Ctx`. Splitting each one now would mean rewriting every page twice, since S1 rebuilds them anyway. So:
- new per-OS sources (processes, per-disk I/O, adapter throughput, services, startup, frames…) are written in `platform` from the start, behind `core` traits;
- each existing module moves its collection into `platform` and its formatting into `core` **in the same change that rebuilds its page in S1/S2**;
- the `gui` crate is extracted once the GUI no longer calls the binary's OS code (end of S1). Until then it stays a module of the binary.

Each step is a small, reviewable commit with green CI.
