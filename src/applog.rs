//! The local app log (docs/v1/engineering.md §9): `<data>/logs/app.log`,
//! rotated at 1 MB with one previous file kept. It records startup facts,
//! warnings and panics, which would otherwise vanish (the GUI has no console).
//! Nothing personal is written, and nothing leaves the machine; the user can
//! attach it to an issue through a diagnostics bundle.

use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

const MAX: u64 = 1 << 20;

static LOCK: Mutex<()> = Mutex::new(());

pub fn path() -> Option<PathBuf> {
    crate::settings::dir().map(|d| d.join("logs").join("app.log"))
}

fn write(level: &str, msg: &str) {
    let Some(p) = path() else { return };
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if std::fs::metadata(&p).is_ok_and(|m| m.len() > MAX) {
        let _ = std::fs::rename(&p, p.with_extension("log.1"));
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) {
        let t = cpuzeez_core::metrics::iso8601(std::time::SystemTime::now());
        let _ = writeln!(f, "{t} {level:<5} {}", msg.replace('\n', " | "));
    }
}

pub fn info(msg: &str) {
    write("INFO", msg);
}

pub fn warn(msg: &str) {
    eprintln!("{msg}");
    write("WARN", msg);
}

/// Log panics (thread name, message, location) before the default handler runs.
pub fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current().name().unwrap_or("unnamed").to_string();
        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned());
        let at = info.location().map(|l| format!(" at {}:{}", l.file(), l.line())).unwrap_or_default();
        write("PANIC", &format!("thread '{thread}': {}{at}", msg.unwrap_or_else(|| "?".into())));
        prev(info);
    }));
}

/// A folder the user can attach to an issue: the app log and the report with
/// serials, MAC and IP addresses masked. Written locally only.
pub fn diagnostics_bundle(report: &[cpuzeez_core::report::Section]) -> std::io::Result<PathBuf> {
    use cpuzeez_core::report;
    let base = crate::settings::dir().ok_or_else(|| std::io::Error::other("no data folder"))?;
    let stamp = cpuzeez_core::metrics::iso8601(std::time::SystemTime::now()).replace([':', '.'], "-");
    let dir = base.join("diagnostics").join(stamp);
    std::fs::create_dir_all(&dir)?;
    let masked = report::masked(report);
    std::fs::write(dir.join("report.txt"), report::to_txt(&masked))?;
    std::fs::write(dir.join("report.json"), report::to_json(&masked))?;
    if let Some(p) = path() {
        for (src, name) in [(p.clone(), "app.log"), (p.with_extension("log.1"), "app.log.1")] {
            if src.exists() {
                std::fs::copy(&src, dir.join(name))?;
            }
        }
    }
    let readme = format!(
        "CpuZeeZ {} diagnostics, made on request.\n\
         report.txt / report.json: this computer's hardware report, with serial numbers, UUIDs, MAC and IP addresses removed.\n\
         app.log: the app's own log (startup, warnings, crashes).\n\
         Nothing here was sent anywhere. Attach it to an issue only if you want to.\n",
        crate::VERSION
    );
    std::fs::write(dir.join("README.txt"), readme)?;
    Ok(dir)
}
