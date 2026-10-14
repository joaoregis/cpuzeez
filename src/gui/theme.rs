//! Design tokens and egui styling: one brand accent (violet), light and dark
//! palettes, the OS's own UI font (Segoe UI / SF Pro / Cantarell, Noto, Inter…)
//! and Phosphor icons. Screens read colors through `palette(ui)`, never literals.

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke, TextStyle, Theme,
    Visuals,
};
use std::sync::Arc;

pub use egui_phosphor::regular as icon;

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub bg: Color32,
    pub sidebar: Color32,
    pub card: Color32,
    pub card_hover: Color32,
    pub border: Color32,
    pub text: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_soft: Color32,
    pub on_accent: Color32,
    pub success: Color32,
    pub success_soft: Color32,
    pub warning: Color32,
    pub warning_soft: Color32,
    pub danger: Color32,
    pub danger_soft: Color32,
    /// Chart series colors (CPU, GPU, chipset/board, memory, storage…).
    pub series: [Color32; 6],
}

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

pub const LIGHT: Palette = Palette {
    bg: rgb(0xF7F7FA),
    sidebar: rgb(0xEFEEF5),
    card: rgb(0xFFFFFF),
    card_hover: rgb(0xF5F3FF),
    border: rgb(0xE4E2EE),
    text: rgb(0x1B1A23),
    text_secondary: rgb(0x5F5C72),
    text_muted: rgb(0x8E8BA3),
    accent: rgb(0x7C3AED),
    accent_hover: rgb(0x6D28D9),
    accent_soft: rgb(0xEDE9FE),
    on_accent: rgb(0xFFFFFF),
    success: rgb(0x15803D),
    success_soft: rgb(0xDCFCE7),
    warning: rgb(0xB45309),
    warning_soft: rgb(0xFEF3C7),
    danger: rgb(0xB91C1C),
    danger_soft: rgb(0xFEE2E2),
    series: [rgb(0x7C3AED), rgb(0xDB2777), rgb(0x0D9488), rgb(0x2563EB), rgb(0xD97706), rgb(0x64748B)],
};

pub const DARK: Palette = Palette {
    bg: rgb(0x111018),
    sidebar: rgb(0x17161F),
    card: rgb(0x1D1C27),
    card_hover: rgb(0x252336),
    border: rgb(0x2E2C3D),
    text: rgb(0xECEBF4),
    text_secondary: rgb(0xA6A3BA),
    text_muted: rgb(0x75728A),
    accent: rgb(0xA78BFA),
    accent_hover: rgb(0xC4B5FD),
    accent_soft: rgb(0x2E2650),
    on_accent: rgb(0x14121C),
    success: rgb(0x4ADE80),
    success_soft: rgb(0x14321F),
    warning: rgb(0xFBBF24),
    warning_soft: rgb(0x3A2C0B),
    danger: rgb(0xF87171),
    danger_soft: rgb(0x3D1717),
    series: [rgb(0xA78BFA), rgb(0xF472B6), rgb(0x2DD4BF), rgb(0x60A5FA), rgb(0xFBBF24), rgb(0x94A3B8)],
};

pub fn palette(ui: &egui::Ui) -> &'static Palette {
    if ui.visuals().dark_mode { &DARK } else { &LIGHT }
}

pub fn palette_ctx(ctx: &egui::Context) -> &'static Palette {
    if ctx.theme() == Theme::Dark { &DARK } else { &LIGHT }
}

// ---- spacing / radii (logical px) ----
pub const RADIUS_CONTROL: u8 = 8;
pub const RADIUS_CARD: u8 = 12;
pub const PAD_CARD: i8 = 14;
pub const PAD_PAGE: f32 = 20.0;
pub const GAP: f32 = 12.0;
pub const SIDEBAR_WIDTH: f32 = 196.0;

/// Named text styles beyond egui's built-ins.
pub fn title() -> TextStyle {
    TextStyle::Name("title".into())
}
pub fn subtitle() -> TextStyle {
    TextStyle::Name("subtitle".into())
}
pub fn metric() -> TextStyle {
    TextStyle::Name("metric".into())
}
pub fn semibold() -> FontFamily {
    FontFamily::Name("semibold".into())
}

/// Fonts + both themes. Call once from the eframe creator.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(fonts());
    for (theme, p) in [(Theme::Light, &LIGHT), (Theme::Dark, &DARK)] {
        let mut style = (*ctx.style_of(theme)).clone();
        apply(&mut style, p, theme == Theme::Dark);
        ctx.set_style_of(theme, style);
    }
}

