use eframe::egui;

pub const SVG_EYE_OPEN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>"#;
pub const SVG_EYE_CLOSED: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="gray" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"/><line x1="1" y1="1" x2="23" y2="23"/></svg>"#;

pub const SVG_LOCK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>"#;
pub const SVG_UNLOCK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="gray" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 9.9-1"/></svg>"#;

#[allow(dead_code)]
pub const SVG_PLAY: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="white"><polygon points="5 3 19 12 5 21 5 3"/></svg>"#;
#[allow(dead_code)]
pub const SVG_PAUSE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="white"><rect x="6" y="4" width="4" height="16"/><rect x="14" y="4" width="4" height="16"/></svg>"#;
#[allow(dead_code)]
pub const SVG_MARKER: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="orange"><path d="M4 15s1-1 4-1 5 2 8 2 4-1 4-1V3s-1 1-4 1-5-2-8-2-4 1-4 1z"/><line x1="4" y1="22" x2="4" y2="15" stroke="orange" stroke-width="2"/></svg>"#;
#[allow(dead_code)]
pub const SVG_KEYFRAME: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="cyan"><polygon points="12 2 22 12 12 22 2 12 12 2"/></svg>"#;
#[allow(dead_code)]
pub const SVG_AUDIO: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="deepskyblue" stroke-width="2"><path d="M11 5L6 9H2v6h4l5 4V5z"/><path d="M19.07 4.93a10 10 0 0 1 0 14.14M15.54 8.46a5 5 0 0 1 0 7.07"/></svg>"#;
#[allow(dead_code)]
pub const SVG_GPU: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="limegreen" stroke-width="2"><rect x="4" y="4" width="16" height="16" rx="2"/><rect x="9" y="9" width="6" height="6"/><line x1="9" y1="1" x2="9" y2="4"/><line x1="15" y1="1" x2="15" y2="4"/><line x1="9" y1="20" x2="9" y2="23"/><line x1="15" y1="20" x2="15" y2="23"/><line x1="20" y1="9" x2="23" y2="9"/><line x1="20" y1="15" x2="23" y2="15"/><line x1="1" y1="9" x2="4" y2="9"/><line x1="1" y1="15" x2="4" y2="15"/></svg>"#;

// ── AE Tool SVG Icons ──
#[allow(dead_code)]
pub const SVG_TOOL_SELECT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="white"><path d="M3 3l7 18 3-7 7-3L3 3z"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_HAND: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M18 11V6a2 2 0 0 0-4 0v5M14 10V4a2 2 0 0 0-4 0v6M10 10.5V2.5a2 2 0 0 0-4 0v11M6 14v-1.5a2 2 0 0 0-4 0V16a8 8 0 0 0 16 0v-5a2 2 0 0 0-4 0"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_ZOOM: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/><line x1="11" y1="8" x2="11" y2="14"/><line x1="8" y1="11" x2="14" y2="11"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_CAMERA: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M23 19a2 2 0 0 1-2 2H3a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4l2-3h6l2 3h4a2 2 0 0 1 2 2z"/><circle cx="12" cy="13" r="4"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_ROTATE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M21.5 2v6h-6M21.34 15.57a10 10 0 1 1-.57-8.38l5.67-5.67"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_SHAPE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><rect x="3" y="3" width="18" height="18" rx="2"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_PEN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M12 19l7-7 3 3-7 7-3-3z"/><path d="M18 13l-1.5-7.5L2 2l3.5 14.5L13 18l5-5z"/><path d="M2 2l7.5 7.5"/><circle cx="11" cy="11" r="2"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_TEXT: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="white"><path d="M5 4h14v3h-5.5v13h-3V7H5V4z"/></svg>"#;

// ── AE Switch SVG Icons ──
#[allow(dead_code)]
pub const SVG_SWITCH_3D: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="deepskyblue" stroke-width="2"><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"/><polyline points="3.27 6.96 12 12.01 20.73 6.96"/><line x1="12" y1="22.08" x2="12" y2="12"/></svg>"#;
#[allow(dead_code)]
pub const SVG_SWITCH_SOLO: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="gold" stroke-width="2"><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="4"/><line x1="12" y1="2" x2="12" y2="4"/><line x1="12" y1="20" x2="12" y2="22"/><line x1="2" y1="12" x2="4" y2="12"/><line x1="20" y1="12" x2="22" y2="12"/></svg>"#;
#[allow(dead_code)]
pub const SVG_SWITCH_GRAPH: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="cyan" stroke-width="2"><path d="M3 3v18h18"/><path d="M19 9l-5 5-4-4-3 3"/></svg>"#;

/// Setup egui_extras image loaders (supports SVG, PNG, GIF, etc.)
pub fn init_image_loaders(ctx: &egui::Context) {
    egui_extras::install_image_loaders(ctx);
}

/// Render an SVG string directly as an egui Image widget.
/// SVG bytes are borrowed statically; only the cache URI is allocated per call.
pub fn render_svg_bytes(ui: &mut egui::Ui, name: &str, svg_str: &'static str, size: egui::Vec2, tint: egui::Color32) -> egui::Response {
    ui.add(
        egui::Image::new(egui::ImageSource::Bytes {
            uri: std::borrow::Cow::Owned(name.to_string()),
            bytes: egui::load::Bytes::Static(svg_str.as_bytes()),
        })
        .fit_to_exact_size(size)
        .tint(tint)
    )
}


