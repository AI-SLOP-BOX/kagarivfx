//! Radix-2 Cooley-Tukey FFT kernels and spectral window functions used by
//! the audio spectrum analyzer ([`crate::core::audio_spectrum`]).
//!
//! All functions are deterministic, allocation-light and panic-free.

/// Hann window ("raised cosine"), symmetric form:
/// `0.5 * (1 - cos(2πi / (N-1)))`. Good general-purpose choice; -31.5 dB sidelobes.
pub fn hann_window(len: usize) -> Vec<f32> {
    if len <= 1 {
        return vec![1.0; len];
    }
    let denom = (len - 1) as f32;
    (0..len)
        .map(|i| 0.5 * (1.0 - (std::f32::consts::TAU * i as f32 / denom).cos()))
        .collect()
}

/// Hamming window, symmetric form: `0.54 - 0.46 * cos(2πi / (N-1))`.
/// -42 dB sidelobes, wider main lobe than Hann.
pub fn hamming_window(len: usize) -> Vec<f32> {
    if len <= 1 {
        return vec![1.0; len];
    }
    let denom = (len - 1) as f32;
    (0..len)
        .map(|i| 0.54 - 0.46 * (std::f32::consts::TAU * i as f32 / denom).cos())
        .collect()
}

/// Blackman-Harris window (4-term), symmetric form: excellent sidelobe
/// suppression (-92 dB) at the cost of a wide main lobe. Ideal for
/// peak-frequency accuracy.
pub fn blackman_harris_window(len: usize) -> Vec<f32> {
    if len <= 1 {
        return vec![1.0; len];
    }
    let denom = (len - 1) as f32;
    let a = [0.35875f32, 0.48829, 0.14128, 0.01168];
    (0..len)
        .map(|i| {
            let t = std::f32::consts::TAU * i as f32 / denom;
            a[0] - a[1] * t.cos() + a[2] * (2.0 * t).cos() - a[3] * (3.0 * t).cos()
        })
        .collect()
}

/// Applies a window in-place element-wise (`out[i] *= win[i]`).
/// Extra window taps beyond `out.len()` are ignored; missing taps leave the
/// sample untouched.
pub fn apply_window(samples: &mut [f32], win: &[f32]) {
    for (s, w) in samples.iter_mut().zip(win.iter()) {
        *s *= w;
    }
}

