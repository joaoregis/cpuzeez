//! Tools screens: benchmark History and Compare reports, plus the small line
//! chart they and the memory benchmark share.

use super::i18n::{tr, trf};
use super::theme::{self, Palette, icon};
use super::widgets;
use crate::diff::{Change, ChangeKind};
use crate::history::{self, Entry};
use crate::report::Section;
use eframe::egui::{self, Color32, Pos2, RichText, Stroke};

pub const PT_BR: &[(&str, &str)] = &[
    ("Benchmark history", "Histórico de benchmarks"),
    ("Compare reports", "Comparar relatórios"),
    ("Kind", "Tipo"),
    ("All", "Todos"),
    ("Date", "Data"),
    ("Label", "Rótulo"),
    ("Score", "Pontuação"),
    ("Threads", "Threads"),
    ("Variant", "Variante"),
    ("Processor", "Processador"),
    ("Delete", "Excluir"),
    ("Reload", "Recarregar"),
    (
        "No saved results yet. Run a benchmark and press \"Save to history\".",
        "Nenhum resultado salvo ainda. Rode um benchmark e clique em \"Salvar no histórico\".",
    ),
    ("Select two entries to compare them.", "Selecione duas entradas para compará-las."),
    ("Difference", "Diferença"),
    ("Before", "Antes"),
    ("After", "Depois"),
    ("Mainboard", "Placa-mãe"),
    ("BIOS", "BIOS"),
    ("Memory", "Memória"),
    ("Operating system", "Sistema operacional"),
    ("Version", "Versão"),
    ("Scores over time", "Pontuações ao longo do tempo"),
    ("CPU Single Thread", "CPU Single Thread"),
    ("CPU Multi Thread", "CPU Multi Thread"),
    ("better", "melhor"),
    ("worse", "pior"),
    ("Old report (A)", "Relatório antigo (A)"),
    ("New report (B)", "Relatório novo (B)"),
    ("Current system", "Sistema atual"),
    ("Compare the current system with a file", "Comparar o sistema atual com um arquivo"),
    (
        "Include live values (clocks, temperatures, voltages…)",
        "Incluir valores ao vivo (clocks, temperaturas, tensões…)",
    ),
    (
        "Drop JSON / .cvf report files here, or type their paths.",
        "Solte aqui arquivos de relatório JSON / .cvf, ou digite os caminhos.",
    ),
    ("Compare", "Comparar"),
    ("Export as text", "Exportar como texto"),
    ("No differences.", "Nenhuma diferença."),
    ("Added", "Adicionado"),
    ("Removed", "Removido"),
    ("Modified", "Modificado"),
    ("Saved {}", "Salvo {}"),
    ("The report is still being read.", "O relatório ainda está sendo lido."),
    ("{} changes", "{} alterações"),
];

