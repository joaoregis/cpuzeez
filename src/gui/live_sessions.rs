//! ZeeZ Live › Sessions (S2b-10, docs/v1/zeez-live.md §6): the capture
//! library, one session's report, and the comparison of several. Reading
//! and writing sessions happens on threads (a capture can be millions of
//! frames).

use super::i18n::{tr, trf};
use super::table::{self, Cell, Column, Table};
use super::theme::{self, icon};
use super::widgets::{self, Tone};
use cpuzeez_core::frames::{self, Frame};
use cpuzeez_core::session::{self, HwLog, Mode, Session};
use eframe::egui::{self, Color32, RichText, Sense, Ui, pos2, vec2};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

type Slot<T> = Arc<Mutex<Option<T>>>;

fn take<T>(s: &Slot<T>) -> Option<T> {
    s.lock().unwrap_or_else(|e| e.into_inner()).take()
}

struct Loaded {
    dir: PathBuf,
    session: Session,
    frames: Vec<Frame>,
    hw: HwLog,
    /// What limited the frame rate, worked out when the session is opened.
    analysis: cpuzeez_core::bottleneck::Analysis,
}

#[derive(Default)]
pub struct Sessions {
    list: Vec<(PathBuf, Session)>,
    listing: Option<Slot<Vec<(PathBuf, Session)>>>,
    listed: bool,
    table: table::State,
    open: Option<Arc<Loaded>>,
    loading: Option<(PathBuf, Slot<Loaded>)>,
    /// Sessions picked for the comparison (their summaries are in `list`).
    pub import_path: String,
    message: Option<(String, Tone)>,
    io: Option<Slot<Result<String, String>>>,
    confirm_delete: bool,
    /// A PNG of the report the app should save (it owns the screenshot).
    pub png_request: Option<PathBuf>,
}

/// A session's row key in the table.
fn key(p: &Path) -> u64 {
    let mut x = 0xcbf29ce484222325u64;
    for b in p.to_string_lossy().bytes() {
        x = (x ^ u64::from(b)).wrapping_mul(0x100000001b3);
    }
    x
}

fn read_session(dir: &Path) -> Option<Session> {
    serde_json::from_str(&std::fs::read_to_string(dir.join("session.json")).ok()?).ok()
}

fn list_sessions() -> Vec<(PathBuf, Session)> {
    let Some(root) = super::live::sessions_dir() else { return Vec::new() };
    let mut v: Vec<(PathBuf, Session)> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| read_session(&e.path()).map(|s| (e.path(), s)))
        .collect();
    v.sort_by(|a, b| b.1.started.cmp(&a.1.started));
    v
}

fn load(dir: &Path) -> Option<Loaded> {
    let session = read_session(dir)?;
    let frames = std::fs::read_to_string(dir.join("frames.csv"))
        .ok()
        .and_then(|t| frames::parse_presentmon(&t).ok())
        .unwrap_or_default();
    let hw = std::fs::read_to_string(dir.join("hw.csv")).map(|t| session::parse_hw_csv(&t)).unwrap_or_default();
    let windows =
        cpuzeez_core::bottleneck::windows_from_session(&frames, &hw, session.system.refresh_hz, &session.throttling);
    let analysis = cpuzeez_core::bottleneck::analyze(&windows, &Default::default());
    Some(Loaded { dir: dir.to_path_buf(), session, frames, hw, analysis })
}

