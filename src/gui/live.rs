//! ZeeZ Live (S2b, docs/v1/zeez-live.md): the sidebar entry with its Live,
//! Overlay and Sessions tabs, and the overlay window itself.
//!
//! The overlay is CpuZeeZ's own window above the game: transparent, always on
//! top, never focused and click-through outside edit mode. Nothing is
//! injected into the game (P4). It shows a Widget Engine layout, the same
//! kind the mini monitors use.

use super::i18n::tr;
use super::sensors::Monitor;
use super::theme::{self, icon};
use super::widgets::{self, Tone};
use cpuzeez_core::layout::presets;
use cpuzeez_core::live::{self as model, Anchor};
use cpuzeez_platform::overlay as os_overlay;
use eframe::egui::{self, RichText, Ui};
use std::time::{Duration, Instant};

/// The overlay window's title: how `platform::overlay` finds it.
pub const OVERLAY_TITLE: &str = "CpuZeeZ ZeeZ Live overlay";

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tab {
    #[default]
    Live,
    Overlay,
    Sessions,
}

#[derive(Default)]
pub struct State {
    pub tab: Tab,
    /// The overlay takes the mouse (drag it), until Esc or "Done".
    pub edit_mode: bool,
    /// First use: 0 style, 1 hotkey, 2 try it.
    first_step: u8,
    customize: bool,
    import: String,
    styled: Option<(os_overlay::Style, Instant)>,
}

/// Every frame: draw the overlay when it's shown.
pub fn overlay(mon: &mut Monitor, st: &mut State, ectx: &egui::Context) {
    let o = mon.settings.overlay.clone();
    if !o.shown {
        st.styled = None;
        return;
    }
    let tel = mon.tel();
    let lease = super::layout_view::prefixes(&o.layout.widgets);
    tel.want("overlay", &lease.iter().map(String::as_str).collect::<Vec<_>>());
    let series = tel.series_with(|_| false);
    let edit = st.edit_mode;
    let builder = egui::ViewportBuilder::default()
        .with_title(OVERLAY_TITLE)
        .with_always_on_top()
        .with_decorations(false)
        .with_resizable(false)
        .with_transparent(true)
        .with_taskbar(false)
        .with_active(false)
        .with_mouse_passthrough(!edit);
    let mut moved: Option<([f32; 2], [f32; 2], [f32; 2])> = None;
    let mut end_edit = false;
    ectx.show_viewport_immediate(egui::ViewportId::from_hash_of("zeez-live-overlay"), builder, |ui, _| {
        egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| {
            let drawn = super::layout_view::show(ui, &o.layout, &series, &|id| tel.points(id)).rect;
            if edit {
                let p = theme::palette(ui);
                ui.painter().rect_stroke(
                    drawn.expand(1.0),
                    egui::CornerRadius::same(4),
                    egui::Stroke::new(2.0, p.accent),
                    egui::StrokeKind::Inside,
                );
                let bg = ui.interact(ui.max_rect(), ui.id().with("drag"), egui::Sense::click_and_drag());
                if bg.drag_started() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    end_edit = true;
                }
            }
            let want = drawn.size() + egui::vec2(2.0, 2.0);
            if (ui.ctx().content_rect().size() - want).length() > 2.0 {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(want));
            }
            let (outer, screen, down) =
                ui.input(|i| (i.viewport().outer_rect, i.viewport().monitor_size, i.pointer.any_down()));
            if let (Some(r), Some(scr)) = (outer, screen) {
                let (size, scr) = ([r.width(), r.height()], [scr.x, scr.y]);
                if edit {
                    if !down {
                        moved = Some(([r.min.x, r.min.y], size, scr));
                    }
                } else {
                    // ponytail: placed on the monitor the window is on, from its
                    // origin, so the primary display; per-monitor origins need winit.
                    let pos = model::place(o.anchor, o.offset, size, scr);
                    if (pos[0] - r.min.x).abs() + (pos[1] - r.min.y).abs() > 1.0 {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos.into()));
                    }
                }
            }
        });
    });
    // Window styles: when they change, and every 2 s (a game going
    // fullscreen can push other topmost windows down).
    let want = os_overlay::Style { click_through: !edit, no_activate: true, exclude_from_capture: o.hide_from_capture };
    if st.styled.is_none_or(|(s, t)| s != want || t.elapsed() > Duration::from_secs(2))
        && (os_overlay::apply(OVERLAY_TITLE, want) || !cfg!(windows))
    {
        st.styled = Some((want, Instant::now()));
    }
    if let Some((pos, size, scr)) = moved {
        let anchor = model::nearest_anchor(pos, size, scr);
        let offset = model::offset_for(anchor, pos, size, scr).map(|v| v.round());
        if anchor != o.anchor || offset != o.offset {
            mon.set_setting(move |s| {
                s.overlay.anchor = anchor;
                s.overlay.offset = offset;
            });
        }
    }
    if end_edit {
        st.edit_mode = false;
    }
    ectx.request_repaint_after(Duration::from_secs_f32(1.0 / o.text_hz.clamp(1.0, 30.0)));
}

