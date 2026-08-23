use eframe::egui;
use std::time::{Duration, Instant};
use crate::ui::theme::colors;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToastLevel {
    Info,
    Warning,
    Error,
}

pub fn toast_color(level: &ToastLevel) -> egui::Color32 {
    match level {
        ToastLevel::Info => colors::ACCENT_BLUE,
        ToastLevel::Warning => colors::ACCENT_ORANGE,
        ToastLevel::Error => colors::ACCENT_RED,
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ToastNotification {
    pub id: u64,
    pub level: ToastLevel,
    pub message: String,
    pub created_at: Instant,
    pub duration: Duration,
}

#[derive(Debug, Clone, Default)]
pub struct ToastManager {
    notifications: Vec<ToastNotification>,
    next_id: u64,
}

impl ToastManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show(&mut self, level: ToastLevel, message: impl Into<String>) {
        self.next_id += 1;
        self.notifications.push(ToastNotification {
            id: self.next_id,
            level,
            message: message.into(),
            created_at: Instant::now(),
            duration: Duration::from_secs(4),
        });
    }

    pub fn info(&mut self, message: impl Into<String>) {
        self.show(ToastLevel::Info, message);
    }

    pub fn warning(&mut self, message: impl Into<String>) {
        self.show(ToastLevel::Warning, message);
    }

    pub fn error(&mut self, message: impl Into<String>) {
        self.show(ToastLevel::Error, message);
    }

    pub fn draw(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        self.notifications.retain(|t| now.duration_since(t.created_at) < t.duration);

        if self.notifications.is_empty() {
            return;
        }

        egui::Area::new(egui::Id::new("toast_notification_area"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                    for toast in self.notifications.iter().rev() {
                        let accent = toast_color(&toast.level);
                        let elapsed = now.duration_since(toast.created_at).as_secs_f32();
                        let remaining_ratio = (1.0 - elapsed / toast.duration.as_secs_f32()).clamp(0.0, 1.0);

                        egui::Frame::window(ui.style())
                            .fill(colors::BG_DEEPEST)
                            .stroke(egui::Stroke::new(1.0, accent))
                            .rounding(egui::Rounding::same(4.0))
                            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                            .show(ui, |ui| {
                                ui.set_max_width(320.0);
                                ui.horizontal(|ui| {
                                    let icon = match toast.level {
                                        ToastLevel::Info => "ℹ",
                                        ToastLevel::Warning => "⚠️",
                                        ToastLevel::Error => "❌",
                                    };
                                    ui.label(egui::RichText::new(icon).strong().color(accent));
                                    ui.label(egui::RichText::new(&toast.message).small().color(colors::TEXT_ON_ACCENT));
                                });
                                // Progress bar line
                                let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width() * remaining_ratio, 2.0), egui::Sense::hover());
                                ui.painter().rect_filled(rect, 1.0, accent);
                            });
                        ui.add_space(6.0);
                    }
                });
            });
    }
}
