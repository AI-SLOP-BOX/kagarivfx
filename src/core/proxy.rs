use serde::{Serialize, Deserialize};

/// Proxy resolution level for layer previews.
/// When enabled, the layer renders at a fraction of full resolution
/// to speed up preview, then switches to full quality on final render.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ProxyResolution {
    #[default]
    Full,
    Half,
    Quarter,
    Eighth,
}

impl ProxyResolution {
    pub fn factor(self) -> f32 {
        match self {
            ProxyResolution::Full => 1.0,
            ProxyResolution::Half => 0.5,
            ProxyResolution::Quarter => 0.25,
            ProxyResolution::Eighth => 0.125,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ProxyResolution::Full => "Full",
            ProxyResolution::Half => "Half",
            ProxyResolution::Quarter => "Quarter",
            ProxyResolution::Eighth => "Eighth",
        }
    }
}

/// Per-layer proxy state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerProxy {
    pub enabled: bool,
    pub resolution: ProxyResolution,
    /// Optional path to a pre-rendered proxy file (image sequence).
    #[serde(default)]
    pub proxy_path: Option<String>,
}

impl Default for LayerProxy {
    fn default() -> Self {
        Self {
            enabled: false,
            resolution: ProxyResolution::Full,
            proxy_path: None,
        }
    }
}

/// Comp-level proxy settings applied during preview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompProxy {
    pub global_resolution: ProxyResolution,
    /// Whether proxy is active during playback (disables on final render).
    pub active_in_preview: bool,
}

impl Default for CompProxy {
    fn default() -> Self {
        Self {
            global_resolution: ProxyResolution::Half,
            active_in_preview: true,
        }
    }
}

/// Compute the effective render scale for a layer considering both
/// comp-level proxy and layer-level proxy. Layer proxy takes priority.
pub fn effective_proxy_scale(
    layer_proxy: Option<&LayerProxy>,
    comp_proxy_active: bool,
    comp_resolution: ProxyResolution,
    is_final_render: bool,
) -> f32 {
    if is_final_render {
        return 1.0;
    }
    if let Some(lp) = layer_proxy {
        if lp.enabled {
            return lp.resolution.factor();
        }
    }
    if comp_proxy_active {
        return comp_resolution.factor();
    }
    1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_resolution_factors() {
        assert_eq!(ProxyResolution::Full.factor(), 1.0);
        assert_eq!(ProxyResolution::Half.factor(), 0.5);
        assert_eq!(ProxyResolution::Quarter.factor(), 0.25);
        assert_eq!(ProxyResolution::Eighth.factor(), 0.125);
    }

    #[test]
    fn test_effective_proxy_scale_final_render_ignores_proxy() {
        let lp = Some(LayerProxy { enabled: true, resolution: ProxyResolution::Quarter, proxy_path: None });
        assert_eq!(effective_proxy_scale(lp.as_ref(), true, ProxyResolution::Half, true), 1.0);
    }

    #[test]
    fn test_effective_proxy_scale_layer_priority() {
        let lp = Some(LayerProxy { enabled: true, resolution: ProxyResolution::Eighth, proxy_path: None });
        assert_eq!(effective_proxy_scale(lp.as_ref(), true, ProxyResolution::Half, false), 0.125);
    }

    #[test]
    fn test_effective_proxy_scale_comp_fallback() {
        assert_eq!(effective_proxy_scale(None, true, ProxyResolution::Quarter, false), 0.25);
    }

    #[test]
    fn test_effective_proxy_scale_no_proxy() {
        assert_eq!(effective_proxy_scale(None, false, ProxyResolution::Half, false), 1.0);
    }
}
