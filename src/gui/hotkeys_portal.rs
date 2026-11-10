//! Global shortcuts on Wayland through the XDG GlobalShortcuts portal
//! (docs/adr/0015). A Wayland client cannot grab keys; it asks the desktop,
//! which shows the user the shortcuts, lets them pick the keys, and sends an
//! `Activated` signal when one is pressed.
//!
//! The flow, all on background threads so the UI never waits for a dialog:
//! 1. `CreateSession`, answered by a `Response` signal on a request object;
//! 2. `BindShortcuts` with our ids, descriptions and a preferred trigger (a
//!    hint: the user may choose other keys, and the desktop remembers them);
//! 3. `Activated(session, id, time, options)` for each press.
//!
//! The pure parts (ids, trigger spelling, request path) are in
//! `cpuzeez_core::live` and tested everywhere. The exchange with a real
//! portal needs a Wayland desktop, so it is checked by hand (docs/testing).

use cpuzeez_core::live::{HotkeyAction, portal_request_path};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::{ObjectPath, OwnedValue, Value};

const DEST: &str = "org.freedesktop.portal.Desktop";
const PATH: &str = "/org/freedesktop/portal/desktop";
const IFACE: &str = "org.freedesktop.portal.GlobalShortcuts";
const REQUEST: &str = "org.freedesktop.portal.Request";

/// One shortcut to ask for.
#[derive(Debug, Clone, PartialEq)]
pub struct Spec {
    pub id: &'static str,
    pub description: String,
    pub trigger: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    /// Connecting, or waiting for the desktop to answer.
    Starting,
    /// The desktop accepted the shortcuts (the user may have changed the keys).
    Ready,
    /// There is no portal, or it has no GlobalShortcuts (older desktops).
    Unavailable(String),
    /// The user dismissed the desktop's dialog.
    Refused,
}

pub struct Portal {
    cmd: Sender<Vec<Spec>>,
    events: Receiver<HotkeyAction>,
    status: Arc<Mutex<Status>>,
}

impl Portal {
    /// Connect on a thread; `bind` hands it the shortcuts.
    pub fn start() -> Portal {
        Portal::start_on(Connection::session)
    }

    fn start_on(connect: impl FnOnce() -> zbus::Result<Connection> + Send + 'static) -> Portal {
        let (cmd, cmd_rx) = channel::<Vec<Spec>>();
        let (ev_tx, events) = channel();
        let status = Arc::new(Mutex::new(Status::Starting));
        let st = status.clone();
        std::thread::Builder::new()
            .name("cz-portal".into())
            .spawn(move || {
                let set = |s: Status| *st.lock().unwrap() = s;
                let conn = match connect() {
                    Ok(c) => c,
                    Err(e) => return set(Status::Unavailable(e.to_string())),
                };
                let session = match create_session(&conn) {
                    Ok(s) => s,
                    Err(e) => return set(Status::Unavailable(e)),
                };
                listen(conn.clone(), ev_tx);
                for specs in cmd_rx {
                    set(Status::Starting);
                    set(match bind(&conn, &session, &specs) {
                        Ok(true) => Status::Ready,
                        Ok(false) => Status::Refused,
                        Err(e) => Status::Unavailable(e),
                    });
                }
            })
            .ok();
        Portal { cmd, events, status }
    }

    /// Ask for these shortcuts (replaces the previous list).
    pub fn bind(&self, specs: Vec<Spec>) {
        let _ = self.cmd.send(specs);
    }

    pub fn status(&self) -> Status {
        self.status.lock().unwrap().clone()
    }

    /// Actions pressed since the last call.
    pub fn poll(&self) -> Vec<HotkeyAction> {
        self.events.try_iter().collect()
    }
}

static TOKEN: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

fn token() -> String {
    format!("cpuzeez_{}", TOKEN.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
}

fn portal(conn: &Connection) -> Result<Proxy<'static>, String> {
    Proxy::new(conn, DEST, PATH, IFACE).map_err(|e| e.to_string())
}

/// Call a portal method that answers through a `Request` object: subscribe to
/// its `Response` first, make the call, wait for the answer. Returns the
/// response code (0 = ok) and the results.
fn request(
    conn: &Connection,
    token: &str,
    call: impl FnOnce() -> zbus::Result<()>,
) -> Result<(u32, HashMap<String, OwnedValue>), String> {
    let unique = conn.unique_name().ok_or("no unique name on the bus")?.to_string();
    let path = portal_request_path(&unique, token);
    let req = Proxy::new(conn, DEST, path, REQUEST).map_err(|e| e.to_string())?;
    let mut answers = req.receive_signal("Response").map_err(|e| e.to_string())?;
    call().map_err(|e| e.to_string())?;
    let msg = answers.next().ok_or("the portal closed the request")?;
    msg.body().deserialize::<(u32, HashMap<String, OwnedValue>)>().map_err(|e| e.to_string())
}

