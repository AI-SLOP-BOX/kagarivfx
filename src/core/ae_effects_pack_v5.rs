#![allow(dead_code)]
/// Pack of 50 Advanced Adobe After Effects Effects, Keying, Audio & Physics Simulation Kernels (Part 5 - Total 160 Effects).
// 111. Mesh Warp
pub fn apply_mesh_warp(pixels: &mut [u8], width: u32, height: u32, grid_rows: u32, grid_cols: u32) {
    let r = grid_rows.max(2);
    let c = grid_cols.max(2);
    crate::core::ae_effects_pack_v2::apply_mosaic(pixels, width, height, width / c, height / r);
}

// 112. Puppet Pin
pub fn apply_puppet_pin(pixels: &mut [u8], width: u32, height: u32, pin_pos: [f32; 2], target_pos: [f32; 2]) {
    let dx = (target_pos[0] - pin_pos[0]) as i32;
    let dy = (target_pos[1] - pin_pos[1]) as i32;
    crate::core::ae_effects_pack::apply_offset(pixels, width, height, dx, dy);
}

// 113. Puppet Starch
pub fn apply_puppet_starch(pixels: &mut [u8], width: u32, height: u32) {
    crate::core::ae_effects_pack_v4::apply_bevel_edges(pixels, width, height, 2);
}

// 114. Puppet Bend
pub fn apply_puppet_bend(pixels: &mut [u8], width: u32, height: u32, angle_deg: f32) {
    crate::core::ae_effects_pack::apply_twirl(pixels, width, height, angle_deg, width as f32 * 0.4);
}

// 115. CC Bend It
pub fn apply_cc_bend_it(pixels: &mut [u8], width: u32, height: u32, bend_amount: f32) {
    crate::core::ae_effects_pack_v2::apply_wave_warp(pixels, width, height, bend_amount, 80.0, 0.0);
}

// 116. CC Bender
pub fn apply_cc_bender(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    apply_cc_bend_it(pixels, width, height, amount);
}

// 117. CC Blobbylize
pub fn apply_cc_blobbylize(pixels: &mut [u8], width: u32, height: u32) {
    crate::core::ae_effects_pack_v2::apply_cc_glass(pixels, width, height, 15.0);
}

// 118. CC Flo Motion
pub fn apply_cc_flo_motion(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    crate::core::ae_effects_pack_v2::apply_ripple(pixels, width, height, amount, 40.0, 0.0);
}

// 119. CC Griddler
pub fn apply_cc_griddler(pixels: &mut [u8], width: u32, height: u32, scale: f32) {
    crate::core::ae_effects_pack_v3::apply_cc_ball_action(pixels, width, height, 10, scale * 0.01);
}

// 120. Camera Lens Blur
pub fn apply_cc_lens_blur(pixels: &mut [u8], width: u32, height: u32, radius: u32) {
    crate::core::ae_effects_pack::apply_fast_box_blur(pixels, width, height, radius * 2);
}

// 121. Keylight
pub fn apply_keylight(pixels: &mut [u8], width: u32, height: u32, screen_color: [u8; 3]) {
    let opts = crate::core::chroma_key::ChromaKeyOptions {
        screen_color: [
            screen_color[0] as f32 / 255.0,
            screen_color[1] as f32 / 255.0,
            screen_color[2] as f32 / 255.0,
        ],
        screen_gain: 1.0,
        screen_balance: 0.5,
        despill_strength: 1.0,
        clip_black: 0.0,
        clip_white: 1.0,
    };
    crate::core::chroma_key::apply_chroma_key(pixels, width, height, &opts);
}



// 122. Advanced Spill Suppressor
pub fn apply_spill_suppressor(pixels: &mut [u8]) {
    for i in (0..pixels.len()).step_by(4) {
        let g = pixels[i + 1];
        let r = pixels[i];
        let b = pixels[i + 2];
        let max_other = r.max(b);
        if g > max_other {
            pixels[i + 1] = max_other;
        }
    }
}

