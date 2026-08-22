use eframe::egui;
use crate::AfterEffectsApp;
use crate::core::timeline::{Layer, LayerType, ProjectItemType};

/// Local asset library: quick-create solids/text from swatches & styles, and
/// browse the project's imported assets. (Replaces the fake "Creative Cloud"
/// panel — everything here works offline against the real project.)
pub fn draw_cc_libraries(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Asset Library (Local)");
    ui.separator();

    ui.add(egui::TextEdit::singleline(&mut app.cc_libraries_search).hint_text("Search assets..."));
    let query = app.cc_libraries_search.to_lowercase();
    ui.add_space(6.0);
    ui.separator();

    // ── Color swatches: click adds a Solid layer ──
    ui.collapsing("Color Swatches", |ui| {
        if query.is_empty() || "color swatch solid".contains(&query) {
            ui.horizontal(|ui| {
                let swatches: [([f32; 4], &str); 6] = [
                    ([0.08, 0.45, 0.90, 1.0], "Studio Blue"),
                    ([0.90, 0.72, 0.00, 1.0], "Brand Gold"),
                    ([0.90, 0.20, 0.20, 1.0], "Alert Red"),
                    ([0.10, 0.80, 0.45, 1.0], "Mint Green"),
                    ([0.60, 0.30, 0.90, 1.0], "Violet"),
                    ([0.95, 0.95, 0.95, 1.0], "Off White"),
                ];
                for (color, name) in swatches {
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::click());
                    ui.painter().rect_filled(rect, 3.0, egui::Color32::from_rgb(
                        (color[0] * 255.0) as u8, (color[1] * 255.0) as u8, (color[2] * 255.0) as u8,
                    ));
                    if resp.clicked() {
                        let lname = format!("Solid: {}", name);
                        app.modify_project(|p| {
                            let comp = p.active_composition_mut();
                            let layer = Layer::new(
                                format!("solid_lib_{}", comp.layers.len()),
                                lname,
                                LayerType::Solid { color },
                                comp.duration_frames,
                            );
                            comp.layers.push(layer);
                        });
                    }
                    resp.on_hover_text(format!("Add {} solid", name));
                }
            });
        }
    });

    // ── Character styles: click adds a styled Text layer ──
    ui.collapsing("Character Styles", |ui| {
        let styles: [(&str, u32, [f32; 4]); 3] = [
            ("Header Bold 48pt", 48, [1.0, 1.0, 1.0, 1.0]),
            ("Subhead Regular 24pt", 24, [0.85, 0.85, 0.85, 1.0]),
            ("Caption Small 16pt", 16, [0.6, 0.6, 0.6, 1.0]),
        ];
        for (name, size, color) in styles {
            if (query.is_empty() || name.to_lowercase().contains(&query))
                && ui.button(format!("T  {}", name)).clicked()
            {
                let lname = format!("Text: {}", name);
                app.modify_project(|p| {
                    let comp = p.active_composition_mut();
                    let layer = Layer::new(
                        format!("text_lib_{}", comp.layers.len()),
                        lname,
                        LayerType::Text {
                            text: name.split_whitespace().next().unwrap_or("Text").to_string(),
                            font_size: size,
                            color,
                            font_family: "Arial".into(),
                            tracking: 0.0,
                            leading: 1.2,
                            align: 1,
                            stroke_color: [0.0; 4],
                            stroke_width: 0.0,
                            text_on_path: false,
                        },
                        comp.duration_frames,
                    );
                    comp.layers.push(layer);
                });
            }
        }
    });

    // ── Project assets: browse + add to comp ──
    ui.collapsing("Project Assets", |ui| {
        let project = app.history.current();
        let assets: Vec<String> = project
            .assets
            .iter()
            .filter(|a| query.is_empty() || a.name.to_lowercase().contains(&query))
            .map(|a| match &a.item_type {
                ProjectItemType::Image { path, .. } => format!("{} — {}", a.name, path),
                ProjectItemType::Video { path, duration_sec } => {
                    format!("{} — {} ({:.1}s)", a.name, path, duration_sec)
                }
                ProjectItemType::Audio { path, duration_sec } => {
                    format!("{} — {} ({:.1}s)", a.name, path, duration_sec)
                }
                ProjectItemType::Solid { .. } => format!("{} — Solid", a.name),
                ProjectItemType::Folder { name } => format!("{} — Folder", name),
                ProjectItemType::Composition { comp_idx } => {
                    format!("{} — Comp #{}", a.name, comp_idx)
                }
            })
            .collect();
        if assets.is_empty() {
            ui.weak("(no assets — import media from the File menu)");
        }
        for a in assets {
            ui.small(a);
        }
    });
}
