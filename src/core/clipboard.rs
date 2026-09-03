#![allow(dead_code)]
use crate::core::timeline::{Composition, Effect, Layer};
use std::collections::HashMap;

/// Clipboard state for copy/paste operations
#[derive(Debug, Clone, Default)]
pub struct ClipboardState {
    /// Copied layers (index → layer clone)
    pub copied_layers: Vec<(usize, Layer)>,
    /// Copied effects from a specific layer
    pub copied_effects: Vec<Effect>,
    /// Copied keyframes: property_path → Vec of keyframes serialized as JSON
    pub copied_keyframes: HashMap<String, String>,
    /// Paste mode
    pub paste_mode: PasteMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PasteMode {
    #[default]
    Insert,
    Overwrite,
}

impl ClipboardState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Copy layers by their indices from the composition
    pub fn copy_layers(&mut self, indices: &[usize], comp: &Composition) {
        self.copied_layers.clear();
        for &idx in indices {
            if let Some(layer) = comp.layers.get(idx) {
                self.copied_layers.push((idx, layer.clone()));
            }
        }
    }

    /// Copy all effects from a layer
    pub fn copy_effects(&mut self, layer: &Layer) {
        self.copied_effects = layer.effects.clone();
    }

    /// Returns true if there are layers in the clipboard
    pub fn has_copied_layers(&self) -> bool {
        !self.copied_layers.is_empty()
    }

    /// Returns true if there are effects in the clipboard
    pub fn has_copied_effects(&self) -> bool {
        !self.copied_effects.is_empty()
    }

    /// Paste copied layers into the composition at the given position.
    /// Each pasted layer receives a new unique ID to avoid collisions.
    pub fn paste_layers(&self, comp: &mut Composition, insert_at: usize) {
        if self.copied_layers.is_empty() {
            return;
        }
        let insert_at = insert_at.min(comp.layers.len());
        for (offset, (orig_idx, layer)) in self.copied_layers.iter().enumerate() {
            let mut cloned = layer.clone();
            // Assign a new unique ID based on original index and a timestamp-like counter
            cloned.id = format!(
                "{}_paste_{}_{}",
                cloned.id,
                orig_idx,
                comp.layers.len() + offset
            );
            // Clear parent reference to avoid dangling pointers
            cloned.parent_id = None;
            comp.layers.insert(insert_at + offset, cloned);
        }
    }

    /// Paste copied effects onto a target layer, appending them
    pub fn paste_effects(&self, layer: &mut Layer) {
        for effect in &self.copied_effects {
            let mut cloned = effect.clone();
            // Assign a new unique ID to the pasted effect
            cloned.id = format!("{}_paste_{}", cloned.id, layer.effects.len());
            layer.effects.push(cloned);
        }
    }

    /// Clear all clipboard contents
    pub fn clear(&mut self) {
        self.copied_layers.clear();
        self.copied_effects.clear();
        self.copied_keyframes.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeline::{Composition, LayerType};

    fn make_test_comp() -> Composition {
        let mut comp = Composition::new("comp1".into(), "Test".into(), 1920, 1080, 30, 300);
        comp.layers.push(Layer::new(
            "layer1".into(),
            "Layer 1".into(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            300,
        ));
        comp.layers.push(Layer::new(
            "layer2".into(),
            "Layer 2".into(),
            LayerType::Null,
            300,
        ));
        comp
    }

    #[test]
    fn test_copy_paste_layers() {
        let comp = make_test_comp();
        let mut clipboard = ClipboardState::new();

        clipboard.copy_layers(&[0, 1], &comp);
        assert!(clipboard.has_copied_layers());
        assert_eq!(clipboard.copied_layers.len(), 2);

        let mut comp2 = make_test_comp();
        clipboard.paste_layers(&mut comp2, 1);

        assert_eq!(comp2.layers.len(), 4);
        // Pasted layers should have new IDs
        assert_ne!(comp2.layers[1].id, "layer1");
        assert_ne!(comp2.layers[2].id, "layer2");
    }

    #[test]
    fn test_copy_paste_effects() {
        let mut comp = make_test_comp();
        let effect = Effect {
            id: "eff1".into(),
            name: "Blur".into(),
            effect_type: crate::core::timeline::EffectType::GaussianBlur {
                blur_radius: crate::core::property::Animatable::new_constant(5.0),
            },
            enabled: true,
        };
        comp.layers[0].effects.push(effect);

        let mut clipboard = ClipboardState::new();
        clipboard.copy_effects(&comp.layers[0]);
        assert!(clipboard.has_copied_effects());

        clipboard.paste_effects(&mut comp.layers[1]);
        assert_eq!(comp.layers[1].effects.len(), 1);
        assert_ne!(comp.layers[1].effects[0].id, "eff1");
    }

    #[test]
    fn test_clear() {
        let comp = make_test_comp();
        let mut clipboard = ClipboardState::new();
        clipboard.copy_layers(&[0], &comp);
        clipboard.clear();
        assert!(!clipboard.has_copied_layers());
        assert!(!clipboard.has_copied_effects());
    }
}
