//! Report tree shared by every probe, the TXT/HTML/JSON writers and the GUI.

use serde::Serialize;
use std::fmt::Write;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Node {
    Field { key: String, value: String },
    Group { title: String, value: String, children: Vec<Node> },
    Hex { title: String, bytes: Vec<u8> },
}

#[derive(Debug, Clone, Serialize)]
pub struct Section {
    pub title: String,
    pub nodes: Vec<Node>,
}

pub fn f(key: impl Into<String>, value: impl ToString) -> Node {
    Node::Field { key: key.into(), value: value.to_string() }
}
pub fn g(title: impl Into<String>, children: Vec<Node>) -> Node {
    Node::Group { title: title.into(), value: String::new(), children }
}
pub fn gv(title: impl Into<String>, value: impl ToString, children: Vec<Node>) -> Node {
    Node::Group { title: title.into(), value: value.to_string(), children }
}
pub fn hex(title: impl Into<String>, bytes: Vec<u8>) -> Node {
    Node::Hex { title: title.into(), bytes }
}
pub fn section(title: impl Into<String>, nodes: Vec<Node>) -> Section {
    Section { title: title.into(), nodes }
}

impl Section {
    /// First field named `key` anywhere in this section (GUI lookup).
    pub fn get(&self, key: &str) -> Option<&str> {
        find(&self.nodes, key)
    }
}

impl Section {
    /// Field `key` inside the first group titled `group` (searched recursively).
    pub fn get_in(&self, group: &str, key: &str) -> Option<&str> {
        groups(&self.nodes, group).into_iter().find_map(|(_, c)| find(c, key))
    }
}

/// Every group titled `title` (prefix match), with its children, depth-first.
pub fn groups<'a>(nodes: &'a [Node], title: &str) -> Vec<(&'a str, &'a [Node])> {
    let mut out = Vec::new();
    for n in nodes {
        if let Node::Group { title: t, value, children } = n {
            if t.starts_with(title) {
                out.push((value.as_str(), children.as_slice()));
            }
            out.extend(groups(children, title));
        }
    }
    out
}

/// All direct fields of a node list, in order.
pub fn fields(nodes: &[Node]) -> impl Iterator<Item = (&str, &str)> {
    nodes.iter().filter_map(|n| match n {
        Node::Field { key, value } => Some((key.as_str(), value.as_str())),
        _ => None,
    })
}

pub fn find<'a>(nodes: &'a [Node], key: &str) -> Option<&'a str> {
    nodes.iter().find_map(|n| match n {
        Node::Field { key: k, value } if k == key => Some(value.as_str()),
        Node::Group { children, .. } => find(children, key),
        _ => None,
    })
}

pub fn find_section<'a>(r: &'a [Section], title: &str) -> Option<&'a Section> {
    r.iter().find(|s| s.title == title)
}

// ---- TXT (same layout as CPU-Z: tab-aligned key column at 32) ----

const RULE: &str = "-------------------------------------------------------------------------";

fn pad(out: &mut String, depth: usize, key: &str) {
    out.push_str(&"\t".repeat(depth));
    out.push_str(key);
    let col = depth * 8 + key.chars().count();
    let tabs = if col >= 32 { 1 } else { (32 - col).div_ceil(8) };
    out.push_str(&"\t".repeat(tabs));
}

fn txt_nodes(out: &mut String, nodes: &[Node], depth: usize) {
    txt_block(out, nodes, depth, true)
}

/// A group with an empty title and value is a blank-line separated block whose
/// children print at the same depth without automatic blank lines (PCI layout).
fn txt_block(out: &mut String, nodes: &[Node], depth: usize, blank: bool) {
    for n in nodes {
        match n {
            Node::Field { key, value } => {
                pad(out, depth, key);
                out.push_str(value);
                out.push('\n');
            }
            Node::Group { title, value, children } if title.is_empty() && value.is_empty() => {
                if !out.ends_with("\n\n") {
                    out.push('\n');
                }
                txt_block(out, children, depth, false);
            }
            Node::Group { title, value, children } => {
                if blank && depth == 0 && !out.ends_with("\n\n") {
                    out.push('\n');
                }
                if blank || !value.is_empty() {
                    pad(out, depth, title);
                } else {
                    out.push_str(&"\t".repeat(depth));
                    out.push_str(title);
                }
                out.push_str(value);
                out.push('\n');
                txt_nodes(out, children, depth + 1);
            }
            Node::Hex { title, bytes } => {
                out.push_str(&"\t".repeat(depth));
                out.push_str(title);
                out.push_str("\t\n"); // CPU-Z: "PCI registers\t", "SPD registers\t"
                hex_rows(out, bytes, depth + 1);
            }
        }
    }
}