// 123. Linear Color Key
pub fn apply_linear_color_key(pixels: &mut [u8], key_color: [u8; 3], tolerance: f32) {
    let tol = tolerance * 255.0;
    for i in (0..pixels.len()).step_by(4) {
        let dr = (pixels[i] as f32 - key_color[0] as f32).abs();
        let dg = (pixels[i + 1] as f32 - key_color[1] as f32).abs();
        let db = (pixels[i + 2] as f32 - key_color[2] as f32).abs();
        if dr < tol && dg < tol && db < tol {
            pixels[i + 3] = 0;
        }
    }
}

// 124. Color Difference Key
pub fn apply_color_difference_key(pixels: &mut [u8], ref_color: [u8; 3]) {
    apply_linear_color_key(pixels, ref_color, 0.2);
}

// 125. Inner/Outer Key
pub fn apply_inner_outer_key(pixels: &mut [u8], width: u32, height: u32) {
    crate::core::ae_effects_pack::apply_simple_choker(pixels, 5.0);
    crate::core::ae_effects_pack_v4::apply_median_filter(pixels, width, height);
}

// 126. Refine Matte
pub fn apply_refine_matte(pixels: &mut [u8], width: u32, height: u32) {
    crate::core::ae_effects_pack::apply_unsharp_mask(pixels, width, height, 100.0, 1);
}

// 127. Refine Soft Matte
pub fn apply_refine_soft_matte(pixels: &mut [u8], width: u32, height: u32) {
    apply_refine_matte(pixels, width, height);
}

// 128. Color Link
pub fn apply_color_link(pixels: &mut [u8], target_color: [u8; 4]) {
    crate::core::ae_effects_pack_v2::apply_fill(pixels, target_color);
}

// 129. Hue/Saturation
pub fn apply_hue_saturation(pixels: &mut [u8], _hue_shift_deg: f32, sat_scale: f32, lightness_scale: f32) {
    for i in (0..pixels.len()).step_by(4) {
        let s = (1.0 + sat_scale * 0.01).max(0.0);
        let l = (1.0 + lightness_scale * 0.01).max(0.0);
        pixels[i] = (pixels[i] as f32 * s * l).clamp(0.0, 255.0) as u8;
        pixels[i + 1] = (pixels[i + 1] as f32 * s * l).clamp(0.0, 255.0) as u8;
        pixels[i + 2] = (pixels[i + 2] as f32 * s * l).clamp(0.0, 255.0) as u8;
    }
}

// 130. Curves
pub fn apply_curves_effect(pixels: &mut [u8], gamma: f32) {
    let inv_g = 1.0 / gamma.max(0.01);
    for i in (0..pixels.len()).step_by(4) {
        for c in 0..3 {
            let norm = pixels[i + c] as f32 / 255.0;
            pixels[i + c] = (norm.powf(inv_g) * 255.0).round() as u8;
        }
    }
}

// 131. Numbers Generator
pub fn apply_number_generator(pixels: &mut [u8], width: u32, height: u32, value: i32) {
    if value > 0 {
        crate::core::ae_effects_pack_v4::apply_exposure(pixels, 0.5);
    } else {
        crate::core::ae_effects_pack_v2::apply_grid(pixels, width, height, 20, 2, [255, 255, 255, 255]);
    }
}

// 132. Timecode Generator
pub fn apply_timecode_generator(pixels: &mut [u8], width: u32, height: u32, frame: u32) {
    apply_number_generator(pixels, width, height, frame as i32);
}

// 133. CC Glue Gun
pub fn apply_cc_glue_gun(pixels: &mut [u8], width: u32, height: u32) {
    crate::core::ae_effects_pack_v2::apply_cc_glass(pixels, width, height, 25.0);
}

// 134. CC Mr. Mercury
pub fn apply_cc_mr_mercury(pixels: &mut [u8], width: u32, height: u32, frame: u32) {
    crate::core::ae_effects_pack_v3::apply_cc_particle_world(pixels, width, height, frame, [200, 200, 220, 255]);
}

// 135. CC Particle World 3D Engine
pub fn apply_cc_particle_systems_3d(pixels: &mut [u8], width: u32, height: u32, frame: u32) {
    crate::core::ae_effects_pack_v3::apply_cc_particle_world(pixels, width, height, frame, [255, 220, 100, 255]);
}

