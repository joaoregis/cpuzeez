//! The capability banner (docs/v1/README.md P3): what a page can't show on
//! this machine, why, and the one action that fixes it. The decision lives in
//! `cpuzeez_core::availability`; every page uses this one component.

use super::i18n::{tr, trf};
use super::theme::{self, icon};
use super::widgets;
use crate::Ctx;
use crate::executor::Done;
use cpuzeez_core::actions::{Action, KernelModule};
use cpuzeez_core::availability::{Capability, Gap, Need};
use cpuzeez_platform::elevation::{self, Outcome, Pending};
use eframe::egui::{self, CornerRadius, Frame, Margin, RichText, Stroke};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

pub const PT_BR: &[(&str, &str)] = &[
    ("Some information isn't available", "Algumas informações não estão disponíveis"),
    ("Needs administrator rights: {}", "Precisa de permissão de administrador: {}"),
    ("Needs root: {}", "Precisa de root: {}"),
    ("Needs the PawnIO driver: {}", "Precisa do driver PawnIO: {}"),
    ("Needs the {} kernel module (sudo modprobe {}): {}", "Precisa do módulo de kernel {} (sudo modprobe {}): {}"),
    ("Not available: {} ({})", "Indisponível: {} ({})"),
    ("Restart as administrator", "Reiniciar como administrador"),
    ("Restart as root", "Reiniciar como root"),
    ("How to install PawnIO", "Como instalar o PawnIO"),
    ("Install hardware access", "Instalar acesso ao hardware"),
    (
        "Installs the PawnIO driver (pawnio.eu) shipped with CPU ZeeZ, after checking its SHA-256. Windows asks for permission.",
        "Instala o driver PawnIO (pawnio.eu) que acompanha o CPU ZeeZ, após conferir o SHA-256. O Windows pede permissão.",
    ),
    ("Hide", "Ocultar"),
    ("Show what's hidden", "Mostrar o que está oculto"),
    ("Waiting for permission…", "Aguardando permissão…"),
    ("Administrator rights were not granted.", "A permissão de administrador não foi concedida."),
    ("Could not restart: {}", "Não foi possível reiniciar: {}"),
    ("Enable {}", "Ativar {}"),
    ("Enabled. Restart CPU ZeeZ to use it.", "Ativado. Reinicie o CPU ZeeZ para usar."),
    ("Could not enable it: {}", "Não foi possível ativar: {}"),
    ("per-core temperatures, clocks and power", "temperaturas, clocks e consumo por núcleo"),
    ("mainboard voltages, fans and temperatures", "tensões, ventoinhas e temperaturas da placa-mãe"),
    (
        "memory module details (SPD) and DIMM temperatures",
        "detalhes dos módulos de memória (SPD) e temperaturas dos DIMMs",
    ),
    ("live memory timings", "timings da memória em tempo real"),
    ("chipset temperature", "temperatura do chipset"),
    ("PawnIO has no module for this CPU or chipset", "o PawnIO não tem módulo para esta CPU ou chipset"),
    (
        "PawnIO's modules are missing: put the pawnio-modules folder next to cpuzeez.exe",
        "faltam os módulos do PawnIO: coloque a pasta pawnio-modules ao lado do cpuzeez.exe",
    ),
    ("no supported Super I/O chip or kernel driver", "nenhum chip Super I/O ou driver de kernel compatível"),
    ("the kernel gives no access to it on this machine", "o kernel não dá acesso a isso nesta máquina"),
    ("macOS doesn't allow third-party register access", "o macOS não permite acesso de terceiros aos registradores"),
    ("not supported on this OS", "não suportado neste sistema"),
];

pub const ES: &[(&str, &str)] = &[
    ("Some information isn't available", "Parte de la información no está disponible"),
    ("Needs administrator rights: {}", "Requiere permisos de administrador: {}"),
    ("Needs root: {}", "Requiere root: {}"),
    ("Needs the PawnIO driver: {}", "Requiere el controlador PawnIO: {}"),
    ("Needs the {} kernel module (sudo modprobe {}): {}", "Requiere el módulo del kernel {} (sudo modprobe {}): {}"),
    ("Not available: {} ({})", "No disponible: {} ({})"),
    ("Restart as administrator", "Reiniciar como administrador"),
    ("Restart as root", "Reiniciar como root"),
    ("How to install PawnIO", "Cómo instalar PawnIO"),
    ("Install hardware access", "Instalar acceso al hardware"),
    (
        "Installs the PawnIO driver (pawnio.eu) shipped with CPU ZeeZ, after checking its SHA-256. Windows asks for permission.",
        "Instala el controlador PawnIO (pawnio.eu) incluido con CPU ZeeZ, tras comprobar su SHA-256. Windows pide permiso.",
    ),
    ("Hide", "Ocultar"),
    ("Show what's hidden", "Mostrar lo oculto"),
    ("Waiting for permission…", "Esperando permiso…"),
    ("Administrator rights were not granted.", "No se concedieron los permisos de administrador."),
    ("Could not restart: {}", "No se pudo reiniciar: {}"),
    ("Enable {}", "Activar {}"),
    ("Enabled. Restart CPU ZeeZ to use it.", "Activado. Reinicia CPU ZeeZ para usarlo."),
    ("Could not enable it: {}", "No se pudo activar: {}"),
    ("per-core temperatures, clocks and power", "temperaturas, relojes y consumo por núcleo"),
    ("mainboard voltages, fans and temperatures", "voltajes, ventiladores y temperaturas de la placa base"),
    (
        "memory module details (SPD) and DIMM temperatures",
        "detalles de los módulos de memoria (SPD) y temperaturas de los DIMM",
    ),
    ("live memory timings", "latencias de la memoria en vivo"),
    ("chipset temperature", "temperatura del chipset"),
    ("PawnIO has no module for this CPU or chipset", "PawnIO no tiene módulo para esta CPU o chipset"),
    (
        "PawnIO's modules are missing: put the pawnio-modules folder next to cpuzeez.exe",
        "faltan los módulos de PawnIO: coloca la carpeta pawnio-modules junto a cpuzeez.exe",
    ),
    ("no supported Super I/O chip or kernel driver", "ningún chip Super I/O ni controlador del kernel compatible"),
    ("the kernel gives no access to it on this machine", "el kernel no da acceso a esto en esta máquina"),
    ("macOS doesn't allow third-party register access", "macOS no permite el acceso de terceros a los registros"),
    ("not supported on this OS", "no compatible con este sistema"),
];