/// Import a PresentMon / CapFrameX CSV or a MangoHud log as a session.
fn import(path: &Path, root: Option<PathBuf>) -> Result<String, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let stem = path.file_stem().map_or("imported".into(), |s| s.to_string_lossy().into_owned());
    let (frames, source) = match frames::parse_presentmon(&text) {
        Ok(f) => (f, "CSV import (PresentMon columns)"),
        Err(_) => (frames::parse_mangohud(&text, &stem)?, "CSV import (MangoHud)"),
    };
    if frames.is_empty() {
        return Err(tr("The file has no frames.").into());
    }
    let game = frames.iter().map(|f| f.app.as_str()).find(|a| !a.is_empty()).unwrap_or(&stem).to_string();
    let root = root.ok_or_else(|| tr("No data folder to save the session in.").to_string())?;
    let iso = cpuzeez_core::metrics::iso8601(std::time::SystemTime::now());
    let dir = root.join(session::folder_name(&format!("{}_{}{}", &iso[..10], &iso[11..13], &iso[14..16]), &game));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let t0 = frames.first().map_or(0.0, |f| f.t_s);
    let frames: Vec<Frame> = frames.into_iter().map(|f| Frame { t_s: f.t_s - t0, ..f }).collect();
    let s = Session {
        version: session::VERSION,
        game,
        started: iso,
        duration_s: frames.iter().map(|f| f.ms).sum::<f64>() / 1000.0,
        mode: Mode::Imported,
        source: source.into(),
        summary: frames::summarize(&frames, Default::default()),
        note: path.display().to_string(),
        ..Default::default()
    };
    super::live::write_session(&dir, &frames, &s);
    Ok(trf("Imported {} frames.", &[&frames.len()]))
}

fn spawn<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Slot<T> {
    let slot: Slot<T> = Arc::new(Mutex::new(None));
    let s = slot.clone();
    std::thread::spawn(move || *s.lock().unwrap_or_else(|e| e.into_inner()) = Some(f()));
    slot
}

/// A small x/y chart (percentile curve, overlaid curves).
fn xy_chart(
    ui: &mut Ui,
    lines: &[(&str, Color32, &[(f64, f64)])],
    h: f32,
    x_fmt: &dyn Fn(f64) -> String,
    y_unit: &str,
) {
    let p = theme::palette(ui);
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, ui.visuals().extreme_bg_color);
    let pts = || lines.iter().flat_map(|l| l.2.iter());
    let (x0, x1) = pts().fold((f64::MAX, f64::MIN), |a, q| (a.0.min(q.0), a.1.max(q.0)));
    let y1 = pts().fold(0.0f64, |a, q| a.max(q.1)) * 1.05;
    if x1 <= x0 || y1 <= 0.0 {
        return;
    }
    let area = egui::Rect::from_min_max(
        pos2(rect.left() + 46.0, rect.top() + 4.0),
        pos2(rect.right() - 4.0, rect.bottom() - 14.0),
    );
    let to = |x: f64, y: f64| {
        pos2(
            area.left() + ((x - x0) / (x1 - x0)) as f32 * area.width(),
            area.bottom() - (y / y1) as f32 * area.height(),
        )
    };
    let font = egui::FontId::proportional(10.0);
    for k in 0..=4 {
        let y = y1 * k as f64 / 4.0;
        let py = to(x0, y).y;
        painter.hline(area.x_range(), py, egui::Stroke::new(0.5, p.border));
        painter.text(
            pos2(rect.left() + 2.0, py),
            egui::Align2::LEFT_CENTER,
            format!("{y:.0} {y_unit}"),
            font.clone(),
            p.text_muted,
        );
    }
    painter.text(
        pos2(area.left(), rect.bottom() - 2.0),
        egui::Align2::LEFT_BOTTOM,
        x_fmt(x0),
        font.clone(),
        p.text_muted,
    );
    painter.text(pos2(area.right(), rect.bottom() - 2.0), egui::Align2::RIGHT_BOTTOM, x_fmt(x1), font, p.text_muted);
    for (_, color, l) in lines {
        let step = (l.len() / 2000).max(1);
        let v: Vec<egui::Pos2> = l.iter().step_by(step).map(|q| to(q.0, q.1)).collect();
        painter.add(egui::Shape::line(v, egui::Stroke::new(1.4, *color)));
    }
    if let Some(h) = resp.hover_pos().filter(|h| area.contains(*h)) {
        let x = x0 + ((h.x - area.left()) / area.width()) as f64 * (x1 - x0);
        let text: Vec<String> = lines
            .iter()
            .filter_map(|(n, _, l)| {
                l.iter()
                    .min_by(|a, b| (a.0 - x).abs().total_cmp(&(b.0 - x).abs()))
                    .map(|q| format!("{n}: {:.2} {y_unit}", q.1))
            })
            .collect();
        resp.on_hover_text(format!("{}\n{}", x_fmt(x), text.join("\n")));
    }
}

