//! ZeeZ Live overlay settings and placement (S2b-01, docs/v1/zeez-live.md
//! §3.2, §5.3). Pure: where the HUD goes on a monitor, and what the user
//! chose. The window itself is in the GUI; the per-OS window tweaks are in
//! `platform::overlay`.

use crate::layout::Layout;
use serde::{Deserialize, Serialize};

/// The nine places the overlay can sit, plus an offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Anchor {
    #[default]
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Anchor {
    pub const ALL: [Anchor; 9] = [
        Anchor::TopLeft,
        Anchor::Top,
        Anchor::TopRight,
        Anchor::Left,
        Anchor::Center,
        Anchor::Right,
        Anchor::BottomLeft,
        Anchor::Bottom,
        Anchor::BottomRight,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Anchor::TopLeft => "Top left",
            Anchor::Top => "Top",
            Anchor::TopRight => "Top right",
            Anchor::Left => "Left",
            Anchor::Center => "Center",
            Anchor::Right => "Right",
            Anchor::BottomLeft => "Bottom left",
            Anchor::Bottom => "Bottom",
            Anchor::BottomRight => "Bottom right",
        }
    }

    /// (column, row) in 0..=2.
    fn cell(self) -> (f32, f32) {
        let i = Anchor::ALL.iter().position(|a| *a == self).unwrap_or(0);
        ((i % 3) as f32, (i / 3) as f32)
    }
}

/// Margin from the screen edge at offset 0 (points).
pub const EDGE: f32 = 12.0;

/// Top-left position of an overlay of `size` on a monitor of `screen`,
/// anchored at `a` and moved by `offset` (towards the screen's inside for
/// positive values on edge anchors).
pub fn place(a: Anchor, offset: [f32; 2], size: [f32; 2], screen: [f32; 2]) -> [f32; 2] {
    let (c, r) = a.cell();
    let axis = |cell: f32, size: f32, screen: f32| -> f32 {
        match cell as u8 {
            0 => EDGE,
            1 => (screen - size) / 2.0,
            _ => screen - size - EDGE,
        }
    };
    let dir = |cell: f32| if cell as u8 == 2 { -1.0 } else { 1.0 };
    let x = axis(c, size[0], screen[0]) + dir(c) * offset[0];
    let y = axis(r, size[1], screen[1]) + dir(r) * offset[1];
    // Keep it on screen whatever the offset.
    [x.clamp(0.0, (screen[0] - size[0]).max(0.0)), y.clamp(0.0, (screen[1] - size[1]).max(0.0))]
}

/// The offset that puts the overlay at `pos` (after the user dragged it in edit mode).
pub fn offset_for(a: Anchor, pos: [f32; 2], size: [f32; 2], screen: [f32; 2]) -> [f32; 2] {
    let base = place(a, [0.0, 0.0], size, screen);
    let (c, r) = a.cell();
    let dir = |cell: f32| if cell as u8 == 2 { -1.0 } else { 1.0 };
    [(pos[0] - base[0]) * dir(c), (pos[1] - base[1]) * dir(r)]
}

/// The anchor nearest to where a dragged overlay ended up (its centre's third of the screen).
pub fn nearest_anchor(pos: [f32; 2], size: [f32; 2], screen: [f32; 2]) -> Anchor {
    let third = |p: f32, s: f32, scr: f32| (((p + s / 2.0) / scr.max(1.0)) * 3.0).floor().clamp(0.0, 2.0) as usize;
    Anchor::ALL[third(pos[1], size[1], screen[1]) * 3 + third(pos[0], size[0], screen[0])]
}

/// What a ZeeZ Live hotkey does (zeez-live §5.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HotkeyAction {
    ToggleOverlay,
    CycleStyle,
    Capture,
    Marker,
    EditMode,
}

impl HotkeyAction {
    pub const ALL: [HotkeyAction; 5] = [
        HotkeyAction::ToggleOverlay,
        HotkeyAction::CycleStyle,
        HotkeyAction::Capture,
        HotkeyAction::Marker,
        HotkeyAction::EditMode,
    ];

