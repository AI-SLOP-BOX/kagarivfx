use eframe::egui;
use crate::AfterEffectsApp;
use crate::ui::theme::colors;

pub fn draw_color_management(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    let comp = app.history.current_mut().active_composition_mut();
    let mut changed = false;

    crate::ui::custom_widgets::ae_section_header(ui, "Working Space", "🎨");

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Color Space").small().color(colors::TEXT_SECONDARY));
        egui::ComboBox::from_id_salt("color_space")
            .selected_text(match app.color_space_idx {
                0 => "Rec.709 sRGB",
                1 => "ACEScg",
                2 => "ACES2065-1",
                3 => "Display P3",
                _ => "Rec.709 sRGB",
            })
            .show_ui(ui, |ui| {
                if ui.selectable_value(&mut app.color_space_idx, 0, "Rec.709 Gamma 2.4 (sRGB)").clicked() { changed = true; }
                if ui.selectable_value(&mut app.color_space_idx, 1, "ACEScg (AP1 Linear)").clicked() { changed = true; }
                if ui.selectable_value(&mut app.color_space_idx, 2, "ACES2065-1 (AP0 Linear)").clicked() { changed = true; }
                if ui.selectable_value(&mut app.color_space_idx, 3, "Display P3").clicked() { changed = true; }
            });
    });

    ui.add_space(4.0);
    crate::ui::custom_widgets::ae_section_header(ui, "Bit Depth", "🔢");
    ui.horizontal(|ui| {
        let depth_labels = ["8-bpc", "16-bpc", "32-bpc Float"];
        for (i, label) in depth_labels.iter().enumerate() {
            let is_selected = app.bit_depth_idx == i;
            if ui.selectable_label(is_selected, egui::RichText::new(*label).small().color(
                if is_selected { colors::ACCENT_CYAN } else { colors::TEXT_PRIMARY }
            )).clicked() {
                app.bit_depth_idx = i;
                changed = true;
            }
        }
    });

    if app.bit_depth_idx == 2 {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("⚠").color(colors::ACCENT_ORANGE));
            ui.label(egui::RichText::new("32-bpc: Full float precision, higher memory usage").small().color(colors::TEXT_MUTED));
        });
    }

    ui.add_space(4.0);
    crate::ui::custom_widgets::ae_section_header(ui, "Display", "🖥");
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Simulation").small().color(colors::TEXT_SECONDARY));
        egui::ComboBox::from_id_salt("display_sim")
            .selected_text(match app.display_sim_idx {
                0 => "Mac sRGB",
                1 => "Rec.709 HDTV",
                2 => "DCI-P3 Cinema",
                _ => "Mac sRGB",
            })
            .show_ui(ui, |ui| {
                if ui.selectable_value(&mut app.display_sim_idx, 0, "Macintosh sRGB").clicked() { changed = true; }
                if ui.selectable_value(&mut app.display_sim_idx, 1, "Rec.709 HDTV").clicked() { changed = true; }
                if ui.selectable_value(&mut app.display_sim_idx, 2, "DCI-P3 Cinema").clicked() { changed = true; }
            });
    });

    ui.add_space(4.0);
    crate::ui::custom_widgets::ae_section_header(ui, "Background", "🖼");
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Color").small().color(colors::TEXT_SECONDARY));
        let c = &mut comp.background_color;
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

    ui.add_space(4.0);
    crate::ui::custom_widgets::ae_section_header(ui, "Motion Blur", "💨");
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Shutter Angle").small().color(colors::TEXT_SECONDARY));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new("°").small().color(colors::TEXT_MUTED));
            if ui.add(egui::DragValue::new(&mut comp.motion_blur_shutter_angle).speed(1.0).range(0.0..=720.0)).changed() {
                changed = true;
            }
        });
    });

    if changed {
        crate::core::frame_cache::bump_version();
    }
}