pub const ES: &[(&str, &str)] = &[
    ("Benchmark history", "Historial de pruebas"),
    ("Compare reports", "Comparar informes"),
    ("Kind", "Tipo"),
    ("All", "Todos"),
    ("Date", "Fecha"),
    ("Label", "Etiqueta"),
    ("Score", "Puntuación"),
    ("Threads", "Hilos"),
    ("Variant", "Variante"),
    ("Processor", "Procesador"),
    ("Delete", "Eliminar"),
    ("Reload", "Recargar"),
    (
        "No saved results yet. Run a benchmark and press \"Save to history\".",
        "Aún no hay resultados guardados. Ejecute una prueba y pulse \"Guardar en el historial\".",
    ),
    ("Select two entries to compare them.", "Seleccione dos entradas para compararlas."),
    ("Difference", "Diferencia"),
    ("Before", "Antes"),
    ("After", "Después"),
    ("Mainboard", "Placa base"),
    ("BIOS", "BIOS"),
    ("Memory", "Memoria"),
    ("Operating system", "Sistema operativo"),
    ("Version", "Versión"),
    ("Scores over time", "Puntuaciones a lo largo del tiempo"),
    ("CPU Single Thread", "CPU Single Thread"),
    ("CPU Multi Thread", "CPU Multi Thread"),
    ("better", "mejor"),
    ("worse", "peor"),
    ("Old report (A)", "Informe antiguo (A)"),
    ("New report (B)", "Informe nuevo (B)"),
    ("Current system", "Sistema actual"),
    ("Compare the current system with a file", "Comparar el sistema actual con un archivo"),
    (
        "Include live values (clocks, temperatures, voltages…)",
        "Incluir valores en vivo (relojes, temperaturas, voltajes…)",
    ),
    (
        "Drop JSON / .cvf report files here, or type their paths.",
        "Suelte aquí archivos de informe JSON / .cvf, o escriba sus rutas.",
    ),
    ("Compare", "Comparar"),
    ("Export as text", "Exportar como texto"),
    ("No differences.", "Sin diferencias."),
    ("Added", "Añadido"),
    ("Removed", "Eliminado"),
    ("Modified", "Modificado"),
    ("Saved {}", "Guardado {}"),
    ("The report is still being read.", "El informe todavía se está leyendo."),
    ("{} changes", "{} cambios"),
];

// ---------------------------------------------------------------- chart

const CHART_HEIGHT: f32 = 180.0;
/// Room for the y labels / x labels.
const CHART_LEFT: f32 = 52.0;
const CHART_BOTTOM: f32 = 18.0;
const CHART_LINE: f32 = 1.8;
const TABLE_HEIGHT: f32 = 240.0;

pub struct Series {
    pub name: String,
    pub color: Color32,
    pub points: Vec<(f64, f64)>,
}

