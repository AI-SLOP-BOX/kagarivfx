# After Effects OSS Alternative — AI Agent Instructions

## Project Overview
An open-source After Effects alternative built in Rust with egui/wgpu.
Goal: professional motion graphics + compositing tool with GPU acceleration.

## Current State (updated: this session)
- **262+ tests passing, clippy clean**
- **201+ commits** on main branch
- **eframe/egui 0.29** + **wgpu 22** — migrated from 0.27/0.19
- **CRITICAL**: `default_fonts` feature MUST be enabled in eframe Cargo.toml
  or NO text renders at all. This was the root cause of invisible text.

## Architecture

### Key Files (DO NOT BREAK)
```
src/main.rs              — Entry point, eframe setup, wgpu init
src/app_state.rs         — Central app state (AfterEffectsApp struct)
src/lib.rs               — Library root, module declarations
src/core/mod.rs          — Core module declarations

src/core/software_renderer.rs — CPU rendering pipeline (main compositor)
  - render_frame_to_pixels() = main entry point
  - Parallel layer data prep → sequential compositing
  - Dither applied only when grading active (exposure≠0 || lut_mode≠0)

src/core/renderer.rs     — GPU renderer (wgpu)
  - LayerUniform struct must match shader.wgsl layout EXACTLY
  - Text textures: cached by (layer_id, text, font_size, stroke_bits)
  - Video frame textures: cached by (layer_id, frames_dir, frame_idx)
  - RAM preview ring: bounded at 300 frames
  - Dirty-checking: skip re-render when inputs unchanged

src/core/shader.wgsl     — WGSL fragment/vertex shader
  - Layer types: 0=solid, 1=textured(image/video/text), 2=shape,
    3=text-rect(fallback), 5=precomp, 7=adjustment, 8=particle
  - Effects: glow, chromatic aberration, vignette, mesh warp, etc.

src/core/timeline.rs     — Data model (Composition, Layer, Keyframe, etc.)
  - Animatable<T> = Constant(T) | Animated(Vec<Keyframe<T>>)
  - InterpolationType::Bezier { outgoing, incoming, custom_bezier }
  - resolve_world_transform() handles parenting + expressions
  - MAX_PARENT_DEPTH=32, find_sub_comp depth-limited to prevent cycles
  - LoopProp enum for loopOut/loopIn in Raw scripts

src/core/keyframe.rs     — Keyframe system
  - EasePreset: 19 variants (Standard→Custom3)
  - solve_bezier_eased_time(): Newton-Raphson + binary search
  - subpixel_refine(): parabola fit for tracker precision

src/core/history.rs      — Undo/redo with 128MB byte budget
src/core/autosave.rs     — Crash recovery (rotating slots, atomic writes)
src/core/frame_cache.rs  — LRU cache, 512MB limit, monotonic LRU counter
```

### UI Files
```
src/ui/timeline/         — Timeline panel (mod.rs=main, layers.rs, utils.rs)
src/ui/viewport.rs       — Viewport (GPU render path + CPU fallback)
src/ui/inspector*.rs     — Property inspector panels
src/ui/effects_library.rs — Effect browser (categorized)
src/ui/audio_mixer.rs    — Audio mixer with live meters
src/ui/theme.rs          — Dark theme (configure_ae_theme called EVERY FRAME)
src/ui/icons.rs          — SVG icons + procedural logo
```

### Critical Gotchas (DO NOT REPEAT THESE MISTAKES)
1. **`default_fonts` feature**: Without `features=["default_fonts"]` on eframe,
   ALL text is invisible. This cost hours to debug.
2. **follow_system_theme**: eframe 0.28+ follows OS theme by default, overriding
   our dark theme with light mode. Fix: call configure_ae_theme(ctx) every frame.
3. **f32 Hash**: f32 doesn't implement Hash/Eq. Use `.to_bits()` for HashMap keys.
4. **wgpu version alignment**: eframe's internal wgpu MUST match our direct
   dependency version. Currently both use wgpu 22.x.
5. **Determinism**: Same input MUST produce byte-identical render output.
   Never use pointer addresses, timestamps, or thread-dependent values in
   pixel calculations.
6. **Layer compositing order**: Bottom-to-top z-order. Adjustment layers apply
   effects to everything below them.
