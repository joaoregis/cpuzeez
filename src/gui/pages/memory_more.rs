//! Memory page, S1-G05 additions: secondary/tertiary timings, the active
//! profile, slots, ECC, DIMM temperatures, what the OS uses RAM for, and
//! theoretical bandwidth.

#[allow(unused_imports)]
use crate::gui::*;
use cpuzeez_core::detail::Level;
use cpuzeez_core::spd::decode::Timing;

pub const PT_BR: &[(&str, &str)] = &[
    ("Secondary timings", "Timings secundários"),
    ("Active profile", "Perfil ativo"),
    ("Running", "Em uso"),
    ("{} ({}-{}-{}-{} @ {} MT/s)", "{} ({}-{}-{}-{} @ {} MT/s)"),
    ("{} with tuned {}", "{} com {} ajustados"),
    ("JEDEC (no profile)", "JEDEC (sem perfil)"),
    ("Doesn't match a stored profile: set by hand", "Não corresponde a um perfil salvo: ajustado manualmente"),
    ("Slots", "Slots"),
    ("Empty", "Vazio"),
    ("Error correction (ECC)", "Correção de erros (ECC)"),
    ("Corrected errors", "Erros corrigidos"),
    ("Uncorrected errors", "Erros não corrigidos"),
    ("Module temperatures", "Temperaturas dos módulos"),
    ("Used by the OS", "Uso pelo sistema"),
    ("In use", "Em uso"),
    ("Standby / cache", "Em espera / cache"),
    ("Compressed", "Comprimida"),
    ("Free", "Livre"),
    ("Bandwidth", "Largura de banda"),
    ("Theoretical", "Teórica"),
    ("Measure it", "Medir"),
    ("Fabric clock (FCLK)", "Clock do fabric (FCLK)"),
];

pub const ES: &[(&str, &str)] = &[
    ("Secondary timings", "Latencias secundarias"),
    ("Active profile", "Perfil activo"),
    ("Running", "En uso"),
    ("{} ({}-{}-{}-{} @ {} MT/s)", "{} ({}-{}-{}-{} @ {} MT/s)"),
    ("{} with tuned {}", "{} con {} ajustados"),
    ("JEDEC (no profile)", "JEDEC (sin perfil)"),
    ("Doesn't match a stored profile: set by hand", "No coincide con un perfil guardado: ajustado a mano"),
    ("Slots", "Ranuras"),
    ("Empty", "Vacía"),
    ("Error correction (ECC)", "Corrección de errores (ECC)"),
    ("Corrected errors", "Errores corregidos"),
    ("Uncorrected errors", "Errores no corregidos"),
    ("Module temperatures", "Temperaturas de los módulos"),
    ("Used by the OS", "Uso del sistema"),
    ("In use", "En uso"),
    ("Standby / cache", "En espera / caché"),
    ("Compressed", "Comprimida"),
    ("Free", "Libre"),
    ("Bandwidth", "Ancho de banda"),
    ("Theoretical", "Teórico"),
    ("Measure it", "Medirlo"),
    ("Fabric clock (FCLK)", "Reloj del fabric (FCLK)"),
];

/// Live timings (from the memory controller) as SPD-comparable numbers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Live {
    pub mhz: f64,
    pub cl: f64,
    pub rcd: u32,
    pub rp: u32,
    pub ras: u32,
}

/// Which stored profile the memory runs, and which primaries differ.
/// None: no profile at this speed (JEDEC or a manual overclock).
pub(crate) fn active_profile<'a>(
    live: &Live,
    profiles: &'a [(String, Timing)],
) -> Option<(&'a str, &'a Timing, Vec<&'static str>)> {
    let (name, t) = profiles.iter().find(|(_, t)| t.freq > 0.0 && (live.mhz - t.freq).abs() / t.freq < 0.02)?;
    let mut tuned = Vec::new();
    for (k, a, b) in [
        ("CL", live.cl, t.cl),
        ("tRCD", live.rcd as f64, t.rcd as f64),
        ("tRP", live.rp as f64, t.rp as f64),
        ("tRAS", live.ras as f64, t.ras as f64),
    ] {
        if (a - b).abs() > 0.01 {
            tuned.push(k);
        }
    }
    Some((name.as_str(), t, tuned))
}

fn num(s: &str) -> Option<f64> {
    s.split_whitespace().next()?.parse().ok()
}

