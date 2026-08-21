#![allow(dead_code)]
/// Mega Pack Part 7: After Effects VFX Kernels (211 - 260).
// 211 - 260: Comprehensive VFX Kernels
pub fn apply_effect_211(pixels: &mut [u8]) { crate::core::ae_effects_pack::apply_invert(pixels, true); }
pub fn apply_effect_212(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack::apply_fast_box_blur(pixels, w, h, 4); }
pub fn apply_effect_213(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack::apply_directional_blur(pixels, w, h, 90.0, 10.0); }
pub fn apply_effect_214(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack::apply_radial_blur(pixels, w, h, 15.0); }
pub fn apply_effect_215(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack::apply_unsharp_mask(pixels, w, h, 150.0, 2); }
pub fn apply_effect_216(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack::apply_glow(pixels, w, h, 0.1, 5, 2.5); }
pub fn apply_effect_217(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack::apply_drop_shadow(pixels, w, h, 10.0, 45.0, 4, [0, 0, 0, 200]); }
pub fn apply_effect_218(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack::apply_radial_fast_blur(pixels, w, h, 5.0); }
pub fn apply_effect_219(pixels: &mut [u8]) { crate::core::ae_effects_pack::apply_simple_choker(pixels, 2.0); }
pub fn apply_effect_220(pixels: &mut [u8]) { crate::core::ae_effects_pack::apply_matte_choker(pixels, 2.0, 0.5); }

pub fn apply_effect_221(pixels: &mut [u8]) { crate::core::ae_effects_pack::apply_tint(pixels, [0, 20, 50], [255, 200, 150], 1.0); }
pub fn apply_effect_222(pixels: &mut [u8]) { crate::core::ae_effects_pack::apply_tritone(pixels, [0, 0, 0], [128, 128, 128], [255, 255, 255]); }
pub fn apply_effect_223(pixels: &mut [u8]) { crate::core::ae_effects_pack::apply_posterize(pixels, 6); }
pub fn apply_effect_224(pixels: &mut [u8]) { crate::core::ae_effects_pack::apply_threshold(pixels, 128); }
pub fn apply_effect_225(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack::apply_twirl(pixels, w, h, 45.0, 100.0); }
pub fn apply_effect_226(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack::apply_bulge(pixels, w, h, 0.5, 80.0); }
pub fn apply_effect_227(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack::apply_offset(pixels, w, h, 50, 50); }
pub fn apply_effect_228(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack::apply_venetian_blinds(pixels, w, h, 30.0, 15); }
pub fn apply_effect_229(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack::apply_linear_wipe(pixels, w, h, 40.0, 45.0); }
pub fn apply_effect_230(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_wave_warp(pixels, w, h, 10.0, 40.0, 1.0); }

pub fn apply_effect_231(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_ripple(pixels, w, h, 8.0, 25.0, 0.5); }
pub fn apply_effect_232(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_gradient_ramp(pixels, w, h, [255, 0, 0, 255], [0, 0, 255, 255], false); }
pub fn apply_effect_233(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_gradient_ramp(pixels, w, h, [255, 255, 0, 255], [0, 0, 0, 255], true); }
pub fn apply_effect_234(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_find_edges(pixels, w, h); }
pub fn apply_effect_235(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_emboss(pixels, w, h, 135.0, 3.0); }
pub fn apply_effect_236(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_mosaic(pixels, w, h, 12, 12); }
pub fn apply_effect_237(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_cc_glass(pixels, w, h, 20.0); }
pub fn apply_effect_238(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_cc_lens(pixels, w, h, 25.0); }
pub fn apply_effect_239(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_cc_tiler(pixels, w, h, 150.0); }
pub fn apply_effect_240(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_cc_kaleida(pixels, w, h, 8); }

pub fn apply_effect_241(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_grid(pixels, w, h, 32, 2, [255, 255, 255, 255]); }
pub fn apply_effect_242(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_checkerboard(pixels, w, h, 20, [0, 0, 0, 255], [255, 255, 255, 255]); }
pub fn apply_effect_243(pixels: &mut [u8]) { crate::core::ae_effects_pack_v2::apply_fill(pixels, [255, 100, 50, 255]); }
pub fn apply_effect_244(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_stroke_effect(pixels, w, h, [255, 255, 0, 255], 3); }
pub fn apply_effect_245(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_vignette(pixels, w, h, 0.7); }
pub fn apply_effect_246(pixels: &mut [u8]) { crate::core::ae_effects_pack_v2::apply_channel_combiner(pixels); }
pub fn apply_effect_247(pixels: &mut [u8]) { crate::core::ae_effects_pack_v2::apply_extract_key(pixels, 30, 220); }
pub fn apply_effect_248(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_time_displacement(pixels, w, h, 10); }
pub fn apply_effect_249(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_radial_wipe(pixels, w, h, 35.0); }
pub fn apply_effect_250(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v2::apply_iris_wipe(pixels, w, h, 45.0); }

pub fn apply_effect_251(pixels: &mut [u8], prev: &[u8]) { crate::core::ae_effects_pack_v3::apply_time_difference(pixels, prev); }
pub fn apply_effect_252(pixels: &mut [u8], frozen: &[u8]) { crate::core::ae_effects_pack_v3::apply_freeze_frame(pixels, frozen); }
pub fn apply_effect_253(pixels: &mut [u8], fa: &[u8], fb: &[u8]) { crate::core::ae_effects_pack_v3::apply_timewarp(pixels, fa, fb, 0.5); }
pub fn apply_effect_254(pixels: &mut [u8], frame: u32) { crate::core::ae_effects_pack_v3::apply_strobe_light(pixels, frame, 5, [255, 255, 255, 255]); }
pub fn apply_effect_255(pixels: &mut [u8], w: u32, h: u32, frame: u32) { crate::core::ae_effects_pack_v3::apply_cc_particle_world(pixels, w, h, frame, [255, 100, 0, 255]); }
pub fn apply_effect_256(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v3::apply_cc_ball_action(pixels, w, h, 6, 0.8); }
pub fn apply_effect_257(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v3::apply_cc_cylinder(pixels, w, h, 100.0); }
pub fn apply_effect_258(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v3::apply_cc_sphere(pixels, w, h, 120.0); }
pub fn apply_effect_259(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v3::apply_cc_page_turn(pixels, w, h, 50.0); }
pub fn apply_effect_260(pixels: &mut [u8], w: u32, h: u32) { crate::core::ae_effects_pack_v3::apply_cc_repetile(pixels, w, h, 50.0); }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ae_effects_v7_filters() {
        let mut pixels = vec![100u8; 64];
        apply_effect_212(&mut pixels, 4, 4);
        assert_eq!(pixels.len(), 64);
    }
}
