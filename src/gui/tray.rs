//! Tray / menu-bar mini monitor: an icon showing the CPU temperature digits
//! (drawn into RGBA here) with a Show / Sensors / Quit menu. Windows and macOS
//! use `tray-icon`; Linux uses `ksni` (StatusNotifierItem over D-Bus, pure
//! Rust). Where no tray host exists (e.g. GNOME without the AppIndicator
//! extension) the caller falls back to the always-on-top mini monitor window.

use super::i18n::tr;
use eframe::egui;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub const PT_BR: &[(&str, &str)] = &[("Show CpuZeeZ", "Mostrar CpuZeeZ"), ("Sensors", "Sensores"), ("Quit", "Sair")];
pub const ES: &[(&str, &str)] = &[("Show CpuZeeZ", "Mostrar CpuZeeZ"), ("Sensors", "Sensores"), ("Quit", "Salir")];

const ICON: usize = 32;
const REFRESH: Duration = Duration::from_secs(3);
/// Icon background by temperature: below warn, warn (Tjmax-10), hot (Tjmax), unknown.
const BG_OK: [u8; 4] = [0x2e, 0x7d, 0x32, 255];
const BG_WARN: [u8; 4] = [0xef, 0x6c, 0x00, 255];
const BG_HOT: [u8; 4] = [0xc6, 0x28, 0x28, 255];
const BG_NONE: [u8; 4] = [0x45, 0x5a, 0x64, 255];
const FG: [u8; 4] = [255, 255, 255, 255];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Show,
    Sensors,
    Quit,
}

/// Menu/click events from the tray backends (their callbacks run on other
/// threads or outside our frame), drained by `Tray::poll`.
static EVENTS: Mutex<Vec<Action>> = Mutex::new(Vec::new());
static WAKE: Mutex<Option<egui::Context>> = Mutex::new(None);

fn push(a: Action) {
    EVENTS.lock().unwrap().push(a);
    if let Some(c) = WAKE.lock().unwrap().as_ref() {
        c.request_repaint();
    }
}

#[derive(Default)]
pub struct Tray {
    imp: Option<imp::Imp>,
    last: Option<Instant>,
    pub error: Option<String>,
}

impl Tray {
    pub fn active(&self) -> bool {
        self.imp.is_some()
    }

    /// Create or remove the icon. Call from the UI (main) thread.
    pub fn set_enabled(&mut self, on: bool, ectx: &egui::Context) {
        if !on {
            self.imp = None;
            return;
        }
        if self.imp.is_some() {
            return;
        }
        *WAKE.lock().unwrap() = Some(ectx.clone());
        match imp::Imp::new(&icon_rgba(None, 95.0)) {
            Ok(i) => {
                self.imp = Some(i);
                self.error = None;
                self.last = None;
            }
            Err(e) => {
                eprintln!("tray icon unavailable: {e}");
                self.error = Some(e);
            }
        }
    }

    /// Refresh icon/tooltip (rate-limited to every few seconds).
    pub fn update(&mut self, cpu_temp: Option<f64>, tjmax: f64, tip: &str) {
        let Some(i) = &mut self.imp else { return };
        if self.last.is_some_and(|t| t.elapsed() < REFRESH) {
            return;
        }
        self.last = Some(Instant::now());
        i.update(&icon_rgba(cpu_temp, tjmax), cpu_temp, tip);
    }

    pub fn poll(&mut self) -> Vec<Action> {
        std::mem::take(&mut *EVENTS.lock().unwrap())
    }
}

/// 3×5 digit glyphs, one row per u8 (bit 2 = left column).
const DIGITS: [[u8; 5]; 10] = [
    [7, 5, 5, 5, 7],
    [2, 6, 2, 2, 7],
    [7, 1, 7, 4, 7],
    [7, 1, 7, 1, 7],
    [5, 5, 7, 1, 1],
    [7, 4, 7, 1, 7],
    [7, 4, 7, 5, 7],
    [7, 1, 1, 1, 1],
    [7, 5, 7, 5, 7],
    [7, 5, 7, 1, 7],
];
const DASH: [u8; 5] = [0, 0, 7, 0, 0];

/// 32×32 RGBA: rounded square colored by temperature with the °C digits.
pub fn icon_rgba(temp: Option<f64>, tjmax: f64) -> Vec<u8> {
    let bg = match temp {
        None => BG_NONE,
        Some(t) if t >= tjmax => BG_HOT,
        Some(t) if t >= tjmax - 10.0 => BG_WARN,
        Some(_) => BG_OK,
    };
    let glyphs: Vec<[u8; 5]> = match temp {
        Some(t) => format!("{:.0}", t.clamp(0.0, 999.0)).bytes().map(|b| DIGITS[(b - b'0') as usize]).collect(),
        None => vec![DASH, DASH],
    };
    let cols = glyphs.len() * 4 - 1;
    let scale = (ICON / cols).clamp(1, ICON / 5);
    let (w, h) = (cols * scale, 5 * scale);
    let (ox, oy) = ((ICON - w) / 2, (ICON - h) / 2);
    let mut px = vec![0u8; ICON * ICON * 4];
    for y in 0..ICON {
        for x in 0..ICON {
            // Clip the 4 corners for a rounded look.
            let (dx, dy) = (x.min(ICON - 1 - x), y.min(ICON - 1 - y));
            if dx + dy < 3 {
                continue;
            }
            let mut c = bg;
            if (ox..ox + w).contains(&x) && (oy..oy + h).contains(&y) {
                let (cx, cy) = ((x - ox) / scale, (y - oy) / scale);
                let (g, gx) = (cx / 4, cx % 4);
                if gx < 3 && glyphs[g][cy] >> (2 - gx) & 1 == 1 {
                    c = FG;
                }
            }
            px[(y * ICON + x) * 4..][..4].copy_from_slice(&c);
        }
    }
    px
}