impl App {
    pub(crate) fn memory_more(&mut self, ui: &mut egui::Ui, r: &[Section]) {
        let level = detail::get(ui.ctx());
        let k = |key: &str| sget(r, "Chipset", key);
        let mods = crate::spd::modules(&self.ctx);

        // Active profile: live primaries vs the first module's profiles.
        let live = (|| {
            Some(Live {
                mhz: num(&k("Memory Frequency"))?,
                cl: num(&k("CAS# latency (CL)"))?,
                rcd: num(&k("RAS# to CAS# delay (tRCD)"))? as u32,
                rp: num(&k("RAS# Precharge (tRP)"))? as u32,
                ras: num(&k("Cycle Time (tRAS)"))? as u32,
            })
        })();
        if let (Some(live), Some(m)) = (live, mods.first()) {
            let profiles: Vec<(String, Timing)> = m
                .profiles
                .iter()
                .map(|p| (if p.name.is_empty() { p.kind.to_string() } else { p.name.clone() }, p.timing.clone()))
                .collect();
            let text = match active_profile(&live, &profiles) {
                Some((name, t, tuned)) => {
                    // XMP and EXPO often hold the same numbers: name every exact match.
                    let same: Vec<&str> = profiles
                        .iter()
                        .filter(|(_, p)| {
                            p.freq == t.freq && p.cl == t.cl && p.rcd == t.rcd && p.rp == t.rp && p.ras == t.ras
                        })
                        .map(|(n, _)| n.as_str())
                        .collect();
                    let name = if same.len() > 1 { same.join(" / ") } else { name.to_string() };
                    let base = trf(
                        "{} ({}-{}-{}-{} @ {} MT/s)",
                        &[&name, &t.cl, &t.rcd, &t.rp, &t.ras, &format!("{:.0}", t.freq * 2.0)],
                    );
                    if tuned.is_empty() { base } else { trf("{} with tuned {}", &[&base, &tuned.join(", ")]) }
                }
                None if m.jedec.iter().any(|t| (live.mhz - t.freq).abs() / t.freq.max(1.0) < 0.02) => {
                    tr("JEDEC (no profile)").to_string()
                }
                None => tr("Doesn't match a stored profile: set by hand").to_string(),
            };
            boxed(ui, "Active profile", &[("Running", text)]);
        }

        if level >= Level::Advanced {
            let secondary: Vec<(&str, String)> = [
                "RAS# to CAS# write (tRCDWR)",
                "tRRD_S",
                "tRRD_L",
                "tFAW",
                "tWR",
                "tRTP",
                "tCWL",
                "tWTR_S",
                "tWTR_L",
                "tREFI",
                "tRFC2",
            ]
            .into_iter()
            .map(|key| (key, k(key)))
            .chain([("Fabric clock (FCLK)", k("Fabric Frequency (FCLK)"))])
            .filter(|(_, v)| !v.is_empty())
            .collect();
            if !secondary.is_empty() {
                boxed(ui, "Secondary timings", &secondary);
            }
        }

        // Slots: every SMBIOS memory device, populated or not.
        if let Some(dmi) = sec(r, "DMI") {
            let slots: Vec<(String, String)> = report::groups(&dmi.nodes, "DMI Memory Device")
                .iter()
                .filter_map(|(_, c)| {
                    let name = report::find(c, "designation")?.to_string();
                    let size = report::find(c, "size").filter(|s| s.chars().next().is_some_and(|c| c.is_ascii_digit()));
                    let what = match size {
                        Some(s) => [
                            Some(s),
                            report::find(c, "type"),
                            report::find(c, "speed"),
                            report::find(c, "manufacturer"),
                        ]
                        .into_iter()
                        .flatten()
                        .filter(|v| !v.is_empty() && *v != "Unknown")
                        .collect::<Vec<_>>()
                        .join(" · "),
                        None => tr("Empty").to_string(),
                    };
                    Some((name, what))
                })
                .collect();
            let ecc = report::groups(&dmi.nodes, "DMI Physical Memory Array")
                .first()
                .and_then(|(_, c)| {
                    report::find(c, "error correction").or_else(|| report::find(c, "error correction type"))
                })
                .unwrap_or_default()
                .to_string();
            let mut rows: Vec<(&str, String)> = slots.iter().map(|(a, b)| (a.as_str(), b.clone())).collect();
            if !ecc.is_empty() {
                rows.push(("Error correction (ECC)", ecc));
            }
            let (ce, ue) = edac();
            if let Some(ce) = ce {
                rows.push(("Corrected errors", ce.to_string()));
            }
            if let Some(ue) = ue {
                rows.push(("Uncorrected errors", ue.to_string()));
            }
            if !rows.is_empty() {
                boxed(ui, "Slots", &rows);
            }
        }

        let tel = self.mon.tel();
        tel.want("memory-page", &["mem/"]);
        let snap = tel.snapshot();
        let temps: Vec<(String, String)> = snap
            .stats
            .iter()
            .filter(|s| s.id.starts_with("mem/temp/"))
            .map(|s| (s.label.clone(), units::temp(s.current, 1)))
            .collect();
        if !temps.is_empty() {
            let rows: Vec<(&str, String)> = temps.iter().map(|(a, b)| (a.as_str(), b.clone())).collect();
            boxed(ui, "Module temperatures", &rows);
        }

        // The OS's view: a bar of in use / standby / compressed / free.
        let mb = |id: &str| snap.get(id).map(|s| s.current);
        if let (Some(used), Some(total)) =
            (mb("mem/size/used"), mb("mem/size/available").zip(mb("mem/size/used")).map(|(a, u)| a + u))
        {
            let p = theme::palette(ui);
            let parts = [
                ("In use", used - mb("mem/size/compressed").unwrap_or(0.0), p.accent),
                ("Compressed", mb("mem/size/compressed").unwrap_or(0.0), p.warning),
                ("Standby / cache", mb("mem/size/cached").unwrap_or(0.0), p.success),
                ("Free", mb("mem/size/free").unwrap_or(0.0), p.border),
            ];
            widgets::card(ui, icon::MEMORY, tr("Used by the OS"), |ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), egui::Sense::hover());
                let mut x = rect.left();
                for (_, v, c) in parts {
                    let w = (v / total).clamp(0.0, 1.0) as f32 * rect.width();
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(w, rect.height())),
                        2.0,
                        c,
                    );
                    x += w;
                }
                ui.horizontal_wrapped(|ui| {
                    for (name, v, c) in parts {
                        ui.label(RichText::new("■").color(c));
                        ui.label(format!("{} {}", tr(name), units::get().size(v)));
                    }
                });
            });
            ui.add_space(theme::GAP);
        }

        // Theoretical bandwidth: MT/s x 8 bytes per 64-bit channel.
        let mts = num(&k("Memory Frequency")).map(|m| m * 2.0);
        let modules = mods.len().max(sec(r, "DMI").map_or(0, |d| {
            report::groups(&d.nodes, "DMI Memory Device")
                .iter()
                .filter(|(_, c)| report::find(c, "size").is_some_and(|s| s.starts_with(|c: char| c.is_ascii_digit())))
                .count()
        }));
        if let Some(mts) = mts.filter(|_| modules > 0) {
            let channels = modules.min(2) as f64; // desktop dual channel; one stick = one channel
            let gbs = mts * 8.0 * channels / 1000.0;
            let mut go = false;
            widgets::card(ui, icon::GAUGE, tr("Bandwidth"), |ui| {
                widgets::kv(ui, tr("Theoretical"), &format!("{gbs:.1} GB/s"));
                go = ui.button(tr("Measure it")).clicked();
            });
            if go {
                self.page = Page::Benchmarks;
                self.bench_view = BenchView::Memory;
            }
            ui.add_space(theme::GAP);
        }
    }
}

