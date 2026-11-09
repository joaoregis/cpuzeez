//! Copy and pin (S1-U03): every label/value row can be copied as a value,
//! as a row or as its whole card in report text, and pinned to the Overview.
//! Rows pin their text (refreshed whenever their page draws them); sensor
//! rows pin their metric id, so the Overview shows them live.

use super::i18n::tr;
use crate::settings::Pin;
use eframe::egui;
use std::collections::HashMap;

pub const PT_BR: &[(&str, &str)] = &[
    ("Copy value", "Copiar valor"),
    ("Copy row", "Copiar linha"),
    ("Copy card as text", "Copiar cartão como texto"),
    ("Pin to Overview", "Fixar na Visão geral"),
    ("Pinned", "Fixados"),
    ("Unpin", "Desafixar"),
    ("Already pinned", "Já fixado"),
];

pub const ES: &[(&str, &str)] = &[
    ("Copy value", "Copiar valor"),
    ("Copy row", "Copiar fila"),
    ("Copy card as text", "Copiar tarjeta como texto"),
    ("Pin to Overview", "Fijar en Resumen"),
    ("Pinned", "Fijados"),
    ("Unpin", "Quitar"),
    ("Already pinned", "Ya fijado"),
];

fn card_id() -> egui::Id {
    egui::Id::new("cz-card-title")
}
fn requests_id() -> egui::Id {
    egui::Id::new("cz-pin-requests")
}
fn cache_id() -> egui::Id {
    egui::Id::new("cz-pin-values")
}
fn pinned_id() -> egui::Id {
    egui::Id::new("cz-pinned-keys")
}

/// The card being drawn (set by `widgets::card` around its body).
pub fn enter_card(ctx: &egui::Context, title: &str) -> Option<String> {
    ctx.data_mut(|d| {
        let prev = d.get_temp::<String>(card_id());
        d.insert_temp(card_id(), title.to_string());
        prev
    })
}

pub fn leave_card(ctx: &egui::Context, prev: Option<String>) {
    ctx.data_mut(|d| match prev {
        Some(p) => {
            d.insert_temp(card_id(), p);
        }
        None => d.remove::<String>(card_id()),
    });
}

pub fn current_card(ctx: &egui::Context) -> String {
    ctx.data(|d| d.get_temp::<String>(card_id())).unwrap_or_default()
}

/// Asks the app to pin `p` (it owns the settings).
pub fn request(ctx: &egui::Context, p: Pin) {
    ctx.data_mut(|d| d.get_temp_mut_or_default::<Vec<Pin>>(requests_id()).push(p));
}

pub fn take_requests(ctx: &egui::Context) -> Vec<Pin> {
    ctx.data_mut(|d| d.remove_temp::<Vec<Pin>>(requests_id())).unwrap_or_default()
}

/// The pins' (card, label) keys, published by the app each frame so rows
/// can say "Already pinned".
pub fn publish(ctx: &egui::Context, pins: &[Pin]) {
    let keys: Vec<(String, String)> = pins.iter().map(|p| (p.card.clone(), p.label.clone())).collect();
    ctx.data_mut(|d| d.insert_temp(pinned_id(), keys));
}

fn is_pinned(ctx: &egui::Context, card: &str, label: &str) -> bool {
    ctx.data(|d| d.get_temp::<Vec<(String, String)>>(pinned_id()))
        .is_some_and(|k| k.iter().any(|(c, l)| c == card && l == label))
}

/// A pinned row's latest text, refreshed whenever its page draws it.
fn remember(ctx: &egui::Context, card: &str, label: &str, value: &str) {
    if is_pinned(ctx, card, label) {
        ctx.data_mut(|d| {
            d.get_temp_mut_or_default::<HashMap<(String, String), String>>(cache_id())
                .insert((card.to_string(), label.to_string()), value.to_string())
        });
    }
}

pub fn latest(ctx: &egui::Context, card: &str, label: &str) -> Option<String> {
    ctx.data(|d| d.get_temp::<HashMap<(String, String), String>>(cache_id()))
        .and_then(|m| m.get(&(card.to_string(), label.to_string())).cloned())
}

/// The card as report text: title, then aligned rows.
pub fn card_text(title: &str, rows: &[(&str, String)]) -> String {
    let w = rows.iter().map(|(k, _)| k.chars().count()).max().unwrap_or(0);
    let mut s = format!("{title}\n");
    for (k, v) in rows {
        s += &format!("  {k:<w$}  {v}\n");
    }
    s
}

/// The right-click menu of one label/value row inside a card.
pub fn row_menu(resp: &egui::Response, label: &str, value: &str, rows: &[(&str, String)]) {
    let ctx = resp.ctx.clone();
    let card = current_card(&ctx);
    remember(&ctx, &card, label, value);
    resp.context_menu(|ui| {
        if ui.button(tr("Copy value")).clicked() {
            ui.ctx().copy_text(value.to_string());
            ui.close();
        }
        if ui.button(tr("Copy row")).clicked() {
            ui.ctx().copy_text(format!("{label}: {value}"));
            ui.close();
        }
        if ui.button(tr("Copy card as text")).clicked() {
            ui.ctx().copy_text(card_text(&card, rows));
            ui.close();
        }
        ui.separator();
        let pinned = is_pinned(&ctx, &card, label);
        let text = if pinned { tr("Already pinned") } else { tr("Pin to Overview") };
        if ui.add_enabled(!pinned, egui::Button::new(text)).clicked() {
            request(&ctx, Pin { card: card.clone(), label: label.to_string(), value: value.to_string(), metric: None });
            ui.close();
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn card_text_aligns_rows() {
        let t = super::card_text("Processor", &[("Name", "Ryzen".into()), ("Max TDP", "170 W".into())]);
        assert_eq!(t, "Processor\n  Name     Ryzen\n  Max TDP  170 W\n");
    }
}