fn bars(ui: &mut Ui, values: &[(String, f64)], color: Color32, h: f32) {
    let p = theme::palette(ui);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, ui.visuals().extreme_bg_color);
    let max = values.iter().map(|v| v.1).fold(0.0, f64::max);
    if values.is_empty() || max <= 0.0 {
        return;
    }
    let bw = (rect.width() - 8.0) / values.len() as f32;
    let font = egui::FontId::proportional(9.0);
    for (i, (label, v)) in values.iter().enumerate() {
        let bh = (*v / max) as f32 * (rect.height() - 26.0);
        let x = rect.left() + 4.0 + i as f32 * bw;
        painter.rect_filled(
            egui::Rect::from_min_size(pos2(x + 1.0, rect.bottom() - 14.0 - bh), vec2((bw - 2.0).max(1.0), bh)),
            2.0,
            color,
        );
        if values.len() <= 40 || i % (values.len() / 20).max(1) == 0 {
            painter.text(
                pos2(x + bw / 2.0, rect.bottom() - 2.0),
                egui::Align2::CENTER_BOTTOM,
                label,
                font.clone(),
                p.text_muted,
            );
        }
    }
}

const COLORS: [Color32; 6] = [
    Color32::from_rgb(139, 92, 246),
    Color32::from_rgb(34, 197, 94),
    Color32::from_rgb(245, 158, 11),
    Color32::from_rgb(59, 130, 246),
    Color32::from_rgb(236, 72, 153),
    Color32::from_rgb(20, 184, 166),
];

impl Sessions {
    /// Show `dir`'s report and select it (after a capture).
    pub fn focus(&mut self, dir: PathBuf) {
        self.table.selected = [key(&dir)].into();
        self.open_dir(dir);
    }

    /// Load this session's report.
    pub fn open_dir(&mut self, dir: PathBuf) {
        self.listed = false;
        let d = dir.clone();
        self.loading = Some((
            dir,
            spawn(move || {
                load(&d).unwrap_or_else(|| Loaded {
                    dir: d.clone(),
                    session: Default::default(),
                    frames: Vec::new(),
                    hw: Default::default(),
                    analysis: Default::default(),
                })
            }),
        ));
    }