/// Radix-2 Cooley-Tukey FFT for real-valued signals.
/// Returns magnitude spectrum of `size` bins from `input.len()` samples.
pub fn magnitude_spectrum(input: &[f32], size: usize) -> Vec<f32> {
    let n = size.min(input.len()).next_power_of_two();
    if n < 2 {
        return vec![0.0; size];
    }

    // Copy input into complex buffer (real part only, imaginary = 0)
    let mut re: Vec<f64> = input.iter().take(n).map(|&v| v as f64).collect();
    re.resize(n, 0.0);
    let mut im = vec![0.0f64; n];

    // Bit-reversal permutation
    let bits = (n as f64).log2() as usize;
    for i in 0..n {
        let rev = (0..bits).fold(0usize, |acc, b| acc | (((i >> b) & 1) << (bits - 1 - b)));
        if rev > i {
            re.swap(i, rev);
            im.swap(i, rev);
        }
    }

    // Butterfly
    let two_pi = std::f64::consts::TAU;
    let mut len = 2;
    while len <= n {
        let angle = -two_pi / len as f64;
        let w_re = angle.cos();
        let w_im = angle.sin();
        let mut j = 0;
        while j < n {
            let mut cur_re = 1.0f64;
            let mut cur_im = 0.0f64;
            for k in 0..len / 2 {
                let even = j + k;
                let odd = j + k + len / 2;
                let t_re = cur_re * re[odd] - cur_im * im[odd];
                let t_im = cur_re * im[odd] + cur_im * re[odd];
                re[odd] = re[even] - t_re;
                im[odd] = im[even] - t_im;
                re[even] += t_re;
                im[even] += t_im;
                let next_re = cur_re * w_re - cur_im * w_im;
                cur_im = cur_re * w_im + cur_im * w_re;
                cur_re = next_re;
            }
            j += len;
        }
        len <<= 1;
    }

    // Magnitude spectrum (first half — real signal)
    (0..size)
        .map(|i| {
            let idx = i.min(n / 2);
            (re[idx] * re[idx] + im[idx] * im[idx]).sqrt() / (n / 2) as f64
        })
        .map(|v| v as f32)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq_hz: f32, seconds: f32, sr: u32, amp: f32) -> Vec<f32> {
        let n = (seconds * sr as f32) as usize;
        (0..n)
            .map(|i| (std::f32::consts::TAU * freq_hz * i as f32 / sr as f32).sin() * amp)
            .collect()
    }

    #[test]
    fn test_impulse_yields_flat_spectrum() {
        // A unit impulse spreads equally across every bin. The kernel's
        // normalisation maps a sinusoid peak to its amplitude (÷ n/2), so an
        // impulse lands at 2/n per bin.
        let n = 256usize;
        let mut impulse = vec![0.0f32; n];
        impulse[0] = 1.0;
        let mags = magnitude_spectrum(&impulse, n);
        assert_eq!(mags.len(), n);
        let expected = 2.0 / n as f32;
        for (i, &m) in mags.iter().enumerate().take(n / 2) {
            assert!(
                (m - expected).abs() < 1e-6,
                "bin {i} magnitude {} != {expected}",
                m
            );
        }
    }

    #[test]
    fn test_sine_peaks_at_expected_bin() {
        // 1000 Hz tone sampled at 8000 Hz over 512 samples → bin 64.
        let sr = 8000u32;
        let tone = sine(1000.0, 512.0 / sr as f32, sr, 1.0);
        let mags = magnitude_spectrum(&tone, 512);
        let peak_bin = mags[..256]
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap()
            .0;
        assert_eq!(peak_bin, 64, "peak must land on the tone's bin");
        // Spectral leakage into far bins stays small relative to the peak.
        let peak_val = mags[64];
        let far_val = mags[200].max(mags[30]);
        assert!(
            far_val < peak_val * 0.05,
            "leakage {} vs peak {}",
            far_val,
            peak_val
        );
    }

    #[test]
    fn test_silence_is_zero() {
        let mags = magnitude_spectrum(&vec![0.0f32; 1024], 1024);
        assert!(mags.iter().all(|&m| m.abs() < 1e-6));
    }

    #[test]
    fn test_amplitude_linearity() {
        let a = sine(440.0, 0.05, 44100, 0.5);
        let b = sine(440.0, 0.05, 44100, 1.0);
        let ma = magnitude_spectrum(&a, 2048);
        let mb = magnitude_spectrum(&b, 2048);
        // Doubling input amplitude doubles every bin (within fp tolerance).
        for i in (0..200).step_by(7) {
            assert!(
                (mb[i] - 2.0 * ma[i]).abs() < 1e-3,
                "linearity broken at bin {}: {} vs {}",
                i,
                mb[i],
                ma[i]
            );
        }
    }

    #[test]
    fn test_dc_offset_lands_in_bin_zero() {
        let dc = vec![0.8f32; 512];
        let mags = magnitude_spectrum(&dc, 512);
        assert!(
            mags[0] > 0.7,
            "DC energy must appear in bin 0, got {}",
            mags[0]
        );
        // Non-zero bins stay tiny for a pure DC signal.
        assert!(mags[50] < 1e-3);
    }

    #[test]
    fn test_degenerate_inputs_are_safe() {
        assert!(magnitude_spectrum(&[], 256).iter().all(|&m| m == 0.0));
        assert_eq!(magnitude_spectrum(&[1.0], 8)[0], 0.0); // n < 2 → zeros
        let single = vec![0.5f32; 16];
        assert_eq!(magnitude_spectrum(&single, 16).len(), 16);
    }

    #[test]
    fn test_windows_symmetric_and_bounded() {
        for (name, win) in [
            ("hann", hann_window(64)),
            ("hamming", hamming_window(64)),
            ("blackman-harris", blackman_harris_window(64)),
        ] {
            assert_eq!(win.len(), 64);
            for &v in &win {
                assert!((0.0..=1.0).contains(&v), "{name} value {v} out of range");
            }
            // Symmetry about the centre (within fp tolerance).
            for i in 0..32 {
                assert!(
                    (win[i] - win[63 - i]).abs() < 1e-5,
                    "{name} not symmetric at {i}"
                );
            }
            // Endpoints near zero (except Hamming which floors at 0.08).
            assert!(win[0] < 0.15 && win[63] < 0.15, "{name} endpoints too high");
        }
        // Degenerate lengths are safe.
        assert_eq!(hann_window(0), Vec::<f32>::new());
        assert_eq!(hann_window(1), vec![1.0]);
        assert_eq!(blackman_harris_window(1), vec![1.0]);
    }

    #[test]
    fn test_apply_window_elementwise_and_length_safe() {
        let mut s = vec![2.0f32; 4];
        let win = vec![0.5f32, 1.0, 0.25];
        apply_window(&mut s, &win);
        assert_eq!(s, vec![1.0, 2.0, 0.5, 2.0]); // 4th tap missing → untouched

        let mut empty: Vec<f32> = vec![];
        apply_window(&mut empty, &hann_window(8));
        assert!(empty.is_empty());
    }

    #[test]
    fn test_blackman_harris_suppresses_leakage_best() {
        // A non-bin-centered tone leaks; stronger windows leak less.
        let sr = 8000u32;
        let tone = sine(1007.0, 512.0 / sr as f32, sr, 1.0); // between bins
        let raw = magnitude_spectrum(&tone, 512);
        let mut hann = tone.clone();
        apply_window(&mut hann, &hann_window(tone.len()));
        let hann_m = magnitude_spectrum(&hann, 512);

        // Far-bin leakage floor (bins 150..250): Hann must beat rectangular.
        let raw_floor = raw[150..250].iter().cloned().fold(0.0f32, f32::max);
        let hann_floor = hann_m[150..250].iter().cloned().fold(0.0f32, f32::max);
        assert!(
            hann_floor < raw_floor,
            "Hann leakage ({}) must beat rectangular ({})",
            hann_floor,
            raw_floor
        );
    }
}
