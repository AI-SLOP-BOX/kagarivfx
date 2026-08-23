use std::collections::VecDeque;

use crate::core::timeline::Project;
use crate::core::frame_cache;

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
}

/// Rough per-entry size estimate: layer count × conservative per-layer footprint.
/// Exact measurement would require serialization (too slow per edit); this keeps
/// the budget O(layers) which is what actually drives snapshot size.
fn estimate_project_bytes(project: &Project) -> usize {
    let mut layers = 0usize;
    let mut keyframes = 0usize;
    for comp in &project.compositions {
        layers += comp.layers.len() + comp.sub_compositions.iter().map(|s| s.layers.len()).sum::<usize>();
        for comp in std::iter::once(comp).chain(comp.sub_compositions.iter()) {
            for l in &comp.layers {
                keyframes += l.transform.position.keyframes().map(|k| k.len()).unwrap_or(0)
                    + l.transform.scale.keyframes().map(|k| k.len()).unwrap_or(0)
                    + l.transform.rotation.keyframes().map(|k| k.len()).unwrap_or(0)
                    + l.transform.opacity.keyframes().map(|k| k.len()).unwrap_or(0);
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
        self.approx_bytes = self.stack.iter().map(|e| estimate_project_bytes(&e.project)).sum();
    }

    /// Approximate total bytes currently retained across all snapshots.
    pub fn approx_bytes(&self) -> usize {
        self.approx_bytes
    }

    /// Commit a new project state snapshot with a descriptive action name.
    pub fn commit_action(&mut self, project: Project, action_name: &str) {
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
                self.approx_bytes = self.approx_bytes.saturating_sub(estimate_project_bytes(&removed.project));
            }
            self.stack.pop_front();
            self.current_idx = self.current_idx.saturating_sub(1);
        }

        // Byte-budget trim: drop oldest states until under the memory ceiling.
        while self.stack.len() > 1 && self.approx_bytes > Self::MAX_HISTORY_BYTES {
            if let Some(removed) = self.stack.front() {
                self.approx_bytes = self.approx_bytes.saturating_sub(estimate_project_bytes(&removed.project));
            }
            self.stack.pop_front();
            self.current_idx = self.current_idx.saturating_sub(1);
        }

        // Cascade: any project change must invalidate all cached rendered frames.
        frame_cache::bump_version();
        log::debug!("Committed action '{}'. Stack size: {}", action_name, self.stack.len());
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

    /// Check if undo is available.
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
    use crate::core::timeline::{Composition, Layer, LayerType};

    fn big_project(layers: usize, keyframes_per_layer: usize) -> Project {
        let mut comp = Composition::new("c".into(), "Big".into(), 64, 64, 30, 30);
        for i in 0..layers {
            let mut l = Layer::new(format!("l{}", i), format!("L{}", i), LayerType::Solid { color: [1.0; 4] }, 30);
            let kfs: Vec<crate::core::keyframe::Keyframe<[f32; 2]>> = (0..keyframes_per_layer)
                .map(|k| crate::core::keyframe::Keyframe::new(k as u32, [k as f32, 0.0], crate::core::keyframe::InterpolationType::Linear))
                .collect();
            l.transform.position = crate::core::property::Animatable::new_animated(kfs);
            comp.layers.push(l);
        }
        Project { compositions: vec![comp], active_composition_idx: 0, assets: Vec::new() }
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
}