/// The ● LIVE indicator, wherever the overlay's state shows.
pub fn live_pill(ui: &mut Ui, shown: bool) {
    if shown {
        widgets::pill(ui, &format!("{} {}", icon::CIRCLE, tr("LIVE")), Tone::Success);
    }
}

impl State {
    pub fn page(&mut self, ui: &mut Ui, mon: &mut Monitor) {
        let shown = mon.settings.overlay.shown;
        widgets::page_header(ui, "ZeeZ Live", tr("Overlay, capture and reports while you play or work."), |ui| {
            let label = if shown { tr("Hide overlay") } else { tr("Show overlay") };
            if widgets::primary_button(ui, &format!("{} {label}", icon::MONITOR)).clicked() {
                mon.set_setting(move |s| s.overlay.shown = !shown);
            }
            live_pill(ui, shown);
        });
        widgets::segmented(
            ui,
            &mut self.tab,
            &[
                (Tab::Live, icon::PULSE, tr("Live")),
                (Tab::Overlay, icon::STACK, tr("Overlay")),
                (Tab::Sessions, icon::FILM_STRIP, tr("Sessions")),
            ],
        );
        ui.add_space(8.0);
        match self.tab {
            Tab::Live => self.live_tab(ui, mon),
            Tab::Overlay => {
                if mon.settings.overlay.first_use_done {
                    self.overlay_tab(ui, mon);
                } else {
                    self.first_use(ui, mon);
                }
            }
            Tab::Sessions => widgets::empty_state(
                ui,
                icon::FILM_STRIP,
                tr("No sessions yet"),
                tr("Captured sessions and their reports will be listed here."),
            ),
        }
    }

    fn live_tab(&mut self, ui: &mut Ui, mon: &mut Monitor) {
        let o = mon.settings.overlay.clone();
        widgets::card(ui, icon::MONITOR, tr("Overlay"), |ui| {
            ui.label(
                RichText::new(if o.shown {
                    tr("The overlay is showing. It never takes focus or clicks; hide it here or with its hotkey.")
                } else {
                    tr("The overlay is hidden.")
                })
                .color(theme::palette(ui).text_secondary),
            );
            let tel = mon.tel();
            let lease = super::layout_view::prefixes(&o.layout.widgets);
            tel.want("live-tab", &lease.iter().map(String::as_str).collect::<Vec<_>>());
            let series = tel.series_with(|_| false);
            ui.add_space(6.0);
            super::layout_view::show(ui, &o.layout, &series, &|id| tel.points(id));
        });
    }