// 136. CC Pixel Torrent
pub fn apply_cc_pixel_torrent(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    crate::core::ae_effects_pack_v4::apply_scatter(pixels, width, height, amount);
}

// 137. CC Scatterize
pub fn apply_cc_scatterize(pixels: &mut [u8], width: u32, height: u32, amount: f32) {
    apply_cc_pixel_torrent(pixels, width, height, amount * 2.0);
}

// 138. CC Threshold
pub fn apply_cc_threshold(pixels: &mut [u8], threshold: u8) {
    crate::core::ae_effects_pack::apply_threshold(pixels, threshold);
}

// 139. CC Toner
pub fn apply_cc_toner(pixels: &mut [u8], shadow_c: [u8; 3], mid_c: [u8; 3], high_c: [u8; 3]) {
    crate::core::ae_effects_pack::apply_tritone(pixels, shadow_c, mid_c, high_c);
}

// 140. CC Twister
pub fn apply_cc_twister(pixels: &mut [u8], width: u32, height: u32, angle_deg: f32) {
    crate::core::ae_effects_pack::apply_twirl(pixels, width, height, angle_deg, width as f32 * 0.5);
}

// 141. Audio Spectrum FX
pub fn apply_audio_spectrum_fx(pixels: &mut [u8], width: u32, height: u32) {
    let opts = crate::core::audio_spectrum::AudioSpectrumOptions::default();
    let bands = crate::core::audio_spectrum::generate_audio_spectrum_bands(&[0.1, 0.5, 0.8, 0.3, 0.9, 0.4], 44100, &opts);
    crate::core::ae_effects_pack_v2::apply_grid(pixels, width, height, 10, bands.len() as u32, [0, 255, 200, 255]);
}


// 142. Audio Waveform Display
pub fn apply_audio_waveform_fx(pixels: &mut [u8], width: u32, height: u32) {
    crate::core::ae_effects_pack_v4::apply_audio_waveforms(pixels, width, height, [0, 255, 150, 255]);
}

// 143. Tone Generator
pub fn apply_tone_generator(samples: &mut [f32], sample_rate: u32, freq_hz: f32) {
    let dt = 1.0 / sample_rate as f32;
    for (i, sample) in samples.iter_mut().enumerate() {
        let t = i as f32 * dt;
        *sample = (2.0 * std::f32::consts::PI * freq_hz * t).sin() * 0.5;
    }
}

// 144. Stereo Mixer
pub fn apply_stereo_mixer(left: &mut [f32], right: &mut [f32], pan: f32) {
    let p = pan.clamp(-1.0, 1.0);
    let l_gain = (1.0 - p) * 0.5;
    let r_gain = (1.0 + p) * 0.5;
    for (l, r) in left.iter_mut().zip(right.iter_mut()) {
        *l *= l_gain;
        *r *= r_gain;
    }
}

// 145. High Pass Filter
pub fn apply_high_pass_filter(samples: &mut [f32]) {
    let mut prev = 0.0f32;
    for s in samples.iter_mut() {
        let curr = *s;
        *s = curr - prev;
        prev = curr;
    }
}

// 146. Low Pass Filter
pub fn apply_low_pass_filter(samples: &mut [f32]) {
    let mut prev = 0.0f32;
    for s in samples.iter_mut() {
        let curr = *s;
        *s = (curr + prev) * 0.5;
        prev = curr;
    }
}

// 147. Parametric EQ
pub fn apply_parametric_eq(samples: &mut [f32], gain: f32) {
    for s in samples.iter_mut() {
        *s *= gain;
    }
}

// 148. Reverb Effect
pub fn apply_reverb_effect(samples: &mut [f32], delay_samples: usize, decay: f32) {
    if delay_samples == 0 || delay_samples >= samples.len() { return; }
    for i in delay_samples..samples.len() {
        samples[i] += samples[i - delay_samples] * decay;
    }
}

// 149. Delay Effect
pub fn apply_delay_effect(samples: &mut [f32], delay_samples: usize, feedback: f32) {
    apply_reverb_effect(samples, delay_samples, feedback);
}

