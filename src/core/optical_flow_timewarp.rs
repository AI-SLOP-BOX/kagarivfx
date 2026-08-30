//! Pixel Motion Timewarp & Dense Optical Flow Interpolation Engine (AE Timewarp Parity).
//!
//! Computes bidirectional dense optical flow fields and performs forward/backward
//! motion-compensated pixel warping for artifact-free slow motion and retiming.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct DenseFlowField {
    pub width: u32,
    pub height: u32,
    pub vectors: Vec<[f32; 2]>, // [dx, dy] per pixel
}

impl DenseFlowField {
    pub fn new(width: u32, height: u32) -> Self {
        const MAX_FLOW_PIXELS: usize = 16_777_216;
        let size = (width as usize)
            .checked_mul(height as usize)
            .filter(|&count| count <= MAX_FLOW_PIXELS)
            .unwrap_or(0);
        Self {
            width,
            height,
            vectors: vec![[0.0, 0.0]; size],
        }
    }

    #[inline]
    pub fn get(&self, x: u32, y: u32) -> [f32; 2] {
        if x < self.width && y < self.height {
            self.vectors
                .get(y as usize * self.width as usize + x as usize)
                .copied()
                .unwrap_or([0.0, 0.0])
        } else {
            [0.0, 0.0]
        }
    }
}

/// Computes block-matching dense optical flow vectors from source to target frame.
pub fn compute_dense_optical_flow(
    src_rgba: &[u8],
    tgt_rgba: &[u8],
    width: u32,
    height: u32,
    block_radius: i32,
    search_radius: i32,
) -> DenseFlowField {
    let mut flow = DenseFlowField::new(width, height);
    let Some(size) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|v| v.checked_mul(4))
    else {
        return flow;
    };
    if width == 0 || height == 0 || src_rgba.len() != size || tgt_rgba.len() != size {
        return flow;
    }
    if flow.vectors.len() != (width as usize).saturating_mul(height as usize) {
        return flow;
    }
    let w = width as i32;
    let h = height as i32;
    let block_radius = block_radius.clamp(0, 64);
    let search_radius = search_radius.clamp(0, 128);

    let get_luma = |buf: &[u8], x: i32, y: i32| -> f32 {
        let cx = x.clamp(0, w - 1) as usize;
        let cy = y.clamp(0, h - 1) as usize;
        let idx = (cy * width as usize + cx) * 4;
        0.299 * buf[idx] as f32 + 0.587 * buf[idx + 1] as f32 + 0.114 * buf[idx + 2] as f32
    };

    // Subsampled dense matching for high performance
    let step = 4;
    for by in (0..h).step_by(step as usize) {
        for bx in (0..w).step_by(step as usize) {
            let mut best_sad = f32::INFINITY;
            let mut best_dx = 0.0f32;
            let mut best_dy = 0.0f32;

            for sdy in -search_radius..=search_radius {
                for sdx in -search_radius..=search_radius {
                    let mut sad = 0.0f32;
                    for py in -block_radius..=block_radius {
                        for px in -block_radius..=block_radius {
                            let src_val = get_luma(src_rgba, bx + px, by + py);
                            let tgt_val = get_luma(tgt_rgba, bx + px + sdx, by + py + sdy);
                            sad += (src_val - tgt_val).abs();
                        }
                    }

                    if sad < best_sad {
                        best_sad = sad;
                        best_dx = sdx as f32;
                        best_dy = sdy as f32;
                    }
                }
            }

            // Populate block cells
            for dy in 0..step {
                let y = by + dy;
                if y >= h {
                    continue;
                }
                for dx in 0..step {
                    let x = bx + dx;
                    if x >= w {
                        continue;
                    }
                    flow.vectors[(y as u32 * width + x as u32) as usize] = [best_dx, best_dy];
                }
            }
        }
    }

    flow
}

