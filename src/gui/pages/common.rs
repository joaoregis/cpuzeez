//! Helpers every page uses (M4-08): headings, boxed rows, report lookups,
//! the screenshot mask.

#[allow(unused_imports)]
use crate::gui::*;

/// `CZ_FRAME_LOG=1`: frames whose part took longer than 4 ms, on stderr
/// (a 60 Hz frame has 16 ms for everything).
pub(in crate::gui) fn frame_log(page: &str, part: &str, t0: std::time::Instant) {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let ms = t0.elapsed().as_secs_f64() * 1000.0;
    if *ON.get_or_init(|| std::env::var_os("CZ_FRAME_LOG").is_some()) && ms > 4.0 {
        eprintln!("frame {page} {part} {ms:.1} ms");
    }
}

pub(in crate::gui) fn value(ui: &mut egui::Ui, text: &str) {
    ui.add(egui::Label::new(RichText::new(text).monospace()).wrap());
}

/// Set by `-screenshots`: published images shouldn't carry the machine's
/// serial numbers, MAC addresses or IP addresses.
pub(in crate::gui) static MASK_PRIVATE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub(in crate::gui) fn shown<'a>(key: &str, v: &'a str) -> &'a str {
    if !MASK_PRIVATE.load(std::sync::atomic::Ordering::Relaxed) || v.is_empty() {
        return v;
    }
    if report::is_private(key) { "••••••••" } else { v }
}

/// A card of label/value rows (labels translated, private values masked),
/// keeping the rows the page's detail level shows; nothing at all if none.
pub(in crate::gui) fn boxed(ui: &mut egui::Ui, title: &str, rows: &[(&str, String)]) {
    use cpuzeez_core::detail::{level_of, shows};
    let level = detail::get(ui.ctx());
    let rows: Vec<(&str, String)> =
        rows.iter().filter(|(k, _)| shows(level, level_of(k))).map(|(k, v)| (tr(k), shown(k, v).to_string())).collect();
    if rows.is_empty() {
        return;
    }
    widgets::card(ui, "", tr(title), |ui| widgets::kv_list(ui, &rows));
    ui.add_space(theme::GAP);
}

/// Expert: thread 0's raw CPUID leaves and MSRs, from the report's thread dumps.
pub(in crate::gui) fn cpuid_dump(ui: &mut egui::Ui, r: &[Section]) {
    let Some(t0) = sec(r, "Thread dumps").and_then(|s| report::groups(&s.nodes, "CPU Thread").into_iter().next())
    else {
        return;
    };
    for (group, title) in [("CPUID", "CPUID leaves (thread 0)"), ("MSR", "MSRs (thread 0)")] {
        let Some((_, rows)) = report::groups(t0.1, group).into_iter().next() else { continue };
        heading(ui, tr(title));
        widgets::card(ui, "", "", |ui| {
            for (k, v) in report::fields(rows) {
                ui.label(RichText::new(format!("{k}  {}", v.replace('\t', "  "))).monospace());
            }
        });
        ui.add_space(theme::GAP);
    }
}

/// Section label between cards.
pub(in crate::gui) fn heading(ui: &mut egui::Ui, text: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(text).text_style(egui::TextStyle::Heading).color(theme::palette(ui).text));
    ui.add_space(4.0);
}

pub(in crate::gui) fn sec<'a>(r: &'a [Section], t: &str) -> Option<&'a Section> {
    report::find_section(r, t)
}

pub(in crate::gui) fn sget(r: &[Section], s: &str, k: &str) -> String {
    sec(r, s).and_then(|x| x.get(k)).unwrap_or_default().to_string()
}

pub(in crate::gui) fn sgi(r: &[Section], s: &str, g: &str, k: &str) -> String {
    sec(r, s).and_then(|x| x.get_in(g, k)).unwrap_or_default().to_string()
}

pub(in crate::gui) fn no_data(ui: &egui::Ui, what: &str, ctx: &Ctx) -> RichText {
    let why = if ctx.hw.name().starts_with("none") {
        "this needs privileged hardware access (see README: PawnIO on Windows, root on Linux)"
    } else {
        "nothing was detected"
    };
    RichText::new(trf("No {} information: {}.", &[&tr(what), &tr(why)])).color(theme::palette(ui).text_muted)
}

pub(in crate::gui) fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .or_else(|| crate::os::read_trim("/etc/hostname"))
        .or_else(|| crate::os::cmd("hostname", &[]).map(|s| s.trim().to_string()))
        .unwrap_or_else(|| crate::NAME.into())
}

impl App {
    /// A page header with the detail switch; the chosen level is remembered per page.
    pub(in crate::gui) fn info_header(&mut self, ui: &mut egui::Ui, title: &str, sub: &str) {
        let key = format!("{:?}", self.page);
        let mut level = self.mon.settings.detail.get(&key).copied().unwrap_or_default();
        let mut changed = false;
        widgets::page_header(ui, title, sub, |ui| changed = detail::switch(ui, &mut level));
        if changed {
            self.mon.settings.detail.insert(key, level);
            if self.auto.is_none() {
                self.mon.settings.save();
            }
        }
        detail::set(ui.ctx(), level);
    }

    pub(in crate::gui) fn save(&mut self, r: &Option<Vec<Section>>, ext: &str) {
        let Some(r) = r else { return };
        let body = match ext {
            "html" => report::to_html(r),
            "cvf" => report::to_json(r),
            _ => report::to_txt(r),
        };
        let name = format!("{}.{ext}", hostname());
        self.status = match std::fs::write(&name, body) {
            Ok(()) => {
                trf("Saved {}", &[&std::fs::canonicalize(&name).map_or(name.clone(), |p| p.display().to_string())])
            }
            Err(e) => format!("{name}: {e}"),
        };
    }
}
