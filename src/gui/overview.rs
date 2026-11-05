//! Overview page: the system at a glance — identity, live temperatures of CPU,
//! GPU and chipset with sparklines, health status, and one card per component.

use super::i18n::tr;
use super::theme::{self, icon};
use super::widgets::{self, Tone};
use crate::Ctx;
use crate::report::{self, Section};
use crate::settings::Pin;
use crate::telemetry::{Series, Telemetry, ThrottleKind, Unit};
use cpuzeez_core::layout::{Widget, presets};
use eframe::egui::{self, RichText, Ui};

pub const PT_BR: &[(&str, &str)] = &[
    ("Resizable BAR is off for {}", "Resizable BAR está desligado para {}"),
    (
        "With Resizable BAR (Smart Access Memory on AMD) the CPU can reach all of the graphics memory at once, which helps many games. Turn on \"Above 4G Decoding\" and \"Re-Size BAR Support\" in the BIOS; it needs UEFI boot (CSM off).",
        "Com o Resizable BAR (Smart Access Memory na AMD) a CPU alcança toda a memória de vídeo de uma vez, o que ajuda muitos jogos. Ative \"Above 4G Decoding\" e \"Re-Size BAR Support\" na BIOS; requer boot UEFI (CSM desligado).",
    ),
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
    ("Done", "Concluir"),
    ("Customize", "Personalizar"),
    ("Open in Sensors", "Abrir em Sensores"),
    ("Build OK", "Montagem OK"),
    ("Build: {} to check", "Montagem: {} para verificar"),
    ("Drive needs attention: {}", "Disco precisa de atenção: {}"),
    ("Drives OK", "Discos OK"),
    ("{}: {}% of its design capacity", "{}: {}% da capacidade original"),
    ("Build health", "Saúde da montagem"),
    ("Customize the Overview", "Personalizar a Visão geral"),
    ("Presets", "Predefinições"),
    ("Move up", "Mover para cima"),
    ("Move down", "Mover para baixo"),
    ("Remove", "Remover"),
    ("Pick a sensor", "Escolha um sensor"),
    ("Add tile", "Adicionar bloco"),
    ("Restore defaults", "Restaurar padrão"),
    ("Default", "Padrão"),
    ("Gamer", "Gamer"),
    ("Overclocker", "Overclocker"),
    ("Laptop", "Notebook"),
    ("Workstation", "Estação de trabalho"),
    ("CPU load", "Carga da CPU"),
    ("GPU load", "Carga da GPU"),
    ("CPU power", "Potência da CPU"),
    ("CPU voltage", "Tensão da CPU"),
    ("Battery", "Bateria"),
    ("Charge rate", "Taxa de carga"),
    ("Memory load", "Uso da memória"),
    ("Disk active time", "Tempo ativo do disco"),
    ("Download", "Download"),
    ("Stress test", "Teste de estresse"),
    ("Save report", "Salvar relatório"),
    ("Sensor log", "Log de sensores"),
    ("Mini monitor", "Mini monitor"),
    ("One memory module: single channel", "Um módulo de memória: canal único"),
    (
        "The CPU gets half the memory bandwidth it could. Two matching modules (in the slots the manual names for dual channel) fix it.",
        "A CPU recebe metade da largura de banda de memória possível. Dois módulos iguais (nos slots que o manual indica para dual channel) resolvem.",
    ),
    (
        "Memory runs at {} MT/s; the modules support {} MT/s ({})",
        "A memória roda a {} MT/s; os módulos suportam {} MT/s ({})",
    ),
    (
        "The memory profile isn't enabled. Turn on {} in the BIOS (often called XMP, EXPO or DOCP).",
        "O perfil de memória não está ativado. Ative {} na BIOS (costuma se chamar XMP, EXPO ou DOCP).",
    ),
    ("{} runs on x{} lanes (it supports x{})", "{} usa x{} linhas (suporta x{})"),
    (
        "Usually the wrong slot, a riser, or a lane-sharing BIOS setting (M.2 drives can take lanes from the GPU slot).",
        "Geralmente é o slot errado, um riser ou uma configuração da BIOS que divide linhas (SSDs M.2 podem tirar linhas do slot da placa de vídeo).",
    ),
    ("Power saving plan active", "Plano de economia de energia ativo"),
    (
        "The CPU is held back to save energy. Pick Balanced or High performance in the power settings.",
        "A CPU é contida para economizar energia. Escolha Equilibrado ou Alto desempenho nas opções de energia.",
    ),
    ("{} runs at {} Hz; it supports {} Hz", "{} está a {} Hz; suporta {} Hz"),
    (
        "Choose the higher refresh rate in the display settings (and check the cable supports it).",
        "Escolha a taxa de atualização maior nas configurações de vídeo (e confira se o cabo suporta).",
    ),
    ("Monitor plugged into the motherboard", "Monitor ligado na placa-mãe"),
    (
        "A graphics card is installed, but the monitor is connected to the motherboard's video output, so games render on the integrated GPU. Plug it into the graphics card.",
        "Há uma placa de vídeo instalada, mas o monitor está ligado na saída de vídeo da placa-mãe, então os jogos rodam na GPU integrada. Ligue-o na placa de vídeo.",
    ),
];

