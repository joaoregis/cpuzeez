//! Overview page: the system at a glance — identity, live temperatures of CPU,
//! GPU and chipset with sparklines, health status, and one card per component.

use super::i18n::tr;
use super::theme::{self, icon};
use super::widgets::{self, Tone};
use crate::Ctx;
use crate::report::{self, Section};
use crate::telemetry::{Series, Telemetry, ThrottleKind, Unit};
use eframe::egui::{self, RichText, Ui};

pub const PT_BR: &[(&str, &str)] = &[
    ("Overview", "Visão geral"),
    ("All good", "Tudo normal"),
    ("Thermal throttling", "Limitação térmica"),
    ("Power limit", "Limite de potência"),
    ("Current limit", "Limite de corrente"),
    ("GPU throttling", "GPU limitando"),
    ("CPU temperature", "Temperatura da CPU"),
    ("CPU clock", "Clock da CPU"),
    ("GPU temperature", "Temperatura da GPU"),
    ("Chipset temperature", "Temperatura do chipset"),
    ("Processor", "Processador"),
    ("Mainboard", "Placa-mãe"),
    ("Memory", "Memória"),
    ("Graphics", "Gráficos"),
    ("Storage", "Armazenamento"),
    ("System", "Sistema"),
    ("Technology", "Tecnologia"),
    ("Cores / threads", "Núcleos / threads"),
    ("Power", "Potência"),
    ("Model", "Modelo"),
    ("Chipset", "Chipset"),
    ("Type", "Tipo"),
    ("Frequency", "Frequência"),
    ("Timings", "Timings"),
    ("Profile", "Perfil"),
    ("Operating system", "Sistema operacional"),
    ("Hardware access", "Acesso ao hardware"),
    ("cores", "núcleos"),
    ("Reading hardware…", "Lendo o hardware…"),
];

pub const ES: &[(&str, &str)] = &[
    ("Overview", "Resumen"),
    ("All good", "Todo normal"),
    ("Thermal throttling", "Limitación térmica"),
    ("Power limit", "Límite de potencia"),
    ("Current limit", "Límite de corriente"),
    ("GPU throttling", "GPU limitando"),
    ("CPU temperature", "Temperatura de la CPU"),
    ("CPU clock", "Reloj de la CPU"),
    ("GPU temperature", "Temperatura de la GPU"),
    ("Chipset temperature", "Temperatura del chipset"),
    ("Processor", "Procesador"),
    ("Mainboard", "Placa base"),
    ("Memory", "Memoria"),
    ("Graphics", "Gráficos"),
    ("Storage", "Almacenamiento"),
    ("System", "Sistema"),
    ("Technology", "Tecnología"),
    ("Cores / threads", "Núcleos / hilos"),
    ("Power", "Potencia"),
    ("Model", "Modelo"),
    ("Chipset", "Chipset"),
    ("Type", "Tipo"),
    ("Frequency", "Frecuencia"),
    ("Timings", "Latencias"),
    ("Profile", "Perfil"),
    ("Operating system", "Sistema operativo"),
    ("Hardware access", "Acceso al hardware"),
    ("cores", "núcleos"),
    ("Reading hardware…", "Leyendo el hardware…"),
];

/// First series in `group` (prefix) whose label contains any of `needles` (case-insensitive).
fn find<'a>(all: &'a [Series], group: &str, unit: Unit, needles: &[&str]) -> Option<&'a Series> {
    let hit = |s: &&Series| {
        let l = s.label.to_lowercase();
        s.group.starts_with(group) && s.unit == unit && (needles.is_empty() || needles.iter().any(|n| l.contains(n)))
    };
    all.iter().find(hit)
}

fn spark(s: Option<&Series>) -> Vec<f64> {
    s.map(|s| s.points.iter().rev().take(60).rev().map(|p| p.1).collect()).unwrap_or_default()
}

fn temp_tone(c: f64, limit: f64) -> Tone {
    if c >= limit {
        Tone::Danger
    } else if c >= limit - 10.0 {
        Tone::Warning
    } else {
        Tone::Neutral
    }
}

