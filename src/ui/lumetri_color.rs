use eframe::egui;
use crate::AfterEffectsApp;

pub fn draw_lumetri_color(_app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("Lumetri Color");
    ui.separator();

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
