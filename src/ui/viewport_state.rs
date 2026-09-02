use eframe::egui;

pub const SNAP_FRAME_ID: &str = "ae_viewport_snap_a";
pub const IS_COMPARING_ID: &str = "ae_viewport_comparing";
pub const WIPE_POS_ID: &str = "ae_viewport_wipe_pos";

pub fn snap_frame(ctx: &egui::Context) -> Option<u32> {
    ctx.data(|d| d.get_temp(egui::Id::new(SNAP_FRAME_ID)))
}

pub fn set_snap_frame(ctx: &egui::Context, frame: u32) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(SNAP_FRAME_ID), frame));
}

pub fn is_comparing(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp(egui::Id::new(IS_COMPARING_ID)).unwrap_or(false))
}

pub fn set_comparing(ctx: &egui::Context, comparing: bool) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(IS_COMPARING_ID), comparing));
}

pub fn toggle_comparing(ctx: &egui::Context) -> bool {
    let next = !is_comparing(ctx);
    set_comparing(ctx, next);
    next
}

pub fn wipe_pos(ctx: &egui::Context) -> f32 {
    ctx.data(|d| d.get_temp(egui::Id::new(WIPE_POS_ID)).unwrap_or(0.5))
}

pub fn set_wipe_pos(ctx: &egui::Context, pos: f32) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new(WIPE_POS_ID), pos.clamp(0.0, 1.0)));
}

/// Fit composition into viewport rect, then apply AE magnification ratio (0 = Fit).
/// Like compute_draw_layout but with a pan offset applied after centering.
pub fn compute_draw_layout_pan(
    rect: egui::Rect,
    aspect: f32,
    mag_ratio: f32,
    pan: egui::Vec2,
) -> (f32, f32, f32, f32) {
    let (origin_x, origin_y, draw_w, draw_h) = compute_draw_layout(rect, aspect, mag_ratio);
    (origin_x + pan.x, origin_y + pan.y, draw_w, draw_h)
}

pub fn compute_draw_layout(rect: egui::Rect, aspect: f32, mag_ratio: f32) -> (f32, f32, f32, f32) {
    let safe_aspect = if aspect.is_nan() || aspect <= 0.001 {
        1.0
    } else {
        aspect
    };
    let mut fit_w = rect.width();
    let mut fit_h = fit_w / safe_aspect;
    if fit_h > rect.height() {
        fit_h = rect.height();
        fit_w = fit_h * safe_aspect;
    }

    let (draw_w, draw_h) = if mag_ratio <= 0.0 {
        (fit_w, fit_h)
    } else {
        (fit_w * mag_ratio, fit_h * mag_ratio)
    };

    let origin_x = rect.left() + (rect.width() - draw_w) * 0.5;
    let origin_y = rect.top() + (rect.height() - draw_h) * 0.5;
    (origin_x, origin_y, draw_w, draw_h)
}
