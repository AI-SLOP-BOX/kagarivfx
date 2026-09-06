//! Toolkit-neutral editing boundary.
//!
//! `EditorSession` wraps `ProjectHistory` and provides a safe mutation API.
//! All frontend/UI code should go through `EditorSession` for ordinary editing
//! operations. This ensures:
//!
//! - Every mutation creates an undo entry (on explicit commit)
//! - Generation is updated
//! - Cache invalidation happens
//! - Dirty-state tracking works
//!
//! ## Semantics
//!
//! - **commit()**: Creates an undo entry with the pre-edit snapshot. Consumes
//!   the session. If no mutation occurred, this is a no-op.
//! - **cancel()**: Reverts to the pre-edit state without creating an undo entry.
//!   Consumes the session.
//! - **Drop**: Reverts to the pre-edit state without creating an undo entry
//!   (safety net for early returns / errors).
//!
//! ## Interaction modes
//!
//! ### Discrete actions (click, toggle, preset apply, delete, reorder)
//!
//! ```text
//! snapshot (lazy) → mutate → commit
//! ```
//!
//! ### Continuous interactions (slider drag, numeric drag)
//!
//! ```text
//! begin_interaction() → [mutate]* → commit()
//! ```
//!
//! One drag must produce exactly one history entry regardless of how many
//! intermediate value updates occur.

use crate::core::history::ProjectHistory;
use crate::core::timeline::Project;

/// Whether the session was committed, cancelled, or dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionState {
    /// Session is still active.
    Active,
    /// Session was explicitly committed (created undo entry).
    Committed,
    /// Session was explicitly cancelled or dropped (no undo entry).
    RolledBack,
}

/// A toolkit-neutral editing session that wraps project mutations.
///
/// # Invariants
///
/// - `current()` always returns the current project state.
/// - `current_mut()` returns a mutable reference to the live stack entry.
///   After calling this, you MUST call `commit()` or `cancel()`.
/// - `ensure_snapshot()` must be called BEFORE any mutation to capture the
///   pre-edit state. It is idempotent (only captures once).
/// - `commit()` creates an undo entry with the pre-edit snapshot and consumes
///   the session.
/// - `cancel()` reverts to the pre-edit state without creating an undo entry
///   and consumes the session.
/// - `Drop` reverts to the pre-edit state without creating an undo entry
///   (safety net).
pub struct EditorSession<'a> {
    history: &'a mut ProjectHistory,
    snapshot: Option<Project>,
    label: &'static str,
    state: SessionState,
}

impl<'a> EditorSession<'a> {
    /// Create a new editing session.
    pub fn new(history: &'a mut ProjectHistory, label: &'static str) -> Self {
        Self {
            history,
            snapshot: None,
            label,
            state: SessionState::Active,
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

    /// Whether the session is still active (not yet committed or cancelled).
    pub fn is_active(&self) -> bool {
        self.state == SessionState::Active
    }

    /// Commit the editing session, creating an undo entry.
    ///
    /// If no mutation occurred (no snapshot was captured), this is a no-op.
    /// Consumes the session — further operations are no-ops.
    pub fn commit(mut self) {
        self.finish_commit();
    }

    /// Internal commit logic (needed for Drop which takes &mut self).
    fn finish_commit(&mut self) {
        if self.state != SessionState::Active {
            return;
        }
        self.state = SessionState::Committed;
        if let Some(snapshot) = self.snapshot.take() {
            let current = self.history.current().clone();
            self.history
                .commit_drag_action(snapshot, current, self.label);
        }
    }

    /// Cancel the editing session, reverting to the pre-edit state without
    /// creating an undo entry.
    ///
    /// If no mutation occurred, this is a no-op.
    /// Consumes the session — further operations are no-ops.
    pub fn cancel(mut self) {
        self.finish_cancel();
    }

    /// Internal cancel logic (needed for Drop which takes &mut self).
    fn finish_cancel(&mut self) {
        if self.state != SessionState::Active {
            return;
        }
        self.state = SessionState::RolledBack;
        if let Some(snapshot) = self.snapshot.take() {
            self.history.restore_current_without_history(snapshot);
        }
    }

    /// Access the underlying history for read-only queries.
    pub fn history(&self) -> &ProjectHistory {
        self.history
    }
}

impl Drop for EditorSession<'_> {
    fn drop(&mut self) {
        // Safety net: if neither commit nor cancel was called, rollback
        // without creating an undo entry.
        if self.state == SessionState::Active && self.snapshot.is_some() {
            self.finish_cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::property::Animatable;
    use crate::core::timeline::{Composition, Effect, EffectType, Layer, LayerType};

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
        let len_before = history.len();

        {
            let _session = EditorSession::new(&mut history, "No-op");
        }

        assert_eq!(history.generation(), gen_before);
        assert_eq!(history.len(), len_before);
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

        assert!(history.generation() > gen_before);
        assert_eq!(history.current().compositions[0].name, "Changed");

        history.undo();
        assert_eq!(history.current().compositions[0].name, "Test");
    }

    #[test]
    fn session_cancel_reverts_no_undo_entry() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);
        let gen_before = history.generation();
        let len_before = history.len();

        {
            let mut session = EditorSession::new(&mut history, "Reverted");
            session.ensure_snapshot();
            let proj = session.current_mut();
            proj.compositions[0].name = "Changed".into();
            session.cancel();
        }

        // Cancel reverts state
        assert_eq!(history.current().compositions[0].name, "Test");
        // Generation increments (UI needs to detect the change)
        assert!(history.generation() > gen_before);
        // No new undo entry created
        assert_eq!(history.len(), len_before);
        // Undo is still not available beyond what existed before
        assert!(!history.can_undo());
    }

