//! The filter syntax of every table (docs/v1/task-manager.md §3.3):
//!
//! - `chrome`: any column contains it (case-insensitive)
//! - `name:chrome`: that column contains it
//! - `pid:1234` / `pid=1234`: equals (numbers compare as numbers)
//! - `cpu>10`, `mem>=1GB`, `temp<80`: numeric comparison, with units
//!   (`KB`, `MB`, `GB`, `TB`, `%`, `ms`, `s`, `°C`, `MHz`, `GHz`, `W`)
//! - `"not responding"` / `status:"not responding"`: quoted phrases
//! - `-svchost` / `-name:svchost`: exclude
//!
//! Terms are ANDed. A malformed term is matched as plain text, never an error,
//! so typing never breaks the table.

#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    Contains,
    Eq,
    Gt,
    Ge,
    Lt,
    Le,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Term {
    /// Column id, lowercased; None = any column.
    pub field: Option<String>,
    pub op: Op,
    /// Lowercased text value.
    pub text: String,
    /// The value as a number in base units (bytes, %, ms…), when it parses.
    pub num: Option<f64>,
    pub negate: bool,
}

/// A cell value as the filter sees it.
#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    Text(&'a str),
    /// Number in the column's base unit (bytes for sizes, % for loads…).
    Num(f64),
}

pub fn parse(q: &str) -> Vec<Term> {
    tokens(q).into_iter().filter_map(|t| term(&t)).collect()
}

