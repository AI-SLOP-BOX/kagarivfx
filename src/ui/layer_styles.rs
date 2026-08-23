use eframe::egui;
use crate::AfterEffectsApp;
use crate::core::timeline::BlendMode;
use crate::ui::theme::colors;

pub fn draw_layer_styles(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    let comp = app.history.current_mut().active_composition_mut();

    let layer_idx = match app.selected_layer_idx {
        Some(idx) if idx < comp.layers.len() => idx,
        _ => {
            ui.label(egui::RichText::new("No layer selected.").small().color(colors::TEXT_MUTED));
            return;
        }
    };

    let layer = &mut comp.layers[layer_idx];
    let style = &mut layer.style;
    let mut changed = false;

    egui::ScrollArea::vertical().show(ui, |ui| {
        // ── Drop Shadow ──
        ui.collapsing("👤 Drop Shadow", |ui| {
            if ui.checkbox(&mut style.drop_shadow.enabled, "Enabled").clicked() {
                changed = true;
            }

            let mut blend_idx = match style.drop_shadow.blend_mode {
                BlendMode::Normal => 0,
                BlendMode::Multiply => 1,
                BlendMode::Screen => 2,
                BlendMode::Overlay => 3,
                BlendMode::Add => 4,
                _ => 0,
            };
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Blend Mode").small().color(colors::TEXT_SECONDARY));
                egui::ComboBox::from_id_salt("ds_blend")
                    .selected_text(format!("{:?}", style.drop_shadow.blend_mode))
                    .show_ui(ui, |ui| {
                        if ui.selectable_value(&mut blend_idx, 0, "Normal").clicked() { changed = true; }
                        if ui.selectable_value(&mut blend_idx, 1, "Multiply").clicked() { changed = true; }
                        if ui.selectable_value(&mut blend_idx, 2, "Screen").clicked() { changed = true; }
                        if ui.selectable_value(&mut blend_idx, 3, "Overlay").clicked() { changed = true; }
                        if ui.selectable_value(&mut blend_idx, 4, "Add").clicked() { changed = true; }
                    });
            });
            if changed {
                style.drop_shadow.blend_mode = match blend_idx {
                    1 => BlendMode::Multiply,
                    2 => BlendMode::Screen,
                    3 => BlendMode::Overlay,
                    4 => BlendMode::Add,
                    _ => BlendMode::Normal,
                };
            }

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Opacity").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut style.drop_shadow.opacity, 0.0..=100.0).suffix("%")).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Angle").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut style.drop_shadow.angle, -180.0..=180.0).suffix("°")).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Distance").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut style.drop_shadow.distance, 0.0..=200.0).suffix(" px")).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Size").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut style.drop_shadow.size, 0.0..=200.0).suffix(" px")).changed() {
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Color").small().color(colors::TEXT_SECONDARY));
                let c = &mut style.drop_shadow.color;
                let mut col = egui::Color32::from_rgba_premultiplied(
                    (c[0] * 255.0) as u8, (c[1] * 255.0) as u8,
                    (c[2] * 255.0) as u8, (c[3] * 255.0) as u8,
                );
                if ui.color_edit_button_srgba(&mut col).changed() {
                    let [r, g, b, a] = col.to_array();
                    *c = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0];
                    changed = true;
                }
            });
        });

        // ── Outer Glow ──
        ui.collapsing("🌟 Outer Glow", |ui| {
            if ui.checkbox(&mut style.outer_glow.enabled, "Enabled").clicked() {
                changed = true;
            }
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Opacity").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut style.outer_glow.opacity, 0.0..=100.0).suffix("%")).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Spread").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut style.outer_glow.spread, 0.0..=100.0).suffix("%")).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Size").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut style.outer_glow.size, 0.0..=200.0).suffix(" px")).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Color").small().color(colors::TEXT_SECONDARY));
                let c = &mut style.outer_glow.color;
                let mut col = egui::Color32::from_rgba_premultiplied(
                    (c[0] * 255.0) as u8, (c[1] * 255.0) as u8,
                    (c[2] * 255.0) as u8, (c[3] * 255.0) as u8,
                );
                if ui.color_edit_button_srgba(&mut col).changed() {
                    let [r, g, b, a] = col.to_array();
                    *c = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0];
                    changed = true;
                }
            });
        });

        // ── Stroke ──
        ui.collapsing("✏ Stroke", |ui| {
            if ui.checkbox(&mut style.stroke.enabled, "Enabled").clicked() {
                changed = true;
            }
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Size").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut style.stroke.size, 1.0..=250.0).suffix(" px")).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Position").small().color(colors::TEXT_SECONDARY));
                egui::ComboBox::from_id_salt("stroke_pos")
                    .selected_text(match style.stroke.position {
                        0 => "Outside",
                        1 => "Inside",
                        2 => "Center",
                        _ => "Outside",
                    })
                    .show_ui(ui, |ui| {
                        if ui.selectable_value(&mut style.stroke.position, 0, "Outside").clicked() { changed = true; }
                        if ui.selectable_value(&mut style.stroke.position, 1, "Inside").clicked() { changed = true; }
                        if ui.selectable_value(&mut style.stroke.position, 2, "Center").clicked() { changed = true; }
                    });
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Color").small().color(colors::TEXT_SECONDARY));
                let c = &mut style.stroke.color;
                let mut col = egui::Color32::from_rgba_premultiplied(
                    (c[0] * 255.0) as u8, (c[1] * 255.0) as u8,
                    (c[2] * 255.0) as u8, (c[3] * 255.0) as u8,
                );
                if ui.color_edit_button_srgba(&mut col).changed() {
                    let [r, g, b, a] = col.to_array();
                    *c = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0];
                    changed = true;
                }
            });
        });
    });

    if changed {
        crate::core::frame_cache::bump_version();
    }
}