pub fn detect_markerless_features(
    frame_rgba: &[u8],
    width: u32,
    height: u32,
    max_features: usize,
    min_spacing: u32,
) -> Vec<[f32; 2]> {
    let Some(size) = (width as usize).checked_mul(height as usize).and_then(|v| v.checked_mul(4)) else { return Vec::new(); };
    if width < 3 || height < 3 || frame_rgba.len() != size || max_features == 0 { return Vec::new(); }
    let luma = |x: u32, y: u32| -> f32 {
        let i = (y as usize * width as usize + x as usize) * 4;
        0.299 * frame_rgba[i] as f32 + 0.587 * frame_rgba[i + 1] as f32 + 0.114 * frame_rgba[i + 2] as f32
    };
    let mut candidates = Vec::new();
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let gx = luma(x + 1, y) - luma(x - 1, y);
            let gy = luma(x, y + 1) - luma(x, y - 1);
            let score = gx * gx + gy * gy;
            if score.is_finite() && score > 1.0 { candidates.push((score, [x as f32, y as f32])); }
        }
    }
    candidates.sort_by(|a, b| b.0.total_cmp(&a.0));
    let spacing = min_spacing.max(1) as f32;
    let mut selected = Vec::with_capacity(max_features.min(candidates.len()));
    for (_, point) in candidates {
        if selected.iter().all(|other: & [f32; 2]| (point[0] - other[0]).hypot(point[1] - other[1]) >= spacing) {
            selected.push(point);
            if selected.len() == max_features { break; }
        }
    }
    selected
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MarkerlessMotionSample {
    pub frame: u32,
    pub position: [f32; 2],
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarkerlessMotionTrack {
    pub samples: Vec<MarkerlessMotionSample>,
}

pub fn track_markerless_motion(
    frames: &[&[u8]],
    width: u32,
    height: u32,
    seed_points: &[[f32; 2]],
    block_radius: i32,
    search_radius: i32,
) -> Vec<MarkerlessMotionTrack> {
    let Some(byte_len) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|v| v.checked_mul(4))
    else {
        return Vec::new();
    };
    if width == 0 || height == 0 || frames.is_empty() || frames.iter().any(|f| f.len() != byte_len)
    {
        return Vec::new();
    }

    let mut positions = Vec::with_capacity(seed_points.len());
    let mut tracks = Vec::with_capacity(seed_points.len());
    for &point in seed_points {
        if !point[0].is_finite() || !point[1].is_finite() {
            continue;
        }
        let position = [
            point[0].clamp(0.0, width.saturating_sub(1) as f32),
            point[1].clamp(0.0, height.saturating_sub(1) as f32),
        ];
        positions.push(position);
        tracks.push(MarkerlessMotionTrack {
            samples: vec![MarkerlessMotionSample {
                frame: 0,
                position,
                confidence: 1.0,
            }],
        });
    }
    if frames.len() == 1 || tracks.is_empty() {
        return tracks;
    }

    for frame in 1..frames.len() {
        let flow = compute_dense_optical_flow(
            frames[frame - 1],
            frames[frame],
            width,
            height,
            block_radius,
            search_radius,
        );
        let reverse_flow = compute_dense_optical_flow(
            frames[frame],
            frames[frame - 1],
            width,
            height,
            block_radius,
            search_radius,
        );
        for (index, position) in positions.iter_mut().enumerate() {
            let x = position[0]
                .round()
                .clamp(0.0, width.saturating_sub(1) as f32) as u32;
            let y = position[1]
                .round()
                .clamp(0.0, height.saturating_sub(1) as f32) as u32;
            let vector = flow.get(x, y);
            let magnitude = vector[0].hypot(vector[1]);
            let predicted = [
                (position[0] + vector[0]).clamp(0.0, width.saturating_sub(1) as f32),
                (position[1] + vector[1]).clamp(0.0, height.saturating_sub(1) as f32),
            ];
            let reverse = reverse_flow.get(predicted[0].round() as u32, predicted[1].round() as u32);
            let consistency = (vector[0] + reverse[0]).hypot(vector[1] + reverse[1]);
            let confidence = if magnitude.is_finite() && consistency.is_finite() {
                let motion_score = 1.0 - magnitude / (search_radius.max(1) as f32 + 1.0);
                let consistency_score = 1.0 - consistency / (search_radius.max(1) as f32 + 1.0);
                motion_score.min(consistency_score).clamp(0.0, 1.0)
            } else {
                0.0
            };
            *position = predicted;
            tracks[index].samples.push(MarkerlessMotionSample {
                frame: frame as u32,
                position: *position,
                confidence,
            });
        }
    }
    tracks
}