// ── Additional Tool Icons ──
#[allow(dead_code)]
pub const SVG_TOOL_ANCHOR: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><circle cx="12" cy="5" r="2"/><circle cx="5" cy="19" r="2"/><circle cx="19" cy="19" r="2"/><path d="M12 7v4M12 11L6 17M12 11l6 6"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_BRUSH: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M9.06 11.9l8.07-8.06a2.85 2.85 0 1 1 4.03 4.03l-8.06 8.08"/><path d="M7.07 14.94c-1.66 0-3 1.35-3 3.02 0 1.33-2.5 1.52-2 2.02 1.08 1.1 2.49 2.02 4 2.02 2.2 0 4-1.8 4-4.04a3.01 3.01 0 0 0-3-3.02z"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_STAMP: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M5 22h14"/><path d="M19.27 13.73A2.5 2.5 0 0 0 17.5 13h-11A2.5 2.5 0 0 0 4 15.5V17a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-1.5c0-.66-.26-1.3-.73-1.77z" transform="translate(0 -1)"/><path d="M14 13V8.5C14 7 15 7 15 5a3 3 0 0 0-6 0c0 2 1 2 1 3.5V13"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_ERASER: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M20 20H7L3 16a1.41 1.41 0 0 1 0-2L13 4a1.41 1.41 0 0 1 2 0l6 6a1.41 1.41 0 0 1 0 2l-9 9"/><line x1="8.5" y1="8.5" x2="15.5" y2="15.5"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_ROTO: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><circle cx="6" cy="6" r="3"/><circle cx="18" cy="6" r="3"/><circle cx="12" cy="18" r="3"/><path d="M8.5 7.5L10.5 15M15.5 7.5L13.5 15M9 6h6"/></svg>"#;
#[allow(dead_code)]
pub const SVG_TOOL_PUPPET: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><circle cx="12" cy="4" r="2" fill="white"/><path d="M12 6v6M12 12l-5 6M12 12l5 6M6 20h12"/></svg>"#;
#[allow(dead_code)]
pub const SVG_RENDER_QUEUE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><rect x="2" y="4" width="20" height="16" rx="2"/><polygon points="10 8 16 12 10 16 10 8" fill="white" stroke="none"/></svg>"#;
#[allow(dead_code)]
pub const SVG_SNAP: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="2"><path d="M3 3v18M21 3v18"/><path d="M7 12h10"/><path d="M7 12l3-3M7 12l3 3M17 12l-3-3M17 12l-3 3"/></svg>"#;

/// The application logo mark: a stylized composition frame with a playhead.
/// Drawn procedurally so it stays crisp at any size and adapts to the theme.
pub fn draw_logo(ui: &mut egui::Ui, size: f32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let p = ui.painter();
    let accent = egui::Color32::from_rgb(0, 160, 240);
    let purple = egui::Color32::from_rgb(150, 80, 220);

    // Rounded frame
    p.rect_stroke(rect.shrink(1.0), 3.0, egui::Stroke::new(1.6, accent));
    // Diagonal split suggesting motion
    let tl = rect.left_top();
    let br = rect.right_bottom();
    let mid = egui::pos2(rect.center().x, rect.center().y);
    p.line_segment([tl, mid], egui::Stroke::new(1.2, purple));
    p.line_segment([mid, br], egui::Stroke::new(1.2, purple));
    // Playhead diamond at center
    let c = rect.center();
    let s = size * 0.14;
    let pts = [
        egui::pos2(c.x, c.y - s),
        egui::pos2(c.x + s, c.y),
        egui::pos2(c.x, c.y + s),
        egui::pos2(c.x - s, c.y),
    ];
    p.add(egui::Shape::convex_polygon(pts.to_vec(), accent, egui::Stroke::NONE));
    resp
}

/// Renders an SVG at an arbitrary position (used by icon-button painting).
pub fn render_svg_at(
    ui: &mut egui::Ui,
    name: String,
    svg_str: &'static str,
    size: egui::Vec2,
    tint: egui::Color32,
    pos: egui::Pos2,
) -> egui::Response {
    ui.allocate_exact_size(size, egui::Sense::hover());
    ui.put(
        egui::Rect::from_min_size(pos, size),
        egui::Image::new(egui::ImageSource::Bytes {
            uri: std::borrow::Cow::Owned(name),
            bytes: egui::load::Bytes::Static(svg_str.as_bytes()),
        })
        .fit_to_exact_size(size)
        .tint(tint),
    )
}

// ── Phosphor glyph helpers (registered via theme::configure_fonts) ──

/// Crisp icon glyph for a layer type, used in timeline rows & panels.
pub fn layer_icon(lt: &crate::core::timeline::LayerType) -> &'static str {
    use crate::core::timeline::LayerType;
    use egui_phosphor::regular as p;
    match lt {
        LayerType::Video { .. } => p::FILM_STRIP,
        LayerType::Image { .. } => p::IMAGE,
        LayerType::Audio { .. } => p::WAVEFORM,
        LayerType::Text { .. } => p::TEXT_T,
        LayerType::Shape { .. } => p::POLYGON,
        LayerType::Solid { .. } => p::SQUARE,
        LayerType::Null => p::CIRCLE,
        LayerType::PreComp { .. } => p::PACKAGE,
        LayerType::AdjustmentLayer => p::CIRCLE_HALF,
        LayerType::Particle { .. } => p::SPARKLE,
    }
}
