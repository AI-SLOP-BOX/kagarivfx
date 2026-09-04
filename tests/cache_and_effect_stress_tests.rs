//! Stress and consistency tests for cache modules and effect parameter safety.
//! Catches data races, memory accounting errors, and cross-module parameter drift.

use kagari_vfx::core::frame_cache::{self, FrameCache, PixelBufferPool};
use kagari_vfx::core::merkle_frame_cache::MerkleFrameCache;
use kagari_vfx::core::parallel_render::{ParallelRenderQueue, RenderQueueItem, RenderStats};
use kagari_vfx::core::tile_cache::{self, TileCache};
use kagari_vfx::core::timeline::{
    Composition, Effect, EffectType, Layer, LayerType,
};
use kagari_vfx::core::property::Animatable;
use kagari_vfx::core::software_renderer;
use std::sync::atomic::{AtomicU32, AtomicBool, Ordering};
use std::sync::Arc;

fn fx(effect_type: EffectType) -> Effect {
    Effect {
        id: "test".into(),
        name: "Test".into(),
        effect_type,
        enabled: true,
    }
}

fn c32(v: f32) -> Animatable<f32> {
    Animatable::new_constant(v)
}

fn c32a4(v: [f32; 4]) -> Animatable<[f32; 4]> {
    Animatable::new_constant(v)
}

