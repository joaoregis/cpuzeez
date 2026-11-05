//! Command palette (S1-U02): Ctrl+K searches every field of the report
//! ("tRFC", "PCIe", "BIOS"), the pages and the app's actions, and Enter goes
//! there or runs it. Matching and ranking are plain functions, tested below.

use super::Page;
use super::i18n::tr;
use crate::report::{Node, Section};
use eframe::egui;

pub(super) const PT_BR: &[(&str, &str)] = &[
    ("Search pages, values and actions…", "Buscar páginas, valores e ações…"),
    ("No matches", "Nada encontrado"),
    ("Page", "Página"),
    ("Action", "Ação"),
    ("Save report as text", "Salvar relatório como texto"),
    ("Save report as HTML", "Salvar relatório como HTML"),
    ("Start or stop the sensor log", "Iniciar ou parar o log de sensores"),
    ("Show or hide the mini monitor", "Mostrar ou ocultar o mini monitor"),
    ("Reset sensor min/max", "Zerar mín/máx dos sensores"),
    ("Open the stress test", "Abrir o teste de estresse"),
];

pub(super) const ES: &[(&str, &str)] = &[
    ("Search pages, values and actions…", "Buscar páginas, valores y acciones…"),
    ("No matches", "Sin resultados"),
    ("Page", "Página"),
    ("Action", "Acción"),
    ("Save report as text", "Guardar informe como texto"),
    ("Save report as HTML", "Guardar informe como HTML"),
    ("Start or stop the sensor log", "Iniciar o detener el registro de sensores"),
    ("Show or hide the mini monitor", "Mostrar u ocultar el mini monitor"),
    ("Reset sensor min/max", "Reiniciar mín/máx de sensores"),
    ("Open the stress test", "Abrir la prueba de estrés"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Act {
    SaveTxt,
    SaveHtml,
    ToggleCsv,
    ToggleMini,
    ResetMinMax,
    Stress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Target {
    Page(Page),
    Act(Act),
}

#[derive(Debug, Clone)]
pub(super) struct Entry {
    /// What's matched and shown first: a field's label, a page or an action name.
    pub(super) title: String,
    /// The value, or where it lives.
    pub(super) detail: String,
    pub(super) target: Target,
}

const ACTIONS: [(Act, &str); 6] = [
    (Act::SaveTxt, "Save report as text"),
    (Act::SaveHtml, "Save report as HTML"),
    (Act::ToggleCsv, "Start or stop the sensor log"),
    (Act::ToggleMini, "Show or hide the mini monitor"),
    (Act::ResetMinMax, "Reset sensor min/max"),
    (Act::Stress, "Open the stress test"),
];

/// The page that shows a report section's values.
fn page_of(section: &str) -> Page {
    match section {
        "Processors" | "Processors Information" | "APICs" | "Thread dumps" | "Timers" => Page::Processor,
        "DMI" | "BIOS" | "Chipset" | "LPCIO" | "Register Spaces" => Page::Mainboard,
        "Memory SPD" => Page::Memory,
        "Display Adapters" | "Graphic APIs" | "Graphics" => Page::Graphics,
        "Hardware Monitors" | "Monitoring" => Page::Sensors,
        "Software" => Page::About,
        _ => Page::Devices,
    }
}

/// Every searchable thing: pages, actions, then each report field.
pub(super) fn index(report: &[Section], pages: &[(Page, &str)]) -> Vec<Entry> {
    let mut v: Vec<Entry> = pages
        .iter()
        .map(|(p, name)| Entry {
            title: tr(name).to_string(),
            detail: tr("Page").to_string(),
            target: Target::Page(*p),
        })
        .collect();
    v.extend(ACTIONS.iter().map(|(a, name)| Entry {
        title: tr(name).to_string(),
        detail: tr("Action").to_string(),
        target: Target::Act(*a),
    }));
    fn walk(nodes: &[Node], path: &str, page: Page, out: &mut Vec<Entry>) {
        for n in nodes {
            match n {
                Node::Field { key, value } if !value.is_empty() && !value.contains('\t') => out.push(Entry {
                    title: key.clone(),
                    detail: format!("{value} · {path}"),
                    target: Target::Page(page),
                }),
                Node::Group { title, value, children } => {
                    let here = if value.is_empty() { title.clone() } else { format!("{title} {value}") };
                    walk(children, &format!("{path} › {here}"), page, out);
                }
                _ => {}
            }
        }
    }
    for s in report {
        walk(&s.nodes, &s.title, page_of(&s.title), &mut v);
    }
    v
}

/// Best matches for `q`: every word must appear in the title or detail;
/// title-prefix matches first, then title matches, then the rest.
pub(super) fn search<'a>(entries: &'a [Entry], q: &str, limit: usize) -> Vec<&'a Entry> {
    let words: Vec<String> = q.split_whitespace().map(str::to_lowercase).collect();
    if words.is_empty() {
        return entries.iter().filter(|e| e.detail == tr("Page") || e.detail == tr("Action")).take(limit).collect();
    }
    let mut hits: Vec<(u8, usize, &Entry)> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let (t, d) = (e.title.to_lowercase(), e.detail.to_lowercase());
            if !words.iter().all(|w| t.contains(w.as_str()) || d.contains(w.as_str())) {
                return None;
            }
            let rank = if t.starts_with(words[0].as_str()) {
                0
            } else if t.contains(words[0].as_str()) {
                1
            } else {
                2
            };
            Some((rank, i, e))
        })
        .collect();
    hits.sort_by_key(|h| (h.0, h.1));
    hits.into_iter().take(limit).map(|h| h.2).collect()
}

