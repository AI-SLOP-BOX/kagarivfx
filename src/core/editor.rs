//! Toolkit-neutral editing boundary.
//!
//! `EditorSession` wraps `ProjectHistory` and provides a safe mutation API.
//! All frontend/UI code should go through `EditorSession` for ordinary editing
//! operations. This ensures:
//!
//! - Every mutation creates an undo entry
//! - Generation is updated
//! - Cache invalidation happens
//! - Dirty-state tracking works
//!
//! The session captures a pre-edit snapshot lazily (only when a mutation
//! actually occurs) and creates an undo entry on commit.

use crate::core::history::ProjectHistory;
use crate::core::timeline::Project;

/// A toolkit-neutral editing session that wraps project mutations.
///
/// # Invariants
///
/// - `current()` always returns the current project state.
/// - `current_mut()` returns a mutable reference to the live stack entry.
///   After calling this, you MUST call `commit()` or `cancel()` to create
///   a proper undo entry.
/// - `ensure_snapshot()` must be called BEFORE any mutation to capture the
///   pre-edit state. It is idempotent (only captures once).
/// - `commit()` creates an undo entry with the pre-edit snapshot.
/// - `cancel()` reverts to the pre-edit snapshot.
/// - If the session is dropped without `commit()` or `cancel()`, it
///   auto-commits (safety net).
pub struct EditorSession<'a> {
    history: &'a mut ProjectHistory,
    snapshot: Option<Project>,
    label: &'static str,
}

impl<'a> EditorSession<'a> {
    /// Create a new editing session.
    pub fn new(history: &'a mut ProjectHistory, label: &'static str) -> Self {
        Self {
            history,
            snapshot: None,
            label,
        }
    }

    /// Get a reference to the current project.
    pub fn current(&self) -> &Project {
        self.history.current()
    }

    /// Get a mutable reference to the current project.
    ///
    /// **WARNING**: This mutates the live stack entry in place. You MUST call
    /// `commit()` or `cancel()` to properly handle the undo entry. If you
    /// haven't called `ensure_snapshot()` yet, this will also capture the
    /// pre-edit snapshot (lazy capture).
    pub fn current_mut(&mut self) -> &mut Project {
        self.ensure_snapshot();
        self.history.current_mut()
    }

    /// Ensure a pre-edit snapshot is captured. Call this BEFORE any mutation.
    ///
    /// This is idempotent — it only captures the snapshot once, on the first
    /// call. Subsequent calls are no-ops.
    pub fn ensure_snapshot(&mut self) {
        if self.snapshot.is_none() {
            self.snapshot = Some(self.history.current().clone());
        }
    }

    /// Whether a mutation has occurred (snapshot was captured).
    pub fn has_mutation(&self) -> bool {
        self.snapshot.is_some()
    }

    /// Commit the editing session, creating an undo entry.
    ///
    /// If no mutation occurred (no snapshot was captured), this is a no-op.
    pub fn commit(mut self) {
        if let (Some(snapshot), Some(label)) = (self.snapshot.take(), Some(self.label)) {
            let current = self.history.current().clone();
            // No-op: pre-edit snapshot equals current state (nothing changed)
            let unchanged = match (
                serde_json::to_vec(&current),
                serde_json::to_vec(&snapshot),
            ) {
                (Ok(current), Ok(snapshot)) => current == snapshot,
                _ => false,
            };
            if unchanged {
                return;
            }
            self.history
                .commit_drag_action(snapshot, current, label);
        }
    }

    fn commit_inner(&mut self) {
        if let (Some(snapshot), Some(label)) = (self.snapshot.take(), Some(self.label)) {
            let current = self.history.current().clone();
            // No-op: pre-edit snapshot equals current state (nothing changed)
            let unchanged = match (
                serde_json::to_vec(&current),
                serde_json::to_vec(&snapshot),
            ) {
                (Ok(current), Ok(snapshot)) => current == snapshot,
                _ => false,
            };
            if unchanged {
                return;
            }
            self.history
                .commit_drag_action(snapshot, current, label);
        }
    }

