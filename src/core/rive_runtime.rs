#![allow(dead_code)]
use std::collections::HashMap;

/// Rive Interactive State Machine Input types.
#[derive(Debug, Clone)]
pub enum RiveInput {
    Boolean(bool),
    Number(f32),
    Trigger,
}

/// Rive State Machine Runtime Engine for interactive 2D real-time vector graphics.
pub struct RiveRuntimeEngine {
    pub inputs: HashMap<String, RiveInput>,
    pub current_state: String,
}

impl RiveRuntimeEngine {
    pub fn new(initial_state: &str) -> Self {
        Self {
            inputs: HashMap::new(),
            current_state: initial_state.to_string(),
        }
    }

    /// Sets an interactive State Machine input parameter (e.g. mouse hover, click).
    pub fn set_input(&mut self, name: &str, input: RiveInput) {
        self.inputs.insert(name.to_string(), input);
    }

    /// Advances the Rive State Machine evaluation by time delta `dt`.
    pub fn advance(&mut self, _dt_sec: f32) -> &str {
        if let Some(RiveInput::Boolean(true)) = self.inputs.get("is_hovered") {
            self.current_state = "HoverState".to_string();
        } else if let Some(RiveInput::Trigger) = self.inputs.get("click_trigger") {
            self.current_state = "ClickState".to_string();
        } else {
            self.current_state = "IdleState".to_string();
        }

        &self.current_state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rive_state_machine_transition() {
        let mut engine = RiveRuntimeEngine::new("IdleState");
        assert_eq!(engine.advance(0.016), "IdleState");

        engine.set_input("is_hovered", RiveInput::Boolean(true));
        assert_eq!(engine.advance(0.016), "HoverState");
    }
}