    fn poll(&mut self, ui: &Ui) {
        if !self.listed && self.listing.is_none() {
            self.listing = Some(spawn(list_sessions));
        }
        if let Some(l) = self.listing.as_ref().and_then(take) {
            self.list = l;
            self.listing = None;
            self.listed = true;
        }
        if let Some((_, slot)) = &self.loading
            && let Some(l) = take(slot)
        {
            self.open = Some(Arc::new(l));
            self.loading = None;
        }
        if let Some(r) = self.io.as_ref().and_then(take) {
            self.io = None;
            self.listed = false;
            self.message = Some(match r {
                Ok(m) => (m, Tone::Success),
                Err(e) => (e, Tone::Danger),
            });
        }
        if self.listing.is_some() || self.loading.is_some() || self.io.is_some() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));
        }
    }

    pub fn show(&mut self, ui: &mut Ui) {
        self.poll(ui);
        let p = theme::palette(ui);
        ui.horizontal_wrapped(|ui| {
            if ui.button(format!("{} {}", icon::ARROW_CLOCKWISE, tr("Refresh"))).clicked() {
                self.listed = false;
            }
            ui.separator();
            ui.add(
                egui::TextEdit::singleline(&mut self.import_path)
                    .desired_width(260.0)
                    .hint_text(tr("Path of a PresentMon, CapFrameX or MangoHud CSV")),
            );
            if ui
                .add_enabled(
                    !self.import_path.trim().is_empty() && self.io.is_none(),
                    egui::Button::new(format!("{} {}", icon::FILE_ARROW_UP, tr("Import"))),
                )
                .clicked()
            {
                let path = PathBuf::from(self.import_path.trim().trim_matches('"'));
                self.io = Some(spawn(move || import(&path, super::live::sessions_dir())));
            }
            if let Some(root) = super::live::sessions_dir()
                && ui.button(format!("{} {}", icon::FOLDER_OPEN, tr("Open folder"))).clicked()
            {
                let _ = std::fs::create_dir_all(&root);
                let _ = cpuzeez_platform::proc_ops::reveal(&root.display().to_string());
            }
        });
        // Dropping a CSV on the window imports it.
        let dropped: Vec<PathBuf> = ui.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
        if let Some(path) = dropped.into_iter().find(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("csv"))) {
            self.io = Some(spawn(move || import(&path, super::live::sessions_dir())));
        }
        if let Some((m, t)) = &self.message {
            widgets::pill(ui, m, *t);
        }
        if self.list.is_empty() {
            widgets::empty_state(
                ui,
                icon::FILM_STRIP,
                if self.listed { tr("No sessions yet") } else { tr("Reading sessions…") },
                tr(
                    "Start a capture on the Live tab (or with its hotkey), or import a CSV. Sessions are saved on this computer only.",
                ),
            );
            return;
        }
        ui.add_space(4.0);
        let list = self.list.clone();
        let h = (list.len() as f32 * 30.0 + 70.0).min(260.0);
        ui.allocate_ui_with_layout(vec2(ui.available_width(), h), egui::Layout::top_down(egui::Align::Min), |ui| {
            Table::new("live_sessions", |r: &(PathBuf, Session)| key(&r.0))
                .col(Column::new("date", tr("Date"), 140.0, |r: &(PathBuf, Session)| {
                    Cell::Text(r.1.started.replace('T', " ").chars().take(16).collect())
                }))
                .col(Column::new("game", tr("Game"), 200.0, |r: &(PathBuf, Session)| Cell::Text(r.1.game.clone())))
                .col(
                    Column::new("dur", tr("Duration"), 80.0, |r: &(PathBuf, Session)| {
                        Cell::num(r.1.duration_s, format!("{:.0} s", r.1.duration_s))
                    })
                    .right(),
                )
                .col(
                    Column::new("avg", tr("Average FPS"), 100.0, |r: &(PathBuf, Session)| {
                        Cell::num(r.1.summary.avg_fps, format!("{:.1}", r.1.summary.avg_fps))
                    })
                    .right(),
                )
                .col(
                    Column::new("low1", tr("1% low"), 80.0, |r: &(PathBuf, Session)| {
                        Cell::num(r.1.summary.low1_fps, format!("{:.1}", r.1.summary.low1_fps))
                    })
                    .right(),
                )
                .col(
                    Column::new("low01", tr("0.1% low"), 80.0, |r: &(PathBuf, Session)| {
                        Cell::num(r.1.summary.low01_fps, format!("{:.1}", r.1.summary.low01_fps))
                    })
                    .right(),
                )
                .col(Column::new("mode", tr("Mode"), 90.0, |r: &(PathBuf, Session)| {
                    Cell::Text(
                        tr(match r.1.mode {
                            Mode::Manual => "Manual",
                            Mode::Timed => "Timed",
                            Mode::Automatic => "Automatic",
                            Mode::Imported => "Imported",
                        })
                        .into(),
                    )
                }))
                .show(ui, &list, &mut self.table, |_, _| {});
        });
        let picked: Vec<&(PathBuf, Session)> =
            list.iter().filter(|r| self.table.selected.contains(&key(&r.0))).collect();
        ui.label(
            RichText::new(tr("Click a session for its report; Ctrl+click several to compare them."))
                .small()
                .color(p.text_muted),
        );
        ui.add_space(6.0);
        match picked.len() {
            0 => {}
            1 => {
                let dir = picked[0].0.clone();
                if self.open.as_ref().is_none_or(|o| o.dir != dir) && self.loading.as_ref().is_none_or(|l| l.0 != dir) {
                    self.open_dir(dir);
                    self.listed = true;
                }
                match self.open.clone() {
                    Some(o) if o.dir == picked[0].0 => self.report(ui, &o),
                    _ => {
                        ui.spinner();
                    }
                }
            }
            _ => compare(ui, &picked),
        }
    }

    fn report(&mut self, ui: &mut Ui, o: &Loaded) {
        let p = theme::palette(ui);
        let s = &o.session;
        let sm = &s.summary;
        widgets::card(ui, icon::CHART_LINE, &s.game, |ui| {
            ui.label(
                RichText::new(format!(
                    "{} · {:.0} s · {}",
                    s.started.replace('T', " ").chars().take(19).collect::<String>(),
                    s.duration_s,
                    s.source
                ))
                .small()
                .color(p.text_secondary),
            );
            ui.horizontal_wrapped(|ui| {
                for (label, v) in [("Average FPS", sm.avg_fps), ("1% low", sm.low1_fps), ("0.1% low", sm.low01_fps)] {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(format!("{v:.1}")).size(24.0).strong());
                        ui.label(RichText::new(tr(label)).small().color(p.text_secondary));
                    });
                    ui.add_space(16.0);
                }
                ui.vertical(|ui| {
                    ui.label(RichText::new(sm.stutters.to_string()).size(24.0).strong());
                    ui.label(RichText::new(tr("Stutters")).small().color(p.text_secondary));
                });
            });
            ui.label(RichText::new(tr(sm.convention.label())).small().color(p.text_muted));
            let kvs = vec![
                (tr("Frames"), sm.frames.to_string()),
                (tr("Min / max FPS"), format!("{:.1} / {:.1}", sm.min_fps, sm.max_fps)),
                (tr("Median frame time"), format!("{:.2} ms", sm.median_ms)),
                (tr("Generated frames (FG)"), sm.generated.to_string()),
                (tr("Dropped frames"), sm.dropped.to_string()),
                ("CPU", s.system.cpu.clone()),
                ("GPU", format!("{} {}", s.system.gpu, s.system.gpu_driver)),
            ];
            widgets::kv_list(ui, &kvs);
            ui.horizontal(|ui| {
                if ui.button(format!("{} {}", icon::FILE_HTML, tr("Export HTML"))).clicked() {
                    let (dir, html) = (o.dir.clone(), session::html_report(s, &o.frames, &o.hw));
                    self.io = Some(spawn(move || {
                        let path = dir.join("report.html");
                        std::fs::write(&path, html).map_err(|e| e.to_string())?;
                        let _ = cpuzeez_platform::proc_ops::reveal(&path.display().to_string());
                        Ok(trf("Saved {}", &[&path.display()]))
                    }));
                }
                if ui.button(format!("{} {}", icon::IMAGE, tr("Export PNG"))).clicked() {
                    self.png_request = Some(o.dir.join("report.png"));
                    self.message = Some((trf("Saved {}", &[&o.dir.join("report.png").display()]), Tone::Success));
                }
                if self.confirm_delete {
                    ui.label(RichText::new(tr("Delete this session's files?")).color(p.danger));
                    if ui.button(tr("Delete")).clicked() {
                        let dir = o.dir.clone();
                        self.io = Some(spawn(move || {
                            std::fs::remove_dir_all(&dir)
                                .map(|_| tr("Session deleted.").to_string())
                                .map_err(|e| e.to_string())
                        }));
                        self.confirm_delete = false;
                        self.open = None;
                        self.table.selected.clear();
                    }
                    if ui.button(tr("Cancel")).clicked() {
                        self.confirm_delete = false;
                    }
                } else if ui.button(format!("{} {}", icon::TRASH, tr("Delete"))).clicked() {
                    self.confirm_delete = true;
                }
            });
        });
        let ft: Vec<(f64, f64)> = o.frames.iter().map(|f| (f.t_s, f.ms)).collect();
        widgets::card(ui, icon::PULSE, tr("Frame time"), |ui| {
            xy_chart(ui, &[(tr("Frame time"), p.accent, &ft)], 160.0, &|x| format!("{x:.0} s"), "ms");
            for b in &s.throttling {
                widgets::pill(
                    ui,
                    &format!("{} {:.0}–{:.0} s · {}", icon::THERMOMETER_HOT, b.from_s, b.to_s, b.label),
                    Tone::Warning,
                );
            }
            for m in &s.markers {
                ui.label(RichText::new(format!("{} {:.1} s · {}", icon::FLAG, m.t_s, m.label)).small());
            }
            for (title, ids) in [
                (tr("Load"), ["cpu/load/total", "gpu0/load/core"]),
                (tr("Temperature"), ["cpu/temp/package", "gpu0/temp/core"]),
            ] {
                let (a, b) = (o.hw.series(ids[0]), o.hw.series(ids[1]));
                if !a.is_empty() || !b.is_empty() {
                    ui.label(RichText::new(title).small().color(p.text_secondary));
                    xy_chart(
                        ui,
                        &[("CPU", COLORS[3], &a), ("GPU", COLORS[1], &b)],
                        110.0,
                        &|x| format!("{x:.0} s"),
                        "",
                    );
                }
            }
        });
        if o.analysis.seconds > 0 {
            widgets::card(ui, icon::GAUGE, tr("What limited the frame rate"), |ui| {
                super::bottleneck_ui::draw_analysis(ui, &o.analysis);
            });
        }
        let ms: Vec<f64> = o.frames.iter().map(|f| f.ms).collect();
        widgets::card(ui, icon::CHART_BAR, tr("Distribution"), |ui| {
            ui.label(RichText::new(tr("Frame time at each percentile")).small().color(p.text_secondary));
            xy_chart(
                ui,
                &[(tr("Frame time"), COLORS[2], &frames::percentile_curve(&ms))],
                130.0,
                &|x| format!("{x:.0}%"),
                "ms",
            );
            ui.label(RichText::new(tr("Share of frames per FPS range")).small().color(p.text_secondary));
            let h: Vec<(String, f64)> =
                frames::histogram(&ms, 10.0).into_iter().map(|(b, v)| (format!("{b:.0}"), v * 100.0)).collect();
            bars(ui, &h, p.accent, 120.0);
        });
    }
}

