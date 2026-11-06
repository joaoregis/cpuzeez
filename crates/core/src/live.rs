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
    /// Windows: keep the HUD out of screenshots, recordings and streams.
    pub hide_from_capture: bool,
    /// The three-step first use is done.
    pub first_use_done: bool,
    /// Text refresh (Hz) and graph refresh (Hz).
    pub text_hz: f32,
    pub graph_hz: f32,
}

impl Default for Overlay {
    fn default() -> Self {
        Overlay {
            shown: false,
            style: Some("Compact".into()),
            layout: crate::layout::presets::compact(100.0),
            anchor: Anchor::TopLeft,
            offset: [0.0, 0.0],
            hide_from_capture: false,
            first_use_done: false,
            text_hz: 4.0,
            graph_hz: 30.0,
        }
    }
}

#[cfg(test)]
mod tests {
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
    fn settings_load_with_defaults() {
        let o: Overlay = serde_json::from_str(r#"{"shown": true, "anchor": "bottom-right"}"#).unwrap();
        assert!(o.shown && o.anchor == Anchor::BottomRight && !o.layout.widgets.is_empty());
        assert_eq!(o.text_hz, 4.0);
    }
}
