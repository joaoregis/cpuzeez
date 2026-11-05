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
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub const PT_BR: &[(&str, &str)] = &[
    ("Filter… e.g. name:chrome cpu>10 mem>1GB", "Filtrar… ex.: name:chrome cpu>10 mem>1GB"),
    ("Columns", "Colunas"),
    ("{} of {} rows", "{} de {} linhas"),
    ("Reset column widths", "Restaurar larguras das colunas"),
    ("Reset column order", "Restaurar ordem das colunas"),
    ("Drag a header to move its column", "Arraste um cabeçalho para mover a coluna"),
    ("Drag to resize, double-click to fit", "Arraste para redimensionar, clique duplo para ajustar"),
];
pub const ES: &[(&str, &str)] = &[
    ("Filter… e.g. name:chrome cpu>10 mem>1GB", "Filtrar… p. ej. name:chrome cpu>10 mem>1GB"),
    ("Columns", "Columnas"),
    ("{} of {} rows", "{} de {} filas"),
    ("Reset column widths", "Restablecer anchos de columna"),
    ("Reset column order", "Restablecer orden de columnas"),
    ("Drag a header to move its column", "Arrastra un encabezado para mover su columna"),
    ("Drag to resize, double-click to fit", "Arrastra para redimensionar, doble clic para ajustar"),
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
    /// Text color per row (None: the default); e.g. a hot temperature.
    pub color: Option<Box<dyn Fn(&R) -> Option<egui::Color32> + 'a>>,
    /// Cell background fill per row (None: transparent); e.g. a Task Manager heat-map cell.
    pub bg: Option<Box<dyn Fn(&R) -> Option<egui::Color32> + 'a>>,
    /// Custom cell drawing (e.g. a sparkline) instead of painting the cell text.
    pub draw: Option<Box<dyn Fn(&egui::Painter, Rect, &R) + 'a>>,
}

impl<'a, R> Column<'a, R> {
    pub fn new(id: &'static str, title: impl Into<String>, width: f32, cell: impl Fn(&R) -> Cell + 'a) -> Self {
        Column { id, title: title.into(), width, right: false, cell: Box::new(cell), color: None, bg: None, draw: None }
    }

    pub fn right(mut self) -> Self {
        self.right = true;
        self
    }

    pub fn colored(mut self, f: impl Fn(&R) -> Option<egui::Color32> + 'a) -> Self {
        self.color = Some(Box::new(f));
        self
    }

    pub fn tinted(mut self, f: impl Fn(&R) -> Option<egui::Color32> + 'a) -> Self {
        self.bg = Some(Box::new(f));
        self
    }

