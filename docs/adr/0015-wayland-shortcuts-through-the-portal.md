# ADR 0015: Wayland global shortcuts go through the XDG portal

- **Status:** accepted (2026-11-07)
- **Decision ref:** Sobras (ZeeZ Live hotkeys) · [zeez-live §5.5](../v1/zeez-live.md)

## Context
The ZeeZ Live hotkeys (show the overlay, capture, marker…) use `global-hotkey`: `RegisterHotKey` on Windows, Carbon on macOS, `XGrabKey` on X11. A Wayland client cannot grab keys, so on Wayland the shortcuts worked only with a CpuZeeZ window focused. That is exactly when they are useless: the user is in a game.

## Decision
On a Wayland session, ask the desktop through the `org.freedesktop.portal.GlobalShortcuts` portal. The app creates a session, binds its five shortcuts (stable ids, a description, a *preferred* trigger), and receives `Activated` signals. The desktop shows the shortcuts and lets the user change the keys; it remembers them. The call runs on background threads (the dialog can wait for the user).

Where the portal is missing (older GNOME, KDE before 5.27, no portal service), the card says so and the shortcuts work with focus, as before. X11, Windows and macOS are unchanged.

## Why
- It is the only supported way: the portal is how GNOME, KDE and wlroots desktops let an app have global shortcuts without a grab.
- No new code in the tree: `zbus` was already compiled for the tray icon (`ksni`); we only use it directly (blocking API). No `ashpd`, no new transitive crates.
- Keys are the user's to choose on Wayland, so the Settings text box is a *preference* there, and the card says what the desktop answered.

## Alternatives considered
- `ashpd`: a good wrapper, but async and heavier, and one more crate to audit for what is about 150 lines of D-Bus.
- Reading `/dev/input` (evdev): needs the `input` group or root and sees every key the user types: against P4 and P1.
- A compositor-specific hook (KDE, GNOME extension): one path per desktop, unmaintainable.

## Consequences
- The exchange with a real portal cannot be tested in CI (no Wayland desktop there). The pure parts (ids, trigger spelling, request path) are tested everywhere; the flow is on the manual checklist for a Wayland tester.
- A user who dismisses the desktop's dialog gets a note, and can bind the shortcuts in the desktop's own settings.
