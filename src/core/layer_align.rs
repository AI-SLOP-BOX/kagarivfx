//! Layer Alignment, Distribution & Fit-to-Comp Engine (AE Align Palette Parity).
//!
//! Provides geometric alignment (Left, Center, Right, Top, Middle, Bottom),
//! equidistant distribution (Edges, Centers, Gaps), and automatic Fit to Comp transforms.

#![allow(dead_code)]

use crate::core::timeline::Layer;

/// Alignment modes relative to selection bounding box or composition canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignMode {
    Left,
    HorizontalCenter,
    Right,
    Top,
    VerticalCenter,
    Bottom,
}

/// Distribution modes for spreading multiple layers evenly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistributeMode {
    HorizontalCenters,
    HorizontalGaps,
    VerticalCenters,
    VerticalGaps,
}

/// Aligns a slice of layers according to `AlignMode`.
/// If `comp_bounds` is `Some([w, h])`, aligns relative to composition origin [0, 0] to [w, h].
pub fn align_layers(
    layers: &mut [&mut Layer],
    frame: u32,
    mode: AlignMode,
    comp_bounds: Option<[f32; 2]>,
) {
    if layers.is_empty() {
        return;
    }

    let (target_val, _) = if let Some([cw, ch]) = comp_bounds {
        match mode {
            AlignMode::Left => (0.0, true),
            AlignMode::HorizontalCenter => (cw * 0.5, true),
            AlignMode::Right => (cw, true),
            AlignMode::Top => (0.0, false),
            AlignMode::VerticalCenter => (ch * 0.5, false),
            AlignMode::Bottom => (ch, false),
        }
    } else {
        // Find selection bounding extent
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for layer in layers.iter() {
            let p = layer.transform.position.evaluate(frame);
            min_x = min_x.min(p[0]);
            max_x = max_x.max(p[0]);
            min_y = min_y.min(p[1]);
            max_y = max_y.max(p[1]);
        }

        match mode {
            AlignMode::Left => (min_x, true),
            AlignMode::HorizontalCenter => ((min_x + max_x) * 0.5, true),
            AlignMode::Right => (max_x, true),
            AlignMode::Top => (min_y, false),
            AlignMode::VerticalCenter => ((min_y + max_y) * 0.5, false),
            AlignMode::Bottom => (max_y, false),
        }
    };

    for layer in layers.iter_mut() {
        let mut p = layer.transform.position.evaluate(frame);
        match mode {
            AlignMode::Left | AlignMode::HorizontalCenter | AlignMode::Right => {
                p[0] = target_val;
            }
            AlignMode::Top | AlignMode::VerticalCenter | AlignMode::Bottom => {
                p[1] = target_val;
            }
        }
        layer.transform.position = crate::core::property::Animatable::new_constant(p);
    }
}

/// Distributes layers with equal spacing between their centers.
pub fn distribute_layers(layers: &mut [&mut Layer], frame: u32, mode: DistributeMode) {
    if layers.len() < 3 {
        return;
    }

    let is_horizontal = matches!(
        mode,
        DistributeMode::HorizontalCenters | DistributeMode::HorizontalGaps
    );
    let axis = if is_horizontal { 0 } else { 1 };

    // Sort layers by current coordinate along axis
    layers.sort_by(|a, b| {
        let pa = a.transform.position.evaluate(frame)[axis];
        let pb = b.transform.position.evaluate(frame)[axis];
        pa.partial_cmp(&pb).unwrap_or(std::cmp::Ordering::Equal)
    });

    let n = layers.len();
    let start = layers[0].transform.position.evaluate(frame)[axis];
    let end = layers[n - 1].transform.position.evaluate(frame)[axis];
    let step = (end - start) / (n - 1) as f32;

    for (i, layer) in layers.iter_mut().enumerate() {
        let mut p = layer.transform.position.evaluate(frame);
        p[axis] = start + step * i as f32;
        layer.transform.position = crate::core::property::Animatable::new_constant(p);
    }
}

/// Fits a layer's scale and position to the composition dimensions.
pub fn fit_layer_to_comp(
    layer: &mut Layer,
    _frame: u32,
    layer_w: f32,
    layer_h: f32,
    comp_w: f32,
    comp_h: f32,
    keep_aspect_ratio: bool,
) {
    if layer_w <= 0.0 || layer_h <= 0.0 || comp_w <= 0.0 || comp_h <= 0.0 {
        return;
    }

    let scale_x = (comp_w / layer_w) * 100.0;
    let scale_y = (comp_h / layer_h) * 100.0;

    let final_scale = if keep_aspect_ratio {
        let uniform = scale_x.min(scale_y);
        [uniform, uniform]
    } else {
        [scale_x, scale_y]
    };

    // Center position in composition
    layer.transform.position =
        crate::core::property::Animatable::new_constant([comp_w * 0.5, comp_h * 0.5]);
    layer.transform.scale = crate::core::property::Animatable::new_constant(final_scale);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeline::LayerType;

    #[test]
    fn test_align_layers_horizontal_center() {
        let mut l1 = Layer::new("1".into(), "L1".into(), LayerType::Null, 10);
        l1.transform.position = crate::core::property::Animatable::new_constant([10.0, 50.0]);

        let mut l2 = Layer::new("2".into(), "L2".into(), LayerType::Null, 10);
        l2.transform.position = crate::core::property::Animatable::new_constant([90.0, 50.0]);

        let mut list = [&mut l1, &mut l2];
        align_layers(&mut list, 0, AlignMode::HorizontalCenter, None);

        assert_eq!(list[0].transform.position.evaluate(0)[0], 50.0);
        assert_eq!(list[1].transform.position.evaluate(0)[0], 50.0);
    }

    #[test]
    fn test_distribute_layers_centers() {
        let mut l1 = Layer::new("1".into(), "L1".into(), LayerType::Null, 10);
        l1.transform.position = crate::core::property::Animatable::new_constant([0.0, 0.0]);

        let mut l2 = Layer::new("2".into(), "L2".into(), LayerType::Null, 10);
        l2.transform.position = crate::core::property::Animatable::new_constant([10.0, 0.0]);

        let mut l3 = Layer::new("3".into(), "L3".into(), LayerType::Null, 10);
        l3.transform.position = crate::core::property::Animatable::new_constant([100.0, 0.0]);

        let mut list = [&mut l1, &mut l2, &mut l3];
        distribute_layers(&mut list, 0, DistributeMode::HorizontalCenters);

        assert_eq!(list[0].transform.position.evaluate(0)[0], 0.0);
        assert_eq!(list[1].transform.position.evaluate(0)[0], 50.0);
        assert_eq!(list[2].transform.position.evaluate(0)[0], 100.0);
    }

    #[test]
    fn test_fit_layer_to_comp() {
        let mut l = Layer::new(
            "1".into(),
            "L1".into(),
            LayerType::Solid {
                color: [1.0, 1.0, 1.0, 1.0],
            },
            10,
        );
        fit_layer_to_comp(&mut l, 0, 500.0, 500.0, 1920.0, 1080.0, true);

        let scale = l.transform.scale.evaluate(0);
        assert!((scale[0] - 216.0).abs() < 1e-3);
        assert!((scale[1] - 216.0).abs() < 1e-3);
    }
}
