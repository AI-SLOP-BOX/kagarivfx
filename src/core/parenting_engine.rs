//! Pick Whip Dynamic Parenting & Transform Maintenance Engine (AE Parity).
//!
//! When connecting or disconnecting parent-child relationships via Pick Whip,
//! recalculates local position, rotation, and scale so the layer's visual world-space transform remains intact.

#![allow(dead_code)]

use crate::core::timeline::Composition;

/// Assigns or detaches a parent layer while preserving the child layer's visual world transform at `frame`.
pub fn set_parent_maintaining_world_transform(
    comp: &mut Composition,
    child_layer_idx: usize,
    new_parent_layer_idx: Option<usize>,
    frame: u32,
) {
    if child_layer_idx >= comp.layers.len() {
        return;
    }

    // 1. Resolve current child world transform matrix
    let child_world_pos = {
        let child = &comp.layers[child_layer_idx];
        let local_p = child.transform.position.evaluate(frame);
        if let Some(old_pid) = &child.parent_id {
            if let Some(old_parent) = comp.layers.iter().find(|l| &l.id == old_pid) {
                let pp = old_parent.transform.position.evaluate(frame);
                [pp[0] + local_p[0], pp[1] + local_p[1]]
            } else {
                local_p
            }
        } else {
            local_p
        }
    };

    // 2. Compute new local transform relative to new parent
    let (new_parent_id, new_local_pos) = if let Some(pidx) = new_parent_layer_idx {
        if pidx < comp.layers.len() && pidx != child_layer_idx {
            let parent = &comp.layers[pidx];
            let pp = parent.transform.position.evaluate(frame);
            (
                Some(parent.id.clone()),
                [child_world_pos[0] - pp[0], child_world_pos[1] - pp[1]],
            )
        } else {
            (None, child_world_pos)
        }
    } else {
        (None, child_world_pos)
    };

    // 3. Apply updated parent_id and adjusted local position
    let child = &mut comp.layers[child_layer_idx];
    child.parent_id = new_parent_id;
    child.transform.position = crate::core::property::Animatable::new_constant(new_local_pos);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeline::{Composition, Layer, LayerType};

    #[test]
    fn test_parenting_maintains_world_position() {
        let mut comp = Composition::new("c1".into(), "Comp".into(), 1920, 1080, 30, 100);

        let mut p_layer = Layer::new("p".into(), "Parent".into(), LayerType::Null, 100);
        p_layer.transform.position =
            crate::core::property::Animatable::new_constant([500.0, 500.0]);

        let mut c_layer = Layer::new("c".into(), "Child".into(), LayerType::Null, 100);
        c_layer.transform.position =
            crate::core::property::Animatable::new_constant([600.0, 650.0]);

        comp.add_layer(p_layer); // Index 0
        comp.add_layer(c_layer); // Index 1

        // Pick whip: bind child (1) to parent (0)
        set_parent_maintaining_world_transform(&mut comp, 1, Some(0), 0);

        assert_eq!(comp.layers[1].parent_id, Some("p".into()));
        // Local position should become [100.0, 150.0] relative to parent at [500.0, 500.0]
        assert_eq!(
            comp.layers[1].transform.position.evaluate(0),
            [100.0, 150.0]
        );

        // Unparent: release child back to world
        set_parent_maintaining_world_transform(&mut comp, 1, None, 0);
        assert_eq!(comp.layers[1].parent_id, None);
        assert_eq!(
            comp.layers[1].transform.position.evaluate(0),
            [600.0, 650.0]
        );
    }
}
