/// Undo/Redo system for project state changes.
///
/// Stores snapshots of the composition state at each action point.
/// Supports unlimited undo/redo with memory-efficient snapshots.
use crate::core::timeline::Composition;

/// A snapshot of project state for undo/redo.
#[derive(Debug, Clone)]
pub struct StateSnapshot {
    /// Description of the action that produced this state
    pub description: String,
    /// Serialized composition state (JSON)
    pub composition_json: String,
    /// Frame at which the snapshot was taken
    pub frame: u32,
}

/// Undo/Redo manager.
pub struct UndoManager {
    /// History stack (oldest first)
    history: Vec<StateSnapshot>,
    /// Current position in history (index into `history`)
    cursor: usize,
    /// Maximum undo levels
    max_levels: usize,
}

impl Default for UndoManager {
    fn default() -> Self {
        Self::new(100)
    }
}

impl UndoManager {
    pub fn new(max_levels: usize) -> Self {
        Self {
            history: Vec::new(),
            cursor: 0,
            max_levels,
        }
    }

    /// Push a new state snapshot. Clears any redo history.
    pub fn push(&mut self, comp: &Composition, frame: u32, description: &str) {
        let json = serde_json::to_string(comp).unwrap_or_default();

        // Discard any redo states (anything after cursor)
        self.history.truncate(self.cursor);

        self.history.push(StateSnapshot {
            description: description.to_string(),
            composition_json: json,
            frame,
        });

        // Enforce max levels
        if self.history.len() > self.max_levels {
            let excess = self.history.len() - self.max_levels;
            self.history.drain(0..excess);
        }

        self.cursor = self.history.len();
    }

    /// Undo: move cursor back one step and return the previous state.
    pub fn undo(&mut self) -> Option<&StateSnapshot> {
        if self.cursor <= 1 {
            return None;
        }
        self.cursor -= 1;
        self.history.get(self.cursor - 1)
    }

    /// Redo: move cursor forward one step and return the next state.
    pub fn redo(&mut self) -> Option<&StateSnapshot> {
        if self.cursor >= self.history.len() {
            return None;
        }
        let snapshot = self.history.get(self.cursor);
        self.cursor += 1;
        snapshot
    }

    /// Check if undo is available.
    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        self.cursor < self.history.len()
    }

    /// Get the current state description.
    pub fn current_description(&self) -> Option<&str> {
        if self.cursor > 0 && self.cursor <= self.history.len() {
            self.history.get(self.cursor - 1).map(|s| s.description.as_str())
        } else {
            None
        }
    }

    /// Get the number of undo levels available.
    pub fn undo_depth(&self) -> usize {
        self.cursor
    }

    /// Get the number of redo levels available.
    pub fn redo_depth(&self) -> usize {
        self.history.len().saturating_sub(self.cursor)
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.history.clear();
        self.cursor = 0;
    }

    /// Get a reference to a snapshot at a specific history index (for UI display).
    pub fn get_snapshot(&self, index: usize) -> Option<&StateSnapshot> {
        self.history.get(index)
    }

    /// Total number of snapshots in history.
    pub fn len(&self) -> usize {
        self.history.len()
    }

    /// Check if history is empty.
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_comp() -> Composition {
        Composition::new("test".into(), "Test".into(), 1920, 1080, 30, 100)
    }

    #[test]
    fn test_push_and_undo() {
        let mut mgr = UndoManager::new(10);
        let comp = test_comp();
        mgr.push(&comp, 0, "Initial");
        mgr.push(&comp, 10, "Add layer");

        assert!(mgr.can_undo());
        let snap = mgr.undo().unwrap();
        // After one undo from "Add layer", cursor moves back, returning "Initial" state
        assert_eq!(snap.description, "Initial");
    }

    #[test]
    fn test_redo() {
        let mut mgr = UndoManager::new(10);
        let comp = test_comp();
        mgr.push(&comp, 0, "Initial");
        mgr.push(&comp, 10, "Add layer");

        mgr.undo();
        assert!(mgr.can_redo());
        let snap = mgr.redo().unwrap();
        assert_eq!(snap.description, "Add layer");
    }

    #[test]
    fn test_undo_clears_redo() {
        let mut mgr = UndoManager::new(10);
        let comp = test_comp();
        mgr.push(&comp, 0, "A");
        mgr.push(&comp, 0, "B");
        mgr.push(&comp, 0, "C");

        mgr.undo(); // back to B
        mgr.undo(); // back to A
        assert!(mgr.can_redo());

        // Push new state — redo should be cleared
        mgr.push(&comp, 0, "D");
        assert!(!mgr.can_redo());
    }

    #[test]
    fn test_max_levels() {
        let mut mgr = UndoManager::new(3);
        let comp = test_comp();
        for i in 0..10 {
            mgr.push(&comp, 0, &format!("Step {}", i));
        }
        assert!(mgr.len() <= 3);
        assert!(mgr.can_undo());
    }

    #[test]
    fn test_undo_at_start() {
        let mut mgr = UndoManager::new(10);
        assert!(!mgr.can_undo());
        assert!(mgr.undo().is_none());
    }

    #[test]
    fn test_redo_at_end() {
        let mut mgr = UndoManager::new(10);
        let comp = test_comp();
        mgr.push(&comp, 0, "A");
        assert!(!mgr.can_redo());
        assert!(mgr.redo().is_none());
    }

    #[test]
    fn test_clear() {
        let mut mgr = UndoManager::new(10);
        let comp = test_comp();
        mgr.push(&comp, 0, "A");
        mgr.clear();
        assert!(mgr.is_empty());
        assert!(!mgr.can_undo());
    }

    #[test]
    fn test_depth_tracking() {
        let mut mgr = UndoManager::new(10);
        let comp = test_comp();
        assert_eq!(mgr.undo_depth(), 0);
        assert_eq!(mgr.redo_depth(), 0);

        mgr.push(&comp, 0, "A");
        assert_eq!(mgr.undo_depth(), 1);

        mgr.push(&comp, 0, "B");
        assert_eq!(mgr.undo_depth(), 2);

        mgr.undo();
        assert_eq!(mgr.undo_depth(), 1);
        assert_eq!(mgr.redo_depth(), 1);
    }
}