7. **Precomp cycle detection**: MAX_PRECOMP_DEPTH=16 guard prevents infinite
   recursion from cyclic pre-comp references.
8. **Parent chain cycles**: resolve_world_transform has MAX_PARENT_DEPTH=32.
9. **Dithering**: Only applies when exposure_ev≠0 or lut_mode≠0. Ungraded
   pixels pass through losslessly (tests depend on exact values).
10. **GPU video textures**: Bounded at 200 frames (~830MB VRAM). FIFO eviction.

### Testing
```bash
cargo test --all-features          # Run all tests
cargo test test_name               # Run specific test
cargo clippy --all-features        # Must have 0 warnings
cargo test --test stress_tests     # Performance tests
cargo test --test fuzz_matrix_tests # Combinatorial/fuzz tests
cargo run --example gen_test_project && cargo run --features cli --bin aevfx -- validate --project test_project.json
```

### Build & Run
```bash
cargo build --features gui --bin aftereffects-oss
cargo run --features gui --bin aftereffects-oss
cargo run --features cli --bin aevfx -- render --project test_project.json --format mp4
```

### Feature Flags
```toml
[features]
gui = ["dep:eframe", "dep:wgpu", "dep:egui_extras", "dep:rfd", "dep:rodio"]
cli = ["dep:clap", "dep:png"]
wgpu = []  # sub-feature of gui
default = []
```

### Dependencies (critical versions)
- eframe/egui: 0.29.1
- wgpu: 22.1.0 (both direct and via eframe)
- rhai: 1.18 (expression engine)
- ab_glyph: 0.2 (font rasterizer)
- rayon: 1.8 (parallel processing)
- rodio: 0.19 (audio playback, gui-only)
- naga: 0.19 (WGSL validation, dev-dependency)

## Expression System
- `Expression::Raw(script)` evaluates via Rhai engine
- Context available: `thisComp.layer("Name").transform.position[0]`
- Loop functions: `loopOut("cycle")`, `loopOut("pingpong")`, `loopIn(...)`
- Token preprocessing replaces these with __loop_* scope variables
- CompSnapshot memoized per (version, frame, comp_ptr) — O(n²)→O(n)

## Audio System
- WAV loading: PCM 8/16/24-bit, chunk walking
- mix_audio_sources_for_frame(): multi-track mixing with gain/pan
- rodio sink synced to playhead (>120ms drift triggers seek)
- Waveform peaks cached in egui temp storage per layer

## Export Pipeline
- ffmpeg subprocess for MP4/GIF (checks is_ffmpeg_available())
- VideoCodec::H264 | ProRes422 | ProRes4444
- audio_wav muxed as second input with AAC encoding
- MLT XML export/import for Shotcut/Kdenlive interop
- Lottie export with shape geometry + keyframes

## Robustness Guarantees
- All renders are deterministic (same input = same bytes)
- No panics from any input (fuzz-tested with 500+ mutations)
- Memory bounds: frame cache 512MB, undo 128MB, video textures ~830MB
- Cyclic parent/precomp chains guarded with depth limits
- NaN/Inf values handled safely in all pixel math

## When Making Changes
1. Run `cargo check --all-features` first
2. Make your changes
3. Run `cargo clippy --all-features` — must be 0 warnings
4. Run `cargo test --all-features` — must pass all
5. Test GUI visually: `cargo run --features gui --bin aftereffects-oss`
6. Commit with descriptive message

## Known Remaining Gaps
- 8bpc pipeline (16bpc would improve grading quality)
- Layer compositing is sequential (bottom-up blend, hard to parallelize)
- Puppet tool / RotoBrush / Paint tools are stubs (buttons only)
- No plugin ecosystem
- 3D camera system is basic (no raytracing, no depth of field)
- Single audio playback track (mixing engine exists but rodio plays one source)

## File Ownership (for parallel AI agents)
See the scope definition in the conversation. Key rule: don't edit
app_state.rs, Cargo.toml, mod.rs files unless you're the main agent.

## Recovery Instructions
If you're a new AI session continuing work:
1. Read this file completely
2. Run `cargo test --all-features` to verify baseline
3. Run `git log --oneline -20` to see recent changes
4. Continue implementing from wherever the last session left off
5. Follow the coding conventions above
6. NEVER break existing tests
