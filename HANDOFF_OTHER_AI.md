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

---

## Antigravity Session Update (Latest)

### New Features Added
| Feature | Location |
|---------|----------|
| Tool shortcuts V/H/Z/Y/Q/G/C, Cmd+T | `src/ui/shortcuts.rs` |
| Effect menu applies 17 effects | `src/ui/menu.rs` (`apply_effect_by_name`) |
| Layer bars use label colors | `src/ui/timeline/mod.rs` |
| Timecode ruler + overlap thinning | `src/ui/timeline/mod.rs` |
| Cmd+N new comp, Cmd+S save | shortcuts + menu |
| Pixel color sampling in status bar | `src/app_state.rs` |
| Draggable work-area handles | `src/ui/timeline/mod.rs` |
| Graph editor click-to-create KF | `src/ui/graph_editor.rs` |
| Motion path eased sampling | `src/ui/viewport_overlays.rs` |
| Render queue sequential FFmpeg batch | `src/ui/render_queue.rs`, `export_dialog.rs` (`start_comp_export`) |
| Duplicate/Split context menu fixes | `src/ui/timeline/mod.rs` (pending_* pattern) |
| U/UU/A reveal filtering works | `src/ui/timeline/mod.rs` |
| Time Stretch real impl | `src/ui/time_remap_panel.rs` |
| Keyframe right-click menu | `layers.rs` on_menu cb, `utils.rs` RightClicked |
| Shift+drag marquee select | `layers.rs` on_box_select cb |
| Selected-KF group move | `layers.rs` on_group_move cb |
| Viewport scale handles | `src/ui/viewport.rs` (`viewport_scale_drag`) |
| Sequence Layers assistant | NEW `src/ui/sequence_layers_dialog.rs` |
| Text dbl-click viewport edit | `viewport.rs` (`inline_text_edit_layer`) |
| Layer markers + Alt+M | `timeline.rs` Layer.markers |
| Hand pans / Zoom clicks | `src/ui/viewport.rs` |
| Save Frame As PNG | `src/ui/menu.rs` |
| Slip edit Alt+drag, ripple Shift+trim | `src/ui/timeline/mod.rs` |
| Import Image menu | `src/ui/menu.rs` |
| Q creates rectangle, T creates text | `src/ui/viewport.rs` drag_stopped |
| Ruler context menu | `src/ui/timeline/mod.rs` |
| PreComp dbl-click opens nested comp | `src/ui/timeline/mod.rs` |

### Coordination Notes
- **J/K/L semantics changed**: J=prev KF, K=next KF, L=play forward. Don't re-add shuttle J/K.
- **T key** = Opacity reveal; **Cmd+T** = Text tool. Bare T must not switch tools.
- **pending_* pattern**: timeline/mod.rs defers mutations out of the row loop (borrow-safe). Follow it for new row-context actions.
- **draw_prop_row_ext** now takes 5 optional callbacks: on_move, on_select, on_menu, on_box_select, on_group_move.
- If you add EffectType variants: register animatable params in `core/effect_params.rs` (`animatable_params()`) for timeline rows; update `apply_effect_by_name` in menu.rs for Effect menu.

### What Antigravity Completed (Session 2 — 2026-08-24)
**8 audit items completed + ~25 additional features/fixes (479 tests pass)**

**Viewport features:**
- Multi-layer group drag (shift-click → drag moves all selected)
- Scale corner handles (Selection tool)
- Checkerboard transparency grid (16px light/dark)
- Tool cursors: Hand→Grab, Selection→PointingHand, Rotation→ResizeHorizontal, AnchorPoint/Pen→Crosshair, Zoom→ZoomIn/Out
- Hand tool pans, Zoom tool click-zooms (were falling through to layer move)
- Double-click text layer opens inline editor
- Q tool creates rectangles, T tool creates text layers on viewport
- Pen tool draws mask vertices (G key), commits on Enter/dblclick

