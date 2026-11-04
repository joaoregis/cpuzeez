//! The one table (docs/v1/engineering.md §4.3), for processes, services,
//! startup items, users, sensors, SMART and history.
//!
//! - Columns with a hide/show menu on the header, click-to-sort (asc, desc, off).
//! - The filter syntax of `cpuzeez_core::query` (`name:chrome cpu>10`).
//! - Optional collapsible groups.
//! - Multi-select (click, Ctrl/Cmd, Shift) and the keyboard (↑/↓, Ctrl+A, Esc,
//!   Ctrl+C copies the selection as tab-separated text).
//! - A context menu supplied by the caller (it builds `Action`s).
//! - Only the visible rows are laid out (`ScrollArea::show_rows`), so tens of
//!   thousands of rows stay cheap.
//!
//! Filtering, sorting and grouping are a pure function ([`view`]) so they are
//! tested without a GUI.

use super::i18n::tr;
use super::theme;
use cpuzeez_core::query::{self, Value};
use eframe::egui::{self, Align2, FontId, Rect, Sense, Stroke, Ui, vec2};
use std::cmp::Ordering;
use std::collections::BTreeSet;

pub const PT_BR: &[(&str, &str)] = &[
    ("Filter… e.g. name:chrome cpu>10 mem>1GB", "Filtrar… ex.: name:chrome cpu>10 mem>1GB"),
    ("Columns", "Colunas"),
    ("{} of {} rows", "{} de {} linhas"),
];
pub const ES: &[(&str, &str)] = &[
    ("Filter… e.g. name:chrome cpu>10 mem>1GB", "Filtrar… p. ej. name:chrome cpu>10 mem>1GB"),
    ("Columns", "Columnas"),
    ("{} of {} rows", "{} de {} filas"),
];

/// One cell. Numbers keep their value (for sorting and `cpu>10`) and a display text.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Text(String),
    Num { value: f64, shown: String },
}

impl Cell {
    pub fn num(value: f64, shown: impl Into<String>) -> Cell {
        Cell::Num { value, shown: shown.into() }
    }

    fn shown(&self) -> &str {
        match self {
            Cell::Text(s) | Cell::Num { shown: s, .. } => s,
        }
    }

    fn query_value(&self) -> Value<'_> {
        match self {
            Cell::Text(s) => Value::Text(s),
            Cell::Num { value, .. } => Value::Num(*value),
        }
    }

    fn cmp(&self, o: &Cell) -> Ordering {
        match (self, o) {
            (Cell::Num { value: a, .. }, Cell::Num { value: b, .. }) => a.total_cmp(b),
            (a, b) => a.shown().to_lowercase().cmp(&b.shown().to_lowercase()),
        }
    }
}

pub struct Column<'a, R> {
    /// Stable key: filter field (`name:`), sort and visibility state.
    pub id: &'static str,
    /// Header text, already translated.
    pub title: String,
    pub width: f32,
    /// Right-aligned (numbers).
    pub right: bool,
    pub cell: Box<dyn Fn(&R) -> Cell + 'a>,
}

impl<'a, R> Column<'a, R> {
    pub fn new(id: &'static str, title: impl Into<String>, width: f32, cell: impl Fn(&R) -> Cell + 'a) -> Self {
        Column { id, title: title.into(), width, right: false, cell: Box::new(cell) }
    }

    pub fn right(mut self) -> Self {
        self.right = true;
        self
    }
}

/// What the user set on a table; keep one per table across frames.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct State {
    pub filter: String,
    /// (column id, ascending).
    pub sort: Option<(String, bool)>,
    pub hidden: BTreeSet<String>,
    pub collapsed: BTreeSet<String>,
    #[serde(skip)]
    pub selected: BTreeSet<u64>,
    #[serde(skip)]
    anchor: Option<u64>,
    #[serde(skip)]
    focused: bool,
}

/// A line of the table: a group header or a row (index into the caller's rows).
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Group { name: String, count: usize, collapsed: bool },
    Row(usize),
}

