# CPU ZeeZ 1.1.0: roadmap

> Part of the [1.1 plan](README.md). The plan says **why** each item is partial and what "finished" means; this document is the **work list**: the decisions, the releases, the order, one row per pull request with its dependencies and a testable "done when", the gates, and the risks.
> **Status:** planned, nothing started · **Baseline:** `v1.0.0` (`b33cdd4`), `main` at `f0e39c0` · **Date:** 2026-11-09.

## 1. Decisions (2026-11-09)

| # | Decision | Why |
|---|---|---|
| D1 | **1.1.0 is corrections and debt only.** No new features. The read-only helper service, the native ETW consumer, the Startup *Advanced* view and the SRUM history import move to a future major version (2.0); the [1.0 backlog](../v1/ROADMAP.md#after-10-backlog-already-decided-to-wait) says so. | 1.0.0 promised a lot; 1.1 makes those promises easier to trust and the code easier to change. |
| D2 | **Release ladder.** `1.0.1` now (dependencies already on `main`). Small fixes users see ship as `1.0.x`. Everything else goes through `1.1.0-beta.N` (Beta channel), then `1.1.0`. A snapshot is cut only if a beta needs a respin. | Fixes reach users without waiting for the refactors; betas protect the big changes. |
| D3 | **Item 8 is partial in 1.1.** A CI ratchet stops `unsafe` from growing outside `platform`. The small modules' OS calls move to `platform`. The large GPU, storage and network modules (129 of the 222 lines) wait for 1.2. Benchmark kernels (SIMD intrinsics, pointer chasing: `bench`, `membench`, `cpusuite`, `memplus`) are an **allowed exception**, documented in [engineering §7](../v1/engineering.md): they are not OS calls, and moving them to `platform` would add nothing. | The real count is 136 `unsafe` blocks, 9 `unsafe fn` and 8 `unsafe impl` in 21 files, not "about 95". All of it would be the longest part of the cycle, with no visible result. |
| D4 | **Item 13: vendor the icon font, do not adopt the fork.** The app uses 104 Phosphor icons. Ship the Phosphor *Regular* font (MIT) and a generated module with those constants, then drop `egui-phosphor`. `egui-pop-phosphor` 0.15 (a fork for egui 0.37) exists, but it is a third party added to the supply chain for a font file. | It unblocks egui 0.37 now, and no future egui release has to wait for an icon crate again. |
| D5 | **No more `-A dead_code`.** CI and the local gate allow dead code globally ("module APIs are used per-OS"), which hides real warnings. The three warnings in the Linux release build are exactly that (`live_frames.rs:32` and `:62`, `peripherals.rs:85`). Without the allowance: Windows 0, Linux 5, macOS 15. Gate each item with `cfg`, then remove the flag. | The Linux build shows warnings that CI does not, and CI should see what users' builds see. |

## 2. Rules for the cycle

- **One step = one pull request**, with CI green, visual regression included. A screenshot difference is accepted only when the step means to change the layout.
- Each pull request updates `CHANGELOG.md` (*Unreleased*) and this roadmap's row. A new string has en, pt-BR and es (the parity tests enforce it).
- **No new features.** A new setting is allowed only where an item needs it (the panel widths in item 12).
- **Tests before refactors.** Items 6 and 8 start only after the tests from items 2 and 3 have landed.
- **Anything that opens a window or runs long on the owner's machine** (soaks, a manual check) waits for the owner's go. CI runs (screenshots, fuzzing) need no go.
- A release is tagged only when the owner authorizes it, after CI is green on the release commit.

## 3. Milestones

```
M0 1.0.1 ──► M1 1.0.2 ──► M2 1.1.0-beta.1 ──► M3 1.1.0-beta.2 ──► M4 1.1.0-beta.3 ──► M5 1.1.0
  deps        fixes users    tests everywhere,    egui 0.37,           refactors with       screenshots,
              see, CI        panels that fit      overlay CPU          no visible change    fuzz, soaks
                                                                                ▲
item 5 (fuzz on CI) runs in the background from M1 on ──────────────────────────┘
```

| Milestone | Release | Plan items | Effort (work days) | Needs the owner |
|---|---|---|---|---|
| M0 | `1.0.1` | — | 0.5 | Release go |
| M1 | `1.0.2` | 10 (part), 11, 14, 15, 17 | 5–7 | A check of *Services* and of an update on their machine |
| M2 | `1.1.0-beta.1` | 1, 2, 3, 4, 10 (rest), 12 | 7–9 | — |
| M3 | `1.1.0-beta.2` | 9, 13 | 4–6 + soaks | 30-minute overlay soaks |
| M4 | `1.1.0-beta.3` | 6, 8 (partial) | 5–7 | — |
| M5 | `1.1.0` | 5, 7, 16 | 3–4 + CI time | The final soaks |

Total: about 23–30 work days, plus waiting on soaks and CI.

---

## M0. `1.0.1`: the dependencies already on `main`

| ID | Item | Depends on | Done when |
|---|---|---|---|
| M0-01 | Bump to `1.0.1` (the root and both crates). Turn *Unreleased* into `[1.0.1] - <date>`: tray-icon 0.26, libc 0.2.191, CI actions, Dependabot grouping, the cache test, this plan. Tag after CI is green. | — | 11 assets published; the latest release, not a pre-release; `msi-releases` upgrades `1.0.0 → 1.0.1` cleanly |

## M1. `1.0.2`: fixes users see, and CI that sees them

| ID | Item | Plan | Depends on | Done when |
|---|---|---|---|---|
| M1-01 | **Undo only after success.** `dispatch_action` takes the undo entry with it and pushes it only on `Done::Ok`. The fix covers both copies of the pattern: `system_services.rs`, 7 pushes, and `system_startup.rs:252` | 11.1 | — | A kittest with a fake backend that refuses: no undo entry — ✅ one `system::dispatch` for Services and Startup carries the undo and keeps it only on success; test `a_refused_change_leaves_no_undo_and_resets_the_editor` |
| M1-02 | **The editor resets on failure.** `edit_startup` and the recovery fields return to the service's real values, which are re-read | 11.2 | M1-01 | The same test: the combo box shows the real startup type — ✅ on failure the startup, account and recovery editors reset and the details are read again |
| M1-03 | **Readable errors.** Win32 codes 5, 1051, 1060, 1061, 1062 and 1072 become a sentence that keeps the code (en, pt-BR, es). Mapped once, where the executor builds the error, not in each page | 11.3 | — | Unit test per code; *Services* and *Startup* show the sentence — ✅ `system::failure_text` (also 1056, 1058); the elevated copy now exits with `1 000 000 + code`, so a non-elevated app no longer says only "exited with code 1" |
| M1-04 | **Say it before the click.** The configuration read also reads the service's DACL. Authenticated users have `RC`, so no elevation is needed. When neither Administrators nor SYSTEM may change the configuration (`DC`) or start and stop the service (`RP`, `WP`), *Apply* and the buttons are disabled, with the reason | 11.4 | M1-03 | Test on a parsed SDDL (the `CodexSandboxService` one as a fixture); the inspector shows the reason — ✅ `core::services::refused_for_admin` reads the DACL (`READ_CONTROL`, no elevation): Administrators, Everyone, Authenticated Users, Interactive, Users and the user's SID; deny wins; anything not understood refuses nothing. Start, Stop, Restart and Apply are disabled with the reason |
| M1-05 | Translate the service states (*Stopped*, …) in the list and the inspector | 11.6 | — | The parity test covers them; no English state appears in pt-BR — ✅ *Stopped* added; test `every_service_state_is_translated` |
| M1-06 | Show the Linux frame-source note per OS (MangoHud logs, not "the graphics kernel's events") | 10.2 | — | The string is chosen by `cfg`; pt-BR and es too — ✅ Windows, Linux (MangoHud) and macOS (Metal HUD) texts |
| M1-07 | **Dead code gated per OS** (D5): about 20 items, Linux and macOS. Remove `-A dead_code` from `ci.yml:37` and from the gate | 10.3 | — | `clippy -D warnings` passes for all three targets with no allowance; the Linux release build has no warnings — ✅ 16 items: decoders get the `cfg` of their callers plus `test`; the frame-source items macOS lacks, and the PresentMon states other OSes never reach, carry a commented `cfg_attr(allow)`. Clean on Windows, Linux x64 and arm64, macOS arm64 and x64. The CLI-only build (no GUI) still has dead code; its job does not deny warnings |
| M1-08 | **Tests that name themselves**: `cargo nextest` in CI. A test running past 60 s is reported and killed at 120 s; one retry, reported as flaky; JUnit uploaded on failure. `cargo test --doc` stays. The test steps get `timeout-minutes: 20` | 14 | — | A deliberately sleeping test is killed and named in a test run — ✅ `.config/nextest.toml` (profile `ci`), in the five OS jobs and the three distro containers; JUnit uploaded on failure; `timeout-minutes` on the test steps. The installer action is pinned by commit |
| M1-09 | **No fixed sleeps in tests.** Add a `wait_until(deadline, cond)` helper to `core::testkit` and replace every sleep-then-assert. Check the `tempdir` sweep tests for shared folders | 14 | M1-08 | Repo-wide grep finds no sleep-then-assert in tests — ✅ audit: the engine tests already wait with a deadline (`wait_until` in `metrics.rs`); the sleeps left measure an interval (CPU and disk counters) or check that nothing happens over a window, which a condition cannot replace; the `tempdir` sweep tests use unique names. One 2 s count became a 10 s deadline (`hotkeys_portal`). No shared helper needed |
| M1-11 | **One running copy** (item 17): a named mutex and two events in `Local\` (user and SYSTEM, Medium label) in `platform::instance`. A second launch brings the first to the front, from the tray too, and quits; with no answer within 3 s it starts anyway. *Restart as administrator* passes `--replaces=<pid>` | 17 | — | `platform` tests: the second claim sees the first, the show request is answered, the slot is free after the holder ends |
| M1-12 | **The update helper**: a copy of the app in the private download folder waits for the app, runs `msiexec /passive`, and reopens the app with the same rights, or the old version with `--update-failed=<code>` and a translated note. The update bar quits through `on_exit`, which stops PresentMon, instead of `exit(0)` | 17 | M1-11 | CI: install `1.0.0`, start the app, a second launch quits; the helper waits for the app, installs `1.0.1` and reopens one copy |
| M1-10 | **Security review notes** (item 15): the Windows TCP and UDP tables are read through a `Vec<u32>` (aligned by construction). *Privacy & data* lists `exports/` | 15 | — | The table tests pass; the privacy page lists and deletes exports — ✅ `proc_ops` reads both tables into `Vec<u32>` (test `the_rpc_endpoint_mapper_is_listening`); *Exports* is a category to see and delete |

**Gate `1.0.2`:** CI green on all jobs. The owner updates from `1.0.1` through the update bar in administrator mode with the app also in the tray: the app closes, installs and reopens by itself. The owner retries the `CodexSandboxService` change (ChatGPT): the reason shows before the click, and there is no leftover undo entry and no raw code.

## M2. `1.1.0-beta.1`: tests on every page, panels that fit

| ID | Item | Plan | Depends on | Done when |
|---|---|---|---|---|
| M2-01 | **Fake data for the hardware pages**: report sections and `Cached<T>` fixtures for Processor, Mainboard, Memory, Graphics, Devices and Sensors, built from the existing test builders (`a11y.rs`, `testkit`) | 2, 3 | — | Each page renders in a test with no OS access |
| M2-02 | **Interaction tests** for Processor, Mainboard, Memory, Graphics, Devices, Sensors, Tools and About, plus the *Live* and *Sessions* tabs of ZeeZ Live: controls exist and have names; copy, pin, detail level and tab switch work | 2 | M2-01 | One test per page and tab |
| M2-03 | **A test that lists the pages** (`NAV`, `NAV_BOTTOM`, the System and Live tabs) and fails on any page or tab that has no interaction test | 2 | M2-02 | Adding a page without a test fails CI |
| M2-04 | **Scale test on every page and tab** (`at_every_zoom`, 75–200 %). Fix what it finds in the same pull request, one page per commit. Expected: the Live cards clipped at ~1120 px (10.1) | 3, 10.1 | M2-01 | Every page passes at 75, 100, 125, 150 and 200 % |
| M2-05 | **Resizable split**: a shared `gui::split` widget used by the *Services* inspector and the *Processes* details. The handle can be dragged and focused, and moves with the arrow keys. Min 280 px, max 60 %. The width is saved per panel and clamped to the window; double-click resets it. Removes `set_width(345)` and `avail − 370` | 12.1 | — | kittest: dragging and the keys change the width; it survives a settings round trip |
| M2-06 | **Inspector content that fits**: rows wrap, the parameters box takes the remaining width, the content's maximum width is the panel's, and long values are truncated with a tooltip | 12.2, 11.5 | M2-05 | The scale test with the longest es and pt-BR values shows no clipping in either panel |
| M2-07 | **A drawer on narrow windows**: below the width where table and panel both fit, the panel opens over the table | 12.3 | M2-05 | kittest at 800 px |
| M2-08 | **Permissions map from the code**: a table in `core::availability` (feature × OS → nothing / administrator / root / driver / not possible). A test compares its markdown with the block between `<!-- permissions:start -->` and `<!-- permissions:end -->` in both READMEs; `permissions_map.py --write` regenerates it | 1 | — | Changing a requirement without regenerating fails CI (en and pt-BR) |
| M2-09 | **Overlay work budget**: count shapes and text layouts per overlay style, with fixed data, against ceilings recorded next to the styles. The soak report adds frame time as a trend | 4 | — | A style that draws twice as much fails CI |
| M2-10 | **Demo frame source for `-screenshots`**: a fixed frame series, like the bottleneck shot's, so the Live tab can be captured without a game | 10.4 | — | `-screenshots` produces the Live tab with plausible numbers |

**Gate `1.1.0-beta.1`:** CI green. The visual-regression differences are reviewed one by one. The screenshot artifacts from CI show *Services* and *Processes* in pt-BR and es with nothing clipped (no window on the owner's machine).

## M3. `1.1.0-beta.2`: egui 0.37, then the overlay's CPU

13 comes first, so the CPU work is measured on the final stack.

| ID | Item | Plan | Depends on | Done when |
|---|---|---|---|---|
| M3-01 | **Vendor the icon font** (D4), still on egui 0.36: `assets/fonts/Phosphor.ttf` plus a generated module with the 104 constants and the license. `theme.rs` loads it. Drop `egui-phosphor` | 13 | — | Visual regression shows no difference; `cargo-deny` is clean |
| M3-02 | **eframe, egui and egui_kittest 0.37 together.** Fix the API changes and review the visual differences; check whether winit's monitor positions now allow edge snapping on secondary monitors (S1-M02) | 13 | M3-01, M2 | CI green; differences reviewed; the S1-M02 answer recorded |
| M3-03 | **Measure first**: `soak.py` records per-thread CPU (Windows) in the overlay-only report. Baseline on 0.37 | 9 | M3-02 | A table in `performance.md`: the UI thread and each worker |
| M3-04 | **Experiments, one per pull request, each measured with a 30-minute overlay-only soak.** (a) The main window stops repainting while minimised: today `App::ui` always asks for a repaint every 500 ms (`gui/mod.rs:404`). (b) The overlay in its own deferred viewport. (c) The overlay repaints only when a shown value changes (`text_hz`, default 4 Hz). (d) Lease only the sources the active style shows | 9 | M3-03 | Each result is in `performance.md`, kept or reverted |
| M3-05 | **Keep what helps**, with memory checked as well. The 1.0 attempt leaked about 1.5 MB/min | 9 | M3-04 | `soak.py --gui --overlay-only … 30 --max-cpu 2 --max-mem-mb 150` passes on Windows |

**Decision point after M3-04.** If no combination reaches 2 %, the owner decides between a lower goal, documented and gated where it lands, and moving the frame source in-process (2.0, D1).

**Gate `1.1.0-beta.2`:** CI green and an overlay-only soak that passes.

## M4. `1.1.0-beta.3`: refactors with no visible change

| ID | Item | Plan | Depends on | Done when |
|---|---|---|---|---|
| M4-01 | **The `unsafe` ratchet** (D3): `.github/scripts/unsafe_ratchet.py` counts `unsafe` per file outside `platform` against `.github/unsafe-allowlist.txt`. It fails when a count rises or a new file appears. The CI job and the gate run it | 8 | — | A new `unsafe` block in `src/` fails CI |
| M4-02 | Move the small modules' OS calls to `platform` (safe functions; decoding stays): `timers`, `update` (`geteuid`), `smbios`, `bios`, `cpu/arm`, `cpu/clocks`, `live_frames`, `telemetry/os`. Lower the allowlist | 8 | M4-01 | Their tests pass; their counts are 0 |
| M4-03 | Same for `sensors/windows`, `sensors/macos`, `pci/windows` and `npu` | 8 | M4-01 | Same |
| M4-04 | Engineering §7 documents the exception for benchmark kernels and the 1.2 list (`gpu/win`, `gpu/nvml`, `gpu/dl`, `storage`, `network`) | 8 | M4-03 | The allowlist holds only those and the kernels |
| M4-05 | `gui/mod.rs` step 1: page-specific state (of App's 40 fields) moves into each page's `State` | 6 | M2 | Visual regression shows no difference; kittest green |
| M4-06 | Step 2: the frame loop's tasks (title, shortcuts, update check, persistence) move to `app_tasks.rs` | 6 | M4-05 | Same |
| M4-07 | Step 3: navigation (sidebar, command palette targets, `NAV`) moves to `nav.rs` | 6 | M4-06 | Same |
| M4-08 | Step 4: the shared page helpers (`boxed`, `heading`, `sget`, …) move to `pages/common.rs`. `mod.rs` keeps `App`, `ui()` and the page dispatch | 6 | M4-07 | `gui/mod.rs` < 400 lines |

M4-02 to M4-04 and M4-05 to M4-08 touch different files and can alternate. No other GUI pull request is open while M4-05 to M4-08 run.

**Gate `1.1.0-beta.3`:** CI green, no visual difference anywhere in M4, and the allowlist lowered.

## M5. `1.1.0`

| ID | Item | Plan | Depends on | Done when |
|---|---|---|---|---|
| M5-01 | **Long fuzz on CI**: `fuzz.yml` takes a duration input and keeps the grown corpus as an artifact; the 11 targets run as a matrix. Each crash becomes a test in `robustness.rs` and a fix | 5 | — (start in M1) | Each target has ≥ 1 h recorded with no open crash, in `docs/security/fuzz-results.md` |
| M5-02 | **Screenshots from CI on every OS**: the Linux (Weston) and macOS jobs upload the README set (the safe pages only: no process lists, users, startup or services). A per-OS tolerance | 7 | M3-02, M2-10 | The artifacts have every README page on all three OSes |
| M5-03 | READMEs (en, pt-BR) show the new pages on Linux and macOS from those artifacts | 7 | M5-02 | No hand-taken shot left in `docs/screenshots/` |
| M5-04 | **Installers on CI** (item 16): the macOS `.pkg` and the Arch package install, start and uninstall | 16 | — | Both jobs green |
| M5-05 | **Final soaks** on the release candidate build: overlay-only for 30 min, the full tour for 8 h | 9, S3-02 | M4 | Both pass; `performance.md` updated |
| M5-06 | Docs: the CHANGELOG `1.1.0` section, `audit/README.md` known gaps, the v1 ROADMAP rows, this roadmap ✅ | — | everything | Release go |

**Gate `1.1.0`:** every row ✅ or explicitly moved, with a reason, to the [carry-over](#4-carry-over-needs-a-person-or-hardware). Both soaks pass. CI is green on the release commit.

---

## 4. Carry-over: needs a person or hardware

Tracked here so nothing is lost. None of it blocks 1.1.0.

| Row (1.0) | What | Waits for |
|---|---|---|
| S0-B06 | Manual test matrix for *Restart as administrator* | People on each OS |
| S0-X02 to S0-X09 | Frame generation and 240+ FPS, the overlay over exclusive fullscreen, overlays on macOS and Wayland, MangoHud latency, Metal HUD, anti-cheat in a match, the overlay's GPU cost | Game sessions on that hardware |
| S0-X11, S2b-07 | macOS: per-process GPU, "not responding", ScreenCaptureKit FPS | A Mac |
| S1-G05 | Intel memory secondaries; DDR5 GDM / 2T / power-down | An Intel DDR5 board |
| S1-D05 | A pinned reader pool, if the rare ~30 ms spikes matter | The overlay GPU-cost measurement (X09) |
| S3-03 | Screen reader pass by a person | A tester |
| S3-04 | Native-speaker review of pt-BR and es | Reviewers |

## 5. Risks

| Risk | Where | Mitigation |
|---|---|---|
| egui 0.37 changes break many kittest tests at once | M3-02 | The icon font is vendored first (M3-01), so the bump changes only egui. M2 leaves a test on every page to show what broke |
| The overlay cannot reach 2 % within the shared event loop | M3 | Measure per thread first. One experiment per pull request. The decision point after M3-04 |
| A repaint change leaks or freezes the overlay (as in 1.0) | M3-04 | Every experiment runs the overlay-only soak with the memory gate, not only CPU |
| The `mod.rs` split conflicts with other work | M4 | Run it last, with no other GUI pull request open, one step per pull request |
| nextest changes timings or misses doctests | M1-08 | Keep `cargo test --doc`; compare one week of job durations |
| The fuzz campaign uses a lot of CI time | M5-01 | A manual trigger with a duration input, not a schedule; matrix jobs under the 6 h limit |
| New Dependabot updates during the cycle | all | Merged in batches at each milestone, as in #16; the egui group waits for M3 |
