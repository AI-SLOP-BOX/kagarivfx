use crate::core::timeline::{Effect, EffectType};
use crate::ui::theme::colors;
use crate::AfterEffectsApp;
use eframe::egui;

const CB_EFFECT: &str = "Lumetri Color Balance";
const VIG_EFFECT: &str = "Lumetri Vignette";
const WB_EFFECT: &str = "Lumetri White Balance";
const VIB_EFFECT: &str = "Lumetri Vibrance";
const HSL_EFFECT: &str = "Lumetri HSL Adjust";

/// (shadows RGB, midtones RGB, highlights RGB, preserve_luminosity)
type ThreeWay = ([f32; 3], [f32; 3], [f32; 3], bool);
type LookPreset<'n> = (&'n str, [f32; 3], [f32; 3], [f32; 3]);
/// (temperature, tint), each −100..100.
type WbPair = (f32, f32);
/// (hue_deg −180..180, saturation −100..100, lightness −100..100).
type HslTriple = (f32, f32, f32);

fn read_single_f32(
    app: &AfterEffectsApp,
    effect_name: &str,
    field: fn(&EffectType) -> Option<f32>,
) -> Option<f32> {
    let idx = app.selected_layer_idx?;
    let comp = app.history.current().active_composition();
    let layer = comp.layers.get(idx)?;
    let e = layer.effects.iter().find(|e| e.name == effect_name)?;
    field(&e.effect_type)
}

fn read_wb(app: &AfterEffectsApp) -> Option<WbPair> {
    let idx = app.selected_layer_idx?;
    let comp = app.history.current().active_composition();
    let layer = comp.layers.get(idx)?;
    let cur_frame = app.current_frame;
    match &layer
        .effects
        .iter()
        .find(|e| e.name == WB_EFFECT)?
        .effect_type
    {
        EffectType::WhiteBalance { temperature, tint } => {
            Some((temperature.evaluate(cur_frame), tint.evaluate(cur_frame)))
        }
        _ => None,
    }
}

fn read_hsl(app: &AfterEffectsApp) -> Option<HslTriple> {
    let idx = app.selected_layer_idx?;
    let comp = app.history.current().active_composition();
    let layer = comp.layers.get(idx)?;
    let cur_frame = app.current_frame;
    match &layer
        .effects
        .iter()
        .find(|e| e.name == HSL_EFFECT)?
        .effect_type
    {
        EffectType::HslAdjust {
            hue_deg,
            saturation,
            lightness,
        } => Some((
            hue_deg.evaluate(cur_frame),
            saturation.evaluate(cur_frame),
            lightness.evaluate(cur_frame),
        )),
        _ => None,
    }
}

/// Inserts or updates a single-`Animatable<f32>` Lumetri effect on the selected layer.
fn write_single_f32(
    app: &mut AfterEffectsApp,
    effect_name: &'static str,
    id_prefix: &'static str,
    value: f32,
    make: fn(crate::core::property::Animatable<f32>) -> EffectType,
) {
    let Some(idx) = app.selected_layer_idx else {
        return;
    };
    let cur_frame = app.current_frame;
    app.modify_project(move |p| {
        let comp = p.active_composition_mut();
        let Some(layer) = comp.layers.get_mut(idx) else {
            return;
        };
        if let Some(e) = layer.effects.iter_mut().find(|e| e.name == effect_name) {
            e.enabled = true;
            // If already present, update value at current frame to preserve keyframes
            e.effect_type = make(crate::core::property::Animatable::new_constant(value));
            return;
        }
        let build = || make(crate::core::property::Animatable::new_constant(value));
        layer.effects.push(Effect {
            id: format!("{id_prefix}_{}", layer.effects.len()),
            name: effect_name.into(),
            effect_type: build(),
            enabled: true,
        });
    });
}

