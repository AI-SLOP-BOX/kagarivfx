use eframe::egui;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_title("After Effects OSS Alternative"),
        ..Default::default()
    };

    eframe::run_native(
        "After Effects OSS Alternative",
        options,
        Box::new(|cc| {
            let mut app = aftereffects_oss::AfterEffectsApp::default();

            let (frame_tx, frame_rx) = std::sync::mpsc::channel();
            let (conn_tx, conn_rx) = std::sync::mpsc::channel();
            if let Err(e) = aftereffects_oss::core::integration::start_sync_server(9000, frame_tx, conn_tx) {
                log::warn!("Dynamic Link sync server unavailable on port 9000: {}", e);
            }

            app.rx_frame = Some(frame_rx);
            app.rx_connection = Some(conn_rx);

            #[cfg(feature = "wgpu")]
            if let Some(wgpu_state) = &cc.wgpu_render_state {
                let renderer = aftereffects_oss::core::renderer::WgpuRenderer::new(
                    wgpu_state.device.clone(),
                    wgpu_state.queue.clone(),
                );
                app.renderer = Some(renderer);
                app.wgpu_state = Some(wgpu_state.clone());
            }

            aftereffects_oss::ui::theme::configure_ae_theme(&cc.egui_ctx);
            aftereffects_oss::ui::icons::init_image_loaders(&cc.egui_ctx);

            Ok(Box::new(app) as Box<dyn eframe::App>)
        }),
    )
}