    pub fn label(self) -> &'static str {
        match self {
            HotkeyAction::ToggleOverlay => "Show/hide overlay",
            HotkeyAction::CycleStyle => "Next style",
            HotkeyAction::Capture => "Start/stop capture",
            HotkeyAction::Marker => "Marker during capture",
            HotkeyAction::EditMode => "Overlay edit mode",
        }
    }

    /// The shortcut's id for the Wayland GlobalShortcuts portal: stable, so the
    /// desktop remembers the user's choice of keys across runs.
    pub fn portal_id(self) -> &'static str {
        match self {
            HotkeyAction::ToggleOverlay => "toggle-overlay",
            HotkeyAction::CycleStyle => "cycle-style",
            HotkeyAction::Capture => "capture",
            HotkeyAction::Marker => "marker",
            HotkeyAction::EditMode => "edit-mode",
        }
    }

    /// The action behind a portal shortcut id.
    pub fn from_portal_id(id: &str) -> Option<HotkeyAction> {
        HotkeyAction::ALL.into_iter().find(|a| a.portal_id() == id)
    }

    pub fn default_keys(self) -> &'static str {
        match self {
            HotkeyAction::ToggleOverlay => "Ctrl+Shift+O",
            HotkeyAction::CycleStyle => "Ctrl+Shift+L",
            HotkeyAction::Capture => "Ctrl+Shift+R",
            HotkeyAction::Marker => "Ctrl+Shift+M",
            HotkeyAction::EditMode => "Ctrl+Shift+E",
        }
    }
}

/// "ctrl + shift+o" → "CTRL+SHIFT+O": modifiers in a fixed order, so two
/// spellings of one shortcut compare equal. None when there's no main key.
pub fn normalize_keys(k: &str) -> Option<String> {
    let mut mods: Vec<&str> = Vec::new();
    let mut key = None;
    for t in k.split('+').map(str::trim).filter(|t| !t.is_empty()) {
        match t.to_ascii_uppercase().as_str() {
            "CTRL" | "CONTROL" => mods.push("CTRL"),
            "SHIFT" => mods.push("SHIFT"),
            "ALT" | "OPTION" => mods.push("ALT"),
            "SUPER" | "META" | "CMD" | "COMMAND" | "WIN" => mods.push("SUPER"),
            other => key = Some(other.to_string()),
        }
    }
    let order = ["CTRL", "ALT", "SHIFT", "SUPER"];
    mods.sort_by_key(|m| order.iter().position(|o| o == m));
    mods.dedup();
    let key = key?;
    Some(mods.into_iter().map(str::to_string).chain([key]).collect::<Vec<_>>().join("+"))
}

/// "Ctrl+Shift+O" in the portal's `preferred_trigger` spelling
/// (`CTRL+SHIFT+o`): modifiers `CTRL`, `ALT`, `SHIFT`, `LOGO`, then the XKB
/// keysym name (letters lowercase, `F9` as is, `Insert`, `Prior`…). It is only
/// a hint: the desktop shows it to the user, who may pick other keys.
pub fn portal_trigger(keys: &str) -> Option<String> {
    let norm = normalize_keys(keys)?;
    let mut parts: Vec<&str> = norm.split('+').collect();
    let key = parts.pop()?;
    let sym = match key {
        "INSERT" => "Insert".to_string(),
        "DELETE" => "Delete".to_string(),
        "HOME" => "Home".to_string(),
        "END" => "End".to_string(),
        "PAGEUP" => "Prior".to_string(),
        "PAGEDOWN" => "Next".to_string(),
        "SPACE" => "space".to_string(),
        "ESCAPE" | "ESC" => "Escape".to_string(),
        "TAB" => "Tab".to_string(),
        "ENTER" | "RETURN" => "Return".to_string(),
        k if k.len() == 1 => k.to_ascii_lowercase(),
        k => k.to_string(), // F1..F24 and the rest keep their spelling
    };
    let mods = parts.into_iter().map(|m| if m == "SUPER" { "LOGO" } else { m });
    Some(mods.map(str::to_string).chain([sym]).collect::<Vec<_>>().join("+"))
}

/// Where the portal answers a request made by the D-Bus peer `unique_name`
/// (":1.42") with `token`. The reply signal is subscribed to before the call,
/// so it cannot be missed.
pub fn portal_request_path(unique_name: &str, token: &str) -> String {
    format!(
        "/org/freedesktop/portal/desktop/request/{}/{}",
        unique_name.trim_start_matches(':').replace('.', "_"),
        token
    )
}

/// Pairs of actions bound to the same shortcut (empty bindings are off).
pub fn hotkey_conflicts(keys: &[(HotkeyAction, String)]) -> Vec<(HotkeyAction, HotkeyAction)> {
    let norm: Vec<(HotkeyAction, Option<String>)> = keys.iter().map(|(a, k)| (*a, normalize_keys(k))).collect();
    let mut out = Vec::new();
    for (i, (a, ka)) in norm.iter().enumerate() {
        for (b, kb) in &norm[i + 1..] {
            if ka.is_some() && ka == kb {
                out.push((*a, *b));
            }
        }
    }
    out
}

