use std::collections::VecDeque;

use crate::core::frame_cache;
use crate::core::timeline::Project;

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub project: Project,
    pub action_name: String,
}

#[derive(Debug, Clone)]
pub struct ProjectHistory {
    // History stack of Project snapshots with action descriptions
    stack: VecDeque<HistoryEntry>,
    current_idx: usize,
    /// Maximum number of undo states retained in memory (default 50).
    max_history_entries: usize,
    /// Approximate total bytes of retained snapshots. Full project clones of
    /// large compositions can each be many MB; without a byte budget, 50 entries
    /// could pin hundreds of MB of RAM.
    approx_bytes: usize,
    /// Monotonic counter that increments on every commit. UI code can cache the
    /// last-seen generation to avoid redundant full-project clones.
    generation: u64,
}

/// Rough per-entry size estimate: layer count × conservative per-layer footprint.
/// Exact measurement would require serialization (too slow per edit); this keeps
/// the budget O(layers) which is what actually drives snapshot size.
fn estimate_project_bytes(project: &Project) -> usize {
    let mut layers = 0usize;
    let mut keyframes = 0usize;
    for comp in &project.compositions {
        layers += comp.layers.len()
            + comp
                .sub_compositions
                .iter()
                .map(|s| s.layers.len())
                .sum::<usize>();
        for comp in std::iter::once(comp).chain(comp.sub_compositions.iter()) {
            for l in &comp.layers {
                keyframes += l
                    .transform
                    .position
                    .keyframes()
                    .map(|k| k.len())
                    .unwrap_or(0)
                    + l.transform.scale.keyframes().map(|k| k.len()).unwrap_or(0)
                    + l.transform
                        .rotation
                        .keyframes()
                        .map(|k| k.len())
                        .unwrap_or(0)
                    + l.transform
                        .opacity
                        .keyframes()
                        .map(|k| k.len())
                        .unwrap_or(0)
                    + l.shape_repeater
                        .as_ref()
                        .and_then(|r| r.copies_animation.as_ref())
                        .and_then(|a| a.keyframes())
                        .map(|k| k.len())
                        .unwrap_or(0)
                    + l.shape_repeater
                        .as_ref()
                        .and_then(|r| r.position_offset_animation.as_ref())
                        .and_then(|a| a.keyframes())
                        .map(|k| k.len())
                        .unwrap_or(0)
                    + l.shape_repeater
                        .as_ref()
                        .and_then(|r| r.scale_offset_animation.as_ref())
                        .and_then(|a| a.keyframes())
                        .map(|k| k.len())
                        .unwrap_or(0)
                    + l.shape_repeater
                        .as_ref()
                        .and_then(|r| r.rotation_offset_animation.as_ref())
                        .and_then(|a| a.keyframes())
                        .map(|k| k.len())
                        .unwrap_or(0)
                    + l.shape_repeater
                        .as_ref()
                        .and_then(|r| r.opacity_animation.as_ref())
                        .and_then(|a| a.keyframes())
                        .map(|k| k.len())
                        .unwrap_or(0);
            }
        }
    }
    // ~2KB base per layer + ~256B per keyframe (conservative upper bound)
    2048 * layers + 256 * keyframes + 4096
}

impl ProjectHistory {
    /// Total snapshot byte budget before oldest entries are trimmed.
    pub const MAX_HISTORY_BYTES: usize = 128 * 1024 * 1024; // 128 MB

    pub fn new(initial: Project) -> Self {
        let mut hist = Self {
            stack: VecDeque::new(),
            current_idx: 0,
            max_history_entries: 50,
            approx_bytes: 0,
            generation: 0,
        };
        hist.stack.push_back(HistoryEntry {
            project: initial,
            action_name: "Initial State".to_string(),
        });
        hist.recompute_bytes();
        hist
    }

    /// Recomputes the byte estimate after construction.
    fn recompute_bytes(&mut self) {
        self.approx_bytes = self
            .stack
            .iter()
            .map(|e| estimate_project_bytes(&e.project))
            .sum();
    }

    /// Approximate total bytes currently retained across all snapshots.
    pub fn approx_bytes(&self) -> usize {
        self.approx_bytes
    }

