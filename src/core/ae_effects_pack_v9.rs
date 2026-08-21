#![allow(dead_code)]
/// Mega Pack Part 9: After Effects VFX Kernels (311 - 360).
pub fn apply_effect_311(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack::apply_directional_blur(pixels, w, h, 180.0, 30.0); }
pub fn apply_effect_312(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack::apply_radial_blur(pixels, w, h, 45.0); }
pub fn apply_effect_313(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack::apply_glow(pixels, w, h, 0.4, 15, 4.0); }
pub fn apply_effect_314(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack::apply_drop_shadow(pixels, w, h, 20.0, 135.0, 10, [0, 0, 0, 255]); }
pub fn apply_effect_315(pixels: &mut [u8]) { crate::core::ae_effects_pack::apply_tint(pixels, [50, 0, 0], [255, 255, 100], 0.5); }
pub fn apply_effect_316(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack::apply_twirl(pixels, w, h, 180.0, 200.0); }
pub fn apply_effect_317(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack::apply_bulge(pixels, w, h, 1.2, 150.0); }
pub fn apply_effect_318(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_wave_warp(pixels, w, h, 30.0, 80.0, 3.0); }
pub fn apply_effect_319(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_ripple(pixels, w, h, 25.0, 50.0, 2.0); }
pub fn apply_effect_320(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_emboss(pixels, w, h, 315.0, 5.0); }

pub fn apply_effect_321(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_mosaic(pixels, w, h, 32, 32); }
pub fn apply_effect_322(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_cc_glass(pixels, w, h, 50.0); }
pub fn apply_effect_323(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_cc_lens(pixels, w, h, 60.0); }
pub fn apply_effect_324(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_cc_tiler(pixels, w, h, 300.0); }
pub fn apply_effect_325(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_cc_kaleida(pixels, w, h, 16); }
pub fn apply_effect_326(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_grid(pixels, w, h, 64, 4, [255, 0, 255, 255]); }
pub fn apply_effect_327(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_checkerboard(pixels, w, h, 64, [0, 50, 50, 255], [255, 255, 255, 255]); }
pub fn apply_effect_328(pixels: &mut [u8]) { crate::core::ae_effects_pack_v2::apply_fill(pixels, [255, 255, 0, 255]); }
pub fn apply_effect_329(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_stroke_effect(pixels, w, h, [255, 0, 255, 255], 8); }
pub fn apply_effect_330(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_vignette(pixels, w, h, 1.2); }

pub fn apply_effect_331(pixels: &mut [u8]) { crate::core::ae_effects_pack_v2::apply_extract_key(pixels, 80, 180); }
pub fn apply_effect_332(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_radial_wipe(pixels, w, h, 90.0); }
pub fn apply_effect_333(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_iris_wipe(pixels, w, h, 90.0); }
pub fn apply_effect_334(pixels: &mut [u8], w: u32, h: u32, frame: u32) { crate::core::ae_effects_pack_v3::apply_cc_particle_world(pixels, w, h, frame, [255, 255, 255, 255]); }
pub fn apply_effect_335(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v3::apply_cc_ball_action(pixels, w, h, 16, 0.2); }
pub fn apply_effect_336(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v3::apply_cc_cylinder(pixels, w, h, 200.0); }
pub fn apply_effect_337(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v3::apply_cc_sphere(pixels, w, h, 250.0); }
pub fn apply_effect_338(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v3::apply_cc_page_turn(pixels, w, h, 95.0); }
pub fn apply_effect_339(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v3::apply_cc_repetile(pixels, w, h, 200.0); }
pub fn apply_effect_340(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v3::apply_cc_split(pixels, w, h, 80.0); }

pub fn apply_effect_341(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v3::apply_cc_pixel_polly(pixels, w, h, 50.0); }
pub fn apply_effect_342(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v3::apply_cc_light_sweep(pixels, w, h, 100.0, 80); }
pub fn apply_effect_343(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v3::apply_fractal_noise(pixels, w, h, 80.0); }
pub fn apply_effect_344(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v3::apply_cell_pattern(pixels, w, h, 64); }
pub fn apply_effect_345(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v4::apply_bevel_edges(pixels, w, h, 8); }
pub fn apply_effect_346(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v4::apply_scatter(pixels, w, h, 25.0); }
pub fn apply_effect_347(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v4::apply_mirror(pixels, w, h, 90.0); }
pub fn apply_effect_348(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v4::apply_spherize_fx(pixels, w, h, 100.0); }
pub fn apply_effect_349(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v4::apply_warp_chromatic(pixels, w, h, 8); }
pub fn apply_effect_350(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v5::apply_mesh_warp(pixels, w, h, 8, 8); }

pub fn apply_effect_351(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v5::apply_puppet_bend(pixels, w, h, 45.0); }
pub fn apply_effect_352(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v5::apply_cc_flo_motion(pixels, w, h, 60.0); }
pub fn apply_effect_353(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v5::apply_keylight(pixels, w, h, [255, 0, 0]); }
pub fn apply_effect_354(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v5::apply_cc_blur_wipe(pixels, w, h, 100.0); }
pub fn apply_effect_355(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v6::apply_shatter(pixels, w, h, 1.0); }
pub fn apply_effect_356(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v6::apply_card_dance(pixels, w, h, 10, 10, 45.0); }
pub fn apply_effect_357(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v6::apply_caustics(pixels, w, h, 30.0); }
pub fn apply_effect_358(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v6::apply_wave_world(pixels, w, h, 5.0, 10.0); }
pub fn apply_effect_359(pixels: &mut [u8], w: u32, h: u32) {
    let dummy_depth = vec![128u8; pixels.len()];
    crate::core::ae_effects_pack_v6::apply_dof_blur(pixels, w, h, &dummy_depth, 128, 5);
}

pub fn apply_effect_360(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v6::apply_3d_glasses(pixels, w, h, 5); }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ae_effects_v9_filters() {
        let mut pixels = vec![100u8; 64];
        apply_effect_311(&mut pixels, 4, 4);
        assert_eq!(pixels.len(), 64);
    }
}
