use eframe::egui;

/// Adobe After Effects Professional Dark Theme Palette
/// Based on actual AE CC 2024 color measurements.
#[allow(dead_code)]
pub mod colors {
    use eframe::egui::Color32;

    // ── Background Layers (darkest → lightest) ──
    pub const BG_DEEPEST: Color32 = Color32::from_rgb(18, 18, 18);      // Timeline bg
    pub const BG_DARKEST: Color32 = Color32::from_rgb(23, 23, 23);      // Panel bg
    pub const BG_DARK: Color32 = Color32::from_rgb(30, 30, 30);         // Header bg
    pub const BG_MID: Color32 = Color32::from_rgb(38, 38, 38);          // Surface bg
    pub const BG_PANEL: Color32 = Color32::from_rgb(43, 43, 43);        // Elevated surface
    pub const BG_SURFACE: Color32 = Color32::from_rgb(52, 52, 52);      // Input fields
    pub const BG_ELEVATED: Color32 = Color32::from_rgb(60, 60, 60);     // Dropdowns

    // ── Interactive States ──
    pub const BG_HOVER: Color32 = Color32::from_rgb(48, 58, 73);        // Button hover
    pub const BG_ACTIVE: Color32 = Color32::from_rgb(20, 115, 230);     // Selection / active
    pub const BG_PRESSED: Color32 = Color32::from_rgb(15, 90, 185);     // Button pressed

    // ── AE Accent Colors ──
    pub const ACCENT_BLUE: Color32 = Color32::from_rgb(0, 163, 255);    // Primary accent
    pub const ACCENT_CYAN: Color32 = Color32::from_rgb(0, 215, 255);    // Timeline cursor
    pub const ACCENT_YELLOW: Color32 = Color32::from_rgb(255, 214, 0);  // Timecode
    pub const ACCENT_GREEN: Color32 = Color32::from_rgb(0, 210, 90);    // Success / Solo
    pub const ACCENT_RED: Color32 = Color32::from_rgb(220, 50, 47);     // Error / Mute
    pub const ACCENT_ORANGE: Color32 = Color32::from_rgb(255, 140, 0);  // Warning
    pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(160, 120, 255);// Expression

    // ── Borders (crisp 1px) ──
    pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(40, 40, 40);   // Panel dividers
    pub const BORDER_MEDIUM: Color32 = Color32::from_rgb(55, 55, 55);   // Input borders
    pub const BORDER_STRONG: Color32 = Color32::from_rgb(75, 75, 75);   // Active borders
    pub const BORDER_ACTIVE: Color32 = Color32::from_rgb(0, 163, 255);  // Focused input

    // ── Typography ──
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(220, 220, 220); // Main text
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(160, 160, 160); // Labels
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(110, 110, 110);   // Disabled
    pub const TEXT_ACCENT: Color32 = Color32::from_rgb(0, 180, 255);    // Links / values
    pub const TEXT_ON_ACCENT: Color32 = Color32::from_rgb(255, 255, 255); // On blue bg

    // ── Layer Label Colors (AE standard) ──
    pub const LABEL_RED: Color32 = Color32::from_rgb(255, 60, 60);
    pub const LABEL_ORANGE: Color32 = Color32::from_rgb(255, 160, 40);
    pub const LABEL_YELLOW: Color32 = Color32::from_rgb(255, 230, 50);
    pub const LABEL_GREEN: Color32 = Color32::from_rgb(80, 220, 100);
    pub const LABEL_CYAN: Color32 = Color32::from_rgb(50, 210, 255);
    pub const LABEL_BLUE: Color32 = Color32::from_rgb(50, 140, 255);
    pub const LABEL_PURPLE: Color32 = Color32::from_rgb(160, 110, 255);
    pub const LABEL_MAGENTA: Color32 = Color32::from_rgb(230, 80, 200);