    /// Monotonic generation counter. Increments on every commit.
    /// UI code should cache this value and only clone the project when it changes.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Commit a new project state snapshot with a descriptive action name.
    pub fn commit_action(&mut self, project: Project, action_name: &str) {
        let unchanged = match (
            serde_json::to_vec(self.current()),
            serde_json::to_vec(&project),
        ) {
            (Ok(current), Ok(next)) => current == next,
            _ => false,
        };
        if unchanged {
            return;
        }
        self.generation = self.generation.wrapping_add(1);
        self.stack.truncate(self.current_idx + 1);
        self.approx_bytes = self
            .stack
            .iter()
            .map(|e| estimate_project_bytes(&e.project))
            .sum();
        let entry_bytes = estimate_project_bytes(&project);
        self.stack.push_back(HistoryEntry {
            project,
            action_name: action_name.to_string(),
        });
        self.current_idx += 1;
        self.approx_bytes += entry_bytes;

        if self.stack.len() > self.max_history_entries {
            if let Some(removed) = self.stack.front() {
                self.approx_bytes = self
                    .approx_bytes
                    .saturating_sub(estimate_project_bytes(&removed.project));
            }
            self.stack.pop_front();
            self.current_idx = self.current_idx.saturating_sub(1);
        }

        // Byte-budget trim: drop oldest states until under the memory ceiling.
        while self.stack.len() > 1 && self.approx_bytes > Self::MAX_HISTORY_BYTES {
            if let Some(removed) = self.stack.front() {
                self.approx_bytes = self
                    .approx_bytes
                    .saturating_sub(estimate_project_bytes(&removed.project));
            }
            self.stack.pop_front();
            self.current_idx = self.current_idx.saturating_sub(1);
        }

        // Cascade: any project change must invalidate all cached rendered frames.
        frame_cache::bump_version();
        log::debug!(
            "Committed action '{}'. Stack size: {}",
            action_name,
            self.stack.len()
        );
    }

    /// Commit a new project state snapshot.
    pub fn commit(&mut self, project: Project) {
        self.commit_action(project, "Edit Project");
    }

    /// Get current action name.
    pub fn current_action_name(&self) -> &str {
        &self.stack[self.current_idx].action_name
    }

    /// Get descriptive name of action that will be undone if Undo is pressed.
    pub fn undo_action_name(&self) -> Option<&str> {
        if self.can_undo() {
            Some(&self.stack[self.current_idx].action_name)
        } else {
            None
        }
    }

    /// Get descriptive name of action that will be redone if Redo is pressed.
    pub fn redo_action_name(&self) -> Option<&str> {
        if self.can_redo() {
            Some(&self.stack[self.current_idx + 1].action_name)
        } else {
            None
        }
    }

    /// Retrieve the current project state.
    pub fn current(&self) -> &Project {
        &self.stack[self.current_idx].project
    }

    /// Retrieve the current project state mutably.
    pub fn current_mut(&mut self) -> &mut Project {
        &mut self.stack[self.current_idx].project
    }

    // Check if undo is available.
    /// Adjust the undo-step cap (Preferences UI). Takes effect on next commit;
    /// clamped to a sane minimum of 10.
    pub fn set_max_history_entries(&mut self, n: usize) {
        self.max_history_entries = n.max(10);
    }

    /// Current undo-step cap.
    pub fn max_history_entries(&self) -> usize {
        self.max_history_entries
    }

    /// Number of stored history steps (including the initial snapshot).
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    /// True when no history entries exist.
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Index of the active entry (for highlighting in a history UI).
    pub fn current_index(&self) -> usize {
        self.current_idx
    }

    /// Action name of the entry at `idx`, if in range.
    pub fn action_name_at(&self, idx: usize) -> Option<&str> {
        self.stack.get(idx).map(|e| e.action_name.as_str())
    }

    /// Jump to an arbitrary history index (Undo History panel support):
    /// repeatedly undoes or redoes until `current_idx` matches. Returns
    /// true when the jump happened.
    pub fn jump_to(&mut self, idx: usize) -> bool {
        if idx >= self.stack.len() || idx == self.current_idx {
            return false;
        }
        self.current_idx = idx;
        crate::core::frame_cache::bump_version();
        true
    }

    pub fn can_undo(&self) -> bool {
        self.current_idx > 0
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        self.current_idx < self.stack.len() - 1
    }

    /// Undo to the previous state.
    pub fn undo(&mut self) -> Option<&Project> {
        if self.can_undo() {
            self.current_idx -= 1;
            self.generation = self.generation.wrapping_add(1);
            frame_cache::bump_version();
            log::info!("Undo success. Frame position index: {}", self.current_idx);
            Some(self.current())
        } else {
            log::warn!("Undo ignored: no history left");
            None
        }
    }