/// A minimal line chart: optional log10 x axis, y from 0, axis labels from
/// the callbacks, hover readout of the nearest point.
pub fn line_chart(
    ui: &mut egui::Ui,
    series: &[Series],
    log_x: bool,
    x_text: &dyn Fn(f64) -> String,
    y_text: &dyn Fn(f64) -> String,
) {
    let tx = |x: f64| if log_x { x.max(1e-9).log10() } else { x };
    let all = || series.iter().flat_map(|s| s.points.iter());
    let (mut x0, mut x1) = all().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(tx(p.0)), b.max(tx(p.0))));
    let y1 = all().fold(0.0_f64, |m, p| m.max(p.1)) * 1.1;
    if x0 >= x1 {
        (x0, x1) = (x0 - 1.0, x0 + 1.0);
    }
    let y1 = if y1 > 0.0 { y1 } else { 1.0 };
    let w = ui.available_width().max(200.0);
    let (resp, painter) = ui.allocate_painter(egui::vec2(w, CHART_HEIGHT), egui::Sense::hover());
    let full = resp.rect;
    let plot =
        egui::Rect::from_min_max(full.min + egui::vec2(CHART_LEFT, 6.0), full.max - egui::vec2(8.0, CHART_BOTTOM));
    let p = theme::palette(ui);
    let (fg, grid) = (p.text_secondary, p.border);
    let font = egui::FontId::proportional(10.0);
    let to_screen = |x: f64, y: f64| {
        Pos2::new(
            plot.left() + ((tx(x) - x0) / (x1 - x0)) as f32 * plot.width(),
            plot.bottom() - (y / y1) as f32 * plot.height(),
        )
    };
    painter.rect_stroke(plot, 0.0, Stroke::new(1.0, grid), egui::StrokeKind::Inside);
    for i in 0..=4 {
        let y = y1 * i as f64 / 4.0;
        let p = to_screen(0.0, y);
        painter.line_segment([Pos2::new(plot.left(), p.y), Pos2::new(plot.right(), p.y)], Stroke::new(1.0, grid));
        painter.text(Pos2::new(plot.left() - 4.0, p.y), egui::Align2::RIGHT_CENTER, y_text(y), font.clone(), fg);
    }
    // X ticks at the data's own x values, thinned to what fits.
    let mut xs: Vec<f64> = all().map(|p| p.0).collect();
    xs.sort_by(f64::total_cmp);
    xs.dedup();
    let step = (xs.len() as f32 / (plot.width() / 48.0)).ceil().max(1.0) as usize;
    for x in xs.iter().step_by(step) {
        let p = to_screen(*x, 0.0);
        painter.line_segment([Pos2::new(p.x, plot.top()), Pos2::new(p.x, plot.bottom())], Stroke::new(1.0, grid));
        painter.text(Pos2::new(p.x, plot.bottom() + 2.0), egui::Align2::CENTER_TOP, x_text(*x), font.clone(), fg);
    }
    let mut nearest: Option<(f32, Pos2, &Series, (f64, f64))> = None;
    let hover = resp.hover_pos();
    for s in series {
        let pts: Vec<Pos2> = s.points.iter().map(|p| to_screen(p.0, p.1)).collect();
        painter.add(egui::Shape::line(pts.clone(), Stroke::new(CHART_LINE, s.color)));
        for (sp, p) in pts.iter().zip(&s.points) {
            painter.circle_filled(*sp, 2.5, s.color);
            if let Some(h) = hover {
                let d = sp.distance(h);
                if nearest.is_none_or(|n| d < n.0) {
                    nearest = Some((d, *sp, s, *p));
                }
            }
        }
    }
    // Legend.
    let mut lx = plot.left() + 6.0;
    for s in series.iter().filter(|s| !s.name.is_empty()) {
        let r = painter.text(Pos2::new(lx + 10.0, plot.top() + 4.0), egui::Align2::LEFT_TOP, &s.name, font.clone(), fg);
        painter.circle_filled(Pos2::new(lx + 4.0, r.center().y), 3.5, s.color);
        lx = r.right() + 12.0;
    }
    if let Some((_, sp, s, v)) = nearest.filter(|n| n.0 < 30.0) {
        painter.circle_stroke(sp, 5.0, Stroke::new(1.5, s.color));
        let text = format!("{}  {}", x_text(v.0), y_text(v.1));
        let anchor = if sp.x > plot.center().x { egui::Align2::RIGHT_BOTTOM } else { egui::Align2::LEFT_BOTTOM };
        painter.text(sp + egui::vec2(0.0, -8.0), anchor, text, egui::FontId::proportional(12.0), p.text);
    }
}

// ---------------------------------------------------------------- history

pub const KINDS: [&str; 3] = ["CPU Single Thread", "CPU Multi Thread", "Memory"];

#[derive(Default)]
pub struct History {
    /// (index in history.json, entry).
    entries: Vec<(usize, Entry)>,
    loaded: bool,
    /// In-memory entries only (screenshots): never written.
    preview: bool,
    kind: Option<String>,
    /// File indices of the (up to two) entries to compare, oldest pick first.
    selected: Vec<usize>,
    /// Label edit buffers, by file index.
    labels: std::collections::HashMap<usize, String>,
}

/// "2026-10-14T10:20:00Z" -> "2026-10-14 10:20".
fn short_date(when: &str) -> String {
    when.get(..16).map_or(when.to_string(), |s| s.replace('T', " "))
}

/// Lower is better for latency.
fn lower_is_better(unit: &str) -> bool {
    unit == "ns"
}

impl History {
    pub fn reload(&mut self) {
        self.entries = history::load().into_iter().enumerate().collect();
        self.loaded = true;
        self.preview = false;
        self.selected.retain(|i| *i < self.entries.len());
        self.labels.clear();
    }

    /// Show these entries without touching history.json (screenshots).
    pub fn preview(&mut self, entries: Vec<Entry>) {
        self.entries = entries.into_iter().enumerate().collect();
        self.loaded = true;
        self.preview = true;
        self.selected.clear();
    }