fn apply(style: &mut egui::Style, p: &Palette, dark: bool) {
    let mut v = if dark { Visuals::dark() } else { Visuals::light() };
    v.panel_fill = p.bg;
    v.window_fill = p.card;
    v.extreme_bg_color = if dark { rgb(0x0C0B12) } else { rgb(0xFFFFFF) };
    v.faint_bg_color = if dark { rgb(0x201F2B) } else { rgb(0xF6F5FA) };
    v.code_bg_color = v.faint_bg_color;
    v.window_stroke = Stroke::new(1.0, p.border);
    v.window_corner_radius = CornerRadius::same(RADIUS_CARD);
    v.menu_corner_radius = CornerRadius::same(RADIUS_CONTROL);
    v.hyperlink_color = p.accent;
    v.selection.bg_fill = p.accent_soft;
    v.selection.stroke = Stroke::new(1.0, p.accent);
    v.striped = false;
    v.window_shadow.color = Color32::from_black_alpha(if dark { 110 } else { 28 });
    v.popup_shadow.color = Color32::from_black_alpha(if dark { 110 } else { 24 });
    let r = CornerRadius::same(RADIUS_CONTROL);
    let w = &mut v.widgets;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text);
    w.noninteractive.corner_radius = r;
    for (st, fill, stroke) in [
        (&mut w.inactive, if dark { rgb(0x262433) } else { rgb(0xFFFFFF) }, p.border),
        (&mut w.hovered, if dark { rgb(0x2E2B40) } else { rgb(0xF5F3FF) }, p.accent),
        (&mut w.active, p.accent_soft, p.accent),
        (&mut w.open, p.accent_soft, p.accent),
    ] {
        st.bg_fill = fill;
        st.weak_bg_fill = fill;
        st.bg_stroke = Stroke::new(1.0, stroke);
        st.corner_radius = r;
        st.fg_stroke = Stroke::new(1.0, p.text);
        st.expansion = 0.0;
    }
    style.visuals = v;
    let s = &mut style.spacing;
    s.item_spacing = egui::vec2(8.0, 6.0);
    s.button_padding = egui::vec2(12.0, 6.0);
    s.interact_size.y = 30.0;
    s.window_margin = Margin::same(16);
    s.menu_margin = Margin::same(8);
    s.combo_width = 140.0;
    s.scroll.bar_width = 8.0;
    style.text_styles = [
        (TextStyle::Small, FontId::proportional(11.5)),
        (TextStyle::Body, FontId::proportional(14.0)),
        (TextStyle::Button, FontId::proportional(14.0)),
        (TextStyle::Monospace, FontId::monospace(13.0)),
        (TextStyle::Heading, FontId::new(17.0, semibold())),
        (title(), FontId::new(24.0, semibold())),
        (subtitle(), FontId::proportional(13.0)),
        (metric(), FontId::new(22.0, semibold())),
    ]
    .into();
}

/// OS UI font (regular + semibold) when found, egui's default otherwise;
/// Phosphor icons as fallback in both families.
fn fonts() -> FontDefinitions {
    let mut f = FontDefinitions::default();
    let (regular, bold) = system_ui_fonts();
    if let Some(bytes) = regular {
        f.font_data.insert("ui".into(), Arc::new(FontData::from_owned(bytes)));
        f.families.entry(FontFamily::Proportional).or_default().insert(0, "ui".into());
    }
    let semibold_key = if let Some(bytes) = bold {
        f.font_data.insert("ui-semibold".into(), Arc::new(FontData::from_owned(bytes)));
        "ui-semibold"
    } else if f.font_data.contains_key("ui") {
        "ui"
    } else {
        ""
    };
    let mut sb: Vec<String> = f.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    if !semibold_key.is_empty() {
        sb.retain(|k| k != semibold_key);
        sb.insert(0, semibold_key.into());
    }
    f.families.insert(semibold(), sb);
    egui_phosphor::add_to_fonts(&mut f, egui_phosphor::Variant::Regular);
    // Icons inside semibold labels too.
    if let Some(fam) = f.families.get_mut(&semibold()) {
        fam.insert(fam.len().min(1), "phosphor".into());
    }
    f
}

fn read_first(paths: &[&str]) -> Option<Vec<u8>> {
    paths.iter().find_map(|p| std::fs::read(p).ok())
}

fn system_ui_fonts() -> (Option<Vec<u8>>, Option<Vec<u8>>) {
    #[cfg(windows)]
    {
        let dir = std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into());
        let r = format!(r"{dir}\Fonts\segoeui.ttf");
        let b = format!(r"{dir}\Fonts\seguisb.ttf");
        (read_first(&[&r]), read_first(&[&b]))
    }
    #[cfg(target_os = "macos")]
    {
        (
            read_first(&["/System/Library/Fonts/SFNS.ttf", "/System/Library/Fonts/Helvetica.ttc"]),
            read_first(&["/System/Library/Fonts/SFNS.ttf"]),
        )
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        // Common distro locations; first family found wins (keeps regular/semibold matched).
        const FAMILIES: [(&[&str], &[&str]); 5] = [
            (
                &[
                    "/usr/share/fonts/inter/Inter-Regular.ttf",
                    "/usr/share/fonts/opentype/inter/Inter-Regular.otf",
                    "/usr/share/fonts/truetype/inter/Inter-Regular.ttf",
                ],
                &[
                    "/usr/share/fonts/inter/Inter-SemiBold.ttf",
                    "/usr/share/fonts/opentype/inter/Inter-SemiBold.otf",
                    "/usr/share/fonts/truetype/inter/Inter-SemiBold.ttf",
                ],
            ),
            (
                &[
                    "/usr/share/fonts/cantarell/Cantarell-VF.otf",
                    "/usr/share/fonts/opentype/cantarell/Cantarell-Regular.otf",
                ],
                &["/usr/share/fonts/opentype/cantarell/Cantarell-Bold.otf"],
            ),
            (
                &["/usr/share/fonts/noto/NotoSans-Regular.ttf", "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf"],
                &[
                    "/usr/share/fonts/noto/NotoSans-SemiBold.ttf",
                    "/usr/share/fonts/truetype/noto/NotoSans-SemiBold.ttf",
                ],
            ),
            (&["/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf"], &["/usr/share/fonts/truetype/ubuntu/Ubuntu-M.ttf"]),
            (
                &["/usr/share/fonts/TTF/DejaVuSans.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"],
                &["/usr/share/fonts/TTF/DejaVuSans-Bold.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"],
            ),
        ];
        for (r, b) in FAMILIES {
            if let Some(reg) = read_first(r) {
                return (Some(reg), read_first(b));
            }
        }
        (None, None)
    }
}