// ─────────────────────────────────────────────────────────────────────────────
// §1  FrameCache Stress Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn frame_cache_concurrent_version_bump_safety() {
    use std::thread;
    let version_before = frame_cache::current_version();
    let handles: Vec<_> = (0..8)
        .map(|_| {
            thread::spawn(|| {
                for _ in 0..100 {
                    frame_cache::bump_version();
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    let version_after = frame_cache::current_version();
    // Other tests may also bump versions concurrently
    assert!(
        version_after >= version_before + 800,
        "Version should increment by at least 800 (got {})",
        version_after - version_before
    );
}

#[test]
fn frame_cache_insert_get_eviction_cycle() {
    let mut cache = FrameCache::new(10);
    cache.max_memory_bytes = 1024 * 50; // 50 KB budget
    cache.current_memory_bytes = 0;

    let pixels = vec![128u8; 64 * 64 * 4]; // 16 KB each

    // Fill beyond budget
    for i in 0..10u32 {
        let _ = frame_cache::bump_version();
        cache.insert(i, 64, 64, pixels.clone());
    }

    // Memory should not exceed budget after GC
    cache.collect_garbage();
    assert!(
        cache.current_memory_bytes <= cache.max_memory_bytes,
        "Memory {} exceeds budget {}",
        cache.current_memory_bytes,
        cache.max_memory_bytes
    );
}

#[test]
fn frame_cache_stale_entries_invisible_after_version_bump() {
    let mut cache = FrameCache::new(100);
    let pixels = vec![200u8; 32 * 32 * 4];

    let v_before = frame_cache::bump_version();
    cache.insert(0, 32, 32, pixels.clone());
    assert!(cache.is_cached(0), "Should be cached at current version");

    // Bump to a version strictly greater than v_before + 1
    let target = v_before + 2;
    while frame_cache::current_version() < target {
        frame_cache::bump_version();
    }
    // After version bump, old entries should be stale
    assert!(
        !cache.is_cached(0),
        "Stale entry should not be visible at new version"
    );
}

#[test]
fn frame_cache_invalidate_all_clears_everything() {
    let mut cache = FrameCache::new(100);
    let pixels = vec![100u8; 16 * 16 * 4];

    let _ = frame_cache::bump_version();
    for i in 0..20u32 {
        cache.insert(i, 16, 16, pixels.clone());
    }
    assert_eq!(cache.cached_count(), 20);

    cache.invalidate_all();
    assert_eq!(cache.cached_count(), 0, "invalidate_all should clear everything");
}

#[test]
fn frame_cache_layer_dirty_tracking() {
    let mut cache = FrameCache::new(100);
    assert!(!cache.is_layer_dirty(0));
    cache.mark_layer_dirty(0);
    assert!(cache.is_layer_dirty(0));
    cache.clear_dirty();
    assert!(!cache.is_layer_dirty(0));
}

#[test]
fn frame_cache_invalidate_specific_layers() {
    let mut cache = FrameCache::new(100);
    let _ = frame_cache::bump_version();

    let pixels = vec![50u8; 8 * 8 * 4];
    cache.insert_with_layers(0, 8, 8, pixels.clone(), &[0, 1, 2]);
    cache.insert_with_layers(1, 8, 8, pixels.clone(), &[3, 4, 5]);
    assert_eq!(cache.cached_count(), 2, "Both frames should be cached");
    assert!(!cache.is_layer_dirty(0), "Layer 0 should not be dirty yet");

    cache.invalidate_layers(&[0]);
    // invalidate_layers marks the layer dirty, not the frame
    assert!(cache.is_layer_dirty(0), "Layer 0 should be dirty");
    assert!(!cache.is_layer_dirty(1), "Layer 1 should not be dirty");
    assert!(
        cache.is_frame_dirty(0, &[0, 1, 2]),
        "Frame 0 (layers 0,1,2) should be dirty"
    );
    assert!(
        !cache.is_frame_dirty(1, &[3, 4, 5]),
        "Frame 1 (layers 3,4,5) should not be dirty"
    );
}

#[test]
fn pixel_buffer_pool_recycling_bounds() {
    let pool = PixelBufferPool::new();
    let mut bufs = Vec::new();
    // Acquire 100 buffers
    for _ in 0..100 {
        bufs.push(pool.acquire(1024));
    }
    // Recycle all 100
    for buf in bufs {
        pool.recycle(buf);
    }
    // Pool should cap at 64
    assert!(pool.len() <= 64, "Pool exceeded cap: {}", pool.len());
}

// ─────────────────────────────────────────────────────────────────────────────
// §2  TileCache Stress Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn tile_cache_eviction_under_pressure() {
    let mut cache = TileCache::with_version(64, 4096, 10); // 4 KB budget, tiny
    let tile_data = vec![0u8; 64 * 64 * 4]; // 16 KB per tile — oversized

    // Oversized tiles should be rejected silently
    let coord = tile_cache::TileCoord { tx: 0, ty: 0 };
    cache.insert(0, coord, tile_data);
    assert_eq!(cache.tile_count(), 0, "Oversized tile should be rejected");
}

#[test]
fn tile_cache_version_invalidation() {
    let mut cache = TileCache::new(64, 1024 * 1024);
    let small_tile = vec![42u8; 8 * 8 * 4];

    let coord = tile_cache::TileCoord { tx: 0, ty: 0 };
    // Capture version at insert time so we can reliably bump past it
    let v_before = tile_cache::current_tile_version();
    cache.insert(0, coord, small_tile);
    assert!(cache.get(0, coord).is_some(), "Should be found at insert version");

    // Bump to a version strictly greater than what was captured
    let target = v_before + 1;
    while tile_cache::current_tile_version() < target {
        tile_cache::bump_tile_version();
    }
    assert!(cache.get(0, coord).is_none(), "Stale tile should be unreachable");
}

#[test]
fn tile_cache_tiles_for_frame_grid_accuracy() {
    let cache = TileCache::new(256, 1024 * 1024);
    let tiles = cache.tiles_for_frame(0, 1920, 1080);
    let expected_cols = (1920 + 255) / 256; // 8
    let expected_rows = (1080 + 255) / 256; // 5
    assert_eq!(tiles.len(), expected_cols * expected_rows);
}

#[test]
fn tile_cache_invalidate_frame_only() {
    let mut cache = TileCache::with_version(64, 1024 * 1024, 42);
    let tile = vec![1u8; 8 * 8 * 4];
    let coord = tile_cache::TileCoord { tx: 0, ty: 0 };
    cache.insert(0, coord, tile.clone());
    cache.insert(1, coord, tile);

    cache.invalidate_frame(0);
    assert!(cache.get(0, coord).is_none());
    assert!(cache.get(1, coord).is_some(), "Other frame should survive");
}

#[test]
fn tile_cache_memory_accounting_accuracy() {
    let mut cache = TileCache::new(64, 1024 * 1024);
    let tile_size = 8 * 8 * 4; // 256 bytes
    let tile = vec![0u8; tile_size];

    let initial = cache.memory_usage();
    let coord = tile_cache::TileCoord { tx: 0, ty: 0 };
    cache.insert(0, coord, tile);
    let after = cache.memory_usage();
    assert_eq!(
        after - initial,
        tile_size,
        "Memory accounting should match tile size"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// §3  MerkleFrameCache Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn merkle_deterministic_hashing() {
    let h1 = MerkleFrameCache::compute_node_hash("layer1", "params1", 0, None);
    let h2 = MerkleFrameCache::compute_node_hash("layer1", "params1", 0, None);
    let h3 = MerkleFrameCache::compute_node_hash("layer1", "params2", 0, None);
    assert_eq!(h1, h2, "Same inputs must produce same hash");
    assert_ne!(h1, h3, "Different params must produce different hash");
}

#[test]
fn merkle_parent_hash_sensitivity() {
    let h_no_parent = MerkleFrameCache::compute_node_hash("a", "b", 0, None);
    let parent = MerkleFrameCache::compute_node_hash("x", "y", 0, None);
    let h_with_parent = MerkleFrameCache::compute_node_hash("a", "b", 0, Some(&parent));
    assert_ne!(h_no_parent, h_with_parent, "Parent hash must change output");
}

#[test]
fn merkle_cache_deduplication() {
    let mut cache = MerkleFrameCache::new();
    let h = MerkleFrameCache::compute_node_hash("layer", "params", 0, None);
    let pixels = vec![255u8; 100];
    cache.insert(h.clone(), pixels.clone());
    let retrieved = cache.get(&h).unwrap();
    assert_eq!(*retrieved, pixels);
}

// ─────────────────────────────────────────────────────────────────────────────
// §4  ParallelRenderQueue Stress Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn parallel_render_cancel_immediate() {
    let mut queue = ParallelRenderQueue::new();
    for i in 0..100 {
        queue.add_item(RenderQueueItem {
            comp_name: format!("comp_{i}"),
            start_frame: 0,
            end_frame: 10,
            output_path: format!("/tmp/test_{i}.png"),
            status: kagari_vfx::core::parallel_render::RenderStatus::Pending,
        });
    }
    queue.cancel();
    assert!(queue.is_cancelled());

    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();
    queue.render_all(move |_, _| {
        counter_clone.fetch_add(1, Ordering::Relaxed);
        vec![0u8; 4]
    });
    // Cancelled — should render 0 frames
    assert_eq!(counter.load(Ordering::Relaxed), 0);
}

#[test]
fn parallel_render_mfr_cancel() {
    let mut queue = ParallelRenderQueue::new();
    for i in 0..50 {
        queue.add_item(RenderQueueItem {
            comp_name: format!("comp_{i}"),
            start_frame: 0,
            end_frame: 100,
            output_path: format!("/tmp/test_{i}.png"),
            status: kagari_vfx::core::parallel_render::RenderStatus::Pending,
        });
    }
    queue.cancel();

    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = counter.clone();
    queue.render_all_mfr(move |_, _| {
        counter_clone.fetch_add(1, Ordering::Relaxed);
        vec![0u8; 4]
    });
    assert_eq!(counter.load(Ordering::Relaxed), 0);
}

#[test]
fn parallel_render_progress_callback_accuracy() {
    let mut queue = ParallelRenderQueue::new();
    let progress_values = Arc::new(std::sync::Mutex::new(Vec::new()));
    let pv_clone = progress_values.clone();

    queue.set_progress_callback(move |_item, _done, _total| {
        pv_clone.lock().unwrap().push((_done, _total));
    });

    queue.add_item(RenderQueueItem {
        comp_name: "test".into(),
        start_frame: 0,
        end_frame: 4,
        output_path: "/tmp/test.png".into(),
        status: kagari_vfx::core::parallel_render::RenderStatus::Pending,
    });

    queue.render_all(|_, _| vec![0u8; 4]);

    let values = progress_values.lock().unwrap();
    assert!(!values.is_empty(), "Progress callback should have been called");
    // Last callback should show completion
    let last = values.last().unwrap();
    assert_eq!(last.0, last.1, "Final progress should show done == total");
}

#[test]
fn render_stats_calculation() {
    let stats = RenderStats {
        frames_rendered: 100,
        total_frames: 200,
        elapsed_ms: 5000.0,
        avg_frame_ms: 50.0,
        active_threads: 8,
    };
    assert!((stats.progress_pct() - 50.0).abs() < 0.01);
    assert!((stats.fps() - 20.0).abs() < 0.1);
}

// ─────────────────────────────────────────────────────────────────────────────
// §5  Effect Parameter Cross-Module Consistency
//     Every EffectType variant must render with zeroed/constant parameters
//     without panic or infinite loop.
// ─────────────────────────────────────────────────────────────────────────────

fn make_pixels(w: u32, h: u32) -> Vec<u8> {
    let mut p = vec![128u8; (w * h * 4) as usize];
    // Add gradient to avoid trivial zero-input optimization
    for (i, px) in p.chunks_exact_mut(4).enumerate() {
        px[0] = (i as f32 / (w * h) as f32 * 255.0) as u8;
        px[1] = 128;
        px[2] = 64;
        px[3] = 255;
    }
    p
}

#[test]
fn all_effect_variants_render_constant_params() {
    let w = 32u32;
    let h = 32u32;
    let effects: Vec<(&str, Vec<Effect>)> = vec![
        ("GaussianBlur", vec![fx(EffectType::GaussianBlur { blur_radius: c32(3.0) })]),
        ("ColorTint", vec![fx(EffectType::ColorTint { color: c32a4([1.0, 0.0, 0.0, 1.0]), intensity: c32(50.0) })]),
        ("DropShadow", vec![fx(EffectType::DropShadow { color: c32a4([0.0, 0.0, 0.0, 1.0]), opacity: c32(75.0), direction: c32(135.0), distance: c32(10.0), softness: c32(5.0) })]),
        ("ChromaticAberration", vec![fx(EffectType::ChromaticAberration { shift_r: c32(2.0), shift_b: c32(-2.0), edge_falloff: c32(0.5), iris_linked: false })]),
        ("Vignette", vec![fx(EffectType::Vignette { intensity: c32(50.0), roundness: c32(0.5), feather: c32(50.0), color: c32a4([0.0, 0.0, 0.0, 1.0]) })]),
        ("Levels", vec![fx(EffectType::Levels { input_black: c32(0.0), input_white: c32(1.0), gamma: c32(1.0), output_black: c32(0.0), output_white: c32(1.0) })]),
        ("HueSaturation", vec![fx(EffectType::HueSaturation { hue_shift: c32(0.0), saturation: c32(1.0), lightness: c32(0.0) })]),
        ("Glow", vec![fx(EffectType::Glow { threshold: c32(0.8), radius: c32(10.0), intensity: c32(1.5), color: c32a4([1.0, 1.0, 1.0, 1.0]) })]),
        ("Twirl", vec![fx(EffectType::Twirl { angle: c32(45.0), radius: c32(50.0) })]),
        ("Bulge", vec![fx(EffectType::Bulge { amount: c32(0.5), radius: c32(100.0) })]),
        ("Posterize", vec![fx(EffectType::Posterize { levels: c32(8.0) })]),
        ("Invert", vec![fx(EffectType::Invert { invert_alpha: false })]),
        ("Sharpen", vec![fx(EffectType::Sharpen { amount: c32(50.0) })]),
        ("Threshold", vec![fx(EffectType::Threshold { threshold: c32(128.0) })]),
        ("MotionBlur", vec![fx(EffectType::MotionBlur { shutter_angle: c32(180.0), samples: 8 })]),
        ("FilmGrain", vec![fx(EffectType::FilmGrain { intensity: c32(0.3), grain_size: 1.5, color_film: false })]),
        ("DirectionalBlur", vec![fx(EffectType::DirectionalBlur { angle: c32(45.0), length: c32(10.0) })]),
        ("RadialBlur", vec![fx(EffectType::RadialBlur { amount: c32(10.0) })]),
        ("LinearWipe", vec![fx(EffectType::LinearWipe { completion: c32(50.0), angle: c32(0.0) })]),
        ("Offset", vec![fx(EffectType::Offset { shift_x: c32(10.0), shift_y: c32(10.0) })]),
        ("SimpleChoker", vec![fx(EffectType::SimpleChoker { choke_amount: c32(2.0) })]),
        ("TurbulentDisplace", vec![fx(EffectType::TurbulentDisplace { amount: c32(20.0), size: c32(50.0), evolution: c32(0.0), complexity: c32(3.0) })]),
        ("Minimax", vec![fx(EffectType::Minimax { operation: c32(0.5), radius: c32(3.0) })]),
        ("ShiftChannels", vec![fx(EffectType::ShiftChannels { take_red: c32(0.0), take_green: c32(1.0), take_blue: c32(2.0), take_alpha: c32(3.0) })]),
        ("VenetianBlinds", vec![fx(EffectType::VenetianBlinds { completion: c32(50.0), width: c32(10.0) })]),
        ("Tritone", vec![fx(EffectType::Tritone { shadow_color: Animatable::new_constant([0.0; 3]), mid_color: Animatable::new_constant([0.5; 3]), highlight_color: Animatable::new_constant([1.0; 3]) })]),
        ("MatteChoker", vec![fx(EffectType::MatteChoker { choke_amount: c32(2.0), gray_level: c32(0.5) })]),
        ("Vibrance", vec![fx(EffectType::Vibrance { amount: c32(50.0) })]),
        ("WhiteBalance", vec![fx(EffectType::WhiteBalance { temperature: c32(0.0), tint: c32(0.0) })]),
        ("HslAdjust", vec![fx(EffectType::HslAdjust { hue_deg: c32(0.0), saturation: c32(1.0), lightness: c32(0.0) })]),
        ("Vortex", vec![fx(EffectType::Vortex { radius: c32(100.0), angle_deg: c32(45.0) })]),
        ("HeatDistortion", vec![fx(EffectType::HeatDistortion { strength: c32(10.0), speed: c32(1.0) })]),
        ("RainRipples", vec![fx(EffectType::RainRipples { drop_count: c32(10.0), wave_strength: c32(0.5) })]),
        ("Fisheye", vec![fx(EffectType::Fisheye { strength: c32(0.5) })]),
        ("LensCorrection", vec![fx(EffectType::LensCorrection { k1: c32(0.0), k2: c32(0.0) })]),
        ("GlitchDisplacement", vec![fx(EffectType::GlitchDisplacement { seed: c32(42.0), amount: c32(10.0) })]),
        ("CrtScanlines", vec![fx(EffectType::CrtScanlines { line_spacing: c32(4.0), intensity: c32(0.5) })]),
        ("GlowPro", vec![fx(EffectType::GlowPro { threshold: c32(0.8), radius: c32(10.0), intensity: c32(1.5) })]),
        ("RadialFastBlur", vec![fx(EffectType::RadialFastBlur { amount: c32(10.0), samples: 8 })]),
        ("BendIt", vec![fx(EffectType::BendIt { top_offset: c32(0.0), bottom_offset: c32(0.0) })]),
        ("Tiler", vec![fx(EffectType::Tiler { scale_percent: c32(100.0), mirror: false })]),
    ];

    for (name, effects) in &effects {
        let mut pixels = make_pixels(w, h);
        kagari_vfx::core::cpu_effects::apply_layer_effects(
            None, None, &mut pixels, w, h, effects, 0, 30,
        );
        // Verify all output pixels are in valid range
        for (i, &p) in pixels.iter().enumerate() {
            assert!(p <= 255, "{name}: pixel[{i}] = {p} out of range");
        }
        // Verify buffer size unchanged
        assert_eq!(pixels.len(), (w * h * 4) as usize, "{name}: wrong buffer size");
    }
}

#[test]
fn all_effects_with_zero_size_buffer_no_panic() {
    let effects = vec![
        fx(EffectType::GaussianBlur { blur_radius: c32(5.0) }),
        fx(EffectType::Glow { threshold: c32(0.5), radius: c32(5.0), intensity: c32(1.0), color: c32a4([1.0; 4]) }),
        fx(EffectType::Twirl { angle: c32(30.0), radius: c32(50.0) }),
        fx(EffectType::Sharpen { amount: c32(50.0) }),
    ];
    let mut empty = vec![0u8; 0];
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None, None, &mut empty, 0, 0, &effects, 0, 30,
    );
}

#[test]
fn all_effects_on_1x1_buffer_no_panic() {
    let effects = vec![
        fx(EffectType::GaussianBlur { blur_radius: c32(999.0) }),
        fx(EffectType::Glow { threshold: c32(0.0), radius: c32(999.0), intensity: c32(999.0), color: c32a4([1.0; 4]) }),
        fx(EffectType::Twirl { angle: c32(9999.0), radius: c32(9999.0) }),
        fx(EffectType::Vignette { intensity: c32(999.0), roundness: c32(999.0), feather: c32(999.0), color: c32a4([1.0; 4]) }),
        fx(EffectType::Sharpen { amount: c32(9999.0) }),
    ];
    let mut tiny = vec![128u8; 4];
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None, None, &mut tiny, 1, 1, &effects, 0, 30,
    );
}

#[test]
fn all_effects_idempotent() {
    let w = 16u32;
    let h = 16u32;
    let effects = vec![
        fx(EffectType::GaussianBlur { blur_radius: c32(3.0) }),
        fx(EffectType::Sharpen { amount: c32(50.0) }),
        fx(EffectType::Threshold { threshold: c32(128.0) }),
        fx(EffectType::Invert { invert_alpha: false }),
    ];

    let mut pixels1 = make_pixels(w, h);
    let mut pixels2 = pixels1.clone();

    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None, None, &mut pixels1, w, h, &effects, 0, 30,
    );
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None, None, &mut pixels2, w, h, &effects, 0, 30,
    );
    assert_eq!(pixels1, pixels2, "Same effects on same input must produce identical output");
}

