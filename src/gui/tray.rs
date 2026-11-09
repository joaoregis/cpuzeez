//! Tray / menu-bar icons (S1-M01): one icon per chosen metric (like HWiNFO),
//! each drawn as a number, bar, mini graph or ring at the real icon size
//! (`trayicon`), coloured by the Widget Engine's thresholds with a shape cue,
//! a several-line tooltip, a menu rebuilt when its state or the language
//! changes, and a configurable left click. macOS gets one menu-bar item whose
//! text lists every chosen metric (`CPU 62° · GPU 55°`).
//!
//! Windows and macOS use `tray-icon`; Linux uses `ksni` (StatusNotifierItem
//! over D-Bus, pure Rust). Where no tray host exists (GNOME without the
//! AppIndicator extension) the caller falls back to the mini monitor and
//! explains why. Windows 11 puts new icons in the overflow menu, which an app
//! can't change; Settings says how to pin them.

use super::i18n::tr;
use super::trayicon::{self, Picture, Style};
use eframe::egui;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

pub const PT_BR: &[(&str, &str)] = &[
    ("Show CPU ZeeZ", "Mostrar CPU ZeeZ"),
    ("Hide CPU ZeeZ", "Ocultar CPU ZeeZ"),
    ("Mini monitor", "Mini monitor"),
    ("Sensors", "Sensores"),
    ("Pause monitoring", "Pausar monitoramento"),
    ("Resume monitoring", "Retomar monitoramento"),
    ("Update every", "Atualizar a cada"),
    ("Lock mini monitors", "Travar mini monitores"),
    ("Quit", "Sair"),
];
pub const ES: &[(&str, &str)] = &[
    ("Show CPU ZeeZ", "Mostrar CPU ZeeZ"),
    ("Hide CPU ZeeZ", "Ocultar CPU ZeeZ"),
    ("Mini monitor", "Minimonitor"),
    ("Sensors", "Sensores"),
    ("Pause monitoring", "Pausar la supervisión"),
    ("Resume monitoring", "Reanudar la supervisión"),
    ("Update every", "Actualizar cada"),
    ("Lock mini monitors", "Bloquear minimonitores"),
    ("Quit", "Salir"),
];

const REFRESH: Duration = Duration::from_secs(2);
/// Sampling periods offered in the menu (ms).
pub const SPEEDS: [u64; 4] = [500, 1000, 2000, 5000];
/// Windows' tooltip holds 127 characters.
const TIP_MAX: usize = 127;

pub use cpuzeez_core::layout::tray::{Click, Item};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Show or hide the main window.
    Toggle,
    Mini,
    Sensors,
    Pause,
    Lock,
    Speed(u64),
    Quit,
}

/// What the menu shows; rebuilt when any of it changes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MenuState {
    pub visible: bool,
    pub mini: bool,
    pub paused: bool,
    pub speed_ms: u64,
    /// Some(locked) while a mini monitor is open.
    pub lock: Option<bool>,
}

/// Menu/click events from the tray backends (their callbacks run on other
/// threads or outside our frame), drained by `Tray::poll`.
static EVENTS: Mutex<Vec<Action>> = Mutex::new(Vec::new());
static WAKE: Mutex<Option<egui::Context>> = Mutex::new(None);
static CLICK: AtomicU8 = AtomicU8::new(0);

fn push(a: Action) {
    EVENTS.lock().unwrap_or_else(|e| e.into_inner()).push(a);
    if let Some(c) = WAKE.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        c.request_repaint();
    }
}

fn click() {
    push(if CLICK.load(Ordering::Relaxed) == 1 { Action::Mini } else { Action::Toggle });
}

fn event(id: &str) {
    match id {
        "toggle" => push(Action::Toggle),
        "mini" => push(Action::Mini),
        "sensors" => push(Action::Sensors),
        "pause" => push(Action::Pause),
        "lock" => push(Action::Lock),
        "quit" => push(Action::Quit),
        s => {
            if let Some(ms) = s.strip_prefix("speed-").and_then(|v| v.parse().ok()) {
                push(Action::Speed(ms));
            }
        }
    }
}

