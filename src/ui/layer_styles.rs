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

        // ── Inner Shadow ──
        ui.collapsing("🕳 Inner Shadow", |ui| {
            if ui.checkbox(&mut style.inner_shadow.enabled, "Enabled").clicked() {
                changed = true;
            }
            let s = &mut style.inner_shadow;
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Opacity").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut s.opacity, 0.0..=100.0).suffix("%")).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Angle").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut s.angle, -180.0..=180.0).suffix("°")).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Distance").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut s.distance, 0.0..=250.0)).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Size").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut s.size, 0.0..=64.0)).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Color").small().color(colors::TEXT_SECONDARY));
                let c = &mut s.color;
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

        // ── Inner Glow ──
        ui.collapsing("💡 Inner Glow", |ui| {
            if ui.checkbox(&mut style.inner_glow.enabled, "Enabled").clicked() {
                changed = true;
            }
            let ig = &mut style.inner_glow;
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Opacity").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut ig.opacity, 0.0..=100.0).suffix("%")).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Size").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut ig.size, 0.0..=64.0)).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Color").small().color(colors::TEXT_SECONDARY));
                let c = &mut ig.color;
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

        // ── Satin ──
        ui.collapsing("🧵 Satin", |ui| {
            if ui.checkbox(&mut style.satin.enabled, "Enabled").clicked() {
                changed = true;
            }
            let s = &mut style.satin;
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Opacity").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut s.opacity, 0.0..=100.0).suffix("%")).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Angle").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut s.angle, -180.0..=180.0).suffix("°")).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Distance").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut s.distance, 0.0..=150.0)).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Size").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut s.size, 1.0..=64.0)).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Color").small().color(colors::TEXT_SECONDARY));
                let c = &mut s.color;
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

        // ── Bevel / Emboss ──
        ui.collapsing("🪨 Bevel / Emboss", |ui| {
            if ui.checkbox(&mut style.bevel_emboss.enabled, "Enabled").clicked() {
                changed = true;
            }
            let bv = &mut style.bevel_emboss;
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Angle").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut bv.angle, -180.0..=180.0).suffix("°")).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Depth").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut bv.depth, 1.0..=20.0)).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Size").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut bv.size, 0.0..=64.0)).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Highlight").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut bv.highlight, 0.0..=100.0).suffix("%")).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Shadow").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut bv.shadow, 0.0..=100.0).suffix("%")).changed() {
                    changed = true;
                }
            });
            for (label, cvar) in [("Light", &mut bv.color_light), ("Dark", &mut bv.color_dark)] {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(label).small().color(colors::TEXT_SECONDARY));
                    let mut col = egui::Color32::from_rgba_premultiplied(
                        (cvar[0] * 255.0) as u8, (cvar[1] * 255.0) as u8,
                        (cvar[2] * 255.0) as u8, (cvar[3] * 255.0) as u8,
                    );
                    if ui.color_edit_button_srgba(&mut col).changed() {
                        let [r, g, b, a] = col.to_array();
                        *cvar = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0];
                        changed = true;
                    }
                });
            }
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

        // ── Gradient Overlay ──
        ui.collapsing("🌈 Gradient Overlay", |ui| {
            if ui.checkbox(&mut style.gradient_overlay.enabled, "Enabled").clicked() {
                changed = true;
            }
            let go = &mut style.gradient_overlay;
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Opacity").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut go.opacity, 0.0..=100.0).suffix("%")).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Angle").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut go.angle, -180.0..=180.0).suffix("°")).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Scale").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut go.scale, 10.0..=400.0).suffix("%")).changed() {
                    changed = true;
                }
            });
            for (label, cvar) in [("Start", &mut go.color_start), ("End", &mut go.color_end)] {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(label).small().color(colors::TEXT_SECONDARY));
                    let mut col = egui::Color32::from_rgba_premultiplied(
                        (cvar[0] * 255.0) as u8, (cvar[1] * 255.0) as u8,
                        (cvar[2] * 255.0) as u8, (cvar[3] * 255.0) as u8,
                    );
                    if ui.color_edit_button_srgba(&mut col).changed() {
                        let [r, g, b, a] = col.to_array();
                        *cvar = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0];
                        changed = true;
                    }
                });
            }
        });

        // ── Color Overlay ──
        ui.collapsing("🎨 Color Overlay", |ui| {
            if ui.checkbox(&mut style.color_overlay.enabled, "Enabled").clicked() {
                changed = true;
            }
            let co = &mut style.color_overlay;
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Opacity").small().color(colors::TEXT_SECONDARY));
                if ui.add(egui::Slider::new(&mut co.opacity, 0.0..=100.0).suffix("%")).changed() {
                    changed = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Color").small().color(colors::TEXT_SECONDARY));
                let c = &mut co.color;
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