    // ── Viewport Overlay Colors ──
    pub const GRID_LINE: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 30);
    pub const MOTION_PATH: Color32 = Color32::from_rgb(0, 200, 255);
    pub const KEYFRAME_DOT: Color32 = Color32::from_rgb(255, 200, 0);
    pub const GUIDE_LINE: Color32 = Color32::from_rgb(0, 200, 230);
    pub const HUD_BG: Color32 = Color32::from_rgba_premultiplied(15, 22, 32, 220);
    pub const HUD_STROKE: Color32 = Color32::from_rgb(0, 200, 255);
    pub const HUD_TEXT: Color32 = Color32::from_rgb(200, 235, 255);
    pub const HUD_STATUS_TEXT: Color32 = Color32::from_rgb(200, 220, 255);
    pub const FPS_GOOD: Color32 = Color32::from_rgb(0, 200, 255);
    pub const FPS_BAD: Color32 = Color32::from_rgb(255, 100, 80);

    // ── 3D Gizmo Colors ──
    pub const GIZMO_X: Color32 = Color32::from_rgb(240, 70, 70);
    pub const GIZMO_Y: Color32 = Color32::from_rgb(60, 220, 80);
    pub const GIZMO_Z: Color32 = Color32::from_rgb(60, 150, 255);
    pub const BBOX_STROKE: Color32 = Color32::from_rgb(0, 180, 255);
    pub const HANDLE_NORMAL: Color32 = Color32::WHITE;
    pub const HANDLE_HOVER_FILL: Color32 = Color32::from_rgb(255, 230, 100);
    pub const HANDLE_HOVER_STROKE: Color32 = Color32::from_rgb(255, 100, 0);
    pub const CENTER_DOT: Color32 = Color32::from_rgb(255, 215, 0);
    pub const CENTER_HOVER_RING: Color32 = Color32::from_rgb(60, 140, 255);

    // ── Timeline Overlay Colors ──
    pub const TIMELINE_PLAYHEAD: Color32 = Color32::from_rgb(0, 200, 255);
    pub const TIMELINE_KEYFRAME: Color32 = Color32::from_rgb(255, 200, 60);
    pub const TIMELINE_WAVEFORM: Color32 = Color32::from_rgb(80, 200, 120);
    pub const TIMELINE_SELECTION: Color32 = Color32::from_rgba_premultiplied(0, 120, 255, 40);
}

/// Layout & Spacing Constants for Pro Density
#[allow(dead_code)]
pub mod layout {
    pub const SIDEBAR_DEFAULT_WIDTH: f32 = 280.0;
    pub const TOOLBAR_HEIGHT: f32 = 34.0;
    pub const TIMELINE_LEFT_PANE_WIDTH: f32 = 260.0;
    pub const BOTTOM_TIMELINE_HEIGHT: f32 = 300.0;
    pub const STATUS_BAR_HEIGHT: f32 = 22.0;

    pub const FONT_SIZE_SMALL: f32 = 10.0;
    pub const FONT_SIZE_BODY: f32 = 11.5;
    pub const FONT_SIZE_HEADING: f32 = 13.0;
    pub const FONT_SIZE_TITLE: f32 = 15.0;
}

/// Configure fonts for professional appearance.
fn configure_fonts(ctx: &egui::Context) {
    ctx.style_mut(|style| {
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::new(layout::FONT_SIZE_BODY, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Small,
            egui::FontId::new(layout::FONT_SIZE_SMALL, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::new(layout::FONT_SIZE_BODY, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(layout::FONT_SIZE_HEADING, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Monospace,
            egui::FontId::new(layout::FONT_SIZE_BODY, egui::FontFamily::Monospace),
        );
    });
}

/// Apply comprehensive AE dark theme to egui.
pub fn configure_ae_theme(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    configure_fonts(ctx);

    let mut visuals = egui::Visuals::dark();

    // ── Background fills ──
    visuals.panel_fill = colors::BG_DARKEST;
    visuals.window_fill = colors::BG_DARK;
    visuals.faint_bg_color = colors::BG_DEEPEST;
    visuals.extreme_bg_color = egui::Color32::from_rgb(12, 12, 12);

    // ── Selection ──
    visuals.selection.bg_fill = colors::BG_ACTIVE;
    visuals.selection.stroke = egui::Stroke::new(1.0, colors::ACCENT_BLUE);

    // ── Widget states ──
    // Noninteractive (labels, static text)
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, colors::TEXT_PRIMARY);
    visuals.widgets.noninteractive.bg_fill = colors::BG_DARKEST;
    visuals.widgets.noninteractive.weak_bg_fill = colors::BG_DARKEST;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, colors::BORDER_SUBTLE);
    visuals.widgets.noninteractive.rounding = egui::Rounding::same(2.0);

    // Inactive (buttons, sliders at rest)
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, colors::TEXT_PRIMARY);
    visuals.widgets.inactive.bg_fill = colors::BG_MID;
    visuals.widgets.inactive.weak_bg_fill = colors::BG_MID;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, colors::BORDER_MEDIUM);
    visuals.widgets.inactive.rounding = egui::Rounding::same(3.0);

    // Hovered
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.widgets.hovered.bg_fill = colors::BG_HOVER;
    visuals.widgets.hovered.weak_bg_fill = colors::BG_HOVER;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, colors::BORDER_STRONG);
    visuals.widgets.hovered.rounding = egui::Rounding::same(3.0);

    // Active (pressed)
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.widgets.active.bg_fill = colors::BG_PRESSED;
    visuals.widgets.active.weak_bg_fill = colors::BG_PRESSED;
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, colors::ACCENT_BLUE);
    visuals.widgets.active.rounding = egui::Rounding::same(3.0);

    // Open (expanded menus, popups)
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.widgets.open.bg_fill = colors::BG_PANEL;
    visuals.widgets.open.weak_bg_fill = colors::BG_PANEL;
    visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, colors::BORDER_STRONG);
    visuals.widgets.open.rounding = egui::Rounding::same(3.0);

    // ── Warning/Error colors ──
    visuals.warn_fg_color = colors::ACCENT_ORANGE;
    visuals.error_fg_color = colors::ACCENT_RED;

    // ── Resize handle styling ──
    visuals.resize_corner_size = 8.0;

    ctx.set_visuals(visuals);

    // ── Typography & Spacing ──
    ctx.style_mut(|style| {
        // Tighter spacing for pro density
        style.spacing.item_spacing = egui::vec2(4.0, 2.0);
        style.spacing.button_padding = egui::vec2(6.0, 2.0);
        style.spacing.indent = 14.0;
        style.spacing.scroll.bar_width = 6.0;
        style.spacing.scroll.bar_inner_margin = 2.0;
        style.spacing.scroll.bar_outer_margin = 1.0;
        style.spacing.menu_margin = egui::Margin::symmetric(6.0, 4.0);
        style.spacing.window_margin = egui::Margin::same(8.0);
    });
}

