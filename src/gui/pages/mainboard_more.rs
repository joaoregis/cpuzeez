//! Mainboard page, S1-G04 additions: firmware, PCI Express links with
//! alerts, Super I/O readings by their board names, slots and ports, the
//! SMBIOS browser and (Expert) the ACPI tables.

use crate::gui::table::{Cell, Column, Table};
#[allow(unused_imports)]
use crate::gui::*;
use cpuzeez_core::detail::Level;
use cpuzeez_core::pci::Device;

pub const PT_BR: &[(&str, &str)] = &[
    ("Resizable BAR", "Resizable BAR"),
    ("Above 4G decoding", "Decodificação acima de 4G"),
    ("Firmware", "Firmware"),
    ("Boot mode", "Modo de inicialização"),
    ("UEFI", "UEFI"),
    ("Legacy BIOS (CSM)", "BIOS legado (CSM)"),
    ("Secure Boot", "Inicialização segura"),
    ("On", "Ligada"),
    ("Off", "Desligada"),
    ("TPM", "TPM"),
    ("Not found", "Não encontrado"),
    ("PCI Express links", "Links PCI Express"),
    ("Device", "Dispositivo"),
    ("Location", "Local"),
    ("Link", "Link"),
    ("Max", "Máx"),
    ("Status", "Situação"),
    ("Narrower than it supports", "Mais estreito do que suporta"),
    ("Slower now (normal at idle)", "Mais lento agora (normal em repouso)"),
    ("OK", "OK"),
    ("Super I/O", "Super I/O"),
    ("Slots and ports", "Slots e portas"),
    ("SMBIOS", "SMBIOS"),
    ("ACPI tables", "Tabelas ACPI"),
];

pub const ES: &[(&str, &str)] = &[
    ("Resizable BAR", "Resizable BAR"),
    ("Above 4G decoding", "Decodificación por encima de 4G"),
    ("Firmware", "Firmware"),
    ("Boot mode", "Modo de arranque"),
    ("UEFI", "UEFI"),
    ("Legacy BIOS (CSM)", "BIOS heredado (CSM)"),
    ("Secure Boot", "Arranque seguro"),
    ("On", "Activado"),
    ("Off", "Desactivado"),
    ("TPM", "TPM"),
    ("Not found", "No encontrado"),
    ("PCI Express links", "Enlaces PCI Express"),
    ("Device", "Dispositivo"),
    ("Location", "Ubicación"),
    ("Link", "Enlace"),
    ("Max", "Máx"),
    ("Status", "Estado"),
    ("Narrower than it supports", "Más estrecho de lo que admite"),
    ("Slower now (normal at idle)", "Más lento ahora (normal en reposo)"),
    ("OK", "OK"),
    ("Super I/O", "Super I/O"),
    ("Slots and ports", "Ranuras y puertos"),
    ("SMBIOS", "SMBIOS"),
    ("ACPI tables", "Tablas ACPI"),
];

fn link_text(width: u8, speed: u8) -> String {
    if width == 0 {
        String::new()
    } else if speed == 0 {
        format!("x{width}")
    } else {
        format!("x{width} Gen {speed}")
    }
}

/// "Narrower than it supports" is worth fixing; a slower link only means
/// the device saved power while idle.
fn link_status(d: &Device) -> &'static str {
    match d.pcie_link() {
        Some(l) if l.width > 0 && l.max_width > 0 && l.width < l.max_width => "Narrower than it supports",
        Some(l) if l.speed > 0 && l.max_speed > 0 && l.speed < l.max_speed => "Slower now (normal at idle)",
        _ => "OK",
    }
}