/// (id, label, checked) rows; None = separator; Some(children) for the speed submenu.
#[derive(Debug, Clone, PartialEq)]
pub enum Row {
    Item { id: String, label: String, checked: Option<bool> },
    Sub { label: String, items: Vec<Row> },
    Separator,
}

/// The menu for `s` in the current language.
pub fn menu(s: &MenuState) -> Vec<Row> {
    let item = |id: &str, label: &str, checked: Option<bool>| Row::Item { id: id.into(), label: label.into(), checked };
    let speeds = SPEEDS
        .iter()
        .map(|&ms| {
            let label = if ms < 1000 { format!("{ms} ms") } else { format!("{} s", ms / 1000) };
            item(&format!("speed-{ms}"), &label, Some(ms == s.speed_ms))
        })
        .collect();
    let mut rows = vec![
        item("toggle", tr(if s.visible { "Hide CPU ZeeZ" } else { "Show CPU ZeeZ" }), None),
        item("mini", tr("Mini monitor"), Some(s.mini)),
    ];
    if let Some(l) = s.lock {
        rows.push(item("lock", tr("Lock mini monitors"), Some(l)));
    }
    rows.extend([
        item("sensors", tr("Sensors"), None),
        Row::Separator,
        item("pause", tr(if s.paused { "Resume monitoring" } else { "Pause monitoring" }), None),
        Row::Sub { label: tr("Update every").into(), items: speeds },
        Row::Separator,
        item("quit", tr("Quit"), None),
    ]);
    rows
}

/// One icon's picture and tooltip line, ready to show.
pub struct Shown {
    pub rgba: Vec<u8>,
    pub line: String,
    /// The value alone ("62°"), for the macOS menu-bar text.
    pub short: String,
}

#[derive(Default)]
pub struct Tray {
    icons: Vec<imp::Imp>,
    count: usize,
    last: Option<Instant>,
    menu: Option<(MenuState, Vec<Row>)>,
    pub error: Option<String>,
}

impl Tray {
    pub fn active(&self) -> bool {
        !self.icons.is_empty()
    }

    /// Create or remove the icons (`n` of them). Call from the UI (main) thread.
    pub fn set_enabled(&mut self, on: bool, n: usize, click: Click, ectx: &egui::Context) {
        CLICK.store(u8::from(click == Click::Mini), Ordering::Relaxed);
        let n = if cfg!(target_os = "macos") { 1 } else { n.clamp(1, 8) };
        if !on {
            self.icons.clear();
            return;
        }
        if self.icons.len() == n {
            return;
        }
        *WAKE.lock().unwrap_or_else(|e| e.into_inner()) = Some(ectx.clone());
        self.icons.clear();
        let rgba = trayicon::draw(None, &placeholder(), trayicon::size());
        let rows = menu(&MenuState::default());
        for i in 0..n {
            match imp::Imp::new(i, &rgba, trayicon::size(), &rows) {
                Ok(t) => self.icons.push(t),
                Err(e) => {
                    eprintln!("tray icon unavailable: {e}");
                    self.error = Some(e);
                    self.icons.clear();
                    return;
                }
            }
        }
        (self.error, self.last, self.menu, self.count) = (None, None, None, n);
    }

    /// Refresh pictures, tooltips and menus (pictures every couple of seconds;
    /// the menu as soon as its state or the language changes).
    pub fn update(&mut self, shown: &[Shown], tip_extra: &[String], state: &MenuState) {
        if self.icons.is_empty() {
            return;
        }
        let rows = menu(state);
        if self.menu.as_ref().is_none_or(|(s, r)| s != state || r != &rows) {
            for i in &mut self.icons {
                i.set_menu(&rows);
            }
            self.menu = Some((state.clone(), rows));
        }
        if self.last.is_some_and(|t| t.elapsed() < REFRESH) {
            return;
        }
        self.last = Some(Instant::now());
        let mut tip: Vec<String> = tip_extra.to_vec();
        tip.extend(shown.iter().map(|s| s.line.clone()));
        let tip = clip(&tip.join("\n"));
        let title: Vec<&str> = shown.iter().map(|s| s.short.as_str()).collect();
        for (i, icon) in self.icons.iter_mut().enumerate() {
            if let Some(s) = shown.get(i) {
                icon.update(&s.rgba, trayicon::size(), &tip, &title.join(" · "));
            }
        }
    }