fn create_session(conn: &Connection) -> Result<zbus::zvariant::OwnedObjectPath, String> {
    let gp = portal(conn)?;
    let tok = token();
    let mut opts: HashMap<&str, Value<'_>> = HashMap::new();
    opts.insert("handle_token", Value::from(tok.as_str()));
    opts.insert("session_handle_token", Value::from(token()));
    let (code, results) = request(conn, &tok, || gp.call_method("CreateSession", &(opts,)).map(|_| ()))?;
    if code != 0 {
        return Err(format!("the portal refused the session (code {code})"));
    }
    let handle = results.get("session_handle").ok_or("no session handle in the answer")?;
    let text: String =
        handle.try_clone().map_err(|e| e.to_string())?.try_into().map_err(|e: zbus::zvariant::Error| e.to_string())?;
    zbus::zvariant::OwnedObjectPath::try_from(text).map_err(|e| e.to_string())
}

/// `BindShortcuts`; Ok(false) when the user dismissed the desktop's dialog.
fn bind(conn: &Connection, session: &zbus::zvariant::OwnedObjectPath, specs: &[Spec]) -> Result<bool, String> {
    let gp = portal(conn)?;
    let tok = token();
    let shortcuts: Vec<(&str, HashMap<&str, Value<'_>>)> = specs
        .iter()
        .map(|s| {
            let mut m: HashMap<&str, Value<'_>> = HashMap::new();
            m.insert("description", Value::from(s.description.as_str()));
            m.insert("preferred_trigger", Value::from(s.trigger.as_str()));
            (s.id, m)
        })
        .collect();
    let mut opts: HashMap<&str, Value<'_>> = HashMap::new();
    opts.insert("handle_token", Value::from(tok.as_str()));
    let parent = "";
    let (code, _) = request(conn, &tok, || {
        gp.call_method("BindShortcuts", &(session.as_ref(), shortcuts, parent, opts)).map(|_| ())
    })?;
    Ok(code == 0)
}

/// Forward `Activated` signals as actions, on its own thread.
fn listen(conn: Connection, out: Sender<HotkeyAction>) {
    std::thread::Builder::new()
        .name("cz-portal-keys".into())
        .spawn(move || {
            let Ok(gp) = portal(&conn) else { return };
            let Ok(signals) = gp.receive_signal("Activated") else { return };
            for msg in signals {
                let Ok((_session, id, _time, _opts)) =
                    msg.body().deserialize::<(ObjectPath<'_>, String, u64, HashMap<String, OwnedValue>)>()
                else {
                    continue;
                };
                if let Some(a) = HotkeyAction::from_portal_id(&id)
                    && out.send(a).is_err()
                {
                    return; // the app dropped the portal
                }
            }
        })
        .ok();
}

/// The shortcuts for these bindings: off ones are left out.
pub fn specs(wanted: &[(HotkeyAction, String)], describe: impl Fn(HotkeyAction) -> String) -> Vec<Spec> {
    wanted
        .iter()
        .filter_map(|(a, keys)| {
            Some(Spec {
                id: a.portal_id(),
                description: describe(*a),
                trigger: cpuzeez_core::live::portal_trigger(keys)?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_bindings_are_not_requested() {
        let wanted =
            vec![(HotkeyAction::ToggleOverlay, "Ctrl+Shift+O".to_string()), (HotkeyAction::Capture, String::new())];
        let s = specs(&wanted, |a| a.label().to_string());
        assert_eq!(s.len(), 1);
        assert_eq!((s[0].id, s[0].trigger.as_str()), ("toggle-overlay", "CTRL+SHIFT+o"));
    }

    /// With no session bus the portal reports itself unavailable, never panics or hangs.
    #[test]
    fn no_bus_means_unavailable() {
        let p = Portal::start_on(|| Err(zbus::Error::Failure("no bus".into())));
        // A deadline, not a count: a busy CI runner may take a while to start the thread.
        let t0 = std::time::Instant::now();
        while p.status() == Status::Starting && t0.elapsed() < std::time::Duration::from_secs(10) {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(matches!(p.status(), Status::Unavailable(_)), "{:?}", p.status());
        p.bind(Vec::new()); // sending after the thread ended is harmless
        assert!(p.poll().is_empty());
    }
}