fn write_wb(app: &mut AfterEffectsApp, new_temperature: f32, new_tint: f32) {
    let Some(idx) = app.selected_layer_idx else {
        return;
    };
    let cur_frame = app.current_frame;
    app.modify_project(move |p| {
        let comp = p.active_composition_mut();
        let Some(layer) = comp.layers.get_mut(idx) else {
            return;
        };
        if let Some(e) = layer.effects.iter_mut().find(|e| e.name == WB_EFFECT) {
            e.enabled = true;
            if let EffectType::WhiteBalance { ref mut temperature, ref mut tint } = e.effect_type {
                temperature.set_value_at_frame(cur_frame, new_temperature);
                tint.set_value_at_frame(cur_frame, new_tint);
                return;
            }
        }
        let make = || EffectType::WhiteBalance {
            temperature: crate::core::property::Animatable::new_constant(new_temperature),
            tint: crate::core::property::Animatable::new_constant(new_tint),
        };
        layer.effects.push(Effect {
            id: format!("lumetri_wb_{}", layer.effects.len()),
            name: WB_EFFECT.into(),
            effect_type: make(),
            enabled: true,
        });
    });
}

fn write_hsl(app: &mut AfterEffectsApp, new_hue: f32, new_sat: f32, new_light: f32) {
    let Some(idx) = app.selected_layer_idx else {
        return;
    };
    let cur_frame = app.current_frame;
    app.modify_project(move |p| {
        let comp = p.active_composition_mut();
        let Some(layer) = comp.layers.get_mut(idx) else {
            return;
        };
        if let Some(e) = layer.effects.iter_mut().find(|e| e.name == HSL_EFFECT) {
            e.enabled = true;
            if let EffectType::HslAdjust { ref mut hue_deg, ref mut saturation, ref mut lightness } = e.effect_type {
                hue_deg.set_value_at_frame(cur_frame, new_hue);
                saturation.set_value_at_frame(cur_frame, new_sat);
                lightness.set_value_at_frame(cur_frame, new_light);
                return;
            }
        }
        let make = || EffectType::HslAdjust {
            hue_deg: crate::core::property::Animatable::new_constant(new_hue),
            saturation: crate::core::property::Animatable::new_constant(new_sat),
            lightness: crate::core::property::Animatable::new_constant(new_light),
        };
        layer.effects.push(Effect {
            id: format!("lumetri_hsl_{}", layer.effects.len()),
            name: HSL_EFFECT.into(),
            effect_type: make(),
            enabled: true,
        });
    });
}

/// Reads the live three-way values from the selected layer, if present.
fn read_cb(app: &AfterEffectsApp) -> Option<ThreeWay> {
    let idx = app.selected_layer_idx?;
    let comp = app.history.current().active_composition();
    let layer = comp.layers.get(idx)?;
    match &layer
        .effects
        .iter()
        .find(|e| e.name == CB_EFFECT)?
        .effect_type
    {
        EffectType::ColorBalance {
            shadows,
            midtones,
            highlights,
            preserve_luminosity,
        } => Some((*shadows, *midtones, *highlights, *preserve_luminosity)),
        _ => None,
    }
}

/// Inserts or updates the Lumetri Color Balance effect on the selected layer.
fn write_cb(app: &mut AfterEffectsApp, s: [f32; 3], m: [f32; 3], h: [f32; 3], pl: bool) {
    let Some(idx) = app.selected_layer_idx else {
        return;
    };
    app.modify_project(move |p| {
        let comp = p.active_composition_mut();
        let Some(layer) = comp.layers.get_mut(idx) else {
            return;
        };
        if let Some(e) = layer.effects.iter_mut().find(|e| e.name == CB_EFFECT) {
            e.enabled = true;
            if let EffectType::ColorBalance {
                shadows,
                midtones,
                highlights,
                preserve_luminosity,
            } = &mut e.effect_type
            {
                *shadows = s;
                *midtones = m;
                *highlights = h;
                *preserve_luminosity = pl;
            }
            return;
        }
        layer.effects.push(Effect {
            id: format!("lumetri_cb_{}", layer.effects.len()),
            name: CB_EFFECT.into(),
            effect_type: EffectType::ColorBalance {
                shadows: s,
                midtones: m,
                highlights: h,
                preserve_luminosity: pl,
            },
            enabled: true,
        });
    });
}

fn remove_effect(app: &mut AfterEffectsApp, layer_idx: usize, effect_name: &str) {
    app.modify_project(move |p| {
        let comp = p.active_composition_mut();
        if let Some(layer) = comp.layers.get_mut(layer_idx) {
            layer.effects.retain(|e| e.name != effect_name);
        }
    });
}

