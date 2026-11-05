//! Processor page, S1-G03 additions: per-core table, topology, limits,
//! security, virtualization, OS power policy and grouped instructions.

use crate::gui::table::{Cell, Column, Table};
#[allow(unused_imports)]
use crate::gui::*;
use cpuzeez_core::detail::Level;

pub const PT_BR: &[(&str, &str)] = &[
    ("Idle states", "Estados de repouso"),
    ("Preferred", "Preferido"),
    ("PL1 (long-term)", "PL1 (longo prazo)"),
    ("PL2 (short-term)", "PL2 (curto prazo)"),
    ("Cores", "Núcleos"),
    ("Topology", "Topologia"),
    ("Package {}", "Encapsulamento {}"),
    ("CCD {}", "CCD {}"),
    ("CCX {}", "CCX {}"),
    ("Core", "Núcleo"),
    ("Clock", "Clock"),
    ("Temperature", "Temperatura"),
    ("Load", "Carga"),
    ("Power limits", "Limites de potência"),
    ("Security", "Segurança"),
    ("Virtualization", "Virtualização"),
    ("Power policy", "Política de energia"),
    ("Instruction sets", "Conjuntos de instruções"),
    ("Supported by the CPU", "Suportada pela CPU"),
    ("Enabled in the firmware", "Ativada no firmware"),
    ("Hypervisor", "Hipervisor"),
    ("Virtualization-Based Security", "Segurança baseada em virtualização"),
    ("Memory integrity (HVCI)", "Integridade de memória (HVCI)"),
    ("KVM available", "KVM disponível"),
    ("Power plan", "Plano de energia"),
    ("CPU frequency driver", "Driver de frequência da CPU"),
    ("Governor", "Governador"),
    ("Energy/performance preference", "Preferência energia/desempenho"),
    ("Yes", "Sim"),
    ("No", "Não"),
    ("None", "Nenhum"),
    ("SIMD (vector math)", "SIMD (matemática vetorial)"),
    ("Cryptography", "Criptografia"),
    ("Other", "Outros"),
];

pub const ES: &[(&str, &str)] = &[
    ("Idle states", "Estados de reposo"),
    ("Preferred", "Preferido"),
    ("PL1 (long-term)", "PL1 (largo plazo)"),
    ("PL2 (short-term)", "PL2 (corto plazo)"),
    ("Cores", "Núcleos"),
    ("Topology", "Topología"),
    ("Package {}", "Encapsulado {}"),
    ("CCD {}", "CCD {}"),
    ("CCX {}", "CCX {}"),
    ("Core", "Núcleo"),
    ("Clock", "Reloj"),
    ("Temperature", "Temperatura"),
    ("Load", "Carga"),
    ("Power limits", "Límites de potencia"),
    ("Security", "Seguridad"),
    ("Virtualization", "Virtualización"),
    ("Power policy", "Política de energía"),
    ("Instruction sets", "Conjuntos de instrucciones"),
    ("Supported by the CPU", "Compatible con la CPU"),
    ("Enabled in the firmware", "Activada en el firmware"),
    ("Hypervisor", "Hipervisor"),
    ("Virtualization-Based Security", "Seguridad basada en virtualización"),
    ("Memory integrity (HVCI)", "Integridad de memoria (HVCI)"),
    ("KVM available", "KVM disponible"),
    ("Power plan", "Plan de energía"),
    ("CPU frequency driver", "Controlador de frecuencia de la CPU"),
    ("Governor", "Gobernador"),
    ("Energy/performance preference", "Preferencia energía/rendimiento"),
    ("Yes", "Sí"),
    ("No", "No"),
    ("None", "Ninguno"),
    ("SIMD (vector math)", "SIMD (matemática vectorial)"),
    ("Cryptography", "Criptografía"),
    ("Other", "Otros"),
];

/// One physical core as the table shows it.
struct CoreRow {
    index: usize,
    kind: String,
    ccd: Option<u32>,
    ccx: Option<u32>,
    mhz: Option<f64>,
    temp: Option<f64>,
    load: Option<f64>,
    /// "C1 20% · C2 70%" (deepest states last).
    cstates: String,
    /// CPPC rank (1 = preferred) and highest performance.
    cppc: Option<(usize, u32)>,
}

