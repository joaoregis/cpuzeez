//! Pieces every benchmark view shares (S1-K01/K09): run-count picker,
//! progress + cancel, the sub-test table (value, points, spread), the
//! conditions line and "Save to history".

use super::i18n::{tr, trf};
use super::theme;
use super::widgets;
use crate::Ctx;
use crate::benchkit::{Conditions, Job, Sub};
use eframe::egui::{self, RichText};

pub const PT_BR: &[(&str, &str)] = &[
    ("Runs", "Execuções"),
    ("Cancel", "Cancelar"),
    ("Test", "Teste"),
    ("Result", "Resultado"),
    ("Points", "Pontos"),
    ("Spread", "Variação"),
    ("Score", "Pontuação"),
    ("Min", "Mín"),
    ("Median", "Mediana"),
    ("Max", "Máx"),
    ("Core-to-core latency", "Latência entre núcleos"),
    ("CPU suite", "Suíte de CPU"),
    ("CPU scaling", "Escala da CPU"),
    ("CPU P × E", "CPU P × E"),
    ("CPU sustained", "CPU sustentada"),
    ("Storage", "Armazenamento"),
    ("{} threads", "{} threads"),
    ("Stopped: {}", "Interrompido: {}"),
    ("Power plan: {}", "Plano de energia: {}"),
    ("Starting CPU temperature: {}", "Temperatura inicial da CPU: {}"),
    ("Median of {} runs", "Mediana de {} execuções"),
    (
        "1000 points = the reference machine (Ryzen 9 7900X). The total is the geometric mean of the sub-tests.",
        "1000 pontos = a máquina de referência (Ryzen 9 7900X). O total é a média geométrica dos subtestes.",
    ),
];

pub const ES: &[(&str, &str)] = &[
    ("Runs", "Ejecuciones"),
    ("Cancel", "Cancelar"),
    ("Test", "Prueba"),
    ("Result", "Resultado"),
    ("Points", "Puntos"),
    ("Spread", "Variación"),
    ("Score", "Puntuación"),
    ("Min", "Mín"),
    ("Median", "Mediana"),
    ("Max", "Máx"),
    ("Core-to-core latency", "Latencia entre núcleos"),
    ("CPU suite", "Suite de CPU"),
    ("CPU scaling", "Escalado de CPU"),
    ("CPU P × E", "CPU P × E"),
    ("CPU sustained", "CPU sostenida"),
    ("Storage", "Almacenamiento"),
    ("{} threads", "{} hilos"),
    ("Stopped: {}", "Detenido: {}"),
    ("Power plan: {}", "Plan de energía: {}"),
    ("Starting CPU temperature: {}", "Temperatura inicial de la CPU: {}"),
    ("Median of {} runs", "Mediana de {} ejecuciones"),
    (
        "1000 points = the reference machine (Ryzen 9 7900X). The total is the geometric mean of the sub-tests.",
        "1000 puntos = la máquina de referencia (Ryzen 9 7900X). El total es la media geométrica de las subpruebas.",
    ),
];

/// The CPU temperature now, for the conditions a run starts under.
pub fn cpu_temp(mon: &mut super::sensors::Monitor) -> Option<f64> {
    let tel = mon.tel();
    tel.want("bench", &["cpu/"]);
    super::sensors::cpu_temp(&tel.series_with(|id| id.starts_with("cpu/"))).map(|s| s.current)
}

/// 1 / 3 / 5 runs.
pub fn runs_picker(ui: &mut egui::Ui, runs: &mut usize) {
    ui.label(tr("Runs"));
    for n in [1, 3, 5] {
        ui.selectable_value(runs, n, n.to_string());
    }
}

/// While `job` runs: progress and a Cancel button. Returns its result once.
pub fn poll<T: Send + 'static>(ui: &mut egui::Ui, job: &mut Option<Job<T>>) -> Option<Result<T, String>> {
    let j = job.as_ref()?;
    if let Some(r) = j.take() {
        *job = None;
        return Some(r);
    }
    ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
    let (stage, f) = j.stage();
    ui.horizontal(|ui| {
        widgets::progress(ui, f, &label(&stage));
        if ui.button(tr("Cancel")).clicked() {
            j.cancel();
        }
    });
    None
}

