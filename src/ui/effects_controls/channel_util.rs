use crate::core::timeline::EffectType;
use eframe::egui;

pub fn draw(
    effect_type: &mut EffectType,
    ui: &mut egui::Ui,
    _current_frame: u32,
    project_changed: &mut bool,
    _next_frame: &mut Option<u32>,
) {
    match effect_type {
        EffectType::ChannelCombiner {
            from_channel,
            to_target,
            invert,
        } => {
            ui.label("🔀 Channel Combiner");
            ui.horizontal(|ui| {
                ui.label("From:");
                egui::ComboBox::from_id_salt("chan_comb_from")
                    .selected_text(format!("{:?}", from_channel))
                    .show_ui(ui, |ui| {
                        for f in [
                            crate::core::channel_combiner::ChannelCombinerFrom::Red,
                            crate::core::channel_combiner::ChannelCombinerFrom::Green,
                            crate::core::channel_combiner::ChannelCombinerFrom::Blue,
                            crate::core::channel_combiner::ChannelCombinerFrom::Alpha,
                            crate::core::channel_combiner::ChannelCombinerFrom::Luminance,
                            crate::core::channel_combiner::ChannelCombinerFrom::Hue,
                            crate::core::channel_combiner::ChannelCombinerFrom::Lightness,
                            crate::core::channel_combiner::ChannelCombinerFrom::Saturation,
                            crate::core::channel_combiner::ChannelCombinerFrom::MinRGB,
                            crate::core::channel_combiner::ChannelCombinerFrom::MaxRGB,
                        ] {
                            if ui
                                .selectable_value(from_channel, f, format!("{:?}", f))
                                .changed()
                            {
                                *project_changed = true;
                            }
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label("To:");
                egui::ComboBox::from_id_salt("chan_comb_to")
                    .selected_text(format!("{:?}", to_target))
                    .show_ui(ui, |ui| {
                        for t in [
                            crate::core::channel_combiner::ChannelCombinerTo::Red,
                            crate::core::channel_combiner::ChannelCombinerTo::Green,
                            crate::core::channel_combiner::ChannelCombinerTo::Blue,
                            crate::core::channel_combiner::ChannelCombinerTo::Alpha,
                            crate::core::channel_combiner::ChannelCombinerTo::RGBOnly,
                            crate::core::channel_combiner::ChannelCombinerTo::RGBA,
                            crate::core::channel_combiner::ChannelCombinerTo::Lightness,
                        ] {
                            if ui
                                .selectable_value(to_target, t, format!("{:?}", t))
                                .changed()
                            {
                                *project_changed = true;
                            }
                        }
                    });
            });
            if ui.checkbox(invert, "Invert").changed() {
                *project_changed = true;
            }
        }
        _ => {}
    }
}
