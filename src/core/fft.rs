#![allow(dead_code)]
//! Minimal, dependency-free radix-2 Cooley–Tukey FFT for real-time audio
//! spectrum analysis. Deterministic and panic-free (errors returned as Result).

/// True when `n` is a power of two.
pub fn is_pow2(n: usize) -> bool {
    n > 0 && n & (n - 1) == 0
}

/// Smallest power of two >= n (minimum 1).
pub fn next_pow2(n: usize) -> usize {
    if n <= 1 {
        return 1;
    }
    let mut p = 1;
    while p < n {
        p <<= 1;
    }
    p
}

/// Hann window coefficients for `len` samples.
pub fn hann_window(len: usize) -> Vec<f32> {
    if len == 0 {
        return Vec::new();
    }
    if len == 1 {
        return vec![1.0];
    }
    (0..len)
        .map(|i| {
            0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / len as f32).cos())
        })
        .collect()
}

/// In-place iterative radix-2 FFT over interleaved real/imaginary buffers.
///
/// Both slices must be the same non-zero power-of-two length.
pub fn fft_in_place(re: &mut [f32], im: &mut [f32]) -> Result<(), &'static str> {
    let n = re.len();
    if n != im.len() {
        return Err("fft: real/imag length mismatch");
    }
    if !is_pow2(n) {
        return Err("fft: length must be a power of two");
    }

    // Bit-reversal permutation.
    let bits = n.trailing_zeros();
    for i in 0..n {
        let j = reverse_bits(i, bits);
        if j > i {
            re.swap(i, j);
            im.swap(i, j);
        }
    }

    // Butterfly stages.
    let mut size = 2;
    while size <= n {
        let half = size / 2;
        let step = std::f32::consts::TAU / size as f32;
        for start in (0..n).step_by(size) {
            for k in 0..half {
                let ang = -step * k as f32;
                let (wr, wi) = (ang.cos(), ang.sin());
                let a = start + k;
                let b = a + half;
                let tr = re[b] * wr - im[b] * wi;
                let ti = re[b] * wi + im[b] * wr;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
            }
        }
        size <<= 1;
    }
    Ok(())
}

fn reverse_bits(mut x: usize, bits: u32) -> usize {
    let mut r = 0usize;
    for _ in 0..bits {
        r = (r << 1) | (x & 1);
        x >>= 1;
    }
    r
}

/// Magnitude spectrum (length n/2) from real input via FFT.
/// Returns normalized magnitudes (bin amplitude relative to full-scale sine).
pub fn magnitude_spectrum(samples: &[f32]) -> Result<Vec<f32>, &'static str> {
    let n = next_pow2(samples.len());
    if n == 0 {
        return Ok(Vec::new());
    }
    let window = hann_window(samples.len());
    let mut re = vec![0.0f32; n];
    let mut im = vec![0.0f32; n];
    for (i, w) in window.iter().enumerate() {
        re[i] = samples[i] * w;
    }
    fft_in_place(&mut re, &mut im)?;
    // Coherent gain of the Hann window ≈ 0.5; normalize so a full-scale sine
    // at bin center reads ~1.0.
    let norm = 2.0 / (samples.len().max(1) as f32 * 0.5);
    let half = n / 2;
    let mut mags = Vec::with_capacity(half);
    for k in 0..half {
        let m = (re[k] * re[k] + im[k] * im[k]).sqrt() * norm;
        mags.push(m.min(4.0));
    }
    Ok(mags)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn naive_dft_bin(samples: &[f32], k: usize) -> f32 {
        let n = samples.len();
        let mut re = 0.0f64;
        let mut im = 0.0f64;
        for (i, s) in samples.iter().enumerate() {
            let ang = -std::f64::consts::TAU * k as f64 * i as f64 / n as f64;
            re += *s as f64 * ang.cos();
            im += *s as f64 * ang.sin();
        }
        ((re * re + im * im) as f32).sqrt()
    }

    #[test]
    fn test_pow2_helpers() {
        assert!(is_pow2(1) && is_pow2(2) && is_pow2(1024));
        assert!(!is_pow2(0) && !is_pow2(3) && !is_pow2(1000));
        assert_eq!(next_pow2(0), 1);
        assert_eq!(next_pow2(1), 1);
        assert_eq!(next_pow2(1000), 1024);
        assert_eq!(next_pow2(1024), 1024);
    }

    #[test]
    fn test_hann_window_properties() {
        let w = hann_window(8);
        assert_eq!(w.len(), 8);
        assert!((w[0]).abs() < 1e-6); // zero at edges
        assert!((w[4] - 1.0).abs() < 1e-6); // peak at centre
        assert_eq!(hann_window(0).len(), 0);
        assert_eq!(hann_window(1), vec![1.0]);
    }

    #[test]
    fn test_fft_matches_naive_dft() {
        // Deterministic pseudo-random signal.
        let mut state = 0x1234_5678u64;
        let mut sig = Vec::with_capacity(16);
        for i in 0..16 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            sig.push(((state & 0xFF) as f32 / 255.0 - 0.5) * (1.0 + i as f32 * 0.01));
        }
        let mut re = sig.clone();
        let mut im = vec![0.0f32; 16];
        fft_in_place(&mut re, &mut im).unwrap_or_else(|e| panic!("{e}"));
        for k in 0..8 {
            let expected = naive_dft_bin(&sig, k);
            let got = (re[k] * re[k] + im[k] * im[k]).sqrt();
            assert!(
                (got - expected).abs() < expected.max(1.0) * 1e-3,
                "bin {k}: {got} vs {expected}"
            );
        }
    }

    #[test]
    fn test_sine_peaks_at_expected_bin() {
        // 64-sample sine at exactly bin 5.
        let n = 64usize;
        let bin = 5usize;
        let sig: Vec<f32> = (0..n)
            .map(|i| (std::f32::consts::TAU * bin as f32 * i as f32 / n as f32).sin())
            .collect();
        let mags = magnitude_spectrum(&sig).unwrap_or_default();
        assert_eq!(mags.len(), 32);
        let peak_idx = mags
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);
        assert_eq!(peak_idx, bin);
        assert!(mags[bin] > 0.85, "full-scale sine should read ~1.0, got {}", mags[bin]);
    }

    #[test]
    fn test_fft_error_paths() {
        let mut re = vec![0.0f32; 3];
        let mut im = vec![0.0f32; 3];
        assert!(fft_in_place(&mut re, &mut im).is_err()); // not pow2
        let mut re2 = vec![0.0f32; 4];
        let mut im2 = vec![0.0f32; 8];
        assert!(fft_in_place(&mut re2, &mut im2).is_err()); // mismatch
        assert!(magnitude_spectrum(&[]).unwrap_or_default().is_empty());
    }

    #[test]
    fn test_impulse_flat_spectrum() {
        let mut sig = vec![0.0f32; 16];
        sig[0] = 1.0;
        let mags = magnitude_spectrum(&sig).unwrap_or_default();
        // Impulse → equal magnitudes across bins (within tolerance).
        let first = mags[0];
        for m in &mags[1..] {
            assert!((m - first).abs() < 1e-4);
        }
    }
}