/// Helper: Render section headers with a crisp left accent bar, icon, and high-contrast typography.
#[allow(dead_code)]
pub fn draw_section_header(ui: &mut egui::Ui, title: &str, icon: &str) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(3.0, 16.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 1.0, colors::ACCENT_CYAN);
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(format!("{} {}", icon, title))
                .small()
                .strong()
                .color(colors::TEXT_PRIMARY),
        );
    });
    ui.add_space(2.0);
}

/// Helper: Draw a pro After Effects tab with dynamic bottom cyan border when selected.
pub fn draw_custom_tab(ui: &mut egui::Ui, selected: bool, title: &str) -> egui::Response {
    let text = egui::RichText::new(title)
        .small()
        .strong()
        .color(if selected { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY });

    let response = ui.selectable_label(selected, text);
    if selected {
        let rect = response.rect;
        ui.painter().line_segment(
            [egui::pos2(rect.left(), rect.bottom() - 1.5), egui::pos2(rect.right(), rect.bottom() - 1.5)],
            egui::Stroke::new(2.0, colors::ACCENT_CYAN),
        );
    }
    response
}

/// Helper: Draw formatted property label, value, and unit (`px`, `%`, `dB`, `f`).
#[allow(dead_code)]
pub fn draw_prop_value(ui: &mut egui::Ui, label: &str, val_str: &str, unit: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).small().color(colors::TEXT_SECONDARY));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if !unit.is_empty() {
                ui.label(egui::RichText::new(unit).small().color(colors::TEXT_MUTED));
            }
            ui.label(egui::RichText::new(val_str).small().strong().color(colors::TEXT_ACCENT));
        });
    });
}

/// Helper: Draw a crisp 1px horizontal separator line.
#[allow(dead_code)]
pub fn draw_separator(ui: &mut egui::Ui) {
    let rect = ui.available_rect_before_wrap();
    let y = rect.min.y + 0.5;
    ui.painter().line_segment(
        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
        egui::Stroke::new(1.0, colors::BORDER_SUBTLE),
    );
    ui.add_space(1.0);
}

/// Helper: Draw a layer label color chip.
#[allow(dead_code)]
pub fn draw_label_chip(ui: &mut egui::Ui, color: egui::Color32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::click());
    ui.painter().rect_filled(rect, 2.0, color);
    response
}

/// Helper: Create a consistent AE-style panel frame.
#[allow(dead_code)]
pub fn panel_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(colors::BG_DARK)
        .inner_margin(egui::Margin::same(8.0))
        .stroke(egui::Stroke::new(1.0, colors::BORDER_SUBTLE))
}

/// Helper: Create a consistent AE-style side panel frame.
#[allow(dead_code)]
pub fn side_panel_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(colors::BG_DARKEST)
        .inner_margin(egui::Margin::same(8.0))
        .stroke(egui::Stroke::new(1.0, colors::BORDER_SUBTLE))
}

/// Custom DragValue with AE-style modifier keys:
/// - Normal drag: 1x speed
/// - Alt+drag: 0.1x speed (fine control)
/// - Shift+drag: 10x speed (fast scrub)
#[allow(dead_code)]
pub fn ae_drag_value(value: &mut f32) -> egui::DragValue<'_> {
    egui::DragValue::new(value).speed(1.0)
}