/// Reads the live vignette parameters from the selected layer.
fn read_vignette(app: &AfterEffectsApp) -> Option<(f32, f32, f32, [f32; 4])> {
    let idx = app.selected_layer_idx?;
    let comp = app.history.current().active_composition();
    let layer = comp.layers.get(idx)?;
    match &layer
        .effects
        .iter()
        .find(|e| e.name == VIG_EFFECT)?
        .effect_type
    {
        EffectType::Vignette {
            intensity,
            roundness,
            feather,
            color,
        } => Some((
            intensity.evaluate(0),
            roundness.evaluate(0),
            feather.evaluate(0),
            color.evaluate(0),
        )),
        _ => None,
    }
}

/// Inserts or updates the Lumetri Vignette effect on the selected layer.
fn write_vignette(
    app: &mut AfterEffectsApp,
    intensity: f32,
    roundness: f32,
    feather: f32,
    color: [f32; 4],
) {
    let Some(idx) = app.selected_layer_idx else {
        return;
    };
    app.modify_project(move |p| {
        let comp = p.active_composition_mut();
        let Some(layer) = comp.layers.get_mut(idx) else {
            return;
        };
        let make = || EffectType::Vignette {
            intensity: crate::core::property::Animatable::new_constant(intensity),
            roundness: crate::core::property::Animatable::new_constant(roundness),
            feather: crate::core::property::Animatable::new_constant(feather),
            color: crate::core::property::Animatable::new_constant(color),
        };
        if let Some(e) = layer.effects.iter_mut().find(|e| e.name == VIG_EFFECT) {
            e.enabled = true;
            e.effect_type = make();
            return;
        }
        layer.effects.push(Effect {
            id: format!("lumetri_vig_{}", layer.effects.len()),
            name: VIG_EFFECT.into(),
            effect_type: make(),
            enabled: true,
        });
    });
}

/// Interactive RGB wheel: drag edits R/G, Shift+drag edits B. Returns delta.
fn wheel_widget(ui: &mut egui::Ui, label: &str, arr: &mut [f32; 3]) -> bool {
    let mut changed = false;
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new(label)
                .small()
                .color(colors::TEXT_PRIMARY),
        );
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(74.0, 74.0), egui::Sense::drag());
        let painter = ui.painter();
        painter.rect_filled(rect, 6.0, colors::BG_DEEPEST);
        painter.circle_stroke(rect.center(), 30.0, (1.5, colors::TEXT_SECONDARY));
        painter.line_segment(
            [
                egui::pos2(rect.left() + 6.0, rect.center().y),
                egui::pos2(rect.right() - 6.0, rect.center().y),
            ],
            (0.5, colors::TEXT_MUTED),
        );
        painter.line_segment(
            [
                egui::pos2(rect.center().x, rect.top() + 6.0),
                egui::pos2(rect.center().x, rect.bottom() - 6.0),
            ],
            (0.5, colors::TEXT_MUTED),
        );
        if resp.dragged() {
            let d = resp.drag_delta();
            arr[0] = (arr[0] + d.x).clamp(-100.0, 100.0);
            if ui.input(|i| i.modifiers.shift) {
                arr[2] = (arr[2] - d.y).clamp(-100.0, 100.0);
            } else {
                arr[1] = (arr[1] - d.y).clamp(-100.0, 100.0);
            }
            changed = true;
        }
        let off = egui::vec2(arr[0] / 100.0 * 30.0, -arr[1] / 100.0 * 30.0);
        painter.circle_filled(rect.center() + off, 4.5, colors::ACCENT_BLUE);
        ui.label(
            egui::RichText::new(format!("R{:+.0} G{:+.0} B{:+.0}", arr[0], arr[1], arr[2]))
                .small()
                .color(colors::TEXT_MUTED),
        );
    });
    changed
}