pub fn track_markerless_auto(
    frames: &[&[u8]],
    width: u32,
    height: u32,
    max_features: usize,
    feature_spacing: u32,
    block_radius: i32,
    search_radius: i32,
) -> Vec<MarkerlessMotionTrack> {
    let Some(first) = frames.first() else { return Vec::new(); };
    let seeds = detect_markerless_features(first, width, height, max_features, feature_spacing);
    track_markerless_motion(frames, width, height, &seeds, block_radius, search_radius)
}

pub fn markerless_track_keyframes(
    track: &MarkerlessMotionTrack,
) -> crate::core::property::Animatable<[f32; 2]> {
    use crate::core::keyframe::{InterpolationType, Keyframe};
    let keyframes = track
        .samples
        .iter()
        .filter(|sample| sample.position.iter().all(|value| value.is_finite()))
        .map(|sample| Keyframe::new(sample.frame, sample.position, InterpolationType::Linear))
        .collect::<Vec<_>>();
    match keyframes.as_slice() {
        [] => crate::core::property::Animatable::Constant([0.0, 0.0]),
        [keyframe] => crate::core::property::Animatable::Constant(keyframe.value),
        _ => crate::core::property::Animatable::Animated(keyframes),
    }
}

pub fn apply_markerless_track_to_tracker_point(
    tracker: &mut crate::core::timeline::TrackerPoint,
    track: &MarkerlessMotionTrack,
    minimum_confidence: f32,
) -> usize {
    use crate::core::keyframe::{InterpolationType, Keyframe};
    let threshold = if minimum_confidence.is_finite() {
        minimum_confidence.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let keyframes = track
        .samples
        .iter()
        .filter(|sample| {
            sample.confidence >= threshold
                && sample.confidence.is_finite()
                && sample.position.iter().all(|value| value.is_finite())
        })
        .map(|sample| Keyframe::new(sample.frame, sample.position, InterpolationType::Linear))
        .collect::<Vec<_>>();
    let count = keyframes.len();
    if count > 0 {
        tracker.position = crate::core::property::Animatable::Animated(keyframes);
    }
    count
}

pub fn smooth_markerless_track(
    track: &MarkerlessMotionTrack,
    radius: usize,
) -> MarkerlessMotionTrack {
    if track.samples.len() < 2 || radius == 0 {
        return track.clone();
    }
    let mut samples = Vec::with_capacity(track.samples.len());
    for (index, sample) in track.samples.iter().enumerate() {
        let start = index.saturating_sub(radius);
        let end = (index + radius + 1).min(track.samples.len());
        let mut weighted_position = [0.0f32; 2];
        let mut weight_sum = 0.0f32;
        let mut confidence_sum = 0.0f32;
        for neighbor in &track.samples[start..end] {
            let weight = neighbor.confidence.clamp(0.0, 1.0).max(0.01);
            if neighbor.position.iter().all(|value| value.is_finite()) {
                weighted_position[0] += neighbor.position[0] * weight;
                weighted_position[1] += neighbor.position[1] * weight;
                confidence_sum += neighbor.confidence.clamp(0.0, 1.0) * weight;
                weight_sum += weight;
            }
        }
        let position = if weight_sum > 0.0 {
            [weighted_position[0] / weight_sum, weighted_position[1] / weight_sum]
        } else {
            sample.position
        };
        samples.push(MarkerlessMotionSample {
            frame: sample.frame,
            position,
            confidence: if weight_sum > 0.0 {
                (confidence_sum / weight_sum).clamp(0.0, 1.0)
            } else {
                0.0
            },
        });
    }
    MarkerlessMotionTrack { samples }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarkerlessPoseFrame {
    pub frame: u32,
    pub joints: Vec<[f32; 2]>,
    pub root: [f32; 2],
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarkerlessPoseTrack {
    pub frames: Vec<MarkerlessPoseFrame>,
    pub bones: Vec<[usize; 2]>,
    pub bone_lengths: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedMarkerlessPoseTrack {
    pub joint_names: Vec<String>,
    pub bones: Vec<[usize; 2]>,
    pub frames: Vec<MarkerlessPoseFrame>,
    pub bone_lengths: Vec<f32>,
}

pub fn standard_humanoid_joint_names(count: usize) -> Vec<String> {
    const NAMES: &[&str] = &[
        "hip", "spine", "chest", "neck", "head", "left_shoulder", "left_elbow",
        "left_wrist", "right_shoulder", "right_elbow", "right_wrist", "left_hip",
        "left_knee", "left_ankle", "right_hip", "right_knee", "right_ankle",
    ];
    (0..count).map(|index| {
        NAMES.get(index).map(|name| (*name).to_string())
            .unwrap_or_else(|| format!("joint_{index:02}"))
    }).collect()
}

pub fn standard_humanoid_bones(joint_count: usize) -> Vec<[usize; 2]> {
    const BONES: &[(usize, usize)] = &[
        (0, 1), (1, 2), (2, 3), (3, 4),
        (2, 5), (5, 6), (6, 7),
        (2, 8), (8, 9), (9, 10),
        (0, 11), (11, 12), (12, 13),
        (0, 14), (14, 15), (15, 16),
    ];
    BONES.iter()
        .filter(|&&(parent, child)| parent < joint_count && child < joint_count)
        .map(|&(parent, child)| [parent, child])
        .collect()
}

pub fn normalize_joint_name(name: &str) -> String {
    name.trim().to_ascii_lowercase().replace([' ', '-'], "_")
}

pub fn name_markerless_pose_track(
    pose: &MarkerlessPoseTrack,
    joint_names: &[String],
) -> NamedMarkerlessPoseTrack {
    let names = joint_names.iter().take(
        pose.frames.first().map(|frame| frame.joints.len()).unwrap_or(0),
    ).map(|name| normalize_joint_name(name)).collect::<Vec<_>>();
    let joint_count = names.len();
    let bones = pose.bones.iter().copied()
        .filter(|bone| bone[0] < joint_count && bone[1] < joint_count)
        .collect();
    NamedMarkerlessPoseTrack {
        joint_names: names,
        bones,
        frames: pose.frames.clone(),
        bone_lengths: pose.bone_lengths.iter().copied().take(pose.bones.len()).collect(),
    }
}

pub fn name_and_connect_markerless_pose_track(
    pose: &MarkerlessPoseTrack,
) -> NamedMarkerlessPoseTrack {
    let count = pose.frames.first().map(|frame| frame.joints.len()).unwrap_or(0);
    let names = standard_humanoid_joint_names(count);
    let mut named = name_markerless_pose_track(pose, &names);
    named.bones = standard_humanoid_bones(named.joint_names.len());
    named.bone_lengths = named.bones.iter().map(|bone| {
        pose.bone_lengths.get(pose.bones.iter().position(|candidate| {
            candidate[0] == bone[0] && candidate[1] == bone[1]
        }).unwrap_or(usize::MAX)).copied().unwrap_or_else(|| {
            pose.frames.first().and_then(|frame| {
                Some((frame.joints[bone[0]][0] - frame.joints[bone[1]][0])
                    .hypot(frame.joints[bone[0]][1] - frame.joints[bone[1]][1]))
            }).unwrap_or(0.0)
        })
    }).collect();
    named
}

pub fn repair_markerless_pose_track(
    pose: &mut MarkerlessPoseTrack,
    max_gap: usize,
) -> usize {
    if pose.frames.is_empty() || max_gap == 0 { return 0; }
    let joint_count = pose.frames.iter().map(|frame| frame.joints.len()).min().unwrap_or(0);
    let mut repaired = 0;
    for joint in 0..joint_count {
        let mut index = 0;
        while index < pose.frames.len() {
            if pose.frames[index].joints[joint].iter().all(|value| value.is_finite()) {
                index += 1;
                continue;
            }
            let start = index;
            while index < pose.frames.len() && !pose.frames[index].joints[joint].iter().all(|value| value.is_finite()) {
                index += 1;
            }
            let end = index;
            if start == 0 || end >= pose.frames.len() || end - start > max_gap {
                continue;
            }
            let before = pose.frames[start - 1].joints[joint];
            let after = pose.frames[end].joints[joint];
            let span = (end - start + 1) as f32;
            for offset in 0..(end - start) {
                let t = (offset + 1) as f32 / span;
                pose.frames[start + offset].joints[joint] = [
                    before[0] + (after[0] - before[0]) * t,
                    before[1] + (after[1] - before[1]) * t,
                ];
                pose.frames[start + offset].confidence = (pose.frames[start + offset].confidence * 0.5).clamp(0.0, 1.0);
                repaired += 1;
            }
        }
    }
    repaired
}

pub fn markerless_pose_to_csv(pose: &NamedMarkerlessPoseTrack) -> String {
    let mut output = String::from("frame,root_x,root_y,confidence");
    for name in &pose.joint_names {
        output.push_str(&format!(",{}_x,{}_y", name, name));
    }
    output.push('\n');
    for frame in &pose.frames {
        output.push_str(&format!("{},{},{},{}", frame.frame, frame.root[0], frame.root[1], frame.confidence));
        for joint in &frame.joints {
            output.push_str(&format!(",{},{}", joint[0], joint[1]));
        }
        output.push('\n');
    }
    output
}

pub fn markerless_pose_to_bvh(pose: &NamedMarkerlessPoseTrack, frame_time: f32) -> String {
    if pose.joint_names.is_empty() || pose.frames.is_empty() {
        return String::new();
    }
    let frame_time = if frame_time.is_finite() && frame_time > 0.0 { frame_time } else { 1.0 / 30.0 };
    let mut children = vec![Vec::<usize>::new(); pose.joint_names.len()];
    for bone in &pose.bones {
        if bone[0] < children.len() && bone[1] < children.len() && bone[0] != bone[1] {
            children[bone[0]].push(bone[1]);
        }
    }
    let first = &pose.frames[0];
    let mut parent = vec![None; pose.joint_names.len()];
    for bone in &pose.bones {
        if bone[1] < parent.len() && parent[bone[1]].is_none() { parent[bone[1]] = Some(bone[0]); }
    }
    let root = parent.iter().position(Option::is_none).unwrap_or(0);
    let mut output = String::from("HIERARCHY\n");
    fn write_joint(out: &mut String, index: usize, parent: Option<usize>, indent: usize, names: &[String], children: &[Vec<usize>], frame: &MarkerlessPoseFrame, is_root: bool, visited: &mut Vec<bool>) {
        if index >= names.len() || visited[index] { return; }
        visited[index] = true;
        let pad = "  ".repeat(indent);
        out.push_str(&format!("{}{} {}\n{}{{\n", pad, if is_root { "ROOT" } else { "JOINT" }, names[index], pad));
        let point = frame.joints.get(index).copied().unwrap_or([0.0, 0.0]);
        let parent_point = parent.and_then(|p| frame.joints.get(p).copied()).unwrap_or([0.0, 0.0]);
        let (ox, oy) = if is_root { (0.0, 0.0) } else { (point[0] - parent_point[0], point[1] - parent_point[1]) };
        out.push_str(&format!("{}  OFFSET {:.6} {:.6} 0.000000\n", pad, ox, oy));
        if is_root { out.push_str(&format!("{}  CHANNELS 6 Xposition Yposition Zposition Zrotation Xrotation Yrotation\n", pad)); }
        else { out.push_str(&format!("{}  CHANNELS 3 Zrotation Xrotation Yrotation\n", pad)); }
        for &child in &children[index] { write_joint(out, child, Some(index), indent + 1, names, children, frame, false, visited); }
        if children[index].is_empty() { out.push_str(&format!("{}  End Site\n{}  {{\n{}    OFFSET 0.000000 0.000000 0.000000\n{}  }}\n", pad, pad, pad, pad)); }
        out.push_str(&format!("{}}}\n", pad));
    }
    let mut visited = vec![false; pose.joint_names.len()];
    write_joint(&mut output, root, None, 0, &pose.joint_names, &children, first, true, &mut visited);
    output.push_str(&format!("MOTION\nFrames: {}\nFrame Time: {:.9}\n", pose.frames.len(), frame_time));
    let initial = &first.joints;
    for (frame_index, frame) in pose.frames.iter().enumerate() {
        let root_point = frame.joints.get(root).copied().unwrap_or([0.0, 0.0]);
        output.push_str(&format!("{:.6} {:.6} 0.000000", root_point[0], root_point[1]));
        for index in 0..pose.joint_names.len() {
            if index == root { continue; }
            let angle = parent[index].and_then(|p| {
                let a = initial.get(index)?; let b = initial.get(p)?;
                let ia = frame.joints.get(index)?; let ib = frame.joints.get(p)?;
                Some(((ia[1] - ib[1]).atan2(ia[0] - ib[0]) - (a[1] - b[1]).atan2(a[0] - b[0])).to_degrees())
            }).unwrap_or(0.0);
            output.push_str(&format!(" {:.6} 0.000000 0.000000", angle));
        }
        if frame_index + 1 < pose.frames.len() { output.push('\n'); }
    }
    output
}

pub fn build_markerless_pose_track(
    joint_tracks: &[MarkerlessMotionTrack],
    bones: &[(usize, usize)],
) -> MarkerlessPoseTrack {
    if joint_tracks.is_empty() {
        return MarkerlessPoseTrack { frames: Vec::new(), bones: Vec::new(), bone_lengths: Vec::new() };
    }
    let frame_count = joint_tracks.iter().map(|track| track.samples.len()).min().unwrap_or(0);
    let mut frames = Vec::with_capacity(frame_count);
    for frame_index in 0..frame_count {
        let joints = joint_tracks.iter().map(|track| track.samples[frame_index].position).collect::<Vec<_>>();
        let root = joints.iter().fold([0.0, 0.0], |sum, point| [sum[0] + point[0], sum[1] + point[1]]);
        let root = [root[0] / joints.len() as f32, root[1] / joints.len() as f32];
        let confidence = joint_tracks.iter().map(|track| track.samples[frame_index].confidence).sum::<f32>()
            / joint_tracks.len() as f32;
        frames.push(MarkerlessPoseFrame {
            frame: joint_tracks[0].samples[frame_index].frame,
            joints,
            root,
            confidence: confidence.clamp(0.0, 1.0),
        });
    }
    let bone_lengths = bones.iter().map(|&(a, b)| {
        let Some(first) = frames.first() else { return 0.0; };
        match (first.joints.get(a), first.joints.get(b)) {
            (Some(a), Some(b)) if a.iter().all(|v| v.is_finite()) && b.iter().all(|v| v.is_finite()) =>
                (a[0] - b[0]).hypot(a[1] - b[1]),
            _ => 0.0,
        }
    }).collect();
    MarkerlessPoseTrack {
        frames,
        bones: bones.iter().map(|&(a, b)| [a, b]).collect(),
        bone_lengths,
    }
}

pub fn markerless_pose_to_json(
    pose: &MarkerlessPoseTrack,
) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(pose)
}

/// Interpolates an intermediate frame at fractional position `t` (0.0 .. 1.0)
/// using bidirectional forward and backward flow fields.
pub fn interpolate_timewarp_frame(
    frame_a: &[u8],
    frame_b: &[u8],
    width: u32,
    height: u32,
    flow_a_to_b: &DenseFlowField,
    flow_b_to_a: &DenseFlowField,
    t: f32,
    out_rgba: &mut [u8],
) {
    let Some(size) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|v| v.checked_mul(4))
    else {
        return;
    };
    if width == 0
        || height == 0
        || frame_a.len() != size
        || frame_b.len() != size
        || out_rgba.len() != size
    {
        return;
    }

    let t = if t.is_finite() {
        t.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let w = width as f32;
    let h = height as f32;

    let sample_bilinear = |buf: &[u8], sx: f32, sy: f32| -> [f32; 4] {
        let x = sx.clamp(0.0, (w - 1.0).max(0.0));
        let y = sy.clamp(0.0, (h - 1.0).max(0.0));

        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(width as usize - 1);
        let y1 = (y0 + 1).min(height as usize - 1);

        let fx = x - x0 as f32;
        let fy = y - y0 as f32;

        let idx00 = (y0 * width as usize + x0) * 4;
        let idx10 = (y0 * width as usize + x1) * 4;
        let idx01 = (y1 * width as usize + x0) * 4;
        let idx11 = (y1 * width as usize + x1) * 4;

        let mut res = [0.0f32; 4];
        for c in 0..4 {
            let v00 = buf[idx00 + c] as f32;
            let v10 = buf[idx10 + c] as f32;
            let v01 = buf[idx01 + c] as f32;
            let v11 = buf[idx11 + c] as f32;
            res[c] = (1.0 - fx) * (1.0 - fy) * v00
                + fx * (1.0 - fy) * v10
                + (1.0 - fx) * fy * v01
                + fx * fy * v11;
        }
        res
    };

    for y in 0..height {
        for x in 0..width {
            let px = x as f32;
            let py = y as f32;

            // Flow vectors
            let v_ab = flow_a_to_b.get(x, y);
            let v_ba = flow_b_to_a.get(x, y);

            // Backward-warp positions
            let src_a_x = px - v_ab[0] * t;
            let src_a_y = py - v_ab[1] * t;

            let src_b_x = px + v_ba[0] * (1.0 - t);
            let src_b_y = py + v_ba[1] * (1.0 - t);

            let col_a = sample_bilinear(frame_a, src_a_x, src_a_y);
            let col_b = sample_bilinear(frame_b, src_b_x, src_b_y);

            let dst_idx = (y as usize * width as usize + x as usize) * 4;
            for c in 0..4 {
                let blended = col_a[c] * (1.0 - t) + col_b[c] * t;
                out_rgba[dst_idx + c] = blended.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dense_flow_bounds_allocation_and_rejects_oversized_compute() {
        let field = DenseFlowField::new(u32::MAX, u32::MAX);
        assert!(field.vectors.is_empty());
        assert_eq!(field.get(0, 0), [0.0, 0.0]);
        let flow = compute_dense_optical_flow(&[0; 4], &[0; 4], u32::MAX, u32::MAX, 1, 1);
        assert!(flow.vectors.is_empty());
    }

    #[test]
    fn test_dense_optical_flow_translation() {
        let width = 16u32;
        let height = 16u32;
        let mut frame_a = vec![0u8; (width * height * 4) as usize];
        let mut frame_b = vec![0u8; (width * height * 4) as usize];

        // Draw a 4x4 white patch moving from (4, 4) to (6, 4)
        for y in 4..8 {
            for x in 4..8 {
                let idx = (y * width + x) as usize * 4;
                frame_a[idx] = 255;
                frame_a[idx + 1] = 255;
                frame_a[idx + 2] = 255;
                frame_a[idx + 3] = 255;
            }
        }

        for y in 4..8 {
            for x in 6..10 {
                let idx = (y * width + x) as usize * 4;
                frame_b[idx] = 255;
                frame_b[idx + 1] = 255;
                frame_b[idx + 2] = 255;
                frame_b[idx + 3] = 255;
            }
        }

        let flow = compute_dense_optical_flow(&frame_a, &frame_b, width, height, 1, 3);
        let center_vec = flow.get(5, 5);
        assert_eq!(center_vec[0], 2.0);
        assert_eq!(center_vec[1], 0.0);
    }

    #[test]
    fn markerless_tracking_follows_translation_and_bakes_keyframes() {
        let (w, h) = (16u32, 16u32);
        let make_frame = |offset: u32| {
            let mut frame = vec![0u8; (w * h * 4) as usize];
            for y in 4..8 {
                for x in (4 + offset)..(8 + offset) {
                    let i = (y * w + x) as usize * 4;
                    frame[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
                }
            }
            frame
        };
        let frames = [make_frame(0), make_frame(2), make_frame(4)];
        let refs = frames.iter().map(Vec::as_slice).collect::<Vec<_>>();
        let tracks = track_markerless_motion(&refs, w, h, &[[5.0, 5.0]], 1, 3);
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].samples.len(), 3);
        assert!(tracks[0].samples[1].position[0] >= 6.0);
        assert!(tracks[0]
            .samples
            .iter()
            .all(|s| (0.0..=1.0).contains(&s.confidence)));
        let keyframes = markerless_track_keyframes(&tracks[0]);
        assert_eq!(keyframes.keyframes().map(|k| k.len()), Some(3));
    }

    #[test]
    fn markerless_tracking_rejects_bad_frames_and_nonfinite_seeds() {
        let valid = vec![0u8; 4 * 4 * 4];
        let invalid = vec![0u8; 3];
        assert!(track_markerless_motion(
            &[valid.as_slice(), invalid.as_slice()],
            4,
            4,
            &[[1.0, 1.0]],
            1,
            1
        )
        .is_empty());
        assert!(track_markerless_motion(
            &[valid.as_slice()],
            4,
            4,
            &[[f32::NAN, 1.0], [f32::INFINITY, 2.0]],
            1,
            1
        )
        .is_empty());
    }

    #[test]
    fn markerless_smoothing_reduces_low_confidence_spike() {
        let track = MarkerlessMotionTrack {
            samples: vec![
                MarkerlessMotionSample { frame: 0, position: [0.0, 0.0], confidence: 1.0 },
                MarkerlessMotionSample { frame: 1, position: [10.0, 0.0], confidence: 0.1 },
                MarkerlessMotionSample { frame: 2, position: [2.0, 0.0], confidence: 1.0 },
            ],
        };
        let smoothed = smooth_markerless_track(&track, 1);
        assert!(smoothed.samples[1].position[0] < 6.0);
        assert_eq!(smoothed.samples[0].frame, 0);
        assert_eq!(smoothed.samples[2].frame, 2);
    }

    #[test]
    fn pose_track_uses_common_frames_and_computes_bones() {
        let make = |offset: f32| MarkerlessMotionTrack {
            samples: vec![
                MarkerlessMotionSample { frame: 0, position: [offset, 0.0], confidence: 1.0 },
                MarkerlessMotionSample { frame: 1, position: [offset + 1.0, 0.0], confidence: 0.8 },
            ],
        };
        let pose = build_markerless_pose_track(&[make(0.0), make(3.0)], &[(0, 1)]);
        assert_eq!(pose.frames.len(), 2);
        assert_eq!(pose.frames[0].root, [1.5, 0.0]);
        assert!((pose.bone_lengths[0] - 3.0).abs() < 0.001);
        assert!((pose.frames[1].confidence - 0.8).abs() < 0.001);
        let json = markerless_pose_to_json(&pose).unwrap();
        assert!(json.contains("bone_lengths"));
        let restored: MarkerlessPoseTrack = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, pose);
        let named = name_markerless_pose_track(&pose, &["hip".into(), "hand".into()]);
        let csv = markerless_pose_to_csv(&named);
        assert!(csv.lines().next().unwrap().contains("hip_x"));
        assert_eq!(csv.lines().count(), 3);
        assert_eq!(normalize_joint_name(" Left Shoulder-01 "), "left_shoulder_01");
        assert_eq!(standard_humanoid_joint_names(2), vec!["hip", "spine"]);
        assert_eq!(standard_humanoid_bones(3), vec![[0, 1], [1, 2]]);
        let auto_named = name_and_connect_markerless_pose_track(&pose);
        assert_eq!(auto_named.joint_names, vec!["hip", "spine"]);
        let bvh = markerless_pose_to_bvh(&named, 0.0);
        assert!(bvh.contains("HIERARCHY"));
        assert!(bvh.contains("Frames: 2"));
        assert!(bvh.contains("Frame Time:"));
    }

    #[test]
    fn pose_repair_interpolates_short_occlusion_and_lowers_confidence() {
        let mut pose = MarkerlessPoseTrack {
            frames: vec![
                MarkerlessPoseFrame { frame: 0, joints: vec![[0.0, 0.0]], root: [0.0, 0.0], confidence: 1.0 },
                MarkerlessPoseFrame { frame: 1, joints: vec![[f32::NAN, f32::NAN]], root: [0.0, 0.0], confidence: 1.0 },
                MarkerlessPoseFrame { frame: 2, joints: vec![[2.0, 0.0]], root: [2.0, 0.0], confidence: 1.0 },
            ],
            bones: Vec::new(),
            bone_lengths: Vec::new(),
        };
        assert_eq!(repair_markerless_pose_track(&mut pose, 1), 1);
        assert_eq!(pose.frames[1].joints[0], [1.0, 0.0]);
        assert!(pose.frames[1].confidence < 1.0);
    }

    #[test]
    fn feature_detection_is_bounded_spaced_and_deterministic() {
        let (w, h) = (12u32, 12u32);
        let mut frame = vec![0u8; (w * h * 4) as usize];
        for &(x, y) in &[(2, 2), (8, 3), (4, 9)] {
            let i = (y * w + x) as usize * 4;
            frame[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
        }
        let first = detect_markerless_features(&frame, w, h, 4, 2);
        let second = detect_markerless_features(&frame, w, h, 4, 2);
        assert_eq!(first, second);
        assert!(first.len() <= 4);
        assert!(first.iter().all(|p| p[0] >= 1.0 && p[0] < 11.0 && p[1] >= 1.0 && p[1] < 11.0));
        let refs = [&frame[..]];
        assert_eq!(track_markerless_auto(&refs, w, h, 2, 2, 1, 1).len(), first.len().min(2));
    }
}
