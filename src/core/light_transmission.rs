#![allow(dead_code)]
/// 3D Light Transmission and Shadow Casting properties.
#[derive(Debug, Clone)]
pub struct LightTransmissionOptions {
    pub cast_shadows: bool,
    pub shadow_darkness: f32, // Percentage (100.0 = full dark)
    pub light_transmission: f32, // Percentage translucency transmission (0.0 .. 100.0)
    pub shadow_color: [f32; 4],
}

impl Default for LightTransmissionOptions {
    fn default() -> Self {
        Self {
            cast_shadows: true,
            shadow_darkness: 100.0,
            light_transmission: 0.0,
            shadow_color: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

/// Evaluates shadow attenuation factor (0.0 = unoccluded light, 1.0 = pitch black shadow)
/// considering light transmission transparency of occluding 3D layers.
pub fn calculate_shadow_attenuation(
    occluder_alpha: f32,
    options: &LightTransmissionOptions,
) -> f32 {
    if !options.cast_shadows || occluder_alpha <= 0.001 {
        return 0.0;
    }

    let transmission_factor = (options.light_transmission / 100.0).clamp(0.0, 1.0);
    let effective_occlusion = occluder_alpha * (1.0 - transmission_factor);
    (effective_occlusion * (options.shadow_darkness / 100.0)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadow_attenuation_opaque() {
        let options = LightTransmissionOptions::default();
        let att = calculate_shadow_attenuation(1.0, &options);
        assert_eq!(att, 1.0);
    }

    #[test]
    fn test_shadow_attenuation_translucent() {
        let options = LightTransmissionOptions {
            cast_shadows: true,
            shadow_darkness: 100.0,
            light_transmission: 50.0, // 50% translucency
            shadow_color: [0.0, 0.0, 0.0, 1.0],
        };
        let att = calculate_shadow_attenuation(1.0, &options);
        assert_eq!(att, 0.5);
    }
}
