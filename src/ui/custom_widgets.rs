use eframe::egui;
use crate::ui::theme::colors;

/// Animated hover state for smooth transitions
pub struct HoverAnim {
    pub progress: f32,
    pub target: f32,
}

impl HoverAnim {
    pub fn new() -> Self {
        Self { progress: 0.0, target: 0.0 }
    }

    pub fn update(&mut self, dt: f32, is_hovered: bool) {
        self.target = if is_hovered { 1.0 } else { 0.0 };
        let speed = 8.0;
        self.progress += (self.target - self.progress) * speed * dt;
        if (self.progress - self.target).abs() < 0.01 {
            self.progress = self.target;
        }
    }
}

/// Professional AE-style button with subtle hover gradient
pub fn ae_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let text = egui::RichText::new(label)
        .small()
        .color(colors::TEXT_PRIMARY);

    let button = egui::Button::new(text)
        .fill(colors::BG_MID)
        .stroke(egui::Stroke::new(1.0, colors::BORDER_MEDIUM))
        .rounding(egui::Rounding::same(3.0))
        .min_size(egui::vec2(60.0, 22.0));

    let response = ui.add(button);

    if response.hovered() {
        let rect = response.rect;
        ui.painter().rect_filled(rect, 3.0, colors::BG_HOVER);
        ui.painter().rect_stroke(rect, 3.0, egui::Stroke::new(1.0, colors::BORDER_STRONG));
    }

    response
}

/// Accent button (blue background)
pub fn ae_button_accent(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let text = egui::RichText::new(label)
        .small()
        .strong()
        .color(colors::TEXT_ON_ACCENT);

    let button = egui::Button::new(text)
        .fill(colors::BG_ACTIVE)
        .stroke(egui::Stroke::new(1.0, colors::ACCENT_BLUE))
        .rounding(egui::Rounding::same(3.0))
        .min_size(egui::vec2(60.0, 22.0));

    let response = ui.add(button);

    if response.hovered() {
        let rect = response.rect;
        ui.painter().rect_filled(rect, 3.0, colors::BG_PRESSED);
    }

    response
}

/// Small icon button (for toolbars)
pub fn ae_icon_button(ui: &mut egui::Ui, icon: &str, tooltip: &str) -> egui::Response {
    let text = egui::RichText::new(icon)
        .small()
        .color(colors::TEXT_SECONDARY);

    let button = egui::Button::new(text)
        .fill(egui::Color32::TRANSPARENT)
        .stroke(egui::Stroke::NONE)
        .rounding(egui::Rounding::same(2.0))
        .min_size(egui::vec2(24.0, 24.0));

    let response = ui.add(button).on_hover_text(tooltip);

    if response.hovered() {
        let rect = response.rect;
        ui.painter().rect_filled(rect, 2.0, colors::BG_HOVER);
    }

    response
}

/// AE-style toggle switch
pub fn ae_toggle(ui: &mut egui::Ui, value: &mut bool, label: &str) -> egui::Response {
    ui.horizontal(|ui| {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(32.0, 16.0),
            egui::Sense::click(),
        );

        // Track
        let track_color = if *value { colors::ACCENT_BLUE } else { colors::BG_SURFACE };
        ui.painter().rect_filled(rect, 8.0, track_color);
        ui.painter().rect_stroke(rect, 8.0, egui::Stroke::new(1.0, colors::BORDER_MEDIUM));

        // Thumb
        let thumb_x = if *value { rect.right() - 10.0 } else { rect.left() + 2.0 };
        let thumb_rect = egui::Rect::from_center_size(
            egui::pos2(thumb_x, rect.center().y),
            egui::vec2(12.0, 12.0),
        );
        ui.painter().rect_filled(thumb_rect, 6.0, egui::Color32::WHITE);

        if response.clicked() {
            *value = !*value;
        }

        ui.add_space(4.0);
        ui.label(egui::RichText::new(label).small().color(colors::TEXT_SECONDARY));

        response
    }).inner
}

