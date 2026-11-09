# CPU ZeeZ 1.1.0: the plan

> Status: **decided** (2026-11-09): corrections and debt only, no new features. The order, the releases and one row per pull request are in the [roadmap](ROADMAP.md). These are the corrections and small improvements that 1.0.0 leaves partly done, found by checking each [ROADMAP](../v1/ROADMAP.md) row against the code. Each item says why it is only partial today, what finishing it takes, and how we will know it is finished. Nothing here changes what 1.0.0 promises users; it makes the promises easier to trust and the code easier to change.
> Difficulty is the work to finish, not its value: **MEDIO** (a day or two, low risk), **ALTO** (several days, needs care), **MUITO ALTO** (a project of its own).

Items rated BAIXO were fixed during the 1.0.0 hardening instead of being listed here: the MSRV job (the declared `rust-version` was wrong: egui 0.36 needs 1.95; it is now declared and checked in CI) and the committed fuzz corpus.

| # | Item | ROADMAP row | Difficulty | Milestone · release |
|---|---|---|---|---|
| 1 | [Permissions map checked against the code](#1-permissions-map-checked-against-the-code) | S0-B04 | MEDIO | M2 · beta.1 |
| 2 | [Interaction tests for the remaining pages](#2-interaction-tests-for-the-remaining-pages) | S0-T05 | MEDIO | M2 · beta.1 |
| 3 | [More pages in the scale test](#3-more-pages-in-the-scale-test) | S3-03 | MEDIO | M2 · beta.1 |
| 4 | [An overlay frame budget that counts work](#4-an-overlay-frame-budget-that-counts-work) | S0-T08 | MEDIO | M2 · beta.1 |
| 5 | [A long fuzz campaign, with its results](#5-a-long-fuzz-campaign-with-its-results) | S0-T07 | MEDIO | M5 · 1.1.0 (runs from M1) |
| 6 | [`gui/mod.rs` down to navigation and composition](#6-guimodrs-down-to-navigation-and-composition) | S0-E04 | ALTO | M4 · beta.3 |
| 7 | [Screenshots of the new pages on Linux and macOS](#7-screenshots-of-the-new-pages-on-linux-and-macos) | S3-07 | ALTO | M5 · 1.1.0 |
| 8 | [`unsafe` only in `platform`](#8-unsafe-only-in-platform) | S0-E06 | MUITO ALTO (partial in 1.1: ALTO) | M4 · beta.3 |
| 9 | [Overlay CPU down to the 2 % goal](#9-overlay-cpu-down-to-the-2--goal) | S3-01 (P6) | ALTO | M3 · beta.2 |
| 10 | [Small fixes found while taking the 1.0.0 screenshots](#10-small-fixes-found-while-taking-the-100-screenshots) | — | BAIXO | M1 · 1.0.2 and M2 |
| 11 | [Services: a change the service's own permissions refuse](#11-services-a-change-the-services-own-permissions-refuse) | — | BAIXO–MEDIO | M1 · 1.0.2 |
| 12 | [Side panels that fit, and can be resized](#12-side-panels-that-fit-and-can-be-resized) | S3-03 | MEDIO | M2 · beta.1 |
| 13 | [egui 0.37, with the icon font vendored](#13-egui-037-with-the-icon-font-vendored) | — | MEDIO | M3 · beta.2 |
| 14 | [Find the tests that hang or fail now and then](#14-find-the-tests-that-hang-or-fail-now-and-then) | S0-T | BAIXO–MEDIO | M1 · 1.0.2 |
| 15 | [Notes from the 2026-11-09 security review](#15-notes-from-the-2026-11-09-security-review) | S3-05 | BAIXO | M1 · 1.0.2 |
| 16 | [macOS and Arch installers exercised in CI](#16-macos-and-arch-installers-exercised-in-ci) | S3-06 | MEDIO | M5 · 1.1.0 |

Out of scope (decision D1 in the [roadmap](ROADMAP.md)): the read-only helper service, the native ETW consumer, the Startup *Advanced* view and the SRUM history import wait for a future major version. What needs a person or specific hardware is in the roadmap's carry-over.

---

## 1. Permissions map checked against the code

**Why it is partial.** `core::availability` is the single source for the capability banners, but the permissions tables in the README (and README.pt-BR) are written by hand. They can drift from the code and nothing says so. The ROADMAP asked for the map to be "generated or checked".

**What finishing takes.**
- Give `availability` a machine-readable listing: for each capability, the data it unlocks and, per OS, whether it needs administrator/root, a driver, or nothing.
- A test that renders that listing and compares it with a marked block in the README files (`<!-- permissions:start --> … <!-- permissions:end -->`), failing with a diff. A small script regenerates the block (`python .github/scripts/permissions_map.py --write`).
- The prose around the table stays hand-written; only the table is generated.

**Done when.** Changing a capability's requirement without regenerating the README fails CI, in English and in Portuguese.

**Risks.** The README groups by feature and the code by capability, so the mapping needs a small table of its own. Do not parse prose.

## 2. Interaction tests for the remaining pages

**Why it is partial.** The pages with actions (Overview, Settings, the System tabs, Bottleneck, the table) have `egui_kittest` tests. Processor, Mainboard, Memory, Graphics, Devices, Sensors and About only have screenshots, which catch layout but not behaviour (a button that does nothing, a copy action, a tab that does not switch).

**What finishing takes.** One test per page, driven through the accessibility tree: the page renders with fake data, its main controls exist and have names, and each action does what it says (copy a row, pin to the Overview, switch the detail level, open a tab). Pages read the OS through `Cached<T>`; the tests feed it fakes, as the System tests do.

**Done when.** Every sidebar page has at least one interaction test, and a new page cannot be added without one (a test lists the pages and fails on a page with no test).

## 3. More pages in the scale test

**Why it is partial.** The 75–200% test covers Bottleneck, Services, Users, App history, Processes and Startup. Overview, Settings, Sensors, the ZeeZ Live tabs and the hardware pages are not in it. The six pages it covers had two real layout defects, so the others are likely to have some.

**What finishing takes.** The same fake-data approach as item 2, so do both together. For each page, `at_every_zoom` renders at five scales and fails on overlapping widgets. Fix what it finds (wrap rows, give columns a minimum width).

**Done when.** Every page passes at 75, 100, 125, 150 and 200%.

## 4. An overlay frame budget that counts work

**Why it is partial.** The engine tick and the 10k-row table have tests with limits. The overlay frame does not, because a wall-clock limit on a shared CI runner is noise (a slow macOS runner already crossed one).

**What finishing takes.** Measure what the frame *does*, which is stable: the number of shapes painted, text layouts and allocations for each overlay style, with a fixed layout and data. Fail when a style crosses a recorded ceiling. Add the wall-clock figure to the soak report, where it is a trend and not a gate.

**Done when.** A change that makes the overlay draw twice as much fails CI, and the ceilings are recorded next to the styles.

## 5. A long fuzz campaign, with its results

**Why it is partial.** The 11 targets exist, have a committed starting corpus and a 5-minute scheduled run each. No long run has been made and no results are recorded, so "fuzzed" means "could be fuzzed".

**What finishing takes.** Run each target for hours on Linux with the nightly toolchain (a manual `workflow_dispatch` with a longer `max_total_time`, or a weekly schedule), keep the grown corpus as a build artifact, turn every crash into a regression test and a fix, and record target, duration and executions in `docs/security/fuzz-results.md`.

**Done when.** Each target has a recorded run of at least one hour with no open crash, and every crash found has a test in `robustness.rs`.

## 6. `gui/mod.rs` down to navigation and composition

**Why it is partial.** The pages live in their own modules, but `gui/mod.rs` is still about 1,500 lines: the app state, the frame loop, the sidebar, shortcuts, the update bar, the mini monitors' glue and many page-specific fields.

**What finishing takes.** Small steps, each its own pull request and each checked by the visual-regression job (no pixel changes):
1. Move page-specific state out of `App` into each page's own `State`.
2. Move the frame loop's tasks (title, shortcuts, update check, persistence) into `app_tasks.rs`.
3. Move navigation (sidebar, command-palette targets) into `nav.rs`.
4. What stays in `mod.rs` is `App`, `ui()` and the page dispatch.

**Done when.** `gui/mod.rs` is under about 400 lines with no behaviour change (visual regression and all kittest tests green after every step).

**Risks.** `mod.rs` is touched by everything, so this conflicts with parallel work; do it in a quiet window, one step per pull request.

## 7. Screenshots of the new pages on Linux and macOS

**Why it is partial.** `-screenshots` renders every page, but each OS needs its own desktop and fonts, and only Windows is available to the author. The README shows the Windows set.

**What finishing takes.**
- Linux: the CI already renders under a headless compositor (Weston); upload the set as an artifact for the new pages and review it, or take it on a Wayland and an X11 virtual machine.
- macOS: the CI runners can render them too; the work is choosing which pages go in the README and keeping the sets in `docs/screenshots/<os>/`.
- Add the new pages (System tabs, ZeeZ Live, Bottleneck) to the set of every OS.

**Done when.** README and README.pt-BR show the new pages on all three OSes, produced by CI and not by hand.

**Risks.** Desktop fonts differ from run to run; the visual-regression tolerance has to be set per OS.

## 8. `unsafe` only in `platform`

**Why it is partial.** `core` and `gui` forbid `unsafe`. The root binary's older hardware modules (`src/**` outside `gui`) have 136 `unsafe` blocks, 9 `unsafe fn` and 8 `unsafe impl` in 21 files (counted 2026-11-09), each with a `SAFETY:` comment, but they sit next to the code that uses them instead of in `platform`.

**What finishing takes.** Move each module's OS calls behind a safe function in `platform` and leave the decoding where it is; module by module (storage, sensors, GPU, chipset, SPD, memory tests), each with its own pull request and the same tests passing. Then `#![forbid(unsafe_code)]` on the binary crate.

**Done when.** `grep -rn unsafe src/` finds nothing and the lint forbids it.

**In 1.1 (decision D3): partial.** A CI ratchet stops it growing, the small modules move, and benchmark kernels (SIMD intrinsics, pointer chasing) stay as a documented exception, since they are not OS calls. `gpu/win` (53 lines), `gpu/nvml` (32), `storage` (20), `network` (16) and `gpu/dl` (8) wait for 1.2. *1.1 is done when* the allowlist holds only those and the kernels.

**Risks.** The largest item here: many files, FFI signatures that are easy to get subtly wrong, and no behaviour to show for it. Better done module by module while touching each one than as a single change.

---

## 9. Overlay CPU down to the 2 % goal

**Why it is partial.** 1.0.0 meets the memory budget and has no leaks (8 hours with every page: memory flat at ~143 MB; 30 minutes overlay-only: ~120 MB), but the overlay-only soak measured **3.5 % of one core** (p95 4.6 %) against the 2 % goal ([performance](../performance.md)). 1.0.0 ships with that number, accepted.

**What finishing takes.** Find where the cost goes before changing anything: per-thread CPU puts about 1–1.9 % on the interface thread and about 1 % on each of two workers. Known lead: with the overlay up, eframe keeps running the main window's interface while it is minimised (the overlay is its child viewport). Not building the hidden page was tried in 1.0 and saved little (≈ 3 %); a first version of it also made memory grow ~1.5 MB/min, so any change there needs the overlay soak for memory too. Other candidates: the overlay's repaint rate, the sources its default style leases (≈ 1.3 % estimated for sampling), moving the overlay to its own deferred viewport.

**Done when.** `soak.py --gui --overlay-only … 30 --max-cpu 2 --max-mem-mb 150` passes on Windows, with memory still flat.

**Risks.** The interface and the overlay share one event loop; changes there can break repaint (a frozen overlay) or leak, as the 1.0 attempt showed.

---

## 10. Small fixes found while taking the 1.0.0 screenshots

Found on 2026-11-09 with `cpuzeez -screenshots` on Windows 11 and Ubuntu 24.04 (WSLg); none blocks 1.0.0.

- **10.1 ZeeZ Live ▸ Live is wider than the window.** At the screenshot size (about 1120 px), the FPS / 1 % low cards in the *Game* card are cut at the right edge. *Done when:* the cards wrap or shrink, and the scale test covers the Live tab.
- **10.2 Wrong frame source text on Linux.** ZeeZ Live (*Bottleneck* tab, and the Live tab's note) says "Frames are read from the graphics kernel's events", which is the Windows path (ETW via PresentMon); on Linux frames come from MangoHud's logs, as the line above it says. *Done when:* the note is per OS (pt-BR and es included).
- **10.3 Three compiler warnings in the Linux release build** (`cargo build --release` on Ubuntu 24.04). *Cause (found 2026-11-09):* all three are dead code that exists only on Linux (`live_frames.rs:32` and `:62`, `peripherals.rs:85`). CI and the local gate run clippy with `-A dead_code`, which hides them; without it, Windows has 0, Linux 5 and macOS 15. *Done when:* each item is gated with `cfg`, the allowance is gone from CI and the gate, and the Linux build has no warnings.

- **10.4 No Live tab in the screenshots.** The screenshot run measures whatever app is in front when no game runs (another copy of CPU ZeeZ, at 2 FPS), so the Live tab is not in the README. *Done when:* `-screenshots` uses a demo frame source, like the one the bottleneck shot uses.

---

## 11. Services: a change the service's own permissions refuse

**Reported** on 2026-11-09 (1.0.0, Windows 11, pt-BR): *System ▸ Services*, `CodexSandboxService.OpenAI.Codex` (display name "ChatGPT"), startup type Automatic → Disabled, *Apply*. Result: `Error: OpenServiceW(CodexSandboxService.OpenAI.Codex) failed (error 5)`.

**Cause: not a CPU ZeeZ bug, the service refuses it.** Error 5 is `ERROR_ACCESS_DENIED`. The service's security descriptor (`sc sdshow CodexSandboxService.OpenAI.Codex`):

```
D:(A;;CCLCSWRPWPDTLOCRRC;;;AU)
  (A;;CCDCLCSWRPWPDTLOCRSDRCWDWO;;;S-1-5-80-1949724575-…)
```

Authenticated users may query, start and stop it; only the service's own SID has `DC` (`SERVICE_CHANGE_CONFIG`), `WD` and `WO`. There is no entry for Administrators or SYSTEM, so the change is refused even from the elevated copy, and Windows' own *Services* tool fails the same way. The packaged app protects its service on purpose. CPU ZeeZ must not take ownership or rewrite the DACL to get around it.

**What CPU ZeeZ does wrong around it** (seen in the same screenshot):

1. **Undo for a change that never happened.** *Apply* pushes the undo entry ("Undo: CodexSandboxService.OpenAI.Codex -> Automatic") before the action runs, and it stays when the action fails. The same pattern is used for start, stop, restart and the recovery settings (`src/gui/system_services.rs`, every `undo_stack.push` before `dispatch_action`). *Fix:* push the undo entry only when the action reports success.
2. **The editor keeps the refused value.** The startup combo still shows *Disabled* while the list shows *Automatic*. *Fix:* on failure, reset `edit_startup` to the service's real value and re-read it.
3. **A raw error.** `OpenServiceW(…) failed (error 5)` says nothing a user can act on. *Fix:* map error 5 to "Windows refused: this service's permissions don't allow changing it, even for administrators" (translated), keeping the code for support; other common codes (1060 not found, 1061 not accepting controls, 1062 not started) likewise.
4. **Say it before the user tries.** The configuration read already opens the service; checking whether the current token (or an administrator) can get `SERVICE_CHANGE_CONFIG` / `SERVICE_START|STOP` would let the inspector disable *Apply* and the buttons with a reason, as it already does for protected services.
5. **The inspector is wider than its panel**: see [item 12](#12-side-panels-that-fit-and-can-be-resized).
6. **"Stopped" is not translated** in the list's Status column (pt-BR shows "Stopped" next to "Em uso"); the inspector's *Status* row too.

**Done when.** A failed action leaves no undo entry and resets the editor; error 5 reads as a sentence in all three languages; a service whose DACL refuses the change has *Apply* disabled with that reason; the inspector fits at 100–200 % scale; the status column is translated. A test with a fake service backend that refuses `SetStartup` covers 1–3.

---

## 12. Side panels that fit, and can be resized

**Reported** on 2026-11-09 (1.0.0, Windows 11, pt-BR, 1920 px wide): the Services inspector (*System ▸ Services*, a service selected) is cut on the right: the *Start with parameters* button ("Iniciar com parâmetro…"), and every value in the details list ("Irrest…" for *Irrestrito*, "Nenh…", "Stopp…", "LocalSyst…"). The user would like to drag the boundary between the table and the panel, or at least never see it cut.

**Why it is cut** (`src/gui/system_services.rs`):

- The table takes `available width − 370 px` and the inspector gets the rest, with its frame pinned at `set_width(345.0)`. The width is fixed whatever the window, the scale or the language.
- Several rows inside are wider than 345 px in pt-BR and es: the start-parameters row is a 200 px text box **plus** a button whose label is longer than in English. egui doesn't shrink them; it grows the content past the frame.
- That one wide row widens the whole scroll area's content, so every right-aligned value in the details list (`kvs`: *Service SID*, *Triggers*, *Status*, *Log on as*…) is aligned to the widened edge, which is off the panel and clipped by the window. That's why values cut even on rows that would fit.
- The panel is not resizable and does not scroll sideways, so there is no way to see the rest.

The *Processes* tab's *Details panel* does better (`(available × 0.36).clamp(280, 440)`), but it is not resizable either and has the same risk with long values.

**What finishing takes.**

1. **Resizable split**, shared by both panels: a drag handle between table and panel (egui's resizable side panel, or a separator the user drags), min 280 px / max 60 % of the page, the width remembered per panel in settings, double-click to reset. Keyboard: the handle is focusable and arrow keys move it (accessibility, S3-03).
2. **Content that fits any width**: rows that can't fit wrap (`horizontal_wrapped`), the parameters box takes the remaining width instead of a fixed 200 px, buttons drop to the next line; values in the details list are truncated with a tooltip with the full text (or wrap), and the content width is set to the panel width (`set_max_width`) so one row cannot push the others out.
3. **Narrow windows**: below a width where table + panel don't both fit, the panel opens over the table (as a drawer) instead of squeezing it.

**Done when.** The Services inspector and the Processes details panel show no clipped text at 100–200 % scale in en, pt-BR and es (the scale test covers both, with the longest service values); both can be resized by mouse and keyboard and keep their width across restarts.

**Risks.** Low. Shared layout code, so the Processes panel must be checked too; the remembered width must be clamped when the window gets smaller.

---

## 13. egui 0.37, with the icon font vendored

**Why it waits.** Dependabot proposed `eframe` 0.36 → 0.37 and `egui_kittest` 0.36 → 0.37 as two pull requests (joaoregis/cpuzeez#10, #12). Each fails alone: the three crates share egui's API, which changed in 0.37. They can't move together yet either: `egui-phosphor` 0.14.0 (the icon font) requires `egui ^0.36` and has no release for 0.37. Both pull requests were closed on 2026-11-09, and Dependabot now groups the egui crates into one pull request.

**Decided (D4).** A fork for 0.37 exists (`egui-pop-phosphor` 0.15), but a third party in the supply chain for a font file is not worth it. The app uses 104 icons: ship the Phosphor *Regular* font (MIT) with a generated module of those constants, still on egui 0.36 (no pixel change), and drop `egui-phosphor`. No future egui release waits for an icon crate again.

**What finishing takes.** When `egui-phosphor` supports 0.37 (or by vendoring its font and the icon constants we use): bump eframe, egui, egui_kittest and egui-phosphor together, fix the API changes, re-run the visual regression and scale tests, refresh the screenshots.

**Done when.** One pull request moves them all, CI green, screenshots compared.

---

## 14. Find the tests that hang or fail now and then

Seen on 2026-11-09, both once and not reproduced:

- **A hang in CI** (`debian:stable`, joaoregis/cpuzeez#15, a docs-only change): `cargo test --workspace` ran 46 minutes until the job was killed; the job usually takes 6–8 minutes, and a rerun passed. The killed job left no log of which test was running.
- **One failure in the `platform` crate's tests** during a local gate run (56 passed, 1 failed); 16 runs after it passed. The failing test's name was not kept.

(A third, `gui::cache::tests::refreshes_in_the_background`, waited a fixed 100 ms for a background refresh; fixed for 1.0.1 by waiting on the result with a deadline.)

**What finishing takes.** Make the next occurrence name itself: run the CI tests with `cargo nextest` (per-test `slow-timeout` that reports and kills a test stuck past, say, 120 s, and retries that flag flaky tests), give the test steps a `timeout-minutes` well below the job's, and keep the gate's test log on failure. Then fix what it names: tests that sleep for a fixed time instead of waiting on a condition, and tests that share temp folders or global state (the `tempdir` sweep tests are a suspect).

**Done when.** A week of CI with no unexplained hang or failure, or each one named and fixed.

---

## 15. Notes from the 2026-11-09 security review

From [review-2026-11-09.md](../security/review-2026-11-09.md#notes-for-110-no-fix-tonight):

- `proc_ops` (Windows) reads the TCP and UDP tables through a `Vec<u8>` cast to 4-byte-aligned rows. It works because the allocator aligns every allocation, but the language does not promise it. *Fix:* a `Vec<u32>` buffer, sound by construction.
- *Privacy & data* does not list the `exports/` folder as a category to see and delete.

**Done when.** Both are fixed. The dump-folder note stays out of scope (UAC is not a security boundary).

## 16. macOS and Arch installers exercised in CI

**Why it is partial.** S3-06: the MSI chain and the `.deb` are installed, upgraded and removed in CI; the macOS `.pkg` and the Arch package are built but never installed there.

**What finishing takes.** A macOS job that installs the `.pkg`, runs `cpuzeez --version` and removes it (the files `pkgutil --files` lists, then `pkgutil --forget`); an `archlinux` container job that installs the `.pkg.tar.zst` with `pacman -U`, runs it and removes it.

**Done when.** Both jobs are green on every pull request that touches packaging.

## Order and release

Decided on 2026-11-09: `1.0.1` now, small fixes as `1.0.x`, then `1.1.0-beta.N` to `1.1.0`. The milestones, the pull requests and the gates are in the [roadmap](ROADMAP.md).
