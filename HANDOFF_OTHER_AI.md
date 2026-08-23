# Handoff: Antigravity → Other AI (2026-08-23)

## What Antigravity Just Completed

### 1. 3D Material Options (Phong Shading)
**Files:**
- `src/core/timeline.rs` — `MaterialOptions` struct (lines 630-662), added to `Layer.material` field (line 975)
- `src/core/software_renderer.rs` — Phong shading with specular highlights (lines 1125-1160)
- `src/ui/inspector_layer.rs` — Material UI sliders (Ambient/Diffuse/Specular/SpecularExp/Emission/Metalness) in the 3D Spatial Transform section

**What changed:** The old 3D lighting was hardcoded Lambertian with 0.35 ambient. Now it uses the layer's `MaterialOptions` for full Phong shading: ambient + diffuse + Blinn-Phong specular + emission. The UI shows 6 sliders when a layer is toggled to 3D.

**Testing:** All 363 tests pass. Test by toggling a layer to 3D, adding a Point light, and adjusting material sliders.

---

### 2. Inline Expression Editor
**Files:**
- `src/ui/inspector_property.rs` — `draw_expression_selector` rewritten (lines 149-230)

**What changed:** The expression selector now has:
- "Custom Script..." option in the combo box
- Inline `TextEdit::singleline` editor for Raw expressions
- Empty script warning indicator
- Script error detection

**Note:** The expression_panel.rs (full multiline editor) still exists separately in the right panel. The inline editor is for quick edits in the inspector.

---

### 3. Audio Mixer Mute/Solo
**Files:**
- `src/app_state.rs` — `MixerChannel` struct (lines 8-27), `audio_mixer_channels: Vec<MixerChannel>` (line 270)
- `src/core/audio_engine.rs` — `mix_audio_sources_for_frame` accepts `Option<&[MixerChannel]>` (line 349), applies mute/solo logic (lines 387-398)
- `src/ui/audio_mixer.rs` — M (Mute, red) and S (Solo, yellow) buttons per track (lines 38-58)

**What changed:** Each mixer channel now has `gain_db`, `pan`, `mute`, `solo` fields. M button mutes the track (red highlight). S button solos (yellow highlight). Solo logic: if any channel is soloed, only soloed channels are heard.

**Testing:** All 363 tests pass.

---

## What Antigravity Just Completed (Session 2)

### 4. Drag-to-Apply Effects from Library
**Files:**
- `src/app_state.rs` — `dragging_effect: Option<(String, usize)>` field (line 189)
- `src/ui/effects_library.rs` — Draggable effect buttons with ⠿ prefix, visual drag feedback, drop zone in Effect Controls panel (lines 270-310, 72-103)
- `src/ui/timeline/mod.rs` — Drop zone on layer rows with blue glow indicator, deferred application to avoid borrow conflicts (lines 666-694, 1110-1126)

**What changed:** Effect buttons in "Effects & Presets" are now draggable. When dragged:
- Button shows blue highlight while dragging
- Timeline layer rows show blue glow + border when hovered
- Effect Controls panel shows a drop zone
- On drop, effect is applied to the target layer with a toast notification
- Click still works as fallback

**How it works:** Uses egui's `drag_started()` / `dragged()` / `drag_stopped()` on buttons. Drop zones use `ui.rect_contains_pointer()` for hit testing. Effect application is deferred to after the layer loop to avoid borrow conflicts with `comp.layers`.

---

## What Antigravity Just Completed (Session 3)

### 5. Effect Preset Save/Load System
**Files:**
- `src/core/effect_presets.rs` — NEW: `EffectPreset` struct (JSON-serializable), save/load/discover functions (112 lines)
- `src/ui/effects_library.rs` — "Save as Preset" button per effect, "Load Preset" button, preset browser in Effect Controls panel

**What changed:** Users can now save any applied effect as a JSON preset and reload it later.
- **Save:** Each effect in the Effect Controls panel has a "💾 Save as Preset" button
- **Load:** "📂 Load Preset from File..." opens a file dialog for .json/.aevfx-preset files
- **Browser:** "📦 Saved Presets (N)" collapsible section lists all presets in `~/.aevfx/presets/`
- Presets include: effect name, type, all parameters, creation timestamp, optional category/description

**Preset format:** Standard JSON with `Effect` struct (already Serialize/Deserialize). File extension: `.aevfx-preset.json`.

### 6. Professional Dark Theme (AE Color Accuracy)
**Files:**
- `src/ui/theme.rs` — Complete rewrite with accurate AE CC 2024 color palette