/// Several sessions side by side (zeez-live §6): bars, curves, validity.
fn compare(ui: &mut Ui, picked: &[&(PathBuf, Session)]) {
    let p = theme::palette(ui);
    widgets::card(ui, icon::SCALES, &trf("Comparing {} sessions", &[&picked.len()]), |ui| {
        let first = &picked[0].1;
        for (_, s) in &picked[1..] {
            for why in session::comparability(first, s) {
                widgets::pill(ui, &format!("{} {}: {}", icon::WARNING, tr(why), s.game), Tone::Warning);
            }
        }
        let avg: Vec<f64> = picked.iter().map(|r| r.1.summary.avg_fps).collect();
        if session::high_variance(&avg) {
            let (m, l1, l01) = session::median_of(&picked.iter().map(|r| r.1.summary.clone()).collect::<Vec<_>>());
            widgets::pill(
                ui,
                &trf(
                    "These runs vary more than 5 %: run it 3x and use the median ({} / {} / {} FPS).",
                    &[&format!("{m:.1}"), &format!("{l1:.1}"), &format!("{l01:.1}")],
                ),
                Tone::Warning,
            );
        }
        egui::Grid::new("live-compare").striped(true).num_columns(5).spacing([16.0, 4.0]).show(ui, |ui| {
            for h in [tr("Session"), tr("Average FPS"), tr("1% low"), tr("0.1% low"), tr("Stutters")] {
                ui.label(RichText::new(h).small().color(p.text_secondary));
            }
            ui.end_row();
            let max = picked.iter().map(|r| r.1.summary.avg_fps).fold(1.0, f64::max);
            for (i, (_, s)) in picked.iter().enumerate() {
                ui.label(
                    RichText::new(format!(
                        "{} {} · {}",
                        icon::SQUARE,
                        s.game,
                        s.started.replace('T', " ").chars().take(16).collect::<String>()
                    ))
                    .color(COLORS[i % COLORS.len()]),
                );
                for v in [s.summary.avg_fps, s.summary.low1_fps, s.summary.low01_fps] {
                    ui.add(egui::ProgressBar::new((v / max) as f32).text(format!("{v:.1}")).desired_width(110.0));
                }
                ui.label(s.summary.stutters.to_string());
                ui.end_row();
            }
        });
        ui.label(RichText::new(tr(first.convention.label())).small().color(p.text_muted));
    });
}

