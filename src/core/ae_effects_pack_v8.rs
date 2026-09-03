#![allow(dead_code)]
/// Mega Pack Part 8: After Effects VFX Kernels (261 - 310).
pub fn apply_effect_261(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack::apply_fast_box_blur(pixels, w, h, 6);
}
pub fn apply_effect_262(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack::apply_directional_blur(pixels, w, h, 45.0, 20.0);
}
pub fn apply_effect_263(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack::apply_radial_blur(pixels, w, h, 30.0);
}
pub fn apply_effect_264(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack::apply_glow(pixels, w, h, 0.2, 10, 3.0);
}
pub fn apply_effect_265(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack::apply_drop_shadow(pixels, w, h, 15.0, 90.0, 8, [0, 0, 0, 255]);
}
pub fn apply_effect_266(pixels: &mut [u8]) {
    crate::core::ae_effects_pack::apply_tint(pixels, [10, 0, 30], [200, 255, 200], 0.8);
}
pub fn apply_effect_267(pixels: &mut [u8]) {
    crate::core::ae_effects_pack::apply_posterize(pixels, 4);
}
pub fn apply_effect_268(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack::apply_twirl(pixels, w, h, 90.0, 150.0);
}
pub fn apply_effect_269(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack::apply_bulge(pixels, w, h, 0.8, 120.0);
}
pub fn apply_effect_270(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack::apply_offset(pixels, w, h, 100, -50);
}

pub fn apply_effect_271(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v2::apply_wave_warp(pixels, w, h, 20.0, 50.0, 2.0);
}
pub fn apply_effect_272(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v2::apply_ripple(pixels, w, h, 15.0, 35.0, 1.0);
}
pub fn apply_effect_273(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v2::apply_find_edges(pixels, w, h);
}
pub fn apply_effect_274(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v2::apply_emboss(pixels, w, h, 225.0, 4.0);
}
pub fn apply_effect_275(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v2::apply_mosaic(pixels, w, h, 20, 20);
}
pub fn apply_effect_276(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v2::apply_cc_glass(pixels, w, h, 35.0);
}
pub fn apply_effect_277(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v2::apply_cc_lens(pixels, w, h, 40.0);
}
pub fn apply_effect_278(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v2::apply_cc_tiler(pixels, w, h, 200.0);
}
pub fn apply_effect_279(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v2::apply_cc_kaleida(pixels, w, h, 12);
}
pub fn apply_effect_280(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v2::apply_grid(pixels, w, h, 48, 3, [0, 255, 255, 255]);
}

pub fn apply_effect_281(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v2::apply_checkerboard(
        pixels,
        w,
        h,
        32,
        [50, 0, 0, 255],
        [255, 200, 0, 255],
    );
}
pub fn apply_effect_282(pixels: &mut [u8]) {
    crate::core::ae_effects_pack_v2::apply_fill(pixels, [0, 150, 255, 255]);
}
pub fn apply_effect_283(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v2::apply_stroke_effect(pixels, w, h, [0, 255, 100, 255], 5);
}
pub fn apply_effect_284(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v2::apply_vignette(pixels, w, h, 0.9);
}
pub fn apply_effect_285(pixels: &mut [u8]) {
    crate::core::ae_effects_pack_v2::apply_extract_key(pixels, 50, 200);
}
pub fn apply_effect_286(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v2::apply_radial_wipe(pixels, w, h, 60.0);
}
pub fn apply_effect_287(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v2::apply_iris_wipe(pixels, w, h, 70.0);
}
pub fn apply_effect_288(pixels: &mut [u8], w: u32, h: u32, frame: u32) {
    crate::core::ae_effects_pack_v3::apply_cc_particle_world(
        pixels,
        w,
        h,
        frame,
        [0, 255, 255, 255],
    );
}
pub fn apply_effect_289(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v3::apply_cc_ball_action(pixels, w, h, 12, 0.5);
}
pub fn apply_effect_290(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v3::apply_cc_cylinder(pixels, w, h, 150.0);
}

pub fn apply_effect_291(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v3::apply_cc_sphere(pixels, w, h, 180.0);
}
pub fn apply_effect_292(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v3::apply_cc_page_turn(pixels, w, h, 75.0);
}
pub fn apply_effect_293(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v3::apply_cc_repetile(pixels, w, h, 100.0);
}
pub fn apply_effect_294(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v3::apply_cc_split(pixels, w, h, 40.0);
}
pub fn apply_effect_295(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v3::apply_cc_pixel_polly(pixels, w, h, 25.0);
}
pub fn apply_effect_296(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v3::apply_cc_light_sweep(pixels, w, h, 75.0, 50);
}
pub fn apply_effect_297(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v3::apply_fractal_noise(pixels, w, h, 40.0);
}
pub fn apply_effect_298(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v3::apply_cell_pattern(pixels, w, h, 32);
}
pub fn apply_effect_299(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v4::apply_bevel_edges(pixels, w, h, 5);
}
pub fn apply_effect_300(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v4::apply_glow_edges(pixels, w, h);
}

pub fn apply_effect_301(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v4::apply_cartoon(pixels, w, h);
}
pub fn apply_effect_302(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v4::apply_scatter(pixels, w, h, 12.0);
}
pub fn apply_effect_303(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v4::apply_mirror(pixels, w, h, 45.0);
}
pub fn apply_effect_304(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v4::apply_spherize_fx(pixels, w, h, 80.0);
}
pub fn apply_effect_305(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v4::apply_warp_chromatic(pixels, w, h, 4);
}
pub fn apply_effect_306(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v5::apply_mesh_warp(pixels, w, h, 6, 6);
}
pub fn apply_effect_307(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v5::apply_puppet_bend(pixels, w, h, 30.0);
}
pub fn apply_effect_308(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v5::apply_cc_flo_motion(pixels, w, h, 40.0);
}
pub fn apply_effect_309(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v5::apply_keylight(pixels, w, h, [0, 0, 255]);
}
pub fn apply_effect_310(pixels: &mut [u8], w: u32, h: u32) {
    crate::core::ae_effects_pack_v5::apply_cc_blur_wipe(pixels, w, h, 80.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ae_effects_v8_filters() {
        let mut pixels = vec![100u8; 64];
        apply_effect_261(&mut pixels, 4, 4);
        assert_eq!(pixels.len(), 64);
    }
}