    /// The Benchmark history page: table, compare two entries, chart.
    pub fn page(&mut self, ui: &mut egui::Ui) {
        if !self.loaded {
            self.reload();
        }
        let mut reload = false;
        widgets::page_header(ui, tr("Benchmark history"), "", |ui| {
            if !self.preview {
                reload = ui.button(format!("{} {}", icon::ARROW_CLOCKWISE, tr("Reload"))).clicked();
            }
            let label = self.kind.as_deref().map_or(tr("All"), tr).to_string();
            egui::ComboBox::from_id_salt("hist-kind").selected_text(label).show_ui(ui, |ui| {
                ui.selectable_value(&mut self.kind, None, tr("All"));
                for k in KINDS {
                    ui.selectable_value(&mut self.kind, Some(k.to_string()), tr(k));
                }
            });
            ui.label(tr("Kind"));
        });
        if reload {
            self.reload();
        }
        let shown: Vec<usize> = (0..self.entries.len())
            .rev() // newest first
            .filter(|&i| self.kind.as_ref().is_none_or(|k| &self.entries[i].1.kind == k))
            .collect();
        if shown.is_empty() {
            widgets::empty_state(
                ui,
                icon::CLOCK_COUNTER_CLOCKWISE,
                tr("Benchmark history"),
                tr("No saved results yet. Run a benchmark and press \"Save to history\"."),
            );
            return;
        }
        widgets::card(ui, icon::TABLE, "", |ui| self.table(ui, &shown));
        ui.add_space(theme::GAP);
        widgets::card(ui, icon::SCALES, tr("Compare"), |ui| self.compare(ui));
        ui.add_space(theme::GAP);
        widgets::card(ui, icon::CHART_LINE, tr("Scores over time"), |ui| self.chart(ui));
    }

    fn table(&mut self, ui: &mut egui::Ui, shown: &[usize]) {
        let mut delete = None;
        let mut relabel = None;
        egui::ScrollArea::both().id_salt("hist-table").max_height(TABLE_HEIGHT).show(ui, |ui| {
            egui::Grid::new("hist-grid").striped(true).spacing([10.0, 4.0]).show(ui, |ui| {
                for h in ["", "Date", "Label", "Kind", "Score", "Threads", "Variant", "Processor", ""] {
                    ui.label(RichText::new(tr(h)).strong());
                }
                ui.end_row();
                for &i in shown {
                    let (idx, e) = &self.entries[i];
                    let mut on = self.selected.contains(idx);
                    if ui.checkbox(&mut on, "").changed() {
                        self.selected.retain(|x| x != idx);
                        if on {
                            self.selected.push(*idx);
                            if self.selected.len() > 2 {
                                self.selected.remove(0);
                            }
                        }
                    }
                    ui.label(short_date(&e.when));
                    let buf = self.labels.entry(*idx).or_insert_with(|| e.label.clone());
                    let r = ui.add_sized([120.0, 22.0], egui::TextEdit::singleline(buf));
                    if r.lost_focus() && *buf != e.label {
                        relabel = Some((i, buf.clone()));
                    }
                    let cell = |ui: &mut egui::Ui, t: String| ui.add(egui::Label::new(t).extend());
                    cell(ui, tr(&e.kind).into());
                    ui.add(egui::Label::new(RichText::new(format!("{:.1} {}", e.score, e.unit)).monospace()).extend());
                    cell(ui, e.threads.to_string());
                    cell(ui, tr(&e.variant).into());
                    cell(ui, e.cpu.clone());
                    if ui.small_button(icon::TRASH).on_hover_text(tr("Delete")).clicked() {
                        delete = Some(i);
                    }
                    ui.end_row();
                }
            });
        });
        if let Some((i, label)) = relabel {
            if self.preview {
                self.entries[i].1.label = label;
            } else {
                // ponytail: history.rs has no update(); remove + add moves the
                // entry to the end of the file (the table sorts by position, so
                // it shows as newest). Add history::set_label if order matters.
                let mut e = self.entries[i].1.clone();
                e.label = label;
                history::remove(self.entries[i].0);
                history::add(e);
                self.reload();
            }
        }
        if let Some(i) = delete {
            if self.preview {
                self.entries.remove(i);
            } else {
                history::remove(self.entries[i].0);
                self.selected.clear();
                self.reload();
            }
        }
    }

