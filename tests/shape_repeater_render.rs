use kagari_vfx::core::keyframe::{InterpolationType, Keyframe};
use kagari_vfx::core::project_migration::{load_project_migrated, save_project_versioned};
use kagari_vfx::core::property::Animatable;
use kagari_vfx::core::shape_repeater::ShapeRepeaterOptions;
use kagari_vfx::core::software_renderer::render_frame_to_pixels;
use kagari_vfx::core::timeline::Project;
use kagari_vfx::core::timeline::{
    BlendMode, Composition, Layer, LayerType, ShapeFillType, ShapeType, TrackMatteMode,
};

fn repeated_comp(options: ShapeRepeaterOptions) -> Composition {
    let mut comp = Composition::new(
        "repeater-test".into(),
        "Repeater Test".into(),
        128,
        64,
        30,
        30,
    );
    let mut layer = Layer::new(
        "shape".into(),
        "Repeated Shape".into(),
        LayerType::Shape {
            shape_type: ShapeType::Rectangle {
                width: Animatable::new_constant(12.0),
                height: Animatable::new_constant(12.0),
                corner_radius: Animatable::new_constant(0.0),
            },
            color: [1.0, 0.0, 0.0, 1.0],
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        30,
    );
    layer.transform.position = Animatable::new_constant([16.0, 32.0]);
    layer.shape_repeater = Some(options);
    comp.layers.push(layer);
    comp
}

fn red_pixel_count(pixels: &[u8]) -> usize {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| pixel[0] > 200 && pixel[1] < 40 && pixel[2] < 40)
        .count()
}

fn red_at(pixels: &[u8], x: usize, y: usize) -> u8 {
    pixels[(y * 128 + x) * 4]
}

#[test]
fn repeater_render_contains_separated_copies() {
    let options = ShapeRepeaterOptions {
        copies: 3,
        position_offset: [32.0, 0.0],
        ..Default::default()
    };
    let pixels = render_frame_to_pixels(&repeated_comp(options), 0, 128, 64, 0.0, 0);
    assert!(red_pixel_count(&pixels) > 3 * 20);
    assert!(red_at(&pixels, 16, 32) > 200);
    assert!(red_at(&pixels, 48, 32) > 200);
    assert!(red_at(&pixels, 80, 32) > 200);
}

#[test]
fn repeater_render_keeps_far_copy_inside_bounds() {
    let options = ShapeRepeaterOptions {
        copies: 2,
        position_offset: [96.0, 0.0],
        ..Default::default()
    };
    let pixels = render_frame_to_pixels(&repeated_comp(options), 0, 128, 64, 0.0, 0);
    assert!(red_at(&pixels, 16, 32) > 200);
    assert!(red_at(&pixels, 112, 32) > 200);
}

#[test]
fn zero_copy_repeater_does_not_render_shape() {
    let options = ShapeRepeaterOptions {
        copies: 0,
        ..Default::default()
    };
    let pixels = render_frame_to_pixels(&repeated_comp(options), 0, 128, 64, 0.0, 0);
    assert_eq!(red_pixel_count(&pixels), 0);
}

#[test]
fn rotated_repeater_copy_keeps_diagonal_corners() {
    let options = ShapeRepeaterOptions {
        copies: 2,
        position_offset: [48.0, 0.0],
        rotation_offset_deg: 45.0,
        ..Default::default()
    };
    let pixels = render_frame_to_pixels(&repeated_comp(options), 0, 128, 64, 0.0, 0);
    assert!(red_pixel_count(&pixels) > 2 * 20);
}

#[test]
fn animated_copy_count_changes_rendered_frame() {
    let mut copies = Animatable::new_constant(1.0);
    copies.add_keyframe(Keyframe::new(0, 1.0, InterpolationType::Linear));
    copies.add_keyframe(Keyframe::new(10, 4.0, InterpolationType::Linear));
    let options = ShapeRepeaterOptions {
        position_offset: [24.0, 0.0],
        copies_animation: Some(copies),
        ..Default::default()
    };

    let frame0 = render_frame_to_pixels(&repeated_comp(options.clone()), 0, 128, 64, 0.0, 0);
    let frame10 = render_frame_to_pixels(&repeated_comp(options), 10, 128, 64, 0.0, 0);
    assert!(red_pixel_count(&frame10) > red_pixel_count(&frame0) * 3);
}