pub const PT_BR: &[(&str, &str)] = &[
    ("The file has no frames.", "O arquivo não tem quadros."),
    ("Imported {} frames.", "{} quadros importados."),
    ("Path of a PresentMon, CapFrameX or MangoHud CSV", "Caminho de um CSV do PresentMon, CapFrameX ou MangoHud"),
    ("Import", "Importar"),
    ("Open folder", "Abrir pasta"),
    ("Reading sessions…", "Lendo sessões…"),
    (
        "Start a capture on the Live tab (or with its hotkey), or import a CSV. Sessions are saved on this computer only.",
        "Inicie uma captura na aba Ao vivo (ou pelo atalho), ou importe um CSV. As sessões ficam só neste computador.",
    ),
    ("Date", "Data"),
    ("Game", "Jogo"),
    ("Duration", "Duração"),
    ("Average FPS", "FPS médio"),
    ("Mode", "Modo"),
    ("Manual", "Manual"),
    ("Timed", "Cronometrada"),
    ("Automatic", "Automática"),
    ("Imported", "Importada"),
    (
        "Click a session for its report; Ctrl+click several to compare them.",
        "Clique numa sessão para ver o relatório; Ctrl+clique em várias para compará-las.",
    ),
    ("Stutters", "Engasgos"),
    ("1% low = 99th-percentile frame time", "1% mínimo = percentil 99 do tempo de quadro"),
    ("1% low = average of the slowest 1% of frames", "1% mínimo = média do 1% de quadros mais lentos"),
    ("Min / max FPS", "FPS mín. / máx."),
    ("Median frame time", "Tempo de quadro mediano"),
    ("Generated frames (FG)", "Quadros gerados (FG)"),
    ("Dropped frames", "Quadros perdidos"),
    ("Export HTML", "Exportar HTML"),
    ("Export PNG", "Exportar PNG"),
    ("Saved {}", "Salvo em {}"),
    ("Delete this session's files?", "Excluir os arquivos desta sessão?"),
    ("Session deleted.", "Sessão excluída."),
    ("Distribution", "Distribuição"),
    ("Frame time at each percentile", "Tempo de quadro em cada percentil"),
    ("Share of frames per FPS range", "Parcela de quadros por faixa de FPS"),
    ("Comparing {} sessions", "Comparando {} sessões"),
    ("Different games", "Jogos diferentes"),
    ("Different resolutions", "Resoluções diferentes"),
    ("Different GPU drivers", "Drivers de GPU diferentes"),
    ("Different 1% low conventions", "Convenções de 1% mínimo diferentes"),
    (
        "These runs vary more than 5 %: run it 3x and use the median ({} / {} / {} FPS).",
        "Estas execuções variam mais de 5%: rode 3 vezes e use a mediana ({} / {} / {} FPS).",
    ),
    ("Session", "Sessão"),
];

