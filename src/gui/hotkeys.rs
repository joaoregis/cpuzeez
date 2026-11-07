//! ZeeZ Live global hotkeys (S2b-04, zeez-live §5.5) through `global-hotkey`:
//! `RegisterHotKey` on Windows, Carbon `RegisterEventHotKey` on macOS (no
//! Accessibility permission) and `XGrabKey` on X11. Wayland has no global
//! grabs for apps: there the XDG GlobalShortcuts portal asks the desktop
//! (`hotkeys_portal.rs`, ADR 0015); on a desktop without it the shortcuts only
//! work while a CPU ZeeZ window has focus, and the tray menu is the way.

use super::i18n::tr;
use cpuzeez_core::live::{HotkeyAction, normalize_keys};
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

#[derive(Default)]
pub struct Hotkeys {
    mgr: Option<GlobalHotKeyManager>,
    failed_start: Option<String>,
    /// What is registered now: (action, the HotKey).
    active: Vec<(HotkeyAction, HotKey)>,
    /// The bindings `active` was made from, to notice changes.
    applied: Vec<(HotkeyAction, String)>,
    /// Bindings the OS refused (another app has them) or that don't parse.
    pub problems: Vec<(HotkeyAction, String)>,
    /// The Wayland portal, once started.
    #[cfg(all(unix, not(target_os = "macos")))]
    portal: Option<super::hotkeys_portal::Portal>,
    /// Wayland: what the desktop says about the shortcuts (None = fine).
    pub portal_note: Option<String>,
}

/// Whether this session is Wayland (X11 grabs do not reach native Wayland windows).
#[cfg(all(unix, not(target_os = "macos")))]
fn wayland() -> bool {
    std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v.eq_ignore_ascii_case("wayland"))
        || std::env::var_os("WAYLAND_DISPLAY").is_some_and(|v| !v.is_empty())
}

impl Hotkeys {
    /// Register `wanted` (re-registers only when it changed).
    pub fn sync(&mut self, wanted: &[(HotkeyAction, String)]) {
        #[cfg(all(unix, not(target_os = "macos")))]
        if wayland() {
            self.sync_portal(wanted);
            return;
        }
        if self.applied == wanted {
            return;
        }
        if self.mgr.is_none() && self.failed_start.is_none() {
            match GlobalHotKeyManager::new() {
                Ok(m) => self.mgr = Some(m),
                Err(e) => self.failed_start = Some(e.to_string()),
            }
        }
        let Some(mgr) = &self.mgr else {
            self.problems = wanted.iter().map(|(a, _)| (*a, self.failed_start.clone().unwrap_or_default())).collect();
            self.applied = wanted.to_vec();
            return;
        };
        for (_, hk) in self.active.drain(..) {
            let _ = mgr.unregister(hk);
        }
        self.problems.clear();
        let conflicts = cpuzeez_core::live::hotkey_conflicts(wanted);
        for (a, keys) in wanted {
            if normalize_keys(keys).is_none() {
                continue; // off
            }
            if conflicts.iter().any(|c| c.1 == *a) {
                self.problems.push((*a, tr("Same shortcut as another action").into()));
                continue;
            }
            match keys.parse::<HotKey>() {
                Ok(hk) => match mgr.register(hk) {
                    Ok(()) => self.active.push((*a, hk)),
                    Err(_) => self.problems.push((*a, tr("Taken by another app").into())),
                },
                Err(_) => self.problems.push((*a, tr("Not a shortcut CPU ZeeZ understands").into())),
            }
        }
        self.applied = wanted.to_vec();
    }

    /// Wayland: ask the desktop for the shortcuts (the user confirms them there).
    #[cfg(all(unix, not(target_os = "macos")))]
    fn sync_portal(&mut self, wanted: &[(HotkeyAction, String)]) {
        use super::hotkeys_portal::{Portal, Status, specs};
        let portal = self.portal.get_or_insert_with(Portal::start);
        if self.applied != wanted {
            self.problems.clear();
            let conflicts = cpuzeez_core::live::hotkey_conflicts(wanted);
            let usable: Vec<_> = wanted
                .iter()
                .filter(|(a, _)| {
                    let clash = conflicts.iter().any(|c| c.1 == *a);
                    if clash {
                        self.problems.push((*a, tr("Same shortcut as another action").into()));
                    }
                    !clash
                })
                .cloned()
                .collect();
            portal.bind(specs(&usable, |a| tr(a.label()).to_string()));
            self.applied = wanted.to_vec();
        }
        // What the desktop says shows beside the shortcuts, until it is Ready.
        self.portal_note = match portal.status() {
            Status::Ready => None,
            Status::Starting => Some(tr("Waiting for your desktop to confirm the shortcuts").into()),
            Status::Refused => {
                Some(tr("Your desktop did not accept the shortcuts; choose them in its settings").into())
            }
            Status::Unavailable(_) => {
                Some(tr("This desktop has no global shortcuts portal; shortcuts work while CPU ZeeZ has focus").into())
            }
        };
    }

    /// Actions pressed since the last call.
    pub fn poll(&self) -> Vec<HotkeyAction> {
        let mut out = Vec::new();
        #[cfg(all(unix, not(target_os = "macos")))]
        if let Some(p) = &self.portal {
            out.extend(p.poll());
        }
        while let Ok(ev) = GlobalHotKeyEvent::receiver().try_recv() {
            if ev.state() != HotKeyState::Pressed {
                continue;
            }
            if let Some((a, _)) = self.active.iter().find(|(_, hk)| hk.id() == ev.id()) {
                out.push(*a);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    /// The default shortcuts parse with the crate's own parser.
    #[test]
    fn defaults_parse() {
        for a in cpuzeez_core::live::HotkeyAction::ALL {
            assert!(a.default_keys().parse::<global_hotkey::hotkey::HotKey>().is_ok(), "{a:?}");
        }
    }
}