fn sget(r: &[Section], s: &str, k: &str) -> String {
    report::find_section(r, s).and_then(|x| x.get(k)).unwrap_or_default().to_string()
}
fn sgi(r: &[Section], s: &str, g: &str, k: &str) -> String {
    report::find_section(r, s).and_then(|x| x.get_in(g, k)).unwrap_or_default().to_string()
}

pub fn page(ui: &mut Ui, ctx: &Ctx, r: &[Section], tel: &Telemetry, core_mhz: Option<f64>) {
    let p = theme::palette(ui);
    let c = crate::cpu::get(ctx);
    let series = tel.series();
    let board = format!("{} {}", sgi(r, "DMI", "DMI Baseboard", "vendor"), sgi(r, "DMI", "DMI Baseboard", "model"));
    let mut sub = vec![];
    if let Some(cn) = &c.info.codename {
        sub.push(cn.clone());
    }
    if let Some(pk) = &c.info.package {
        sub.push(pk.trim_start_matches("Socket ").to_string());
    }
    sub.push(format!("{} {} / {} threads", c.cores, tr("cores"), c.threads));
    if !board.trim().is_empty() {
        sub.push(board.trim().to_string());
    }

    let throttles = tel.throttling();
    widgets::page_header(ui, &c.name, &sub.join(" · "), |ui| {
        if throttles.is_empty() {
            widgets::pill(ui, &format!("{}  {}", icon::CHECK_CIRCLE, tr("All good")), Tone::Success);
        } else {
            for t in &throttles {
                let what = match t.kind {
                    ThrottleKind::Thermal => tr("Thermal throttling"),
                    ThrottleKind::Power => tr("Power limit"),
                    ThrottleKind::Current => tr("Current limit"),
                    ThrottleKind::Gpu => tr("GPU throttling"),
                };
                widgets::pill(ui, &format!("{}  {what} · {}", icon::WARNING, t.device), Tone::Warning)
                    .on_hover_text(&t.detail);
            }
        }
    });

    // Live tiles: CPU temp, CPU clock, GPU temp, chipset temp.
    let tj = c.info.tjmax_c.unwrap_or(95.0);
    let cpu_t = find(&series, "CPU", Unit::Celsius, &["package", "tctl", "cpu"]);
    let gpu_t = find(&series, "GPU", Unit::Celsius, &["core", "gpu", "edge", "temperature"]);
    let chip_t = find(&series, "Chipset", Unit::Celsius, &[]);
    let clk = find(&series, "CPU", Unit::Mhz, &["core #0", "core 0", "clock"]);
    let tiles: Vec<(&str, String, Vec<f64>, egui::Color32, Tone)> = vec![
        (
            tr("CPU temperature"),
            cpu_t.map_or("–".into(), |s| format!("{:.0} °C", s.current)),
            spark(cpu_t),
            p.series[0],
            cpu_t.map_or(Tone::Neutral, |s| temp_tone(s.current, tj)),
        ),
        (
            tr("CPU clock"),
            core_mhz.or(clk.map(|s| s.current)).map_or("–".into(), |m| format!("{:.2} GHz", m / 1000.0)),
            spark(clk),
            p.series[0],
            Tone::Neutral,
        ),
        (
            tr("GPU temperature"),
            gpu_t.map_or("–".into(), |s| format!("{:.0} °C", s.current)),
            spark(gpu_t),
            p.series[1],
            gpu_t.map_or(Tone::Neutral, |s| temp_tone(s.current, 90.0)),
        ),
        (
            tr("Chipset temperature"),
            chip_t.map_or("–".into(), |s| format!("{:.0} °C", s.current)),
            spark(chip_t),
            p.series[2],
            chip_t.map_or(Tone::Neutral, |s| temp_tone(s.current, 90.0)),
        ),
    ];
    widgets::columns(ui, 4, 150.0, tiles.len(), |ui, i| {
        let (l, v, sp, col, tone) = &tiles[i];
        widgets::metric(ui, l, v, sp, *col, *tone);
    });

    // Component cards.
    let pkg_w = find(&series, "CPU", Unit::Watt, &["package"]);
    let l3 = c.caches.iter().find(|x| x.level == 3).map(|x| format!("{} × {}", x.count, crate::cpu::size_text(x.size)));
    let chipset = |k: &str| sget(r, "Chipset", k);
    let spd = report::find_section(r, "Memory SPD");
    let profile = spd
        .and_then(|s| {
            ["EXPO profile", "XMP profile"].iter().find_map(|g| {
                report::groups(&s.nodes, g).first().map(|(v, _)| format!("{} {}", g.split(' ').next().unwrap_or(""), v))
            })
        })
        .unwrap_or_default();
    let gpus: Vec<String> = report::find_section(r, "Display Adapters")
        .map(|s| {
            report::groups(&s.nodes, "Display adapter")
                .iter()
                .map(|(_, n)| {
                    let name = report::find(n, "Name").unwrap_or("GPU");
                    let mem = report::find(n, "Memory size").map(|m| format!(" · {m}")).unwrap_or_default();
                    format!("{name}{mem}")
                })
                .collect()
        })
        .unwrap_or_default();
    let drives: Vec<(String, String)> = crate::storage::drives(ctx)
        .iter()
        .map(|d| (d.name.clone(), format!("{:.0} GB", d.capacity as f64 / 1e9)))
        .collect();

    let cards: Vec<(&str, &str, Vec<(&str, String)>)> = vec![
        (
            icon::CPU,
            tr("Processor"),
            vec![
                (tr("Technology"), c.info.technology.clone().unwrap_or_default()),
                ("TDP", c.info.tdp_w.map(|w| format!("{w:.0} W")).unwrap_or_default()),
                (tr("Cores / threads"), format!("{} / {}", c.cores, c.threads)),
                ("L3", l3.unwrap_or_default()),
                (tr("Power"), pkg_w.map(|s| format!("{:.1} W", s.current)).unwrap_or_default()),
            ],
        ),
        (
            icon::CIRCUITRY,
            tr("Mainboard"),
            vec![
                (tr("Model"), board.trim().to_string()),
                (tr("Chipset"), chipset("Southbridge")),
                ("BIOS", format!("{} · {}", sgi(r, "DMI", "DMI BIOS", "version"), sgi(r, "DMI", "DMI BIOS", "date"))),
            ],
        ),
        (
            icon::MEMORY,
            tr("Memory"),
            vec![
                (
                    tr("Type"),
                    format!("{} · {} · {}", chipset("Memory Type"), chipset("Memory Size"), chipset("Channels")),
                ),
                (tr("Frequency"), chipset("Memory Frequency")),
                (
                    tr("Timings"),
                    [
                        chipset("CAS# latency (CL)"),
                        chipset("RAS# to CAS# delay (tRCD)"),
                        chipset("RAS# Precharge (tRP)"),
                        chipset("Cycle Time (tRAS)"),
                    ]
                    .iter()
                    .filter(|s| !s.is_empty())
                    .map(|s| s.trim_end_matches(".0").to_string())
                    .collect::<Vec<_>>()
                    .join("-"),
                ),
                (tr("Profile"), profile),
            ],
        ),
        (icon::GRAPHICS_CARD, tr("Graphics"), gpus.iter().map(|g| ("GPU", g.clone())).collect()),
        (icon::HARD_DRIVES, tr("Storage"), drives.iter().map(|(n, s)| (n.as_str(), s.clone())).collect()),
        (
            icon::DESKTOP,
            tr("System"),
            vec![(tr("Operating system"), crate::software::os_line(ctx)), (tr("Hardware access"), ctx.hw.name())],
        ),
    ];
    widgets::columns(ui, 2, 300.0, cards.len(), |ui, i| {
        let (ic, title, rows) = &cards[i];
        widgets::card(ui, ic, title, |ui| widgets::kv_list(ui, rows));
    });
    let _ = RichText::new("");
}