pub const ES: &[(&str, &str)] = &[
    ("Resizable BAR is off for {}", "Resizable BAR está desactivado para {}"),
    (
        "With Resizable BAR (Smart Access Memory on AMD) the CPU can reach all of the graphics memory at once, which helps many games. Turn on \"Above 4G Decoding\" and \"Re-Size BAR Support\" in the BIOS; it needs UEFI boot (CSM off).",
        "Con Resizable BAR (Smart Access Memory en AMD) la CPU alcanza toda la memoria de vídeo a la vez, lo que ayuda a muchos juegos. Active \"Above 4G Decoding\" y \"Re-Size BAR Support\" en la BIOS; requiere arranque UEFI (CSM desactivado).",
    ),
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
    ("Done", "Listo"),
    ("Customize", "Personalizar"),
    ("Open in Sensors", "Abrir en Sensores"),
    ("Build OK", "Equipo OK"),
    ("Build: {} to check", "Equipo: {} por revisar"),
    ("Drive needs attention: {}", "Disco requiere atención: {}"),
    ("Drives OK", "Discos OK"),
    ("{}: {}% of its design capacity", "{}: {}% de su capacidad original"),
    ("Build health", "Salud del equipo"),
    ("Customize the Overview", "Personalizar el Resumen"),
    ("Presets", "Preajustes"),
    ("Move up", "Subir"),
    ("Move down", "Bajar"),
    ("Remove", "Quitar"),
    ("Pick a sensor", "Elige un sensor"),
    ("Add tile", "Añadir mosaico"),
    ("Restore defaults", "Restaurar valores predeterminados"),
    ("Default", "Predeterminado"),
    ("Gamer", "Gamer"),
    ("Overclocker", "Overclocker"),
    ("Laptop", "Portátil"),
    ("Workstation", "Estación de trabajo"),
    ("CPU load", "Carga de la CPU"),
    ("GPU load", "Carga de la GPU"),
    ("CPU power", "Potencia de la CPU"),
    ("CPU voltage", "Voltaje de la CPU"),
    ("Battery", "Batería"),
    ("Charge rate", "Ritmo de carga"),
    ("Memory load", "Uso de memoria"),
    ("Disk active time", "Tiempo activo del disco"),
    ("Download", "Descarga"),
    ("Stress test", "Prueba de estrés"),
    ("Save report", "Guardar informe"),
    ("Sensor log", "Registro de sensores"),
    ("Mini monitor", "Mini monitor"),
    ("One memory module: single channel", "Un módulo de memoria: un solo canal"),
    (
        "The CPU gets half the memory bandwidth it could. Two matching modules (in the slots the manual names for dual channel) fix it.",
        "La CPU recibe la mitad del ancho de banda de memoria posible. Dos módulos iguales (en las ranuras que el manual indica para doble canal) lo resuelven.",
    ),
    (
        "Memory runs at {} MT/s; the modules support {} MT/s ({})",
        "La memoria funciona a {} MT/s; los módulos admiten {} MT/s ({})",
    ),
    (
        "The memory profile isn't enabled. Turn on {} in the BIOS (often called XMP, EXPO or DOCP).",
        "El perfil de memoria no está activado. Activa {} en la BIOS (suele llamarse XMP, EXPO o DOCP).",
    ),
    ("{} runs on x{} lanes (it supports x{})", "{} usa x{} líneas (admite x{})"),
    (
        "Usually the wrong slot, a riser, or a lane-sharing BIOS setting (M.2 drives can take lanes from the GPU slot).",
        "Suele ser la ranura equivocada, un riser o un ajuste de la BIOS que reparte líneas (las unidades M.2 pueden quitar líneas a la ranura de la GPU).",
    ),
    ("Power saving plan active", "Plan de ahorro de energía activo"),
    (
        "The CPU is held back to save energy. Pick Balanced or High performance in the power settings.",
        "La CPU se limita para ahorrar energía. Elige Equilibrado o Alto rendimiento en las opciones de energía.",
    ),
    ("{} runs at {} Hz; it supports {} Hz", "{} funciona a {} Hz; admite {} Hz"),
    (
        "Choose the higher refresh rate in the display settings (and check the cable supports it).",
        "Elige la frecuencia de actualización más alta en la configuración de pantalla (y comprueba que el cable la admite).",
    ),
    ("Monitor plugged into the motherboard", "Monitor conectado a la placa base"),
    (
        "A graphics card is installed, but the monitor is connected to the motherboard's video output, so games render on the integrated GPU. Plug it into the graphics card.",
        "Hay una tarjeta gráfica instalada, pero el monitor está conectado a la salida de vídeo de la placa base, así que los juegos usan la GPU integrada. Conéctalo a la tarjeta gráfica.",
    ),
];