/// Filter, sort and group `rows` into the lines to show.
pub fn view<R>(rows: &[R], cols: &[Column<R>], st: &State, group: Option<&dyn Fn(&R) -> String>) -> Vec<Item> {
    let terms = query::parse(&st.filter);
    let cells: Vec<Vec<Cell>> = rows.iter().map(|r| cols.iter().map(|c| (c.cell)(r)).collect()).collect();
    let mut idx: Vec<usize> = (0..rows.len())
        .filter(|&i| {
            let vals: Vec<Value> = cells[i].iter().map(Cell::query_value).collect();
            let by =
                |f: &str| cols.iter().position(|c| c.id.eq_ignore_ascii_case(f)).map(|k| cells[i][k].query_value());
            query::matches(&terms, by, &vals)
        })
        .collect();
    if let Some((id, asc)) = &st.sort
        && let Some(k) = cols.iter().position(|c| c.id == id)
    {
        idx.sort_by(|&a, &b| {
            let o = cells[a][k].cmp(&cells[b][k]);
            if *asc { o } else { o.reverse() }
        });
    }
    let Some(group) = group else { return idx.into_iter().map(Item::Row).collect() };
    // Groups in first-appearance order, rows keep the sort inside each group.
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for i in idx {
        let g = group(&rows[i]);
        match groups.iter_mut().find(|x| x.0 == g) {
            Some(x) => x.1.push(i),
            None => groups.push((g, vec![i])),
        }
    }
    let mut out = Vec::new();
    for (name, members) in groups {
        let collapsed = st.collapsed.contains(&name);
        out.push(Item::Group { name, count: members.len(), collapsed });
        if !collapsed {
            out.extend(members.into_iter().map(Item::Row));
        }
    }
    out
}

/// Tab-separated text of `items`' rows (visible columns), with a header.
pub fn to_tsv<R>(rows: &[R], cols: &[Column<R>], items: &[Item], st: &State) -> String {
    let shown: Vec<&Column<R>> = cols.iter().filter(|c| !st.hidden.contains(c.id)).collect();
    let clean = |s: &str| s.replace(['\t', '\n'], " ");
    let mut s = shown.iter().map(|c| clean(&c.title)).collect::<Vec<_>>().join("\t") + "\n";
    for it in items {
        if let Item::Row(i) = it {
            s += &shown.iter().map(|c| clean((c.cell)(&rows[*i]).shown())).collect::<Vec<_>>().join("\t");
            s.push('\n');
        }
    }
    s
}

pub struct Table<'a, R> {
    pub id: &'static str,
    pub cols: Vec<Column<'a, R>>,
    /// Stable row identity for selection (e.g. a PID).
    pub key: Box<dyn Fn(&R) -> u64 + 'a>,
    pub group: Option<Box<dyn Fn(&R) -> String + 'a>>,
}

impl<'a, R> Table<'a, R> {
    pub fn new(id: &'static str, key: impl Fn(&R) -> u64 + 'a) -> Self {
        Table { id, cols: Vec::new(), key: Box::new(key), group: None }
    }

    pub fn col(mut self, c: Column<'a, R>) -> Self {
        self.cols.push(c);
        self
    }

    pub fn grouped(mut self, g: impl Fn(&R) -> String + 'a) -> Self {
        self.group = Some(Box::new(g));
        self
    }

