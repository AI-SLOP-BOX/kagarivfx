#![allow(dead_code)]
/// Options matching After Effects Difference Matte effect.
#[derive(Debug, Clone)]
pub struct DifferenceMatteOptions {
    pub tolerance: f32,  // Difference threshold matching percentage (0.0 .. 1.0)
    pub blur_matte: f32, // Matte edge softening radius
}

impl Default for DifferenceMatteOptions {
    fn default() -> Self {
        Self {
            tolerance: 0.15,
            blur_matte: 0.0,
        }
    }
}

/// Generates matte mask by comparing current frame pixels with reference background frame.
pub fn apply_difference_matte(
    current_pixels: &mut [u8],
    reference_pixels: &[u8],
    width: u32,
    height: u32,
    options: &DifferenceMatteOptions,
) {
    let Some(num_pixels) = (width as usize)
        .checked_mul(height as usize)
        .filter(|&count| count <= usize::MAX / 4)
    else {
        return;
    };
    if width == 0
        || height == 0
        || current_pixels.len() != num_pixels * 4
        || reference_pixels.len() != num_pixels * 4
    {
        return;
    }

    let tolerance = if options.tolerance.is_finite() {
        options.tolerance.clamp(0.0, 1.0)
    } else {
        0.0
    };

    for i in 0..num_pixels {
        let idx = i * 4;
        let r1 = current_pixels[idx] as f32 / 255.0;
        let g1 = current_pixels[idx + 1] as f32 / 255.0;
        let b1 = current_pixels[idx + 2] as f32 / 255.0;

        let r2 = reference_pixels[idx] as f32 / 255.0;
        let g2 = reference_pixels[idx + 1] as f32 / 255.0;
        let b2 = reference_pixels[idx + 2] as f32 / 255.0;

        let dr = r1 - r2;
        let dg = g1 - g2;
        let db = b1 - b2;
        let diff = (dr * dr + dg * dg + db * db).sqrt();

        let matte = if diff > tolerance {
            ((diff - tolerance) / (1.0 - tolerance).max(0.001)).clamp(0.0, 1.0)
        } else {
            0.0
        };

        let current_a = current_pixels[idx + 3] as f32 / 255.0;
        current_pixels[idx + 3] = (current_a * matte * 255.0).round().clamp(0.0, 255.0) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_difference_matte_extraction() {
        let mut current = vec![255, 0, 0, 255]; // Red foreground
        let reference = vec![0, 0, 255, 255]; // Blue background
        let options = DifferenceMatteOptions::default();

        apply_difference_matte(&mut current, &reference, 1, 1, &options);
        assert!(current[3] > 200); // Difference detected, alpha opaque
    }

    #[test]
    fn test_difference_matte_rejects_overflow_and_sanitizes_tolerance() {
        let original = vec![255, 0, 0, 255];
        let mut current = original.clone();
        let reference = vec![0, 0, 255, 255];
        apply_difference_matte(
            &mut current,
            &reference,
            u32::MAX,
            u32::MAX,
            &DifferenceMatteOptions {
                tolerance: f32::NAN,
                blur_matte: f32::INFINITY,
            },
        );
        assert_eq!(current, original);

        let mut current = original.clone();
        apply_difference_matte(
            &mut current,
            &reference,
            1,
            1,
            &DifferenceMatteOptions {
                tolerance: f32::INFINITY,
                blur_matte: 0.0,
            },
        );
        assert_eq!(current[3], 0);
    }
}