/// "Integer ST" → translated test name + tag; "8 threads" → translated.
pub fn label(name: &str) -> String {
    if let Some((base, tag)) = name.rsplit_once(' ').filter(|(_, t)| matches!(*t, "ST" | "MT")) {
        return format!("{} {tag}", tr(base));
    }
    if let Some((a, b)) = name.split_once(" · ") {
        return format!("{} · {b}", tr(a));
    }
    if let Some(n) = name.strip_suffix(" threads") {
        return trf("{} threads", &[&n]);
    }
    tr(name).to_string()
}

/// The sub-test table.
pub fn table(ui: &mut egui::Ui, id: &str, subs: &[Sub]) {
    let p = theme::palette(ui);
    let scored = subs.iter().any(|s| s.points.is_some());
    let spread = subs.iter().any(|s| s.spread.is_some());
    egui::Grid::new(id).striped(true).spacing([24.0, 5.0]).show(ui, |ui| {
        ui.label(RichText::new(tr("Test")).strong());
        ui.label(RichText::new(tr("Result")).strong());
        if scored {
            ui.label(RichText::new(tr("Points")).strong());
        }
        if spread {
            ui.label(RichText::new(tr("Spread")).strong());
        }
        ui.end_row();
        for s in subs {
            ui.label(label(&s.name));
            ui.label(RichText::new(format!("{} {}", num(s.value), s.unit)).monospace().color(p.text));
            if scored {
                ui.label(RichText::new(s.points.map_or("–".into(), |x| format!("{x:.0}"))).monospace());
            }
            if spread {
                let t = s.spread.map_or("–".into(), |x| format!("±{:.1}%", x / 2.0));
                let c = if s.spread.is_some_and(|x| x > 10.0) { p.warning } else { p.text_secondary };
                ui.label(RichText::new(t).monospace().color(c));
            }
            ui.end_row();
        }
    });
}

/// Three significant figures without going scientific.
pub fn num(v: f64) -> String {
    match v.abs() {
        x if x >= 100.0 => format!("{v:.0}"),
        x if x >= 10.0 => format!("{v:.1}"),
        _ => format!("{v:.2}"),
    }
}

/// Power plan, starting temperature and run count.
pub fn conditions(ui: &mut egui::Ui, c: &Conditions) {
    let p = theme::palette(ui);
    let mut parts = Vec::new();
    if let Some(plan) = &c.power_plan {
        parts.push(trf("Power plan: {}", &[plan]));
    }
    if let Some(t) = c.start_temp_c {
        parts.push(trf("Starting CPU temperature: {}", &[&super::units::temp(t, 0)]));
    }
    if c.runs > 1 {
        parts.push(trf("Median of {} runs", &[&c.runs]));
    }
    if !parts.is_empty() {
        ui.label(RichText::new(parts.join(" · ")).small().color(p.text_muted));
    }
}

/// A finished result, ready to save.
pub struct Saved {
    pub kind: &'static str,
    pub score: f64,
    pub unit: &'static str,
    pub variant: String,
    pub threads: usize,
    pub subs: Vec<Sub>,
    pub conditions: Conditions,
    pub series: Vec<(f64, f64)>,
}

impl Saved {
    pub fn entry(&self, ctx: &Ctx, label: &str) -> crate::history::Entry {
        let mut e = crate::history::system_entry(ctx, self.kind, self.score, self.unit);
        e.label = label.into();
        e.variant = self.variant.clone();
        e.threads = self.threads;
        e.details = serde_json::json!({
            "subs": self.subs,
            "conditions": self.conditions,
            "series": self.series,
        });
        e
    }
}

/// Label + "Save to history" under a result; `status` holds "Saved".
pub fn save(ui: &mut egui::Ui, ctx: &Ctx, saved: &Saved, label: &mut String, status: &mut String) {
    if super::bench_tab::save_row(ui, label, status, status.is_empty()) {
        crate::history::add(saved.entry(ctx, label));
        *status = tr("Saved to history.").into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_numbers() {
        assert_eq!(label("Integer ST"), "Integer ST");
        assert_eq!(label("8 threads"), "8 threads");
        assert_eq!(label("Integer · P-Cores"), "Integer · P-Cores");
        assert_eq!((num(1234.5), num(12.34), num(1.234)), ("1234".into(), "12.3".into(), "1.23".into()));
    }
}