    fn first_use(&mut self, ui: &mut Ui, mon: &mut Monitor) {
        let p = theme::palette(ui);
        let tjmax = mon.tjmax();
        let steps = [tr("Pick a style"), tr("Confirm the hotkey"), tr("Try it now")];
        ui.horizontal(|ui| {
            for (i, s) in steps.iter().enumerate() {
                let tone = if i as u8 == self.first_step { Tone::Accent } else { Tone::Neutral };
                widgets::pill(ui, &format!("{}. {s}", i + 1), tone);
            }
        });
        ui.add_space(8.0);
        match self.first_step {
            0 => {
                let current = mon.settings.overlay.style.clone();
                for l in presets::live::all(tjmax) {
                    let on = current.as_deref() == Some(l.name.as_str());
                    let title = tr(&l.name).to_string();
                    widgets::card(ui, if on { icon::CHECK_CIRCLE } else { icon::CIRCLE }, &title, |ui| {
                        let tel = mon.tel();
                        let lease = super::layout_view::prefixes(&l.widgets);
                        tel.want("live-styles", &lease.iter().map(String::as_str).collect::<Vec<_>>());
                        let series = tel.series_with(|_| false);
                        super::layout_view::show(ui, &l, &series, &|id| tel.points(id));
                        if ui.button(tr("Use this style")).clicked() {
                            let name = l.name.clone();
                            mon.set_setting(move |s| {
                                s.overlay.layout = l.clone();
                                s.overlay.style = Some(name.clone());
                            });
                            self.first_step = 1;
                        }
                    });
                }
            }
            1 => {
                ui.label(RichText::new(tr("Show or hide the overlay with this shortcut, from any app:")).color(p.text));
                ui.label(RichText::new("Ctrl+Shift+O").monospace().strong().size(18.0));
                ui.horizontal(|ui| {
                    if ui.button(tr("Back")).clicked() {
                        self.first_step = 0;
                    }
                    if widgets::primary_button(ui, tr("OK")).clicked() {
                        self.first_step = 2;
                    }
                });
            }
            _ => {
                ui.label(
                    RichText::new(tr(
                        "The overlay appears over your desktop now, so you see it before starting a game.",
                    ))
                    .color(p.text),
                );
                ui.horizontal(|ui| {
                    if ui.button(tr("Back")).clicked() {
                        self.first_step = 1;
                    }
                    if widgets::primary_button(ui, tr("Try it now")).clicked() {
                        mon.set_setting(|s| {
                            s.overlay.shown = true;
                            s.overlay.first_use_done = true;
                        });
                    }
                });
            }
        }
    }