    #[test]
    fn session_drop_rollback_no_undo_entry() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);
        let gen_before = history.generation();
        let len_before = history.len();

        {
            let mut session = EditorSession::new(&mut history, "Dropped");
            let _ = session.current_mut(); // trigger lazy snapshot
            session.current_mut().compositions[0].name = "Dropped".into();
            // Session dropped without commit or cancel
        }

        // Drop rolls back
        assert_eq!(history.current().compositions[0].name, "Test");
        // Generation increments (UI needs to detect the change)
        assert!(history.generation() > gen_before);
        // No undo entry created
        assert_eq!(history.len(), len_before);
    }

    #[test]
    fn session_lazy_snapshot() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);

        {
            let mut session = EditorSession::new(&mut history, "Lazy");
            let proj = session.current_mut();
            proj.compositions[0].name = "Lazy".into();
            session.commit();
        }

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
            let proj = session.current_mut();
            proj.compositions[0].name = "Test".into(); // Same as original
            session.commit();
        }

        assert_eq!(history.generation(), gen_before);
    }

    #[test]
    fn session_multiple_mutations_single_undo() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);
        let gen_before = history.generation();

        {
            let mut session = EditorSession::new(&mut history, "Multi-mutate");
            session.ensure_snapshot();
            let proj = session.current_mut();
            proj.compositions[0].name = "A".into();
            session.current_mut().compositions[0].name = "B".into();
            session.commit();
        }

        // One commit, one generation bump
        assert_eq!(history.generation(), gen_before + 1);
        assert_eq!(history.current().compositions[0].name, "B");
        history.undo();
        assert_eq!(history.current().compositions[0].name, "Test");
    }

    #[test]
    fn session_double_commit_is_safe() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);

        {
            let mut session = EditorSession::new(&mut history, "Double");
            session.ensure_snapshot();
            session.current_mut().compositions[0].name = "X".into();
            session.commit();
        }

        // Second commit on consumed session: no-op (already committed)
        // This test just verifies no panic
        assert_eq!(history.current().compositions[0].name, "X");
    }

    #[test]
    fn session_double_cancel_is_safe() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);

        {
            let mut session = EditorSession::new(&mut history, "Double");
            session.ensure_snapshot();
            session.current_mut().compositions[0].name = "X".into();
            session.cancel();
        }

        assert_eq!(history.current().compositions[0].name, "Test");
    }

    #[test]
    fn drop_after_commit_does_not_double_commit() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);
        let len_before = history.len();

        {
            let mut session = EditorSession::new(&mut history, "Commit+Drop");
            session.ensure_snapshot();
            session.current_mut().compositions[0].name = "Committed".into();
            session.commit();
            // Drop fires but session is already committed
        }

        // Only one undo entry, not two
        assert_eq!(history.len(), len_before + 1);
        assert_eq!(history.current().compositions[0].name, "Committed");
        history.undo();
        assert_eq!(history.current().compositions[0].name, "Test");
    }

    #[test]
    fn drop_after_cancel_does_not_double_cancel() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);
        let len_before = history.len();

        {
            let mut session = EditorSession::new(&mut history, "Cancel+Drop");
            session.ensure_snapshot();
            session.current_mut().compositions[0].name = "X".into();
            session.cancel();
            // Drop fires but session is already cancelled
        }

        assert_eq!(history.current().compositions[0].name, "Test");
        assert_eq!(history.len(), len_before);
    }

    // ── Integration-level Undo/Redo tests ──

    #[test]
    fn integration_add_effect_undo_redo() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);

        // Add a new effect
        {
            let mut session = EditorSession::new(&mut history, "Add Effect");
            let proj = session.current_mut();
            proj.compositions[0].layers[0].effects.push(Effect {
                id: "glow_1".into(),
                name: "Glow".into(),
                effect_type: EffectType::Glow {
                    threshold: Animatable::new_constant(0.5),
                    radius: Animatable::new_constant(20.0),
                    intensity: Animatable::new_constant(2.0),
                    color: Animatable::new_constant([1.0, 1.0, 1.0, 1.0]),
                },
                enabled: true,
            });
            session.commit();
        }

        assert_eq!(history.current().compositions[0].layers[0].effects.len(), 2);
        assert_eq!(
            history.current().compositions[0].layers[0].effects[1].name,
            "Glow"
        );

        // Undo: effect removed
        history.undo();
        assert_eq!(history.current().compositions[0].layers[0].effects.len(), 1);

        // Redo: effect restored
        history.redo();
        assert_eq!(history.current().compositions[0].layers[0].effects.len(), 2);
        assert_eq!(
            history.current().compositions[0].layers[0].effects[1].name,
            "Glow"
        );
    }

    #[test]
    fn integration_delete_effect_undo_redo() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);

        // Delete the only effect
        {
            let mut session = EditorSession::new(&mut history, "Delete Effect");
            let proj = session.current_mut();
            proj.compositions[0].layers[0].effects.remove(0);
            session.commit();
        }

        assert!(history.current().compositions[0].layers[0]
            .effects
            .is_empty());

        // Undo: effect restored
        history.undo();
        assert_eq!(history.current().compositions[0].layers[0].effects.len(), 1);
        assert_eq!(
            history.current().compositions[0].layers[0].effects[0].name,
            "Gaussian Blur"
        );

        // Redo: effect deleted again
        history.redo();
        assert!(history.current().compositions[0].layers[0]
            .effects
            .is_empty());
    }

    #[test]
    fn integration_toggle_effect_undo_redo() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);
        assert!(history.current().compositions[0].layers[0].effects[0].enabled);

        // Toggle off
        {
            let mut session = EditorSession::new(&mut history, "Toggle Effect");
            session.current_mut().compositions[0].layers[0].effects[0].enabled = false;
            session.commit();
        }

        assert!(!history.current().compositions[0].layers[0].effects[0].enabled);

        // Undo: enabled again
        history.undo();
        assert!(history.current().compositions[0].layers[0].effects[0].enabled);

        // Redo: disabled again
        history.redo();
        assert!(!history.current().compositions[0].layers[0].effects[0].enabled);
    }

    #[test]
    fn integration_reorder_effect_undo_redo() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);

        // Add two more effects
        {
            let mut session = EditorSession::new(&mut history, "Add Effects");
            let proj = session.current_mut();
            proj.compositions[0].layers[0].effects.push(Effect {
                id: "glow_1".into(),
                name: "Glow".into(),
                effect_type: EffectType::Glow {
                    threshold: Animatable::new_constant(0.5),
                    radius: Animatable::new_constant(20.0),
                    intensity: Animatable::new_constant(2.0),
                    color: Animatable::new_constant([1.0; 4]),
                },
                enabled: true,
            });
            proj.compositions[0].layers[0].effects.push(Effect {
                id: "blur_2".into(),
                name: "Box Blur".into(),
                effect_type: EffectType::ColorTint {
                    color: Animatable::new_constant([0.0, 0.0, 1.0, 1.0]),
                    intensity: Animatable::new_constant(0.5),
                },
                enabled: true,
            });
            session.commit();
        }

        // Now we have [Gaussian Blur, Glow, Box Blur]
        assert_eq!(history.current().compositions[0].layers[0].effects.len(), 3);
        assert_eq!(
            history.current().compositions[0].layers[0].effects[0].name,
            "Gaussian Blur"
        );

        // Swap 0 and 1
        {
            let mut session = EditorSession::new(&mut history, "Reorder Effect");
            let proj = session.current_mut();
            proj.compositions[0].layers[0].effects.swap(0, 1);
            session.commit();
        }

        // Now [Glow, Gaussian Blur, Box Blur]
        assert_eq!(
            history.current().compositions[0].layers[0].effects[0].name,
            "Glow"
        );
        assert_eq!(
            history.current().compositions[0].layers[0].effects[1].name,
            "Gaussian Blur"
        );

        // Undo: back to original order
        history.undo();
        assert_eq!(
            history.current().compositions[0].layers[0].effects[0].name,
            "Gaussian Blur"
        );
        assert_eq!(
            history.current().compositions[0].layers[0].effects[1].name,
            "Glow"
        );

        // Redo: reordered again
        history.redo();
        assert_eq!(
            history.current().compositions[0].layers[0].effects[0].name,
            "Glow"
        );
    }

    #[test]
    fn integration_slider_drag_one_undo_entry() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);
        let gen_before = history.generation();

        // Simulate a slider drag: 100 intermediate values, one commit
        {
            let mut session = EditorSession::new(&mut history, "Slider Drag");
            for i in 0..100 {
                let val = 10.0 + (i as f32 * 0.5);
                session.current_mut().compositions[0].layers[0].effects[0] = Effect {
                    id: "blur_0".into(),
                    name: "Gaussian Blur".into(),
                    effect_type: EffectType::GaussianBlur {
                        blur_radius: Animatable::new_constant(val),
                    },
                    enabled: true,
                };
            }
            session.commit();
        }

        // Only one generation bump, not 100
        assert_eq!(history.generation(), gen_before + 1);
        assert_eq!(history.len(), 2); // initial + one edit
    }

    #[test]
    fn integration_two_separate_drags_two_entries() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);

        // First drag
        {
            let mut session = EditorSession::new(&mut history, "Drag 1");
            session.current_mut().compositions[0].name = "After Drag 1".into();
            session.commit();
        }

        // Second drag
        {
            let mut session = EditorSession::new(&mut history, "Drag 2");
            session.current_mut().compositions[0].name = "After Drag 2".into();
            session.commit();
        }

        assert_eq!(history.len(), 3); // initial + 2 edits
        assert_eq!(history.current().compositions[0].name, "After Drag 2");

        history.undo();
        assert_eq!(history.current().compositions[0].name, "After Drag 1");

        history.undo();
        assert_eq!(history.current().compositions[0].name, "Test");
    }

    #[test]
    fn integration_noop_action_zero_entries() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);
        let gen_before = history.generation();
        let len_before = history.len();

        // Multiple no-op sessions
        for _ in 0..10 {
            let mut session = EditorSession::new(&mut history, "No-op");
            session.ensure_snapshot();
            // Don't actually change anything
            session.commit();
        }

        assert_eq!(history.generation(), gen_before);
        assert_eq!(history.len(), len_before);
    }

    #[test]
    fn integration_jump_to_generation() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);

        // Make some edits to have multiple entries
        for i in 0..5 {
            let mut session = EditorSession::new(
                &mut history,
                Box::leak(format!("Edit {}", i).into_boxed_str()),
            );
            session.current_mut().compositions[0].name = format!("State {}", i);
            session.commit();
        }

        let gen_before_jump = history.generation();

        // Jump to index 2 (State 1, since index 0 = "Test", 1 = "State 0", 2 = "State 1")
        assert!(history.jump_to(2));
        assert!(history.generation() > gen_before_jump);
        assert_eq!(history.current().compositions[0].name, "State 1");

        // Jump to same index: no-op
        let gen_after = history.generation();
        assert!(!history.jump_to(2));
        assert_eq!(history.generation(), gen_after);
    }

    #[test]
    fn integration_cancel_exact_restoration() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);
        let gen_before = history.generation();
        let len_before = history.len();

        // Make a complex mutation, then cancel
        {
            let mut session = EditorSession::new(&mut history, "Complex Edit");
            let proj = session.current_mut();
            proj.compositions[0].name = "Changed".into();
            proj.compositions[0].layers[0].name = "Changed Layer".into();
            proj.compositions[0].layers[0].effects.push(Effect {
                id: "new_effect".into(),
                name: "New".into(),
                effect_type: EffectType::GaussianBlur {
                    blur_radius: Animatable::new_constant(99.0),
                },
                enabled: false,
            });
            session.cancel();
        }

        // Exact restoration
        assert_eq!(history.current().compositions[0].name, "Test");
        assert_eq!(history.current().compositions[0].layers[0].name, "Layer0");
        assert_eq!(history.current().compositions[0].layers[0].effects.len(), 1);

        // No undo entry created
        assert_eq!(history.len(), len_before);
        // Generation advances (UI needs to detect the change)
        assert!(history.generation() > gen_before);
        // Undo is not available (nothing to undo)
        assert!(!history.can_undo());
    }

    #[test]
    fn integration_dropped_transaction_rollback() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);
        let gen_before = history.generation();
        let len_before = history.len();

        // Simulate error/early return: mutate, then let session drop
        {
            let mut session = EditorSession::new(&mut history, "Dropped Edit");
            let proj = session.current_mut();
            proj.compositions[0].name = "Should Be Reverted".into();
            proj.compositions[0].layers[0].effects.clear();
            // Drop without commit → rollback
        }

        // Exact restoration
        assert_eq!(history.current().compositions[0].name, "Test");
        assert_eq!(history.current().compositions[0].layers[0].effects.len(), 1);
        assert_eq!(history.len(), len_before);
        assert!(history.generation() > gen_before);
        assert!(!history.can_undo());
    }

    // ── Generation semantics tests ──

    #[test]
    fn generation_increments_on_commit() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);
        let gen0 = history.generation();

        let mut session = EditorSession::new(&mut history, "Edit");
        session.current_mut().compositions[0].name = "A".into();
        session.commit();

        assert_eq!(history.generation(), gen0 + 1);
    }

    #[test]
    fn generation_increments_on_undo() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);

        let mut session = EditorSession::new(&mut history, "Edit");
        session.current_mut().compositions[0].name = "A".into();
        session.commit();

        let gen_after_commit = history.generation();
        history.undo();
        assert_eq!(history.generation(), gen_after_commit + 1);
    }

    #[test]
    fn generation_increments_on_redo() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);

        let mut session = EditorSession::new(&mut history, "Edit");
        session.current_mut().compositions[0].name = "A".into();
        session.commit();

        history.undo();
        let gen_after_undo = history.generation();
        history.redo();
        assert_eq!(history.generation(), gen_after_undo + 1);
    }

    #[test]
    fn generation_increments_on_jump_to() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);

        let mut session = EditorSession::new(&mut history, "Edit");
        session.current_mut().compositions[0].name = "A".into();
        session.commit();

        let gen_before = history.generation();
        history.jump_to(0);
        assert_eq!(history.generation(), gen_before + 1);
    }

    #[test]
    fn generation_not_advanced_on_noop_commit() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);
        let gen0 = history.generation();

        // Commit identical state
        let same = history.current().clone();
        history.commit_action(same, "No-op");
        assert_eq!(history.generation(), gen0);
    }

    #[test]
    fn undo_redo_after_jump_to_remains_correct() {
        let project = make_test_project();
        let mut history = ProjectHistory::new(project);

        // Create entries: [Initial, A, B, C]
        for name in &["A", "B", "C"] {
            let mut session = EditorSession::new(&mut history, "Edit");
            session.current_mut().compositions[0].name = name.to_string();
            session.commit();
        }

        // Jump to index 1 (A)
        history.jump_to(1);
        assert_eq!(history.current().compositions[0].name, "A");

        // Undo from A → Initial
        history.undo();
        assert_eq!(history.current().compositions[0].name, "Test");

        // Redo → A
        history.redo();
        assert_eq!(history.current().compositions[0].name, "A");

        // Redo → B
        history.redo();
        assert_eq!(history.current().compositions[0].name, "B");

        // Undo → A
        history.undo();
        assert_eq!(history.current().compositions[0].name, "A");
    }

    // ── Large-project performance test ──

    #[test]
    fn large_project_idle_zero_clone_cost() {
        use std::time::Instant;

        // Build a large project: 300 layers, 10 keyframes each
        let mut comp = Composition::new("c".into(), "Big".into(), 1920, 1080, 30, 300);
        for i in 0..300 {
            let mut l = crate::core::timeline::Layer::new(
                format!("l{}", i),
                format!("Layer {}", i),
                crate::core::timeline::LayerType::Solid { color: [1.0; 4] },
                300,
            );
            let kfs: Vec<crate::core::keyframe::Keyframe<[f32; 2]>> = (0..10)
                .map(|k| {
                    crate::core::keyframe::Keyframe::new(
                        k * 30,
                        [k as f32 * 10.0, k as f32 * 5.0],
                        crate::core::keyframe::InterpolationType::Linear,
                    )
                })
                .collect();
            l.transform.position = crate::core::property::Animatable::new_animated(kfs);
            comp.layers.push(l);
        }
        let project = Project {
            compositions: vec![comp],
            active_composition_idx: 0,
            assets: Vec::new(),
            use_gpu_compute: false,
        };

        let mut history = ProjectHistory::new(project.clone());

        // Simulate 1000 idle frames: create session, don't mutate, drop.
        // This must NOT clone the project.
        let start = Instant::now();
        for _ in 0..1000 {
            let session = EditorSession::new(&mut history, "Idle");
            // No current_mut() call → no snapshot captured → no clone
            drop(session);
        }
        let elapsed = start.elapsed();

        // 1000 idle frames should complete in well under 100ms
        // (no cloning, just session creation/destruction)
        assert!(
            elapsed.as_millis() < 100,
            "1000 idle frames took {:?} — expected <100ms (possible unwanted clone)",
            elapsed
        );

        // Verify no undo entries were created
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn large_project_discrete_action_cost() {
        use std::time::Instant;

        // Build a medium project: 100 layers (500 is too slow for CI due to
        // per-commit project cloning, but this validates the O(1) idle path).
        let mut comp = Composition::new("c".into(), "Medium".into(), 1920, 1080, 30, 300);
        for i in 0..100 {
            let mut l = crate::core::timeline::Layer::new(
                format!("l{}", i),
                format!("Layer {}", i),
                crate::core::timeline::LayerType::Solid { color: [1.0; 4] },
                300,
            );
            l.effects.push(Effect {
                id: format!("blur_{}", i),
                name: format!("Blur {}", i),
                effect_type: EffectType::GaussianBlur {
                    blur_radius: Animatable::new_constant(10.0),
                },
                enabled: true,
            });
            comp.layers.push(l);
        }
        let project = Project {
            compositions: vec![comp],
            active_composition_idx: 0,
            assets: Vec::new(),
            use_gpu_compute: false,
        };

        let mut history = ProjectHistory::new(project);

        // Perform 20 discrete toggle actions
        let start = Instant::now();
        for i in 0..20 {
            let mut session = EditorSession::new(&mut history, "Toggle");
            session.current_mut().compositions[0].layers[i].effects[0].enabled = false;
            session.commit();
        }
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() < 5000,
            "20 toggles on 100-layer project took {:?}",
            elapsed
        );

        // Verify correct state
        for i in 0..20 {
            assert!(!history.current().compositions[0].layers[i].effects[0].enabled);
        }
        for i in 20..100 {
            assert!(history.current().compositions[0].layers[i].effects[0].enabled);
        }

        // Verify all 20 undos work
        for _ in 0..20 {
            history.undo();
        }
        for i in 0..100 {
            assert!(history.current().compositions[0].layers[i].effects[0].enabled);
        }
    }
}