fn hex_rows(out: &mut String, bytes: &[u8], depth: usize) {
    let ind = "\t".repeat(depth);
    let _ = write!(out, "{ind}\t");
    for i in 0..16 {
        let _ = write!(out, "{i:02X} ");
    }
    out.push('\n');
    for (row, chunk) in bytes.chunks(16).enumerate() {
        // CPU-Z: "00".."F0", then "100".. in longer dumps (SPD, PCIe config).
        let _ = write!(out, "{ind}{:02X}\t", row * 16);
        for b in chunk {
            let _ = write!(out, "{b:02X} ");
        }
        out.push('\n');
    }
}

pub fn to_txt(report: &[Section]) -> String {
    let mut out = format!("{} TXT Report\n{RULE}\n\n", crate::DISPLAY_NAME);
    for s in report {
        let _ = write!(out, "{}\n{RULE}\n\n", s.title);
        txt_nodes(&mut out, &s.nodes, 0);
        out.push_str("\n\n");
    }
    out
}

// ---- HTML ----

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn html_nodes(out: &mut String, nodes: &[Node]) {
    out.push_str("<table>");
    for n in nodes {
        match n {
            Node::Field { key, value } => {
                let _ = write!(out, "<tr><th>{}</th><td>{}</td></tr>", esc(key), esc(value));
            }
            Node::Group { title, value, children } => {
                let _ = write!(out, "<tr><th class=g>{}</th><td>{}", esc(title), esc(value));
                html_nodes(out, children);
                out.push_str("</td></tr>");
            }
            Node::Hex { title, bytes } => {
                let mut s = String::new();
                hex_rows(&mut s, bytes, 0);
                let _ = write!(out, "<tr><th>{}</th><td><pre>{}</pre></td></tr>", esc(title), esc(&s));
            }
        }
    }
    out.push_str("</table>");
}

pub fn to_html(report: &[Section]) -> String {
    let mut out = format!(
        "<!doctype html><html><head><meta charset=utf-8><title>{n} report</title><style>\
body{{font:13px system-ui,sans-serif;margin:16px;background:#fff;color:#111}}\
h2{{background:#2c3e66;color:#fff;padding:4px 8px;font-size:14px}}\
table{{border-collapse:collapse}}th{{text-align:left;vertical-align:top;padding:1px 12px 1px 0;font-weight:500;color:#444;white-space:nowrap}}\
th.g{{color:#2c3e66;font-weight:700}}td{{padding:1px 0}}pre{{margin:0;font-size:11px}}\
@media(prefers-color-scheme:dark){{body{{background:#16181d;color:#ddd}}th{{color:#aaa}}th.g{{color:#8fb0ff}}}}\
</style></head><body><h1>{n} report</h1>",
        n = crate::DISPLAY_NAME
    );
    for s in report {
        let _ = write!(out, "<h2>{}</h2>", esc(&s.title));
        html_nodes(&mut out, &s.nodes);
    }
    out.push_str("</body></html>\n");
    out
}

pub fn to_json(report: &[Section]) -> String {
    serde_json::to_string_pretty(report).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn txt_layout() {
        let r = vec![section("CPU", vec![f("Name", "X"), g("Cache", vec![f("L1", "32 KB")])])];
        let t = to_txt(&r);
        assert!(t.contains("CPU\n----"));
        assert!(t.contains("Name\t\t\t\tX\n"));
        assert!(t.contains("\tL1\t\t\t32 KB\n"));
        assert_eq!(find_section(&r, "CPU").unwrap().get("L1"), Some("32 KB"));
    }
}