    fn overlay_tab(&mut self, ui: &mut Ui, mon: &mut Monitor) {
        let p = theme::palette(ui);
        let tjmax = mon.tjmax();
        let mut o = mon.settings.overlay.clone();
        widgets::card(ui, icon::STACK, tr("Style"), |ui| {
            ui.horizontal_wrapped(|ui| {
                for l in presets::live::all(tjmax) {
                    if ui.selectable_label(o.style.as_deref() == Some(l.name.as_str()), tr(&l.name)).clicked() {
                        o.style = Some(l.name.clone());
                        o.layout = l;
                    }
                }
                if ui.selectable_label(self.customize, format!("{} {}", icon::PENCIL_SIMPLE, tr("Customize"))).clicked()
                {
                    self.customize = !self.customize;
                }
            });
            if self.customize {
                let before = o.layout.clone();
                egui::Frame::group(ui.style())
                    .show(ui, |ui| super::layout_editor::editor(ui, "overlay", &mut o.layout, &mut self.import, tjmax));
                if o.layout != before {
                    o.style = None;
                }
            }
        });
        widgets::card(ui, icon::ARROWS_OUT_CARDINAL, tr("Position"), |ui| {
            egui::Grid::new("overlay-anchors").spacing([4.0, 4.0]).show(ui, |ui| {
                for (i, a) in Anchor::ALL.into_iter().enumerate() {
                    if ui.selectable_label(o.anchor == a, tr(a.label())).clicked() {
                        o.anchor = a;
                        o.offset = [0.0, 0.0];
                    }
                    if i % 3 == 2 {
                        ui.end_row();
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label(tr("Offset"));
                ui.add(egui::DragValue::new(&mut o.offset[0]).range(-4000.0..=4000.0).suffix(" px"));
                ui.add(egui::DragValue::new(&mut o.offset[1]).range(-4000.0..=4000.0).suffix(" px"));
            });
            ui.horizontal(|ui| {
                let label = if self.edit_mode { tr("Done") } else { tr("Move with the mouse") };
                if ui.add_enabled(o.shown, egui::Button::new(format!("{} {label}", icon::HAND_GRABBING))).clicked() {
                    self.edit_mode = !self.edit_mode;
                }
                if self.edit_mode {
                    ui.label(
                        RichText::new(tr("Drag the overlay, then press Esc or Done.")).small().color(p.text_muted),
                    );
                }
            });
        });
        widgets::card(ui, icon::GEAR_SIX, tr("Behaviour"), |ui| {
            if cfg!(windows) {
                ui.checkbox(&mut o.hide_from_capture, tr("Hide from screenshots, recordings and streams"));
            }
            ui.horizontal(|ui| {
                ui.label(tr("Text refresh"));
                ui.add(egui::DragValue::new(&mut o.text_hz).range(1.0..=30.0).suffix(" Hz"));
                ui.label(tr("Graph refresh"));
                ui.add(egui::DragValue::new(&mut o.graph_hz).range(1.0..=60.0).suffix(" Hz"));
            });
            for note in limits() {
                ui.label(RichText::new(note).small().color(p.text_muted));
            }
        });
        if o != mon.settings.overlay {
            mon.set_setting(move |s| s.overlay = o.clone());
        }
    }
}

/// What the overlay can't do on this OS (zeez-live §8), said up front.
fn limits() -> Vec<&'static str> {
    let mut v = Vec::new();
    if cfg!(windows) {
        v.push(tr(
            "Games in true exclusive fullscreen draw over every window: switch them to borderless to see the overlay.",
        ));
    }
    if cfg!(target_os = "linux") {
        v.push(tr("On Wayland, apps can't place themselves above others: there the overlay may stay behind games."));
    }
    if cfg!(target_os = "macos") {
        v.push(tr("Over fullscreen apps (their own Space) the overlay isn't shown yet."));
    }
    v
}

pub const PT_BR: &[(&str, &str)] = &[
    ("Minimal", "Mínimo"),
    ("Benchmark", "Benchmark"),
    ("Overclocker", "Overclocker"),
    ("Frames", "Quadros"),
    ("Frame time", "Tempo de quadro"),
    ("Busiest core", "Núcleo mais ocupado"),
    ("GPU Busy", "GPU ocupada"),
    ("1% low", "1% mínimo"),
    ("0.1% low", "0,1% mínimo"),
    ("GPU clock", "Clock da GPU"),
    ("VID", "VID"),
    ("GPU power", "Energia da GPU"),
    ("RAM", "RAM"),
    ("ZeeZ Live", "ZeeZ Live"),
    (
        "Overlay, capture and reports while you play or work.",
        "Overlay, captura e relatórios enquanto você joga ou trabalha.",
    ),
    ("Show overlay", "Mostrar overlay"),
    ("Hide overlay", "Ocultar overlay"),
    ("LIVE", "AO VIVO"),
    ("Live", "Ao vivo"),
    ("Overlay", "Overlay"),
    ("Sessions", "Sessões"),
    ("No sessions yet", "Nenhuma sessão ainda"),
    (
        "Captured sessions and their reports will be listed here.",
        "As sessões capturadas e seus relatórios aparecem aqui.",
    ),
    (
        "The overlay is showing. It never takes focus or clicks; hide it here or with its hotkey.",
        "O overlay está visível. Ele nunca pega o foco nem os cliques; oculte aqui ou pelo atalho.",
    ),
    ("The overlay is hidden.", "O overlay está oculto."),
    ("Pick a style", "Escolha um estilo"),
    ("Confirm the hotkey", "Confirme o atalho"),
    ("Try it now", "Experimente agora"),
    ("Use this style", "Usar este estilo"),
    (
        "Show or hide the overlay with this shortcut, from any app:",
        "Mostre ou oculte o overlay com este atalho, de qualquer app:",
    ),
    ("Back", "Voltar"),
    ("OK", "OK"),
    (
        "The overlay appears over your desktop now, so you see it before starting a game.",
        "O overlay aparece agora sobre a área de trabalho, para você vê-lo antes de abrir um jogo.",
    ),
    ("Style", "Estilo"),
    ("Customize", "Personalizar"),
    ("Position", "Posição"),
    ("Top left", "Superior esquerdo"),
    ("Top", "Superior"),
    ("Top right", "Superior direito"),
    ("Left", "Esquerda"),
    ("Center", "Centro"),
    ("Right", "Direita"),
    ("Bottom left", "Inferior esquerdo"),
    ("Bottom", "Inferior"),
    ("Bottom right", "Inferior direito"),
    ("Offset", "Deslocamento"),
    ("Move with the mouse", "Mover com o mouse"),
    ("Drag the overlay, then press Esc or Done.", "Arraste o overlay e depois pressione Esc ou Concluir."),
    ("Behaviour", "Comportamento"),
    ("Hide from screenshots, recordings and streams", "Ocultar de capturas de tela, gravações e transmissões"),
    ("Text refresh", "Atualização do texto"),
    ("Graph refresh", "Atualização dos gráficos"),
    (
        "Games in true exclusive fullscreen draw over every window: switch them to borderless to see the overlay.",
        "Jogos em tela cheia exclusiva de verdade desenham sobre qualquer janela: mude para sem bordas para ver o overlay.",
    ),
    (
        "On Wayland, apps can't place themselves above others: there the overlay may stay behind games.",
        "No Wayland, apps não podem se colocar acima de outros: lá o overlay pode ficar atrás dos jogos.",
    ),
    (
        "Over fullscreen apps (their own Space) the overlay isn't shown yet.",
        "Sobre apps em tela cheia (no próprio Space) o overlay ainda não aparece.",
    ),
];

pub const ES: &[(&str, &str)] = &[
    ("Minimal", "Mínimo"),
    ("Benchmark", "Benchmark"),
    ("Overclocker", "Overclocker"),
    ("Frames", "Fotogramas"),
    ("Frame time", "Tiempo de fotograma"),
    ("Busiest core", "Núcleo más ocupado"),
    ("GPU Busy", "GPU ocupada"),
    ("1% low", "1% mínimo"),
    ("0.1% low", "0,1% mínimo"),
    ("GPU clock", "Reloj de GPU"),
    ("VID", "VID"),
    ("GPU power", "Potencia de GPU"),
    ("RAM", "RAM"),
    ("ZeeZ Live", "ZeeZ Live"),
    ("Overlay, capture and reports while you play or work.", "Overlay, captura e informes mientras juegas o trabajas."),
    ("Show overlay", "Mostrar overlay"),
    ("Hide overlay", "Ocultar overlay"),
    ("LIVE", "EN VIVO"),
    ("Live", "En vivo"),
    ("Overlay", "Overlay"),
    ("Sessions", "Sesiones"),
    ("No sessions yet", "Aún no hay sesiones"),
    (
        "Captured sessions and their reports will be listed here.",
        "Las sesiones capturadas y sus informes aparecen aquí.",
    ),
    (
        "The overlay is showing. It never takes focus or clicks; hide it here or with its hotkey.",
        "El overlay está visible. Nunca toma el foco ni los clics; ocúltalo aquí o con su atajo.",
    ),
    ("The overlay is hidden.", "El overlay está oculto."),
    ("Pick a style", "Elige un estilo"),
    ("Confirm the hotkey", "Confirma el atajo"),
    ("Try it now", "Pruébalo ahora"),
    ("Use this style", "Usar este estilo"),
    (
        "Show or hide the overlay with this shortcut, from any app:",
        "Muestra u oculta el overlay con este atajo, desde cualquier app:",
    ),
    ("Back", "Atrás"),
    ("OK", "Aceptar"),
    (
        "The overlay appears over your desktop now, so you see it before starting a game.",
        "El overlay aparece ahora sobre el escritorio, para que lo veas antes de abrir un juego.",
    ),
    ("Style", "Estilo"),
    ("Customize", "Personalizar"),
    ("Position", "Posición"),
    ("Top left", "Arriba a la izquierda"),
    ("Top", "Arriba"),
    ("Top right", "Arriba a la derecha"),
    ("Left", "Izquierda"),
    ("Center", "Centro"),
    ("Right", "Derecha"),
    ("Bottom left", "Abajo a la izquierda"),
    ("Bottom", "Abajo"),
    ("Bottom right", "Abajo a la derecha"),
    ("Offset", "Desplazamiento"),
    ("Move with the mouse", "Mover con el ratón"),
    ("Drag the overlay, then press Esc or Done.", "Arrastra el overlay y luego pulsa Esc o Listo."),
    ("Behaviour", "Comportamiento"),
    ("Hide from screenshots, recordings and streams", "Ocultar de capturas, grabaciones y transmisiones"),
    ("Text refresh", "Actualización del texto"),
    ("Graph refresh", "Actualización de gráficos"),
    (
        "Games in true exclusive fullscreen draw over every window: switch them to borderless to see the overlay.",
        "Los juegos en pantalla completa exclusiva real dibujan sobre cualquier ventana: cámbialos a sin bordes para ver el overlay.",
    ),
    (
        "On Wayland, apps can't place themselves above others: there the overlay may stay behind games.",
        "En Wayland, las apps no pueden colocarse sobre otras: ahí el overlay puede quedar detrás de los juegos.",
    ),
    (
        "Over fullscreen apps (their own Space) the overlay isn't shown yet.",
        "Sobre apps a pantalla completa (en su propio Space) el overlay aún no se muestra.",
    ),
];