pub const ES: &[(&str, &str)] = &[
    ("The file has no frames.", "El archivo no tiene fotogramas."),
    ("Imported {} frames.", "{} fotogramas importados."),
    ("Path of a PresentMon, CapFrameX or MangoHud CSV", "Ruta de un CSV de PresentMon, CapFrameX o MangoHud"),
    ("Import", "Importar"),
    ("Open folder", "Abrir carpeta"),
    ("Reading sessions…", "Leyendo sesiones…"),
    (
        "Start a capture on the Live tab (or with its hotkey), or import a CSV. Sessions are saved on this computer only.",
        "Inicia una captura en la pestaña En vivo (o con su atajo), o importa un CSV. Las sesiones se guardan solo en este equipo.",
    ),
    ("Date", "Fecha"),
    ("Game", "Juego"),
    ("Duration", "Duración"),
    ("Average FPS", "FPS medio"),
    ("Mode", "Modo"),
    ("Manual", "Manual"),
    ("Timed", "Cronometrada"),
    ("Automatic", "Automática"),
    ("Imported", "Importada"),
    (
        "Click a session for its report; Ctrl+click several to compare them.",
        "Haz clic en una sesión para ver su informe; Ctrl+clic en varias para compararlas.",
    ),
    ("Stutters", "Tirones"),
    ("1% low = 99th-percentile frame time", "1% mínimo = percentil 99 del tiempo de fotograma"),
    ("1% low = average of the slowest 1% of frames", "1% mínimo = media del 1% de fotogramas más lentos"),
    ("Min / max FPS", "FPS mín. / máx."),
    ("Median frame time", "Tiempo de fotograma mediano"),
    ("Generated frames (FG)", "Fotogramas generados (FG)"),
    ("Dropped frames", "Fotogramas perdidos"),
    ("Export HTML", "Exportar HTML"),
    ("Export PNG", "Exportar PNG"),
    ("Saved {}", "Guardado en {}"),
    ("Delete this session's files?", "¿Eliminar los archivos de esta sesión?"),
    ("Session deleted.", "Sesión eliminada."),
    ("Distribution", "Distribución"),
    ("Frame time at each percentile", "Tiempo de fotograma en cada percentil"),
    ("Share of frames per FPS range", "Proporción de fotogramas por rango de FPS"),
    ("Comparing {} sessions", "Comparando {} sesiones"),
    ("Different games", "Juegos distintos"),
    ("Different resolutions", "Resoluciones distintas"),
    ("Different GPU drivers", "Controladores de GPU distintos"),
    ("Different 1% low conventions", "Convenciones de 1% mínimo distintas"),
    (
        "These runs vary more than 5 %: run it 3x and use the median ({} / {} / {} FPS).",
        "Estas ejecuciones varían más del 5%: ejecútalo 3 veces y usa la mediana ({} / {} / {} FPS).",
    ),
    ("Session", "Sesión"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mangohud_log_imports_as_a_session() {
        let root = std::env::temp_dir().join(format!("cz-import-{}", std::process::id()));
        let src = root.join("cs2_2026-11-05.csv");
        std::fs::create_dir_all(&root).unwrap();
        let mut log = String::from(
            "os,cpu
Linux,x
fps,frametime,elapsed
",
        );
        for i in 0..120 {
            log += &format!(
                "120,{},{}
",
                if i == 60 { 40.0 } else { 8.0 },
                i * 8_000_000
            );
        }
        std::fs::write(&src, log).unwrap();
        let sessions = root.join("sessions");
        assert!(import(&src, Some(sessions.clone())).is_ok());
        let dir = std::fs::read_dir(&sessions).unwrap().flatten().next().unwrap().path();
        let l = load(&dir).unwrap();
        assert_eq!((l.frames.len(), l.session.mode, l.session.summary.stutters), (120, Mode::Imported, 1));
        assert!(import(&root.join("missing.csv"), Some(sessions)).is_err());
        let _ = std::fs::remove_dir_all(root);
    }
}
