use eframe::egui;
use crate::core::timeline::{Layer, LayerType};
use crate::core::text_animator::{SelectorShape, TextAnimatorSettings};
use crate::ui::inspector_property::{draw_property_ui, draw_easy_ease_button, draw_expression_selector};
use crate::ui::theme::colors;

pub fn draw_layer_transforms(
    ui: &mut egui::Ui,
    layer: &mut Layer,
    current_frame: u32,
    fps: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
) {
    ui.group(|ui| {
        if layer.is_3d {
            ui.label("Transform 3D");
            
            let pos_before = layer.transform_3d.position.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Position (XYZ)", &mut layer.transform_3d.position, |ui, val| {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                    ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                    ui.add(egui::DragValue::new(&mut val[2]).speed(1.0).prefix("Z: "));
                });
            }) { *next_frame = Some(nf); }
            draw_easy_ease_button(ui, &mut layer.transform_3d.position, project_changed);
            if pos_before != layer.transform_3d.position { *project_changed = true; }

            ui.separator();
            let rot_before = layer.transform_3d.rotation.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Rotation (YPR)", &mut layer.transform_3d.rotation, |ui, val| {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).suffix("° P"));
                    ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).suffix("° Y"));
                    ui.add(egui::DragValue::new(&mut val[2]).speed(1.0).suffix("° R"));
                });
            }) { *next_frame = Some(nf); }
            draw_easy_ease_button(ui, &mut layer.transform_3d.rotation, project_changed);
            if rot_before != layer.transform_3d.rotation { *project_changed = true; }

            ui.separator();
            let scale_before = layer.transform_3d.scale.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Scale (XYZ)", &mut layer.transform_3d.scale, |ui, val| {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut val[0]).speed(0.1).suffix("% X"));
                    ui.add(egui::DragValue::new(&mut val[1]).speed(0.1).suffix("% Y"));
                    ui.add(egui::DragValue::new(&mut val[2]).speed(0.1).suffix("% Z"));
                });
            }) { *next_frame = Some(nf); }
            draw_easy_ease_button(ui, &mut layer.transform_3d.scale, project_changed);
            if scale_before != layer.transform_3d.scale { *project_changed = true; }

            // ── 3D Material Options ──
            ui.add_space(4.0);
            ui.collapsing("🧱 Material Options", |ui| {
                let mat = &mut layer.material;
                if ui.checkbox(&mut mat.cast_shadows, "Casts Shadows").clicked() { *project_changed = true; }
                ui.horizontal(|ui| {
                    ui.label("Ambient:");
                    if ui.add(egui::Slider::new(&mut mat.ambient, 0.0..=1.0)).changed() { *project_changed = true; }
                });
                ui.horizontal(|ui| {
                    ui.label("Diffuse:");
                    if ui.add(egui::Slider::new(&mut mat.diffuse, 0.0..=1.0)).changed() { *project_changed = true; }
                });
                ui.horizontal(|ui| {
                    ui.label("Specular:");
                    if ui.add(egui::Slider::new(&mut mat.specular, 0.0..=1.0)).changed() { *project_changed = true; }
                });
                ui.horizontal(|ui| {
                    ui.label("Shininess:");
                    if ui.add(egui::Slider::new(&mut mat.specular_exponent, 1.0..=256.0)).changed() { *project_changed = true; }
                });
                ui.horizontal(|ui| {
                    ui.label("Metalness:");
                    if ui.add(egui::Slider::new(&mut mat.metalness, 0.0..=1.0)).changed() { *project_changed = true; }
                });
                ui.horizontal(|ui| {
                    ui.label("Emission:");
                    if ui.add(egui::Slider::new(&mut mat.emission, 0.0..=1.0)).changed() { *project_changed = true; }
                });
            });
        } else {
            ui.label("Transform 2D");
            
            let val_before = layer.transform.anchor_point.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Anchor Point", &mut layer.transform.anchor_point, |ui, val| {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                    ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                });
            }) { *next_frame = Some(nf); }

            // 🎯 3x3 Anchor Point Quick Grid Picker
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                ui.small("Snap Grid: ");
                let b_size = layer.bounding_size();
                let (w, h) = (b_size[0], b_size[1]);

                for (label, ax, ay) in [
                    ("◤", 0.0, 0.0),      // Top-Left
                    ("▲", w * 0.5, 0.0),  // Top-Center
                    ("◥", w, 0.0),        // Top-Right
                    ("◀", 0.0, h * 0.5),  // Mid-Left
                    ("🎯", w * 0.5, h * 0.5), // Center
                    ("▶", w, h * 0.5),    // Mid-Right
                    ("◣", 0.0, h),        // Bottom-Left
                    ("▼", w * 0.5, h),    // Bottom-Center
                    ("◢", w, h),          // Bottom-Right
                ] {
                    if ui.small_button(label).on_hover_text(format!("Snap Anchor Point to ({:.0}, {:.0})", ax, ay)).clicked() {
                        layer.transform.anchor_point = crate::core::property::Animatable::new_constant([ax, ay]);
                        *project_changed = true;
                    }
                }
            });

            if val_before != layer.transform.anchor_point { *project_changed = true; }
            // Anchor point expression (same rich editor as other properties)
            draw_expression_selector(ui, "anchor", &mut layer.transform.anchor_point_expression, project_changed, Some(current_frame), Some(fps));

            ui.separator();
            let pos_before = layer.transform.position.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Position", &mut layer.transform.position, |ui, val| {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X: "));
                    ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y: "));
                });
            }) { *next_frame = Some(nf); }
            draw_easy_ease_button(ui, &mut layer.transform.position, project_changed);
            draw_expression_selector(ui, "position", &mut layer.transform.position_expression, project_changed, Some(current_frame), Some(fps));
            if pos_before != layer.transform.position { *project_changed = true; }

            ui.separator();
            let scale_before = layer.transform.scale.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Scale", &mut layer.transform.scale, |ui, val| {
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut val[0]).speed(0.1).suffix("% X"));
                    ui.add(egui::DragValue::new(&mut val[1]).speed(0.1).suffix("% Y"));
                });
            }) { *next_frame = Some(nf); }
            draw_easy_ease_button(ui, &mut layer.transform.scale, project_changed);
            draw_expression_selector(ui, "scale", &mut layer.transform.scale_expression, project_changed, Some(current_frame), Some(fps));
            if scale_before != layer.transform.scale { *project_changed = true; }

            ui.separator();
            let rot_before = layer.transform.rotation.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Rotation", &mut layer.transform.rotation, |ui, val| {
                ui.add(egui::Slider::new(val, -360.0..=360.0).suffix("°"));
            }) { *next_frame = Some(nf); }
            draw_easy_ease_button(ui, &mut layer.transform.rotation, project_changed);
            draw_expression_selector(ui, "rotation", &mut layer.transform.rotation_expression, project_changed, Some(current_frame), Some(fps));
            if rot_before != layer.transform.rotation { *project_changed = true; }

            // ── Auto-Orient (AE parity): rotation follows motion path ──
            {
                use crate::core::auto_orient::AutoOrientMode;
                let ao_before = layer.auto_orient;
                let mode_text = match layer.auto_orient {
                    AutoOrientMode::Off => "Off",
                    AutoOrientMode::OrientAlongPath => "Orient Along Path",
                    AutoOrientMode::OrientTowardsPoint { .. } => "Orient Towards Point",
                };
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Auto-Orient").small().color(colors::TEXT_SECONDARY));
                    egui::ComboBox::from_id_salt("insp_auto_orient")
                        .selected_text(mode_text)
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(layer.auto_orient == AutoOrientMode::Off, "Off").clicked() {
                                layer.auto_orient = AutoOrientMode::Off;
                            }
                            if ui.selectable_label(layer.auto_orient == AutoOrientMode::OrientAlongPath, "Orient Along Path")
                                .on_hover_text("Layer rotates to follow its position motion path")
                                .clicked()
                            {
                                layer.auto_orient = AutoOrientMode::OrientAlongPath;
                            }
                            let cur_target = match layer.auto_orient {
                                AutoOrientMode::OrientTowardsPoint { target_point } => target_point,
                                _ => [960.0, 540.0],
                            };
                            if ui.selectable_label(matches!(layer.auto_orient, AutoOrientMode::OrientTowardsPoint { .. }), "Orient Towards Point")
                                .on_hover_text("Layer rotates to face a fixed point")
                                .clicked()
                            {
                                layer.auto_orient = AutoOrientMode::OrientTowardsPoint { target_point: cur_target };
                            }
                        });
                });
                if let AutoOrientMode::OrientTowardsPoint { target_point } = &mut layer.auto_orient {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Target").small().color(colors::TEXT_SECONDARY));
                        ui.add(egui::DragValue::new(&mut target_point[0]).prefix("X: ").speed(1.0));
                        ui.add(egui::DragValue::new(&mut target_point[1]).prefix("Y: ").speed(1.0));
                    });
                }
                if ao_before != layer.auto_orient { *project_changed = true; }
            }

            ui.separator();
            let _op_before = layer.transform.opacity.clone();
            if let Some(nf) = draw_property_ui(current_frame, ui, "Opacity", &mut layer.transform.opacity, |ui, val| {
                ui.add(egui::Slider::new(val, 0.0..=100.0).suffix("%"));
            }) { *next_frame = Some(nf); }
            draw_easy_ease_button(ui, &mut layer.transform.opacity, project_changed);
            draw_expression_selector(ui, "opacity", &mut layer.transform.opacity_expression, project_changed, Some(current_frame), Some(fps));
            if layer.is_3d {
                ui.separator();
                ui.label(egui::RichText::new("🧊 3D Spatial Transform").small().strong().color(colors::ACCENT_CYAN));
                
                let mut pos3d = layer.transform_3d.position.evaluate(current_frame);
                ui.horizontal(|ui| {
                    ui.label("Position Z (Depth):");
                    if ui.add(egui::DragValue::new(&mut pos3d[2]).speed(1.0).suffix(" px")).changed() {
                        layer.transform_3d.position = crate::core::property::Animatable::new_constant(pos3d);
                        *project_changed = true;
                    }
                });

                let mut rot3d = layer.transform_3d.rotation.evaluate(current_frame);
                ui.horizontal(|ui| {
                    ui.label("X Rotation:");
                    if ui.add(egui::DragValue::new(&mut rot3d[0]).speed(1.0).suffix("°")).changed() {
                        layer.transform_3d.rotation = crate::core::property::Animatable::new_constant(rot3d);
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Y Rotation:");
                    if ui.add(egui::DragValue::new(&mut rot3d[1]).speed(1.0).suffix("°")).changed() {
                        layer.transform_3d.rotation = crate::core::property::Animatable::new_constant(rot3d);
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Z Rotation:");
                    if ui.add(egui::DragValue::new(&mut rot3d[2]).speed(1.0).suffix("°")).changed() {
                        layer.transform_3d.rotation = crate::core::property::Animatable::new_constant(rot3d);
                        *project_changed = true;
                    }
                });

                // ── 3D Material Options ──
                ui.separator();
                ui.label(egui::RichText::new("🎨 Material Options").small().strong().color(colors::ACCENT_YELLOW));

                ui.horizontal(|ui| {
                    ui.label("Ambient:");
                    if ui.add(egui::Slider::new(&mut layer.material.ambient, 0.0..=1.0).step_by(0.01)).changed() {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Diffuse:");
                    if ui.add(egui::Slider::new(&mut layer.material.diffuse, 0.0..=1.0).step_by(0.01)).changed() {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Specular:");
                    if ui.add(egui::Slider::new(&mut layer.material.specular, 0.0..=1.0).step_by(0.01)).changed() {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Specular Exp:");
                    if ui.add(egui::DragValue::new(&mut layer.material.specular_exponent).speed(1.0).range(1.0..=256.0)).changed() {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Emission:");
                    if ui.add(egui::Slider::new(&mut layer.material.emission, 0.0..=1.0).step_by(0.01)).changed() {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Metalness:");
                    if ui.add(egui::Slider::new(&mut layer.material.metalness, 0.0..=1.0).step_by(0.01)).changed() {
                        *project_changed = true;
                    }
                });
            }
        }

        ui.separator();
        ui.collapsing("📌 Responsive Constraints (Pinning)", |ui| {
            use crate::core::layer_constraints::{HorizontalPin, VerticalPin};
            let constraints_before = layer.constraints;

            ui.horizontal(|ui| {
                ui.label("Horizontal Pin:");
                egui::ComboBox::from_id_salt(format!("pin_h_{}", layer.id))
                    .selected_text(format!("{:?}", layer.constraints.horizontal))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut layer.constraints.horizontal, HorizontalPin::Left, "Left");
                        ui.selectable_value(&mut layer.constraints.horizontal, HorizontalPin::Center, "Center");
                        ui.selectable_value(&mut layer.constraints.horizontal, HorizontalPin::Right, "Right");
                        ui.selectable_value(&mut layer.constraints.horizontal, HorizontalPin::Scale, "Scale");
                    });
            });

            ui.horizontal(|ui| {
                ui.label("Vertical Pin:");
                egui::ComboBox::from_id_salt(format!("pin_v_{}", layer.id))
                    .selected_text(format!("{:?}", layer.constraints.vertical))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut layer.constraints.vertical, VerticalPin::Top, "Top");
                        ui.selectable_value(&mut layer.constraints.vertical, VerticalPin::Center, "Center");
                        ui.selectable_value(&mut layer.constraints.vertical, VerticalPin::Bottom, "Bottom");
                        ui.selectable_value(&mut layer.constraints.vertical, VerticalPin::Scale, "Scale");
                    });
            });

            if constraints_before != layer.constraints {
                *project_changed = true;
            }
        });

        // 🏷 Label Color picker (AE standard track color)
        ui.collapsing("🏷 Label Color", |ui| {
            ui.horizontal(|ui| {
                use crate::core::timeline::LabelColor;
                for color in [
                    LabelColor::None, LabelColor::Red, LabelColor::Yellow,
                    LabelColor::Aqua, LabelColor::Pink, LabelColor::Lavender,
                    LabelColor::Peach, LabelColor::Sea, LabelColor::Blue,
                    LabelColor::Purple,
                ] {
                    let rgb = color.to_rgb();
                    let c32 = egui::Color32::from_rgb(
                        (rgb[0] * 255.0) as u8, (rgb[1] * 255.0) as u8, (rgb[2] * 255.0) as u8,
                    );
                    let btn = ui.add(egui::Button::new("  ").fill(c32).min_size(egui::vec2(18.0, 18.0)));
                    if btn.clicked() && layer.label != color {
                        layer.label = color;
                        *project_changed = true;
                    }
                    if layer.label == color {
                        ui.painter().rect_stroke(btn.rect, 2.0, egui::Stroke::new(2.0, egui::Color32::WHITE));
                    }
                }
            });
        });

        // 🎨 Layer Styles (Drop Shadow & Stroke) UI Controls
        ui.collapsing("🎨 Layer Styles", |ui| {
            let ds = &mut layer.style.drop_shadow;
            ui.collapsing("👥 Drop Shadow", |ui| {
                if ui.checkbox(&mut ds.enabled, "Enabled").changed() {
                    *project_changed = true;
                }
                ui.horizontal(|ui| {
                    ui.label("Distance:");
                    if ui.add(egui::Slider::new(&mut ds.distance, 0.0..=200.0).suffix(" px")).changed() {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Blur Size:");
                    if ui.add(egui::Slider::new(&mut ds.size, 0.0..=100.0).suffix(" px")).changed() {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Opacity:");
                    if ui.add(egui::Slider::new(&mut ds.opacity, 0.0..=1.0)).changed() {
                        *project_changed = true;
                    }
                });
            });

            let st = &mut layer.style.stroke;
            ui.collapsing("✏️ Stroke", |ui| {
                if ui.checkbox(&mut st.enabled, "Enabled").changed() {
                    *project_changed = true;
                }
                ui.horizontal(|ui| {
                    ui.label("Size:");
                    if ui.add(egui::Slider::new(&mut st.size, 1.0..=50.0).suffix(" px")).changed() {
                        *project_changed = true;
                    }
                });
            });
        });
    });
}