    pub fn drawn(mut self, f: impl Fn(&egui::Painter, Rect, &R) + 'a) -> Self {
        self.draw = Some(Box::new(f));
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
    /// Column widths the user dragged (column id -> points).
    pub widths: BTreeMap<String, f32>,
    /// Column ids in the order the user dragged them to (empty: as defined).
    pub order: Vec<String>,
    /// The column whose header is being dragged.
    #[serde(skip)]
    dragging: Option<String>,
    #[serde(skip)]
    pub selected: BTreeSet<u64>,
    /// The row the user double-clicked (or pressed Enter on) this frame.
    #[serde(skip)]
    pub activated: Option<u64>,
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

/// The visible columns in the user's order.
pub fn shown<'c, 'a, R>(cols: &'c [Column<'a, R>], st: &State) -> Vec<&'c Column<'a, R>> {
    let mut v: Vec<(usize, &Column<R>)> = cols.iter().enumerate().filter(|(_, c)| !st.hidden.contains(c.id)).collect();
    v.sort_by_key(|(i, c)| st.order.iter().position(|o| o == c.id).map_or((1, *i), |p| (0, p)));
    v.into_iter().map(|(_, c)| c).collect()
}

/// Move visible column `from` to insertion point `to` (0..=visible.len()),
/// returning the full order (hidden columns keep their slots).
pub fn move_column(full: &[String], visible: &[&str], from: usize, to: usize) -> Vec<String> {
    let mut vis: Vec<&str> = visible.to_vec();
    if from >= vis.len() {
        return full.to_vec();
    }
    let c = vis.remove(from);
    let to = if to > from { to - 1 } else { to }.min(vis.len());
    vis.insert(to, c);
    let mut next = vis.into_iter();
    full.iter()
        .map(|id| if visible.contains(&id.as_str()) { next.next().unwrap_or(id).to_string() } else { id.clone() })
        .collect()
}

/// Tab-separated text of `items`' rows (visible columns), with a header.
pub fn to_tsv<R>(rows: &[R], cols: &[Column<R>], items: &[Item], st: &State) -> String {
    let shown: Vec<&Column<R>> = shown(cols, st);
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
        let shown: Vec<&Column<R>> = shown(&self.cols, st);
        let row_h = ui.text_style_height(&egui::TextStyle::Body) + 8.0;
        let font = FontId::proportional(ui.text_style_height(&egui::TextStyle::Body) - 2.0);

        let widths: Vec<f32> = shown.iter().map(|c| st.widths.get(c.id).copied().unwrap_or(c.width)).collect();
        let total_w = 16.0 + widths.iter().sum::<f32>();
        // Header: click sorts, right-click picks columns, the edges resize. It is
        // painted after the body so it follows the body's horizontal scroll.
        let (hdr, resp) = ui.allocate_exact_size(vec2(ui.available_width(), row_h), Sense::click_and_drag());

        st.activated = None;
        self.keys(ui, rows, &items, st);
        let mut clicked: Option<(usize, egui::Modifiers)> = None;
        let mut toggle: Option<String> = None;
        let mut menu_for: Option<usize> = None;
        let mut tip: Option<String> = None;
        let mut activated: Option<u64> = None;
        let out = egui::ScrollArea::both().id_salt(self.id).auto_shrink([false, false]).show_rows(
            ui,
            row_h,
            items.len(),
            |ui, range| {
                let row_w = ui.available_width().max(total_w);
                for i in range {
                    let (r, resp) = ui.allocate_exact_size(vec2(row_w, row_h), Sense::click());
                    match &items[i] {
                        Item::Group { name, count, collapsed } => {
                            let caret = if *collapsed { theme::icon::CARET_RIGHT } else { theme::icon::CARET_DOWN };
                            let t = format!("{caret} {name}  ({count})");
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
                            let hover = resp.hover_pos();
                            for (c, &w) in shown.iter().zip(&widths) {
                                let cell_bg_rect =
                                    Rect::from_min_size(egui::pos2(x - 3.0, r.top() + 1.0), vec2(w, row_h - 2.0));
                                if !selected && let Some(fill) = c.bg.as_ref().and_then(|f| f(row)) {
                                    ui.painter().rect_filled(cell_bg_rect, egui::CornerRadius::same(3), fill);
                                }
                                let cr = Rect::from_min_size(egui::pos2(x, r.top()), vec2(w - PAD, row_h));
                                if let Some(draw) = &c.draw {
                                    draw(&ui.painter().with_clip_rect(cr), cr.shrink2(vec2(2.0, 4.0)), row);
                                } else {
                                    let color = c.color.as_ref().and_then(|f| f(row)).unwrap_or(p.text);
                                    let cell = (c.cell)(row);
                                    let g = fit(ui.painter(), cell.shown(), &font, color, cr.width());
                                    let elided = g.elided;
                                    cell_text(ui.painter(), cr, c.right, g, color);
                                    if elided && hover.is_some_and(|h| cr.contains(h)) {
                                        tip = Some(cell.shown().to_string());
                                    }
                                }
                                x += w;
                            }
                            let resp = match tip.take() {
                                Some(t) => resp.on_hover_text(t),
                                None => resp,
                            };
                            if resp.double_clicked() {
                                activated = Some((self.key)(row));
                            }
                            if resp.clicked() {
                                clicked = Some((i, ui.input(|inp| inp.modifiers)));
                            }
                            if resp.secondary_clicked() {
                                menu_for = Some(i);
                            }
                            resp.context_menu(|ui| {
                                // Never wrap menu items into a narrow column.
                                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                                // Right-clicking an unselected row acts on that row: its
                                // selection lands after this frame, and an empty first
                                // frame would leave the menu stuck at zero width.
                                let sel: Vec<&R> = if selected {
                                    rows.iter().filter(|r| st.selected.contains(&(self.key)(r))).collect()
                                } else {
                                    vec![row]
                                };
                                menu(ui, &sel);
                            });
                        }
                    }
                }
            },
        );
        self.header(
            ui,
            Header {
                rect: hdr,
                resp: &resp,
                shown: &shown,
                widths: &widths,
                offset_x: out.state.offset.x,
                row_h,
                font: &font,
            },
            rows,
            st,
        );
        if activated.is_some() {
            st.activated = activated;
        }
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

    fn header(&self, ui: &mut Ui, h: Header<'_, 'a, R>, rows: &[R], st: &mut State) {
        let p = theme::palette(ui);
        let (hdr, row_h, font) = (h.rect, h.row_h, h.font);
        let painter = ui.painter().with_clip_rect(hdr);
        let mut x = hdr.left() + 8.0 - h.offset_x;
        let mut on_edge = false;
        // Column left edges, for dragging a header to a new place.
        let mut edges: Vec<f32> = Vec::with_capacity(h.shown.len() + 1);
        for (c, &w) in h.shown.iter().zip(h.widths) {
            edges.push(x);
            let r = Rect::from_min_size(egui::pos2(x, hdr.top()), vec2(w, row_h));
            // Phosphor carets: the UI fonts have no arrow glyphs on every OS.
            let arrow = match &st.sort {
                Some((id, true)) if id == c.id => format!(" {}", theme::icon::CARET_UP),
                Some((id, false)) if id == c.id => format!(" {}", theme::icon::CARET_DOWN),
                _ => String::new(),
            };
            let title = format!("{}{arrow}", c.title);
            let g = fit(&painter, &title, font, p.text_secondary, w - 10.0);
            let text_rect = Rect::from_min_max(r.min, egui::pos2(r.right() - PAD, r.bottom()));
            cell_text(&painter, text_rect, c.right, g, p.text_secondary);

            // Resize handle on the right edge.
            let edge = Rect::from_center_size(egui::pos2(r.right() - 3.0, r.center().y), vec2(8.0, row_h));
            if edge.intersects(hdr) {
                let hr = ui
                    .interact(
                        edge.intersect(hdr),
                        egui::Id::new((self.id, "col-resize", c.id)),
                        Sense::click_and_drag(),
                    )
                    .on_hover_text(tr("Drag to resize, double-click to fit"));
                let active = hr.hovered() || hr.dragged();
                on_edge |= active;
                if active {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                }
                let stroke = if active { Stroke::new(2.0, p.accent) } else { Stroke::new(1.0, p.border) };
                painter.vline(r.right() - 3.0, r.y_range().shrink(if active { 2.0 } else { 7.0 }), stroke);
                if hr.dragged() {
                    st.widths.insert(c.id.into(), (w + hr.drag_delta().x).max(MIN_W));
                }
                if hr.double_clicked() {
                    st.widths.insert(c.id.into(), fit_width(&painter, rows, c, &title, font));
                }
            }

            // Where the press began: a fast drag is already over the next column when it starts.
            let origin = ui.input(|i| i.pointer.press_origin());
            if !on_edge && h.resp.drag_started() && origin.is_some_and(|pos| r.contains(pos)) {
                st.dragging = Some(c.id.into());
            }
            if !on_edge && h.resp.clicked() && h.resp.interact_pointer_pos().is_some_and(|pos| r.contains(pos)) {
                st.sort = match &st.sort {
                    Some((id, true)) if id == c.id => Some((c.id.into(), false)),
                    Some((id, false)) if id == c.id => None,
                    _ => Some((c.id.into(), true)),
                };
            }
            x += w;
        }
        edges.push(x);
        if let Some(id) = st.dragging.clone() {
            let from = h.shown.iter().position(|c| c.id == id);
            let px = ui.ctx().pointer_latest_pos().map_or(0.0, |p| p.x);
            // The insertion point nearest the pointer.
            let to = edges
                .iter()
                .enumerate()
                .min_by(|a, b| (a.1 - px).abs().total_cmp(&(b.1 - px).abs()))
                .map_or(0, |e| e.0);
            if h.resp.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                painter.vline(edges[to] - 4.0, hdr.y_range(), Stroke::new(2.0, p.accent));
            } else {
                st.dragging = None;
                if let Some(from) = from
                    && to != from
                    && to != from + 1
                {
                    let full: Vec<String> = if st.order.is_empty() {
                        self.cols.iter().map(|c| c.id.to_string()).collect()
                    } else {
                        let mut f = st.order.clone();
                        f.extend(self.cols.iter().filter(|c| !st.order.iter().any(|o| o == c.id)).map(|c| c.id.into()));
                        f
                    };
                    let vis: Vec<&str> = h.shown.iter().map(|c| c.id).collect();
                    st.order = move_column(&full, &vis, from, to);
                }
            }
        }
        // Painted text is invisible to screen readers: describe the header.
        let header: Vec<&str> = h.shown.iter().map(|c| c.title.as_str()).collect();
        h.resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, header.join(", ")));
        h.resp.context_menu(|ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            ui.label(egui::RichText::new(tr("Columns")).strong());
            for c in &self.cols {
                let mut on = !st.hidden.contains(c.id);
                if ui.checkbox(&mut on, &c.title).changed() {
                    if on {
                        st.hidden.remove(c.id);
                    } else if h.shown.len() > 1 {
                        st.hidden.insert(c.id.into());
                    }
                }
            }
            ui.separator();
            if ui.button(tr("Reset column widths")).clicked() {
                st.widths.clear();
                ui.close();
            }
            if ui.add_enabled(!st.order.is_empty(), egui::Button::new(tr("Reset column order"))).clicked() {
                st.order.clear();
                ui.close();
            }
            ui.label(egui::RichText::new(tr("Drag a header to move its column")).small().weak());
        });
        ui.painter().hline(hdr.x_range(), hdr.bottom(), Stroke::new(1.0, p.border));
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
        let (up, down, all, esc, copy, enter) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
                i.modifiers.command && i.key_pressed(egui::Key::A),
                i.key_pressed(egui::Key::Escape),
                i.modifiers.command && i.key_pressed(egui::Key::C),
                i.key_pressed(egui::Key::Enter),
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
        if enter && st.selected.len() == 1 {
            st.activated = st.anchor;
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

const MIN_W: f32 = 36.0;
/// Space between a cell's text and the next column.
const PAD: f32 = 12.0;

/// What the header needs from the frame's layout.
struct Header<'h, 'a, R> {
    rect: Rect,
    resp: &'h egui::Response,
    shown: &'h [&'h Column<'a, R>],
    widths: &'h [f32],
    offset_x: f32,
    row_h: f32,
    font: &'h FontId,
}

/// One line of text cut with an ellipsis at `w`.
fn fit(painter: &egui::Painter, text: &str, font: &FontId, color: egui::Color32, w: f32) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), font.clone(), color);
    job.wrap = egui::text::TextWrapping::truncate_at_width(w.max(1.0));
    painter.layout_job(job)
}