#[cfg(any(windows, target_os = "macos"))]
mod imp {
    use super::{Action, ICON, push, tr};
    use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
    use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

    pub struct Imp {
        icon: TrayIcon,
    }

    fn icon(rgba: &[u8]) -> Option<Icon> {
        Icon::from_rgba(rgba.to_vec(), ICON as u32, ICON as u32).ok()
    }

    impl Imp {
        pub fn new(rgba: &[u8]) -> Result<Imp, String> {
            // The handlers live in process-wide OnceCells: install them once.
            static HANDLERS: std::sync::Once = std::sync::Once::new();
            HANDLERS.call_once(|| {
                MenuEvent::set_event_handler(Some(|e: MenuEvent| match e.id.0.as_str() {
                    "show" => push(Action::Show),
                    "sensors" => push(Action::Sensors),
                    "quit" => push(Action::Quit),
                    _ => {}
                }));
                TrayIconEvent::set_event_handler(Some(|e: TrayIconEvent| {
                    if let TrayIconEvent::DoubleClick { button: MouseButton::Left, .. }
                    | TrayIconEvent::Click {
                        button: MouseButton::Left, button_state: MouseButtonState::Up, ..
                    } = e
                    {
                        // macOS opens the menu on left click instead.
                        if cfg!(windows) {
                            push(Action::Show);
                        }
                    }
                }));
            });
            let menu = Menu::new();
            menu.append_items(&[
                &MenuItem::with_id("show", tr("Show CpuZeeZ"), true, None),
                &MenuItem::with_id("sensors", tr("Sensors"), true, None),
                &PredefinedMenuItem::separator(),
                &MenuItem::with_id("quit", tr("Quit"), true, None),
            ])
            .map_err(|e| e.to_string())?;
            let icon = TrayIconBuilder::new()
                .with_menu(Box::new(menu))
                .with_tooltip(crate::DISPLAY_NAME)
                .with_icon(icon(rgba).ok_or("icon")?)
                .build()
                .map_err(|e| e.to_string())?;
            Ok(Imp { icon })
        }

        pub fn update(&mut self, rgba: &[u8], temp: Option<f64>, tip: &str) {
            let _ = self.icon.set_icon(icon(rgba));
            let _ = self.icon.set_tooltip(Some(tip));
            // macOS menu bar: show the temperature as text next to the icon too.
            #[cfg(target_os = "macos")]
            self.icon.set_title(temp.map(|t| format!("{t:.0}°")));
            let _ = temp;
        }
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod imp {
    use super::{Action, ICON, push, tr};
    use ksni::blocking::{Handle, TrayMethods};

    struct Sni {
        argb: Vec<u8>,
        tip: String,
    }

    impl ksni::Tray for Sni {
        fn id(&self) -> String {
            crate::NAME.into()
        }
        fn title(&self) -> String {
            crate::DISPLAY_NAME.into()
        }
        fn icon_pixmap(&self) -> Vec<ksni::Icon> {
            vec![ksni::Icon { width: ICON as i32, height: ICON as i32, data: self.argb.clone() }]
        }
        fn tool_tip(&self) -> ksni::ToolTip {
            ksni::ToolTip { title: crate::DISPLAY_NAME.into(), description: self.tip.clone(), ..Default::default() }
        }
        fn activate(&mut self, _x: i32, _y: i32) {
            push(Action::Show);
        }
        fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
            use ksni::menu::StandardItem;
            let item = |label: &str, a: Action| {
                StandardItem {
                    label: label.into(),
                    activate: Box::new(move |_: &mut Self| push(a)),
                    ..Default::default()
                }
                .into()
            };
            vec![
                item(tr("Show CpuZeeZ"), Action::Show),
                item(tr("Sensors"), Action::Sensors),
                ksni::MenuItem::Separator,
                item(tr("Quit"), Action::Quit),
            ]
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
        pub fn new(rgba: &[u8]) -> Result<Imp, String> {
            let handle = Sni { argb: argb(rgba), tip: String::new() }.spawn().map_err(|e| e.to_string())?;
            Ok(Imp { handle })
        }

        pub fn update(&mut self, rgba: &[u8], _temp: Option<f64>, tip: &str) {
            let (a, t) = (argb(rgba), tip.to_string());
            self.handle.update(move |s| {
                s.argb = a;
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
    #[test]
    fn icon_digits() {
        let px = super::icon_rgba(Some(71.0), 95.0);
        assert_eq!(px.len(), 32 * 32 * 4);
        assert!(px.chunks(4).any(|p| p == super::FG));
        assert!(px.chunks(4).any(|p| p == super::BG_OK));
        assert!(super::icon_rgba(Some(96.0), 95.0).chunks(4).any(|p| p == super::BG_HOT));
        assert_eq!(super::icon_rgba(None, 95.0).len(), 32 * 32 * 4);
    }
}
