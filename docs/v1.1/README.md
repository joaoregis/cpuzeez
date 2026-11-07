# CpuZeeZ 1.1.0: the plan

> Status: planned. These are the corrections and small improvements that 1.0.0 leaves partly done, found by checking each [ROADMAP](../v1/ROADMAP.md) row against the code. Each item says why it is only partial today, what finishing it takes, and how we will know it is finished. Nothing here changes what 1.0.0 promises users; it makes the promises easier to trust and the code easier to change.
> Difficulty is the work to finish, not its value: **MEDIO** (a day or two, low risk), **ALTO** (several days, needs care), **MUITO ALTO** (a project of its own).

Items rated BAIXO were fixed during the 1.0.0 hardening instead of being listed here: the MSRV job (the declared `rust-version` was wrong: egui 0.36 needs 1.95; it is now declared and checked in CI) and the committed fuzz corpus.

| # | Item | ROADMAP row | Difficulty | Order |
|---|---|---|---|---|
| 1 | [Permissions map checked against the code](#1-permissions-map-checked-against-the-code) | S0-B04 | MEDIO | 1 |
| 2 | [Interaction tests for the remaining pages](#2-interaction-tests-for-the-remaining-pages) | S0-T05 | MEDIO | 2 |
| 3 | [More pages in the scale test](#3-more-pages-in-the-scale-test) | S3-03 | MEDIO | 3 |
| 4 | [An overlay frame budget that counts work](#4-an-overlay-frame-budget-that-counts-work) | S0-T08 | MEDIO | 4 |
| 5 | [A long fuzz campaign, with its results](#5-a-long-fuzz-campaign-with-its-results) | S0-T07 | MEDIO | 5 |
| 6 | [`gui/mod.rs` down to navigation and composition](#6-guimodrs-down-to-navigation-and-composition) | S0-E04 | ALTO | 6 |
| 7 | [Screenshots of the new pages on Linux and macOS](#7-screenshots-of-the-new-pages-on-linux-and-macos) | S3-07 | ALTO | 7 |
| 8 | [`unsafe` only in `platform`](#8-unsafe-only-in-platform) | S0-E06 | MUITO ALTO | 8 |

Also waiting for a free machine, not for work: the long GUI soak and the overlay's CPU and memory against the budget ([performance.md](../performance.md)), and the Windows screenshots of the new pages. Both open a window, so they are scheduled with the owner.

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

**Why it is partial.** `core` and `gui` forbid `unsafe`. The root binary's older hardware modules (`src/**` outside `gui`) have about 95 `unsafe` uses, each with a `SAFETY:` comment, but they sit next to the code that uses them instead of in `platform`.

**What finishing takes.** Move each module's OS calls behind a safe function in `platform` and leave the decoding where it is; module by module (storage, sensors, GPU, chipset, SPD, memory tests), each with its own pull request and the same tests passing. Then `#![forbid(unsafe_code)]` on the binary crate.

**Done when.** `grep -rn unsafe src/` finds nothing and the lint forbids it.

**Risks.** The largest item here: many files, FFI signatures that are easy to get subtly wrong, and no behaviour to show for it. Better done module by module while touching each one than as a single change.

---

## Order and release

Items 1–5 are independent and can ship as `1.0.x` patches if they are ready earlier. Items 6 and 8 are refactors: they go into `1.1.0` only with no visible change. Item 7 needs the screenshot jobs to be stable first.
