use crate::core::proxy::ProxyResolution;
use crate::ui::theme::colors;
use crate::AfterEffectsApp;
use eframe::egui;

/// Draw proxy controls in the timeline header or viewport overlay.
pub fn draw_proxy_controls(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    let (current, active_in_preview) = {
        let comp = app.history.current().active_composition();
        (
            comp.comp_proxy.global_resolution,
            comp.comp_proxy.active_in_preview,
        )
    };

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Proxy")
                .small()
                .color(colors::TEXT_PRIMARY),
        );

        let btn = |ui: &mut egui::Ui, label: &str, res: ProxyResolution| {
            let active = current == res;
            ui.selectable_label(
                active,
                egui::RichText::new(label).small().color(if active {
                    colors::ACCENT_CYAN
                } else {
                    colors::TEXT_PRIMARY
                }),
            )
        };

        if btn(ui, "Full", ProxyResolution::Full).clicked() {
            app.history
                .current_mut()
                .active_composition_mut()
                .comp_proxy
                .global_resolution = ProxyResolution::Full;
        }
        if btn(ui, "Half", ProxyResolution::Half).clicked() {
            app.history
                .current_mut()
                .active_composition_mut()
                .comp_proxy
                .global_resolution = ProxyResolution::Half;
        }
        if btn(ui, "¼", ProxyResolution::Quarter).clicked() {
            app.history
                .current_mut()
                .active_composition_mut()
                .comp_proxy
                .global_resolution = ProxyResolution::Quarter;
        }
        if btn(ui, "⅛", ProxyResolution::Eighth).clicked() {
            app.history
                .current_mut()
                .active_composition_mut()
                .comp_proxy
                .global_resolution = ProxyResolution::Eighth;
        }

        ui.separator();

        if ui
            .selectable_label(
                active_in_preview,
                egui::RichText::new(if active_in_preview {
                    "Proxy ON"
                } else {
                    "Proxy OFF"
                })
                .small()
                .color(if active_in_preview {
                    colors::ACCENT_CYAN
                } else {
                    colors::TEXT_MUTED
                }),
            )
            .clicked()
        {
            app.history
                .current_mut()
                .active_composition_mut()
                .comp_proxy
                .active_in_preview = !active_in_preview;
        }

        ui.separator();
        let mut mfr_enabled = ui.ctx().data(|d| {
            d.get_temp::<bool>(egui::Id::new("ae_mfr_enabled"))
                .unwrap_or(true)
        });
        if ui
            .selectable_label(mfr_enabled, "⚡ MFR")
            .on_hover_text("Multi-Frame Rendering (Rayon CPU Parallelism)")
            .clicked()
        {
            mfr_enabled = !mfr_enabled;
            ui.ctx()
                .data_mut(|d| d.insert_temp(egui::Id::new("ae_mfr_enabled"), mfr_enabled));
        }

        if ui
            .small_button("🗑 Purge RAM")
            .on_hover_text("Purge all RAM Preview Frame Cache")
            .clicked()
        {
            crate::core::frame_cache::bump_version();
            app.toasts.info("RAM Preview Cache purged");
        }
    });
}

/// Draw per-layer proxy toggle in the layer controls area.
pub fn draw_layer_proxy(app: &mut AfterEffectsApp, ui: &mut egui::Ui, layer_idx: usize) {
    let (enabled, res) = {
        let comp = app.history.current().active_composition();
        if layer_idx >= comp.layers.len() {
            return;
        }
        let proxy = &comp.layers[layer_idx].proxy;
        (proxy.enabled, proxy.resolution)
    };

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("Proxy")
                .small()
                .color(colors::TEXT_MUTED),
        );

        if ui.checkbox(&mut enabled.clone(), "").changed() {
            app.history.current_mut().active_composition_mut().layers[layer_idx]
                .proxy
                .enabled = !enabled;
        }

        if enabled {
            let btn = |ui: &mut egui::Ui, label: &str, r: ProxyResolution| {
                ui.selectable_label(res == r, egui::RichText::new(label).small())
            };
            if btn(ui, "½", ProxyResolution::Half).clicked() {
                app.history.current_mut().active_composition_mut().layers[layer_idx]
                    .proxy
                    .resolution = ProxyResolution::Half;
            }
            if btn(ui, "¼", ProxyResolution::Quarter).clicked() {
                app.history.current_mut().active_composition_mut().layers[layer_idx]
                    .proxy
                    .resolution = ProxyResolution::Quarter;
            }
            if btn(ui, "⅛", ProxyResolution::Eighth).clicked() {
                app.history.current_mut().active_composition_mut().layers[layer_idx]
                    .proxy
                    .resolution = ProxyResolution::Eighth;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_resolution_labels() {
        assert_eq!(ProxyResolution::Full.label(), "Full");
        assert_eq!(ProxyResolution::Half.label(), "Half");
        assert_eq!(ProxyResolution::Quarter.label(), "Quarter");
        assert_eq!(ProxyResolution::Eighth.label(), "Eighth");
    }

    #[test]
    fn test_proxy_resolution_factors() {
        assert_eq!(ProxyResolution::Full.factor(), 1.0);
        assert_eq!(ProxyResolution::Half.factor(), 0.5);
        assert_eq!(ProxyResolution::Quarter.factor(), 0.25);
        assert_eq!(ProxyResolution::Eighth.factor(), 0.125);
    }
}