    /// Redo to the next state.
    pub fn redo(&mut self) -> Option<&Project> {
        if self.can_redo() {
            self.current_idx += 1;
            self.generation = self.generation.wrapping_add(1);
            frame_cache::bump_version();
            log::info!("Redo success. Frame position index: {}", self.current_idx);
            Some(self.current())
        } else {
            log::warn!("Redo ignored: end of history");
            None
        }
    }
}

#[cfg(test)]
mod memory_bound_tests {
    use super::*;
    use crate::core::keyframe::{InterpolationType, Keyframe};
    use crate::core::property::Animatable;
    use crate::core::shape_repeater::ShapeRepeaterOptions;
    use crate::core::timeline::{Composition, Layer, LayerType};

    fn big_project(layers: usize, keyframes_per_layer: usize) -> Project {
        let mut comp = Composition::new("c".into(), "Big".into(), 64, 64, 30, 30);
        for i in 0..layers {
            let mut l = Layer::new(
                format!("l{}", i),
                format!("L{}", i),
                LayerType::Solid { color: [1.0; 4] },
                30,
            );
            let kfs: Vec<crate::core::keyframe::Keyframe<[f32; 2]>> = (0..keyframes_per_layer)
                .map(|k| {
                    crate::core::keyframe::Keyframe::new(
                        k as u32,
                        [k as f32, 0.0],
                        crate::core::keyframe::InterpolationType::Linear,
                    )
                })
                .collect();
            l.transform.position = crate::core::property::Animatable::new_animated(kfs);
            comp.layers.push(l);
        }
        Project {
            compositions: vec![comp],
            active_composition_idx: 0,
            assets: Vec::new(),
            use_gpu_compute: false,
        }
    }

    #[test]
    fn test_byte_budget_trims_oldest_entries() {
        let mut history = ProjectHistory::new(big_project(100, 100));
        let initial_bytes = history.approx_bytes();
        assert!(initial_bytes > 0, "estimate must be positive");

        // Commit many large snapshots: 100 layers x 100 kfs ≈ 4.6MB each.
        // 128MB budget → should trim well before 50 entries.
        for i in 0..60 {
            history.commit_action(big_project(100, 100), &format!("edit {}", i));
        }

        assert!(
            history.approx_bytes() <= ProjectHistory::MAX_HISTORY_BYTES,
            "byte budget must be enforced, got {}",
            history.approx_bytes()
        );
        assert!(
            history.stack.len() < 50,
            "byte budget should trim before entry limit, got {} entries",
            history.stack.len()
        );
        // Current state must remain accessible
        let _ = history.current_action_name();
    }

    #[test]
    fn test_small_projects_keep_full_history() {
        let mut history = ProjectHistory::new(big_project(2, 2));
        for i in 0..30 {
            history.commit_action(big_project(2, 2), &format!("small {}", i));
        }
        // Tiny projects: entry-count limit (50) is the binding constraint, not bytes
        assert!(history.approx_bytes() < ProjectHistory::MAX_HISTORY_BYTES);
        assert!(history.stack.len() <= 50);
    }

    #[test]
    fn test_named_edit_survives_undo_and_redo() {
        let initial = big_project(1, 0);
        let mut history = ProjectHistory::new(initial.clone());
        let mut edited = initial;
        if let LayerType::Solid { ref mut color } = edited.compositions[0].layers[0].layer_type {
            color[0] = 0.25;
        }
        history.commit_action(edited, "Repeater Rotation Keyframe");
        assert_eq!(history.current_action_name(), "Repeater Rotation Keyframe");
        assert!(history.undo().is_some());
        if let LayerType::Solid { color } = &history.current().compositions[0].layers[0].layer_type
        {
            assert_eq!(color[0], 1.0);
        } else {
            panic!("expected solid layer");
        }
        assert_eq!(
            history.redo_action_name(),
            Some("Repeater Rotation Keyframe")
        );
        assert!(history.redo().is_some());
        assert_eq!(history.current_action_name(), "Repeater Rotation Keyframe");
        if let LayerType::Solid { color } = &history.current().compositions[0].layers[0].layer_type
        {
            assert_eq!(color[0], 0.25);
        } else {
            panic!("expected solid layer");
        }
    }

