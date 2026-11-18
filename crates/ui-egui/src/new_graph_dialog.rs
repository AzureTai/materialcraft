//! File ▸ New ▸ Graph: template cards by category on the left; the graph's name, size and the
//! package it goes in on the right

use egui::{RichText, Ui, vec2};
use egui_phosphor::regular as icon;
use materialcraft_engine::core::GraphKind;
use serde_json::json;

use crate::MaterialCraftApp;
use crate::i18n::tr;
use crate::template_card::{CARD_SIZE, FUNCTION_TEMPLATE, TemplateCard};
use crate::theme::Tokens;

const WINDOW_SIZE: egui::Vec2 = vec2(1000.0, 640.0);
const SETTINGS_WIDTH: f32 = 340.0;
const CARD_GAP: f32 = 14.0;
const BUTTON_WIDTH: f32 = 120.0;
/// The sides a new graph can have, in pixels
const SIDES: [u32; 6] = [256, 512, 1024, 2048, 4096, 8192];
/// The categories the dialog lists: every template, or the templates of one kind of graph
const CATEGORIES: [(Option<GraphKind>, &str); 3] =
    [(None, "All templates"), (Some(GraphKind::Material), "Material graph"), (Some(GraphKind::Function), "Function graph")];

/// Open the dialog for a kind of graph, aimed at the active package (or a new one when none is open)
pub(crate) fn open(app: &mut MaterialCraftApp, kind: GraphKind) {
    app.ui.new_graph_kind = kind;
    app.ui.new_graph_category = Some(kind);
    if kind == GraphKind::Function {
        app.ui.new_graph_template = FUNCTION_TEMPLATE.into();
    } else if app.ui.new_graph_template == FUNCTION_TEMPLATE {
        app.ui.new_graph_template = "pbr".into();
    }
    app.ui.new_graph_package = app.session.has_package().then_some(app.session.document_id);
    app.ui.windows.new_graph = true;
}

/// Pick a card: its kind decides the graph's kind
fn choose(app: &mut MaterialCraftApp, card: &TemplateCard) {
    app.ui.new_graph_kind = card.kind;
    app.ui.new_graph_template = card.id.to_string();
}

pub(crate) fn show(app: &mut MaterialCraftApp, ctx: &egui::Context) {
    let mut open = true;
    let mut create = false;
    let title = if app.ui.new_graph_kind == GraphKind::Function { tr("New function graph") } else { tr("New graph") };
    crate::windows::modal(ctx, "new-graph", title, &mut open, WINDOW_SIZE, |ui| {
        ui.set_min_size(WINDOW_SIZE);
        let whole = ui.available_rect_before_wrap();
        let cards = egui::Rect::from_min_max(whole.min, egui::pos2(whole.right() - SETTINGS_WIDTH - 24.0, whole.bottom()));
        let settings = egui::Rect::from_min_max(egui::pos2(whole.right() - SETTINGS_WIDTH, whole.top()), whole.max);

        card_grid(app, &mut ui.new_child(egui::UiBuilder::new().max_rect(cards)));
        create = graph_settings(app, &mut ui.new_child(egui::UiBuilder::new().max_rect(settings)));
        ui.advance_cursor_after_rect(whole);
    });

    if create {
        open = !run(app);
    }
    if !open {
        app.ui.windows.new_graph = false;
    }
}

/// Create the graph. Returns true when it was created
fn run(app: &mut MaterialCraftApp) -> bool {
    let target = app.ui.new_graph_package;
    if let Some(document) = target
        && document != app.session.document_id
        && app.run("package.activate", json!({"document": document})).is_none()
    {
        return false;
    }

    let kind = if app.ui.new_graph_kind == GraphKind::Function { "function" } else { "material" };
    let height = if app.ui.new_graph_linked { app.ui.new_graph_size } else { app.ui.new_graph_height };
    let parameters = json!({
        "kind": kind,
        "template": app.ui.new_graph_template,
        "name": app.ui.new_graph_name,
        "size": app.ui.new_graph_size,
        "height": height,
        "newPackage": target.is_none(),
    });
    let created = app.run("file.new", parameters).is_some();
    if created {
        app.ui.frame_all = true;
    }
    created
}