/// First series in `group` (prefix) whose label contains any of `needles` (case-insensitive).
fn find<'a>(all: &'a [Series], group: &str, unit: Unit, needles: &[&str]) -> Option<&'a Series> {
    let hit = |s: &&Series| {
        let l = s.label.to_lowercase();
        s.group.starts_with(group) && s.unit == unit && (needles.is_empty() || needles.iter().any(|n| l.contains(n)))
    };
    all.iter().find(hit)
}

/// "a · b · c" from the non-empty parts.
fn join(parts: &[String]) -> String {
    parts.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ")
}

fn sget(r: &[Section], s: &str, k: &str) -> String {
    report::find_section(r, s).and_then(|x| x.get(k)).unwrap_or_default().to_string()
}
fn sgi(r: &[Section], s: &str, g: &str, k: &str) -> String {
    report::find_section(r, s).and_then(|x| x.get_in(g, k)).unwrap_or_default().to_string()
}

/// Where a click on the Overview leads (handled by the app, which owns the pages).
pub(super) enum Nav {
    Page(super::Page),
    /// The Sensors page, filtered (a tile's metric).
    Sensors(String),
    Act(super::palette::Act),
    /// Settings changed here (tiles, pins): save them.
    Save,
}

/// What the Overview keeps between frames.
#[derive(Default)]
pub(super) struct State {
    customize: bool,
    health_open: bool,
    /// Build health, computed once per report (it reads the OS display list).
    findings: Option<Vec<cpuzeez_core::health::Finding>>,
    /// The metric picked in Customize's "Add tile" list.
    add: Option<String>,
}

/// A tile's value with the precision tiles use: whole degrees, GHz clocks.
fn tile_text(w: &Widget, s: &Series) -> String {
    if w.decimals.is_some() {
        return super::layout_view::value_text(w, s);
    }
    match s.unit {
        Unit::Celsius => super::units::temp(s.current, 0),
        Unit::Mhz => super::units::clock(s.current, true, 2),
        _ => super::sensors::fmt(s.current, s.unit),
    }
}

fn tone_of(t: cpuzeez_core::layout::Tone) -> Tone {
    use cpuzeez_core::layout::Tone as L;
    match t {
        L::Danger => Tone::Danger,
        L::Warning => Tone::Warning,
        L::Good => Tone::Success,
        L::Normal => Tone::Neutral,
    }
}

