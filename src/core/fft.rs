/// Radix-2 Cooley-Tukey FFT for real-valued signals.
/// Returns magnitude spectrum of `size` bins from `input.len()` samples.
pub fn magnitude_spectrum(input: &[f32], size: usize) -> Vec<f32> {
    let n = size.min(input.len()).next_power_of_two();
    if n < 2 { return vec![0.0; size]; }

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
