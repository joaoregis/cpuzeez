//! Where windows go on a desktop with more than one monitor: the overlay on
//! the monitor the game is on, the mini monitors snapping to the edges of the
//! monitor they were dropped on. Pure geometry; the GUI feeds it the monitors'
//! rectangles (in egui points) and applies the result.

use crate::live::{Anchor, nearest_anchor, offset_for, place};
use serde::{Deserialize, Serialize};

/// One monitor's rectangle in the desktop's coordinate space (the top-left of
/// the primary monitor is usually 0,0; a monitor to its left has negative x).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Screen {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub primary: bool,
}

impl Screen {
    pub fn origin(&self) -> [f32; 2] {
        [self.x, self.y]
    }

    pub fn size(&self) -> [f32; 2] {
        [self.w, self.h]
    }

    pub fn centre(&self) -> [f32; 2] {
        [self.x + self.w / 2.0, self.y + self.h / 2.0]
    }

    pub fn contains(&self, p: [f32; 2]) -> bool {
        p[0] >= self.x && p[0] < self.x + self.w && p[1] >= self.y && p[1] < self.y + self.h
    }
}

/// Which monitor the overlay lives on (a setting).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Which {
    /// Where the game is; the primary monitor when there is no game.
    #[default]
    Auto,
    Primary,
    /// The n-th monitor in the order the OS lists them.
    Index(usize),
}

/// The monitor holding `p`; when it is on none (a window half off screen, a
/// monitor that was unplugged), the one whose centre is nearest.
pub fn screen_at(screens: &[Screen], p: [f32; 2]) -> Option<usize> {
    if let Some(i) = screens.iter().position(|s| s.contains(p)) {
        return Some(i);
    }
    let dist = |s: &Screen| {
        let c = s.centre();
        (c[0] - p[0]).hypot(c[1] - p[1])
    };
    (0..screens.len()).min_by(|a, b| dist(&screens[*a]).total_cmp(&dist(&screens[*b])))
}

fn primary(screens: &[Screen]) -> Option<usize> {
    screens.iter().position(|s| s.primary).or(if screens.is_empty() { None } else { Some(0) })
}

/// The monitor to draw on. `game` is the centre of the game's window, if there is one.
pub fn choose(screens: &[Screen], which: Which, game: Option<[f32; 2]>) -> Option<usize> {
    match which {
        Which::Auto => game.and_then(|g| screen_at(screens, g)).or_else(|| primary(screens)),
        Which::Primary => primary(screens),
        // A monitor that is no longer there falls back to the primary one.
        Which::Index(i) if i < screens.len() => Some(i),
        Which::Index(_) => primary(screens),
    }
}

/// Top-left position of an overlay of `size` anchored at `a` on `screen`.
pub fn place_on(screen: &Screen, a: Anchor, offset: [f32; 2], size: [f32; 2]) -> [f32; 2] {
    let p = place(a, offset, size, screen.size());
    [screen.x + p[0], screen.y + p[1]]
}

/// Where a dragged overlay at `pos` ended up: the monitor under its centre, the
/// nearest anchor of that monitor and the offset from it.
pub fn locate(screens: &[Screen], pos: [f32; 2], size: [f32; 2]) -> Option<(usize, Anchor, [f32; 2])> {
    let centre = [pos[0] + size[0] / 2.0, pos[1] + size[1] / 2.0];
    let i = screen_at(screens, centre)?;
    let s = &screens[i];
    let local = [pos[0] - s.x, pos[1] - s.y];
    let anchor = nearest_anchor(local, size, s.size());
    Some((i, anchor, offset_for(anchor, local, size, s.size())))
}

