#![allow(dead_code)]
/// OpenFX (OFX) Standard C-ABI Host & Plugin structures.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct OfxImageEffectHandle {
    pub effect_id: String,
    pub plugin_name: String,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfxStatus {
    OK = 0,
    Failed = 1,
    ErrMemory = 2,
}

/// Bridge interface representing an external OpenFX visual effect plugin.
pub struct OpenFxPluginBridge {
    pub plugin_name: String,
    pub handle: OfxImageEffectHandle,
}

impl OpenFxPluginBridge {
    pub fn new(plugin_name: &str) -> Self {
        Self {
            plugin_name: plugin_name.to_string(),
            handle: OfxImageEffectHandle {
                effect_id: format!("ofx.plugin.{}", plugin_name.to_lowercase()),
                plugin_name: plugin_name.to_string(),
            },
        }
    }

    /// Invokes the OpenFX plugin's `kOfxImageEffectActionRender` action.
    pub fn render_frame(&self, in_pixels: &[u8], out_pixels: &mut [u8], width: u32, height: u32, frame: f64) -> OfxStatus {
        if in_pixels.len() != out_pixels.len() {
            return OfxStatus::ErrMemory;
        }

        // Pass-through execution simulating an OFX plugin render call
        out_pixels.copy_from_slice(in_pixels);
        log::info!("OpenFX Plugin [{}] rendered frame {:.1} at {}x{}", self.plugin_name, frame, width, height);

        OfxStatus::OK
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openfx_render_action() {
        let bridge = OpenFxPluginBridge::new("SapphireBlur");
        let src = vec![128u8; 16];
        let mut dst = vec![0u8; 16];

        let status = bridge.render_frame(&src, &mut dst, 2, 2, 0.0);
        assert_eq!(status, OfxStatus::OK);
        assert_eq!(dst[0], 128);
    }
}