/// Instruction groups and what each one is for (short, plain words).
fn group_of(i: &str) -> (&'static str, &'static str) {
    let u = i.to_ascii_uppercase();
    let has = |p: &[&str]| p.iter().any(|x| u.starts_with(x));
    if has(&["AES", "SHA", "VAES", "VPCLMULQDQ", "PCLMUL", "GFNI"]) {
        ("Cryptography", "Hardware encryption and hashing: faster disk encryption, TLS and compression.")
    } else if has(&["MMX", "SSE", "SSSE", "AVX", "FMA", "F16C", "AMX", "XOP", "3DNOW"]) {
        ("SIMD (vector math)", "Do the same math on many numbers at once: media, games, compression, AI.")
    } else if has(&["VT-X", "AMD-V", "VMX", "SVM"]) {
        ("Virtualization", "Run virtual machines and Windows' security features at full speed.")
    } else if has(&["SGX", "SEV", "TME", "SMEP", "SMAP", "CET", "TDX"]) {
        ("Security", "Isolate and protect memory from other software and attacks.")
    } else {
        ("Other", "")
    }
}

impl App {
    pub(crate) fn processor_more(&mut self, ui: &mut egui::Ui) {
        if detail::get(ui.ctx()) < Level::Advanced {
            return;
        }
        let c = crate::cpu::get(&self.ctx);
        let tel = self.mon.tel();
        tel.want("processor-cores", &["cpu/"]);
        let snap = tel.snapshot();
        let get = |id: String| snap.get(&id).map(|s| s.current);
        let cst = cpuzeez_platform::cstates::read();
        let highest = crate::cpu::power::cppc(&self.ctx);
        let ranks = cpuzeez_core::cpu::limits::ranks(highest);
        let cores: Vec<CoreRow> = c
            .logical
            .iter()
            .filter(|l| l.smt == 0)
            .enumerate()
            .map(|(i, l)| {
                let threads: Vec<f64> = c
                    .logical
                    .iter()
                    .filter(|t| t.package == l.package && t.core == l.core)
                    .filter_map(|t| get(format!("cpu/load/cpu{}", t.cpu)))
                    .collect();
                CoreRow {
                    index: i,
                    kind: l.kind.clone().unwrap_or_default(),
                    ccd: l.ccd,
                    ccx: l.ccx,
                    mhz: get(format!("cpu/clock/core{i}")),
                    temp: get(format!("cpu/temp/core{i}")),
                    load: (!threads.is_empty()).then(|| threads.iter().sum::<f64>() / threads.len() as f64),
                    cstates: cst
                        .get(l.cpu)
                        .map(|v| v.iter().map(|(n, p)| format!("{n} {p:.0}%")).collect::<Vec<_>>().join(" · "))
                        .unwrap_or_default(),
                    cppc: ranks.get(i).copied().flatten().zip(highest.get(i).copied().flatten()),
                }
            })
            .collect();

        heading(ui, tr("Cores"));
        let dash = || Cell::num(f64::NAN, "–");
        let table = Table::new("cores", |r: &CoreRow| r.index as u64)
            .col(Column::new("core", tr("Core"), 70.0, |r: &CoreRow| {
                Cell::num(r.index as f64, format!("#{}", r.index))
            }))
            .col(Column::new("type", tr("Type"), 90.0, |r: &CoreRow| Cell::Text(r.kind.clone())))
            .col(Column::new("ccd", "CCD", 60.0, |r: &CoreRow| {
                r.ccd.map_or(Cell::Text(String::new()), |v| Cell::num(v as f64, v.to_string()))
            }))
            .col(Column::new("ccx", "CCX", 60.0, |r: &CoreRow| {
                r.ccx.map_or(Cell::Text(String::new()), |v| Cell::num(v as f64, v.to_string()))
            }))
            .col(
                Column::new("clock", tr("Clock"), 120.0, move |r: &CoreRow| {
                    r.mhz.map_or_else(dash, |m| Cell::num(m, units::clock(m, false, 0)))
                })
                .right(),
            )
            .col(
                Column::new("temp", tr("Temperature"), 120.0, move |r: &CoreRow| {
                    r.temp.map_or_else(dash, |t| Cell::num(t, units::temp(t, 1)))
                })
                .right(),
            )
            .col(
                Column::new("load", tr("Load"), 100.0, move |r: &CoreRow| {
                    r.load.map_or_else(dash, |l| Cell::num(l, format!("{l:.0} %")))
                })
                .right(),
            )
            .col(Column::new("cstates", tr("Idle states"), 200.0, |r: &CoreRow| Cell::Text(r.cstates.clone())))
            .col(Column::new("cppc", tr("Preferred"), 110.0, |r: &CoreRow| match r.cppc {
                Some((rank, perf)) => Cell::num(rank as f64, format!("#{rank} ({perf})")),
                None => Cell::Text(String::new()),
            }));
        // Hybrid CPUs (P/E cores) only: elsewhere the column would be empty.
        if cores.iter().all(|r| r.kind.is_empty()) {
            self.cores_table.hidden.insert("type".into());
        }
        // Columns with nothing to show here (no CPPC ranking, no C-state counters).
        if cores.iter().all(|r| r.cppc.is_none()) {
            self.cores_table.hidden.insert("cppc".into());
        }
        if cores.iter().all(|r| r.cstates.is_empty()) {
            self.cores_table.hidden.insert("cstates".into());
        }
        let h = (cores.len() as f32 + 3.0) * 30.0;
        widgets::card(ui, "", "", |ui| {
            ui.allocate_ui(egui::vec2(ui.available_width(), h.min(520.0)), |ui| {
                table.show(ui, &cores, &mut self.cores_table, |_, _| {});
            });
        });
        ui.add_space(theme::GAP);

        topology(ui, c);

        // AMD SMU limits (PawnIO / ryzen_smu); Intel PL1/PL2 from MSRs 606h/610h.
        let lim =
            |id: &str, label: &'static str, unit: &str| get(id.to_string()).map(|v| (label, format!("{v:.0} {unit}")));
        let mut limits: Vec<(&str, String)> = [
            lim("cpu/power/pptlimit", "PPT Limit", "W"),
            lim("cpu/curr/tdclimit", "TDC Limit", "A"),
            lim("cpu/curr/edclimit", "EDC Limit", "A"),
        ]
        .into_iter()
        .flatten()
        .collect();
        if let Some((pl1, pl2)) = crate::cpu::power::intel_limits(&self.ctx) {
            let off = |l: &cpuzeez_core::cpu::limits::Limit| if l.enabled { "" } else { " (off)" };
            let tau = pl1.seconds.map_or(String::new(), |s| format!(", τ {s:.0} s"));
            limits.push(("PL1 (long-term)", format!("{:.0} W{tau}{}", pl1.watts, off(&pl1))));
            limits.push(("PL2 (short-term)", format!("{:.0} W{}", pl2.watts, off(&pl2))));
        }
        if !limits.is_empty() {
            boxed(ui, "Power limits", &limits);
        }

        let vulns: Vec<(String, String)> = cpuzeez_platform::cpu_os::vulnerabilities();
        if !vulns.is_empty() {
            let rows: Vec<(&str, String)> = vulns.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
            boxed(ui, "Security", &rows);
        }

        let v = cpuzeez_platform::cpu_os::virtualization();
        let yn = |b: Option<bool>| b.map(|b| tr(if b { "Yes" } else { "No" }).to_string()).unwrap_or_default();
        let hw = c.instructions.iter().any(|i| {
            let u = i.to_ascii_uppercase();
            u.starts_with("VT-X") || u.starts_with("AMD-V")
        });
        boxed(
            ui,
            "Virtualization",
            &[
                ("Supported by the CPU", tr(if hw { "Yes" } else { "No" }).to_string()),
                ("Enabled in the firmware", yn(v.firmware_enabled)),
                ("Hypervisor", c.hypervisor.clone().unwrap_or_else(|| tr("None").to_string())),
                ("Virtualization-Based Security", yn(v.vbs)),
                ("Memory integrity (HVCI)", yn(v.hvci)),
                ("KVM available", yn(v.kvm)),
            ],
        );

        let pw = cpuzeez_platform::cpu_os::power();
        boxed(
            ui,
            "Power policy",
            &[
                ("Power plan", pw.plan.unwrap_or_default()),
                ("CPU frequency driver", pw.driver.unwrap_or_default()),
                ("Governor", pw.governor.unwrap_or_default()),
                ("Energy/performance preference", pw.epp.unwrap_or_default()),
            ],
        );

        instructions(ui, &c.instructions);
    }
}