/// A mini monitor dropped at `pos`, snapped to the edges of the monitor it is on.
pub fn snap_on(screens: &[Screen], pos: [f32; 2], size: [f32; 2], dist: f32) -> [f32; 2] {
    let centre = [pos[0] + size[0] / 2.0, pos[1] + size[1] / 2.0];
    let Some(i) = screen_at(screens, centre) else { return pos };
    let s = &screens[i];
    let local = crate::layout::mini::snap([pos[0] - s.x, pos[1] - s.y], size, s.size(), dist);
    [s.x + local[0], s.y + local[1]]
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: Screen = Screen { x: 0.0, y: 0.0, w: 1920.0, h: 1080.0, primary: true };
    /// To the left of the primary, so its x is negative; taller.
    const B: Screen = Screen { x: -2560.0, y: -200.0, w: 2560.0, h: 1440.0, primary: false };
    /// Above-right, a portrait monitor.
    const C: Screen = Screen { x: 1920.0, y: -600.0, w: 1080.0, h: 1920.0, primary: false };

    #[test]
    fn the_monitor_under_a_point_even_with_negative_origins() {
        let all = [A, B, C];
        assert_eq!(screen_at(&all, [100.0, 100.0]), Some(0));
        assert_eq!(screen_at(&all, [-100.0, 100.0]), Some(1));
        assert_eq!(screen_at(&all, [2000.0, -500.0]), Some(2));
        // Off every monitor (a gap, or an unplugged one): the nearest centre.
        assert_eq!(screen_at(&all, [-3000.0, 1500.0]), Some(1));
        assert_eq!(screen_at(&[], [0.0, 0.0]), None);
    }

    #[test]
    fn choosing_where_to_draw() {
        let all = [B, A, C]; // the primary is not first
        assert_eq!(choose(&all, Which::Auto, None), Some(1), "no game: the primary");
        assert_eq!(choose(&all, Which::Auto, Some([-500.0, 0.0])), Some(0), "the game's monitor");
        assert_eq!(choose(&all, Which::Auto, Some([9999.0, 9999.0])), Some(2), "off screen: the nearest one");
        assert_eq!(choose(&all, Which::Primary, Some([-500.0, 0.0])), Some(1), "primary ignores the game");
        assert_eq!(choose(&all, Which::Index(2), None), Some(2));
        assert_eq!(choose(&all, Which::Index(7), None), Some(1), "a monitor that went away: the primary");
        assert_eq!(choose(&[], Which::Auto, None), None);
        let no_primary = [Screen { primary: false, ..A }];
        assert_eq!(choose(&no_primary, Which::Primary, None), Some(0), "no flag: the first one");
    }

    #[test]
    fn placement_is_relative_to_the_monitors_origin() {
        let size = [300.0, 100.0];
        let on_a = place_on(&A, Anchor::TopLeft, [0.0, 0.0], size);
        let on_b = place_on(&B, Anchor::TopLeft, [0.0, 0.0], size);
        assert_eq!(on_b[0] - on_a[0], B.x - A.x);
        assert_eq!(on_b[1] - on_a[1], B.y - A.y);
        // Bottom right of C stays inside C.
        let p = place_on(&C, Anchor::BottomRight, [0.0, 0.0], size);
        assert!(C.contains(p) && C.contains([p[0] + size[0] - 1.0, p[1] + size[1] - 1.0]), "{p:?}");
    }

    /// Dropping an overlay and placing it again gives the same spot, on any monitor.
    #[test]
    fn locate_and_place_round_trip() {
        let all = [A, B, C];
        let size = [320.0, 140.0];
        for (i, s) in all.iter().enumerate() {
            for (fx, fy) in [(0.05, 0.05), (0.5, 0.5), (0.9, 0.85), (0.1, 0.9)] {
                let pos = [s.x + (s.w - size[0]) * fx, s.y + (s.h - size[1]) * fy]; // fully on the monitor
                let (j, a, off) = locate(&all, pos, size).unwrap();
                assert_eq!(j, i);
                let back = place_on(&all[j], a, off, size);
                assert!((back[0] - pos[0]).abs() < 1.0 && (back[1] - pos[1]).abs() < 1.0, "{i} {pos:?} -> {back:?}");
            }
        }
    }

    #[test]
    fn snapping_uses_the_monitor_it_is_on() {
        let all = [A, B];
        // 8 px from B's left edge (x = -2560) and 5 px from its top (y = -200): snaps to both.
        let p = snap_on(&all, [-2552.0, -195.0], [300.0, 100.0], 20.0);
        assert_eq!(p, [-2560.0, -200.0]);
        // Far from every edge: untouched.
        assert_eq!(snap_on(&all, [500.0, 400.0], [300.0, 100.0], 20.0), [500.0, 400.0]);
        // The old single-screen maths would have snapped a window on B to A's origin.
        assert_eq!(snap_on(&[], [5.0, 5.0], [10.0, 10.0], 20.0), [5.0, 5.0], "no monitors: untouched");
    }

    #[test]
    fn the_setting_survives_json_and_old_files() {
        for w in [Which::Auto, Which::Primary, Which::Index(2)] {
            let back: Which = serde_json::from_str(&serde_json::to_string(&w).unwrap()).unwrap();
            assert_eq!(back, w);
        }
        let o: crate::live::Overlay = serde_json::from_str("{}").unwrap();
        assert_eq!(o.screen, Which::Auto, "files from before this setting");
    }
}
