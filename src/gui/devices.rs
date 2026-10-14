//! Devices screen: drives (health badge, SMART / NVMe health table,
//! temperature), network adapters and batteries. Probed once in the
//! background (drive IOCTLs and adapter queries are slow), on demand again
//! with Refresh.

use super::i18n::{tr, trf};
use super::theme::{self, icon};
use super::widgets::{self, Tone};
use crate::Ctx;
use eframe::egui::{self, RichText};
use std::sync::{Arc, Mutex};

pub const PT_BR: &[(&str, &str)] = &[
    ("Devices", "Dispositivos"),
    ("Storage", "Armazenamento"),
    ("Network", "Rede"),
    ("Batteries", "Baterias"),
    ("Refresh", "Atualizar"),
    ("Reading devices…", "Lendo dispositivos…"),
    ("No drives found.", "Nenhum disco encontrado."),
    ("No network adapters found.", "Nenhum adaptador de rede encontrado."),
    ("No battery (desktop or not reported).", "Nenhuma bateria (desktop ou não informada)."),
    ("Healthy", "Saudável"),
    ("Caution", "Atenção"),
    ("Failing", "Falhando"),
    ("Unknown", "Desconhecido"),
    ("Capacity", "Capacidade"),
    ("Type", "Tipo"),
    ("Bus Type", "Tipo de barramento"),
    ("Rotation speed", "Velocidade de rotação"),
    ("Features", "Recursos"),
    ("Controller", "Controladora"),
    ("Link Speed", "Velocidade do link"),
    ("Volume", "Volume"),
    ("Temperature", "Temperatura"),
    ("Firmware", "Firmware"),
    ("Health details ({})", "Detalhes de saúde ({})"),
    ("Attribute", "Atributo"),
    ("Value", "Valor"),
    ("Description", "Descrição"),
    ("MAC", "MAC"),
    ("State", "Estado"),
    ("Up", "Conectado"),
    ("Down", "Desconectado"),
    ("Link speed", "Velocidade do link"),
    ("MTU", "MTU"),
    ("IPv4", "IPv4"),
    ("IPv6", "IPv6"),
    ("Gateway", "Gateway"),
    ("DNS", "DNS"),
    ("Driver", "Driver"),
    ("Device", "Dispositivo"),
    ("Wi-Fi", "Wi-Fi"),
    ("Manufacturer", "Fabricante"),
    ("Serial", "Número de série"),
    ("Chemistry", "Química"),
    ("Health", "Saúde"),
    ("Wear level", "Desgaste"),
    ("Cycle count", "Ciclos de carga"),
    ("Design capacity", "Capacidade de projeto"),
    ("Full charge capacity", "Capacidade de carga total"),
    ("Remaining", "Restante"),
    ("Charge", "Carga"),
    ("Status", "Status"),
    ("Rate", "Taxa"),
    ("Voltage", "Tensão"),
    ("Fixed", "Fixo"),
    ("Removable", "Removível"),
    ("Wear (endurance used)", "Desgaste (vida útil consumida)"),
    ("Power-on hours", "Horas ligado"),
    ("Power cycles", "Ciclos de energia"),
    ("Data read", "Dados lidos"),
    ("Data written", "Dados gravados"),
    ("ID", "ID"),
    ("Current", "Atual"),
    ("Worst", "Pior"),
    ("Threshold", "Limite"),
    ("Raw", "Bruto"),
];