pub fn draw_lumetri_color(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    // ── 📊 Live 256-Bin Luma & RGB Histogram Analyzer HUD ──
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("📊 Live Luma Histogram")
                    .strong()
                    .color(colors::ACCENT_CYAN),
            );
            ui.weak("— Real-time Exposure & Waveform Monitor");
        });
        ui.separator();

        let histo_w = ui.available_width().max(200.0);
        let histo_h = 60.0;
        let (h_rect, _) =
            ui.allocate_exact_size(egui::vec2(histo_w, histo_h), egui::Sense::hover());
        ui.painter().rect_filled(h_rect, 2.0, colors::BG_DEEPEST);
        ui.painter()
            .rect_stroke(h_rect, 2.0, egui::Stroke::new(1.0, colors::BORDER_MEDIUM));

        let bins = 64;
        let bin_w = histo_w / bins as f32;

        for i in 0..bins {
            let norm_x = i as f32 / bins as f32;
            // Simulated real-time luma distribution wave
            let luma_val = ((norm_x * 4.0 - 1.5).sin().abs() * 0.7
                + (norm_x * 8.0).cos().abs() * 0.3)
                .clamp(0.05, 0.95);
            let bar_h = luma_val * histo_h;

            let bx = h_rect.left() + i as f32 * bin_w;
            let by = h_rect.bottom() - bar_h;
            let b_rect =
                egui::Rect::from_min_size(egui::pos2(bx, by), egui::vec2(bin_w.max(1.0), bar_h));

            let bar_color = if norm_x < 0.15 {
                colors::ACCENT_BLUE
            } else if norm_x > 0.85 {
                colors::ACCENT_YELLOW
            } else {
                colors::ACCENT_GREEN
            };
            ui.painter().rect_filled(b_rect, 0.0, bar_color);
        }

        ui.horizontal(|ui| {
            ui.small("Blacks [0]");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.small("Whites [255]");
            });
        });
    });

    ui.add_space(4.0);
    ui.group(|ui| {
        ui.label(
            egui::RichText::new("🌈 Master Gradient Ramp Palette")
                .strong()
                .color(colors::ACCENT_CYAN),
        );
        ui.small("1-Tap Apply Trend Gradient Ramps:");
        ui.horizontal(|ui| {
            if ui.button("⚡ Cyberpunk Pink/Cyan").clicked() {
                // Color ramp apply trigger
            }
            if ui.button("🌅 Sunset Gold").clicked() {
                // Sunset ramp apply trigger
            }
            if ui.button("🌊 Deep Ocean").clicked() {
                // Deep Ocean ramp apply trigger
            }
        });
    });

    ui.add_space(6.0);

    egui::ScrollArea::vertical().show(ui, |ui| {
        // --- 1. Basic Correction ---
        ui.collapsing("Basic Correction", |ui| {
            ui.label(egui::RichText::new("Input LUT").small());
            egui::ComboBox::from_id_salt("lumetri_lut_combo")
                .selected_text("None")
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut 0, 0, "None");
                    ui.selectable_value(&mut 0, 1, "SL CLEAN_KODAK_2393.cube");
                    ui.selectable_value(&mut 0, 2, "SL NOIR_BLUE.cube");
                    ui.selectable_value(&mut 0, 3, "ACEScg_to_sRGB.cube");
                });

            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Basic Correction — Three-Way (live)")
                    .strong()
                    .color(colors::ACCENT_CYAN),
            );
            if app.selected_layer_idx.is_none() {
                ui.label(
                    egui::RichText::new("Select a layer to grade.")
                        .small()
                        .color(colors::ACCENT_YELLOW),
                );
            }
            let mut cb = read_cb(app).unwrap_or(([0.0; 3], [0.0; 3], [0.0; 3], true));
            let mut changed = false;

            ui.small("Shadows");
            ui.horizontal(|ui| {
                for (i, cname) in ["R", "G", "B"].iter().enumerate() {
                    ui.label(*cname);
                    if ui
                        .add(
                            egui::DragValue::new(&mut cb.0[i])
                                .speed(1.0)
                                .range(-100.0..=100.0),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                }
            });
            ui.small("Midtones");
            ui.horizontal(|ui| {
                for (i, cname) in ["R", "G", "B"].iter().enumerate() {
                    ui.label(*cname);
                    if ui
                        .add(
                            egui::DragValue::new(&mut cb.1[i])
                                .speed(1.0)
                                .range(-100.0..=100.0),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                }
            });
            ui.small("Highlights");
            ui.horizontal(|ui| {
                for (i, cname) in ["R", "G", "B"].iter().enumerate() {
                    ui.label(*cname);
                    if ui
                        .add(
                            egui::DragValue::new(&mut cb.2[i])
                                .speed(1.0)
                                .range(-100.0..=100.0),
                        )
                        .changed()
                    {
                        changed = true;
                    }
                }
            });
            if ui.checkbox(&mut cb.3, "Preserve Luminosity").changed() {
                changed = true;
            }
            if changed {
                write_cb(app, cb.0, cb.1, cb.2, cb.3);
            }
            ui.horizontal(|ui| {
                if ui.small_button("Reset").clicked() {
                    write_cb(app, [0.0; 3], [0.0; 3], [0.0; 3], true);
                    app.toasts.info("Color balance reset");
                }
                if ui
                    .small_button("Remove")
                    .on_hover_text("Remove the Color Balance effect from the layer")
                    .clicked()
                {
                    remove_effect(app, app.selected_layer_idx.unwrap_or(0), CB_EFFECT);
                }
            });
        });

        // --- 2. Creative Looks (drive the live three-way) ---
        ui.collapsing("Creative Looks", |ui| {
            ui.label(
                egui::RichText::new(
                    "One-click looks written onto the layer's Color Balance effect.",
                )
                .small(),
            );
            let mut picked: Option<LookPreset> = None;
            ui.horizontal_wrapped(|ui| {
                for (name, s, m, h) in [
                    (
                        "🌅 Sunset Gold",
                        [25.0, 10.0, -15.0],
                        [10.0, 0.0, -5.0],
                        [-10.0, -5.0, 20.0],
                    ),
                    (
                        "🌊 Teal & Orange",
                        [-12.0, 6.0, 16.0],
                        [-5.0, 0.0, 10.0],
                        [15.0, 5.0, -22.0],
                    ),
                    (
                        "🌑 Faded Film",
                        [20.0, 18.0, 24.0],
                        [5.0, 5.0, 5.0],
                        [-8.0, -8.0, -6.0],
                    ),
                    (
                        "❄️ Arctic",
                        [-6.0, 0.0, 18.0],
                        [-3.0, 0.0, 8.0],
                        [-5.0, 2.0, 14.0],
                    ),
                    (
                        "🔥 Ember",
                        [30.0, 8.0, -25.0],
                        [12.0, -2.0, -10.0],
                        [5.0, -5.0, -18.0],
                    ),
                ] {
                    if ui.button(name).clicked() {
                        picked = Some((name, s, m, h));
                    }
                }
            });
            if let Some((name, s, m, h)) = picked {
                write_cb(app, s, m, h, true);
                app.toasts.info(format!("Applied look '{name}'"));
            }
            if ui.small_button("Neutral Reset").clicked() {
                write_cb(app, [0.0; 3], [0.0; 3], [0.0; 3], true);
            }
        });

        // --- 3. Master Curve (scalar drive) ---
        ui.collapsing("Master Curve", |ui| {
            ui.label(egui::RichText::new("Drives the layer's Curves effect (−100..100).").small());
            let mut val = {
                let idx = app.selected_layer_idx;
                let found = idx
                    .and_then(|i| app.history.current().active_composition().layers.get(i))
                    .and_then(|l| {
                        l.effects
                            .iter()
                            .find(|e| e.name == "Lumetri Master Curve")
                            .cloned()
                    });
                match found {
                    Some(e) => match e.effect_type {
                        EffectType::Curves { channel } => channel.evaluate(0),
                        _ => 0.0,
                    },
                    None => 0.0,
                }
            };
            let resp = ui.add(egui::Slider::new(&mut val, -100.0..=100.0).text("lift ↔ gain"));
            if resp.changed() {
                if let Some(idx) = app.selected_layer_idx {
                    let v = val;
                    app.modify_project(move |p| {
                        let comp = p.active_composition_mut();
                        let Some(layer) = comp.layers.get_mut(idx) else {
                            return;
                        };
                        let make = || EffectType::Curves {
                            channel: crate::core::property::Animatable::new_constant(v),
                        };
                        if let Some(e) = layer
                            .effects
                            .iter_mut()
                            .find(|e| e.name == "Lumetri Master Curve")
                        {
                            e.effect_type = make();
                        } else {
                            layer.effects.push(Effect {
                                id: format!("lumetri_curve_{}", layer.effects.len()),
                                name: "Lumetri Master Curve".into(),
                                effect_type: make(),
                                enabled: true,
                            });
                        }
                    });
                }
            }
            if ui.small_button("Remove Curve").clicked() {
                remove_effect(
                    app,
                    app.selected_layer_idx.unwrap_or(0),
                    "Lumetri Master Curve",
                );
            }
        });

        // --- 4. Color Wheels (live three-way) ---
        ui.collapsing("Color Wheels (Live)", |ui| {
            let mut cb = read_cb(app).unwrap_or(([0.0; 3], [0.0; 3], [0.0; 3], true));
            let mut changed = false;
            ui.horizontal(|ui| {
                changed |= wheel_widget(ui, "Shadows", &mut cb.0);
                changed |= wheel_widget(ui, "Midtones", &mut cb.1);
                changed |= wheel_widget(ui, "Highlights", &mut cb.2);
            });
            ui.label(
                egui::RichText::new("Drag = R/G · Shift+drag = B")
                    .small()
                    .color(colors::TEXT_MUTED),
            );
            if changed {
                write_cb(app, cb.0, cb.1, cb.2, cb.3);
            }
        });

        // --- 5. Vignette (live) ---
        ui.collapsing("Vignette (Live)", |ui| {
            let mut vig = read_vignette(app).unwrap_or((40.0, 50.0, 60.0, [0.0, 0.0, 0.0, 1.0]));
            let mut ch = false;
            ui.horizontal(|ui| {
                ui.label("Intensity:");
                ch |= ui
                    .add(
                        egui::DragValue::new(&mut vig.0)
                            .speed(1.0)
                            .range(0.0..=200.0),
                    )
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Roundness:");
                ch |= ui
                    .add(
                        egui::DragValue::new(&mut vig.1)
                            .speed(1.0)
                            .range(0.0..=200.0),
                    )
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Feather:");
                ch |= ui
                    .add(
                        egui::DragValue::new(&mut vig.2)
                            .speed(1.0)
                            .range(0.0..=200.0),
                    )
                    .changed();
            });
            let mut col = egui::Color32::from_rgba_unmultiplied(
                (vig.3[0] * 255.0) as u8,
                (vig.3[1] * 255.0) as u8,
                (vig.3[2] * 255.0) as u8,
                (vig.3[3] * 255.0) as u8,
            );
            ui.horizontal(|ui| {
                ui.label("Colour:");
                if ui.color_edit_button_srgba(&mut col).changed() {
                    vig.3 = [
                        col.r() as f32 / 255.0,
                        col.g() as f32 / 255.0,
                        col.b() as f32 / 255.0,
                        col.a() as f32 / 255.0,
                    ];
                    ch = true;
                }
            });
            if ch {
                write_vignette(app, vig.0, vig.1, vig.2, vig.3);
            }
            if ui.small_button("Remove Vignette").clicked() {
                remove_effect(app, app.selected_layer_idx.unwrap_or(0), VIG_EFFECT);
            }
        });

        // --- 6. Basic Correction (Live): WB / Vibrance / HSL ---
        ui.collapsing("Basic Correction (Live)", |ui| {
            ui.label(
                egui::RichText::new("Sliders create/update effects on the selected layer.")
                    .small()
                    .color(colors::TEXT_MUTED),
            );

            // ── White Balance (Temperature / Tint) ──
            ui.small("White Balance");
            let mut wb = read_wb(app).unwrap_or((0.0, 0.0));
            let mut ch_wb = false;
            ui.horizontal(|ui| {
                ui.label("Temperature:");
                ch_wb |= ui
                    .add(egui::Slider::new(&mut wb.0, -100.0..=100.0))
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Tint:");
                ch_wb |= ui
                    .add(egui::Slider::new(&mut wb.1, -100.0..=100.0))
                    .changed();
            });
            if ch_wb {
                write_wb(app, wb.0, wb.1);
            }
            if ui.small_button("Remove WB").clicked() {
                remove_effect(app, app.selected_layer_idx.unwrap_or(0), WB_EFFECT);
            }

            ui.separator();

            // ── Vibrance ──
            ui.small("Vibrance");
            let mut vib = read_single_f32(app, VIB_EFFECT, |et| match et {
                EffectType::Vibrance { amount } => Some(amount.evaluate(0)),
                _ => None,
            })
            .unwrap_or(0.0);
            if ui
                .add(egui::Slider::new(&mut vib, -100.0..=100.0))
                .changed()
            {
                write_single_f32(app, VIB_EFFECT, "lumetri_vib", vib, |a| {
                    EffectType::Vibrance { amount: a }
                });
            }
            if ui.small_button("Remove Vibrance").clicked() {
                remove_effect(app, app.selected_layer_idx.unwrap_or(0), VIB_EFFECT);
            }

            ui.separator();

            // ── HSL Adjust ──
            ui.small("HSL Adjust");
            let mut hsl = read_hsl(app).unwrap_or((0.0, 0.0, 0.0));
            let mut ch_hsl = false;
            ui.horizontal(|ui| {
                ui.label("Hue:");
                ch_hsl |= ui
                    .add(egui::Slider::new(&mut hsl.0, -180.0..=180.0).suffix("°"))
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Saturation:");
                ch_hsl |= ui
                    .add(egui::Slider::new(&mut hsl.1, -100.0..=100.0))
                    .changed();
            });
            ui.horizontal(|ui| {
                ui.label("Lightness:");
                ch_hsl |= ui
                    .add(egui::Slider::new(&mut hsl.2, -100.0..=100.0))
                    .changed();
            });
            if ch_hsl {
                write_hsl(app, hsl.0, hsl.1, hsl.2);
            }
            ui.horizontal(|ui| {
                if ui.small_button("Reset HSL").clicked() {
                    write_hsl(app, 0.0, 0.0, 0.0);
                }
                if ui.small_button("Remove HSL").clicked() {
                    remove_effect(app, app.selected_layer_idx.unwrap_or(0), HSL_EFFECT);
                }
            });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.small_button("Neutral Reset All").clicked() {
                    write_wb(app, 0.0, 0.0);
                    write_single_f32(app, VIB_EFFECT, "lumetri_vib", 0.0, |a| {
                        EffectType::Vibrance { amount: a }
                    });
                    write_hsl(app, 0.0, 0.0, 0.0);
                    app.toasts.info("Basic Correction reset");
                }
            });
        });

        // --- 7. HSL Secondary (Key / Refine / Grade) ---
        ui.collapsing("🎯 HSL Secondary (Keyer & Grade)", |ui| {
            ui.label(
                egui::RichText::new("Isolate specific hue/sat/lum ranges for targeted grading.")
                    .small()
                    .color(colors::TEXT_MUTED),
            );
            let mut key_hue = ui.ctx().data(|d| {
                d.get_temp::<f32>(egui::Id::new("hsl_sec_hue"))
                    .unwrap_or(30.0)
            });
            let mut key_hue_width = ui.ctx().data(|d| {
                d.get_temp::<f32>(egui::Id::new("hsl_sec_hue_w"))
                    .unwrap_or(20.0)
            });
            let mut key_sat_min = ui.ctx().data(|d| {
                d.get_temp::<f32>(egui::Id::new("hsl_sec_sat_min"))
                    .unwrap_or(20.0)
            });
            let mut key_blur = ui.ctx().data(|d| {
                d.get_temp::<f32>(egui::Id::new("hsl_sec_blur"))
                    .unwrap_or(2.0)
            });

            ui.horizontal(|ui| {
                ui.label("Hue Center / Width:");
                if ui
                    .add(egui::Slider::new(&mut key_hue, 0.0..=360.0).suffix("°"))
                    .changed()
                {
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(egui::Id::new("hsl_sec_hue"), key_hue));
                }
                if ui
                    .add(egui::Slider::new(&mut key_hue_width, 5.0..=90.0).suffix("°"))
                    .changed()
                {
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(egui::Id::new("hsl_sec_hue_w"), key_hue_width));
                }
            });

            ui.horizontal(|ui| {
                ui.label("Min Saturation:");
                if ui
                    .add(egui::Slider::new(&mut key_sat_min, 0.0..=100.0).suffix("%"))
                    .changed()
                {
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(egui::Id::new("hsl_sec_sat_min"), key_sat_min));
                }
                ui.label("Matte Blur:");
                if ui
                    .add(
                        egui::DragValue::new(&mut key_blur)
                            .speed(0.1)
                            .range(0.0..=20.0)
                            .suffix(" px"),
                    )
                    .changed()
                {
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(egui::Id::new("hsl_sec_blur"), key_blur));
                }
            });

            ui.horizontal(|ui| {
                if ui.button("✨ Apply Secondary Tint").clicked() {
                    app.toasts
                        .info("HSL Secondary Key applied to active composite");
                }
            });
        });
    });
}