    fn compare(&self, ui: &mut egui::Ui) {
        let pick = |idx: usize| self.entries.iter().find(|(i, _)| *i == idx).map(|(_, e)| e);
        let (Some(a), Some(b)) =
            (self.selected.first().and_then(|&i| pick(i)), self.selected.get(1).and_then(|&i| pick(i)))
        else {
            ui.label(RichText::new(tr("Select two entries to compare them.")).weak());
            return;
        };
        let p = theme::palette(ui);
        // Older entry = "before".
        let (a, b) = if a.when <= b.when { (a, b) } else { (b, a) };
        egui::Grid::new("hist-cmp").striped(true).spacing([12.0, 3.0]).show(ui, |ui| {
            ui.label("");
            ui.label(RichText::new(tr("Before")).strong());
            ui.label(RichText::new(tr("After")).strong());
            ui.label(RichText::new(tr("Difference")).strong());
            ui.end_row();
            ui.label(tr("Score"));
            super::value(ui, &format!("{:.1} {}", a.score, a.unit));
            super::value(ui, &format!("{:.1} {}", b.score, b.unit));
            if a.kind == b.kind && a.unit == b.unit && a.score != 0.0 {
                let pct = (b.score - a.score) / a.score * 100.0;
                let better = if lower_is_better(&a.unit) { pct < 0.0 } else { pct > 0.0 };
                let color = if pct.abs() < 0.05 {
                    p.text_muted
                } else if better {
                    p.success
                } else {
                    p.danger
                };
                let word = if pct.abs() < 0.05 { "" } else { tr(if better { "better" } else { "worse" }) };
                ui.label(RichText::new(format!("{pct:+.1}% {word}")).color(color).strong());
            } else {
                ui.label("-");
            }
            ui.end_row();
            let rows: [(&str, &str, &str); 9] = [
                ("Label", &a.label, &b.label),
                ("Kind", tr(&a.kind), tr(&b.kind)),
                ("Variant", &a.variant, &b.variant),
                ("Processor", &a.cpu, &b.cpu),
                ("Mainboard", &a.board, &b.board),
                ("BIOS", &a.bios, &b.bios),
                ("Memory", &a.memory, &b.memory),
                ("Operating system", &a.os, &b.os),
                ("Version", &a.app_version, &b.app_version),
            ];
            for (k, x, y) in rows {
                ui.label(tr(k));
                ui.label(x);
                let c = if x == y { p.text } else { p.warning };
                ui.label(RichText::new(y).color(c));
                ui.label(if x == y { "" } else { "≠" });
                ui.end_row();
            }
            ui.label(tr("Threads"));
            ui.label(a.threads.to_string());
            ui.label(b.threads.to_string());
            ui.end_row();
        });
    }

    fn chart(&self, ui: &mut egui::Ui) {
        let p = theme::palette(ui);
        for (n, kind) in KINDS.iter().enumerate() {
            if self.kind.as_ref().is_some_and(|k| k != kind) {
                continue;
            }
            // x = position in the file, which is chronological.
            let pts: Vec<(f64, f64)> =
                self.entries.iter().filter(|(_, e)| e.kind == *kind).map(|(i, e)| (*i as f64 + 1.0, e.score)).collect();
            if pts.is_empty() {
                continue;
            }
            let unit = self.entries.iter().find(|(_, e)| e.kind == *kind).map_or("", |(_, e)| e.unit.as_str());
            let names: Vec<String> = self.entries.iter().map(|(_, e)| short_date(&e.when)).collect();
            let name = format!("{} ({unit})", tr(kind));
            let s = [Series { name, color: p.series[n % p.series.len()], points: pts }];
            line_chart(
                ui,
                &s,
                false,
                &|x| names.get(x as usize - 1).map_or(String::new(), |d| d.get(5..10).unwrap_or(d).to_string()),
                &|y| format!("{y:.0}"),
            );
        }
    }
}

