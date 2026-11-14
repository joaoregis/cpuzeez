# CPU ZeeZ 1.1.0: roadmap

> Part of the [1.1 plan](README.md). The plan says **why** each item is partial and what "finished" means; this document is the **work list**: the decisions, the releases, the order, one row per pull request with its dependencies and a testable "done when", the gates, and the risks.
> **Status:** `1.1.0` released (M0–M5; the soaks waived by the owner) · **Baseline:** `v1.0.0` (`b33cdd4`), `main` at `f0e39c0` · **Date:** 2026-11-09.

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
| M1-11 | **One running copy** (item 17): a named mutex and two events in `Local\` (user and SYSTEM, Medium label) in `platform::instance`. A second launch brings the first to the front, from the tray too, and quits; with no answer within 3 s it starts anyway. *Restart as administrator* passes `--replaces=<pid>` | 17 | — | `platform` tests: the second claim sees the first, the show request is answered, the slot is free after the holder ends — ✅ `platform::instance`; the listener starts before the first frame, so a second launch is answered during startup too |
| M1-12 | **The update helper**: a copy of the app in the private download folder waits for the app, runs `msiexec /passive`, and reopens the app with the same rights, or the old version with `--update-failed=<code>` and a translated note. The update bar quits through `on_exit`, which stops PresentMon, instead of `exit(0)` | 17 | M1-11 | CI: install `1.0.0`, start the app, a second launch quits; the helper waits for the app, installs `1.0.1` and reopens one copy — ✅ the MSI job checks one running copy, then the helper installs the new build and reopens it |
| M1-10 | **Security review notes** (item 15): the Windows TCP and UDP tables are read through a `Vec<u32>` (aligned by construction). *Privacy & data* lists `exports/` | 15 | — | The table tests pass; the privacy page lists and deletes exports — ✅ `proc_ops` reads both tables into `Vec<u32>` (test `the_rpc_endpoint_mapper_is_listening`); *Exports* is a category to see and delete |

**Gate `1.0.2`:** CI green on all jobs, the MSI job included (one running copy; the helper installs and reopens the app). The update from `1.0.1` still runs `1.0.1`'s updater, so for that one update the owner closes the tray copy and administrator mode first; closing and reopening by itself is checked on the first update after `1.0.2`. On `1.0.2` the owner retries the `CodexSandboxService` change (ChatGPT): the reason shows before the click, and there is no leftover undo entry and no raw code.

## M2. `1.1.0-beta.1`: tests on every page, panels that fit

| ID | Item | Plan | Depends on | Done when |
|---|---|---|---|---|
| M2-01 | **Fake data for the hardware pages**: report sections and `Cached<T>` fixtures for Processor, Mainboard, Memory, Graphics, Devices and Sensors, built from the existing test builders (`a11y.rs`, `testkit`) | 2, 3 | — | Each page renders in a test with no OS access — ✅ `gui::fixture`: a `Ctx` with every probe off and no hardware backend, a fixed CPU and report, seeded `Cached<T>` probes (`Cached::seed` never refreshes), fixed devices and sessions, and a telemetry engine over fixed readings. Two cheap reads remain (C-states, volume encryption) |
| M2-02 | **Interaction tests** for Processor, Mainboard, Memory, Graphics, Devices, Sensors, Tools and About, plus the *Live* and *Sessions* tabs of ZeeZ Live: controls exist and have names; copy, pin, detail level and tab switch work | 2 | M2-01 | One test per page and tab — ✅ `gui::page_tests`: every control has a name; copy and pin (Processor, Mainboard, Memory, Graphics, Sensors), the detail switch, the Tools and Live tabs, About's links, the Sessions list |
| M2-03 | **A test that lists the pages** (`NAV`, `NAV_BOTTOM`, the System and Live tabs) and fails on any page or tab that has no interaction test | 2 | M2-02 | Adding a page without a test fails CI — ✅ exhaustive matches over `Page`, `SysTab` and `live::Tab` name each test; a new variant does not compile without one, and a named test that does not exist fails |
| M2-04 | **Scale test on every page and tab** (`at_every_zoom`, 75–200 %). Fix what it finds in the same pull request, one page per commit. Expected: the Live cards clipped at ~1120 px (10.1) | 3, 10.1 | M2-01 | Every page passes at 75, 100, 125, 150 and 200 % — ✅ every page in `NAV`, the Live and Tools tabs; the check also fails on text past the window's right edge (rows about as wide as the window are tables, which scroll). Fixed: segmented tabs wrap; Processes' network note no longer covers the problem pills; the Live tiles take as many columns as fit, and the overlay preview becomes a column when its strip does not fit |
| M2-05 | **Resizable split**: a shared `gui::split` widget used by the *Services* inspector and the *Processes* details. The handle can be dragged and focused, and moves with the arrow keys. Min 280 px, max 60 %. The width is saved per panel and clamped to the window; double-click resets it. Removes `set_width(345)` and `avail − 370` | 12.1 | — | kittest: dragging and the keys change the width; it survives a settings round trip — ✅ `gui::split` (one closure for both sides, so both may borrow the page's state). The width lives in `settings.panel_widths`, published to the frame like the pins and saved once the drag ends. Tests: drag, ←/→, the 60 % and 280 limits, double-click reset, the settings round trip |
| M2-06 | **Inspector content that fits**: rows wrap, the parameters box takes the remaining width, the content's maximum width is the panel's, and long values are truncated with a tooltip | 12.2, 11.5 | M2-05 | The scale test with the longest es and pt-BR values shows no clipping in either panel — ✅ tests `the_services_inspector_fits_…` and `the_processes_details_fit_…` (pt-BR and es, 75–200 %). Each side of the split gets a fixed rectangle; rows wrap; labels go above the combo boxes, which are capped to the panel and truncate; the parameters box takes what the button leaves; long names and dependency buttons truncate with a tooltip; the Processes preset combo starts a new toolbar line when it does not fit (`widgets::wrap_before`) |
| M2-07 | **A drawer on narrow windows**: below the width where table and panel both fit, the panel opens over the table | 12.3 | M2-05 | kittest at 800 px — ✅ below 648 points (panel 280 + table 360 + handle) the panel opens over the table, at least 360 wide, on a layer inside the page so it scrolls with it. Test at 600 points (800 px fits both); the 200 % scale tests (430 points) exercise it in both panels. The Services inspector no longer needs 700 points to appear |
| M2-08 | **Permissions map from the code**: a table in `core::availability` (feature × OS → nothing / administrator / root / driver / not possible). A test compares its markdown with the block between `<!-- permissions:start -->` and `<!-- permissions:end -->` in both READMEs; `permissions_map.py --write` regenerates it | 1 | — | Changing a requirement without regenerating fails CI (en and pt-BR) — ✅ `availability::MAP` holds the *What else needs administrator or root* table in English and Portuguese; `the_readmes_show_this_map` compares both README blocks, and `the_hardware_row_matches_the_banner` checks the hardware row against the banner's own rules (`registers(os)`). The first table (with and without privileges) stays hand-written prose |
| M2-09 | **Overlay work budget**: count shapes and text layouts per overlay style, with fixed data, against ceilings recorded next to the styles. The soak report adds frame time as a trend | 4 | — | A style that draws twice as much fails CI — ✅ `every_overlay_style_stays_within_its_drawing_budget` counts the shapes and text layouts each built-in style paints from the fixture readings, against ceilings about a third above today's (Minimal 3/1 … Detailed 34/24); a style without a ceiling fails too. The soak build logs the overlay's drawing time each minute and `soak.py` reports its first and last quarter and slope per hour |
| M2-10 | **Demo frame source for `-screenshots`**: a fixed frame series, like the bottleneck shot's, so the Live tab can be captured without a game | 10.4 | — | `-screenshots` produces the Live tab with plausible numbers — ✅ `live_frames::demo` replays the recorded Deadlock capture (the bottleneck shot's) in a loop through the hub, and the real frame source never starts; the Live shot waits 4 s for the lows. Test `the_demo_replays_in_a_loop` |

**Gate `1.1.0-beta.1`:** CI green. The visual-regression differences are reviewed one by one. The screenshot artifacts from CI show *Services* and *Processes* in pt-BR and es with nothing clipped (no window on the owner's machine).

✅ **Passed 2026-11-12.** CI green on every pull request (#28–#36). Two visual differences, both intended and reviewed: `system-services.png` (the inspector beside the table, labels above the combo boxes, M2-05–07) and `live.png` (the recorded capture, M2-10). The pt-BR and es check is met by tests rather than screenshots: the CI screenshot run is English, and `the_services_inspector_fits_…` and `the_processes_details_fit_…` draw both panels in pt-BR and es at 75–200 % and fail on any clipping.

## M3. `1.1.0-beta.2`: egui 0.37, then the overlay's CPU

13 comes first, so the CPU work is measured on the final stack.

| ID | Item | Plan | Depends on | Done when |
|---|---|---|---|---|
| M3-01 | **Vendor the icon font** (D4), still on egui 0.36: `assets/fonts/Phosphor.ttf` plus a generated module with the 104 constants and the license. `theme.rs` loads it. Drop `egui-phosphor` | 13 | — | Visual regression shows no difference; `cargo-deny` is clean — ✅ the same font file `egui-phosphor` 0.14 bundled, so the pixels cannot change; `gui::theme::icon` holds the 104 constants, and `every_icon_is_in_the_font` checks each has a glyph in that font |
| M3-02 | **eframe, egui and egui_kittest 0.37 together.** Fix the API changes and review the visual differences; check whether winit's monitor positions now allow edge snapping on secondary monitors (S1-M02) | 13 | M3-01, M2 | CI green; differences reviewed; the S1-M02 answer recorded — ✅ the API changes (screenshots arrive through a callback, `WidgetType` → `Role`, `Id::unique`, `Ui::scope_id`, `ComboBox<'a>`). egui 0.37 no longer bundles the emoji fonts: `egui`'s `monochrome_emoji_fonts` feature keeps them. egui_kittest 0.37 fails a test on a missing glyph or an unnamed control, which found real gaps: icons in monospace text and ● in the egui font drew as boxes, and about 40 controls had no accessible name; each is now named after the label beside it (`widgets::Control`, `labelled_by`) or given one. **S1-M02:** egui still reports only the current monitor's size (`ViewportInfo::monitor_size`; a monitor index exists only for fullscreen), but the mini monitor already snaps to every monitor from the OS's own list (`gui/screens.rs`), so nothing waits on winit |
| M3-03 | **Measure first**: `soak.py` records per-thread CPU (Windows) in the overlay-only report. Baseline on 0.37 | 9 | M3-02 | A table in `performance.md`: the UI thread and each worker — ✅ `soak.py` lists the busiest threads in the report; the table and the baseline on 0.37 are in [performance](../performance.md#the-overlay-alone-per-thread-m3-03). **Found on the way:** private memory grows about 1.5 MB/min in this soak on 0.36 and on 0.37 alike, which the 2026-11-09 run did not show; M3-05 cannot pass until that is found |
| M3-04 | **Experiments, one per pull request, each measured with a 30-minute overlay-only soak.** (a) The main window stops repainting while minimised: today `App::ui` always asks for a repaint every 500 ms (`gui/mod.rs:404`). (b) The overlay in its own deferred viewport. (c) The overlay repaints only when a shown value changes (`text_hz`, default 4 Hz). (d) Lease only the sources the active style shows | 9 | M3-03 | Each result is in `performance.md`, kept or reverted — ✅ closed without a soak (owner, 2026-11-13). **(a)** read on the code and dropped: the overlay is an immediate child viewport (`show_viewport_immediate`) that asks for its own repaint every `1/text_hz` s (250 ms by default), so the main window's 500 ms request is never the one that sets the pace while the overlay is up; slowing it only helps with the overlay hidden, which is not the case the budget is for. **(b), (c), (d)** not tried: each changes what the overlay draws or when, and without a soak there is no way to tell a gain from a leak (the 1.0 attempt leaked) |
| M3-05 | **Keep what helps**, with memory checked as well. The 1.0 attempt leaked about 1.5 MB/min | 9 | M3-04 | `soak.py --gui --overlay-only … 30 --max-cpu 2 --max-mem-mb 150` passes on Windows — ⛔ waived by the owner (no soak): nothing was changed, so nothing was kept. The overlay stays at the measured ≈ 3.2–3.6 % of one core, over the 2 % goal, and `performance.md` says so. The decision point below is left open for the owner |

**Decision point after M3-04.** If no combination reaches 2 %, the owner decides between a lower goal, documented and gated where it lands, and moving the frame source in-process (2.0, D1).

**Gate `1.1.0-beta.2`:** CI green. The overlay-only soak that passed here was waived by the owner (2026-11-13); the 2 % goal is not met and the memory growth below is not explained (see [performance](../performance.md#the-overlay-alone-per-thread-m3-03)).

## M4. `1.1.0-beta.3`: refactors with no visible change

| ID | Item | Plan | Depends on | Done when |
|---|---|---|---|---|
| M4-01 | **The `unsafe` ratchet** (D3): `.github/scripts/unsafe_ratchet.py` counts `unsafe` per file outside `platform` against `.github/unsafe-allowlist.txt`. It fails when a count rises or a new file appears. The CI job and the gate run it | 8 | — | A new `unsafe` block in `src/` fails CI — ✅ `unsafe_ratchet.py` counts the keyword in the 21 files outside `crates/platform` that have it (225 in all, comments and strings ignored) against `.github/unsafe-allowlist.txt`; a higher count or a new file fails, a lower one asks for `--update`. Runs in the `native` job and in the gate; has a `--selftest` |
| M4-02 | Move the small modules' OS calls to `platform` (safe functions; decoding stays): `timers`, `update` (`geteuid`), `smbios`, `bios`, `cpu/arm`, `cpu/clocks`, `live_frames`, `telemetry/os`. Lower the allowlist | 8 | M4-01 | Their tests pass; their counts are 0 — ✅ the calls are safe functions in `platform`: `clock` (timer rates, `rdtsc`, the add chain), `firmware::{table, uefi}`, `os::{euid, final_path, sysctl_str, sysctl_u64, cpu_load_ticks, catch_ctrl_c}`. 32 `unsafe` gone; the list is 193 in 13 files |
| M4-03 | Same for `sensors/windows`, `sensors/macos`, `pci/windows` and `npu` | 8 | M4-01 | Same  — ✅ `platform::pdh` (the sensors' four counters), `os::battery`, `smc` (macOS), `setupapi::pci_devices`, `dxcore::compute_adapters`; decoding stays in the binary. 29 `unsafe` gone; the list is 164 in 9 files |
| M4-04 | Engineering §7 documents the exception for benchmark kernels and the 1.2 list (`gpu/win`, `gpu/nvml`, `gpu/dl`, `storage`, `network`) | 8 | M4-03 | The allowlist holds only those and the kernels  — ✅ §7 lists both groups and why; the list is exactly the 4 kernels and the 5 files moving in 1.2 (164 `unsafe`, from 225) |
| M4-05 | `gui/mod.rs` step 1: page-specific state (of App's 40 fields) moves into each page's `State` | 6 | M2 | Visual regression shows no difference; kittest green  — ✅ `App` keeps 26 fields, from 45: `pages::{Processor, Mainboard, Memory, Graphics, Benches, Tools}`, `update_ui::Updates`, `settings_ui::Minis` |
| M4-06 | Step 2: the frame loop's tasks (title, shortcuts, update check, persistence) move to `app_tasks.rs` | 6 | M4-05 | Same  — ✅ `take_pins`, `take_widths`, `hotkeys`, `check_updates_if_allowed` and `window_title` |
| M4-07 | Step 3: navigation (sidebar, command palette targets, `NAV`) moves to `nav.rs` | 6 | M4-06 | Same  — ✅ `nav.rs`: `Page`, `NAV`, `NAV_BOTTOM`, `page_name`, the sidebar, the logo and `run_palette` |
| M4-08 | Step 4: the shared page helpers (`boxed`, `heading`, `sget`, …) move to `pages/common.rs`. `mod.rs` keeps `App`, `ui()` and the page dispatch | 6 | M4-07 | `gui/mod.rs` < 400 lines  — ✅ 386 lines, from 1618: `pages/common.rs` (helpers), `pages/{benchmarks, settings}.rs`, `strings.rs` (the general pt-BR/es tables), `start.rs` (`run`), the screenshot types in `shots.rs`, `app_tests.rs` |

M4-02 to M4-04 and M4-05 to M4-08 touch different files and can alternate. No other GUI pull request is open while M4-05 to M4-08 run.

**Gate `1.1.0-beta.3`:** CI green, no visual difference anywhere in M4, and the allowlist lowered.

## M5. `1.1.0`

| ID | Item | Plan | Depends on | Done when |
|---|---|---|---|---|
| M5-01 | **Long fuzz on CI**: `fuzz.yml` takes a duration input and keeps the grown corpus as an artifact; the 11 targets run as a matrix. Each crash becomes a test in `robustness.rs` and a fix | 5 | — (start in M1) | Each target has ≥ 1 h recorded with no open crash, in `docs/security/fuzz-results.md` — 🟡 workflow ready (1.0.2): *Run workflow* with `minutes`, executions in the run summary, grown corpus as an artifact; ✅ 60 min per target: the first run crashed 4 targets (fixed in #55, replayed by `robustness::fuzz_campaign_crashes`), the second ran all 11 clean; numbers in [fuzz-results](../security/fuzz-results.md) |
| M5-02 | **Screenshots from CI on every OS**: the Linux (Weston) and macOS jobs upload the README set (the safe pages only: no process lists, users, startup or services). A per-OS tolerance | 7 | M3-02, M2-10 | The artifacts have every README page on all three OSes  — ✅ every native job with a README folder keeps the safe pages as `readme-<os>` (Windows, Linux, macOS arm64 and Intel) and fails when a README page is missing (`readme_shots.py`). No per-OS tolerance is needed: nothing compares these sets, only the Windows visual regression compares PR with base |
| M5-03 | READMEs (en, pt-BR) show the new pages on Linux and macOS from those artifacts | 7 | M5-02 | No hand-taken shot left in `docs/screenshots/`  — ✅ Linux, macOS and macOS Intel replaced from run 43536868305. The Windows set stays from the owner's machine: it is the `-screenshots` automation too (not hand-taken), on real hardware, which a CI virtual machine cannot show |
| M5-04 | **Installers on CI** (item 16): the macOS `.pkg` and the Arch package install, start and uninstall | 16 | — | Both jobs green  — ✅ `macos-pkg` (pkgbuild as release.yml, `installer`, a report, removal as the README says) and `arch-pkg` (the release tarball and PKGBUILD, `makepkg`, `pacman -U`, a report, `pacman -R`) |
| M5-05 | **Final soaks** on the release candidate build: overlay-only for 30 min, the full tour for 8 h | 9, S3-02 | M4 | Both pass; `performance.md` updated  — ⛔ waived by the owner (no soaks in 1.1, 2026-11-13); the last numbers are the 2026-11-13 overlay runs in `performance.md` |
| M5-06 | Docs: the CHANGELOG `1.1.0` section, `audit/README.md` known gaps, the v1 ROADMAP rows, this roadmap ✅ | — | everything | Release go  — ✅ the CHANGELOG `1.1.0` section, `audit/README.md` §7, the v1 ROADMAP rows S0-E04, S0-E06, S0-T05, this roadmap |
| M5-07 | **Last, at the owner's request: the overlay's memory growth.** Private memory grows about 1.5 MB/min in the overlay-only soak on 0.36 and 0.37 alike; 1.0.0 was flat. Find the change after 1.0.0 that does it, from the code first | — | everything else | The cause is named and fixed, or written down as known in the 1.1.0 notes  — ✅ written down as known (no soak, owner's decision): headless probes of the process collector, the telemetry with frames and the whole `App` with the overlay are flat once their windows fill; the growth needs the real renderer, and the machine took two Windows updates between the flat and the growing runs. See [performance](../performance.md#the-overlays-memory-growth-m5-07) |

**Gate `1.1.0`:** every row ✅ or explicitly moved, with a reason, to the [carry-over](#4-carry-over-needs-a-person-or-hardware). CI is green on the release commit. The two soaks were waived by the owner (2026-11-13).

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
| A repaint change leaks or freezes the overlay (as in 1.0) | M3-04 | Every experiment runs the overlay-only soak with the memory gate, not only CPU. **Not done: no experiment shipped** |
| **Open:** private memory grows ≈ 1.5 MB/min in the overlay-only soak, on 0.36 and 0.37 alike; the 1.0.0 run was flat | after 1.0.0 | M5-07: not in the app's own logic (headless probes are flat); likely the renderer or the system. Known in the 1.1.0 notes |
| The `mod.rs` split conflicts with other work | M4 | Run it last, with no other GUI pull request open, one step per pull request |
| nextest changes timings or misses doctests | M1-08 | Keep `cargo test --doc`; compare one week of job durations |
| The fuzz campaign uses a lot of CI time | M5-01 | A manual trigger with a duration input, not a schedule; matrix jobs under the 6 h limit |
| New Dependabot updates during the cycle | all | Merged in batches at each milestone, as in #16; the egui group waits for M3 |