pub(super) fn page(
    ui: &mut Ui,
    ctx: &Ctx,
    r: &[Section],
    mon: &mut super::sensors::Monitor,
    st: &mut State,
) -> Vec<Nav> {
    let tel = &mon.tel();
    let core_mhz = crate::telemetry::core_mhz(&tel.snapshot(), 0);
    let alerts = mon.alerts().to_vec();
    let s = &mut mon.settings;
    let mut nav = Vec::new();
    let p = theme::palette(ui);
    let c = crate::cpu::get(ctx);
    let tj = c.info.tjmax_c.unwrap_or(95.0);
    let tiles = s.overview_tiles.clone().unwrap_or_else(|| presets::overview("Default", tj));
    let mut lease: Vec<String> = super::layout_view::prefixes(&tiles);
    lease.extend(s.pins.iter().filter_map(|p| p.metric.clone()));
    tel.want("overview", &lease.iter().map(String::as_str).collect::<Vec<_>>());
    let series = tel.series_with(|_| false);
    let board = format!("{} {}", sgi(r, "DMI", "DMI Baseboard", "vendor"), sgi(r, "DMI", "DMI Baseboard", "model"));

    let throttles = tel.throttling();
    let mut customize = st.customize;
    widgets::page_header(ui, &c.name, "", |ui| {
        let label = if customize { tr("Done") } else { tr("Customize") };
        if ui.button(format!("{}  {label}", icon::PENCIL_SIMPLE)).clicked() {
            customize = !customize;
        }
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
    st.customize = customize;
    if !alerts.is_empty() {
        let p = theme::palette(ui);
        egui::Frame::default()
            .fill(p.danger.gamma_multiply(0.15))
            .stroke(egui::Stroke::new(1.0, p.danger))
            .corner_radius(egui::CornerRadius::same(theme::RADIUS_CARD))
            .inner_margin(egui::Margin::same(theme::PAD_CARD))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                for a in &alerts {
                    ui.label(RichText::new(format!("{}  {a}", icon::WARNING_OCTAGON)).color(p.text));
                }
            });
        ui.add_space(theme::GAP);
    }
    identity(ui, ctx, r, &board, &mut nav);
    health(ui, ctx, r, st, &mut nav);
    if st.customize {
        editor(ui, s, st, &series, tj, &mut nav);
    }

    // Live tiles (Widget Engine); each one opens Sensors on its metric.
    let shown: Vec<(&Widget, Option<&Series>)> =
        tiles.iter().map(|w| (w, w.metric.as_deref().and_then(|m| super::layout_view::resolve(m, &series)))).collect();
    widgets::columns(ui, 4, 150.0, shown.len(), |ui, i| {
        let (w, sr) = shown[i];
        let label = tr(w.label.as_deref().unwrap_or("")).to_string();
        // The pre-1.0 clock tile shows core #0's live APERF/MPERF clock.
        let value = match sr {
            Some(_) if w.metric.as_deref() == Some("@cpu_clock") && core_mhz.is_some() => {
                super::units::clock(core_mhz.unwrap_or_default(), true, 2)
            }
            Some(x) => tile_text(w, x),
            None => "–".into(),
        };
        let pts = sr.map(|x| tel.points(&x.id)).unwrap_or_default();
        let spark: Vec<f64> = pts.iter().rev().take(60).rev().map(|p| p.1).collect();
        let tone = sr.map_or(Tone::Neutral, |x| tone_of(w.tone(x.current)));
        let color = p.series[i % p.series.len()];
        let r = ui
            .scope(|ui| widgets::metric(ui, &label, &value, &spark, color, tone))
            .response
            .interact(egui::Sense::click());
        if let Some(x) = sr
            && r.on_hover_text(tr("Open in Sensors")).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
        {
            nav.push(Nav::Sensors(format!("id:{}", x.id)));
        }
    });
    quick_actions(ui, &mut nav);
    if pinned(ui, tel, &mut s.pins) {
        nav.push(Nav::Save);
    }

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
                ("BIOS", join(&[sgi(r, "DMI", "DMI BIOS", "version"), sgi(r, "DMI", "DMI BIOS", "date")])),
            ],
        ),
        (
            icon::MEMORY,
            tr("Memory"),
            vec![
                (tr("Type"), join(&[chipset("Memory Type"), chipset("Memory Size"), chipset("Channels")])),
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
    nav
}

/// One dense, clickable line: CPU · GPU · board · memory · storage · OS.
fn identity(ui: &mut Ui, ctx: &Ctx, r: &[Section], board: &str, nav: &mut Vec<Nav>) {
    use super::Page;
    let c = crate::cpu::get(ctx);
    let all = crate::gpu::adapters(ctx);
    let discrete: Vec<&crate::gpu::Adapter> = all.iter().filter(|a| !crate::health::integrated(a)).collect();
    let gpus: Vec<String> = (if discrete.is_empty() { all.iter().collect() } else { discrete })
        .iter()
        .map(|a| match a.mem_size_mb {
            Some(m) if m >= 1024 => format!("{} {:.0} GB", a.name, m as f64 / 1024.0),
            _ => a.name.clone(),
        })
        .collect();
    let ch = |k: &str| sget(r, "Chipset", k);
    let mts = ch("Memory Frequency").split(' ').next().and_then(|m| m.parse::<f64>().ok()).map(|m| m * 2.0);
    let mem = join(&[
        ch("Memory Size"),
        match (ch("Memory Type"), mts) {
            (t, Some(m)) if !t.is_empty() => format!("{t}-{m:.0}"),
            (t, _) => t,
        },
    ]);
    let total: u64 = crate::storage::drives(ctx).iter().filter(|d| !d.removable).map(|d| d.capacity).sum();
    let storage = match total {
        0 => String::new(),
        b if b >= 1_000_000_000_000 => format!("{:.1} TB", b as f64 / 1e12),
        b => format!("{:.0} GB", b as f64 / 1e9),
    };
    let items = [
        (c.name.clone(), Page::Processor),
        (gpus.join(" + "), Page::Graphics),
        (board.trim().to_string(), Page::Mainboard),
        (mem, Page::Memory),
        (storage, Page::Devices),
        (crate::software::os_line(ctx), Page::About),
    ];
    let p = theme::palette(ui);
    ui.horizontal_wrapped(|ui| {
        let mut first = true;
        for (text, page) in items.iter().filter(|(t, _)| !t.trim().is_empty()) {
            if !first {
                ui.label(RichText::new("·").color(p.text_muted));
            }
            first = false;
            let l = ui.add(egui::Label::new(RichText::new(text).color(p.text_secondary)).sense(egui::Sense::click()));
            if l.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                nav.push(Nav::Page(*page));
            }
        }
    });
    ui.add_space(theme::GAP / 2.0);
}

