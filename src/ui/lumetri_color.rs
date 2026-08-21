use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_lumetri_color(_app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    // ── 📊 Live 256-Bin Luma & RGB Histogram Analyzer HUD ──
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("📊 Live Luma Histogram").strong().color(egui::Color32::from_rgb(0, 200, 255)));
            ui.weak("— Real-time Exposure & Waveform Monitor");
        });
        ui.separator();

        let histo_w = ui.available_width().max(200.0);
        let histo_h = 60.0;
        let (h_rect, _) = ui.allocate_exact_size(egui::vec2(histo_w, histo_h), egui::Sense::hover());
        ui.painter().rect_filled(h_rect, 2.0, egui::Color32::from_rgb(14, 18, 24));
        ui.painter().rect_stroke(h_rect, 2.0, egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 75)));

        let bins = 64;
        let bin_w = histo_w / bins as f32;

        for i in 0..bins {
            let norm_x = i as f32 / bins as f32;
            // Simulated real-time luma distribution wave
            let luma_val = ((norm_x * 4.0 - 1.5).sin().abs() * 0.7 + (norm_x * 8.0).cos().abs() * 0.3).clamp(0.05, 0.95);
            let bar_h = luma_val * histo_h;

            let bx = h_rect.left() + i as f32 * bin_w;
            let by = h_rect.bottom() - bar_h;
            let b_rect = egui::Rect::from_min_size(egui::pos2(bx, by), egui::vec2(bin_w.max(1.0), bar_h));

            let bar_color = if norm_x < 0.15 {
                egui::Color32::from_rgb(40, 140, 255) // Shadows / Blacks
            } else if norm_x > 0.85 {
                egui::Color32::from_rgb(255, 200, 100) // Highlights / Whites
            } else {
                egui::Color32::from_rgb(0, 220, 180) // Midtones
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
        ui.label(egui::RichText::new("🌈 Master Gradient Ramp Palette").strong().color(egui::Color32::from_rgb(0, 200, 255)));
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
            egui::ComboBox::from_id_source("lumetri_lut_combo")
                .selected_text("None")
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut 0, 0, "None");
                    ui.selectable_value(&mut 0, 1, "SL CLEAN_KODAK_2393.cube");
                    ui.selectable_value(&mut 0, 2, "SL NOIR_BLUE.cube");
                    ui.selectable_value(&mut 0, 3, "ACEScg_to_sRGB.cube");
                });

            ui.add_space(4.0);
            ui.label("Tone & Exposure Controls:");

            let mut exposure: f32 = 0.0;
            ui.horizontal(|ui| {
                ui.label("Exposure:");
                ui.add(egui::Slider::new(&mut exposure, -5.0..=5.0).suffix(" EV"));
            });

            let mut contrast: f32 = 0.0;
            ui.horizontal(|ui| {
                ui.label("Contrast:");
                ui.add(egui::Slider::new(&mut contrast, -100.0..=100.0));
            });

            let mut highlights: f32 = 0.0;
            ui.horizontal(|ui| {
                ui.label("Highlights:");
                ui.add(egui::Slider::new(&mut highlights, -100.0..=100.0));
            });

            let mut shadows: f32 = 0.0;
            ui.horizontal(|ui| {
                ui.label("Shadows:");
                ui.add(egui::Slider::new(&mut shadows, -100.0..=100.0));
            });

            let mut whites: f32 = 0.0;
            ui.horizontal(|ui| {
                ui.label("Whites:");
                ui.add(egui::Slider::new(&mut whites, -100.0..=100.0));
            });

            let mut blacks: f32 = 0.0;
            ui.horizontal(|ui| {
                ui.label("Blacks:");
                ui.add(egui::Slider::new(&mut blacks, -100.0..=100.0));
            });

            let mut saturation: f32 = 100.0;
            ui.horizontal(|ui| {
                ui.label("Saturation:");
                ui.add(egui::Slider::new(&mut saturation, 0.0..=200.0).suffix("%"));
            });
        });

        // --- 2. Creative Look & LUTs ---
        ui.collapsing("Creative", |ui| {
            ui.label(egui::RichText::new("Look Presets").small());
            let mut faded_film: f32 = 0.0;
            ui.horizontal(|ui| {
                ui.label("Faded Film:");
                ui.add(egui::Slider::new(&mut faded_film, 0.0..=100.0));
            });

            let mut sharpen: f32 = 0.0;
            ui.horizontal(|ui| {
                ui.label("Sharpen:");
                ui.add(egui::Slider::new(&mut sharpen, 0.0..=100.0));
            });

            let mut vibrance: f32 = 0.0;
            ui.horizontal(|ui| {
                ui.label("Vibrance:");
                ui.add(egui::Slider::new(&mut vibrance, -100.0..=100.0));
            });
        });

        // --- 3. Curves ---
        ui.collapsing("Curves (RGB & Hue)", |ui| {
            ui.label("RGB Master Curves Spline");
            let (rect, _response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 120.0), egui::Sense::hover());
            ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(25, 25, 25));
            ui.painter().rect_stroke(rect, 4.0, (1.0, egui::Color32::from_rgb(60, 60, 60)));

            // Draw diagonal reference line
            ui.painter().line_segment(
                [rect.left_bottom(), rect.right_top()],
                (1.0, egui::Color32::from_rgb(100, 100, 100))
            );
        });

        // --- 4. Color Wheels & Match ---
        ui.collapsing("Color Wheels & Match", |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Shadows").small());
                    let (r, _) = ui.allocate_exact_size(egui::vec2(60.0, 60.0), egui::Sense::hover());
                    ui.painter().circle_stroke(r.center(), 25.0, (1.5, egui::Color32::from_rgb(200, 200, 200)));
                });
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Midtones").small());
                    let (r, _) = ui.allocate_exact_size(egui::vec2(60.0, 60.0), egui::Sense::hover());
                    ui.painter().circle_stroke(r.center(), 25.0, (1.5, egui::Color32::from_rgb(200, 200, 200)));
                });
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Highlights").small());
                    let (r, _) = ui.allocate_exact_size(egui::vec2(60.0, 60.0), egui::Sense::hover());
                    ui.painter().circle_stroke(r.center(), 25.0, (1.5, egui::Color32::from_rgb(200, 200, 200)));
                });
            });
        });

        // --- 5. Vignette ---
        ui.collapsing("Vignette", |ui| {
            let mut amount: f32 = 0.0;
            ui.horizontal(|ui| {
                ui.label("Amount:");
                ui.add(egui::Slider::new(&mut amount, -5.0..=5.0));
            });
        });
    });
}