    #[test]
    fn test_repeater_copies_keyframes_survive_undo_and_redo() {
        let initial = big_project(1, 0);
        let mut history = ProjectHistory::new(initial.clone());
        let mut edited = initial.clone();
        let mut copies = Animatable::new_constant(2.0);
        copies.add_keyframe(Keyframe::new(0, 2.0, InterpolationType::Linear));
        copies.add_keyframe(Keyframe::new(12, 6.0, InterpolationType::Linear));
        edited.compositions[0].layers[0].shape_repeater = Some(ShapeRepeaterOptions {
            copies_animation: Some(copies),
            ..ShapeRepeaterOptions::default()
        });
        history.commit_action(edited, "Repeater Copies Keyframes");

        let restored = history.undo().expect("undo returns initial project");
        assert!(restored.compositions[0].layers[0].shape_repeater.is_none());
        let redone = history.redo().expect("redo returns repeater project");
        let animation = redone.compositions[0].layers[0]
            .shape_repeater
            .as_ref()
            .and_then(|repeater| repeater.copies_animation.as_ref())
            .and_then(|copies| copies.keyframes())
            .expect("copies keyframes are restored");
        assert_eq!(animation.len(), 3);
        assert_eq!(animation[2].frame, 12);
        assert_eq!(animation[2].value, 6.0);
    }

    #[test]
    fn camera_lens_keyframe_edit_is_reversible() {
        let initial = big_project(1, 0);
        let mut history = ProjectHistory::new(initial.clone());
        let mut edited = initial;
        edited.compositions[0].active_camera.set_fov_at(12, 90.0);
        edited.compositions[0]
            .active_camera
            .set_dof_max_blur_at(12, 32.0);
        history.commit_action(edited, "Camera Lens Keyframe");
        assert_eq!(
            history.undo().unwrap().compositions[0]
                .active_camera
                .fov_at(12),
            50.0
        );
        let restored = history.redo().unwrap();
        assert_eq!(restored.compositions[0].active_camera.fov_at(12), 90.0);
        assert_eq!(
            restored.compositions[0].active_camera.dof_max_blur_at(12),
            32.0
        );
    }

    #[test]
    fn transform_keyframe_move_is_reversible_through_history() {
        let initial = big_project(1, 0);
        let mut history = ProjectHistory::new(initial.clone());
        let mut edited = initial;
        let position = &mut edited.compositions[0].layers[0].transform.position;
        position.add_keyframe(Keyframe::new(8, [12.0, 24.0], InterpolationType::Hold));
        assert!(position.move_keyframe(8, 18));
        history.commit_action(edited, "Move Transform Keyframe");

        let undone = history.undo().unwrap();
        assert!(undone.compositions[0].layers[0]
            .transform
            .position
            .keyframes()
            .is_none_or(|keys| keys.iter().all(|key| key.frame != 18)));
        let redone = history.redo().unwrap();
        let key = redone.compositions[0].layers[0]
            .transform
            .position
            .keyframes()
            .unwrap()
            .iter()
            .find(|key| key.frame == 18)
            .unwrap();
        assert_eq!(key.value, [12.0, 24.0]);
        assert!(matches!(key.interpolation, InterpolationType::Hold));
    }

    #[test]
    fn identical_project_commit_does_not_create_history_entry() {
        let initial = big_project(1, 0);
        let mut history = ProjectHistory::new(initial.clone());
        history.commit_action(initial, "No-op Edit");
        assert_eq!(history.len(), 1);
        assert_eq!(history.current_action_name(), "Initial State");
        assert!(!history.can_undo());
    }

    #[test]
    fn no_op_commit_after_undo_keeps_redo_branch() {
        let initial = big_project(1, 0);
        let mut history = ProjectHistory::new(initial.clone());
        let mut edited = initial.clone();
        edited.compositions[0].layers[0].in_frame = 3;
        history.commit_action(edited, "Trim Layer");
        assert!(history.undo().is_some());
        history.commit_action(initial, "No-op After Undo");
        assert!(history.can_redo());
        assert_eq!(history.redo_action_name(), Some("Trim Layer"));
    }

    #[test]
    fn serialization_failure_never_swallows_a_real_commit() {
        let mut initial = big_project(1, 0);
        initial.compositions[0].layers[0].transform.opacity = Animatable::new_constant(f32::NAN);
        let mut history = ProjectHistory::new(initial.clone());
        let mut edited = initial;
        edited.compositions[0].layers[0].name = "Changed".into();
        history.commit_action(edited, "Edit NaN Project");
        assert_eq!(history.len(), 2);
        assert!(history.can_undo());
    }
}