/// Health strip: build health (click for the list), drives, battery wear.
fn health(ui: &mut Ui, ctx: &Ctx, r: &[Section], st: &mut State, nav: &mut Vec<Nav>) {
    use crate::storage::HealthLevel;
    if st.findings.is_none() && !r.is_empty() {
        st.findings = Some(crate::health::check(ctx, r));
    }
    let findings = st.findings.clone().unwrap_or_default();
    let drives = crate::storage::drives(ctx);
    let worst = drives.iter().filter_map(|d| d.smart.as_ref().map(|s| (d, s.health))).max_by_key(|(_, h)| match h {
        HealthLevel::Bad => 3,
        HealthLevel::Caution => 2,
        HealthLevel::Good => 1,
        HealthLevel::Unknown => 0,
    });
    let worn: Vec<(String, f64)> = crate::battery::batteries(ctx)
        .iter()
        .filter_map(|b| b.health_pct.filter(|h| *h < 80.0).map(|h| (b.name.clone(), h)))
        .collect();
    ui.horizontal_wrapped(|ui| {
        let build = if findings.is_empty() {
            widgets::pill(ui, &format!("{}  {}", icon::CHECK_CIRCLE, tr("Build OK")), Tone::Success)
        } else {
            let t = super::i18n::trf("Build: {} to check", &[&findings.len()]);
            widgets::pill(ui, &format!("{}  {t}", icon::WRENCH), Tone::Warning)
        };
        if !findings.is_empty()
            && build.interact(egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
        {
            st.health_open = !st.health_open;
        }
        match worst {
            Some((d, HealthLevel::Bad | HealthLevel::Caution)) => {
                let t = super::i18n::trf("Drive needs attention: {}", &[&d.name]);
                if widgets::pill(ui, &format!("{}  {t}", icon::HARD_DRIVES), Tone::Danger)
                    .interact(egui::Sense::click())
                    .clicked()
                {
                    nav.push(Nav::Page(super::Page::Devices));
                }
            }
            Some(_) => {
                widgets::pill(ui, &format!("{}  {}", icon::HARD_DRIVES, tr("Drives OK")), Tone::Success);
            }
            None => {}
        }
        for (name, h) in &worn {
            let t = super::i18n::trf("{}: {}% of its design capacity", &[name, &format!("{h:.0}")]);
            widgets::pill(ui, &format!("{}  {t}", icon::BATTERY_WARNING), Tone::Warning);
        }
    });
    if st.health_open && !findings.is_empty() {
        let p = theme::palette(ui);
        ui.add_space(theme::GAP / 2.0);
        widgets::card(ui, icon::WRENCH, tr("Build health"), |ui| {
            for f in &findings {
                let args: Vec<&dyn std::fmt::Display> = f.args.iter().map(|a| a as &dyn std::fmt::Display).collect();
                ui.label(RichText::new(super::i18n::trf(f.title, &args)).strong().color(p.text));
                ui.label(RichText::new(super::i18n::trf(f.detail, &args)).color(p.text_secondary));
                ui.add_space(4.0);
            }
        });
    }
    ui.add_space(theme::GAP / 2.0);
}

/// Customize mode: presets, order, add and remove tiles, restore the defaults.
fn editor(
    ui: &mut Ui,
    s: &mut crate::settings::Settings,
    st: &mut State,
    series: &[Series],
    tj: f64,
    nav: &mut Vec<Nav>,
) {
    let mut tiles = s.overview_tiles.clone().unwrap_or_else(|| presets::overview("Default", tj));
    let before = tiles.clone();
    let mut restored = false;
    widgets::card(ui, icon::PENCIL_SIMPLE, tr("Customize the Overview"), |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("Presets"));
            for name in presets::OVERVIEW {
                if ui.button(tr(name)).clicked() {
                    tiles = presets::overview(name, tj);
                }
            }
        });
        ui.add_space(4.0);
        let mut action: Option<(usize, i32)> = None;
        for (i, w) in tiles.iter().enumerate() {
            ui.horizontal(|ui| {
                if ui.small_button(icon::ARROW_UP).on_hover_text(tr("Move up")).clicked() {
                    action = Some((i, -1));
                }
                if ui.small_button(icon::ARROW_DOWN).on_hover_text(tr("Move down")).clicked() {
                    action = Some((i, 1));
                }
                if ui.small_button(icon::X).on_hover_text(tr("Remove")).clicked() {
                    action = Some((i, 0));
                }
                ui.label(tr(w.label.as_deref().unwrap_or("")));
                ui.weak(w.metric.as_deref().unwrap_or(""));
            });
        }
        match action {
            Some((i, 0)) => {
                tiles.remove(i);
            }
            Some((i, d)) => {
                let j = i as i32 + d;
                if (0..tiles.len() as i32).contains(&j) {
                    tiles.swap(i, j as usize);
                }
            }
            None => {}
        }
        ui.horizontal(|ui| {
            let name = |id: &str| {
                series.iter().find(|x| x.id == id).map_or(id.to_string(), |x| format!("{} · {}", x.group, x.label))
            };
            egui::ComboBox::from_id_salt("add-tile")
                .width(320.0)
                .selected_text(st.add.as_deref().map(name).unwrap_or_else(|| tr("Pick a sensor").to_string()))
                .show_ui(ui, |ui| {
                    for x in series {
                        ui.selectable_value(&mut st.add, Some(x.id.clone()), format!("{} · {}", x.group, x.label));
                    }
                });
            if ui.add_enabled(st.add.is_some() && tiles.len() < 8, egui::Button::new(tr("Add tile"))).clicked()
                && let Some(id) = st.add.take()
            {
                let label = series.iter().find(|x| x.id == id).map_or(id.clone(), |x| x.label.clone());
                tiles.push(Widget::metric(cpuzeez_core::layout::Kind::Sparkline, &id, &label));
            }
            if ui.button(tr("Restore defaults")).clicked() {
                restored = true;
            }
        });
    });
    ui.add_space(theme::GAP);
    if restored {
        s.overview_tiles = None; // follows future changes to the default set
        nav.push(Nav::Save);
    } else if tiles != before {
        s.overview_tiles = Some(tiles);
        nav.push(Nav::Save);
    }
}

