# Accessibility (S3-03)

What CPU ZeeZ guarantees, how it is tested, and what it does not cover yet. The tests are in `src/gui/a11y.rs`, `src/gui/table.rs` and `src/gui/widgets.rs`.

## Checklist

| Area | Rule | Automated check |
|---|---|---|
| **Text contrast** | Every text colour reaches 4.5:1 (WCAG AA) on every surface it sits on, in both themes | `text_colours_meet_aa_on_every_surface` |
| **Chart contrast** | Series colours reach 3:1 against the chart background | `chart_colours_meet_the_graphics_minimum` |
| **Colour is never the only signal** | Warning and danger marks carry an icon; process classes are told apart by more than hue | `warning_and_danger_marks_carry_an_icon`, `class_colours_are_distinct_and_readable` |
| **Scale** | Pages stay readable and nothing overlaps at 75%, 100%, 125%, 150% and 200% | `*_holds_together_at_every_scale` (Bottleneck, Services, Users, App history, Processes) |
| **Keyboard: navigation** | Sidebar items and tabs are reachable and activated with the keyboard; focus is drawn as a 2 px ring | `sidebar_and_tabs_work_from_the_keyboard` |
| **Keyboard: tables** | ↑/↓ move the selection, Enter opens, Ctrl+A selects all, Esc clears, Ctrl+C copies | `the_keyboard_selects_and_activates` |
| **Names and states** | Painted widgets (sidebar items, progress bars, segmented choices, bottleneck meters) expose a name, a state and a value to assistive technology through AccessKit | kittest queries in the tests above |

Contrast uses the WCAG relative-luminance formula; the tests compute it from the palette in `src/gui/theme.rs`, so a palette change that breaks AA fails the build.

## Colour use

| Colour | Meaning | Also shown as |
|---|---|---|
| green | OK | the value in text |
| amber | warning | warning icon before the value |
| red | danger | octagon-warning icon before the value |
| series colours | one line per source | legend with the name; hover shows the value |

## Not covered yet

- **Screen readers on painted tables and charts.** Table rows and plots are drawn, not built from widgets. A chart has no text equivalent beyond the legend and the hover value. Users who need it can export the report (HTML/CSV/JSON), which is plain text.
- **A human pass.** Contrast and structure are checked by machine. A person using a screen reader or keyboard only must still go through the app; that is in the manual round before 1.0.
