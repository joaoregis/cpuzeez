//! GUI translations, gettext style: the key is the English text, so a missing
//! translation simply shows English. Reports (`-txt`/`-html`/`-json`) stay in
//! English, like CPU-Z's, so they can be compared and parsed.
//!
//! Each GUI file owns its strings: it declares `pub const PT_BR` / `ES` tables
//! and is listed in `tables()` below.

use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    PtBr,
    Es,
}

pub const LANGS: [(Lang, &str, &str); 3] =
    [(Lang::En, "en", "English"), (Lang::PtBr, "pt-BR", "Português (Brasil)"), (Lang::Es, "es", "Español")];

static CURRENT: AtomicUsize = AtomicUsize::new(0);

type Table = &'static [(&'static str, &'static str)];

/// (pt-BR, es) tables of every GUI file.
fn tables() -> [(Table, Table); 1] {
    [(super::PT_BR, super::ES)]
}

pub fn set(lang: Lang) {
    CURRENT.store(LANGS.iter().position(|l| l.0 == lang).unwrap_or(0), Ordering::Relaxed);
}

pub fn current() -> Lang {
    LANGS[CURRENT.load(Ordering::Relaxed)].0
}

pub fn from_code(code: &str) -> Lang {
    let c = code.to_ascii_lowercase().replace('_', "-");
    if c.starts_with("pt") {
        Lang::PtBr
    } else if c.starts_with("es") {
        Lang::Es
    } else {
        Lang::En
    }
}

/// The OS UI language.
pub fn os_lang() -> Lang {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(var)
            && !v.is_empty()
            && v != "C"
            && v != "POSIX"
        {
            return from_code(&v);
        }
    }
    #[cfg(windows)]
    {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetUserDefaultLocaleName(buf: *mut u16, len: i32) -> i32;
        }
        let mut buf = [0u16; 85];
        let n = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
        if n > 1 {
            return from_code(&String::from_utf16_lossy(&buf[..n as usize - 1]));
        }
    }
    #[cfg(target_os = "macos")]
    if let Some(l) = crate::os::cmd("defaults", &["read", "-g", "AppleLocale"]) {
        return from_code(l.trim());
    }
    Lang::En
}

/// Translate an English UI string.
pub fn tr(en: &'static str) -> &'static str {
    let pick: fn(&(Table, Table)) -> Table = match current() {
        Lang::En => return en,
        Lang::PtBr => |t| t.0,
        Lang::Es => |t| t.1,
    };
    tables().iter().map(pick).find_map(|t| t.iter().find(|(k, _)| *k == en).map(|(_, v)| *v)).unwrap_or(en)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_and_fallback() {
        assert_eq!(from_code("pt_BR.UTF-8"), Lang::PtBr);
        assert_eq!(from_code("es-ES"), Lang::Es);
        assert_eq!(from_code("en-US"), Lang::En);
        set(Lang::PtBr);
        assert_eq!(tr("__never translated__"), "__never translated__");
        set(Lang::En);
    }
}
