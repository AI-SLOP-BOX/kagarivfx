use eframe::egui;
use crate::AfterEffectsApp;
use crate::ui::theme::colors;

pub fn draw_shortcuts_dialog(app: &mut AfterEffectsApp, ctx: &egui::Context) {
    let mut show = app.show_shortcuts_dialog;
    if !show {
        return;
    }

    egui::Window::new("⌨ After Effects Keyboard Shortcuts")
        .open(&mut show)
        .resizable(true)
        .default_size(egui::vec2(520.0, 420.0))
        .show(ctx, |ui| {
            ui.heading("Standard After Effects Keybindings");
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("shortcuts_grid")
                    .striped(true)
                    .spacing([20.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Shortcut").strong());
                        ui.label(egui::RichText::new("Action").strong());
                        ui.end_row();

                        let shortcuts = [
                            ("Cmd + K / Ctrl + K", "✨ Open Command Palette (Fuzzy Search Everything)"),
                            ("Tab", "🕸 Toggle Hybrid Node Graph / Timeline Dual View"),
                            ("Double-Click Layer", "✏ Open Canvas Inline Quick Numeric Editor"),
                            ("Shift + Drag", "🎯 15° Rotation Snap / Orthogonal Axis Movement"),
                            ("Cmd + Y", "Create New Solid Layer"),
                            ("Cmd + Alt + Shift + T", "Create New Text Layer"),
                            ("Cmd + Alt + Shift + Y", "Create New Null Object Layer"),
                            ("Cmd + Alt + Y", "Create New Adjustment Layer"),
                            ("Cmd + Shift + C", "Pre-Compose Selected Layers"),
                            ("Cmd + D", "Duplicate Selected Layer"),
                            ("Cmd + Shift + D", "Split Layer at Current Time"),
                            ("F9", "Easy Ease Keyframes (with Bezier Presets)"),
                            ("J", "Jump to Previous Keyframe"),
                            ("K", "Jump to Next Keyframe"),
                            ("B", "Set Work Area Start at Current Frame"),
                            ("N", "Set Work Area End at Current Frame"),
                            ("Space", "Play / Stop RAM Preview"),
                            ("Cmd + Z", "Undo Single Drag Gesture"),
                            ("Cmd + Shift + Z", "Redo"),
                            ("P", "Select Position Property"),
                            ("S", "Select Scale Property"),
                            ("R", "Select Rotation Property"),
                            ("T", "Select Opacity Property"),
                            ("V", "Selection Tool"),
                            ("H", "Hand / Pan Tool"),
                            ("Z", "Zoom Tool"),
                            ("W", "Rotation Tool"),
                            ("Y", "Pan Behind / Anchor Point Tool"),
                            ("Q", "Shape Tool (Rectangle / Ellipse)"),
                            ("G", "Pen / Vector Bezier Path Tool"),
                        ];

                        for (sc, desc) in shortcuts {
                            ui.label(egui::RichText::new(sc).monospace().color(colors::ACCENT_BLUE));
                            ui.label(desc);
                            ui.end_row();
                        }
                    });
            });
        });

    app.show_shortcuts_dialog = show;
}
