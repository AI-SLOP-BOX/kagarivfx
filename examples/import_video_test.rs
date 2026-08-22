use aftereffects_oss::core::timeline::{Composition, Layer, LayerType};
use aftereffects_oss::core::property::Animatable;
use aftereffects_oss::core::video_import::{import_video, frame_path};

fn main() {
    let dest = std::path::Path::new("/tmp/aevfx_video_import_test");
    let _ = std::fs::remove_dir_all(dest);
    let asset = import_video("/tmp/aevfx_test_video.mp4", dest, 10.0).expect("import");

    // Build a comp with a Video layer
    let mut comp = Composition::new("c1".into(), "VideoTest".into(), 160, 120, 10, 20);
    let bg = Layer::new("bg".into(), "BG".into(), LayerType::Solid { color: [0.0, 0.0, 0.0, 1.0] }, 10);
    comp.layers.push(bg);
    let mut vid = Layer::new("v1".into(), "MyVideo".into(), LayerType::Video {
        source: asset.source_path.clone(),
        frames_dir: asset.frames_dir.clone(),
        frame_count: asset.frame_count,
        audio_wav: asset.audio_wav.clone(),
    }, 10);
    vid.transform.position = Animatable::new_constant([80.0, 60.0]);
    comp.layers.push(vid);

    // Render two frames — video content must differ between them (testsrc animates)
    let f0 = aftereffects_oss::core::software_renderer::render_frame_to_pixels(&comp, 2, 160, 120, 0.0, 0);
    let f1 = aftereffects_oss::core::software_renderer::render_frame_to_pixels(&comp, 12, 160, 120, 0.0, 0);
    assert_eq!(f0.len(), 160 * 120 * 4);

    let bright = |px: &[u8]| (0..px.len()).step_by(4).filter(|&i| px[i] > 100).count();
    let nonzero = |px: &[u8]| (0..px.len()).step_by(4).filter(|&i| px[i] > 0).count();
    println!("bright frame2={} frame12={} nonzero2={} nonzero12={}", bright(&f0), bright(&f1), nonzero(&f0), nonzero(&f1));
    println!("differs: {}", f0 != f1);

    // Frame path sanity
    println!("last frame path exists: {}", frame_path(&asset, 19).exists());
}
