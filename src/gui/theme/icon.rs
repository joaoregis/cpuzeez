//! The Phosphor *Regular* icons the app uses (D4): the font is
//! `assets/fonts/Phosphor.ttf` (MIT, Phosphor Icons, license next to it), and
//! each constant is its glyph's codepoint. To add an icon, take its codepoint
//! from <https://phosphoricons.com> (or the font's own `cmap`) and add it to
//! `ALL` too; the test checks every one has a glyph.

// A few icons appear only on some OSes (Copy on Windows and Linux, …): a
// table of constants, like the crate it replaces.
#![allow(dead_code)]

/// The font file, loaded by `theme::fonts`.
pub const FONT: &[u8] = include_bytes!("../../../assets/fonts/Phosphor.ttf");

pub const ARROWS_OUT_CARDINAL: &str = "\u{E0A4}";
pub const ARROW_CIRCLE_UP: &str = "\u{E030}";
pub const ARROW_CLOCKWISE: &str = "\u{E036}";
pub const ARROW_COUNTER_CLOCKWISE: &str = "\u{E038}";
pub const ARROW_DOWN: &str = "\u{E03E}";
pub const ARROW_ELBOW_DOWN_RIGHT: &str = "\u{E046}";
pub const ARROW_RIGHT: &str = "\u{E06C}";
pub const ARROW_UP: &str = "\u{E08E}";
pub const BATTERY_FULL: &str = "\u{E0C0}";
pub const BATTERY_HIGH: &str = "\u{E0C2}";
pub const BATTERY_WARNING: &str = "\u{E0C8}";
pub const BELL: &str = "\u{E0CE}";
pub const BLUETOOTH: &str = "\u{E0DA}";
pub const CAMERA: &str = "\u{E10E}";
pub const CARET_DOWN: &str = "\u{E136}";
pub const CARET_RIGHT: &str = "\u{E13A}";
pub const CARET_UP: &str = "\u{E13C}";
pub const CHART_BAR: &str = "\u{E150}";
pub const CHART_LINE: &str = "\u{E154}";
pub const CHART_LINE_UP: &str = "\u{E156}";
pub const CHECK: &str = "\u{E182}";
pub const CHECK_CIRCLE: &str = "\u{E184}";
pub const CIRCLE: &str = "\u{E18A}";
pub const CIRCUITRY: &str = "\u{E9C2}";
pub const CLOCK: &str = "\u{E19A}";
pub const CLOCK_COUNTER_CLOCKWISE: &str = "\u{E1A0}";
pub const COPY: &str = "\u{E1CA}";
pub const CPU: &str = "\u{E610}";
pub const DESKTOP: &str = "\u{E560}";
pub const DOT_OUTLINE: &str = "\u{ECE0}";
pub const DOWNLOAD_SIMPLE: &str = "\u{E20C}";
pub const EXPORT: &str = "\u{EAF0}";
pub const FILES: &str = "\u{E710}";
pub const FILE_ARROW_UP: &str = "\u{E61E}";
pub const FILE_HTML: &str = "\u{EB38}";
pub const FILE_TEXT: &str = "\u{E23A}";
pub const FILM_STRIP: &str = "\u{E792}";
pub const FIRE: &str = "\u{E242}";
pub const FLAG: &str = "\u{E244}";
pub const FLAME: &str = "\u{E624}";
pub const FLOPPY_DISK: &str = "\u{E248}";
pub const FOLDER_OPEN: &str = "\u{E256}";
pub const GAME_CONTROLLER: &str = "\u{E26E}";
pub const GAUGE: &str = "\u{E628}";
pub const GEAR_SIX: &str = "\u{E272}";
pub const GIT_BRANCH: &str = "\u{E278}";
pub const GIT_DIFF: &str = "\u{E27C}";
pub const GLOBE: &str = "\u{E288}";
pub const GRAPHICS_CARD: &str = "\u{E612}";
pub const GRID_FOUR: &str = "\u{E296}";
pub const HAND_GRABBING: &str = "\u{E57C}";
pub const HARD_DRIVE: &str = "\u{E29E}";
pub const HARD_DRIVES: &str = "\u{E2A0}";
pub const HEARTBEAT: &str = "\u{E2AC}";
pub const IMAGE: &str = "\u{E2CA}";
pub const INFO: &str = "\u{E2CE}";
pub const KEYBOARD: &str = "\u{E2D8}";
pub const LIGHTBULB: &str = "\u{E2DC}";
pub const LIGHTNING: &str = "\u{E2DE}";
pub const LIST_BULLETS: &str = "\u{E2F2}";
pub const LOCK: &str = "\u{E2FA}";
pub const LOCK_SIMPLE: &str = "\u{E308}";
pub const MEMORY: &str = "\u{E9C4}";
pub const MICROPHONE: &str = "\u{E326}";
pub const MONITOR: &str = "\u{E32E}";
pub const NETWORK: &str = "\u{EDDE}";
pub const PALETTE: &str = "\u{E6C8}";
pub const PAUSE: &str = "\u{E39E}";
pub const PENCIL_SIMPLE: &str = "\u{E3B4}";
pub const PICTURE_IN_PICTURE: &str = "\u{E64C}";
pub const PLAY: &str = "\u{E3D0}";
pub const PLUS: &str = "\u{E3D4}";
pub const PULSE: &str = "\u{E000}";
pub const PUSH_PIN: &str = "\u{E3E2}";
pub const RECORD: &str = "\u{E3EE}";
pub const ROCKET_LAUNCH: &str = "\u{E3FE}";
pub const RULER: &str = "\u{E6B8}";
pub const SCALES: &str = "\u{E750}";
pub const SCAN: &str = "\u{EBB6}";
pub const SHIELD_CHECK: &str = "\u{E40C}";
pub const SIDEBAR_SIMPLE: &str = "\u{EC24}";
pub const SPEAKER_HIGH: &str = "\u{E44A}";
pub const SPEEDOMETER: &str = "\u{EE74}";
pub const SQUARE: &str = "\u{E45E}";
pub const SQUARES_FOUR: &str = "\u{E464}";
pub const STACK: &str = "\u{E466}";
pub const STETHOSCOPE: &str = "\u{E7EA}";
pub const STOP: &str = "\u{E46C}";
pub const TABLE: &str = "\u{E476}";
pub const TERMINAL_WINDOW: &str = "\u{EAE8}";
pub const THERMOMETER: &str = "\u{E5C6}";
pub const THERMOMETER_HOT: &str = "\u{E5CA}";
pub const TIMER: &str = "\u{E492}";
pub const TOOLBOX: &str = "\u{ECA0}";
pub const TRASH: &str = "\u{E4A6}";
pub const TRAY: &str = "\u{E4AA}";
pub const TRAY_ARROW_DOWN: &str = "\u{E010}";
pub const USB: &str = "\u{E956}";
pub const USERS: &str = "\u{E4D6}";
pub const WARNING: &str = "\u{E4E0}";
pub const WARNING_OCTAGON: &str = "\u{E4E4}";
pub const WIFI_HIGH: &str = "\u{E4EA}";
pub const WRENCH: &str = "\u{E5D4}";
pub const X: &str = "\u{E4F6}";