/// AE-style section header with accent bar
pub fn ae_section_header(ui: &mut egui::Ui, title: &str, icon: &str) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        // Accent bar
        let (bar_rect, _) = ui.allocate_exact_size(egui::vec2(3.0, 14.0), egui::Sense::hover());
        ui.painter().rect_filled(bar_rect, 1.0, colors::ACCENT_CYAN);

        ui.add_space(4.0);

        // Title
        ui.label(
            egui::RichText::new(format!("{} {}", icon, title))
                .small()
                .strong()
                .color(colors::TEXT_PRIMARY),
        );
    });
    ui.add_space(2.0);
}

/// AE-style property row with label and value
pub fn ae_property_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).small().color(colors::TEXT_SECONDARY));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(value).small().strong().color(colors::TEXT_ACCENT));
        });
    });
}

/// AE-style horizontal separator
pub fn ae_separator(ui: &mut egui::Ui) {
    let rect = ui.available_rect_before_wrap();
    let y = rect.min.y + 0.5;
    ui.painter().line_segment(
        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
        egui::Stroke::new(1.0, colors::BORDER_SUBTLE),
    );
    ui.add_space(1.0);
}

/// AE-style panel header with subtle gradient
pub fn ae_panel_header(ui: &mut egui::Ui, title: &str) {
    let rect = ui.available_rect_before_wrap();
    let header_rect = egui::Rect::from_min_size(
        rect.min,
        egui::vec2(rect.width(), 28.0),
    );

    // Subtle gradient background
    ui.painter().rect_filled(header_rect, 0.0, colors::BG_DARK);

    // Bottom border
    ui.painter().line_segment(
        [egui::pos2(header_rect.left(), header_rect.bottom()),
         egui::pos2(header_rect.right(), header_rect.bottom())],
        egui::Stroke::new(1.0, colors::BORDER_SUBTLE),
    );

    // Title
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new(title)
                .small()
                .strong()
                .color(colors::TEXT_PRIMARY),
        );
    });
    ui.add_space(6.0);
}

/// AE-style color swatch
pub fn ae_color_swatch(ui: &mut egui::Ui, color: egui::Color32, size: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(size, size),
        egui::Sense::click(),
    );

    // Outer border
    ui.painter().rect_stroke(rect, 2.0, egui::Stroke::new(1.0, colors::BORDER_MEDIUM));

    // Inner color
    let inner = rect.shrink(1.0);
    ui.painter().rect_filled(inner, 2.0, color);

    response
}

/// AE-style progress bar
pub fn ae_progress_bar(ui: &mut egui::Ui, progress: f32, height: f32) {
    let rect = ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), height),
        egui::Layout::left_to_right(egui::Align::Center),
        |_ui| {},
    ).response.rect;

    // Background
    ui.painter().rect_filled(rect, 2.0, colors::BG_SURFACE);

    // Fill
    let fill_width = rect.width() * progress.clamp(0.0, 1.0);
    let fill_rect = egui::Rect::from_min_size(
        rect.min,
        egui::vec2(fill_width, rect.height()),
    );
    ui.painter().rect_filled(fill_rect, 2.0, colors::ACCENT_BLUE);

    // Border
    ui.painter().rect_stroke(rect, 2.0, egui::Stroke::new(1.0, colors::BORDER_SUBTLE));
}

/// AE-style text input field
pub fn ae_text_field(ui: &mut egui::Ui, text: &mut String, placeholder: &str) -> egui::Response {
    let edit = egui::TextEdit::singleline(text)
        .hint_text(egui::RichText::new(placeholder).small().color(colors::TEXT_MUTED))
        .desired_width(ui.available_width())
        .margin(egui::Margin::symmetric(6.0, 4.0));

    ui.add(edit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hover_anim_new() {
        let anim = HoverAnim::new();
        assert_eq!(anim.progress, 0.0);
        assert_eq!(anim.target, 0.0);
    }

    #[test]
    fn test_hover_anim_update() {
        let mut anim = HoverAnim::new();
        anim.update(0.1, true);
        assert!(anim.progress > 0.0);
        assert!(anim.progress <= 1.0);
    }

    #[test]
    fn test_hover_anim_settle() {
        let mut anim = HoverAnim::new();
        for _ in 0..100 {
            anim.update(0.016, true);
        }
        assert!((anim.progress - 1.0).abs() < 0.01);
    }
}