/// The palette's state while open.
#[derive(Default)]
pub(super) struct Palette {
    pub(super) open: bool,
    query: String,
    sel: usize,
    entries: Vec<Entry>,
}

impl Palette {
    pub(super) fn open(&mut self, report: &[Section], pages: &[(Page, &str)]) {
        *self = Palette { open: true, entries: index(report, pages), ..Default::default() };
    }

    /// Draws it; returns what the user picked.
    pub(super) fn show(&mut self, ctx: &egui::Context) -> Option<Target> {
        if !self.open {
            return None;
        }
        let mut picked = None;
        let modal = egui::Modal::new(egui::Id::new("cz-palette")).show(ctx, |ui| {
            ui.set_width(520.0);
            let edit = ui.add(
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text(tr("Search pages, values and actions…"))
                    .desired_width(f32::INFINITY),
            );
            edit.request_focus();
            if edit.changed() {
                self.sel = 0;
            }
            let hits = search(&self.entries, &self.query, 12);
            let (up, down, enter) = ui.input(|i| {
                (
                    i.key_pressed(egui::Key::ArrowUp),
                    i.key_pressed(egui::Key::ArrowDown),
                    i.key_pressed(egui::Key::Enter),
                )
            });
            if down {
                self.sel = (self.sel + 1).min(hits.len().saturating_sub(1));
            }
            if up {
                self.sel = self.sel.saturating_sub(1);
            }
            ui.add_space(6.0);
            if hits.is_empty() {
                ui.weak(tr("No matches"));
            }
            for (i, e) in hits.iter().enumerate() {
                let text = format!("{}  —  {}", e.title, e.detail);
                if ui.selectable_label(i == self.sel, text).clicked() || (enter && i == self.sel) {
                    picked = Some(e.target);
                }
            }
        });
        if picked.is_some() || modal.should_close() {
            self.open = false;
        }
        picked
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{f, g, section};

    fn report() -> Vec<Section> {
        vec![
            section("Memory SPD", vec![g("DIMM #1", vec![f("Row Refresh Cycle Time (tRFC)", "560.0")])]),
            section("PCI Devices", vec![f("Link", "PCIe 4.0 x16")]),
            section("DMI", vec![f("BIOS version", "1.A2A")]),
        ]
    }

    #[test]
    fn finds_fields_and_the_page_that_shows_them() {
        let idx = index(&report(), &[(Page::Overview, "Overview"), (Page::Memory, "Memory")]);
        let hit = search(&idx, "trfc", 5);
        assert_eq!(hit[0].title, "Row Refresh Cycle Time (tRFC)");
        assert_eq!(hit[0].target, Target::Page(Page::Memory));
        assert!(hit[0].detail.contains("560.0") && hit[0].detail.contains("DIMM #1"));
        assert_eq!(search(&idx, "pcie", 5)[0].target, Target::Page(Page::Devices));
        assert_eq!(search(&idx, "bios", 5)[0].target, Target::Page(Page::Mainboard));
    }

    #[test]
    fn ranks_title_prefix_first_and_needs_every_word() {
        let idx = index(&report(), &[(Page::Memory, "Memory")]);
        assert_eq!(search(&idx, "mem", 5)[0].target, Target::Page(Page::Memory), "page name before field details");
        assert!(search(&idx, "trfc 999", 5).is_empty());
        assert!(search(&idx, "save html", 5).iter().any(|e| e.target == Target::Act(Act::SaveHtml)));
        assert!(!search(&idx, "", 50).is_empty(), "empty query lists pages and actions");
    }

    #[test]
    fn typing_and_enter_picks_the_first_hit() {
        use egui_kittest::{Harness, kittest::Queryable};
        let mut h = Harness::builder().with_size(egui::vec2(800.0, 600.0)).build_ui_state(
            |ui, st: &mut (Palette, Option<Target>)| {
                if let Some(t) = st.0.show(ui.ctx()) {
                    st.1 = Some(t);
                }
            },
            (Palette::default(), None),
        );
        h.state_mut().0.open(&report(), &[(Page::Memory, "Memory")]);
        h.run();
        h.get_by_role(egui::accesskit::Role::TextInput).type_text("trfc");
        h.run();
        h.key_press(egui::Key::Enter);
        h.run();
        assert_eq!(h.state().1, Some(Target::Page(Page::Memory)));
        assert!(!h.state().0.open, "closes after picking");
    }
}