/// Every icon above, for the glyph test.
#[cfg(test)]
pub const ALL: &[&str] = &[
    ARROWS_OUT_CARDINAL,
    ARROW_CIRCLE_UP,
    ARROW_CLOCKWISE,
    ARROW_COUNTER_CLOCKWISE,
    ARROW_DOWN,
    ARROW_ELBOW_DOWN_RIGHT,
    ARROW_RIGHT,
    ARROW_UP,
    BATTERY_FULL,
    BATTERY_HIGH,
    BATTERY_WARNING,
    BELL,
    BLUETOOTH,
    CAMERA,
    CARET_DOWN,
    CARET_RIGHT,
    CARET_UP,
    CHART_BAR,
    CHART_LINE,
    CHART_LINE_UP,
    CHECK,
    CHECK_CIRCLE,
    CIRCLE,
    CIRCUITRY,
    CLOCK,
    CLOCK_COUNTER_CLOCKWISE,
    COPY,
    CPU,
    DESKTOP,
    DOT_OUTLINE,
    DOWNLOAD_SIMPLE,
    EXPORT,
    FILES,
    FILE_ARROW_UP,
    FILE_HTML,
    FILE_TEXT,
    FILM_STRIP,
    FIRE,
    FLAG,
    FLAME,
    FLOPPY_DISK,
    FOLDER_OPEN,
    GAME_CONTROLLER,
    GAUGE,
    GEAR_SIX,
    GIT_BRANCH,
    GIT_DIFF,
    GLOBE,
    GRAPHICS_CARD,
    GRID_FOUR,
    HAND_GRABBING,
    HARD_DRIVE,
    HARD_DRIVES,
    HEARTBEAT,
    IMAGE,
    INFO,
    KEYBOARD,
    LIGHTBULB,
    LIGHTNING,
    LIST_BULLETS,
    LOCK,
    LOCK_SIMPLE,
    MEMORY,
    MICROPHONE,
    MONITOR,
    NETWORK,
    PALETTE,
    PAUSE,
    PENCIL_SIMPLE,
    PICTURE_IN_PICTURE,
    PLAY,
    PLUS,
    PULSE,
    PUSH_PIN,
    RECORD,
    ROCKET_LAUNCH,
    RULER,
    SCALES,
    SCAN,
    SHIELD_CHECK,
    SIDEBAR_SIMPLE,
    SPEAKER_HIGH,
    SPEEDOMETER,
    SQUARE,
    SQUARES_FOUR,
    STACK,
    STETHOSCOPE,
    STOP,
    TABLE,
    TERMINAL_WINDOW,
    THERMOMETER,
    THERMOMETER_HOT,
    TIMER,
    TOOLBOX,
    TRASH,
    TRAY,
    TRAY_ARROW_DOWN,
    USB,
    USERS,
    WARNING,
    WARNING_OCTAGON,
    WIFI_HIGH,
    WRENCH,
    X,
];

/// Every constant has a glyph in the vendored font itself, not in a fallback.
#[test]
fn every_icon_is_in_the_font() {
    use eframe::egui::{Context, FontData, FontDefinitions, FontFamily, FontId};
    let mut f = FontDefinitions::empty();
    f.font_data.insert("phosphor".into(), std::sync::Arc::new(FontData::from_static(FONT)));
    f.families.insert(FontFamily::Proportional, vec!["phosphor".into()]);
    let ctx = Context::default();
    ctx.set_fonts(f);
    ctx.run_ui(Default::default(), |_| {}).textures_delta.clear();
    let id = FontId::proportional(14.0);
    let missing: Vec<&str> =
        ALL.iter().filter(|s| !ctx.fonts_mut(|f| s.chars().all(|c| f.has_glyph(&id, c)))).copied().collect();
    assert_eq!(ALL.len(), 104);
    // The check can fail: a character the font lacks is reported missing.
    assert!(!ctx.fonts_mut(|f| f.has_glyph(&id, 'Ω')));
    assert!(missing.is_empty(), "no glyph for {missing:?}");
}