    pub fn poll(&mut self) -> Vec<Action> {
        std::mem::take(&mut *EVENTS.lock().unwrap_or_else(|e| e.into_inner()))
    }
}

fn placeholder() -> Picture<'static> {
    Picture { style: Style::Number, text: None, fill: 0.0, tone: None, history: &[] }
}

/// Fits Windows' tooltip limit without cutting a character in half.
fn clip(s: &str) -> String {
    if s.len() <= TIP_MAX {
        return s.to_string();
    }
    let mut end = TIP_MAX - 1;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

#[cfg(any(windows, target_os = "macos"))]
mod imp {
    use super::{Row, click, event};
    use tray_icon::menu::{CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
    use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

    pub struct Imp {
        icon: TrayIcon,
    }

    fn icon(rgba: &[u8], n: usize) -> Option<Icon> {
        Icon::from_rgba(rgba.to_vec(), n as u32, n as u32).ok()
    }

    fn items(rows: &[Row]) -> Vec<Box<dyn IsMenuItem>> {
        rows.iter()
            .map(|r| -> Box<dyn IsMenuItem> {
                match r {
                    Row::Item { id, label, checked: None } => {
                        Box::new(MenuItem::with_id(id.as_str(), label, true, None))
                    }
                    Row::Item { id, label, checked: Some(c) } => {
                        Box::new(CheckMenuItem::with_id(id.as_str(), label, true, *c, None))
                    }
                    Row::Sub { label, items: sub } => {
                        let s = Submenu::new(label, true);
                        for i in items(sub) {
                            let _ = s.append(i.as_ref());
                        }
                        Box::new(s)
                    }
                    Row::Separator => Box::new(PredefinedMenuItem::separator()),
                }
            })
            .collect()
    }

    fn build(rows: &[Row]) -> Menu {
        let m = Menu::new();
        for i in items(rows) {
            let _ = m.append(i.as_ref());
        }
        m
    }

    impl Imp {
        pub fn new(i: usize, rgba: &[u8], n: usize, rows: &[Row]) -> Result<Imp, String> {
            // The handlers live in process-wide OnceCells: install them once.
            static HANDLERS: std::sync::Once = std::sync::Once::new();
            HANDLERS.call_once(|| {
                MenuEvent::set_event_handler(Some(|e: MenuEvent| event(e.id.0.as_str())));
                TrayIconEvent::set_event_handler(Some(|e: TrayIconEvent| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left, button_state: MouseButtonState::Up, ..
                    } = e
                    {
                        // macOS opens the menu on left click instead.
                        if cfg!(windows) {
                            click();
                        }
                    }
                }));
            });
            let icon = TrayIconBuilder::new()
                .with_id(format!("cpuzeez-{i}"))
                .with_menu(Box::new(build(rows)))
                .with_tooltip(crate::DISPLAY_NAME)
                .with_icon(icon(rgba, n).ok_or("icon")?)
                .build()
                .map_err(|e| e.to_string())?;
            Ok(Imp { icon })
        }

        pub fn set_menu(&mut self, rows: &[Row]) {
            self.icon.set_menu(Some(Box::new(build(rows))));
        }

        pub fn update(&mut self, rgba: &[u8], n: usize, tip: &str, title: &str) {
            let _ = self.icon.set_icon(icon(rgba, n));
            let _ = self.icon.set_tooltip(Some(tip));
            // macOS menu bar: every chosen metric as text next to the icon.
            #[cfg(target_os = "macos")]
            self.icon.set_title(Some(title));
            let _ = title;
        }
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod imp {
    use super::{Row, click, event};
    use ksni::blocking::{Handle, TrayMethods};

    struct Sni {
        i: usize,
        n: usize,
        argb: Vec<u8>,
        tip: String,
        rows: Vec<Row>,
    }

    fn items(rows: &[Row]) -> Vec<ksni::MenuItem<Sni>> {
        use ksni::menu::{CheckmarkItem, StandardItem, SubMenu};
        rows.iter()
            .map(|r| match r {
                Row::Item { id, label, checked: None } => {
                    let id = id.clone();
                    StandardItem {
                        label: label.clone(),
                        activate: Box::new(move |_: &mut Sni| event(&id)),
                        ..Default::default()
                    }
                    .into()
                }
                Row::Item { id, label, checked: Some(c) } => {
                    let id = id.clone();
                    CheckmarkItem {
                        label: label.clone(),
                        checked: *c,
                        activate: Box::new(move |_: &mut Sni| event(&id)),
                        ..Default::default()
                    }
                    .into()
                }
                Row::Sub { label, items: sub } => {
                    SubMenu { label: label.clone(), submenu: items(sub), ..Default::default() }.into()
                }
                Row::Separator => ksni::MenuItem::Separator,
            })
            .collect()
    }

    impl ksni::Tray for Sni {
        fn id(&self) -> String {
            format!("{}-{}", crate::NAME, self.i)
        }
        fn title(&self) -> String {
            crate::DISPLAY_NAME.into()
        }
        fn icon_pixmap(&self) -> Vec<ksni::Icon> {
            vec![ksni::Icon { width: self.n as i32, height: self.n as i32, data: self.argb.clone() }]
        }
        fn tool_tip(&self) -> ksni::ToolTip {
            ksni::ToolTip { title: crate::DISPLAY_NAME.into(), description: self.tip.clone(), ..Default::default() }
        }
        fn activate(&mut self, _x: i32, _y: i32) {
            click();
        }
        fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
            items(&self.rows)
        }
    }

    /// RGBA -> ARGB32 in network byte order.
    fn argb(rgba: &[u8]) -> Vec<u8> {
        rgba.as_chunks::<4>().0.iter().flat_map(|p| [p[3], p[0], p[1], p[2]]).collect()
    }

    pub struct Imp {
        handle: Handle<Sni>,
    }

    impl Imp {
        pub fn new(i: usize, rgba: &[u8], n: usize, rows: &[Row]) -> Result<Imp, String> {
            let sni = Sni { i, n, argb: argb(rgba), tip: String::new(), rows: rows.to_vec() };
            let handle = sni.spawn().map_err(|e| e.to_string())?;
            Ok(Imp { handle })
        }

        pub fn set_menu(&mut self, rows: &[Row]) {
            let rows = rows.to_vec();
            self.handle.update(move |s| s.rows = rows);
        }

        pub fn update(&mut self, rgba: &[u8], n: usize, tip: &str, _title: &str) {
            let (a, t) = (argb(rgba), tip.to_string());
            self.handle.update(move |s| {
                s.argb = a;
                s.n = n;
                s.tip = t;
            });
        }
    }

    impl Drop for Imp {
        fn drop(&mut self) {
            let _ = self.handle.shutdown();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_follows_its_state() {
        let s = MenuState { visible: true, mini: false, paused: false, speed_ms: 1000, lock: None };
        let m = menu(&s);
        assert!(matches!(&m[0], Row::Item { label, .. } if label == "Hide CPU ZeeZ"));
        let Row::Sub { items, .. } = &m[5] else { panic!("speed submenu") };
        assert!(items.iter().any(|r| matches!(r, Row::Item { id, checked: Some(true), .. } if id == "speed-1000")));
        let hidden = menu(&MenuState { visible: false, paused: true, ..s });
        assert!(matches!(&hidden[0], Row::Item { label, .. } if label == "Show CPU ZeeZ"));
        assert!(matches!(&hidden[4], Row::Item { label, .. } if label == "Resume monitoring"));
    }

    #[test]
    fn events_and_tooltip() {
        event("speed-2000");
        event("toggle");
        CLICK.store(1, Ordering::Relaxed);
        click();
        CLICK.store(0, Ordering::Relaxed);
        let got = std::mem::take(&mut *EVENTS.lock().unwrap());
        for a in [Action::Speed(2000), Action::Toggle, Action::Mini] {
            assert!(got.contains(&a), "{got:?}");
        }
        let long = "CPU 62 °C\n".repeat(30);
        let c = clip(&long);
        assert!(c.len() <= TIP_MAX + 3 && c.ends_with('…'));
        assert_eq!(clip("short"), "short");
    }
}