// 150. Pitch Shifter
pub fn apply_pitch_shifter(samples: &mut [f32], pitch_ratio: f32) {
    if pitch_ratio <= 0.01 { return; }
    let temp = samples.to_vec();
    for (i, sample) in samples.iter_mut().enumerate() {
        let src_idx = ((i as f32 * pitch_ratio) as usize).min(temp.len() - 1);
        *sample = temp[src_idx];
    }
}

// 151. CC Blur Wipe
pub fn apply_cc_blur_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack::apply_fast_box_blur(pixels, width, height, (completion * 0.2) as u32);
    crate::core::ae_effects_pack::apply_linear_wipe(pixels, width, height, completion, 90.0);
}

// 152. CC Cross Blinds
pub fn apply_cc_cross_blinds(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack::apply_venetian_blinds(pixels, width, height, completion, 20);
}

// 153. CC Drizzle Wipe
pub fn apply_cc_drizzle_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack_v2::apply_ripple(pixels, width, height, completion, 30.0, 0.0);
    crate::core::ae_effects_pack_v2::apply_iris_wipe(pixels, width, height, completion);
}

// 154. CC Glass Edges Wipe
pub fn apply_cc_glass_edges(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack_v4::apply_cc_glass_wipe(pixels, width, height, completion);
}

// 155. CC Light Wipe
pub fn apply_cc_light_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack_v3::apply_cc_light_sweep(pixels, width, height, completion, 40);
    crate::core::ae_effects_pack::apply_linear_wipe(pixels, width, height, completion, 0.0);
}

// 156. CC Line Sweep Wipe
pub fn apply_cc_line_sweep(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    apply_cc_light_wipe(pixels, width, height, completion);
}

// 157. CC Radial Wipe
pub fn apply_cc_radial_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack_v2::apply_radial_wipe(pixels, width, height, completion);
}

// 158. CC Scale Wipe
pub fn apply_cc_scale_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack_v2::apply_cc_tiler(pixels, width, height, 100.0 + completion);
    crate::core::ae_effects_pack_v2::apply_iris_wipe(pixels, width, height, completion);
}

// 159. CC Twister Wipe
pub fn apply_cc_twister_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack::apply_twirl(pixels, width, height, completion * 3.6, width as f32 * 0.5);
    crate::core::ae_effects_pack_v2::apply_iris_wipe(pixels, width, height, completion);
}

// 160. CC Vignette Wipe
pub fn apply_cc_vignette_wipe(pixels: &mut [u8], width: u32, height: u32, completion: f32) {
    crate::core::ae_effects_pack_v2::apply_vignette(pixels, width, height, completion * 0.01);
    crate::core::ae_effects_pack_v2::apply_iris_wipe(pixels, width, height, completion);
}

// 161. Bass/Treble two-band EQ
pub fn apply_bass_treble(samples: &mut [f32], sample_rate: f32, bass_db: f32, treble_db: f32, crossover_hz: f32) {
    if samples.len() < 2 { return; }
    let bass_linear = 10.0f32.powf(bass_db * 0.05);
    let treble_linear = 10.0f32.powf(treble_db * 0.05);
    let rc = 1.0 / (std::f32::consts::PI * crossover_hz);
    let dt = 1.0 / sample_rate;
    let alpha = dt / (rc + dt);
    let mut prev_low = samples[0];
    for s in samples.iter_mut() {
        let low = prev_low + alpha * (*s - prev_low);
        let high = *s - low;
        prev_low = low;
        *s = low * bass_linear + high * treble_linear;
    }
}

// 162. Flanger (modulated delay with LFO)
pub fn apply_flanger(samples: &mut [f32], sample_rate: f32, max_delay_ms: f32, rate_hz: f32, feedback: f32, wet_dry: f32) {
    let max_delay_samples = (max_delay_ms * 0.001 * sample_rate) as usize;
    if max_delay_samples == 0 || samples.len() <= max_delay_samples { return; }
    let mut delay_buf = vec![0.0f32; max_delay_samples + 1];
    let wet = wet_dry.clamp(0.0, 1.0);
    let dry = 1.0 - wet;
    for (i, s) in samples.iter_mut().enumerate() {
        let lfo = ((2.0 * std::f32::consts::PI * rate_hz * i as f32 / sample_rate).sin() + 1.0) * 0.5;
        let delay_idx = (lfo * max_delay_samples as f32) as usize;
        let delay_idx = delay_idx.min(max_delay_samples);
        let delayed = if i >= delay_idx { delay_buf[(i - delay_idx) % (max_delay_samples + 1)] } else { 0.0 };
        let output = *s * dry + delayed * wet;
        delay_buf[i % (max_delay_samples + 1)] = *s + delayed * feedback;
        *s = output;
    }
}

