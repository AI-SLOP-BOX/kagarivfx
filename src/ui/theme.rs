use eframe::egui;

/// Adobe After Effects / Premiere Professional Dark Theme Palette
#[allow(dead_code)]
pub mod colors {
    use eframe::egui::Color32;

    pub const BG_DARKEST: Color32 = Color32::from_rgb(18, 18, 18);
    pub const BG_PANEL: Color32 = Color32::from_rgb(26, 26, 26);
    pub const BG_HEADER: Color32 = Color32::from_rgb(34, 34, 34);
    pub const BG_SURFACE: Color32 = Color32::from_rgb(42, 42, 42);
    pub const BG_SURFACE_ELEVATED: Color32 = Color32::from_rgb(50, 50, 50);
    
    // Interactive states
    pub const BG_HOVER: Color32 = Color32::from_rgb(55, 65, 80);
    pub const BG_ACTIVE: Color32 = Color32::from_rgb(20, 115, 230); // AE Accent Blue
    
    // Accents
    pub const ACCENT_BLUE: Color32 = Color32::from_rgb(0, 163, 255);
    pub const ACCENT_CYAN: Color32 = Color32::from_rgb(0, 220, 255);
    pub const ACCENT_YELLOW: Color32 = Color32::from_rgb(255, 234, 0); // AE Timecode Yellow
    pub const ACCENT_ORANGE: Color32 = Color32::from_rgb(255, 140, 0);
    
    // Borders
    pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(38, 38, 38);
    pub const BORDER_MEDIUM: Color32 = Color32::from_rgb(58, 58, 58);
    pub const BORDER_STRONG: Color32 = Color32::from_rgb(80, 80, 80);
    pub const BORDER_ACTIVE: Color32 = Color32::from_rgb(0, 163, 255);

    // Typography
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(240, 240, 240);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(175, 175, 175);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(115, 115, 115);
    pub const TEXT_ACCENT: Color32 = Color32::from_rgb(100, 200, 255);
}

/// Layout & Spacing Constants for Consistent AE Ergonomics
#[allow(dead_code)]
pub mod layout {
    pub const SIDEBAR_DEFAULT_WIDTH: f32 = 280.0;
    pub const TOOLBAR_HEIGHT: f32 = 36.0;
    pub const TIMELINE_LEFT_PANE_WIDTH: f32 = 280.0;
    pub const BOTTOM_TIMELINE_HEIGHT: f32 = 320.0;
    pub const STATUS_BAR_HEIGHT: f32 = 24.0;

    pub const FONT_SIZE_SMALL: f32 = 10.0;
    pub const FONT_SIZE_BODY: f32 = 12.0;
    pub const FONT_SIZE_HEADING: f32 = 14.0;
    pub const FONT_SIZE_TITLE: f32 = 16.0;
}

/// Apply global Adobe After Effects high-contrast pro dark theme to egui.
pub fn configure_ae_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    // Dark Glassmorphism Base Colors
    visuals.panel_fill = egui::Color32::from_rgb(18, 21, 27);
    visuals.window_fill = egui::Color32::from_rgba_unmultiplied(18, 21, 27, 235);
    visuals.faint_bg_color = egui::Color32::from_rgb(12, 14, 18);
    visuals.extreme_bg_color = egui::Color32::from_rgb(10, 12, 16);

    // Selection highlight (Neon Blue Accent)
    visuals.selection.bg_fill = egui::Color32::from_rgb(0, 120, 215);
    visuals.selection.stroke = egui::Stroke::new(1.0, colors::ACCENT_BLUE);

    // Non-interactive widgets (Labels, Dividers, Cards)
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgba_unmultiplied(26, 32, 42, 200);
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 55, 70));
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, colors::TEXT_PRIMARY);
    visuals.widgets.noninteractive.rounding = egui::Rounding::same(6.0);

    // Inactive interactive widgets (Buttons, Dropdowns)
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgba_unmultiplied(32, 40, 52, 220);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(55, 68, 88));
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, colors::TEXT_PRIMARY);
    visuals.widgets.inactive.rounding = egui::Rounding::same(6.0);

    // Hovered widgets
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(42, 54, 72);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.5, colors::ACCENT_BLUE);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.widgets.hovered.rounding = egui::Rounding::same(6.0);

    // Active (Clicked/Dragged) widgets
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0, 140, 240);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.5, colors::ACCENT_CYAN);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.widgets.active.rounding = egui::Rounding::same(6.0);

    // Open/Expanded popups
    visuals.widgets.open.bg_fill = egui::Color32::from_rgba_unmultiplied(28, 35, 46, 240);
    visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, colors::ACCENT_BLUE);
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, colors::TEXT_PRIMARY);
    visuals.widgets.open.rounding = egui::Rounding::same(6.0);

    // Window shadow & glassmorphism borders
    visuals.window_stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(55, 70, 95));
    visuals.window_rounding = egui::Rounding::same(8.0);

    ctx.set_visuals(visuals);

    // Adjust global spacing and padding for pro ergonomics.
    // Tighter than egui defaults: pro NLE/VFX apps pack more information per
    // pixel, with small controls and dense rows.
    ctx.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(4.0, 3.0);
        style.spacing.button_padding = egui::vec2(5.0, 2.0);
        style.spacing.indent = 12.0;
        style.spacing.scroll.bar_width = 7.0;
        style.spacing.menu_margin = egui::Margin::symmetric(6.0, 3.0);
        style.spacing.window_margin = egui::Margin::same(8.0);
    });

    // Compact typography for panel content
    let mut fonts = egui::FontDefinitions::default();
    fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(1, "ubuntu-light".into());
    let _ = fonts; // family tweaks reserved; sizes applied via text styles below
    ctx.style_mut(|style| {
        use egui::{FontId, TextStyle};
        style.text_styles.insert(TextStyle::Body, FontId::proportional(13.0));
        style.text_styles.insert(TextStyle::Small, FontId::proportional(11.0));
        style.text_styles.insert(TextStyle::Button, FontId::proportional(13.0));
        style.text_styles.insert(TextStyle::Heading, FontId::proportional(17.0));
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