// ---------------------------------------------------------------- compare

#[derive(Default)]
pub struct Compare {
    pub a: String,
    pub b: String,
    /// A = the current system's report.
    pub current: bool,
    pub volatile: bool,
    result: Option<Result<Vec<Change>, String>>,
    status: String,
}

/// The diff as plain text (English, like reports).
pub fn diff_text(changes: &[Change]) -> String {
    let mut out = String::new();
    let mut last = "";
    for c in changes {
        if c.section != last {
            out.push_str(&format!("\n[{}]\n", c.section));
            last = &c.section;
        }
        let at = if c.path.is_empty() { c.key.clone() } else { format!("{} › {}", c.path, c.key) };
        let (b, a) = (c.before.as_deref().unwrap_or(""), c.after.as_deref().unwrap_or(""));
        out.push_str(&match c.kind {
            ChangeKind::Added => format!("+ {at}: {a}\n"),
            ChangeKind::Removed => format!("- {at}: {b}\n"),
            ChangeKind::Modified => format!("~ {at}: {b} -> {a}\n"),
        });
    }
    out.trim_start().to_string()
}

impl Compare {
    /// The Compare reports page. `current` = the live report, once read.
    pub fn page(&mut self, ui: &mut egui::Ui, current: Option<&[Section]>) {
        // Files dropped anywhere on the window while this page is shown.
        let dropped: Vec<String> =
            ui.ctx().input(|i| i.raw.dropped_files.iter().map(|f| f.path().display().to_string()).collect());
        for p in dropped {
            if !self.current && self.a.is_empty() {
                self.a = p;
            } else {
                self.b = p;
            }
        }
        let p = theme::palette(ui);
        widgets::page_header(
            ui,
            tr("Compare reports"),
            tr("Drop JSON / .cvf report files here, or type their paths."),
            |_| {},
        );
        widgets::card(ui, icon::FILES, "", |ui| {
            ui.checkbox(&mut self.current, tr("Compare the current system with a file"));
            egui::Grid::new("cmp-paths").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                ui.label(tr("Old report (A)"));
                if self.current {
                    ui.label(RichText::new(tr("Current system")).italics());
                } else {
                    ui.add(egui::TextEdit::singleline(&mut self.a).desired_width(f32::INFINITY));
                }
                ui.end_row();
                ui.label(tr("New report (B)"));
                ui.add(egui::TextEdit::singleline(&mut self.b).desired_width(f32::INFINITY));
                ui.end_row();
            });
            ui.checkbox(&mut self.volatile, tr("Include live values (clocks, temperatures, voltages…)"));
            ui.horizontal(|ui| {
                if widgets::primary_button(ui, &format!("{} {}", icon::GIT_DIFF, tr("Compare"))).clicked() {
                    self.run(current);
                }
                let have = matches!(&self.result, Some(Ok(c)) if !c.is_empty());
                let export = egui::Button::new(format!("{} {}", icon::EXPORT, tr("Export as text")));
                if ui.add_enabled(have, export).clicked()
                    && let Some(Ok(c)) = &self.result
                {
                    let name = format!("{}-diff.txt", crate::NAME);
                    self.status = match std::fs::write(&name, diff_text(c)) {
                        Ok(()) => trf(
                            "Saved {}",
                            &[&std::fs::canonicalize(&name).map_or(name.clone(), |p| p.display().to_string())],
                        ),
                        Err(e) => format!("{name}: {e}"),
                    };
                }
                ui.label(RichText::new(&self.status).small());
            });
        });
        ui.add_space(theme::GAP);
        match &self.result {
            None => {}
            Some(Err(e)) => {
                widgets::pill(ui, e, widgets::Tone::Danger);
            }
            Some(Ok(c)) if c.is_empty() => {
                widgets::pill(ui, tr("No differences."), widgets::Tone::Success);
            }
            Some(Ok(c)) => widgets::card(ui, icon::GIT_DIFF, &trf("{} changes", &[&c.len()]), |ui| changes(ui, p, c)),
        }
    }

    pub fn run(&mut self, current: Option<&[Section]>) {
        let load = |p: &str| {
            crate::diff::load(std::path::Path::new(p.trim().trim_matches('"'))).map_err(|e| format!("{p}: {e}"))
        };
        self.result = Some((|| {
            let a = if self.current {
                current.ok_or_else(|| tr("The report is still being read.").to_string())?.to_vec()
            } else {
                load(&self.a)?
            };
            let b = load(&self.b)?;
            Ok(crate::diff::diff(&a, &b, self.volatile))
        })());
        self.status.clear();
    }
}