/// The category list and the cards of the chosen category
fn card_grid(app: &mut MaterialCraftApp, ui: &mut Ui) {
    let tokens = Tokens::get(ui.ctx());
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr("Category")).strong());
        let current = CATEGORIES.iter().find(|(kind, _)| *kind == app.ui.new_graph_category).map_or("", |(_, label)| label);
        let combo = egui::ComboBox::from_id_salt("new-graph-category").width(220.0).selected_text(tr(current)).show_ui(ui, |ui| {
            for (kind, label) in CATEGORIES {
                ui.selectable_value(&mut app.ui.new_graph_category, kind, tr(label));
            }
        });
        crate::widgets::name(&combo.response, egui::Role::ComboBox, "Category");
    });
    ui.add_space(10.0);

    let cards: Vec<TemplateCard> =
        crate::template_card::cards().into_iter().filter(|card| app.ui.new_graph_category.is_none_or(|kind| card.kind == kind)).collect();
    egui::ScrollArea::vertical().id_salt("new-graph-cards").auto_shrink([false, false]).show(ui, |ui| {
        for kind in [GraphKind::Material, GraphKind::Function] {
            let group: Vec<&TemplateCard> = cards.iter().filter(|card| card.kind == kind).collect();
            let Some(first) = group.first() else { continue };

            ui.horizontal(|ui| {
                ui.label(RichText::new(tr(first.kind_label())).size(16.0).color(tokens.text_dim));
                let line = ui.available_rect_before_wrap();
                ui.painter().hline(line.left() + 8.0..=line.right(), line.center().y, egui::Stroke::new(1.0, tokens.separator));
            });
            ui.add_space(6.0);
            let columns = ((ui.available_width() + CARD_GAP) / (CARD_SIZE.x + CARD_GAP)).floor().max(1.0) as usize;
            for row in group.chunks(columns) {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = CARD_GAP;
                    for card in row {
                        let chosen = app.ui.new_graph_kind == card.kind && app.ui.new_graph_template == card.id;
                        let preview = crate::windows::template_preview(ui.ctx(), card.id);
                        let response = crate::template_card::show(ui, card, chosen, preview);
                        if response.clicked() {
                            choose(app, card);
                        }
                        if response.double_clicked() {
                            choose(app, card);
                            app.ui.new_graph_create_now = true;
                        }
                    }
                });
                ui.add_space(CARD_GAP);
            }
        }
    });
}

/// The graph's name, size and package, and the Create and Cancel buttons. Returns true for Create
fn graph_settings(app: &mut MaterialCraftApp, ui: &mut Ui) -> bool {
    let tokens = Tokens::get(ui.ctx());
    ui.horizontal(|ui| {
        ui.label(RichText::new(tr("Graph name")).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(icon::QUESTION).color(tokens.text_dim))
                .on_hover_text(tr("A graph's name is unique in its package; a taken name gets a number"));
        });
    });
    let name = ui.add(egui::TextEdit::singleline(&mut app.ui.new_graph_name).desired_width(f32::INFINITY));
    crate::widgets::name(&name, egui::Role::TextInput, "Graph name");
    ui.add_space(14.0);

    if app.ui.new_graph_kind == GraphKind::Material {
        ui.label(RichText::new(tr("Size")).strong());
        ui.horizontal(|ui| {
            let side_width = (ui.available_width() - 40.0) * 0.5;
            side_combo(ui, "new-graph-width", "Width", &mut app.ui.new_graph_size, side_width);
            let link = crate::widgets::icon_button(ui, icon::LINK, tr("Keep the width and height the same"), app.ui.new_graph_linked, 24.0);
            if link.clicked() {
                app.ui.new_graph_linked = !app.ui.new_graph_linked;
            }
            if app.ui.new_graph_linked {
                app.ui.new_graph_height = app.ui.new_graph_size;
            }
            ui.add_enabled_ui(!app.ui.new_graph_linked, |ui| side_combo(ui, "new-graph-height", "Height", &mut app.ui.new_graph_height, side_width));
        });
        ui.add_space(14.0);
    }

    ui.label(RichText::new(tr("Create graph in")).strong());
    let packages: Vec<(u64, String)> = app.session.open_packages().into_iter().map(|(id, package, _, _)| (id, package.name.clone())).collect();
    let current = app
        .ui
        .new_graph_package
        .and_then(|id| packages.iter().find(|(package, _)| *package == id))
        .map_or_else(|| tr("New package").to_string(), |(_, name)| name.clone());
    let combo = egui::ComboBox::from_id_salt("new-graph-package").width(ui.available_width()).selected_text(current).show_ui(ui, |ui| {
        ui.selectable_value(&mut app.ui.new_graph_package, None, tr("New package"));
        for (id, name) in &packages {
            ui.selectable_value(&mut app.ui.new_graph_package, Some(*id), name);
        }
    });
    crate::widgets::name(&combo.response, egui::Role::ComboBox, "Create graph in");

    let mut create = std::mem::take(&mut app.ui.new_graph_create_now);
    let bottom = ui.max_rect().bottom();
    let bar = egui::Rect::from_min_max(egui::pos2(ui.max_rect().left(), bottom - 32.0), ui.max_rect().right_bottom());
    let mut bar_ui = ui.new_child(egui::UiBuilder::new().max_rect(bar).layout(egui::Layout::right_to_left(egui::Align::Center)));
    if bar_ui.add_sized([BUTTON_WIDTH, 28.0], egui::Button::new(tr("Cancel"))).clicked() {
        app.ui.windows.new_graph = false;
    }
    create |= bar_ui.add_sized([BUTTON_WIDTH, 28.0], crate::widgets::primary_button(&bar_ui, tr("Create"))).clicked();
    create
}

/// A list of the sides a graph can have
fn side_combo(ui: &mut Ui, id: &str, label: &str, side: &mut u32, width: f32) {
    let combo = egui::ComboBox::from_id_salt(id).width(width).selected_text(format!("{side} px")).show_ui(ui, |ui| {
        for option in SIDES {
            ui.selectable_value(side, option, format!("{option} px"));
        }
    });
    crate::widgets::name(&combo.response, egui::Role::ComboBox, label);
}