impl App {
    pub(crate) fn mainboard_more(&mut self, ui: &mut egui::Ui, r: &[Section]) {
        let level = detail::get(ui.ctx());
        // Read in the background (ACPI tables, TPM): it took longer than a frame.
        let fw = crate::gui::cache::firmware(&self.ctx).unwrap_or_default();
        // The discrete GPU's windows (the iGPU's are fixed by the platform).
        let gpu_bars = crate::gpu::adapters(&self.ctx).iter().filter(|a| !crate::health::integrated(a)).find_map(|a| {
            crate::peripherals::bars().iter().find(|b| b.vendor == a.vendor_id && b.device == a.device_id)
        });
        let onoff = |b: Option<bool>| b.map(|b| tr(if b { "On" } else { "Off" }).to_string()).unwrap_or_default();
        boxed(
            ui,
            "Firmware",
            &[
                (
                    "Boot mode",
                    fw.uefi.map(|u| tr(if u { "UEFI" } else { "Legacy BIOS (CSM)" }).to_string()).unwrap_or_default(),
                ),
                ("Secure Boot", onoff(fw.secure_boot)),
                ("TPM", fw.tpm.clone().unwrap_or_else(|| tr("Not found").to_string())),
                // From the graphics card's BARs (Above 4G: a window above 4 GiB).
                ("Resizable BAR", gpu_bars.map(|b| onoff(Some(b.rebar()))).unwrap_or_default()),
                ("Above 4G decoding", gpu_bars.map(|b| onoff(Some(b.above_4g))).unwrap_or_default()),
            ],
        );
        if level < Level::Advanced {
            return;
        }

        let devs: Vec<&Device> = crate::pci::devices(&self.ctx)
            .iter()
            .filter(|d| d.pcie_link().is_some_and(|l| l.max_width > 0 && l.width > 0))
            .collect();
        if !devs.is_empty() {
            heading(ui, tr("PCI Express links"));
            let p = theme::palette(ui);
            let name = |d: &Device| d.name.clone().unwrap_or_else(|| cpuzeez_core::pci::class_desc(d.class));
            let table = Table::new("pcie", |d: &&Device| {
                (d.bdf.seg as u64) << 24 | (d.bdf.bus as u64) << 16 | (d.bdf.dev as u64) << 8 | d.bdf.func as u64
            })
            .col(Column::new("device", tr("Device"), 360.0, move |d: &&Device| Cell::Text(name(d))))
            .col(Column::new("location", tr("Location"), 110.0, |d: &&Device| Cell::Text(d.bdf.to_string())))
            .col(Column::new("link", tr("Link"), 100.0, |d: &&Device| {
                let l = d.pcie_link().unwrap_or_default();
                Cell::num(l.width as f64, link_text(l.width, l.speed))
            }))
            .col(Column::new("max", tr("Max"), 100.0, |d: &&Device| {
                let l = d.pcie_link().unwrap_or_default();
                Cell::num(l.max_width as f64, link_text(l.max_width, l.max_speed))
            }))
            .col(
                Column::new("status", tr("Status"), 230.0, |d: &&Device| Cell::Text(tr(link_status(d)).to_string()))
                    .colored(move |d| (link_status(d) == "Narrower than it supports").then_some(p.warning)),
            );
            let h = ((devs.len() as f32 + 3.0) * 30.0).min(420.0);
            widgets::card(ui, "", "", |ui| {
                ui.allocate_ui(egui::vec2(ui.available_width(), h), |ui| {
                    table.show(ui, &devs, &mut self.pcie_table, |_, _| {});
                });
            });
            ui.add_space(theme::GAP);
        }

        // Super I/O readings, named by the board map (S0's sensor maps).
        let tel = self.mon.tel();
        tel.want("mainboard", &["board/"]);
        let snap = tel.snapshot();
        let board: Vec<(String, String)> = snap
            .stats
            .iter()
            .filter(|s| s.id.starts_with("board/"))
            .map(|s| (s.label.clone(), sensors::fmt(s.current, s.unit)))
            .collect();
        if !board.is_empty() {
            let chip =
                format!("{} {}", sget(r, "LPCIO", "LPCIO Vendor"), sget(r, "LPCIO", "LPCIO Model")).trim().to_string();
            let rows: Vec<(&str, String)> = board.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
            let title =
                if chip.is_empty() { tr("Super I/O").to_string() } else { format!("{} · {chip}", tr("Super I/O")) };
            boxed(ui, &title, &rows);
        }

        if let Some(dmi) = sec(r, "DMI") {
            let mut rows: Vec<(String, String)> = Vec::new();
            for (kind, label) in [("DMI Extension Slot", "designation"), ("DMI Port Connector", "internal designator")]
            {
                for (_, c) in report::groups(&dmi.nodes, kind) {
                    let name =
                        report::find(c, label).or_else(|| report::find(c, "external designator")).unwrap_or_default();
                    let what: Vec<&str> = ["type", "usage", "connector type", "port type", "length"]
                        .iter()
                        .filter_map(|k| report::find(c, k))
                        .filter(|v| !v.is_empty() && *v != "None" && *v != "Other")
                        .collect();
                    if !name.is_empty() {
                        rows.push((name.to_string(), what.join(", ")));
                    }
                }
            }
            if !rows.is_empty() {
                let rows: Vec<(&str, String)> = rows.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
                boxed(ui, "Slots and ports", &rows);
            }
            heading(ui, tr("SMBIOS"));
            widgets::card(ui, "", "", |ui| tree(ui, &dmi.nodes, "smbios"));
            ui.add_space(theme::GAP);
        }

        if level == Level::Expert && !fw.acpi.is_empty() {
            heading(ui, tr("ACPI tables"));
            widgets::card(ui, "", "", |ui| {
                for t in &fw.acpi {
                    ui.label(
                        RichText::new(format!(
                            "{:<4}  {:<6}  {:<8}  rev {:<2}  {:>7} B  {} {:08X}",
                            t.signature,
                            t.oem_id,
                            t.oem_table_id,
                            t.revision,
                            t.length,
                            t.creator_id,
                            t.creator_revision
                        ))
                        .monospace(),
                    );
                }
            });
            ui.add_space(theme::GAP);
        }
    }
}

/// A report subtree as collapsible groups of label/value rows.
fn tree(ui: &mut egui::Ui, nodes: &[Node], salt: &str) {
    let fields: Vec<(&str, String)> = report::fields(nodes).map(|(k, v)| (k, v.to_string())).collect();
    if !fields.is_empty() {
        widgets::kv_list(ui, &fields);
    }
    for (i, n) in nodes.iter().enumerate() {
        if let Node::Group { title, value, children } = n {
            let head = if value.is_empty() { title.clone() } else { format!("{title}  {value}") };
            let id = format!("{salt}/{i}");
            egui::CollapsingHeader::new(head).id_salt(&id).show(ui, |ui| tree(ui, children, &id));
        }
    }
}

#[cfg(test)]
mod tests {
    use cpuzeez_core::pci::{Bdf, Device, Link};

    #[test]
    fn link_alerts_only_for_width() {
        let mut d = Device::new(Bdf { seg: 0, bus: 1, dev: 0, func: 0 });
        d.link = Some(Link { width: 4, speed: 4, max_width: 16, max_speed: 4 });
        assert_eq!(super::link_status(&d), "Narrower than it supports");
        d.link = Some(Link { width: 16, speed: 1, max_width: 16, max_speed: 4 });
        assert_eq!(super::link_status(&d), "Slower now (normal at idle)");
        d.link = Some(Link { width: 16, speed: 4, max_width: 16, max_speed: 4 });
        assert_eq!(super::link_status(&d), "OK");
        assert_eq!(super::link_text(8, 4), "x8 Gen 4");
    }
}