#[test]
fn shape_alpha_matte_uses_actual_shape_coverage() {
    let mut comp = Composition::new("matte-test".into(), "Shape Matte".into(), 64, 64, 30, 30);
    let mut matte = Layer::new(
        "matte".into(),
        "Ellipse Matte".into(),
        LayerType::Shape {
            shape_type: ShapeType::Ellipse {
                width: Animatable::new_constant(32.0),
                height: Animatable::new_constant(32.0),
            },
            color: [1.0; 4],
            stroke_color: [0.0; 4],
            stroke_width: 0.0,
            fill_type: ShapeFillType::Solid,
            extrusion_depth: 0.0,
            bevel_depth: 0.0,
        },
        30,
    );
    matte.transform.position = Animatable::new_constant([32.0, 32.0]);
    matte.transform.scale = Animatable::new_constant([100.0, 100.0]);
    comp.layers.push(matte);

    let mut content = Layer::new(
        "content".into(),
        "Content".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        30,
    );
    content.track_matte = TrackMatteMode::AlphaMatte;
    content.blend_mode = BlendMode::Normal;
    comp.layers.push(content);

    let mut matte_only = comp.clone();
    matte_only.layers.truncate(1);
    let matte_pixels = render_frame_to_pixels(&matte_only, 0, 64, 64, 0.0, 0);
    assert!(matte_pixels[corner_index(4, 4)] < 20);
    let pixels = render_frame_to_pixels(&comp, 0, 64, 64, 0.0, 0);
    let center = (32 * 64 + 32) * 4;
    let corner = 4 * 4;
    assert!(pixels[center] > 200);
    assert!(
        pixels[corner] < 20,
        "corner red should be clipped by shape matte: {}",
        pixels[corner]
    );
}

#[test]
fn image_alpha_matte_uses_source_alpha() {
    let path = std::env::temp_dir().join(format!("aura-matte-{}.png", std::process::id()));
    let mut image = image::RgbaImage::new(2, 1);
    image.put_pixel(0, 0, image::Rgba([255, 255, 255, 255]));
    image.put_pixel(1, 0, image::Rgba([255, 255, 255, 0]));
    image.save(&path).expect("write matte fixture");

    let mut comp = Composition::new(
        "image-matte-test".into(),
        "Image Matte".into(),
        64,
        32,
        30,
        30,
    );
    let mut matte = Layer::new(
        "image-matte".into(),
        "Image Matte".into(),
        LayerType::Image {
            path: path.to_string_lossy().into_owned(),
        },
        30,
    );
    matte.transform.position = Animatable::new_constant([32.0, 16.0]);
    comp.layers.push(matte);

    let mut content = Layer::new(
        "image-content".into(),
        "Content".into(),
        LayerType::Solid {
            color: [1.0, 0.0, 0.0, 1.0],
        },
        30,
    );
    content.track_matte = TrackMatteMode::AlphaMatte;
    comp.layers.push(content);

    let pixels = render_frame_to_pixels(&comp, 0, 64, 32, 0.0, 0);
    assert!(pixels[(16 * 64 + 16) * 4] > 200);
    assert!(pixels[(16 * 64 + 48) * 4] < 20);
    let _ = std::fs::remove_file(path);
}

fn corner_index(x: usize, y: usize) -> usize {
    (y * 64 + x) * 4
}

#[test]
fn versioned_project_roundtrip_preserves_repeater_render() {
    let options = ShapeRepeaterOptions {
        copies: 3,
        position_offset: [28.0, 0.0],
        ..Default::default()
    };
    let project = Project {
        compositions: vec![repeated_comp(options)],
        active_composition_idx: 0,
        assets: Vec::new(),
        use_gpu_compute: false,
    };
    let before = render_frame_to_pixels(&project.compositions[0], 0, 128, 64, 0.0, 0);
    let json = save_project_versioned(&project).expect("project serializes");
    let restored = load_project_migrated(&json).expect("versioned project loads");
    let after = render_frame_to_pixels(&restored.compositions[0], 0, 128, 64, 0.0, 0);
    assert_eq!(before, after);
}