pub const ES: &[(&str, &str)] = &[
    ("Devices", "Dispositivos"),
    ("Storage", "Almacenamiento"),
    ("Network", "Red"),
    ("Batteries", "Baterías"),
    ("Refresh", "Actualizar"),
    ("Reading devices…", "Leyendo dispositivos…"),
    ("No drives found.", "No se encontraron discos."),
    ("No network adapters found.", "No se encontraron adaptadores de red."),
    ("No battery (desktop or not reported).", "Sin batería (equipo de escritorio o no informada)."),
    ("Healthy", "Saludable"),
    ("Caution", "Precaución"),
    ("Failing", "Fallando"),
    ("Unknown", "Desconocido"),
    ("Capacity", "Capacidad"),
    ("Type", "Tipo"),
    ("Bus Type", "Tipo de bus"),
    ("Rotation speed", "Velocidad de rotación"),
    ("Features", "Características"),
    ("Controller", "Controladora"),
    ("Link Speed", "Velocidad del enlace"),
    ("Volume", "Volumen"),
    ("Temperature", "Temperatura"),
    ("Firmware", "Firmware"),
    ("Health details ({})", "Detalles de salud ({})"),
    ("Attribute", "Atributo"),
    ("Value", "Valor"),
    ("Description", "Descripción"),
    ("MAC", "MAC"),
    ("State", "Estado"),
    ("Up", "Conectado"),
    ("Down", "Desconectado"),
    ("Link speed", "Velocidad del enlace"),
    ("MTU", "MTU"),
    ("IPv4", "IPv4"),
    ("IPv6", "IPv6"),
    ("Gateway", "Puerta de enlace"),
    ("DNS", "DNS"),
    ("Driver", "Controlador"),
    ("Device", "Dispositivo"),
    ("Wi-Fi", "Wi-Fi"),
    ("Manufacturer", "Fabricante"),
    ("Serial", "Número de serie"),
    ("Chemistry", "Química"),
    ("Health", "Salud"),
    ("Wear level", "Desgaste"),
    ("Cycle count", "Ciclos de carga"),
    ("Design capacity", "Capacidad de diseño"),
    ("Full charge capacity", "Capacidad de carga completa"),
    ("Remaining", "Restante"),
    ("Charge", "Carga"),
    ("Status", "Estado"),
    ("Rate", "Tasa"),
    ("Voltage", "Voltaje"),
    ("Fixed", "Fijo"),
    ("Removable", "Extraíble"),
    ("Wear (endurance used)", "Desgaste (vida útil consumida)"),
    ("Power-on hours", "Horas encendido"),
    ("Power cycles", "Ciclos de encendido"),
    ("Data read", "Datos leídos"),
    ("Data written", "Datos escritos"),
    ("ID", "ID"),
    ("Current", "Actual"),
    ("Worst", "Peor"),
    ("Threshold", "Umbral"),
    ("Raw", "Bruto"),
];

#[derive(Default)]
struct Data {
    drives: Vec<crate::storage::Drive>,
    adapters: Vec<crate::network::Adapter>,
    batteries: Vec<crate::battery::Battery>,
}

