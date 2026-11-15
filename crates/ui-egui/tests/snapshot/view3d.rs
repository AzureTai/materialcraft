//! Renders of the 3D view: its menus, tool strip, helpers and its parts in the Properties panel

use std::time::Duration;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use materialcraft_ui_egui::{MaterialCraftApp, UiState};
use serde_json::json;

/// Step until the 3D view's picture is the finished one for its current settings
fn rendered(harness: &mut Harness<'static, MaterialCraftApp>) {
    let before = harness.state().view3d.renders;
    for _ in 0..600 {
        harness.step();
        let app = harness.state();
        if app.view3d.renders > before && !app.view3d.busy() && !app.cooker.busy() && !app.session.dirty {
            harness.run_steps(3);
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn change(harness: &mut Harness<'static, MaterialCraftApp>, settings: serde_json::Value) {
    harness.state_mut().run("view.3d", settings).unwrap();
    rendered(harness);
}

#[test]
#[ignore = "requires an offscreen GPU; run explicitly with --ignored"]
fn the_3d_view_shows_its_settings() {
    let Some(directory) = std::env::var_os("MC_SNAPSHOT_DIR").map(std::path::PathBuf::from) else { return };

    std::fs::create_dir_all(&directory).unwrap();
    let app = MaterialCraftApp::headless(materialcraft_engine::sample_session(), UiState::default());
    let mut harness = super::harness(app, 1600.0, 960.0);
    rendered(&mut harness);
    super::save(&mut harness, &directory, "view3d-plain");

    change(&mut harness, json!({"grid": true, "axis": true, "boundingBox": true, "sceneStats": true, "resolution": true, "fps": true, "lightHelpers": true}));
    super::save(&mut harness, &directory, "view3d-helpers");

    change(&mut harness, json!({"grid": false, "boundingBox": false, "wireframe": true, "vertexTangentSpace": true, "scene": "sphere"}));
    super::save(&mut harness, &directory, "view3d-wireframe");

    change(
        &mut harness,
        json!({"wireframe": false, "vertexTangentSpace": false, "scene": "rounded_cube", "background": "environment", "light1": {"enabled": true, "intensity": 3.0}, "shadows": true}),
    );
    for label in ["Lights", "Edit properties"] {
        harness.get_by_label(label).click();
        harness.run_steps(4);
    }
    super::save(&mut harness, &directory, "view3d-lights");

    // The panel's own Camera chip goes away with the panel, which leaves the menu's
    for label in ["Back to the selection's properties", "Camera", "Edit properties"] {
        harness.get_by_label(label).click();
        harness.run_steps(4);
    }
    change(
        &mut harness,
        json!({"focalLength": 18.0, "overrideResolution": true, "viewportResolution": false, "renderResolution": [1280, 720], "background": "color", "backgroundColor": [0.02, 0.03, 0.05]}),
    );
    super::save(&mut harness, &directory, "view3d-camera");

    change(
        &mut harness,
        json!({"focalLength": 35.0, "overrideResolution": false, "shader": "metallicRoughness", "material": {"baseMetalness": 1.0, "specularRoughness": 0.25}, "channels": {"baseColor": false}}),
    );
    for label in ["Back to the selection's properties", "Materials", "Default", "Edit properties"] {
        harness.get_by_label(label).click();
        harness.run_steps(4);
    }
    harness.state_mut().view3d.scene_browser = true;
    rendered(&mut harness);
    super::save(&mut harness, &directory, "view3d-material");
}
