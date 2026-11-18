//! Preferences ▸ Shortcuts: keys that add an atomic node or a graph item under the pointer

use egui::{Key, RichText, Ui};
use egui_phosphor::regular as icon;
use materialcraft_engine::node_library::LibraryItem;

use super::heading;
use crate::MaterialCraftApp;
use crate::i18n::tr;
use crate::theme::Tokens;

const ROW_HEIGHT: f32 = 28.0;
const ICON_SIDE: f32 = 22.0;
const LABEL_WIDTH: f32 = 260.0;
const FIELD_WIDTH: f32 = 220.0;

pub(super) fn show(app: &mut MaterialCraftApp, ui: &mut Ui) {
    let tokens = Tokens::get(ui.ctx());
    ui.horizontal(|ui| {
        heading(ui, "Node creation shortcuts");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let any = !app.ui.prefs.node_shortcuts.is_empty();
            if ui
                .add_enabled_ui(any, |ui| {
                    crate::widgets::icon_button(ui, icon::ARROW_COUNTER_CLOCKWISE, tr("Remove every node creation shortcut"), false, 22.0)
                })
                .inner
                .clicked()
            {
                app.ui.prefs.node_shortcuts.clear();
            }
        });
    });
    ui.label(RichText::new(tr("For the material graph: press the keys over the graph to add the node under the pointer")).color(tokens.text_dim));

    let state = &mut app.ui.preferences;
    ui.horizontal(|ui| {
        let search = ui.add(egui::TextEdit::singleline(&mut state.shortcut_search).hint_text(tr("Search")).desired_width(LABEL_WIDTH));
        crate::widgets::name(&search, egui::Role::SearchInput, "Search");
        let by_key = ui.add(egui::TextEdit::singleline(&mut state.shortcut_key_search).hint_text(tr("Search by shortcut")).desired_width(FIELD_WIDTH));
        crate::widgets::name(&by_key, egui::Role::SearchInput, "Search by shortcut");
        ui.toggle_value(&mut state.only_assigned, tr("Show all assigned"));
    });
    ui.add_space(4.0);

    let (search, key_search, only_assigned) = (state.shortcut_search.to_lowercase(), state.shortcut_key_search.to_lowercase(), state.only_assigned);
    let entries: Vec<_> = app
        .session
        .library_entries()
        .into_iter()
        .filter(|entry| entry.missing.is_none() && crate::node_shortcuts::CATEGORIES.contains(&entry.category.as_str()))
        .filter(|entry| search.is_empty() || entry.label.to_lowercase().contains(&search))
        .filter(|entry| {
            let shortcut = app.ui.prefs.node_shortcuts.get(&entry.id).map(|text| text.to_lowercase());
            (!only_assigned || shortcut.is_some()) && (key_search.is_empty() || shortcut.is_some_and(|text| text.contains(&key_search)))
        })
        .collect();

    capture(app, ui);
    egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, ROW_HEIGHT, entries.len(), |ui, rows| {
        for entry in &entries[rows] {
            ui.horizontal(|ui| {
                ui.set_height(ROW_HEIGHT);
                let picture = match &entry.item {
                    LibraryItem::Node { def } if def != "dot" => app.thumbs.library(ui.ctx(), def),
                    _ => None,
                };
                match picture {
                    Some(texture) => {
                        ui.add(egui::Image::new((texture, egui::vec2(ICON_SIDE, ICON_SIDE))).corner_radius(3));
                    }
                    None => {
                        ui.add_sized([ICON_SIDE, ICON_SIDE], egui::Label::new(RichText::new(icon::FLOW_ARROW).color(tokens.text_dim)));
                    }
                }
                let left = egui::Layout::left_to_right(egui::Align::Center);
                ui.allocate_ui_with_layout(egui::vec2(LABEL_WIDTH, ROW_HEIGHT), left, |ui| {
                    ui.set_min_width(LABEL_WIDTH);
                    ui.add(egui::Label::new(tr(&entry.label)).truncate());
                });
                field(app, ui, &entry.id, &entry.label);
            });
        }
    });
}

/// The button that shows an entry's shortcut and, clicked, waits for new keys
fn field(app: &mut MaterialCraftApp, ui: &mut Ui, entry: &str, label: &str) {
    let waiting = app.ui.preferences.capturing.as_deref() == Some(entry);
    let text = if waiting {
        tr("Press keys, Backspace to clear, Esc to cancel").to_string()
    } else {
        app.ui.prefs.node_shortcuts.get(entry).map(|text| crate::menus::shortcut_label(ui.ctx(), text)).unwrap_or_default()
    };

    // A field-like button, framed even while empty, so every row shows where its keys go
    let tokens = Tokens::get(ui.ctx());
    let fill = if waiting { tokens.tool_active } else { tokens.field };
    let field = egui::Button::new(text).fill(fill).stroke(egui::Stroke::new(1.0, tokens.field_border));
    let button = ui.add_sized([FIELD_WIDTH, ROW_HEIGHT - 6.0], field);
    crate::widgets::name(&button, egui::Role::Button, &format!("Shortcut for {label}"));
    if button.clicked() {
        app.ui.preferences.capturing = Some(entry.to_string());
    }
}

/// While a field waits, the next key press with its modifiers becomes that entry's shortcut
fn capture(app: &mut MaterialCraftApp, ui: &mut Ui) {
    let Some(entry) = app.ui.preferences.capturing.clone() else { return };

    let pressed = ui.input(|input| {
        input.events.iter().find_map(|event| match event {
            egui::Event::Key { key, pressed: true, modifiers, .. } => Some((*modifiers, *key)),
            _ => None,
        })
    });
    let Some((modifiers, key)) = pressed else { return };

    app.ui.preferences.capturing = None;
    if modifiers.is_none() && key == Key::Escape {
        return;
    }
    if modifiers.is_none() && matches!(key, Key::Backspace | Key::Delete) {
        app.ui.prefs.node_shortcuts.remove(&entry);
        return;
    }

    let Some(text) = crate::node_shortcuts::text(modifiers, key) else { return };

    if let Some(command) = crate::node_shortcuts::taken_by(&text) {
        app.toast(format!("{text} {} {command}", tr("is already the shortcut of")), true);
        return;
    }

    // A shortcut adds one entry: giving it to this one takes it from any other
    app.ui.prefs.node_shortcuts.retain(|_, assigned| *assigned != text);
    app.ui.prefs.node_shortcuts.insert(entry, text);
}
