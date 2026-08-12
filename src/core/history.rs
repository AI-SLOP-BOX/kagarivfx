use crate::core::timeline::Project;
use crate::core::frame_cache;

#[derive(Debug, Clone)]
pub struct ProjectHistory {
    // History stack of Project snapshots
    stack: Vec<Project>,
    current_idx: usize,
    /// Maximum number of undo states retained in memory (default 50).
    max_history_entries: usize,
}

impl ProjectHistory {
    pub fn new(initial: Project) -> Self {
        Self {
            stack: vec![initial],
            current_idx: 0,
            max_history_entries: 50,
        }
    }

    /// Commit a new project state snapshot.
    /// Any redo history (states beyond current_idx) is discarded.
    /// Automatically bumps the global frame cache version to prevent stale previews.
    pub fn commit(&mut self, project: Project) {
        self.stack.truncate(self.current_idx + 1);
        self.stack.push(project);
        self.current_idx += 1;

        if self.stack.len() > self.max_history_entries {
            self.stack.remove(0);
            self.current_idx = self.current_idx.saturating_sub(1);
        }

        // Cascade: any project change must invalidate all cached rendered frames.
        frame_cache::bump_version();
        log::debug!("Committed project history. Stack size: {}", self.stack.len());
    }

    /// Retrieve the current project state.
    pub fn current(&self) -> &Project {
        &self.stack[self.current_idx]
    }

    /// Retrieve the current project state mutably.
    pub fn current_mut(&mut self) -> &mut Project {
        &mut self.stack[self.current_idx]
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
