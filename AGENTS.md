# Kagari VFX — Agent Instructions

## Project Overview
Kagari VFX is an open-source motion graphics & compositing application written in Rust with GPU rendering (wgpu/Metal), a professional dark UI (egui), and a comprehensive effects pipeline.

## Build & Test Commands
```bash
# Build
cargo build --all-features

# Run all tests
cargo test --all-features

# Lint (must be zero warnings)
cargo clippy --all-features

# Run the GUI app
cargo run --features gui --bin kagari-studio

# Run the CLI tool
cargo run --features cli --bin kagari -- frame --project test_project.json --frame 0 --output /tmp/test.png
```

## Architecture

### Rendering Pipeline
- `src/core/software_renderer.rs` — CPU compositor (layer-by-layer, rayon-parallelized effects)
- `src/core/renderer.rs` — GPU renderer (wgpu 22, Metal backend on macOS)
- `src/core/shader.wgsl` — WGSL shader (all layer types, effects, mesh warp, glow)

### Key Modules
- `src/core/timeline.rs` — Composition/Layer/Keyframe data model (47 EffectType variants, MaterialOptions, Camera3D DOF)
- `src/core/keyframe.rs` — Keyframe interpolation (Linear/Hold/Bezier with 19 ease presets)
- `src/core/property.rs` — Animatable<T> with expression support
- `src/core/expression_engine.rs` — Rhai-based expression evaluation (LOOP_ENGINE cached)
- `src/core/video_import.rs` — FFmpeg video → frame sequence
- `src/core/ffmpeg_export.rs` — MP4 (H.264/ProRes) / GIF export
- `src/core/mlt_export.rs` — MLT XML (Shotcut/Kdenlive)
- `src/core/lottie_exporter.rs` — Lottie/Bodymovin JSON
- `src/core/audio_engine.rs` — WAV loading, multi-track mixing with Mute/Solo
- `src/core/audio_playback.rs` — rodio-synced audio playback
- `src/core/tracker_engine.rs` — SAD motion tracking with subpixel refinement
- `src/core/particle_system.rs` — Deterministic particle simulation
- `src/core/font_rasterizer.rs` — ab_glyph text rasterization (TTC support)
- `src/core/ae_effects_pack.rs` — 20+ CPU effects (box blur, directional, radial, glow, etc.)
- `src/core/cpu_effects.rs` — CPU effect dispatch for all 47 EffectType variants

### UI Panels (src/ui/)
- `viewport.rs` — Composition viewer (GPU/CPU dual path)
- `timeline/` — Timeline with tracks, keyframes, graph editor
- `inspector*.rs` — Property inspector with expression support
- `effects_library.rs` — Categorized effect browser
- `audio_mixer.rs` — Multi-track mixer with live meters
- `tracker_panel.rs` — Motion tracker UI

### State Management
- `src/app_state.rs` — KagariApp (all app state)
- `src/core/history.rs` — ProjectHistory with 128MB byte budget
- `src/core/autosave.rs` — Crash recovery with 5 rotating slots

## Critical Rules

1. **NEVER remove `default_fonts` from eframe features** — without it NO text renders
2. **Always run `cargo test --all-features` before committing** — 469+ tests must pass
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

---

# egui致命的罠リスト（全エージェント必読）

## 罠①: 大量行の「頂点数超過」クラッシュ
eguiは画面外要素を自動クリップするが、ウィジェット生成自体が数千個に達すると
内部で頂点バッファ上限を超え、UIが壊れるかパニックする。

### 対策ルール（違反禁止）
- レイヤー一覧等の大量行は ScrollArea::show_rows() で可視行のみ生成するか、
  ui.is_rect_visible() プローブで非表示行のウィジェット生成を完全スキップする。
- painter呼び出し数はフレームあたりO(画面内要素)に抑えること。全フレームループでのrect描画は禁止。
- 実装済み対策: RAMプレビューバーは連続キャッシュ区間を1矩形に統合、ビート線は自動間隔倍増で最大約200本。

## 罠②: request_repaint()の無秩序発火によるCPU常時消費
- 無条件の ctx.request_repaint() は禁止。is_playingゲート／request_repaint_afterスロットル／操作直後1回のいずれかで発火すること。
- 停止中はマウス操作時のみ再描画される省エネモードが正しい挙動。
- 実装済み対策: 再生ループはis_playingゲート(app_state.rs)、エクスポート進捗は100msスロットル(export_dialog.rs)。

## 罠③: マルチラインheredocの端末貼り付け禁止
cat << EOF 形式の複数行ヒアドキュメントは端末統合が入力を正しく送れずファイル破損を起こす。
ファイル編集は必ず replace_in_file / write_to_file を使うこと。やむを得ない場合は perl -i -pe の単行置換で行う。

## UI Coordination Notes (added by Antigravity session)

- **J/K/L**: J=prev keyframe, K=next keyframe, L=play forward. Do not re-add shuttle J/K handlers.
- **T key** reveals Opacity; **Cmd+T** selects Text tool. Bare T must never switch tools.
- **timeline/mod.rs pending_* pattern**: row-loop actions (duplicate/split/marker/ripple/open-precomp) are collected into `pending_*` locals and applied AFTER the layer loop to stay borrow-safe. Follow this for new row-context actions.
- **draw_prop_row_ext callbacks**: `(on_move, on_select, on_menu, on_box_select, on_group_move)` — all optional.
- **GPU mask compositing shipped** — masks composite on the GPU preview path via a group-3 mask texture (CPU-rasterized coverage, EDT feather, FIFO-cached rasters); position KF dots on the motion path are drag-editable. The CPU/export renderer remains the reference implementation.
- **Effect menu apply helper**: `apply_effect_by_name` in menu.rs — extend it when adding Effect menu entries.
- **Effect timeline rows**: register animatable params in `core/effect_params.rs` (`animatable_params()`). Unregistered variants compile fine but show no keyframe rows in the timeline.

## Multi-AI Build Coordination (added after 6 overnight build breaks — all resolved <5min)

- **Never `git add -A` / `git add .`**: the other AI works on different files concurrently; blanket staging silently commits their half-finished edits. Stage only files you edited (`git add <file> ...`) and verify with `git diff --cached --stat` before committing.
- **Re-read before you edit**: the other AI saves continuously. Always re-read a file immediately before editing it — your in-memory copy may be stale and overwriting loses their work.
- **Build broken by the other AI mid-edit? Don't touch their file.** Loop instead: poll `cargo check` until errors = 0, then run test + clippy, then commit. Their breakages historically resolve within minutes.
- **Flaky tests under parallel cargo runs**: audio WAV roundtrip and frame_cache version tests can fail transiently when two agents run cargo simultaneously. Re-run the single test before diagnosing.
- **Commit discipline**: `cargo test --all-features` EXIT:0 and clippy zero warnings at commit time. If the tree is dirty with unknown changes, assume the other AI is mid-task — share status first.