pub fn draw_layer_type_specs(
    ui: &mut egui::Ui,
    layer: &mut Layer,
    current_frame: u32,
    project_changed: &mut bool,
    next_frame: &mut Option<u32>,
    comp_fps: f32,
) {
    ui.group(|ui| {
        ui.label("Layer Specs");
        match &mut layer.layer_type {
            LayerType::Solid { color } => {
                let val_before = *color;
                ui.horizontal(|ui| {
                    ui.label("Color:");
                    ui.color_edit_button_rgba_unmultiplied(color);
                });
                if val_before != *color { *project_changed = true; }
            }
            LayerType::Image { path } => {
                let val_before = path.clone();
                ui.text_edit_singleline(path);
                if !path.is_empty() && ui.small("📂 Reveal in Finder").on_hover_text("Open the source file location in Finder").clicked() {
                    let _ = std::process::Command::new("open")
                        .arg("-R")
                        .arg(path.as_str())
                        .spawn();
                }
                if val_before != *path { *project_changed = true; }
            }
            LayerType::Video { source, frames_dir, frame_count, audio_wav, speed } => {
                let before_src = source.clone();
                let before_frames = frames_dir.clone();
                let before_speed = *speed;
                let before_count = *frame_count;
                ui.label(egui::RichText::new("Video Layer").strong());
                ui.horizontal(|ui| {
                    ui.label("Source:");
                    ui.text_edit_singleline(source);
                });
                if !source.is_empty() && ui.small("📂 Reveal in Finder").on_hover_text("Open the source file location in Finder").clicked() {
                    let _ = std::process::Command::new("open")
                        .arg("-R")
                        .arg(source.as_str())
                        .spawn();
                }
                ui.horizontal(|ui| {
                    ui.label("Frames dir:");
                    ui.text_edit_singleline(frames_dir);
                });
                // Playback speed: 1.0 = realtime, 0.5 = half, 2.0 = double
                ui.horizontal(|ui| {
                    ui.label("Speed:");
                    ui.add(
                        egui::DragValue::new(speed)
                            .speed(0.05)
                            .range(0.05..=10.0)
                            .suffix("x"),
                    );
                    if ui.button("1x").on_hover_text("Reset to realtime").clicked() {
                        *speed = 1.0;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Frame count:");
                    ui.add(egui::DragValue::new(frame_count).range(1..=100_000));
                });
                ui.label(format!(
                    "Effective duration: {:.1}s at {:.2}x | audio: {}",
                    *frame_count as f32 / comp_fps.max(1.0) / speed.max(0.01),
                    speed,
                    if audio_wav.is_some() { "yes" } else { "no" }
                ));

                // Time remap toggle
                ui.horizontal(|ui| {
                    let remap_enabled = layer.time_remap.is_some();
                    let mut new_enabled = remap_enabled;
                    if ui.checkbox(&mut new_enabled, "Time Remap").changed() {
                        if new_enabled && !remap_enabled {
                            // Initialize remap: linear 0..frame_count mapping
                            layer.time_remap = Some(crate::core::property::Animatable::new_animated(vec![
                                crate::core::keyframe::Keyframe::new(0, 0.0, crate::core::keyframe::InterpolationType::Linear),
                                crate::core::keyframe::Keyframe::new(0u32, 0.0f32, crate::core::keyframe::InterpolationType::Linear),
                            ]));
                            *project_changed = true;
                        } else if !new_enabled && remap_enabled {
                            layer.time_remap = None;
                            *project_changed = true;
                        }
                    }
                });
                if let Some(remap) = &mut layer.time_remap {
                    ui.small("Time remap: source frame ← timeline frame");
                    let kfs = remap.keyframes();
                    if let Some(kfs) = kfs {
                        ui.small(format!("  {} keyframes", kfs.len()));
                    }
                }
                if before_src != *source || before_frames != *frames_dir
                    || before_speed != *speed || before_count != *frame_count {
                    *project_changed = true;
                }
            }
            LayerType::Text { text, font_size, color, .. } => {
                let val_before_text = text.clone();
                let val_before_sz = *font_size;
                let val_before_col = *color;
                
                ui.text_edit_multiline(text);
                ui.horizontal(|ui| {
                    ui.label("Font Size:");
                    ui.add(egui::DragValue::new(font_size).range(8..=256));
                });
                ui.horizontal(|ui| {
                    ui.label("Color:");
                    ui.color_edit_button_rgba_unmultiplied(color);
                });
                if val_before_text != *text || val_before_sz != *font_size || val_before_col != *color {
                    *project_changed = true;
                }

                ui.separator();
                ui.collapsing("Text Animator", |ui| {
                    if layer.text_animator.is_none() {
                        layer.text_animator = Some(TextAnimatorSettings::default());
                    }
                    let Some(anim) = layer.text_animator.as_mut() else {
                        ui.label(egui::RichText::new("Animator unavailable").small().color(colors::TEXT_MUTED));
                        return;
                    };
                    let enabled_before = anim.enabled;
                    ui.checkbox(&mut anim.enabled, "Enable Animator");
                    if enabled_before != anim.enabled {
                        *project_changed = true;
                    }

                    ui.horizontal(|ui| {
                        ui.label("Range Start:");
                        if ui.add(egui::DragValue::new(&mut anim.selector.start).speed(0.5).suffix("%").range(0.0..=100.0)).changed() {
                            *project_changed = true;
                        }
                        ui.label("End:");
                        if ui.add(egui::DragValue::new(&mut anim.selector.end).speed(0.5).suffix("%").range(0.0..=100.0)).changed() {
                            *project_changed = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Range Offset:");
                        if ui.add(egui::Slider::new(&mut anim.selector.offset, -100.0..=100.0).suffix("%")).changed() {
                            *project_changed = true;
                        }
                    });
                    ui.horizontal(|ui| {

                        ui.label("Shape:");
                        let shape_before = anim.selector.shape;
                        egui::ComboBox::from_id_salt(format!("text_anim_shape_{}", layer.id))
                            .selected_text(format!("{:?}", anim.selector.shape))
                            .show_ui(ui, |ui| {
                                for shape in [
                                    SelectorShape::Square,
                                    SelectorShape::RampUp,
                                    SelectorShape::RampDown,
                                    SelectorShape::Triangle,
                                    SelectorShape::Round,
                                    SelectorShape::Smooth,
                                ] {
                                    ui.selectable_value(&mut anim.selector.shape, shape, format!("{:?}", shape));
                                }
                            });
                        if shape_before != anim.selector.shape {
                            *project_changed = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Position Offset:");
                        if ui.add(egui::DragValue::new(&mut anim.position_offset[0]).speed(0.5).prefix("X: ")).changed()
                            || ui.add(egui::DragValue::new(&mut anim.position_offset[1]).speed(0.5).prefix("Y: ")).changed()
                        {
                            *project_changed = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Opacity Target:");
                        if ui.add(egui::Slider::new(&mut anim.opacity, 0.0..=1.0)).changed() {
                            *project_changed = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Tracking:");
                        if ui.add(egui::DragValue::new(&mut anim.tracking).speed(0.5)).changed() {
                            *project_changed = true;
                        }
                    });
                });

                // ── Advanced Animator Stack (multi-animator composition) ──
                ui.separator();
                ui.collapsing("Animator Stack", |ui| {
                    if layer.text_animator_stack.is_none() {
                        layer.text_animator_stack = Some(crate::core::text_animator_advanced::AnimatorStack::default());
                    }
                    let stack = layer.text_animator_stack.as_mut().expect("text_animator_stack");
                    let mut pending_remove: Option<usize> = None;

                    for (ai, adv) in stack.animators.iter_mut().enumerate() {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut adv.enabled, "");
                                ui.label(egui::RichText::new(format!("Animator {}", ai + 1)).strong());
                                if ui.small_button("✕").on_hover_text("Remove").clicked() {
                                    pending_remove = Some(ai);
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Unit:");
                                egui::ComboBox::from_id_salt(format!("adv_unit_{}_{}", layer.id, ai))
                                    .selected_text(format!("{:?}", adv.unit))
                                    .show_ui(ui, |ui| {
                                        use crate::core::text_animator_advanced::SelectorUnit;
                                        for u in [SelectorUnit::Characters, SelectorUnit::Words, SelectorUnit::Lines] {
                                            ui.selectable_value(&mut adv.unit, u, format!("{:?}", u));
                                        }
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.label("Range:");
                                ui.add(egui::DragValue::new(&mut adv.selector.start).speed(0.5).suffix("%").range(0.0..=100.0));
                                ui.label("→");
                                ui.add(egui::DragValue::new(&mut adv.selector.end).speed(0.5).suffix("%").range(0.0..=100.0));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Shape:");
                                egui::ComboBox::from_id_salt(format!("adv_shape_{}_{}", layer.id, ai))
                                    .selected_text(format!("{:?}", adv.selector.shape))
                                    .show_ui(ui, |ui| {
                                        use crate::core::text_animator::SelectorShape;
                                        for s in [SelectorShape::Square, SelectorShape::RampUp, SelectorShape::RampDown, SelectorShape::Triangle, SelectorShape::Round, SelectorShape::Smooth] {
                                            ui.selectable_value(&mut adv.selector.shape, s, format!("{:?}", s));
                                        }
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.label("Position:");
                                ui.add(egui::DragValue::new(&mut adv.position[0]).speed(0.5).prefix("X: "));
                                ui.add(egui::DragValue::new(&mut adv.position[1]).speed(0.5).prefix("Y: "));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Scale:");
                                ui.add(egui::DragValue::new(&mut adv.scale[0]).speed(0.5).prefix("X: ").range(0.0..=10.0));
                                ui.add(egui::DragValue::new(&mut adv.scale[1]).speed(0.5).prefix("Y: ").range(0.0..=10.0));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Opacity:");
                                ui.add(egui::Slider::new(&mut adv.opacity, 0.0..=1.0));
                                ui.label("Rotation:");
                                ui.add(egui::DragValue::new(&mut adv.rotation).speed(1.0).suffix("°"));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Blur:");
                                ui.add(egui::DragValue::new(&mut adv.blur).speed(0.5).range(0.0..=50.0));
                                ui.label("Tracking:");
                                ui.add(egui::DragValue::new(&mut adv.tracking).speed(0.5));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Skew:");
                                ui.add(egui::DragValue::new(&mut adv.advanced.skew).speed(0.5).suffix("°"));
                                ui.label("Axis:");
                                ui.add(egui::DragValue::new(&mut adv.advanced.skew_axis).speed(0.5).suffix("°"));
                            });
                            ui.horizontal(|ui| {
                                ui.label("Char Offset:");
                                ui.add(egui::DragValue::new(&mut adv.advanced.character_offset).speed(1));
                            });
                        });
                        ui.add_space(2.0);
                    }

                    if let Some(ai) = pending_remove {
                        stack.animators.remove(ai);
                        *project_changed = true;
                    }

                    if ui.small_button("+ Add Animator").clicked() {
                        stack.animators.push(crate::core::text_animator_advanced::TextAnimatorAdvanced::default());
                        *project_changed = true;
                    }
                });
            }
            LayerType::Shape { shape_type, color, stroke_color, stroke_width, .. } => {
                ui.label(format!("Shape: {:?}", shape_type));
                let mut c_arr = *color;
                ui.horizontal(|ui| {
                    ui.label("Fill Color:");
                    if ui.color_edit_button_rgba_unmultiplied(&mut c_arr).changed() {
                        *color = c_arr;
                        *project_changed = true;
                    }
                });
                let mut sc_arr = *stroke_color;
                ui.horizontal(|ui| {
                    ui.label("Stroke Color:");
                    if ui.color_edit_button_rgba_unmultiplied(&mut sc_arr).changed() {
                        *stroke_color = sc_arr;
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Stroke Width:");
                    if ui.add(egui::DragValue::new(stroke_width).speed(0.5).range(0.0..=50.0).suffix(" px")).changed() {
                        *project_changed = true;
                    }
                });

                // ── Trim Paths (AE Shape Operator) ──
                ui.add_space(4.0);
                ui.collapsing("✂ Trim Paths", |ui| {
                    let has_trim = layer.trim_paths.is_some();
                    ui.horizontal(|ui| {
                        if !has_trim {
                            if ui.button("+ Add Trim Paths").clicked() {
                                layer.trim_paths = Some(crate::core::timeline::TrimPaths::default());
                                *project_changed = true;
                            }
                        } else if ui.small_button("🗑 Remove").clicked() {
                            layer.trim_paths = None;
                            *project_changed = true;
                        }
                    });

                    if let Some(ref mut trim) = layer.trim_paths {
                        let mut start_val = trim.start.evaluate(current_frame);
                        ui.horizontal(|ui| {
                            ui.label("Start:");
                            if ui.add(egui::Slider::new(&mut start_val, 0.0..=100.0).suffix("%")).changed() {
                                trim.start = crate::core::property::Animatable::new_constant(start_val);
                                *project_changed = true;
                            }
                        });

                        let mut end_val = trim.end.evaluate(current_frame);
                        ui.horizontal(|ui| {
                            ui.label("End:");
                            if ui.add(egui::Slider::new(&mut end_val, 0.0..=100.0).suffix("%")).changed() {
                                trim.end = crate::core::property::Animatable::new_constant(end_val);
                                *project_changed = true;
                            }
                        });

                        let mut offset_val = trim.offset.evaluate(current_frame);
                        ui.horizontal(|ui| {
                            ui.label("Offset:");
                            if ui.add(egui::DragValue::new(&mut offset_val).suffix("°")).changed() {
                                trim.offset = crate::core::property::Animatable::new_constant(offset_val);
                                *project_changed = true;
                            }
                        });
                    }
                });

                // ── Merge Paths & Offset Paths (AE Vector Operators) ──
                ui.collapsing("🔗 Vector Path Operators", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Merge Paths Mode:");
                        let mut merge_mode = ui.ctx().data(|d| d.get_temp::<i32>(egui::Id::new("shape_merge_mode")).unwrap_or(0));
                        egui::ComboBox::from_id_salt("shape_merge_mode_combo")
                            .selected_text(match merge_mode {
                                0 => "Add (Union)",
                                1 => "Subtract (Difference)",
                                2 => "Intersect",
                                3 => "Exclude (XOR)",
                                _ => "Add",
                            })
                            .show_ui(ui, |ui| {
                                if ui.selectable_value(&mut merge_mode, 0, "Add (Union)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("shape_merge_mode"), 0)); *project_changed = true; }
                                if ui.selectable_value(&mut merge_mode, 1, "Subtract (Difference)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("shape_merge_mode"), 1)); *project_changed = true; }
                                if ui.selectable_value(&mut merge_mode, 2, "Intersect").clicked() { ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("shape_merge_mode"), 2)); *project_changed = true; }
                                if ui.selectable_value(&mut merge_mode, 3, "Exclude (XOR)").clicked() { ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("shape_merge_mode"), 3)); *project_changed = true; }
                            });
                    });

                    let mut offset_dist = ui.ctx().data(|d| d.get_temp::<f32>(egui::Id::new("shape_offset_path_amt")).unwrap_or(0.0));
                    ui.horizontal(|ui| {
                        ui.label("Offset Paths:");
                        if ui.add(egui::DragValue::new(&mut offset_dist).speed(0.5).range(-100.0..=100.0).suffix(" px")).changed() {
                            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("shape_offset_path_amt"), offset_dist));
                            *project_changed = true;
                        }
                    });
                });
            }
            LayerType::Null => {
                ui.label("Null Object (Controller)");
            }
            LayerType::PreComp { comp_id } => {
                ui.label(format!("Pre-Composition ({})", comp_id));
            }
            LayerType::Audio { path, volume } => {
                ui.label(format!("Audio File ({})", path));
                if !path.is_empty() && ui.small("📂 Reveal in Finder").on_hover_text("Open the source file location in Finder").clicked() {
                    let _ = std::process::Command::new("open")
                        .arg("-R")
                        .arg(path.as_str())
                        .spawn();
                }
                let v_before = volume.clone();
                if let Some(nf) = draw_property_ui(current_frame, ui, "  Volume", volume, |ui, val| {
                    ui.add(egui::Slider::new(val, -48.0..=12.0).suffix(" dB"));
                }) {
                    *next_frame = Some(nf);
                }
                if v_before != *volume { *project_changed = true; }
            }
            LayerType::AdjustmentLayer => {
                ui.label("⚙ Adjustment Layer (Applies effects to lower composite)");
            }
            LayerType::Particle { emitter } => {
                ui.label("✦ Particle Emitter");
                let mut changed = false;
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::DragValue::new(&mut emitter.rate).suffix(" /s")).changed();
                    ui.label("Rate");
                });
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::DragValue::new(&mut emitter.lifetime).suffix(" s")).changed();
                    ui.label("Lifetime");
                });
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::Slider::new(&mut emitter.speed, 0.0..=2000.0)).changed();
                    ui.label("Speed");
                });
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::Slider::new(&mut emitter.spread_degrees, 0.0..=360.0).suffix("°")).changed();
                    ui.label("Spread");
                });
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::DragValue::new(&mut emitter.gravity[1])).changed();
                    ui.label("Gravity Y");
                });
                ui.horizontal(|ui| {
                    changed |= ui.add(egui::Slider::new(&mut emitter.turbulence, 0.0..=1000.0)).changed();
                    ui.label("Turbulence");
                });
                ui.collapsing("🌀 Forces", |ui| {
                    ui.horizontal(|ui| {
                        changed |= ui.add(egui::Slider::new(&mut emitter.vortex_strength, -2000.0..=2000.0)).changed();
                        ui.label("Vortex (+CW)");
                    });
                    ui.horizontal(|ui| {
                        changed |= ui.add(egui::DragValue::new(&mut emitter.vortex_center[0])).changed();
                        changed |= ui.add(egui::DragValue::new(&mut emitter.vortex_center[1])).changed();
                        ui.label("Vortex Center");
                    });
                    ui.horizontal(|ui| {
                        changed |= ui.add(egui::Slider::new(&mut emitter.attract_strength, -2000.0..=2000.0)).changed();
                        ui.label("Attract (+) / Repel (−)");
                    });
                    ui.horizontal(|ui| {
                        changed |= ui.add(egui::DragValue::new(&mut emitter.attract_center[0])).changed();
                        changed |= ui.add(egui::DragValue::new(&mut emitter.attract_center[1])).changed();
                        ui.label("Force Point");
                    });
                    ui.separator();
                    let mut ds = emitter.death_spawn_count as i32;
                    if ui.add(egui::DragValue::new(&mut ds).range(0..=32).prefix("×")).changed() {
                        emitter.death_spawn_count = ds.max(0) as u32;
                        changed = true;
                    }
                    ui.label("Death Spawn");
                    if emitter.death_spawn_count > 0 {
                        ui.horizontal(|ui| {
                            changed |= ui.add(egui::Slider::new(&mut emitter.death_spawn_speed_scale, 0.05..=2.0)).changed();
                            ui.label("Child Speed ×");
                        });
                        ui.horizontal(|ui| {
                            changed |= ui.add(egui::Slider::new(&mut emitter.death_spawn_life_scale, 0.05..=1.0)).changed();
                            ui.label("Child Life ×");
                        });
                    }
                });
                ui.collapsing("✨ Trail", |ui| {
                    ui.horizontal(|ui| {
                        let mut tl = emitter.trail_length as i32;
                        if ui.add(egui::Slider::new(&mut tl, 0..=8)).changed() {
                            emitter.trail_length = tl as u8;
                            changed = true;
                        }
                        ui.label("Length");
                    });
                    ui.horizontal(|ui| {
                        changed |= ui.add(egui::Slider::new(&mut emitter.trail_taper, 0.1..=1.0)).changed();
                        ui.label("Taper");
                    });
                });
                ui.collapsing("🎥 3D Depth", |ui| {
                    changed |= ui.checkbox(&mut emitter.depth_enabled, "Project through camera").changed();
                    if emitter.depth_enabled {
                        ui.horizontal(|ui| {
                            changed |= ui.add(egui::DragValue::new(&mut emitter.depth_range[0]).suffix(" min")).changed();
                            ui.label("Z Range");
                            changed |= ui.add(egui::DragValue::new(&mut emitter.depth_range[1]).suffix(" max")).changed();
                        });
                        ui.label(egui::RichText::new("Particles scale with distance from the active camera").small().color(colors::TEXT_MUTED));
                    }
                });
                ui.horizontal(|ui| {
                    changed |= ui.color_edit_button_rgba_unmultiplied(&mut emitter.color_start).changed();
                    ui.label("Start Color");
                });
                ui.horizontal(|ui| {
                    changed |= ui.color_edit_button_rgba_unmultiplied(&mut emitter.color_end).changed();
                    ui.label("End Color");
                });
                if changed { *project_changed = true; }
            }
        }

        ui.separator();
        // ── Shape Repeater (AE Contents > Repeater parity) ──
        ui.collapsing("⧉ Shape Repeater", |ui| {
            if layer.shape_repeater.is_none() {
                if ui.button("+ Add Repeater").on_hover_text("Duplicate the shape N times with cumulative offsets").clicked() {
                    layer.shape_repeater = Some(crate::core::shape_repeater::ShapeRepeaterOptions::default());
                    *project_changed = true;
                }
            } else if let Some(rep) = &mut layer.shape_repeater {
                ui.horizontal(|ui| {
                    ui.label("Copies:");
                    let mut copies_i = rep.copies as i32;
                    if ui.add(egui::DragValue::new(&mut copies_i).range(1..=500).speed(1)).changed() {
                        rep.copies = copies_i.max(1) as u32;
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Offset:");
                    if ui.add(egui::DragValue::new(&mut rep.offset).range(-100.0..=100.0).speed(0.1)).changed() {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Position Offset:");
                    if ui.add(egui::DragValue::new(&mut rep.position_offset[0]).speed(1.0).prefix("X ")).changed()
                        || ui.add(egui::DragValue::new(&mut rep.position_offset[1]).speed(1.0).prefix("Y ")).changed() {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Scale /copy %:");
                    if ui.add(egui::DragValue::new(&mut rep.scale_offset[0]).range(0.05..=5.0).speed(0.01).suffix("×")).changed()
                        || ui.add(egui::DragValue::new(&mut rep.scale_offset[1]).range(0.05..=5.0).speed(0.01).suffix("×")).changed() {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Rotation /copy:");
                    if ui.add(egui::DragValue::new(&mut rep.rotation_offset_deg).speed(1.0).suffix("°")).changed() {
                        *project_changed = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Opacity fade:");
                    if ui.add(egui::Slider::new(&mut rep.start_opacity, 0.0..=1.0).suffix("%")).changed()
                        || ui.add(egui::Slider::new(&mut rep.end_opacity, 0.0..=1.0).suffix("%")).changed() {
                        *project_changed = true;
                    }
                });
                if ui.checkbox(&mut rep.composite_below, "Composite below").changed() {
                    *project_changed = true;
                }
                if ui.button("🗑 Remove Repeater").clicked() {
                    layer.shape_repeater = None;
                    *project_changed = true;
                }
            }
        });

        ui.separator();
        // ── Puppet Pins (deformation mesh handles, AE Puppet Tool parity) ──
        ui.collapsing("🧷 Puppet Pins & Mesh", |ui| {
            ui.horizontal(|ui| {
                if ui.button("+ Position Pin").on_hover_text("Deformation position pin").clicked() {
                    let n = layer.puppet_pins.len() + 1;
                    let center = layer.transform.position.evaluate(current_frame);
                    layer.puppet_pins.push(crate::core::timeline::PuppetPin::new(
                        format!("pin_{}", n),
                        format!("Pin {}", n),
                        center,
                    ));
                    *project_changed = true;
                }
                if ui.button("🧱 + Starch Pin").on_hover_text("Pin to add stiffness and prevent stretching").clicked() {
                    let n = layer.puppet_pins.len() + 1;
                    let center = layer.transform.position.evaluate(current_frame);
                    layer.puppet_pins.push(crate::core::timeline::PuppetPin::new(
                        format!("starch_{}", n),
                        format!("Starch {}", n),
                        center,
                    ));
                    *project_changed = true;
                }
            });

            let mut mesh_density = ui.ctx().data(|d| d.get_temp::<f32>(egui::Id::new("puppet_mesh_density")).unwrap_or(50.0));
            ui.horizontal(|ui| {
                ui.label("Mesh Density:");
                if ui.add(egui::Slider::new(&mut mesh_density, 10.0..=100.0).suffix(" tri")).changed() {
                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("puppet_mesh_density"), mesh_density));
                    *project_changed = true;
                }
            });
            let mut remove_idx: Option<usize> = None;
            for (pi, pin) in layer.puppet_pins.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("🧷 {}", pin.name)).small());
                    if let Some(nf) = draw_property_ui(current_frame, ui, "", &mut pin.position, |ui, val| {
                        ui.add(egui::DragValue::new(&mut val[0]).speed(1.0).prefix("X "));
                        ui.add(egui::DragValue::new(&mut val[1]).speed(1.0).prefix("Y "));
                    }) { *next_frame = Some(nf); }
                    if ui.small_button("🗑").on_hover_text("Remove pin").clicked() {
                        remove_idx = Some(pi);
                    }
                });
            }
            if let Some(ri) = remove_idx {
                layer.puppet_pins.remove(ri);
                *project_changed = true;
            }
        });

        ui.separator();
        // ── Paint Strokes ──
        ui.collapsing("🖌 Paint Strokes", |ui| {
            let n = layer.paint_strokes.len();
            if n == 0 {
                ui.label(egui::RichText::new("No strokes — use the Brush tool in the viewport").small().color(colors::TEXT_MUTED));
            } else {
                ui.label(egui::RichText::new(format!("{} stroke{}", n, if n == 1 { "" } else { "s" })).small().color(colors::TEXT_SECONDARY));
            }
            let mut remove_idx: Option<usize> = None;
            for (si, s) in layer.paint_strokes.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    let cr = (s.color[0] * 255.0) as u8;
                    let cg = (s.color[1] * 255.0) as u8;
                    let cb = (s.color[2] * 255.0) as u8;
                    let (rct, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                    ui.painter().rect_filled(rct, 2.0, egui::Color32::from_rgb(cr, cg, cb));
                    ui.label(egui::RichText::new(format!("Stroke {} · {} pts", si + 1, s.points.len())).small());
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("In").small().color(colors::TEXT_MUTED));
                    if ui.add(egui::DragValue::new(&mut s.start_frame).range(0..=999_999)).changed() {
                        *project_changed = true;
                    }
                    ui.label(egui::RichText::new("Out").small().color(colors::TEXT_MUTED));
                    let mut ef = s.end_frame;
                    let suffix = if ef == 0 { " (auto)" } else { "" };
                    if ui.add(egui::DragValue::new(&mut ef).range(0..=999_999).suffix(suffix)).changed() {
                        s.end_frame = ef;
                        *project_changed = true;
                    }
                    if ui.small_button("🗑").on_hover_text("Delete stroke").clicked() {
                        remove_idx = Some(si);
                    }
                });
            }
            if let Some(ri) = remove_idx {
                layer.paint_strokes.remove(ri);
                *project_changed = true;
            }
        });

        ui.separator();
        ui.collapsing("Trim Paths Animator", |ui| {
            if layer.trim_paths.is_none() {
                if ui.button("+ Add Trim Paths").clicked() {
                    layer.trim_paths = Some(crate::core::timeline::TrimPaths::default());
                    *project_changed = true;
                }
            } else if let Some(ref mut trim) = layer.trim_paths {
                ui.horizontal(|ui| {
                    ui.label("Start:");
                    if let Some(nf) = draw_property_ui(current_frame, ui, "Start", &mut trim.start, |ui, val| {
                        ui.add(egui::Slider::new(val, 0.0..=100.0).suffix("%"));
                    }) { *next_frame = Some(nf); }
                });
                ui.horizontal(|ui| {
                    ui.label("End:");
                    if let Some(nf) = draw_property_ui(current_frame, ui, "End", &mut trim.end, |ui, val| {
                        ui.add(egui::Slider::new(val, 0.0..=100.0).suffix("%"));
                    }) { *next_frame = Some(nf); }
                });
                ui.horizontal(|ui| {
                    ui.label("Offset:");
                    if let Some(nf) = draw_property_ui(current_frame, ui, "Offset", &mut trim.offset, |ui, val| {
                        ui.add(egui::DragValue::new(val).speed(1.0).suffix("°"));
                    }) { *next_frame = Some(nf); }
                });
            }
        });

        ui.separator();
        // ── Layer Styles Inspector Section ──
        ui.collapsing("🎨 Layer Styles", |ui| {
            let style = &mut layer.style;

            // Stroke
            ui.collapsing("✏ Stroke", |ui| {
                if ui.checkbox(&mut style.stroke.enabled, "Enabled").clicked() { *project_changed = true; }
                ui.horizontal(|ui| {
                    ui.label("Size:");
                    if ui.add(egui::Slider::new(&mut style.stroke.size, 1.0..=100.0).suffix(" px")).changed() { *project_changed = true; }
                });
                ui.horizontal(|ui| {
                    ui.label("Color:");
                    let c = &mut style.stroke.color;
                    let mut col = egui::Color32::from_rgba_premultiplied(
                        (c[0] * 255.0) as u8, (c[1] * 255.0) as u8,
                        (c[2] * 255.0) as u8, (c[3] * 255.0) as u8,
                    );
                    if ui.color_edit_button_srgba(&mut col).changed() {
                        let [r, g, b, a] = col.to_array();
                        *c = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0];
                        *project_changed = true;
                    }
                });
            });

            // Drop Shadow
            ui.collapsing("👤 Drop Shadow", |ui| {
                if ui.checkbox(&mut style.drop_shadow.enabled, "Enabled").clicked() { *project_changed = true; }
                ui.horizontal(|ui| {
                    ui.label("Opacity:");
                    if ui.add(egui::Slider::new(&mut style.drop_shadow.opacity, 0.0..=100.0).suffix("%")).changed() { *project_changed = true; }
                });
                ui.horizontal(|ui| {
                    ui.label("Distance / Size:");
                    if ui.add(egui::DragValue::new(&mut style.drop_shadow.distance).prefix("Dist: ")).changed() { *project_changed = true; }
                    if ui.add(egui::DragValue::new(&mut style.drop_shadow.size).prefix("Size: ")).changed() { *project_changed = true; }
                });
                ui.horizontal(|ui| {
                    ui.label("Angle:");
                    if ui.add(egui::Slider::new(&mut style.drop_shadow.angle, -180.0..=180.0).suffix("°")).changed() { *project_changed = true; }
                });
            });

            // Color Overlay
            ui.collapsing("🎨 Color Overlay", |ui| {
                if ui.checkbox(&mut style.color_overlay.enabled, "Enabled").clicked() { *project_changed = true; }
                ui.horizontal(|ui| {
                    ui.label("Opacity:");
                    if ui.add(egui::Slider::new(&mut style.color_overlay.opacity, 0.0..=100.0).suffix("%")).changed() { *project_changed = true; }
                });
                ui.horizontal(|ui| {
                    ui.label("Color:");
                    let c = &mut style.color_overlay.color;
                    let mut col = egui::Color32::from_rgba_premultiplied(
                        (c[0] * 255.0) as u8, (c[1] * 255.0) as u8,
                        (c[2] * 255.0) as u8, (c[3] * 255.0) as u8,
                    );
                    if ui.color_edit_button_srgba(&mut col).changed() {
                        let [r, g, b, a] = col.to_array();
                        *c = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0];
                        *project_changed = true;
                    }
                });
            });

            // Gradient Overlay
            ui.collapsing("🌈 Gradient Overlay", |ui| {
                if ui.checkbox(&mut style.gradient_overlay.enabled, "Enabled").clicked() { *project_changed = true; }
                let go = &mut style.gradient_overlay;
                ui.horizontal(|ui| {
                    ui.label("Opacity / Angle:");
                    if ui.add(egui::DragValue::new(&mut go.opacity).prefix("Op: ").suffix("%")).changed() { *project_changed = true; }
                    if ui.add(egui::DragValue::new(&mut go.angle).prefix("Ang: ").suffix("°")).changed() { *project_changed = true; }
                });
            });

            // Bevel & Emboss
            ui.collapsing("🪨 Bevel / Emboss", |ui| {
                if ui.checkbox(&mut style.bevel_emboss.enabled, "Enabled").clicked() { *project_changed = true; }
                let bv = &mut style.bevel_emboss;
                ui.horizontal(|ui| {
                    ui.label("Depth / Size:");
                    if ui.add(egui::DragValue::new(&mut bv.depth).prefix("Depth: ")).changed() { *project_changed = true; }
                    if ui.add(egui::DragValue::new(&mut bv.size).prefix("Size: ")).changed() { *project_changed = true; }
                });
                ui.horizontal(|ui| {
                    ui.label("Highlight / Shadow:");
                    if ui.add(egui::DragValue::new(&mut bv.highlight).prefix("Hi: ").suffix("%")).changed() { *project_changed = true; }
                    if ui.add(egui::DragValue::new(&mut bv.shadow).prefix("Sh: ").suffix("%")).changed() { *project_changed = true; }
                });
            });
        });
    });
}