    /// Filter box, header and body. `menu` fills the right-click menu for the selection.
    pub fn show(&self, ui: &mut Ui, rows: &[R], st: &mut State, mut menu: impl FnMut(&mut Ui, &[&R])) {
        let p = theme::palette(ui);
        let items = view(rows, &self.cols, st, self.group.as_deref());
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut st.filter)
                    .hint_text(tr("Filter… e.g. name:chrome cpu>10 mem>1GB"))
                    .desired_width(320.0),
            );
            let n = items.iter().filter(|i| matches!(i, Item::Row(_))).count();
            ui.label(
                egui::RichText::new(super::i18n::trf("{} of {} rows", &[&n, &rows.len()])).small().color(p.text_muted),
            );
        });
        let shown: Vec<&Column<R>> = self.cols.iter().filter(|c| !st.hidden.contains(c.id)).collect();
        let row_h = ui.text_style_height(&egui::TextStyle::Body) + 8.0;
        let font = FontId::proportional(ui.text_style_height(&egui::TextStyle::Body) - 2.0);

        // Header: click sorts, right-click picks columns.
        let (hdr, resp) = ui.allocate_exact_size(vec2(ui.available_width(), row_h), Sense::click());
        let mut x = hdr.left() + 8.0;
        for c in &shown {
            let r = Rect::from_min_size(egui::pos2(x, hdr.top()), vec2(c.width, row_h));
            let arrow = match &st.sort {
                Some((id, true)) if id == c.id => " ▲",
                Some((id, false)) if id == c.id => " ▼",
                _ => "",
            };
            let (anchor, tx) =
                if c.right { (Align2::RIGHT_CENTER, r.right() - 6.0) } else { (Align2::LEFT_CENTER, r.left()) };
            ui.painter().with_clip_rect(r).text(
                egui::pos2(tx, r.center().y),
                anchor,
                format!("{}{arrow}", c.title),
                font.clone(),
                p.text_secondary,
            );
            if resp.clicked() && resp.interact_pointer_pos().is_some_and(|pos| r.contains(pos)) {
                st.sort = match &st.sort {
                    Some((id, true)) if id == c.id => Some((c.id.into(), false)),
                    Some((id, false)) if id == c.id => None,
                    _ => Some((c.id.into(), true)),
                };
            }
            x += c.width;
        }
        // Painted text is invisible to screen readers: describe the header.
        let header: Vec<&str> = shown.iter().map(|c| c.title.as_str()).collect();
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, header.join(", ")));
        resp.context_menu(|ui| {
            ui.label(egui::RichText::new(tr("Columns")).strong());
            for c in &self.cols {
                let mut on = !st.hidden.contains(c.id);
                if ui.checkbox(&mut on, &c.title).changed() {
                    if on {
                        st.hidden.remove(c.id);
                    } else if shown.len() > 1 {
                        st.hidden.insert(c.id.into());
                    }
                }
            }
        });
        ui.painter().hline(hdr.x_range(), hdr.bottom(), Stroke::new(1.0, p.border));

        self.keys(ui, rows, &items, st);
        let mut clicked: Option<(usize, egui::Modifiers)> = None;
        let mut toggle: Option<String> = None;
        let mut menu_for: Option<usize> = None;
        egui::ScrollArea::vertical().id_salt(self.id).auto_shrink([false, false]).show_rows(
            ui,
            row_h,
            items.len(),
            |ui, range| {
                for i in range {
                    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), row_h), Sense::click());
                    match &items[i] {
                        Item::Group { name, count, collapsed } => {
                            let t = format!("{} {name}  ({count})", if *collapsed { "▸" } else { "▾" });
                            ui.painter().text(
                                egui::pos2(r.left() + 6.0, r.center().y),
                                Align2::LEFT_CENTER,
                                t,
                                font.clone(),
                                p.text,
                            );
                            let state = if *collapsed { "collapsed" } else { "expanded" };
                            let label = format!("{name}, {count}, {state}");
                            resp.widget_info(|| {
                                egui::WidgetInfo::labeled(egui::WidgetType::CollapsingHeader, true, &label)
                            });
                            if resp.clicked() {
                                toggle = Some(name.clone());
                            }
                        }
                        Item::Row(k) => {
                            let row = &rows[*k];
                            let selected = st.selected.contains(&(self.key)(row));
                            let bg = if selected {
                                p.accent_soft
                            } else if resp.hovered() {
                                p.card_hover
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            ui.painter().rect_filled(r, egui::CornerRadius::same(4), bg);
                            // The row as text for screen readers (and the GUI tests).
                            let text: Vec<String> = shown.iter().map(|c| (c.cell)(row).shown().to_string()).collect();
                            let label = text.join(", ");
                            resp.widget_info(|| {
                                egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, &label)
                            });
                            let mut x = r.left() + 8.0;
                            for c in &shown {
                                let cr = Rect::from_min_size(egui::pos2(x, r.top()), vec2(c.width - 6.0, row_h));
                                let (anchor, tx) = if c.right {
                                    (Align2::RIGHT_CENTER, cr.right())
                                } else {
                                    (Align2::LEFT_CENTER, cr.left())
                                };
                                ui.painter().with_clip_rect(cr).text(
                                    egui::pos2(tx, cr.center().y),
                                    anchor,
                                    (c.cell)(row).shown(),
                                    font.clone(),
                                    p.text,
                                );
                                x += c.width;
                            }
                            if resp.clicked() {
                                clicked = Some((i, ui.input(|inp| inp.modifiers)));
                            }
                            if resp.secondary_clicked() {
                                menu_for = Some(i);
                            }
                            resp.context_menu(|ui| {
                                let sel: Vec<&R> =
                                    rows.iter().filter(|r| st.selected.contains(&(self.key)(r))).collect();
                                menu(ui, &sel);
                            });
                        }
                    }
                }
            },
        );
        if let Some(g) = toggle
            && !st.collapsed.remove(&g)
        {
            st.collapsed.insert(g);
        }
        // A right-click on an unselected row selects it first, like every file manager.
        if let Some(i) = menu_for
            && let Item::Row(k) = items[i]
            && !st.selected.contains(&(self.key)(&rows[k]))
        {
            clicked = Some((i, egui::Modifiers::NONE));
        }
        if let Some((i, m)) = clicked {
            st.focused = true;
            self.select(rows, &items, st, i, m);
        }
    }

    fn select(&self, rows: &[R], items: &[Item], st: &mut State, i: usize, m: egui::Modifiers) {
        let Item::Row(k) = items[i] else { return };
        let key = (self.key)(&rows[k]);
        if m.shift
            && let Some(a) = st.anchor
            && let Some(ai) = items.iter().position(|it| matches!(it, Item::Row(j) if (self.key)(&rows[*j]) == a))
        {
            let (lo, hi) = (ai.min(i), ai.max(i));
            if !m.command {
                st.selected.clear();
            }
            st.selected.extend(
                items[lo..=hi]
                    .iter()
                    .filter_map(|it| if let Item::Row(j) = it { Some((self.key)(&rows[*j])) } else { None }),
            );
            return;
        }
        if m.command {
            if !st.selected.remove(&key) {
                st.selected.insert(key);
            }
        } else {
            st.selected.clear();
            st.selected.insert(key);
        }
        st.anchor = Some(key);
    }

    /// ↑/↓ move the selection, Ctrl+A selects all, Esc clears, Ctrl+C copies.
    fn keys(&self, ui: &mut Ui, rows: &[R], items: &[Item], st: &mut State) {
        if !st.focused || ui.ctx().memory(|m| m.focused().is_some()) {
            return; // a text field (the filter) has the keyboard
        }
        let pos = |st: &State| {
            items.iter().position(|it| matches!(it, Item::Row(j) if st.anchor == Some((self.key)(&rows[*j]))))
        };
        let (up, down, all, esc, copy) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
                i.modifiers.command && i.key_pressed(egui::Key::A),
                i.key_pressed(egui::Key::Escape),
                i.modifiers.command && i.key_pressed(egui::Key::C),
            )
        });
        let step = |from: Option<usize>, back: bool| -> Option<usize> {
            let mut i = from.unwrap_or(if back { items.len() } else { usize::MAX });
            loop {
                i = if back { i.checked_sub(1)? } else { i.wrapping_add(1) };
                if i >= items.len() {
                    return None;
                }
                if matches!(items[i], Item::Row(_)) {
                    return Some(i);
                }
            }
        };
        if (up || down)
            && let Some(i) = step(pos(st), up)
        {
            self.select(rows, items, st, i, egui::Modifiers::NONE);
        }
        if all {
            st.selected = items
                .iter()
                .filter_map(|it| if let Item::Row(j) = it { Some((self.key)(&rows[*j])) } else { None })
                .collect();
        }
        if esc {
            st.selected.clear();
        }
        if copy && !st.selected.is_empty() {
            let sel: Vec<Item> = items
                .iter()
                .filter(|it| matches!(it, Item::Row(j) if st.selected.contains(&(self.key)(&rows[*j]))))
                .cloned()
                .collect();
            ui.ctx().copy_text(to_tsv(rows, &self.cols, &sel, st));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct P {
        pid: u64,
        name: &'static str,
        cpu: f64,
        kind: &'static str,
    }

    fn cols<'a>() -> Vec<Column<'a, P>> {
        vec![
            Column::new("name", "Name", 200.0, |p: &P| Cell::Text(p.name.into())),
            Column::new("pid", "PID", 60.0, |p: &P| Cell::num(p.pid as f64, p.pid.to_string())).right(),
            Column::new("cpu", "CPU", 60.0, |p: &P| Cell::num(p.cpu, format!("{:.1} %", p.cpu))).right(),
        ]
    }

    fn rows() -> Vec<P> {
        vec![
            P { pid: 10, name: "svchost.exe", cpu: 0.5, kind: "Windows" },
            P { pid: 42, name: "chrome.exe", cpu: 12.0, kind: "Apps" },
            P { pid: 7, name: "chrome.exe", cpu: 3.0, kind: "Apps" },
            P { pid: 99, name: "game.exe", cpu: 55.0, kind: "Apps" },
        ]
    }

    fn pids(rows: &[P], items: &[Item]) -> Vec<u64> {
        items.iter().filter_map(|i| if let Item::Row(k) = i { Some(rows[*k].pid) } else { None }).collect()
    }

    #[test]
    fn filter_sort_and_group() {
        let (r, c) = (rows(), cols());
        let mut st = State { filter: "chrome".into(), ..Default::default() };
        assert_eq!(pids(&r, &view(&r, &c, &st, None)), [42, 7]);
        st.filter = "cpu>2".into();
        st.sort = Some(("cpu".into(), false));
        assert_eq!(pids(&r, &view(&r, &c, &st, None)), [99, 42, 7], "numeric filter, numeric sort descending");
        st.sort = Some(("name".into(), true));
        st.filter.clear();
        assert_eq!(pids(&r, &view(&r, &c, &st, None))[0], 42, "text sort is case-insensitive and stable");
        let g = |p: &P| p.kind.to_string();
        let v = view(&r, &c, &st, Some(&g));
        assert_eq!(v[0], Item::Group { name: "Apps".into(), count: 3, collapsed: false });
        st.collapsed.insert("Apps".into());
        let v = view(&r, &c, &st, Some(&g));
        assert_eq!(pids(&r, &v), [10], "a collapsed group hides its rows");
        assert_eq!(v.len(), 3, "both headers and the Windows row");
    }

    #[test]
    fn tsv_export_has_visible_columns_only() {
        let (r, c) = (rows(), cols());
        let st = State { hidden: ["pid".to_string()].into(), filter: "game".into(), ..Default::default() };
        let items = view(&r, &c, &st, None);
        assert_eq!(to_tsv(&r, &c, &items, &st), "Name\tCPU\ngame.exe\t55.0 %\n");
    }

    /// GUI interaction through the accessibility tree (egui_kittest).
    #[test]
    fn rows_are_readable_selectable_and_filterable() {
        use egui_kittest::{Harness, kittest::Queryable};
        let r = rows();
        let mut h = Harness::new_ui_state(
            |ui, st: &mut State| {
                let t = Table { id: "procs", cols: cols(), key: Box::new(|p: &P| p.pid), group: None };
                t.show(ui, &r, st, |_, _| {});
            },
            State::default(),
        );
        h.run();
        h.get_by_label("game.exe, 99, 55.0 %").click();
        h.run();
        assert_eq!(h.state().selected, [99].into());
        h.get_by_label("chrome.exe, 42, 12.0 %").click_modifiers(egui::Modifiers::COMMAND);
        h.run();
        assert_eq!(h.state().selected, [42, 99].into(), "Ctrl/Cmd+click adds to the selection");
        h.get_by_role(egui::accesskit::Role::TextInput).click();
        h.run();
        h.get_by_role(egui::accesskit::Role::TextInput).type_text("cpu>50");
        h.run(); // the filter applies to the next frame's rows
        h.run();
        assert!(h.query_by_label("chrome.exe, 42, 12.0 %").is_none(), "filtered out");
        assert!(h.query_by_label("game.exe, 99, 55.0 %").is_some());
    }

    #[test]
    fn ten_thousand_rows_filter_and_sort_quickly() {
        let r: Vec<P> = (0..10_000)
            .map(|i| P { pid: i, name: if i % 3 == 0 { "a.exe" } else { "b.exe" }, cpu: (i % 97) as f64, kind: "Apps" })
            .collect();
        let c = cols();
        let st = State { filter: "name:a cpu>50".into(), sort: Some(("cpu".into(), false)), ..Default::default() };
        let t = std::time::Instant::now();
        let v = view(&r, &c, &st, None);
        let ms = t.elapsed().as_secs_f64() * 1e3;
        assert!(!v.is_empty());
        // Generous for debug builds on CI; a release build is far below this.
        assert!(ms < 500.0, "view() of 10k rows took {ms:.1} ms");
    }
}