/// Linux EDAC error counters summed over memory controllers.
fn edac() -> (Option<u64>, Option<u64>) {
    #[cfg(target_os = "linux")]
    {
        let sum = |f: &str| -> Option<u64> {
            let dir = std::fs::read_dir("/sys/devices/system/edac/mc").ok()?;
            let v: Vec<u64> = dir
                .flatten()
                .filter_map(|e| std::fs::read_to_string(e.path().join(f)).ok()?.trim().parse().ok())
                .collect();
            (!v.is_empty()).then(|| v.iter().sum())
        };
        return (sum("ce_count"), sum("ue_count"));
    }
    #[allow(unreachable_code)]
    (None, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(freq: f64, cl: f64, rcd: u32, rp: u32, ras: u32) -> Timing {
        Timing { freq, cl, rcd, rp, ras, ..Timing::default() }
    }

    #[test]
    fn finds_the_running_profile_and_tuning() {
        let profiles = vec![
            ("XMP-6000".to_string(), t(3000.0, 30.0, 36, 36, 76)),
            ("XMP-5600".to_string(), t(2800.0, 28.0, 34, 34, 70)),
        ];
        let live = Live { mhz: 2999.9, cl: 30.0, rcd: 36, rp: 36, ras: 76 };
        let (name, _, tuned) = active_profile(&live, &profiles).unwrap();
        assert_eq!((name, tuned.is_empty()), ("XMP-6000", true));
        let tuned_live = Live { ras: 68, ..live };
        assert_eq!(active_profile(&tuned_live, &profiles).unwrap().2, ["tRAS"]);
        assert!(active_profile(&Live { mhz: 2400.0, ..live }, &profiles).is_none());
    }
}