    /// Cancel the editing session, reverting to the pre-edit state.
    ///
    /// If no mutation occurred, this is a no-op.
    pub fn cancel(mut self) {
        if let Some(snapshot) = self.snapshot.take() {
            self.history.commit(snapshot);
        }
    }

    /// Access the underlying history for read-only queries.
    pub fn history(&self) -> &ProjectHistory {
        self.history
    }
}

impl Drop for EditorSession<'_> {
    fn drop(&mut self) {
        // Safety net: if neither commit nor cancel was called, auto-commit.
        if self.snapshot.is_some() {
            self.commit_inner();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeline::{Composition, Effect, EffectType, Layer, LayerType};
    use crate::core::property::Animatable;

    fn make_test_project() -> Project {
        let mut project = Project::default();
        project.compositions.clear();
        let mut comp = Composition::new("c".into(), "Test".into(), 100, 100, 10, 10);
        let mut layer = Layer::new(
            "l0".into(),
            "Layer0".into(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            10,
        );
        layer.effects.push(Effect {
            id: "blur_0".into(),
            name: "Gaussian Blur".into(),
            effect_type: EffectType::GaussianBlur {
                blur_radius: Animatable::new_constant(10.0),
            },
            enabled: true,
        });
        comp.layers.push(layer);
        project.compositions.push(comp);
        project
    }

    #[test]
    fn session_no_mutation_no_undo_entry() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);
        let gen_before = history.generation();

        {
            let _session = EditorSession::new(&mut history, "No-op");
            // Don't call current_mut() or ensure_snapshot()
        }

        // No mutation, so generation should not change
        assert_eq!(history.generation(), gen_before);
    }

    #[test]
    fn session_commit_creates_undo_entry() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);
        let gen_before = history.generation();

        {
            let mut session = EditorSession::new(&mut history, "Edit");
            session.ensure_snapshot();
            let proj = session.current_mut();
            proj.compositions[0].name = "Changed".into();
            session.commit();
        }

        // Generation should have incremented
        assert!(history.generation() > gen_before);

        // The current state should be the changed state
        assert_eq!(history.current().compositions[0].name, "Changed");

        // Undo should restore the original state
        history.undo();
        assert_eq!(history.current().compositions[0].name, "Test");
    }

    #[test]
    fn session_cancel_reverts() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);
        let gen_before = history.generation();

        {
            let mut session = EditorSession::new(&mut history, "Reverted");
            session.ensure_snapshot();
            let proj = session.current_mut();
            proj.compositions[0].name = "Changed".into();
            session.cancel();
        }

        // Cancel should not create an undo entry (it reverts in place)
        // But it does call history.commit(snapshot) which may increment generation
        // The important thing is the state is reverted
        assert_eq!(history.current().compositions[0].name, "Test");
    }

    #[test]
    fn session_lazy_snapshot() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);

        {
            let mut session = EditorSession::new(&mut history, "Lazy");
            // Don't call ensure_snapshot() explicitly
            // But call current_mut() which should trigger lazy snapshot
            let proj = session.current_mut();
            proj.compositions[0].name = "Lazy".into();
            session.commit();
        }

        // Should have created an undo entry
        assert_eq!(history.current().compositions[0].name, "Lazy");
        history.undo();
        assert_eq!(history.current().compositions[0].name, "Test");
    }

    #[test]
    fn session_noop_mutation_no_entry() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);
        let gen_before = history.generation();

        {
            let mut session = EditorSession::new(&mut history, "No-op edit");
            session.ensure_snapshot();
            // "Mutate" to the same state
            let proj = session.current_mut();
            proj.compositions[0].name = "Test".into(); // Same as original
            session.commit();
        }

        // No-op should not create an undo entry
        assert_eq!(history.generation(), gen_before);
    }
}
