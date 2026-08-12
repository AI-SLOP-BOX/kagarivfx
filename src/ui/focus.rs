use eframe::egui;

/// Helper: Returns true if any text edit widget currently holds keyboard focus in egui.
/// Use this check to suppress global single-key shortcuts (e.g. Space, V, H, Z, C, M, F4, Delete)
/// when the user is typing into a text field.
pub fn is_text_input_focused(ctx: &egui::Context) -> bool {
    ctx.memory(|m| m.focused().is_some()) || ctx.wants_keyboard_input()
}