fn quick_actions(ui: &mut Ui, nav: &mut Vec<Nav>) {
    use super::palette::Act;
    ui.add_space(theme::GAP / 2.0);
    ui.horizontal_wrapped(|ui| {
        for (act, ic, label) in [
            (Act::Stress, icon::FIRE, "Stress test"),
            (Act::SaveHtml, icon::FILE_TEXT, "Save report"),
            (Act::ToggleCsv, icon::RECORD, "Sensor log"),
            (Act::ToggleMini, icon::PICTURE_IN_PICTURE, "Mini monitor"),
        ] {
            if ui.button(format!("{ic}  {}", tr(label))).clicked() {
                nav.push(Nav::Act(act));
            }
        }
    });
}

/// The rows pinned from other pages (S1-U03): sensors live, other rows as
/// their page last drew them. Returns true if one was unpinned.
fn pinned(ui: &mut Ui, tel: &Telemetry, pins: &mut Vec<Pin>) -> bool {
    if pins.is_empty() {
        return false;
    }
    let ids: Vec<&str> = pins.iter().filter_map(|p| p.metric.as_deref()).collect();
    if !ids.is_empty() {
        tel.want("overview-pins", &ids);
    }
    let snap = tel.snapshot();
    let p = theme::palette(ui);
    let mut remove = None;
    ui.add_space(theme::GAP);
    widgets::card(ui, icon::PUSH_PIN, tr("Pinned"), |ui| {
        for (i, pin) in pins.iter().enumerate() {
            let value = match &pin.metric {
                Some(id) => snap.get(id).map(|s| super::sensors::fmt(s.current, s.unit)).unwrap_or_else(|| "–".into()),
                None => super::pins::latest(ui.ctx(), &pin.card, &pin.label).unwrap_or_else(|| pin.value.clone()),
            };
            ui.horizontal(|ui| {
                if ui.small_button(icon::X).on_hover_text(tr("Unpin")).clicked() {
                    remove = Some(i);
                }
                ui.label(RichText::new(format!("{} · {}", pin.card, pin.label)).color(p.text_secondary));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(value).color(p.text));
                });
            });
        }
    });
    if let Some(i) = remove {
        pins.remove(i);
    }
    remove.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};

    type St = (crate::settings::Settings, State, Vec<Nav>);

    /// S1-G01: Customize mode applies a preset, and Restore defaults goes back.
    #[test]
    fn customize_presets_and_restore() {
        let mut h = Harness::builder().with_size(egui::vec2(900.0, 700.0)).build_ui_state(
            |ui, st: &mut St| {
                if !ui.style().text_styles.contains_key(&egui::TextStyle::Name("title".into())) {
                    super::super::theme::install(ui.ctx());
                    ui.ctx().request_repaint();
                    return;
                }
                let (s, state, nav) = st;
                editor(ui, s, state, &[], 95.0, nav);
            },
            (crate::settings::Settings::default(), State::default(), Vec::new()),
        );
        h.run();
        h.get_by_label("Gamer").click();
        h.run();
        let tiles = h.state().0.overview_tiles.clone().expect("a preset sets the tiles");
        assert_eq!(tiles, presets::overview("Gamer", 95.0));
        assert!(h.state().2.iter().any(|n| matches!(n, Nav::Save)));
        h.get_by_label("Restore defaults").click();
        h.run();
        assert!(h.state().0.overview_tiles.is_none());
    }
}