**What changed:** UI now uses pixel-accurate AE colors instead of generic dark theme.
- **Backgrounds:** 7-tier depth system (BG_DEEPEST → BG_ELEVATED)
- **Borders:** Crisp 1px borders with 4-level hierarchy (SUBTLE/MEDIUM/STRONG/ACTIVE)
- **Typography:** Optimized font sizes for pro density (10px small, 11.5px body)
- **Widget states:** Distinct colors for inactive/hovered/active/open states
- **Helper functions:** `draw_separator()`, `draw_label_chip()`, `panel_frame()`, `side_panel_frame()` for consistent styling
- **Viewport overlay colors:** 20+ new color constants for grid, motion path, gizmos, HUD, FPS indicators
- **Panel resize handle:** Increased to 8px for better usability

**Panel borders:** Updated toolbar, viewport, status bar, render queue to use theme colors instead of hardcoded values.
**Hardcoded colors replaced:** 50+ instances across viewport_overlays.rs, timeline/mod.rs, timeline/utils.rs, graph_editor.rs, toolbar.rs, app_state.rs.

---

## What You Should Work On Next

### Priority 1: Camera DOF (Depth of Field)
**Status:** Camera3D struct already has `dof_enabled`, `dof_max_blur`, `dof_iris_sides` fields (timeline.rs lines 629-637). UI is in inspector_camera.rs (lines 37-67). **Missing: the actual DOF blur implementation in software_renderer.rs.**

**Implementation approach:**
1. During compositing, record each layer's Z depth in a depth buffer
2. After compositing, apply variable-radius blur based on distance from `cam.focus_distance`
3. Use `cam.aperture` to scale blur amount (higher f-number = less blur)
4. `dof_iris_sides` can shape the bokeh (circle=0, triangle=3, etc.)

### Priority 2: GPU Separable Gaussian Blur (2-pass)
**Status:** Current blur in shader.wgsl is single-pass 5-tap (line 189-202). This produces visible banding on large blur radii.

**Implementation approach:**
1. Add a second render pass: horizontal blur → intermediate texture → vertical blur
2. Use 13-tap or 9-tap separable kernel (much better quality than current 5-tap)
3. This is the single biggest visual quality improvement for the GPU path

### Priority 3: Shape Layer Hierarchy (AE Shape Groups)
**Status:** Shapes are flat (Rectangle/Ellipse/Star/Polygon). AE has Shape → Group → Path/Stroke/Fill hierarchy.

**This is a large architectural change.** Consider whether it's worth the effort vs focusing on other gaps.

### Priority 4: More UI Polish
- Effects Library: Make the 30+ stub tabs functional (at least show "Coming Soon" placeholders)
- Graph Editor: Add auto-zoom/fit-to-view, speed/value graph toggle
- Timeline: Add layer bar color gradient per label

---

## Known Issues / Gotchas

1. **frame_blending.rs** — You have uncommitted changes in Pixel Motion interpolation. Don't let Antigravity touch this file.
2. **Clippy** — Must be zero warnings before commit. Run `cargo clippy --all-features`.
3. **Tests** — Must all pass. Run `cargo test --all-features`.
4. **Default fonts** — NEVER remove `default_fonts` from eframe features.
5. **Deterministic rendering** — All renders must be byte-identical for same input. No pointer hashing.

---

## Files Modified This Session

| File | Change |
|------|--------|
| `src/core/timeline.rs` | Added `MaterialOptions` struct, `Layer.material` field, Camera3D DOF fields |
| `src/core/software_renderer.rs` | Phong shading with material properties |
| `src/core/audio_engine.rs` | MixerChannel-based mute/solo support |
| `src/ui/inspector_layer.rs` | Material options UI for 3D layers |
| `src/ui/inspector_property.rs` | Inline expression script editor |
| `src/ui/audio_mixer.rs` | Mute/Solo buttons, MixerChannel access |
| `src/app_state.rs` | MixerChannel struct, audio_mixer_channels type, dragging_effect field |
| `src/ui/effects_library.rs` | Drag-to-apply effect buttons, drop zone in Effect Controls, preset save/load UI |
| `src/ui/timeline/mod.rs` | Effect drop zone on layer rows, playhead/work area colors |
| `src/core/effect_presets.rs` | NEW: EffectPreset system (save/load/discover JSON presets) |
| `src/ui/theme.rs` | Complete AE color palette rewrite, helper functions, viewport overlay colors |
| `src/ui/viewport_overlays.rs` | Viewport overlay colors replaced (35+ instances) |
| `src/ui/timeline/utils.rs` | Keyframe tick colors replaced |
| `src/ui/graph_editor.rs` | Graph editor colors replaced |
| `src/ui/toolbar.rs` | Toolbar colors replaced |