**Timeline features:**
- Duplicate Layer inserts clone (was no-op)
- Split Layer creates true tail layer (was just trim)
- U/UU/A reveal modes filter property rows
- Time Stretch rescales duration + keyframe times
- Layer bars use label colors (selected=full, unselected=dimmed)
- Timecode ruler (HH:MM:SS:FF) with overlap thinning
- Draggable work-area In/Out handles
- Playhead inverted triangle handle
- Ruler scrub cursor feedback
- Alt+drag slip edit on layer bars
- Shift+drag Out-point ripple edit
- Layer bar vertical drag reorders in stack (new this session)
- Layer markers (Alt+M)
- Double-click PreComp opens nested composition
- Ruler right-click menu (Zoom to WA, Set Duration, Reset WA)
- Right-click keyframe context menu (Linear/Easy Ease/Hold/Reverse/Delete)
- Shift+drag marquee box-select for keyframes
- Selected keyframes move together (group drag)

**Other UI:**
- Inspector shows composition dimensions + frame rate (📐 WxH ⏱ fps)
- Transport panel shows current timecode + frame number
- Sequence Layers keyframe assistant dialog
- Effect menu applies 17+ effects to selected layer
- Graph editor click-to-create keyframes
- Motion path eased sampling + playhead dot
- Edit > Duplicate menu now works (was stub toast)
- Layer > Light creation
- File > Import Image
- Composition > Save Frame As PNG
- Cmd+N new composition, Cmd+S overwrite save
- Render queue sequential batch FFmpeg export
- Keyframe navigation: J/K/L, T reveals Opacity
- Shortcuts dialog + command palette updated

**Core infrastructure:**
- `core/effect_params.rs` — generic param reflection for ALL EffectType variants (timeline rows auto-generated from `animatable_params()` method; future-proof catch-all)
- `core/mask.rs` — MaskPath, MaskVertex, MaskMode structs for pen tool masks
- Status bar pixel color sampling from frame cache

**Files the other AI should NOT touch (Antigravity-owned):**
- `src/ui/shortcuts.rs`, `src/ui/menu.rs`, `src/ui/viewport.rs`, `src/ui/viewport_overlays.rs`
- `src/ui/timeline/` (all files), `src/ui/graph_editor.rs`
- `src/ui/transport_panel.rs`, `src/ui/inspector.rs`, `src/ui/inspector_layer.rs`
- `src/ui/render_queue.rs`, `src/ui/export_dialog.rs`, `src/ui/shortcuts_dialog.rs`
- `src/ui/command_palette.rs`, `src/ui/sequence_layers_dialog.rs`
- `src/app_state.rs`, `src/core/mask.rs`, `src/core/effect_params.rs`

### Remaining Gaps (Priority Order)
1. **GPU mask compositing** — shader.wgsl/renderer.rs (your zone); masks are CPU-only; viewport shows HUD warning
2. **Spatial bezier handles** on motion path (AE shows control points on the path overlay)
3. **Keyframe value numeric editing** from graph editor (click → popup to type exact value) ✅ done (4fc5b75)
4. **Extend slip/ripple to effect keyframes** (currently only transform tracks) ✅ done (1fa7278)
5. **Effect timeline rows** for remaining ~60 unregistered EffectType variants in effect_params.rs

---

## Session Update (2026-08-24, ox-alpha): 2 more orphaned modules wired end-to-end → commit `3d17136`

### Corner Pin (`src/core/corner_pin.rs` was orphaned)
- `EffectType::CornerPin { top_left, top_right, bottom_right, bottom_left }` (all `Animatable<[f32; 2]>`, layer-pixel space)
- CPU dispatch in `cpu_effects::apply_one` → visible in preview/export/CPU fallback automatically
- Keyframeable rows registered in `effect_params.rs`; GPU display-name + id `"corner_pin"` in `effect_plugin.rs`
- UI: Distort menu entry (defaults to comp size), effects library preset "+ Corner Pin", X/Y drag pin editors in Effect Controls
- Tests: content-shift, degenerate-quad passthrough, determinism

### Rove Across Time (`src/core/spatial_keyframe.rs` was orphaned)
- Command Palette → "Keyframe Assistant: Rove Across Time (Position)"
- Slides interior position keyframes along time for constant path velocity (AE semantics); one undo unit via `modify_project()`

### Housekeeping
- Fixed pre-existing clippy warnings (unused_parens / identity_op) in cpu_effects tests → clippy --all-features is at zero again
- Full suite: 482 passed / 0 failed at commit time
- Still orphaned (need software_renderer/compositor integration, currently your dirty zone): echo_effect, set_matte, difference_matte, light_transmission, frame_blending, shape_modifiers, stroke_modifier, typography_engine, vfx_graph_compiler

