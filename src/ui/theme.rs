use eframe::egui;

/// 🎨 Adobe After Effects / Premiere Professional Dark Theme Palette
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

/// Apply global Adobe After Effects high-contrast pro dark theme to egui.
pub fn configure_ae_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    // Base background colors
    visuals.panel_fill = colors::BG_PANEL;
    visuals.window_fill = colors::BG_PANEL;
    visuals.faint_bg_color = colors::BG_DARKEST;
    visuals.extreme_bg_color = colors::BG_DARKEST;

    // Selection highlight
    visuals.selection.bg_fill = colors::BG_ACTIVE;
    visuals.selection.stroke = egui::Stroke::new(1.0, colors::ACCENT_CYAN);

    // Non-interactive widgets (Labels, Dividers, Cards)
    visuals.widgets.noninteractive.bg_fill = colors::BG_SURFACE;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, colors::BORDER_SUBTLE);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, colors::TEXT_PRIMARY);
    visuals.widgets.noninteractive.rounding = egui::Rounding::same(3.0);

    // Inactive interactive widgets (Buttons, Dropdowns)
    visuals.widgets.inactive.bg_fill = colors::BG_HEADER;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, colors::BORDER_MEDIUM);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, colors::TEXT_PRIMARY);
    visuals.widgets.inactive.rounding = egui::Rounding::same(3.0);

    // Hovered widgets
    visuals.widgets.hovered.bg_fill = colors::BG_HOVER;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.5, colors::ACCENT_CYAN);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.widgets.hovered.rounding = egui::Rounding::same(3.0);

    // Active (Clicked/Dragged) widgets
    visuals.widgets.active.bg_fill = colors::BG_ACTIVE;
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.5, colors::ACCENT_CYAN);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.widgets.active.rounding = egui::Rounding::same(3.0);

    // Open/Expanded popups
    visuals.widgets.open.bg_fill = colors::BG_SURFACE_ELEVATED;
    visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, colors::BORDER_ACTIVE);
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, colors::TEXT_PRIMARY);
    visuals.widgets.open.rounding = egui::Rounding::same(3.0);

    // Window shadow & borders
    visuals.window_stroke = egui::Stroke::new(1.0, colors::BORDER_STRONG);
    visuals.window_rounding = egui::Rounding::same(4.0);

    ctx.set_visuals(visuals);

    // Adjust global spacing and padding for pro ergonomics
    ctx.style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(6.0, 5.0);
        style.spacing.button_padding = egui::vec2(7.0, 4.0);
        style.spacing.indent = 14.0;
        style.spacing.scroll.bar_width = 8.0;
    });
}

/// Helper: Render section headers with a crisp left accent bar, icon, and high-contrast typography.
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
