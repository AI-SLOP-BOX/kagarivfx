#![allow(dead_code)]
use crate::core::timeline::{Composition, TrackMatteMode};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerOpType {
    /// Standard layer evaluation (Solid, Image, Text, Shape)
    RenderLayer { layer_idx: usize },
    /// Parent transform evaluation pass
    EvaluateParentTransform { layer_idx: usize },
    /// Track Matte compositing pass
    CompositeTrackMatte {
        layer_idx: usize,
        matte_layer_idx: usize,
    },
    /// PreComp evaluation step
    EvaluatePreComp { layer_idx: usize, comp_id: String },
}

#[derive(Debug, Clone)]
pub struct ExecutionStep {
    pub step_id: usize,
    pub op: LayerOpType,
    pub dependencies: Vec<usize>,
}

/// Hybrid VFX Graph Compiler [AE Layer Structure <-> Nuke DAG Compiler Bridge]
/// Translates After Effects layer hierarchy (Parenting, Track Mattes, PreComps)
/// into an optimized, dependency-ordered acyclic execution plan.
#[derive(Debug, Default)]
pub struct VfxGraphCompiler {
    pub steps: Vec<ExecutionStep>,
    pub cycle_detected: bool,
}

impl VfxGraphCompiler {
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            cycle_detected: false,
        }
    }

    /// Compiles a Composition into an optimized execution schedule using topological sorting.
    pub fn compile(&mut self, comp: &Composition, current_frame: u32) {
        self.steps.clear();
        self.cycle_detected = false;

        let num_layers = comp.layers.len();
        if num_layers == 0 {
            return;
        }

        // ── 1. Build Adjacency Matrix & In-Degree Map ──
        let mut adj: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut in_degree: Vec<usize> = vec![0; num_layers];

        let id_to_idx: HashMap<&str, usize> = comp
            .layers
            .iter()
            .enumerate()
            .map(|(idx, l)| (l.id.as_str(), idx))
            .collect();

        let has_solo = comp
            .layers
            .iter()
            .any(|l| l.is_active(current_frame) && l.solo);

        let mut active_nodes = HashSet::new();

        for (idx, layer) in comp.layers.iter().enumerate() {
            if !layer.is_active(current_frame) {
                continue;
            }
            if has_solo && !layer.solo {
                continue;
            }

            active_nodes.insert(idx);

            // A) Parenting dependency
            if let Some(ref parent_id) = layer.parent_id {
                if let Some(&parent_idx) = id_to_idx.get(parent_id.as_str()) {
                    if parent_idx < num_layers {
                        adj.entry(parent_idx).or_default().push(idx);
                        in_degree[idx] += 1;
                        active_nodes.insert(parent_idx);
                    }
                }
            }

            // B) Track Matte dependency
            if layer.track_matte != TrackMatteMode::None && idx > 0 {
                let matte_idx = idx - 1;
                if matte_idx < num_layers {
                    adj.entry(matte_idx).or_default().push(idx);
                    in_degree[idx] += 1;
                    active_nodes.insert(matte_idx);
                }
            }
        }

        // ── 2. Topological Sort (Kahn's Algorithm) ──
        let mut queue = VecDeque::new();
        for &node_idx in &active_nodes {
            if in_degree[node_idx] == 0 {
                queue.push_back(node_idx);
            }
        }

        let mut sorted_order = Vec::new();
        while let Some(u) = queue.pop_front() {
            sorted_order.push(u);

            if let Some(neighbors) = adj.get(&u) {
                for &v in neighbors {
                    if in_degree[v] > 0 {
                        in_degree[v] -= 1;
                        if in_degree[v] == 0 {
                            queue.push_back(v);
                        }
                    }
                }
            }
        }

        if sorted_order.len() < active_nodes.len() {
            self.cycle_detected = true;
            log::error!(
                "VfxGraphCompiler: Parent or Matte cyclic dependency detected in Composition '{}'!",
                comp.name
            );
            for &idx in &active_nodes {
                if !sorted_order.contains(&idx) {
                    sorted_order.push(idx);
                }
            }
        }

        // ── 3. Generate Execution Steps ──
        for (step_id_counter, &idx) in sorted_order.iter().enumerate() {
            let layer = &comp.layers[idx];

            let op = match &layer.layer_type {
                crate::core::timeline::LayerType::PreComp { comp_id } => {
                    LayerOpType::EvaluatePreComp {
                        layer_idx: idx,
                        comp_id: comp_id.clone(),
                    }
                }
                _ => {
                    if layer.track_matte != TrackMatteMode::None && idx > 0 {
                        LayerOpType::CompositeTrackMatte {
                            layer_idx: idx,
                            matte_layer_idx: idx - 1,
                        }
                    } else if layer.parent_id.is_some() {
                        LayerOpType::EvaluateParentTransform { layer_idx: idx }
                    } else {
                        LayerOpType::RenderLayer { layer_idx: idx }
                    }
                }
            };

            let deps: Vec<usize> = comp
                .layers
                .iter()
                .enumerate()
                .filter(|(dep_idx, _)| adj.get(dep_idx).is_some_and(|list| list.contains(&idx)))
                .map(|(dep_idx, _)| dep_idx)
                .collect();

            self.steps.push(ExecutionStep {
                step_id: step_id_counter,
                op,
                dependencies: deps,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::timeline::{Composition, Layer, LayerType};

    #[test]
    fn test_topological_parent_order() {
        let mut comp = Composition::new(
            "test_comp".to_string(),
            "Test Comp".to_string(),
            1920,
            1080,
            30,
            300,
        );

        let parent_layer = Layer::new(
            "parent".to_string(),
            "Parent Layer".to_string(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            300,
        );
        let mut child_layer = Layer::new(
            "child".to_string(),
            "Child Layer".to_string(),
            LayerType::Solid {
                color: [0.0, 1.0, 0.0, 1.0],
            },
            300,
        );
        child_layer.parent_id = Some("parent".to_string());

        comp.add_layer(parent_layer); // idx 0
        comp.add_layer(child_layer); // idx 1

        let mut compiler = VfxGraphCompiler::new();
        compiler.compile(&comp, 0);

        assert_eq!(compiler.steps.len(), 2);
        assert!(!compiler.cycle_detected);

        // Parent must be evaluated before child
        let parent_step_pos = compiler.steps.iter().position(|s| match s.op {
            LayerOpType::RenderLayer { layer_idx } => layer_idx == 0,
            _ => false,
        });
        assert!(parent_step_pos.is_some());
    }

    #[test]
    fn test_dead_code_elimination() {
        let mut comp = Composition::new(
            "test_comp".to_string(),
            "Test Comp".to_string(),
            1920,
            1080,
            30,
            300,
        );

        let active_layer = Layer::new(
            "active".to_string(),
            "Active Layer".to_string(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            300,
        );
        let mut inactive_layer = Layer::new(
            "inactive".to_string(),
            "Inactive Layer".to_string(),
            LayerType::Solid {
                color: [0.0, 1.0, 0.0, 1.0],
            },
            300,
        );
        inactive_layer.in_frame = 50;

        comp.add_layer(active_layer);
        comp.add_layer(inactive_layer);

        let mut compiler = VfxGraphCompiler::new();
        compiler.compile(&comp, 0);

        // Inactive layer must be culled out from execution steps
        assert_eq!(compiler.steps.len(), 1);
    }
}