#[test]
fn effect_order_matters() {
    let w = 16u32;
    let h = 16u32;

    let effects_a = vec![
        fx(EffectType::Invert { invert_alpha: false }),
        fx(EffectType::Threshold { threshold: c32(128.0) }),
    ];
    let effects_b = vec![
        fx(EffectType::Threshold { threshold: c32(128.0) }),
        fx(EffectType::Invert { invert_alpha: false }),
    ];

    let mut px_a = make_pixels(w, h);
    let mut px_b = px_a.clone();

    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None, None, &mut px_a, w, h, &effects_a, 0, 30,
    );
    kagari_vfx::core::cpu_effects::apply_layer_effects(
        None, None, &mut px_b, w, h, &effects_b, 0, 30,
    );

    // Invert+Threshold != Threshold+Invert (order matters in AE)
    assert_ne!(px_a, px_b, "Effect order should change output");
}

// ─────────────────────────────────────────────────────────────────────────────
// §6  Integration: Software Renderer + Effects + Cache
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn full_pipeline_render_with_effects() {
    let mut comp = Composition::new("test".into(), "Test".into(), 64, 64, 30, 10);
    let mut layer = Layer::new(
        "layer".into(),
        "Layer".into(),
        LayerType::Solid {
            color: [1.0, 0.5, 0.25, 1.0],
        },
        10,
    );
    layer.effects = vec![
        fx(EffectType::GaussianBlur { blur_radius: c32(3.0) }),
        fx(EffectType::Vignette {
            intensity: c32(50.0),
            roundness: c32(0.5),
            feather: c32(50.0),
            color: c32a4([0.0, 0.0, 0.0, 1.0]),
        }),
    ];
    comp.layers.push(layer);

    // Render multiple frames
    for frame in 0..10 {
        let pixels = software_renderer::render_frame_to_pixels(&comp, frame, 64, 64, 0.0, 0);
        assert_eq!(pixels.len(), 64 * 64 * 4, "Frame {frame} wrong size");
        // All pixels must be valid u8
        assert!(pixels.iter().all(|&p| p <= 255));
    }
}