/// A game's own overlay settings and automation (zeez-live §5.4), opt-in.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    /// The executable's file name ("cs2.exe").
    pub exe: String,
    /// Its own layout and position (None: the overlay's).
    pub layout: Option<Layout>,
    pub anchor: Option<Anchor>,
    pub offset: Option<[f32; 2]>,
    /// Show the overlay while the game runs.
    pub auto_show: bool,
    /// Capture while the game runs (off by default; ● REC shows).
    pub auto_capture: bool,
    /// Anti-cheat safe mode (S2b-12): no reads through the PawnIO driver while it runs.
    pub safe_mode: bool,
}

impl Profile {
    pub fn automates(&self) -> bool {
        self.auto_show || self.auto_capture || self.safe_mode
    }
}

/// The file name of a path or name, for matching a profile.
pub fn exe_name(s: &str) -> &str {
    s.rsplit(['/', '\\']).next().unwrap_or(s)
}

pub fn profile_for<'a>(profiles: &'a [Profile], exe: &str) -> Option<&'a Profile> {
    let exe = exe_name(exe);
    profiles.iter().find(|p| !p.exe.is_empty() && exe_name(&p.exe).eq_ignore_ascii_case(exe))
}

/// What a profile changes in what was ever running: (started, stopped).
pub fn transitions<'a>(before: &[String], now: &'a [String]) -> (Vec<&'a str>, Vec<String>) {
    let has = |list: &[String], x: &str| list.iter().any(|y| y.eq_ignore_ascii_case(x));
    let started = now.iter().filter(|x| !has(before, x)).map(String::as_str).collect();
    let stopped = before.iter().filter(|x| !has(now, x)).cloned().collect();
    (started, stopped)
}

/// What the user set for the overlay (saved with the settings).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Overlay {
    pub shown: bool,
    /// The built-in style the layout came from (None: customized).
    pub style: Option<String>,
    pub layout: Layout,
    pub anchor: Anchor,
    pub offset: [f32; 2],
    /// Which monitor it lives on.
    pub screen: crate::screens::Which,
    /// Windows: keep the HUD out of screenshots, recordings and streams.
    pub hide_from_capture: bool,
    /// The three-step first use is done.
    pub first_use_done: bool,
    /// Text refresh (Hz) and graph refresh (Hz).
    pub text_hz: f32,
    pub graph_hz: f32,
    /// Shortcut per action ("" = off).
    pub hotkeys: Vec<(HotkeyAction, String)>,
    pub profiles: Vec<Profile>,
    /// A small "GPU", "CPU"… pill from the bottleneck detector on the overlay.
    pub bottleneck_pill: bool,
}

impl Overlay {
    /// The layout, anchor and offset in effect while `exe` is the target.
    pub fn effective(&self, exe: Option<&str>) -> (Layout, Anchor, [f32; 2]) {
        let p = exe.and_then(|e| profile_for(&self.profiles, e));
        (
            p.and_then(|p| p.layout.clone()).unwrap_or_else(|| self.layout.clone()),
            p.and_then(|p| p.anchor).unwrap_or(self.anchor),
            p.and_then(|p| p.offset).unwrap_or(self.offset),
        )
    }

    /// The shortcut bound to `a` ("" when off).
    pub fn keys(&self, a: HotkeyAction) -> &str {
        self.hotkeys.iter().find(|h| h.0 == a).map_or("", |h| h.1.as_str())
    }
}