#[derive(Default)]
pub struct State {
    data: Arc<Mutex<Option<Data>>>,
    started: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Health {
    Good,
    Caution,
    Bad,
    Unknown,
}

/// Overall drive health from whatever SMART / NVMe values the probe found.
fn health(values: &[(String, String)]) -> Health {
    let mut h = Health::Unknown;
    for (k, v) in values {
        let (k, v) = (k.to_ascii_lowercase(), v.to_ascii_lowercase());
        let num = v.trim_end_matches(['%', ' ', 'c']).parse::<f64>().ok();
        let this = if k.contains("smart status") || k == "health" || k.contains("overall") {
            if v.contains("fail") || v.contains("bad") {
                Health::Bad
            } else if v.contains("verified") || v.contains("ok") || v.contains("pass") || v.contains("good") {
                Health::Good
            } else {
                Health::Unknown
            }
        } else if k.contains("critical warning") {
            if num == Some(0.0) || v == "0x00" || v == "none" { Health::Good } else { Health::Bad }
        } else if k.contains("percentage used") {
            match num {
                Some(p) if p >= 100.0 => Health::Bad,
                Some(p) if p >= 90.0 => Health::Caution,
                Some(_) => Health::Good,
                None => Health::Unknown,
            }
        } else if k.contains("reallocated") || k.contains("pending") || k.contains("uncorrectable") {
            match num {
                Some(n) if n > 0.0 => Health::Caution,
                Some(_) => Health::Good,
                None => Health::Unknown,
            }
        } else {
            continue;
        };
        h = match (h, this) {
            (Health::Bad, _) | (_, Health::Bad) => Health::Bad,
            (Health::Caution, _) | (_, Health::Caution) => Health::Caution,
            (Health::Good, _) | (_, Health::Good) => Health::Good,
            _ => Health::Unknown,
        };
    }
    h
}

fn tone(h: Health) -> (Tone, &'static str) {
    match h {
        Health::Good => (Tone::Success, "Healthy"),
        Health::Caution => (Tone::Warning, "Caution"),
        Health::Bad => (Tone::Danger, "Failing"),
        Health::Unknown => (Tone::Neutral, "Unknown"),
    }
}

fn gb(b: u64) -> String {
    format!("{:.1} GB", b as f64 / 1e9)
}

fn opt<T>(v: Option<T>, f: impl FnOnce(T) -> String) -> String {
    v.map(f).unwrap_or_default()
}

/// Translated labels, empty rows dropped.
fn rows(r: Vec<(&'static str, String)>) -> Vec<(&'static str, String)> {
    r.into_iter().filter(|x| !x.1.is_empty()).map(|(k, v)| (tr(k), v)).collect()
}

impl State {
    fn ensure(&mut self, ctx: &Arc<Ctx>) {
        if self.started {
            return;
        }
        self.started = true;
        let (slot, ctx) = (self.data.clone(), ctx.clone());
        std::thread::spawn(move || {
            let d = Data {
                drives: crate::storage::drives(&ctx),
                adapters: crate::network::adapters(&ctx),
                batteries: crate::battery::batteries(&ctx),
            };
            *slot.lock().unwrap() = Some(d);
        });
    }

    /// True once the background probe finished (screenshots wait for it).
    pub fn loaded(&mut self, ctx: &Arc<Ctx>) -> bool {
        self.ensure(ctx);
        self.data.lock().unwrap().is_some()
    }

    /// The Devices page: storage, network, batteries.
    pub fn page(&mut self, ui: &mut egui::Ui, ctx: &Arc<Ctx>) {
        self.ensure(ctx);
        let mut refresh = false;
        widgets::page_header(ui, tr("Devices"), "", |ui| {
            refresh = ui.button(format!("{} {}", icon::ARROW_CLOCKWISE, tr("Refresh"))).clicked();
        });
        {
            let guard = self.data.lock().unwrap();
            let Some(d) = guard.as_ref() else {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(tr("Reading devices…"));
                });
                return;
            };
            section(ui, icon::HARD_DRIVES, "Storage");
            if d.drives.is_empty() {
                widgets::empty_state(ui, icon::HARD_DRIVE, tr("Storage"), tr("No drives found."));
            }
            for (i, dr) in d.drives.iter().enumerate() {
                drive(ui, i, dr);
            }
            section(ui, icon::NETWORK, "Network");
            if d.adapters.is_empty() {
                widgets::empty_state(ui, icon::NETWORK, tr("Network"), tr("No network adapters found."));
            }
            for a in &d.adapters {
                adapter(ui, a);
            }
            section(ui, icon::BATTERY_HIGH, "Batteries");
            if d.batteries.is_empty() {
                widgets::empty_state(
                    ui,
                    icon::BATTERY_HIGH,
                    tr("Batteries"),
                    tr("No battery (desktop or not reported)."),
                );
            }
            for b in &d.batteries {
                battery(ui, b);
            }
        }
        if refresh {
            *self.data.lock().unwrap() = None;
            self.started = false;
        }
    }
}

fn section(ui: &mut egui::Ui, ic: &str, title: &'static str) {
    ui.add_space(theme::GAP);
    ui.label(RichText::new(format!("{ic}  {}", tr(title))).text_style(egui::TextStyle::Heading));
    ui.add_space(4.0);
}

/// A card with a status pill under its header.
fn device_card(ui: &mut egui::Ui, ic: &str, title: &str, status: (Tone, &str), body: impl FnOnce(&mut egui::Ui)) {
    widgets::card(ui, ic, title, |ui| {
        widgets::pill(ui, status.1, status.0);
        body(ui);
    });
    ui.add_space(theme::GAP);
}

fn level(l: crate::storage::HealthLevel) -> Health {
    use crate::storage::HealthLevel as L;
    match l {
        L::Good => Health::Good,
        L::Caution => Health::Caution,
        L::Bad => Health::Bad,
        L::Unknown => Health::Unknown,
    }
}

fn bytes_tb(b: u64) -> String {
    if b >= 1_000_000_000_000 { format!("{:.2} TB", b as f64 / 1e12) } else { format!("{:.1} GB", b as f64 / 1e9) }
}