/// The document at the tag of this version, so the link works for a pre-release too
/// (a branch name or `main` may not have the file yet).
fn pawnio_doc() -> String {
    format!("{}/blob/v{}/docs/windows-pawnio.md", crate::REPO_URL, crate::VERSION)
}

/// Banner state shared by every page.
#[derive(Default)]
pub struct Access {
    gaps: Option<Vec<Gap>>,
    hidden: BTreeSet<&'static str>,
    pending: Option<Pending>,
    message: Option<String>,
    /// An action running off the UI thread (its elevation prompt blocks).
    running: Option<Arc<Mutex<Option<Done>>>>,
    /// The elevated copy is up: the app should close.
    pub restarted: bool,
}

impl Access {
    fn gaps(&mut self, ctx: &Ctx) -> &[Gap] {
        self.gaps.get_or_insert_with(|| crate::access::gaps(ctx))
    }

    /// Run `a` (with its own elevation prompt if needed) off the UI thread.
    fn start(&mut self, a: Action) {
        let slot = Arc::new(Mutex::new(None));
        let s = slot.clone();
        std::thread::spawn(move || *s.lock().unwrap_or_else(|e| e.into_inner()) = Some(crate::executor::run(&a)));
        self.running = Some(slot);
        self.message = None;
    }

    /// Restart CPU ZeeZ elevated (a page outside the banner asked for it).
    pub fn restart_elevated(&mut self) {
        if self.pending.is_none() && elevation::can_relaunch() {
            let args: Vec<String> = std::env::args().skip(1).filter(|a| !a.contains("elevated-handoff")).collect();
            self.pending = Some(elevation::relaunch(&args));
        }
    }

    pub fn restarting(&self) -> bool {
        self.pending.is_some()
    }

    /// Call every frame: finishes a pending restart or action.
    pub fn frame(&mut self, ectx: &egui::Context) {
        if let Some(slot) = &self.running {
            let done = slot.lock().unwrap_or_else(|e| e.into_inner()).take();
            match done {
                None => ectx.request_repaint_after(std::time::Duration::from_millis(200)),
                Some(d) => {
                    self.running = None;
                    if matches!(d, Done::Ok(_)) {
                        self.gaps = None; // re-check: e.g. PawnIO now installed, so "run as admin" is next
                    }
                    self.message = match d {
                        Done::Ok(_) => Some(tr("Enabled. Restart CPU ZeeZ to use it.").into()),
                        Done::Cancelled => Some(tr("Administrator rights were not granted.").into()),
                        Done::Failed(e) => Some(trf("Could not enable it: {}", &[&e])),
                    };
                }
            }
        }
        let Some(p) = &mut self.pending else { return };
        match p.poll() {
            None => ectx.request_repaint_after(std::time::Duration::from_millis(200)),
            Some(Outcome::Started) => {
                self.pending = None;
                self.restarted = true;
            }
            Some(Outcome::Cancelled) => {
                self.pending = None;
                self.message = Some(tr("Administrator rights were not granted.").into());
            }
            Some(Outcome::Unsupported) => self.pending = None,
            Some(Outcome::Failed(e)) => {
                self.pending = None;
                self.message = Some(trf("Could not restart: {}", &[&e]));
            }
        }
    }

