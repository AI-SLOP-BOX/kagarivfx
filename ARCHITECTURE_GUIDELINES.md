# AE-OSS Architecture & Code Hardening Guidelines

This document outlines mandatory architectural conventions for team collaboration and codebase hardening.

---

## 1. Single Responsibility & File Size Caps

### Rule 1.1: 500-Line Soft Cap per File
- No single source file should exceed **500 lines**.
- When a panel or module grows beyond 500 lines, sub-features MUST be extracted into sub-modules (e.g. `inspector_camera.rs`, `effects_controls.rs`).

### Rule 1.2: Component Extraction
- UI panels are composed of modular subcomponents taking explicit slice references or domain structs (`&mut Layer`, `&mut Mask`), not global God Objects.

---

## 2. UI / Logic Separation & Domain Struct Encapsulation

### Rule 2.1: Domain Sub-State Structs
- Application state is partitioned into domain-specific structs (e.g., `PlaybackState`, `SelectionState`, `TimelineHeaderState`).
- Components receive only the minimal state references required for rendering.

### Rule 2.2: Pure Functions for Business Logic
- Keyframe evaluation, spatial transformation, and pixel compositing must be implemented as pure, testable functions in `src/core/`.

---

## 3. Repaint & Idle Power Management

### Rule 3.1: Throttled Repaints
- `ctx.request_repaint()` or `ctx.request_repaint_after(...)` MUST NOT be called unconditionally during idle state.
- Repaints are requested ONLY when:
  1. Playback is active (`is_playing == true`).
  2. Background exports or dynamic links are running.
  3. Interactive user dragging / scrubbing occurs.

---

## 4. Algorithmic Efficiency & Temporal Locality

### Rule 4.1: $O(1)$ Keyframe Locality
- Keyframe sampling across scrubbed timelines must utilize index-cached lookup (`evaluate_with_hint`) rather than $O(N)$ linear scans.

### Rule 4.2: Zero Per-Frame Heap Allocation
- Hot rendering loops must reuse pre-allocated memory buffers (`PixelBufferPool`).

---

## 5. Non-Blocking Threading & Panic Isolation

### Rule 5.1: Non-Blocking Channels
- Background threads communicate with the UI thread strictly via `std::sync::mpsc` channels polled with `.try_recv()`.
- Never call `.recv()` or block the UI thread on a Mutex/Channel lock.

### Rule 5.2: Panic Isolation
- All background thread closures wrap execution in `std::panic::catch_unwind(std::panic::AssertUnwindSafe(...))`.

---

## 6. Cross-Platform Shortcuts & Internationalization (i18n)

### Rule 6.1: OS-Aware Modifier Keys
- Always use `format_shortcut(key, cmd, shift, alt)` from `crate::ui::shortcuts`.

### Rule 6.2: Text Focus Protection
- Global shortcuts query `crate::ui::focus::is_text_input_focused(ctx)` before execution.

---

## 7. Zero-Unwrap Protocol & Defensive Error Handling

### Rule 7.1: Zero `.unwrap()` / `.expect()` in Production Code
- Calling `.unwrap()` or `.expect()` in `src/core/` and `src/ui/` is prohibited.
- Use pattern matching (`if let Some(...)`), defensive fallbacks (`.unwrap_or(...)`), or `Result` propagation.

---

## 8. Borrow Checker Safety & Data-Driven Component Catalog

### Rule 8.1: Scope Isolation for Mutability (`history.current_mut()`)
- When modifying project state via `app.history.current_mut()`, restrict mutable borrow lifetimes to isolated block scopes `{ ... }` or copy primitive values/IDs beforehand.
- Avoid holding `current_mut()` references while calling methods on other `app` fields to eliminate double-borrow collisions.

### Rule 8.2: Data-Driven Preset Registries
- UI element lists (such as effect presets, plugin panels, tool selections) must be declared as static data registries (`EffectPreset` array) and rendered dynamically via iterator loops.