impl Default for Overlay {
    fn default() -> Self {
        Overlay {
            shown: false,
            style: Some("Compact".into()),
            layout: crate::layout::presets::live::compact(100.0),
            anchor: Anchor::TopLeft,
            offset: [0.0, 0.0],
            screen: Default::default(),
            hide_from_capture: false,
            first_use_done: false,
            text_hz: 4.0,
            graph_hz: 30.0,
            hotkeys: HotkeyAction::ALL.iter().map(|a| (*a, a.default_keys().to_string())).collect(),
            profiles: Vec::new(),
            bottleneck_pill: true,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn portal_triggers_and_ids() {
        assert_eq!(portal_trigger("Ctrl+Shift+O").as_deref(), Some("CTRL+SHIFT+o"));
        assert_eq!(portal_trigger("shift + ctrl + F9").as_deref(), Some("CTRL+SHIFT+F9"));
        assert_eq!(portal_trigger("Super+Alt+Home").as_deref(), Some("ALT+LOGO+Home"));
        assert_eq!(portal_trigger("Ctrl+PageDown").as_deref(), Some("CTRL+Next"));
        assert_eq!(portal_trigger("Ctrl+Space").as_deref(), Some("CTRL+space"));
        assert_eq!(portal_trigger(""), None, "off stays off");
        assert_eq!(portal_trigger("Ctrl+"), None);
        for a in HotkeyAction::ALL {
            assert!(portal_trigger(a.default_keys()).is_some(), "{a:?} has a default trigger");
            assert_eq!(HotkeyAction::from_portal_id(a.portal_id()), Some(a));
        }
        let ids: std::collections::HashSet<_> = HotkeyAction::ALL.iter().map(|a| a.portal_id()).collect();
        assert_eq!(ids.len(), HotkeyAction::ALL.len(), "ids are unique");
        assert_eq!(HotkeyAction::from_portal_id("nope"), None);
        assert_eq!(portal_request_path(":1.42", "cz_1"), "/org/freedesktop/portal/desktop/request/1_42/cz_1");
    }

    use super::*;

    #[test]
    fn nine_anchors_and_offsets() {
        let (size, scr) = ([200.0, 50.0], [1920.0, 1080.0]);
        assert_eq!(place(Anchor::TopLeft, [0.0, 0.0], size, scr), [EDGE, EDGE]);
        assert_eq!(place(Anchor::BottomRight, [0.0, 0.0], size, scr), [1920.0 - 200.0 - EDGE, 1080.0 - 50.0 - EDGE]);
        assert_eq!(place(Anchor::Center, [0.0, 0.0], size, scr), [860.0, 515.0]);
        // A positive offset moves inwards from the right/bottom edges.
        assert_eq!(place(Anchor::BottomRight, [10.0, 20.0], size, scr), [1920.0 - 222.0, 1080.0 - 82.0]);
        assert_eq!(place(Anchor::TopLeft, [-500.0, -500.0], size, scr), [0.0, 0.0], "kept on screen");
        for a in Anchor::ALL {
            let pos = place(a, [30.0, 15.0], size, scr);
            assert_eq!(offset_for(a, pos, size, scr), [30.0, 15.0], "{a:?}");
            assert_eq!(nearest_anchor(place(a, [0.0, 0.0], size, scr), size, scr), a, "{a:?}");
        }
    }

    #[test]
    fn five_styles_round_trip() {
        let all = crate::layout::presets::live::all(95.0);
        let names: Vec<&str> = all.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["Minimal", "Compact", "Detailed", "Benchmark", "Overclocker"]);
        for l in &all {
            assert_eq!(&Layout::from_json(&l.to_json()).unwrap(), l, "{}", l.name);
        }
        assert!(all[1].metrics().contains(&"frames/fps"), "Compact starts with FPS");
    }

    #[test]
    fn hotkeys_normalize_and_conflict() {
        assert_eq!(normalize_keys("shift + ctrl+o").as_deref(), Some("CTRL+SHIFT+O"));
        assert_eq!(normalize_keys("Ctrl+Shift"), None, "no main key");
        let mut keys = Overlay::default().hotkeys;
        assert!(hotkey_conflicts(&keys).is_empty(), "the defaults don't collide");
        keys[1].1 = "shift+CTRL+o".into();
        assert_eq!(hotkey_conflicts(&keys), [(HotkeyAction::ToggleOverlay, HotkeyAction::CycleStyle)]);
        keys[1].1.clear();
        keys[2].1.clear();
        assert!(hotkey_conflicts(&keys).is_empty(), "turned-off bindings never conflict");
        assert_eq!(Overlay::default().keys(HotkeyAction::Capture), "Ctrl+Shift+R");
    }

    #[test]
    fn profiles_match_and_apply() {
        let mut o = Overlay::default();
        o.profiles.push(Profile {
            exe: "CS2.exe".into(),
            layout: Some(crate::layout::presets::live::minimal()),
            anchor: Some(Anchor::BottomRight),
            ..Default::default()
        });
        assert!(profile_for(&o.profiles, r"C:\Games\cs2.exe").is_some(), "by file name, any case");
        assert!(profile_for(&o.profiles, "notcs2.exe").is_none());
        let (l, a, off) = o.effective(Some("cs2.exe"));
        assert_eq!((l.name.as_str(), a, off), ("Minimal", Anchor::BottomRight, [0.0, 0.0]));
        assert_eq!(o.effective(Some("other.exe")).1, Anchor::TopLeft);
        let (before, now) =
            (vec!["a.exe".to_string(), "b.exe".to_string()], vec!["B.exe".to_string(), "c.exe".to_string()]);
        let (started, stopped) = transitions(&before, &now);
        assert_eq!((started, stopped), (vec!["c.exe"], vec!["a.exe".to_string()]));
    }

    #[test]
    fn settings_load_with_defaults() {
        let o: Overlay = serde_json::from_str(r#"{"shown": true, "anchor": "bottom-right"}"#).unwrap();
        assert!(o.shown && o.anchor == Anchor::BottomRight && !o.layout.widgets.is_empty());
        assert_eq!(o.text_hz, 4.0);
    }
}