fn drive(ui: &mut egui::Ui, i: usize, d: &crate::storage::Drive) {
    let s = d.smart.as_ref();
    let temp = s.and_then(|s| s.temperature_c).map(|t| format!("{t:.0} °C")).or_else(|| {
        d.health.iter().find(|(k, _)| k.to_ascii_lowercase().contains("temperature")).map(|(_, v)| v.clone())
    });
    let kind = format!("{}{}", tr(if d.removable { "Removable" } else { "Fixed" }), if d.ssd { ", SSD" } else { "" });
    let mut r = vec![
        ("Capacity", gb(d.capacity)),
        ("Type", kind),
        ("Bus Type", crate::storage::bus_name(d.bus).to_string()),
        ("Temperature", temp.unwrap_or_default()),
        ("Wear (endurance used)", opt(s.and_then(|s| s.percent_used), |p| format!("{p}%"))),
        ("Power-on hours", opt(s.and_then(|s| s.power_on_hours), |h| format!("{h} h"))),
        ("Power cycles", opt(s.and_then(|s| s.power_cycles), |c| c.to_string())),
        ("Data read", opt(s.and_then(|s| s.data_read_bytes), bytes_tb)),
        ("Data written", opt(s.and_then(|s| s.data_written_bytes), bytes_tb)),
        ("Firmware", d.revision.clone().unwrap_or_default()),
        ("Serial", super::shown("Serial", d.serial.as_deref().unwrap_or_default()).to_string()),
        ("Rotation speed", opt(d.rpm.filter(|&r| r > 1), |r| format!("{r} RPM"))),
        ("Features", d.features.join(", ")),
        ("Controller", d.controller.clone().unwrap_or_default()),
        ("Link Speed", d.link.clone().unwrap_or_default()),
    ];
    r.extend(d.volumes.iter().map(|v| ("Volume", crate::storage::volume_line(v))));
    let h = s.map_or_else(|| health(&d.health), |s| level(s.health));
    let (t, text) = tone(h);
    device_card(ui, icon::HARD_DRIVE, &d.name, (t, tr(text)), |ui| {
        if let Some(s) = s {
            let p = theme::palette(ui);
            for reason in &s.reasons {
                ui.label(RichText::new(format!("{} {reason}", icon::WARNING)).color(p.warning));
            }
        }
        widgets::kv_list(ui, &rows(r));
        let Some(s) = s else { return };
        let n = s.ata_attributes.len() + s.nvme_log.len();
        if n == 0 {
            return;
        }
        egui::CollapsingHeader::new(trf("Health details ({})", &[&n])).id_salt(("smart", i)).show(ui, |ui| {
            ui.label(RichText::new(&s.source).small().weak());
            egui::ScrollArea::horizontal().id_salt(("smart-scroll", i)).show(ui, |ui| {
                egui::Grid::new(("smart-grid", i)).striped(true).spacing([12.0, 3.0]).show(ui, |ui| {
                    if s.ata_attributes.is_empty() {
                        ui.label(RichText::new(tr("Attribute")).strong());
                        ui.label(RichText::new(tr("Value")).strong());
                        ui.end_row();
                        for (k, v) in &s.nvme_log {
                            ui.label(tr(k));
                            super::value(ui, v);
                            ui.end_row();
                        }
                        return;
                    }
                    for h in ["ID", "Attribute", "Current", "Worst", "Threshold", "Raw"] {
                        ui.label(RichText::new(tr(h)).strong());
                    }
                    ui.end_row();
                    let danger = theme::palette(ui).danger;
                    for a in &s.ata_attributes {
                        let txt = |t: String| {
                            let rt = RichText::new(t).monospace();
                            if a.failing() { rt.color(danger) } else { rt }
                        };
                        ui.label(txt(format!("{:02X}", a.id)));
                        ui.label(txt(a.name.clone()));
                        ui.label(txt(a.current.to_string()));
                        ui.label(txt(a.worst.to_string()));
                        ui.label(txt(a.threshold.map_or("-".into(), |t| t.to_string())));
                        ui.label(txt(a.raw.to_string()));
                        ui.end_row();
                    }
                });
            });
        });
    });
}

