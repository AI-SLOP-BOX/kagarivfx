# After Effects OSS Alternative — Agent Instructions

## Project Overview
An open-source After Effects alternative written in Rust with GPU rendering (wgpu/Metal), a professional dark UI (egui), and a comprehensive motion graphics pipeline.

## Build & Test Commands
```bash
# Build
cargo build --all-features

# Run all tests
cargo test --all-features

# Lint (must be zero warnings)
cargo clippy --all-features

# Run the GUI app
cargo run --features gui --bin aftereffects-oss

# Run the CLI tool
cargo run --features cli --bin aevfx -- frame --project test_project.json --frame 0 --output /tmp/test.png
```

## Architecture

### Rendering Pipeline
- `src/core/software_renderer.rs` — CPU compositor (layer-by-layer, rayon-parallelized effects)
- `src/core/renderer.rs` — GPU renderer (wgpu 22, Metal backend on macOS)
- `src/core/shader.wgsl` — WGSL shader (all layer types, effects, mesh warp, glow)

### Key Modules
- `src/core/timeline.rs` — Composition/Layer/Keyframe data model
- `src/core/keyframe.rs` — Keyframe interpolation (Linear/Hold/Bezier with 19 ease presets)
- `src/core/property.rs` — Animatable<T> with expression support
- `src/core/expression_engine.rs` — Rhai-based expression evaluation
- `src/core/video_import.rs` — FFmpeg video → frame sequence
- `src/core/ffmpeg_export.rs` — MP4 (H.264/ProRes) / GIF export
- `src/core/mlt_export.rs` — MLT XML (Shotcut/Kdenlive)
- `src/core/lottie_exporter.rs` — Lottie/Bodymovin JSON
- `src/core/audio_engine.rs` — WAV loading, multi-track mixing
- `src/core/audio_playback.rs` — rodio-synced audio playback
- `src/core/tracker_engine.rs` — SAD motion tracking with subpixel refinement
- `src/core/particle_system.rs` — Deterministic particle simulation
- `src/core/font_rasterizer.rs` — ab_glyph text rasterization (TTC support)

### UI Panels (src/ui/)
- `viewport.rs` — Composition viewer (GPU/CPU dual path)
- `timeline/` — Timeline with tracks, keyframes, graph editor
- `inspector*.rs` — Property inspector with expression support
- `effects_library.rs` — Categorized effect browser
- `audio_mixer.rs` — Multi-track mixer with live meters
- `tracker_panel.rs` — Motion tracker UI

### State Management
- `src/app_state.rs` — AfterEffectsApp (all app state)
- `src/core/history.rs` — ProjectHistory with 128MB byte budget
- `src/core/autosave.rs` — Crash recovery with 5 rotating slots

## Critical Rules

1. **NEVER remove `default_fonts` from eframe features** — without it NO text renders
2. **Always run `cargo test --all-features` before committing** — 262 tests must pass
3. **Always run `cargo clippy --all-features`** — must be zero warnings
4. **Use `git commit` after every meaningful change** — atomic commits
5. **GPU rendering**: viewport uses WgpuRenderer when available; falls back to CPU
6. **Determinism**: renders must be byte-identical for same input (no pointer hashing)
7. **Thread safety**: use `Arc<T>` for GPU resources, `RefCell` for caches in render paths
8. **Memory bounds**: frame cache 512MB LRU, undo history 128MB, video textures 200 frames

## Coding Style
- No comments unless explaining non-obvious logic
- Prefer `impl Trait` over `dyn Trait` in hot paths
- Use `#[derive(Serialize, Deserialize)]` with `#[serde(default)]` for backward compat
- All new effects go in `src/core/ae_effects_pack.rs` or a new `v*.rs` file
- All new UI panels go in `src/ui/` and get registered in `src/ui/mod.rs`

## Testing
- Unit tests: same file as implementation (`#[cfg(test)] mod tests`)
- Integration: `tests/` directory
- Fuzz: `tests/fuzz_matrix_tests.rs`, `tests/mutation_fuzz_tests.rs`
- Stress: `tests/stress_tests.rs`
- Shader: `tests/shader_validation.rs`

## Known Limitations
- 8bpc pipeline (16/32bpc is a future goal)
- Layer compositing is sequential (bottom-up blend order)
- No real-time GPU effects processing (effects run on CPU)
- Puppet tool, roto brush, paint tools are UI stubs only