/// Socket → CCD → CCX → cores, drawn as nested boxes.
fn topology(ui: &mut egui::Ui, c: &crate::cpu::Cpu) {
    use std::collections::BTreeMap;
    let mut tree: BTreeMap<u32, BTreeMap<Option<u32>, BTreeMap<Option<u32>, Vec<usize>>>> = BTreeMap::new();
    for (i, l) in c.logical.iter().filter(|l| l.smt == 0).enumerate() {
        tree.entry(l.package).or_default().entry(l.ccd).or_default().entry(l.ccx).or_default().push(i);
    }
    if tree.is_empty() {
        return;
    }
    let p = theme::palette(ui);
    let l3 = c.caches.iter().find(|x| x.level == 3).map(|x| crate::cpu::size_text(x.size));
    heading(ui, tr("Topology"));
    widgets::card(ui, "", "", |ui| {
        for (pkg, ccds) in &tree {
            ui.label(RichText::new(trf("Package {}", &[pkg])).strong().color(p.text));
            ui.horizontal_wrapped(|ui| {
                for (ccd, ccxs) in ccds {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| {
                            if let Some(d) = ccd {
                                ui.label(RichText::new(trf("CCD {}", &[d])).color(p.text_secondary));
                            }
                            for (ccx, cores) in ccxs {
                                egui::Frame::group(ui.style()).fill(p.card_hover).show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        let head = match (ccx, &l3) {
                                            (Some(x), Some(s)) => format!("{} · L3 {s}", trf("CCX {}", &[x])),
                                            (Some(x), None) => trf("CCX {}", &[x]),
                                            (None, Some(s)) => format!("L3 {s}"),
                                            (None, None) => String::new(),
                                        };
                                        if !head.is_empty() {
                                            ui.small(head);
                                        }
                                        ui.horizontal_wrapped(|ui| {
                                            ui.set_max_width(220.0);
                                            for core in cores {
                                                let kind = c
                                                    .logical
                                                    .iter()
                                                    .filter(|l| l.smt == 0)
                                                    .nth(*core)
                                                    .and_then(|l| l.kind.clone());
                                                let t = format!(
                                                    "{core}{}",
                                                    kind.map(|k| format!(" {}", &k[..1])).unwrap_or_default()
                                                );
                                                egui::Frame::new()
                                                    .fill(p.accent_soft)
                                                    .corner_radius(egui::CornerRadius::same(4))
                                                    .inner_margin(egui::Margin::symmetric(6, 2))
                                                    .show(ui, |ui| {
                                                        ui.label(RichText::new(t).monospace().color(p.text))
                                                    });
                                            }
                                        });
                                    });
                                });
                            }
                        });
                    });
                }
            });
        }
    });
    ui.add_space(theme::GAP);
}

/// Instruction sets grouped by what they're for; hover explains the group.
fn instructions(ui: &mut egui::Ui, all: &[String]) {
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<&str, (&str, Vec<&str>)> = BTreeMap::new();
    for i in all {
        let (g, why) = group_of(i);
        groups.entry(g).or_insert((why, Vec::new())).1.push(i);
    }
    let p = theme::palette(ui);
    heading(ui, tr("Instruction sets"));
    widgets::card(ui, "", "", |ui| {
        for (g, (why, items)) in &groups {
            let r = ui.label(RichText::new(tr(g)).strong().color(p.text));
            if !why.is_empty() {
                r.on_hover_text(*why);
            }
            ui.label(RichText::new(items.join(", ")).color(p.text_secondary));
            ui.add_space(4.0);
        }
    });
    ui.add_space(theme::GAP);
}

#[cfg(test)]
mod tests {
    #[test]
    fn instruction_groups() {
        assert_eq!(super::group_of("AVX512 (DQ, BW)").0, "SIMD (vector math)");
        assert_eq!(super::group_of("AES").0, "Cryptography");
        assert_eq!(super::group_of("AMD-V").0, "Virtualization");
        assert_eq!(super::group_of("x86-64").0, "Other");
    }
}