fn adapter(ui: &mut egui::Ui, a: &crate::network::Adapter) {
    let speed =
        opt(a.link_mbps, |m| if m >= 1000 { format!("{:.1} Gbps", m as f64 / 1000.0) } else { format!("{m} Mbps") });
    let wifi = [
        a.wifi_ssid.as_deref().map(|s| super::shown("SSID", s).to_string()),
        a.wifi_band.clone(),
        a.wifi_signal_pct.map(|p| format!("{p}%")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ");
    let join = |key: &str, v: &[String]| v.iter().map(|x| super::shown(key, x)).collect::<Vec<_>>().join(", ");
    let r = vec![
        ("Description", a.description.clone()),
        ("Type", a.kind.clone()),
        ("MAC", super::shown("MAC", &a.mac).to_string()),
        ("Link speed", speed),
        ("MTU", opt(a.mtu, |m| m.to_string())),
        ("IPv4", join("IPv4", &a.ipv4)),
        ("IPv6", join("IPv6", &a.ipv6)),
        ("Gateway", join("Gateway", &a.gateway)),
        ("DNS", join("DNS", &a.dns)),
        ("Wi-Fi", wifi),
        ("Driver", format!("{} {}", a.driver, a.driver_version).trim().to_string()),
        ("Device", a.device.clone()),
    ];
    let state = if a.up { (Tone::Success, tr("Up")) } else { (Tone::Neutral, tr("Down")) };
    let title = if a.name.is_empty() { &a.description } else { &a.name };
    let ic = if a.kind == "Wi-Fi" { icon::WIFI_HIGH } else { icon::NETWORK };
    device_card(ui, ic, title, state, |ui| widgets::kv_list(ui, &rows(r)));
}

fn battery(ui: &mut egui::Ui, b: &crate::battery::Battery) {
    let mwh = |v: Option<f64>| opt(v, |v| format!("{v:.0} mWh"));
    let h = match b.health_pct {
        Some(p) if p >= 80.0 => Health::Good,
        Some(p) if p >= 50.0 => Health::Caution,
        Some(_) => Health::Bad,
        None => Health::Unknown,
    };
    let r = vec![
        ("Manufacturer", b.manufacturer.clone()),
        ("Serial", super::shown("Serial", &b.serial).to_string()),
        ("Chemistry", b.chemistry.clone()),
        ("Health", opt(b.health_pct, |p| format!("{p:.1}%"))),
        ("Wear level", opt(b.health_pct, |p| format!("{:.1}%", (100.0 - p).max(0.0)))),
        ("Cycle count", opt(b.cycles, |c| c.to_string())),
        ("Design capacity", mwh(b.design_mwh)),
        ("Full charge capacity", mwh(b.full_mwh)),
        ("Remaining", mwh(b.remaining_mwh)),
        ("Status", b.status.clone()),
        ("Rate", opt(b.rate_w, |w| format!("{w:+.2} W"))),
        ("Voltage", opt(b.voltage_v, |v| format!("{v:.3} V"))),
        ("Temperature", opt(b.temperature_c, |t| format!("{t:.1} °C"))),
    ];
    let (t, text) = tone(h);
    device_card(ui, icon::BATTERY_HIGH, &b.name, (t, tr(text)), |ui| {
        if let Some(p) = b.percent {
            widgets::progress(ui, (p / 100.0) as f32, &format!("{} {p:.0}%", tr("Charge")));
        }
        widgets::kv_list(ui, &rows(r));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_badges() {
        let v = |pairs: &[(&str, &str)]| pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect::<Vec<_>>();
        assert!(health(&[]) == Health::Unknown);
        assert!(health(&v(&[("Percentage Used", "3%"), ("Temperature", "40 C")])) == Health::Good);
        assert!(health(&v(&[("Percentage Used", "95%")])) == Health::Caution);
        assert!(health(&v(&[("SMART Status", "Verified")])) == Health::Good);
        assert!(health(&v(&[("SMART Status", "Failing"), ("Percentage Used", "1%")])) == Health::Bad);
        assert!(health(&v(&[("Reallocated Sectors Count", "8")])) == Health::Caution);
    }
}