## Session: GPU Layer Mask Compositing (shader.wgsl + renderer.rs)

### What shipped
- Masks now render on the **GPU viewport path** for the common cases:
  - `@group(3)` dedicated mask texture + sampler (`t_mask`/`s_mask`), pipeline layout has 4 bind groups
  - Per-layer mask flags moved from Globals → **LayerUniform** (`mask_enabled/mode/inverted/feather`); padding 10→9 keeps size multiple of 256 for dynamic offsets
  - CPU rasterizer in renderer.rs: even-odd scanline fill (`rasterize_polygon_evenodd`) + AE mask-mode combine (`combine_mask_shapes`: Add/Lighten=over, Subtract, Intersect/Darken=min, Difference=XOR). First-mask Subtract starts from full frame; inverted Add carries its complement directly
  - `rasterize_layer_masks()` evaluates `path.to_polygon(frame,12)` per enabled mask, scales comp→effective preview res, packs RGBA8 (white, alpha=coverage)
  - Single distinct raster per frame → uploaded once and shared by all masked draws; >1 distinct rasters fall back to unmasked GPU draw that frame (single-submit upload ordering constraint) with a log::debug
  - Shader softens via smoothstep feather approximation on the alpha ramp

### Gotchas for your sessions
- `texture_bind_group_layout` (group 2) is back to 2 entries — text/video bind groups unchanged and valid again
- Don't add fields to `LayerUniform` without mirroring byte order in shader `struct Layer` AND adjusting `_padding_align`
- 8 new tests in `renderer.rs::gpu_mask_tests`; full suite 505 passed / clippy zero at commit
- Also relocated `start_png_sequence_export` above `mod tests` in ffmpeg_export.rs (clippy items_after_test_module)

## Session: GPU masks v2 — submit-splitting + real feather

### What shipped (renderer.rs, viewport_overlays.rs)
- **Multi-mask correctness**: render_internal now builds contiguous runs by mask-raster key and issues ONE SUBMIT PER RUN (first run LoadOp::Clear(bg), later runs LoadOp::Load). Each distinct raster is uploaded just before its own submit → painter order preserved across submits, no more "unmasked fallback" when layers carry different masks
- **Real feather**: Felzenszwalb–Huttenlocher EDT (`edt_1d`/`edt_2d`) computes exact Euclidean distance to polygon boundary; alpha ramps linearly across `feather` px centered on the edge (saturates at ±feather/2 inside/outside). Baked per-shape into coverage BEFORE mode combine & opacity; `MaskRaster.feather` now always 0 (shader-side smoothstep approx is dead code path)
- HUD CPU-only notice: masks removed from reasons list entirely (was ">1 distinct" condition) — GPU now composites any mask count
- Gotcha for EDT users: cast to f64 BEFORE subtracting site indices in the query loop (`q as f64 - v[k] as f64`) — all-INF rows can leave envelope sites past q and usize underflow panics in debug
- Tests: gpu_mask_tests grew to 11 (+3 feather/EDT). Suite note: 2 software_renderer tests fail on the OTHER AI's in-flight dirty file — unrelated to this change (module untouched)

### Still open
- Export/CPU renderer remains reference implementation (its mask feather semantics may differ slightly from the GPU ramp — compare visually someday)
- Orphan modules list unchanged (blocked on software_renderer.rs ownership)

## Session: Gap 2 closed — draggable position keyframe dots on motion path

### What shipped (viewport.rs, app_state.rs, viewport_overlays.rs) → commit `7551494`
- Selection-tool hit-testing (8px) on the motion-path keyframe dots; dragging moves that keyframe's VALUE at its own frame (spatial editing) without touching temporal bezier data
- Mirrors mask-vertex drag exactly: `begin_drag` once per gesture → direct `current_mut` edits during drag → single undo via `commit_drag()`
- New state `viewport_pos_kf_drag_state: Option<(layer, kf_frame, start_value, start_ptr)>`; wired into mid-drag repaint gate, `was_dragging`, and release-clears
- Dragged dot renders a playhead-colored ring in the overlay
- Priority: KF dot > mask vertex > corner-scale > whole-layer drag (all gated to Selection tool for dots)
- Note: `ae_effects_pack_v24::test_bars_cover_fraction` failed on your in-flight dirty tree at commit time (your file, untouched by me)
