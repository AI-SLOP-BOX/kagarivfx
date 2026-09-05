#![allow(clippy::unwrap_used)]

use kagari_vfx::core::compute_pipeline::GpuComputeContext;

fn ctx() -> Option<GpuComputeContext> {
    GpuComputeContext::new()
}

fn solid_pixels(n: u8) -> Vec<u8> {
    vec![n; 64 * 64 * 4]
}

#[test]
fn gpu_context_creation_succeeds_or_headless() {
    let r = ctx();
    assert!(r.is_some() || std::env::var("CI").is_ok());
}

#[test]
fn gpu_gaussian_blur_roundtrip() {
    let Some(c) = ctx() else {
        return;
    };
    let mut px = solid_pixels(200);
    assert!(c.gaussian_blur(&mut px, 64, 64, 2));
    assert!(px.iter().all(|&b| b <= 200));
}

#[test]
fn gpu_directional_blur_smoke() {
    let Some(c) = ctx() else {
        return;
    };
    let mut px = solid_pixels(100);
    assert!(c.directional_blur(&mut px, 64, 64, 8, 45.0));
}

#[test]
fn gpu_radial_blur_smoke() {
    let Some(c) = ctx() else {
        return;
    };
    let mut px = solid_pixels(100);
    assert!(c.radial_blur(&mut px, 64, 64, 4));
}

#[test]
fn gpu_repeated_calls_no_leak() {
    let Some(c) = ctx() else {
        return;
    };
    for _ in 0..10 {
        let mut px = solid_pixels(128);
        c.gaussian_blur(&mut px, 64, 64, 1);
    }
}

#[test]
fn gpu_zero_radius_is_noop() {
    let Some(c) = ctx() else {
        return;
    };
    let mut px = solid_pixels(100);
    let original = px.clone();
    assert!(c.gaussian_blur(&mut px, 64, 64, 0));
    assert_eq!(px, original, "radius=0 must be identity");
}

#[test]
fn gpu_empty_pixels_returns_false() {
    let Some(c) = ctx() else {
        return;
    };
    let mut px: Vec<u8> = vec![];
    assert!(!c.gaussian_blur(&mut px, 0, 0, 4));
}

#[test]
fn gpu_context_dropped_clean_teardown() {
    {
        let c = ctx();
        if let Some(c) = c {
            let mut px = solid_pixels(100);
            c.gaussian_blur(&mut px, 64, 64, 2);
        }
    }
}

#[test]
fn gpu_multiple_contexts_sequential() {
    let c1 = ctx();
    let c2 = ctx();
    if let (Some(c1), Some(c2)) = (c1, c2) {
        let mut px1 = solid_pixels(50);
        let mut px2 = solid_pixels(50);
        c1.gaussian_blur(&mut px1, 64, 64, 3);
        c2.gaussian_blur(&mut px2, 64, 64, 3);
    }
}