/// Split on whitespace, keeping "quoted phrases" (also after `field:`) together.
fn tokens(q: &str) -> Vec<String> {
    let (mut out, mut cur, mut quoted) = (Vec::new(), String::new(), false);
    for c in q.chars() {
        match c {
            '"' => quoted = !quoted,
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn term(t: &str) -> Option<Term> {
    let (negate, t) = match t.strip_prefix('-') {
        Some(rest) if !rest.is_empty() && !rest.starts_with(|c: char| c.is_ascii_digit()) => (true, rest),
        _ => (false, t),
    };
    // Longest operators first so ">=" isn't read as ">".
    for (sym, op) in [(">=", Op::Ge), ("<=", Op::Le), (">", Op::Gt), ("<", Op::Lt), ("=", Op::Eq), (":", Op::Contains)]
    {
        if let Some(i) = t.find(sym)
            && i > 0
            && t[..i].chars().all(|c| c.is_alphanumeric() || c == '_')
        {
            let (field, value) = (t[..i].to_lowercase(), &t[i + sym.len()..]);
            if value.is_empty() {
                return None; // still being typed
            }
            let num = number(value);
            let op = match op {
                // `field:123` means equals when both sides are numbers.
                Op::Contains if num.is_some() => Op::Eq,
                Op::Gt | Op::Ge | Op::Lt | Op::Le if num.is_none() => return Some(plain(t, negate)),
                o => o,
            };
            return Some(Term { field: Some(field), op, text: value.to_lowercase(), num, negate });
        }
    }
    Some(plain(t, negate))
}

fn plain(t: &str, negate: bool) -> Term {
    Term { field: None, op: Op::Contains, text: t.to_lowercase(), num: None, negate }
}

/// "1.5GB" → bytes, "80%" → 80, "2s" → 2000 (ms), "4.8GHz" → 4800 (MHz)…
pub fn number(s: &str) -> Option<f64> {
    let s = s.trim().to_lowercase().replace(',', ".");
    let split = s.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-')).unwrap_or(s.len());
    let (n, unit) = s.split_at(split);
    let n: f64 = n.parse().ok()?;
    let k = 1024.0;
    let mul = match unit.trim() {
        "" | "%" | "ms" | "°c" | "c" | "mhz" | "w" | "rpm" | "v" => 1.0,
        "b" => 1.0,
        "k" | "kb" | "kib" => k,
        "m" | "mb" | "mib" => k * k,
        "g" | "gb" | "gib" => k * k * k,
        "t" | "tb" | "tib" => k * k * k * k,
        "s" => 1000.0,
        "ghz" => 1000.0,
        _ => return None,
    };
    Some(n * mul)
}

impl Term {
    /// Does a row match, given a lookup of its cells by column id?
    pub fn matches<'a>(&self, cell: impl Fn(&str) -> Option<Value<'a>>, all: &[Value<'a>]) -> bool {
        let hit = match &self.field {
            None => all.iter().any(|v| self.test(v)),
            Some(f) => cell(f).is_some_and(|v| self.test(&v)),
        };
        hit != self.negate
    }

    fn test(&self, v: &Value) -> bool {
        match (v, self.num) {
            (Value::Num(x), Some(n)) => match self.op {
                Op::Eq => (x - n).abs() < 1e-9 * n.abs().max(1.0),
                Op::Gt => *x > n,
                Op::Ge => *x >= n,
                Op::Lt => *x < n,
                Op::Le => *x <= n,
                Op::Contains => false,
            },
            (Value::Num(x), None) => format!("{x}").contains(&self.text),
            (Value::Text(t), _) => {
                let t = t.to_lowercase();
                match self.op {
                    Op::Eq => t == self.text,
                    Op::Contains => t.contains(&self.text),
                    _ => false,
                }
            }
        }
    }
}

/// All terms (ANDed) against one row.
pub fn matches<'a>(terms: &[Term], cell: impl Fn(&str) -> Option<Value<'a>> + Copy, all: &[Value<'a>]) -> bool {
    terms.iter().all(|t| t.matches(cell, all))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row<'a>(name: &'a str, pid: f64, cpu: f64, mem: f64) -> (Vec<(&'static str, Value<'a>)>, Vec<Value<'a>>) {
        let cells = vec![
            ("name", Value::Text(name)),
            ("pid", Value::Num(pid)),
            ("cpu", Value::Num(cpu)),
            ("mem", Value::Num(mem)),
        ];
        let all = cells.iter().map(|c| c.1.clone()).collect();
        (cells, all)
    }

    fn hit(q: &str, r: &(Vec<(&'static str, Value<'_>)>, Vec<Value<'_>>)) -> bool {
        let terms = parse(q);
        matches(&terms, |f| r.0.iter().find(|c| c.0 == f).map(|c| c.1.clone()), &r.1)
    }

    #[test]
    fn syntax() {
        let chrome = row("chrome.exe", 1234.0, 12.5, 1.5 * 1024.0 * 1024.0 * 1024.0);
        assert!(hit("CHROME", &chrome), "plain text, any column, case-insensitive");
        assert!(hit("name:chrome", &chrome));
        assert!(
            hit("pid:1234", &chrome) && hit("pid=1234", &chrome) && !hit("pid:123", &chrome),
            "numbers compare whole"
        );
        assert!(hit("cpu>10", &chrome) && !hit("cpu>=12.6", &chrome) && hit("cpu<=12.5", &chrome));
        assert!(hit("mem>1GB", &chrome) && !hit("mem>2GB", &chrome) && hit("mem<1600MB", &chrome));
        assert!(hit("chrome cpu>10", &chrome) && !hit("chrome cpu>50", &chrome), "terms are ANDed");
        assert!(!hit("-chrome", &chrome) && hit("-name:svchost", &chrome), "exclusion");
        assert!(hit("name:\"chrome.exe\"", &chrome), "quotes after field:");
    }

    #[test]
    fn half_typed_and_odd_input_never_fails() {
        let r = row("not responding app", 1.0, 0.0, 0.0);
        assert!(hit("\"not responding\"", &r));
        assert!(hit("name:", &r), "empty value: ignored while typing");
        assert!(!hit("cpu>abc", &r), "non-numeric comparison is plain text");
        assert!(hit("", &r) && hit("   ", &r));
        assert_eq!(number("4.8GHz"), Some(4800.0));
        assert_eq!(number("2s"), Some(2000.0));
        assert_eq!(number("80%"), Some(80.0));
        assert_eq!(number("lots"), None);
        assert!(!parse("-5")[0].negate, "a negative number is not an exclusion");
    }
}
