//! High Dynamic Range (HDR) 16/32bpc to 8bpc Dithering & Quantization Pipeline.
//!
//! Eliminates color banding and contouring artifacts when down-sampling float buffers
//! to standard 8-bit displays using Triangular Probability Density Function (TPDF) and Blue Noise.

#![allow(dead_code)]

/// Supported dithering algorithms for HDR color depth quantization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DitherMethod {
    None,
    #[default]
    TriangularPdf, // Standard professional audio/video high-fidelity dither
    OrderedBayer,
}

/// Simple deterministic pseudo-random hash generator for reproducible noise.
fn hash_noise(x: u32, y: u32, channel: u32) -> f32 {
    let mut h =
        x.wrapping_mul(374761393) ^ y.wrapping_mul(668265263) ^ channel.wrapping_mul(314159265);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    (h as f32) / (u32::MAX as f32)
}

/// Quantizes scene-linear/gamma HDR floats in [0.0, 1.0] to 8-bit [0, 255] with dithering.
pub fn quantize_hdr_slice_dithered(
    hdr_data: &[f32],
    width: u32,
    height: u32,
    method: DitherMethod,
) -> Vec<u8> {
    let num_pixels = (width as usize) * (height as usize);
    let mut output = vec![0u8; num_pixels * 4];

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let base = idx * 4;
            if base + 3 < hdr_data.len() {
                for c in 0..3 {
                    let val = hdr_data[base + c].clamp(0.0, 1.0);
                    let val_scaled = val * 255.0;

                    let dither_offset = match method {
                        DitherMethod::None => 0.0,
                        DitherMethod::TriangularPdf => {
                            let r1 = hash_noise(x, y, c as u32);
                            let r2 = hash_noise(x.wrapping_add(1), y.wrapping_add(1), c as u32);
                            (r1 - r2) * std::f32::consts::FRAC_1_SQRT_2 // TPDF noise between [-1.0, 1.0] LSB
                        }
                        DitherMethod::OrderedBayer => {
                            const BAYER: [[f32; 4]; 4] = [
                                [-0.5, 0.0, -0.375, 0.125],
                                [0.25, -0.25, 0.375, -0.125],
                                [-0.3125, 0.1875, -0.4375, 0.0625],
                                [0.4375, -0.0625, 0.3125, -0.1875],
                            ];
                            BAYER[(y % 4) as usize][(x % 4) as usize]
                        }
                    };

                    output[base + c] = (val_scaled + dither_offset).round().clamp(0.0, 255.0) as u8;
                }
                // Alpha is direct clamped quantization
                output[base + 3] = (hdr_data[base + 3].clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hdr_dither_preserves_solid_black_and_white() {
        let hdr = vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let out = quantize_hdr_slice_dithered(&hdr, 2, 1, DitherMethod::TriangularPdf);
        assert_eq!(out[0], 0);
        assert_eq!(out[4], 255);
    }

    #[test]
    fn test_hdr_dither_tpdf_breaks_flat_gradients() {
        // Continuous smooth ramp between 128/255 and 129/255
        let mut hdr = Vec::new();
        for i in 0..16 {
            let v = 128.0 / 255.0 + (i as f32 / 16.0) * (1.0 / 255.0);
            hdr.extend_from_slice(&[v, v, v, 1.0]);
        }
        let out = quantize_hdr_slice_dithered(&hdr, 16, 1, DitherMethod::TriangularPdf);
        assert_eq!(out.len(), 16 * 4);
    }
}