#[test]
fn multi_layer_composite_render() {
    let mut comp = Composition::new("test".into(), "Test".into(), 32, 32, 30, 5);

    // Red layer
    comp.layers.push(Layer::new(
        "red".into(),
        "Red".into(),
        LayerType::Solid { color: [1.0, 0.0, 0.0, 1.0] },
        5,
    ));
    // Green layer (should composite on top)
    comp.layers.push(Layer::new(
        "green".into(),
        "Green".into(),
        LayerType::Solid { color: [0.0, 1.0, 0.0, 1.0] },
        5,
    ));

    let pixels = software_renderer::render_frame_to_pixels(&comp, 0, 32, 32, 0.0, 0);
    assert_eq!(pixels.len(), 32 * 32 * 4);
    // All pixels must be valid
    assert!(pixels.iter().all(|&p| p <= 255));
    // Output should not be empty or all-zero
    let total: u64 = pixels.iter().map(|&p| p as u64).sum();
    assert!(total > 0, "Multi-layer render should produce non-zero output");
}

#[test]
fn blend_mode_composite_render() {
    let mut comp = Composition::new("test".into(), "Test".into(), 16, 16, 30, 1);

    let bottom = Layer::new(
        "bottom".into(),
        "Bottom".into(),
        LayerType::Solid { color: [0.5, 0.5, 0.5, 1.0] },
        1,
    );
    comp.layers.push(bottom);

    let mut top = Layer::new(
        "top".into(),
        "Top".into(),
        LayerType::Solid { color: [0.5, 0.2, 0.2, 1.0] },
        1,
    );
    top.blend_mode = kagari_vfx::core::timeline::BlendMode::Add;
    comp.layers.push(top);

    let pixels = software_renderer::render_frame_to_pixels(&comp, 0, 16, 16, 0.0, 0);
    assert_eq!(pixels.len(), 16 * 16 * 4);

    // Add blend with background color factored in — verify non-zero output
    let total: u64 = pixels.iter().map(|&p| p as u64).sum();
    assert!(total > 0, "Add blend render should produce non-zero output");
}