fn cell_text(painter: &egui::Painter, r: Rect, right: bool, g: Arc<egui::Galley>, color: egui::Color32) {
    let y = r.center().y - g.size().y / 2.0;
    let x = if right { r.right() - g.size().x } else { r.left() };
    painter.galley(egui::pos2(x, y), g, color);
}

/// The width that shows the title and the widest cell (first 2000 rows) whole.
fn fit_width<R>(painter: &egui::Painter, rows: &[R], c: &Column<R>, title: &str, font: &FontId) -> f32 {
    if c.draw.is_some() {
        return c.width; // a sparkline has no text width
    }
    let text_w = |t: &str| painter.layout_no_wrap(t.to_owned(), font.clone(), egui::Color32::WHITE).size().x;
    let widest = rows.iter().take(2000).map(|r| text_w((c.cell)(r).shown())).fold(text_w(title) + 4.0, f32::max);
    (widest + 16.0).clamp(MIN_W, 900.0)
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
    fn columns_move_and_keep_their_order() {
        let full: Vec<String> = ["name", "pid", "cpu", "mem"].map(String::from).to_vec();
        // pid is hidden: moving cpu before name leaves pid's slot alone.
        let vis = ["name", "cpu", "mem"];
        assert_eq!(move_column(&full, &vis, 1, 0), ["cpu", "pid", "name", "mem"]);
        assert_eq!(move_column(&full, &vis, 0, 3), ["cpu", "pid", "mem", "name"], "to the end");
        assert_eq!(move_column(&full, &vis, 0, 1), full, "dropped where it was");
        let c = cols();
        let st = State { order: vec!["cpu".into(), "name".into()], ..Default::default() };
        let ids: Vec<&str> = shown(&c, &st).iter().map(|c| c.id).collect();
        assert_eq!(ids, ["cpu", "name", "pid"], "listed first, the rest as defined");
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

    /// Right-clicking an unselected row opens its menu for that row on the very
    /// first frame (an empty first frame left the menu stuck at zero width).
    #[test]
    fn right_click_menu_gets_the_row_at_once() {
        use egui_kittest::{Harness, kittest::Queryable};
        let r = rows();
        let mut h = Harness::new_ui_state(
            |ui, seen: &mut Vec<Vec<u64>>| {
                let mut st = ui.data_mut(|d| d.get_temp::<State>(egui::Id::new("st")).unwrap_or_default());
                let t = Table { id: "procs", cols: cols(), key: Box::new(|p: &P| p.pid), group: None };
                t.show(ui, &r, &mut st, |_, sel| seen.push(sel.iter().map(|p| p.pid).collect()));
                ui.data_mut(|d| d.insert_temp(egui::Id::new("st"), st));
            },
            Vec::new(),
        );
        h.run();
        h.get_by_label("game.exe, 99, 55.0 %").click_secondary();
        h.run();
        assert!(!h.state().is_empty(), "the menu opened");
        assert!(
            h.state().iter().all(|s| s == &[99]),
            "every menu frame acts on the right-clicked row: {:?}",
            h.state()
        );
    }

    /// The header with a real mouse: drag a column edge to resize it, drag a
    /// title to move the column, click a title to sort.
    #[test]
    fn header_resizes_moves_and_sorts_with_the_mouse() {
        use egui_kittest::{Harness, kittest::Queryable};
        let r = rows();
        let mut h = Harness::builder().with_size(egui::vec2(800.0, 400.0)).build_ui_state(
            |ui, st: &mut State| {
                let t = Table { id: "procs", cols: cols(), key: Box::new(|p: &P| p.pid), group: None };
                t.show(ui, &r, st, |_, _| {});
            },
            State::default(),
        );
        h.run();
        let hdr = h.get_by_label("Name, PID, CPU").rect();
        let y = hdr.center().y;
        let drag = |h: &mut Harness<State>, from: egui::Pos2, to: egui::Pos2| {
            h.hover_at(from);
            h.run();
            h.drag_at(from);
            h.run();
            for i in 1..=5 {
                h.hover_at(from + (to - from) * (i as f32 / 5.0));
                h.run();
            }
            h.drop_at(to);
            h.run();
        };
        // Name is 200 wide from x+8: its edge handle sits at x+205.
        let edge = egui::pos2(hdr.left() + 8.0 + 200.0 - 3.0, y);
        drag(&mut h, edge, edge + egui::vec2(60.0, 0.0));
        let w = h.state().widths.get("name").copied().unwrap_or_default();
        assert!((w - 260.0).abs() < 2.0, "dragged 60 px wider: {w}");
        // Drag the CPU title (x+8+260+60 .. ) to the far left.
        let cpu = egui::pos2(hdr.left() + 8.0 + 260.0 + 60.0 + 30.0, y);
        drag(&mut h, cpu, egui::pos2(hdr.left() + 4.0, y));
        assert_eq!(h.state().order.first().map(String::as_str), Some("cpu"), "{:?}", h.state().order);
        assert!(h.query_by_label("CPU, Name, PID").is_some(), "the header reads in the new order");
        assert_eq!(h.state().sort, None, "a drag isn't a click");
        // A click on the (now first) CPU title sorts by it.
        let at = egui::pos2(hdr.left() + 30.0, y);
        h.hover_at(at);
        h.run();
        h.drag_at(at);
        h.run();
        h.drop_at(at);
        h.run();
        assert_eq!(h.state().sort, Some(("cpu".into(), true)));
    }

    #[test]
    fn ten_thousand_rows_filter_and_sort_quickly() {
        let _heavy = crate::heavy_test();
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