fn kind_style(p: &Palette, k: &ChangeKind) -> (&'static str, &'static str, Color32) {
    match k {
        ChangeKind::Added => ("+", "Added", p.success),
        ChangeKind::Removed => ("−", "Removed", p.danger),
        ChangeKind::Modified => ("~", "Modified", p.warning),
    }
}

fn changes(ui: &mut egui::Ui, p: &Palette, c: &[Change]) {
    ui.horizontal(|ui| {
        for k in [ChangeKind::Added, ChangeKind::Removed, ChangeKind::Modified] {
            let (_, name, color) = kind_style(p, &k);
            let n = c.iter().filter(|x| x.kind == k).count();
            ui.label(RichText::new(format!("{} {n}", tr(name))).color(color));
        }
    });
    // Sections in order of first appearance.
    let mut sections: Vec<(&str, Vec<&Change>)> = Vec::new();
    for x in c {
        match sections.iter_mut().find(|s| s.0 == x.section) {
            Some(s) => s.1.push(x),
            None => sections.push((&x.section, vec![x])),
        }
    }
    for (title, items) in sections {
        egui::CollapsingHeader::new(format!("{title} ({})", items.len())).default_open(true).show(ui, |ui| {
            for x in items {
                let (sign, _, color) = kind_style(p, &x.kind);
                let at = if x.path.is_empty() { x.key.clone() } else { format!("{} › {}", x.path, x.key) };
                let (b, a) = (x.before.as_deref().unwrap_or(""), x.after.as_deref().unwrap_or(""));
                let text = match x.kind {
                    ChangeKind::Added => format!("{sign} {at}: {a}"),
                    ChangeKind::Removed => format!("{sign} {at}: {b}"),
                    ChangeKind::Modified => format!("{sign} {at}: {b} → {a}"),
                };
                ui.add(egui::Label::new(RichText::new(text).monospace().color(color)).wrap());
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_as_text() {
        let ch = |kind, s: &str, b: Option<&str>, a: Option<&str>| Change {
            kind,
            section: s.into(),
            path: "DIMM # 1".into(),
            key: "Size".into(),
            before: b.map(Into::into),
            after: a.map(Into::into),
        };
        let t = diff_text(&[
            ch(ChangeKind::Modified, "Memory SPD", Some("8"), Some("16")),
            ch(ChangeKind::Added, "Memory SPD", None, Some("x")),
            ch(ChangeKind::Removed, "DMI", Some("y"), None),
        ]);
        assert_eq!(
            t,
            "[Memory SPD]\n~ DIMM # 1 › Size: 8 -> 16\n+ DIMM # 1 › Size: x\n\n[DMI]\n- DIMM # 1 › Size: y\n"
        );
        assert_eq!(short_date("2026-10-14T10:20:00Z"), "2026-10-14 10:20");
    }
}