    /// The banner for `page`, covering `caps`. Draws nothing when all is available.
    pub fn banner(&mut self, ui: &mut egui::Ui, ctx: &Ctx, page: &'static str, caps: &[Capability]) {
        let gaps: Vec<Gap> = self.gaps(ctx).iter().filter(|g| caps.contains(&g.cap)).cloned().collect();
        if gaps.is_empty() && self.message.is_none() {
            return;
        }
        let p = theme::palette(ui);
        if self.hidden.contains(page) {
            if ui.small_button(format!("{} {}", icon::WARNING, tr("Show what's hidden"))).clicked() {
                self.hidden.remove(page);
            }
            ui.add_space(theme::GAP / 2.0);
            return;
        }
        let list = |need: &dyn Fn(&Need) -> bool| -> String {
            gaps.iter().filter(|g| need(&g.need)).map(|g| tr(g.cap.label())).collect::<Vec<_>>().join("; ")
        };
        Frame::default()
            .fill(p.warning_soft)
            .stroke(Stroke::new(1.0, p.warning))
            .corner_radius(CornerRadius::same(theme::RADIUS_CARD))
            .inner_margin(Margin::same(theme::PAD_CARD))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(icon::WARNING).color(p.warning));
                    ui.label(RichText::new(tr("Some information isn't available")).strong().color(p.text));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button(tr("Hide")).clicked() {
                            self.hidden.insert(page);
                        }
                    });
                });
                let elevation = list(&|n| *n == Need::Elevation);
                if !elevation.is_empty() {
                    let key = if cfg!(windows) { "Needs administrator rights: {}" } else { "Needs root: {}" };
                    ui.label(RichText::new(trf(key, &[&elevation])).color(p.text_secondary));
                }
                let driver = list(&|n| *n == Need::Driver);
                if !driver.is_empty() {
                    ui.label(RichText::new(trf("Needs the PawnIO driver: {}", &[&driver])).color(p.text_secondary));
                }
                for m in gaps
                    .iter()
                    .filter_map(|g| if let Need::KernelModule(m) = g.need { Some(m) } else { None })
                    .collect::<BTreeSet<_>>()
                {
                    let l = list(&|n| *n == Need::KernelModule(m));
                    let t = trf("Needs the {} kernel module (sudo modprobe {}): {}", &[&m, &m, &l]);
                    ui.label(RichText::new(t).color(p.text_secondary));
                }
                for reason in gaps
                    .iter()
                    .filter_map(|g| if let Need::Unsupported(r) = g.need { Some(r) } else { None })
                    .collect::<BTreeSet<_>>()
                {
                    let l = list(&|n| *n == Need::Unsupported(reason));
                    ui.label(RichText::new(trf("Not available: {} ({})", &[&l, &tr(reason)])).color(p.text_muted));
                }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if !elevation.is_empty() && elevation::can_relaunch() {
                        if self.pending.is_some() {
                            ui.spinner();
                            ui.label(tr("Waiting for permission…"));
                        } else {
                            let label = if cfg!(windows) { "Restart as administrator" } else { "Restart as root" };
                            if widgets::primary_button(ui, &format!("{} {}", icon::SHIELD_CHECK, tr(label))).clicked() {
                                self.message = None;
                                let args: Vec<String> =
                                    std::env::args().skip(1).filter(|a| !a.contains("elevated-handoff")).collect();
                                self.pending = Some(elevation::relaunch(&args));
                            }
                        }
                    }
                    for m in gaps
                        .iter()
                        .filter_map(|g| if let Need::KernelModule(m) = g.need { kernel_module(m) } else { None })
                        .collect::<BTreeSet<_>>()
                    {
                        if self.running.is_some() {
                            ui.spinner();
                        } else if ui.button(trf("Enable {}", &[&m.name()])).clicked() {
                            self.start(Action::LoadKernelModule { module: m });
                        }
                    }
                    if !driver.is_empty() && crate::executor::bundled_pawnio().is_some() {
                        let label = format!("{} {}", icon::SHIELD_CHECK, tr("Install hardware access"));
                        if self.running.is_some() {
                            ui.spinner();
                        } else if widgets::primary_button(ui, &label)
                            .on_hover_text(tr("Installs the PawnIO driver (pawnio.eu) shipped with CPU ZeeZ, after checking its SHA-256. Windows asks for permission."))
                            .clicked()
                        {
                            self.start(Action::InstallPawnIo);
                        }
                    }
                    if !driver.is_empty() && ui.button(tr("How to install PawnIO")).clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(pawnio_doc())); // opens only on this click (P1)
                    }
                });
                if let Some(m) = &self.message {
                    ui.label(RichText::new(m).small().color(p.text_muted));
                }
            });
        ui.add_space(theme::GAP);
    }
}

/// The allow-listed module behind a kernel-module need.
fn kernel_module(name: &str) -> Option<KernelModule> {
    [KernelModule::Msr, KernelModule::I2cDev].into_iter().find(|m| m.name() == name)
}

/// The capabilities each page shows, for its banner.
pub fn page_caps(page: &str) -> &'static [Capability] {
    use Capability::*;
    match page {
        "Overview" => &[CpuSensors, ChipsetTemp],
        "Processor" => &[CpuSensors],
        "Mainboard" => &[BoardSensors, ChipsetTemp],
        "Memory" => &[Spd, MemoryTimings],
        "Sensors" => &[CpuSensors, BoardSensors, Spd, ChipsetTemp],
        _ => &[],
    }
}