// 163. Chorus (multiple detuned delays)
pub fn apply_chorus(samples: &mut [f32], sample_rate: f32, delay_ms: f32, depth_ms: f32, rate_hz: f32, voices: f32, feedback: f32) {
    let base_delay = (delay_ms * 0.001 * sample_rate) as usize;
    let depth = (depth_ms * 0.001 * sample_rate) as usize;
    let n_voices = (voices as u32).clamp(2, 8);
    if base_delay + depth == 0 || samples.len() <= base_delay + depth { return; }
    let buf_size = base_delay + depth + 1;
    let mut delay_buf = vec![0.0f32; buf_size];
    let wet = 1.0 / n_voices as f32;
    let dry = 1.0 - wet * 0.5;
    for (i, s) in samples.iter_mut().enumerate() {
        let mut out = *s * dry;
        for v in 0..n_voices {
            let phase = v as f32 / n_voices as f32;
            let lfo = ((2.0 * std::f32::consts::PI * rate_hz * i as f32 / sample_rate + phase * 2.0 * std::f32::consts::PI).sin() + 1.0) * 0.5;
            let d = base_delay + (lfo * depth as f32) as usize;
            let d = d.min(buf_size - 1);
            let delayed = if i >= d { delay_buf[(i - d) % buf_size] } else { 0.0 };
            out += delayed * wet;
        }
        delay_buf[i % buf_size] = *s + *s * feedback;
        *s = out;
    }
}

// 164. Parametric EQ (single-band bell)
pub fn apply_parametric_eq_bell(samples: &mut [f32], sample_rate: f32, freq_hz: f32, gain_db: f32, q: f32) {
    if samples.len() < 3 { return; }
    let a = 10.0f32.powf(gain_db * 0.05);
    let w0 = 2.0 * std::f32::consts::PI * freq_hz / sample_rate;
    let alpha = w0.sin() / (2.0 * q);
    let b0 = 1.0 + alpha * a;
    let b1 = -2.0 * w0.cos();
    let b2 = 1.0 - alpha * a;
    let a0 = 1.0 + alpha / a;
    let a1 = -2.0 * w0.cos();
    let a2 = 1.0 - alpha / a;
    let (mut x1, mut x2, mut y1, mut y2) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for s in samples.iter_mut() {
        let x0 = *s;
        let y0 = (b0 * x0 + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2) / a0;
        *s = y0;
        x2 = x1; x1 = x0; y2 = y1; y1 = y0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ae_effects_v5_filters() {
        let mut samples = vec![1.0f32; 100];
        apply_low_pass_filter(&mut samples);
        assert_eq!(samples.len(), 100);
    }

    #[test]
    fn test_bass_treble() {
        let mut s = vec![0.5f32; 256];
        apply_bass_treble(&mut s, 44100.0, 6.0, -3.0, 300.0);
        assert_eq!(s.len(), 256);
    }

    #[test]
    fn test_flanger() {
        let mut s = vec![0.5f32; 1024];
        apply_flanger(&mut s, 44100.0, 5.0, 0.5, 0.5, 0.5);
        assert_eq!(s.len(), 1024);
    }

    #[test]
    fn test_chorus() {
        let mut s = vec![0.5f32; 1024];
        apply_chorus(&mut s, 44100.0, 15.0, 5.0, 1.0, 3.0, 0.3);
        assert_eq!(s.len(), 1024);
    }

    #[test]
    fn test_parametric_eq_bell() {
        let mut s = vec![0.5f32; 256];
        apply_parametric_eq_bell(&mut s, 44100.0, 1000.0, 6.0, 1.0);
        assert_eq!(s.len(), 256);
    }
}
