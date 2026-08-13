# After Effects OSS — Team Architecture & Engineering Guidelines

## 1. Architectural Principles & Vision
This codebase follows a **Unidirectional Data Flow & Core-UI Separation** pattern designed for high-performance VFX software and team collaboration.

```
       ┌────────────────────────────────────────────────────────┐
       │                 Core Domain Engine                     │
       │  (Composition, Layer, Animatable<T>, Keyframe, Cache)  │
       └───────────────────────────▲────────────────────────────┘
                                   │ Immutable Read / Mutable Command
       ┌───────────────────────────┴────────────────────────────┐
       │             Unidirectional State Manager               │
       │       (app_state.rs: Playback, Selection, Export)      │
       └───────────────────────────▲────────────────────────────┘
                                   │ State Subscriptions
       ┌───────────────────────────┴────────────────────────────┐
       │                Modular UI View Panels                  │
       │   (37 Specialized Panels in src/ui/: timeline, etc)   │
       └────────────────────────────────────────────────────────┘
```

---

## 2. Team Collaboration & File Modularization Rules

### Rule 2.1: Small, Single-Responsibility UI Submodules
- **Never add monolithic multi-thousand line files.**
- Each UI panel must reside in its own specialized submodule under `src/ui/` (e.g., `src/ui/character_panel.rs`, `src/ui/tracker_panel.rs`, `src/ui/timeline/`).
- If a panel exceeds 500 lines, it **must** be broken down into sub-components (e.g., `timeline/header.rs`, `timeline/tracks.rs`, `timeline/graph_editor.rs`).

### Rule 2.2: Zero Git Conflict Protocol
- Developers working on feature PRs must only touch the specific panel module for their domain.
- Core engine changes in `src/core/` must be verified via pure unit tests in `cargo test` prior to submitting a Pull Request.

---

## 3. State Management & Ownership Guidelines

### Rule 3.1: Strict Domain Sub-State Structs (`src/app_state.rs`)
- **No God Objects**: Never add arbitrary flat fields to `AfterEffectsApp`.
- All application state must be placed in one of the domain sub-structs:
  - `PlaybackDomainState`: Frame counter, play/pause transport, work area bounds.
  - `SelectionDomainState`: Active layer index, multi-selection set, active property path.
  - `UiTabsDomainState`: Dock tabs, search filters, view ratios.
  - `ExportDomainState`: Non-blocking render channels, progress, export presets.

### Rule 3.2: Transactional In-Place Mutation & Lazy Commit Pattern
- UI panels read and mutate project state in-place using `app.history.current_mut()`.
- **Zero per-frame cloning**: Never clone the `Project` or `Composition` inside frame draw loops.
- History snapshots (`app.history.commit(...)`) are pushed **lazily** only on pointer release (`!is_pointer_down`) after actual property mutations.

---

## 4. UI vs. Core Business Logic Separation & Testing Strategy

### Rule 4.1: UI Functions are Pure Views
- Functions in `src/ui/` must take references (e.g. `&mut egui::Ui`, `&Composition`, `&mut SelectionDomainState`) and render controls.
- **Zero business logic in UI closures**: All calculations (keyframe evaluation, matrix transforms, expression parsing, color space conversions) must delegate to `src/core/`.

### Rule 4.2: 100% Automated Unit Test Requirement for Core Modules
- Every file in `src/core/` **must** include a `#[cfg(test)] mod tests` suite.
- Core tests must execute in under 1.0 second (`cargo test`) to ensure continuous integration (CI) speed.

---

## 5. Non-Blocking Async Threading & Panic Safety

### Rule 5.1: Non-Blocking Message Passing
- Background threads (FFmpeg renderers, tracker engines, Dynamic Link servers) communicate with the UI thread strictly via `std::sync::mpsc` channels polled with `.try_recv()`.
- **Never call `.recv()` or block the UI thread on a Mutex/Channel lock.**

### Rule 5.2: Panic Isolation
- All background thread closures must wrap execution in `std::panic::catch_unwind(std::panic::AssertUnwindSafe(...))`.
- Cancellation is coordinated via atomic flags (`Arc<AtomicBool>`).

---

## 6. Cross-Platform Shortcuts & Internationalization (i18n)

### Rule 6.1: OS-Aware Modifier Keys
- Always use `format_shortcut(key, cmd, shift, alt)` from `crate::ui::shortcuts` when displaying keybindings.
- Automatically resolves `Cmd` on macOS and `Ctrl` on Windows/Linux.

### Rule 6.2: Text Focus Protection
- Global shortcuts must query `crate::ui::focus::is_text_input_focused(ctx)` to prevent key leaks when typing into text boxes or dialog fields.

---

## 7. Zero-Unwrap Protocol & Defensive Error Handling

### Rule 7.1: Zero `.unwrap()` / `.expect()` in Production Code
- Calling `.unwrap()` or `.expect()` on runtime option/result values in production domain code (`src/core/` and `src/ui/`) is strictly prohibited.
- Use pattern matching (`if let Some(...)`, `match`), defensive fallback values (`.unwrap_or_default()`), or `Result<T, String>` propagation.

### Rule 7.2: Graceful Fault Recovery
- If invalid user JSON project state or corrupted assets are encountered during runtime playback, the app must log a warning via `log::warn!` or trigger a user notification toast rather than panicking.
