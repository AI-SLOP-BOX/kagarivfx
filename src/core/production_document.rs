//! Versionable cross-domain document joining the VFX project and audio clock.

use crate::core::audio_types::MixerChannel;
use crate::core::automation_binding::{AutomationBinding, ProductionClock};
use crate::core::timeline::{EffectType, LayerType, Project, ProjectItemType};
use crate::core::unified_time::TempoMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

static SAVE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDocumentSettings {
    pub sample_rate: u32,
    pub master_gain: f32,
    pub channels: Vec<MixerChannel>,
}

impl Default for AudioDocumentSettings {
    fn default() -> Self {
        Self {
            sample_rate: 48_000,
            master_gain: 1.0,
            channels: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionDocument {
    pub schema_version: u32,
    pub project: Project,
    #[serde(default)]
    pub audio: AudioDocumentSettings,
    #[serde(default)]
    pub tempo: TempoMap,
    #[serde(default)]
    pub bindings: Vec<AutomationBinding>,
}

impl ProductionDocument {
    pub const CURRENT_SCHEMA_VERSION: u32 = 1;
    pub const MAX_AUDIO_CHANNELS: usize = 4096;
    pub const MAX_BINDINGS: usize = 8192;
    pub const MAX_COMPOSITION_FRAMES: u32 = 10_000_000;
    pub const MAX_ASSET_DURATION_SEC: f32 = 10_000_000.0;

    pub fn new(project: Project) -> Self {
        Self {
            schema_version: Self::CURRENT_SCHEMA_VERSION,
            project,
            audio: AudioDocumentSettings::default(),
            tempo: TempoMap::new(120.0),
            bindings: Vec::new(),
        }
    }

    pub fn clock(&self) -> ProductionClock {
        ProductionClock {
            tempo: self.tempo.clone(),
            sample_rate: self.audio.sample_rate,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version > Self::CURRENT_SCHEMA_VERSION {
            return Err(format!(
                "production document schema {} is newer than supported {}",
                self.schema_version,
                Self::CURRENT_SCHEMA_VERSION
            ));
        }
        if self.project.compositions.is_empty() {
            return Err("production document must contain at least one composition".into());
        }
        if self.project.active_composition_idx >= self.project.compositions.len() {
            return Err("production document active composition index is out of range".into());
        }
        let mut composition_ids = HashSet::new();
        for composition in &self.project.compositions {
            validate_composition(composition, 0, &mut composition_ids)?;
        }
        for composition in &self.project.compositions {
            validate_precomp_references(composition, &composition_ids)?;
        }
        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();
        for composition in &self.project.compositions {
            validate_precomp_cycles(
                composition,
                &self.project.compositions,
                &mut visiting,
                &mut visited,
            )?;
        }
        let mut asset_ids = HashSet::new();
        let folder_ids = self
            .project
            .assets
            .iter()
            .filter_map(|asset| match &asset.item_type {
                ProjectItemType::Folder { .. } => Some(asset.id.as_str()),
                _ => None,
            })
            .collect::<HashSet<_>>();
        for asset in &self.project.assets {
            if asset.id.trim().is_empty()
                || asset.name.trim().is_empty()
                || !asset_ids.insert(asset.id.clone())
            {
                return Err("asset ids and names must be non-empty; ids must be unique".into());
            }
            if let Some(parent_folder) = &asset.parent_folder {
                if !folder_ids.contains(parent_folder.as_str()) {
                    return Err("asset parent folder reference is invalid".into());
                }
            }
            match &asset.item_type {
                ProjectItemType::Composition { comp_idx }
                    if *comp_idx >= self.project.compositions.len() =>
                {
                    return Err("asset composition index is out of range".into());
                }
                ProjectItemType::Image { width, height, .. }
                    if *width == 0 || *height == 0 || *width > 65_535 || *height > 65_535 =>
                {
                    return Err("asset image dimensions are out of range".into());
                }
                ProjectItemType::Image { path, .. }
                | ProjectItemType::Video { path, .. }
                | ProjectItemType::Audio { path, .. }
                    if path.trim().is_empty() =>
                {
                    return Err("media asset path must not be empty".into());
                }
                ProjectItemType::Solid { color }
                    if color.iter().any(|channel| !channel.is_finite()) =>
                {
                    return Err("solid asset color must be finite".into());
                }
                ProjectItemType::Video { duration_sec, .. }
                | ProjectItemType::Audio { duration_sec, .. }
                    if !duration_sec.is_finite()
                        || *duration_sec < 0.0
                        || *duration_sec > Self::MAX_ASSET_DURATION_SEC =>
                {
                    return Err("asset duration is out of range".into());
                }
                _ => {}
            }
        }
        let folder_parents = self
            .project
            .assets
            .iter()
            .filter_map(|asset| match &asset.item_type {
                ProjectItemType::Folder { .. } => {
                    Some((asset.id.as_str(), asset.parent_folder.as_deref()))
                }
                _ => None,
            })
            .collect::<HashMap<_, _>>();
        for folder_id in folder_parents.keys() {
            let mut current = Some(*folder_id);
            let mut visited = HashSet::new();
            while let Some(id) = current {
                if !visited.insert(id) {
                    return Err("asset folder hierarchy contains a cycle".into());
                }
                current = folder_parents.get(id).copied().flatten();
            }
        }
        if !(1..=384_000).contains(&self.audio.sample_rate) {
            return Err("audio sample rate is outside the supported range".into());
        }
        if !self.audio.master_gain.is_finite() || self.audio.master_gain < 0.0 {
            return Err("audio master gain must be finite and non-negative".into());
        }
        if self.audio.channels.len() > Self::MAX_AUDIO_CHANNELS {
            return Err("production document contains too many audio channels".into());
        }
        for channel in &self.audio.channels {
            channel.validate().map_err(str::to_owned)?;
        }
        self.tempo.validate().map_err(str::to_owned)?;
        if self.bindings.len() > Self::MAX_BINDINGS {
            return Err("production document contains too many automation bindings".into());
        }
        for binding in &self.bindings {
            binding.validate().map_err(str::to_owned)?;
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String, String> {
        self.validate()?;
        serde_json::to_string_pretty(self).map_err(|error| error.to_string())
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        let document: Self = serde_json::from_str(json).map_err(|error| error.to_string())?;
        document.validate()?;
        Ok(document)
    }

    pub fn save_atomic(&self, path: impl AsRef<std::path::Path>) -> Result<(), String> {
        let target = path.as_ref();
        let sequence = SAVE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = target.with_extension(format!(
            "production.{}.{}.tmp",
            std::process::id(),
            sequence
        ));
        let json = self.to_json()?;
        let result = (|| {
            let mut file = std::fs::File::create(&temporary)
                .map_err(|error| format!("failed to create production document: {error}"))?;
            use std::io::Write;
            file.write_all(json.as_bytes())
                .map_err(|error| format!("failed to write production document: {error}"))?;
            file.sync_all()
                .map_err(|error| format!("failed to sync production document: {error}"))?;
            drop(file);
            std::fs::rename(&temporary, target)
                .map_err(|error| format!("failed to replace production document: {error}"))?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result
    }

    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        let json = std::fs::read_to_string(path.as_ref())
            .map_err(|error| format!("failed to read production document: {error}"))?;
        Self::from_json(&json)
    }

    /// Upgrade a legacy Project JSON into the unified production document.
    /// Audio, tempo, and binding data receive safe defaults until the caller
    /// supplies domain-specific values.
    pub fn from_legacy_project_json(json: &str) -> Result<Self, String> {
        let project = crate::core::project_migration::load_project_migrated(json)?;
        let document = Self::new(project);
        document.validate()?;
        Ok(document)
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn project_mut(&mut self) -> &mut Project {
        &mut self.project
    }
}

fn validate_composition(
    composition: &crate::core::timeline::Composition,
    depth: usize,
    composition_ids: &mut HashSet<String>,
) -> Result<(), String> {
    if depth > 1024 {
        return Err("production document composition nesting is too deep".into());
    }
    if composition.id.trim().is_empty() {
        return Err("composition id must not be empty".into());
    }
    if composition.name.trim().is_empty() {
        return Err("composition name must not be empty".into());
    }
    if !composition_ids.insert(composition.id.clone()) {
        return Err(format!("duplicate composition id: {}", composition.id));
    }
    if !(1..=65_535).contains(&composition.width) || !(1..=65_535).contains(&composition.height) {
        return Err("composition dimensions are outside the supported range".into());
    }
    if !(1..=240).contains(&composition.fps)
        || !(1..=ProductionDocument::MAX_COMPOSITION_FRAMES).contains(&composition.duration_frames)
    {
        return Err("composition frame rate or duration is invalid".into());
    }
    if !composition.motion_blur_shutter_angle.is_finite()
        || !composition.motion_blur_shutter_phase.is_finite()
    {
        return Err("composition motion blur settings must be finite".into());
    }
    if composition
        .background_color
        .iter()
        .any(|channel| !channel.is_finite())
    {
        return Err("composition background color must be finite".into());
    }
    validate_camera(&composition.active_camera)?;
    let mut camera_names = HashSet::new();
    if !camera_names.insert(composition.active_camera.name.clone()) {
        return Err("camera names must be unique within a composition".into());
    }
    for camera in &composition.cameras {
        validate_camera(camera)?;
        if !camera_names.insert(camera.name.clone()) {
            return Err("camera names must be unique within a composition".into());
        }
    }
    let mut light_ids = HashSet::new();
    let mut light_names = HashSet::new();
    for light in &composition.lights {
        if light.id.trim().is_empty()
            || light.name.trim().is_empty()
            || !light_ids.insert(light.id.clone())
            || !light_names.insert(light.name.clone())
            || light.color.iter().any(|value| !value.is_finite())
            || !light.intensity.is_finite()
            || light.intensity < 0.0
            || !vector3_animation_is_finite(&light.position)
            || !light.shadow_darkness.is_finite()
            || !(0.0..=100.0).contains(&light.shadow_darkness)
            || !light.falloff.is_finite()
            || light.falloff < 0.0
            || !light.max_radius.is_finite()
            || light.max_radius < 0.0
            || !light_type_is_valid(&light.light_type)
        {
            return Err("light settings are invalid".into());
        }
    }
    for marker in &composition.markers {
        if marker.frame >= composition.duration_frames
            || marker.label.trim().is_empty()
            || marker.color.iter().any(|value| !value.is_finite())
        {
            return Err("timeline marker settings are invalid".into());
        }
    }
    let mut layer_ids = HashSet::new();
    let mut layer_parents = HashMap::new();
    for layer in &composition.layers {
        if layer.id.trim().is_empty()
            || layer.name.trim().is_empty()
            || !layer_ids.insert(layer.id.clone())
        {
            return Err("layer ids must be non-empty and unique within a composition".into());
        }
        if let Some(parent_id) = &layer.parent_id {
            if !layer_ids.contains(parent_id)
                && !composition
                    .layers
                    .iter()
                    .any(|candidate| candidate.id == *parent_id)
            {
                return Err("layer parent reference is invalid".into());
            }
            layer_parents.insert(layer.id.as_str(), parent_id.as_str());
        }
        if layer.in_frame >= layer.out_frame {
            return Err("layer frame range must have a positive duration".into());
        }
        if !vector_animation_is_finite(&layer.transform.anchor_point)
            || !vector_animation_is_finite(&layer.transform.position)
            || !vector_animation_is_finite(&layer.transform.scale)
            || !scalar_animation_is_finite(&layer.transform.rotation)
            || !scalar_animation_is_unit_interval(&layer.transform.opacity)
        {
            return Err("layer transform animation values must be finite".into());
        }
        if !vector3_animation_is_finite(&layer.transform_3d.position)
            || !vector3_animation_is_finite(&layer.transform_3d.rotation)
            || !vector3_animation_is_finite(&layer.transform_3d.scale)
        {
            return Err("layer 3D transform animation values must be finite".into());
        }
        if layer
            .time_remap
            .as_ref()
            .is_some_and(|value| !scalar_animation_is_nonnegative(value))
        {
            return Err("layer time remap values must be finite and non-negative".into());
        }
        if let LayerType::Video {
            frame_count, speed, ..
        } = &layer.layer_type
        {
            if *frame_count == 0 || !speed.is_finite() || *speed <= 0.0 || *speed > 1_000.0 {
                return Err("video layer frame count or speed is out of range".into());
            }
        }
        if let LayerType::Audio { path, volume } = &layer.layer_type {
            if path.trim().is_empty() || !scalar_animation_is_finite(volume) {
                return Err("audio layer path or volume is invalid".into());
            }
        }
        if let LayerType::Text {
            font_size,
            color,
            tracking,
            leading,
            align,
            font_family,
            stroke_color,
            stroke_width,
            ..
        } = &layer.layer_type
        {
            if *font_size == 0
                || *font_size > 16_384
                || color.iter().any(|value| !value.is_finite())
                || stroke_color.iter().any(|value| !value.is_finite())
                || !tracking.is_finite()
                || !leading.is_finite()
                || *leading <= 0.0
                || !stroke_width.is_finite()
                || *stroke_width < 0.0
                || *align > 2
                || font_family.trim().is_empty()
            {
                return Err("text layer settings are invalid".into());
            }
        }
        if let LayerType::Shape {
            shape_type,
            color,
            stroke_color,
            stroke_width,
            extrusion_depth,
            bevel_depth,
            fill_type,
        } = &layer.layer_type
        {
            if color.iter().any(|value| !value.is_finite())
                || stroke_color.iter().any(|value| !value.is_finite())
                || !stroke_width.is_finite()
                || *stroke_width < 0.0
                || !extrusion_depth.is_finite()
                || *extrusion_depth < 0.0
                || !bevel_depth.is_finite()
                || *bevel_depth < 0.0
            {
                return Err("shape layer settings are invalid".into());
            }
            if let crate::core::timeline::ShapeType::FreeformBezier {
                points, tangents, ..
            } = shape_type
            {
                if points
                    .iter()
                    .flatten()
                    .chain(
                        tangents.iter().flat_map(|(incoming, outgoing)| {
                            incoming.iter().chain(outgoing.iter())
                        }),
                    )
                    .any(|value| !value.is_finite())
                {
                    return Err("shape geometry coordinates must be finite".into());
                }
            }
            let shape_animation_values = match shape_type {
                crate::core::timeline::ShapeType::FreeformBezier { .. } => Vec::new(),
                crate::core::timeline::ShapeType::Rectangle {
                    width,
                    height,
                    corner_radius,
                } => vec![width, height, corner_radius],
                crate::core::timeline::ShapeType::Ellipse { width, height } => {
                    vec![width, height]
                }
                crate::core::timeline::ShapeType::Star {
                    points,
                    inner_radius,
                    outer_radius,
                } => vec![points, inner_radius, outer_radius],
                crate::core::timeline::ShapeType::Polygon { sides, radius } => {
                    vec![sides, radius]
                }
            };
            if shape_animation_values
                .iter()
                .any(|value| !scalar_animation_is_finite(value))
            {
                return Err("shape animation values must be finite".into());
            }
            let shape_nonnegative_values = match shape_type {
                crate::core::timeline::ShapeType::FreeformBezier { .. } => Vec::new(),
                crate::core::timeline::ShapeType::Rectangle {
                    width,
                    height,
                    corner_radius,
                } => vec![width, height, corner_radius],
                crate::core::timeline::ShapeType::Ellipse { width, height } => {
                    vec![width, height]
                }
                crate::core::timeline::ShapeType::Star {
                    points,
                    inner_radius,
                    outer_radius,
                } => vec![points, inner_radius, outer_radius],
                crate::core::timeline::ShapeType::Polygon { sides, radius } => {
                    vec![sides, radius]
                }
            };
            if shape_nonnegative_values
                .iter()
                .any(|value| !scalar_animation_is_nonnegative(value))
            {
                return Err("shape dimensions must be non-negative".into());
            }
            match shape_type {
                crate::core::timeline::ShapeType::Star { points, .. }
                | crate::core::timeline::ShapeType::Polygon { sides: points, .. }
                    if !scalar_animation_is_at_least(points, 3.0) =>
                {
                    return Err("polygon point counts must be at least three".into());
                }
                _ => {}
            }
            let fill_is_valid = match fill_type {
                crate::core::timeline::ShapeFillType::Solid => true,
                crate::core::timeline::ShapeFillType::LinearGradient {
                    start,
                    end,
                    colors,
                    stops,
                } => {
                    start
                        .iter()
                        .chain(end.iter())
                        .all(|value| value.is_finite())
                        && valid_gradient_stops(colors.as_slice(), stops.as_slice())
                }
                crate::core::timeline::ShapeFillType::RadialGradient {
                    center,
                    radius,
                    colors,
                    stops,
                } => {
                    center.iter().all(|value| value.is_finite())
                        && radius.is_finite()
                        && *radius > 0.0
                        && valid_gradient_stops(colors.as_slice(), stops.as_slice())
                }
            };
            if !fill_is_valid {
                return Err("shape gradient settings are invalid".into());
            }
        }
        if let LayerType::Particle { emitter } = &layer.layer_type {
            if !particle_emitter_is_valid(emitter) {
                return Err("particle emitter settings are invalid".into());
            }
        }
        let mut effect_ids = HashSet::new();
        for effect in &layer.effects {
            if effect.id.trim().is_empty()
                || effect.name.trim().is_empty()
                || !effect_ids.insert(effect.id.clone())
            {
                return Err("effect ids must be non-empty and unique within a layer".into());
            }
            let effect_debug = format!("{:?}", effect.effect_type);
            if !effect_debug_is_finite(&effect_debug) {
                return Err("effect parameters must be finite and serializable".into());
            }
        }
        let mut mask_ids = HashSet::new();
        for mask in &layer.masks {
            if mask.id.trim().is_empty()
                || mask.name.trim().is_empty()
                || !mask_ids.insert(mask.id.clone())
            {
                return Err("mask ids must be non-empty and unique within a layer".into());
            }
            validate_mask(mask)?;
        }
    }
    for layer_id in layer_parents.keys() {
        let mut current = Some(*layer_id);
        let mut visited = HashSet::new();
        while let Some(id) = current {
            if !visited.insert(id) {
                return Err("layer parent hierarchy contains a cycle".into());
            }
            current = layer_parents.get(id).copied();
        }
    }
    for nested in &composition.sub_compositions {
        validate_composition(nested, depth + 1, composition_ids)?;
    }
    Ok(())
}

fn validate_precomp_references(
    composition: &crate::core::timeline::Composition,
    composition_ids: &HashSet<String>,
) -> Result<(), String> {
    for layer in &composition.layers {
        if let LayerType::PreComp { comp_id } = &layer.layer_type {
            if !composition_ids.contains(comp_id) {
                return Err(format!("precomp references missing composition: {comp_id}"));
            }
        }
    }
    for nested in &composition.sub_compositions {
        validate_precomp_references(nested, composition_ids)?;
    }
    Ok(())
}

fn validate_camera(camera: &crate::core::timeline::Camera3D) -> Result<(), String> {
    if camera.name.trim().is_empty()
        || !camera.fov_degrees.is_finite()
        || !(0.0..180.0).contains(&camera.fov_degrees)
        || !camera.focus_distance.is_finite()
        || camera.focus_distance <= 0.0
        || !camera.aperture.is_finite()
        || camera.aperture < 0.0
        || !camera.dof_max_blur.is_finite()
        || camera.dof_max_blur < 0.0
        || (camera.dof_iris_sides != 0 && !(3..=32).contains(&camera.dof_iris_sides))
    {
        return Err("camera settings are invalid".into());
    }
    if !vector3_animation_is_finite(&camera.transform.position)
        || !vector3_animation_is_finite(&camera.transform.rotation)
        || !vector3_animation_is_finite(&camera.transform.scale)
    {
        return Err("camera transform animation values must be finite".into());
    }
    Ok(())
}

fn validate_mask(mask: &crate::core::mask::Mask) -> Result<(), String> {
    let validate_vertices =
        |vertices: &[[f32; 2]]| vertices.iter().flatten().all(|value| value.is_finite());
    match &mask.path.vertices {
        crate::core::property::Animatable::Constant(vertices) => {
            if !validate_vertices(vertices) {
                return Err("mask vertex coordinates must be finite".into());
            }
        }
        crate::core::property::Animatable::Animated(keyframes) => {
            if keyframes
                .iter()
                .any(|keyframe| !validate_vertices(&keyframe.value))
            {
                return Err("mask vertex coordinates must be finite".into());
            }
        }
    }
    if let Some(tangents) = &mask.path.tangents {
        if tangents
            .iter()
            .flat_map(|(incoming, outgoing)| incoming.iter().chain(outgoing.iter()))
            .any(|value| !value.is_finite())
        {
            return Err("mask tangent coordinates must be finite".into());
        }
    }
    for value in [&mask.feather, &mask.expansion] {
        if !scalar_animation_is_finite(value) {
            return Err("mask animation values must be finite".into());
        }
    }
    if !scalar_animation_is_unit_interval(&mask.opacity) {
        return Err("mask opacity must be within 0..1".into());
    }
    Ok(())
}

fn scalar_animation_is_finite(value: &crate::core::property::Animatable<f32>) -> bool {
    match value {
        crate::core::property::Animatable::Constant(value) => value.is_finite(),
        crate::core::property::Animatable::Animated(keyframes) => {
            keyframes.iter().all(|keyframe| keyframe.value.is_finite())
        }
    }
}

fn scalar_animation_is_nonnegative(value: &crate::core::property::Animatable<f32>) -> bool {
    match value {
        crate::core::property::Animatable::Constant(value) => value.is_finite() && *value >= 0.0,
        crate::core::property::Animatable::Animated(keyframes) => keyframes
            .iter()
            .all(|keyframe| keyframe.value.is_finite() && keyframe.value >= 0.0),
    }
}

fn scalar_animation_is_unit_interval(value: &crate::core::property::Animatable<f32>) -> bool {
    match value {
        crate::core::property::Animatable::Constant(value) => {
            value.is_finite() && (0.0..=1.0).contains(value)
        }
        crate::core::property::Animatable::Animated(keyframes) => keyframes
            .iter()
            .all(|keyframe| keyframe.value.is_finite() && (0.0..=1.0).contains(&keyframe.value)),
    }
}

fn scalar_animation_is_at_least(
    value: &crate::core::property::Animatable<f32>,
    minimum: f32,
) -> bool {
    match value {
        crate::core::property::Animatable::Constant(value) => {
            value.is_finite() && *value >= minimum
        }
        crate::core::property::Animatable::Animated(keyframes) => keyframes
            .iter()
            .all(|keyframe| keyframe.value.is_finite() && keyframe.value >= minimum),
    }
}

fn valid_gradient_stops(colors: &[[f32; 4]], stops: &[f32]) -> bool {
    colors.len() >= 2
        && colors.len() == stops.len()
        && colors
            .iter()
            .all(|color| color.iter().all(|value| value.is_finite()))
        && stops.windows(2).all(|pair| {
            pair[0].is_finite()
                && pair[1].is_finite()
                && (0.0..=1.0).contains(&pair[0])
                && pair[0] <= pair[1]
        })
        && stops
            .first()
            .is_some_and(|stop| stop.is_finite() && (0.0..=1.0).contains(stop))
        && stops
            .last()
            .is_some_and(|stop| stop.is_finite() && (0.0..=1.0).contains(stop))
}

fn effect_debug_is_finite(debug: &str) -> bool {
    !debug
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '-')
        .any(|token| matches!(token, "NaN" | "inf" | "-inf"))
}

fn particle_emitter_is_valid(emitter: &crate::core::particle_system::ParticleEmitter) -> bool {
    let finite = [
        emitter.rate,
        emitter.lifetime,
        emitter.lifetime_variance,
        emitter.speed,
        emitter.speed_variance,
        emitter.spread_degrees,
        emitter.emitter_size[0],
        emitter.emitter_size[1],
        emitter.gravity[0],
        emitter.gravity[1],
        emitter.wind[0],
        emitter.wind[1],
        emitter.turbulence,
        emitter.color_start[0],
        emitter.color_start[1],
        emitter.color_start[2],
        emitter.color_start[3],
        emitter.color_end[0],
        emitter.color_end[1],
        emitter.color_end[2],
        emitter.color_end[3],
        emitter.size_start,
        emitter.size_end,
        emitter.opacity_start,
        emitter.opacity_end,
        emitter.rotation_speed,
        emitter.rotation_start,
        emitter.rotation_speed_variance,
        emitter.wind_gust_strength,
        emitter.wind_gust_frequency,
        emitter.drag,
        emitter.restitution,
        emitter.surface_friction,
        emitter.particle_diameter,
        emitter.trail_taper,
        emitter.vortex_strength,
        emitter.vortex_center[0],
        emitter.vortex_center[1],
        emitter.attract_strength,
        emitter.attract_center[0],
        emitter.attract_center[1],
        emitter.depth_range[0],
        emitter.depth_range[1],
        emitter.death_spawn_speed_scale,
        emitter.death_spawn_life_scale,
    ];
    finite.iter().all(|value| value.is_finite())
        && emitter.rate >= 0.0
        && emitter.max_particles > 0
        && emitter.lifetime > 0.0
        && emitter.lifetime_variance >= 0.0
        && emitter.speed >= 0.0
        && emitter.speed_variance >= 0.0
        && (0.0..=360.0).contains(&emitter.spread_degrees)
        && emitter.emitter_size.iter().all(|value| *value >= 0.0)
        && (0.0..=1.0).contains(&emitter.color_start[3])
        && (0.0..=1.0).contains(&emitter.color_end[3])
        && emitter.size_start >= 0.0
        && emitter.size_end >= 0.0
        && emitter.opacity_start >= 0.0
        && emitter.opacity_start <= 1.0
        && emitter.opacity_end >= 0.0
        && emitter.opacity_end <= 1.0
        && emitter.drag >= 0.0
        && emitter.restitution >= 0.0
        && emitter.surface_friction >= 0.0
        && emitter.particle_diameter > 0.0
        && emitter.depth_range[0] <= emitter.depth_range[1]
        && emitter.death_spawn_speed_scale >= 0.0
        && emitter.death_spawn_life_scale >= 0.0
        && emitter.blend_mode <= 2
        && emitter
            .gravity_curve
            .0
            .iter()
            .all(|value| value.is_finite())
        && (!emitter.collision_enabled
            || (emitter
                .collision_bounds
                .iter()
                .all(|value| value.is_finite())
                && emitter.collision_bounds[0] <= emitter.collision_bounds[2]
                && emitter.collision_bounds[1] <= emitter.collision_bounds[3]))
}

fn light_type_is_valid(light_type: &crate::core::timeline::LightType) -> bool {
    match light_type {
        crate::core::timeline::LightType::Spot {
            cone_angle_deg,
            cone_feather_pct,
        } => {
            cone_angle_deg.is_finite()
                && (0.0..=180.0).contains(cone_angle_deg)
                && cone_feather_pct.is_finite()
                && (0.0..=100.0).contains(cone_feather_pct)
        }
        _ => true,
    }
}

fn vector_animation_is_finite(value: &crate::core::property::Animatable<[f32; 2]>) -> bool {
    match value {
        crate::core::property::Animatable::Constant(value) => {
            value.iter().all(|component| component.is_finite())
        }
        crate::core::property::Animatable::Animated(keyframes) => keyframes
            .iter()
            .all(|keyframe| keyframe.value.iter().all(|component| component.is_finite())),
    }
}

fn vector3_animation_is_finite(value: &crate::core::property::Animatable<[f32; 3]>) -> bool {
    match value {
        crate::core::property::Animatable::Constant(value) => {
            value.iter().all(|component| component.is_finite())
        }
        crate::core::property::Animatable::Animated(keyframes) => keyframes
            .iter()
            .all(|keyframe| keyframe.value.iter().all(|component| component.is_finite())),
    }
}

fn validate_precomp_cycles(
    composition: &crate::core::timeline::Composition,
    roots: &[crate::core::timeline::Composition],
    visiting: &mut HashSet<String>,
    visited: &mut HashSet<String>,
) -> Result<(), String> {
    if visited.contains(&composition.id) {
        return Ok(());
    }
    if !visiting.insert(composition.id.clone()) {
        return Err("composition precomp graph contains a cycle".into());
    }
    for layer in &composition.layers {
        if let LayerType::PreComp { comp_id } = &layer.layer_type {
            let target = roots
                .iter()
                .find_map(|root| find_nested_composition(root, comp_id));
            if let Some(target) = target {
                validate_precomp_cycles(target, roots, visiting, visited)?;
            }
        }
    }
    visiting.remove(&composition.id);
    visited.insert(composition.id.clone());
    Ok(())
}

fn find_nested_composition<'a>(
    composition: &'a crate::core::timeline::Composition,
    id: &str,
) -> Option<&'a crate::core::timeline::Composition> {
    if composition.id == id {
        return Some(composition);
    }
    composition
        .sub_compositions
        .iter()
        .find_map(|nested| find_nested_composition(nested, id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preserves_audio_vfx_contract() {
        let mut document = ProductionDocument::new(Project::default());
        document.audio.sample_rate = 44_100;
        document.bindings.push(AutomationBinding {
            source: "audio.bass".into(),
            target: "vfx.glow.intensity".into(),
            curve: crate::core::automation_binding::AutomationCurve {
                points: vec![crate::core::automation_binding::AutomationPoint {
                    time: crate::core::unified_time::Time::ZERO,
                    value: 0.5,
                }],
            },
            input_min: 0.0,
            input_max: 1.0,
            output_min: 0.0,
            output_max: 100.0,
        });

        let restored = ProductionDocument::from_json(&document.to_json().unwrap()).unwrap();
        assert_eq!(restored.audio.sample_rate, 44_100);
        assert_eq!(restored.bindings.len(), 1);
        assert_eq!(
            restored
                .clock()
                .sample_position(crate::core::unified_time::Time::new(1, 1)),
            44_100
        );
    }

    #[test]
    fn rejects_future_schema_and_invalid_audio() {
        let mut document = ProductionDocument::new(Project::default());
        document.schema_version += 1;
        assert!(document.validate().is_err());

        document.schema_version = ProductionDocument::CURRENT_SCHEMA_VERSION;
        document.audio.sample_rate = 0;
        assert!(document.validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.active_composition_idx = invalid_project.compositions.len();
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0].width = 0;
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0].duration_frames = 0;
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0].background_color[0] = f32::NAN;
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0].duration_frames =
            ProductionDocument::MAX_COMPOSITION_FRAMES + 1;
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0].id.clear();
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0].name = "  ".into();
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        let duplicate = invalid_project.compositions[0].clone();
        invalid_project.compositions.push(duplicate);
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project
            .assets
            .push(crate::core::timeline::ProjectItem::new(
                "huge-duration",
                "Huge Duration",
                ProjectItemType::Video {
                    path: "video.mp4".into(),
                    duration_sec: ProductionDocument::MAX_ASSET_DURATION_SEC + 1.0,
                },
            ));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0]
            .layers
            .push(crate::core::timeline::Layer::new(
                "invalid-video-speed".into(),
                "Invalid Video Speed".into(),
                LayerType::Video {
                    source: "video.mp4".into(),
                    frames_dir: "frames".into(),
                    frame_count: 1,
                    audio_wav: None,
                    speed: 1_001.0,
                },
                300,
            ));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project
            .assets
            .push(crate::core::timeline::ProjectItem::new(
                "bad-comp",
                "Bad Comp",
                ProjectItemType::Composition { comp_idx: 99 },
            ));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project
            .assets
            .push(crate::core::timeline::ProjectItem::new(
                "huge-image",
                "Huge Image",
                ProjectItemType::Image {
                    path: "image.png".into(),
                    width: 65_536,
                    height: 1080,
                },
            ));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0]
            .layers
            .push(crate::core::timeline::Layer::new(
                "invalid-shape-dimensions".into(),
                "Invalid Shape Dimensions".into(),
                LayerType::Shape {
                    shape_type: crate::core::timeline::ShapeType::Polygon {
                        sides: crate::core::property::Animatable::new_constant(2.0),
                        radius: crate::core::property::Animatable::new_constant(-1.0),
                    },
                    color: [1.0, 1.0, 1.0, 1.0],
                    stroke_color: [0.0, 0.0, 0.0, 1.0],
                    stroke_width: 0.0,
                    fill_type: Default::default(),
                    extrusion_depth: 0.0,
                    bevel_depth: 0.0,
                },
                300,
            ));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project
            .assets
            .push(crate::core::timeline::ProjectItem::new(
                "bad-image",
                "Bad Image",
                ProjectItemType::Image {
                    path: "image.png".into(),
                    width: 0,
                    height: 1080,
                },
            ));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project
            .assets
            .push(crate::core::timeline::ProjectItem::new(
                "empty-media",
                "Empty Media",
                ProjectItemType::Audio {
                    path: "  ".into(),
                    duration_sec: 1.0,
                },
            ));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project
            .assets
            .push(crate::core::timeline::ProjectItem::new(
                "bad-solid",
                "Bad Solid",
                ProjectItemType::Solid {
                    color: [f32::INFINITY, 0.0, 0.0, 1.0],
                },
            ));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project
            .assets
            .push(crate::core::timeline::ProjectItem::new(
                "item_comp1",
                "Duplicate",
                ProjectItemType::Folder { name: "x".into() },
            ));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        let mut asset = crate::core::timeline::ProjectItem::new(
            "asset",
            "Asset",
            ProjectItemType::Folder { name: "x".into() },
        );
        asset.parent_folder = Some("missing".into());
        invalid_project.assets.push(asset);
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        let mut folder_a = crate::core::timeline::ProjectItem::new(
            "folder-a",
            "A",
            ProjectItemType::Folder { name: "A".into() },
        );
        folder_a.parent_folder = Some("folder-b".into());
        let mut folder_b = crate::core::timeline::ProjectItem::new(
            "folder-b",
            "B",
            ProjectItemType::Folder { name: "B".into() },
        );
        folder_b.parent_folder = Some("folder-a".into());
        invalid_project.assets.extend([folder_a, folder_b]);
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0]
            .layers
            .push(crate::core::timeline::Layer::new(
                "missing-precomp".into(),
                "Missing Precomp".into(),
                LayerType::PreComp {
                    comp_id: "does-not-exist".into(),
                },
                300,
            ));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0]
            .layers
            .push(crate::core::timeline::Layer::new(
                "invalid-audio".into(),
                "Invalid Audio".into(),
                LayerType::Audio {
                    path: "".into(),
                    volume: crate::core::property::Animatable::new_constant(f32::NAN),
                },
                300,
            ));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0].layers[0].out_frame =
            invalid_project.compositions[0].layers[0].in_frame;
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        let duplicate = invalid_project.compositions[0].layers[0].clone();
        invalid_project.compositions[0].layers.push(duplicate);
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        let comp_id = invalid_project.compositions[0].id.clone();
        invalid_project.compositions[0]
            .layers
            .push(crate::core::timeline::Layer::new(
                "self-precomp".into(),
                "Self Precomp".into(),
                LayerType::PreComp { comp_id },
                300,
            ));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        let layer_id = invalid_project.compositions[0].layers[0].id.clone();
        invalid_project.compositions[0].layers[0].parent_id = Some(layer_id);
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        let effect = crate::core::timeline::Effect {
            id: "duplicate-effect".into(),
            name: "Blur".into(),
            effect_type: EffectType::GaussianBlur {
                blur_radius: crate::core::property::Animatable::new_constant(4.0),
            },
            enabled: true,
        };
        invalid_project.compositions[0].layers[0]
            .effects
            .extend([effect.clone(), effect]);
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0].layers[0]
            .effects
            .push(crate::core::timeline::Effect {
                id: "nonfinite-effect".into(),
                name: "Blur".into(),
                effect_type: EffectType::GaussianBlur {
                    blur_radius: crate::core::property::Animatable::new_constant(f32::NAN),
                },
                enabled: true,
            });
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        let mask = crate::core::mask::Mask::new_rect(
            "duplicate-mask".into(),
            "Mask".into(),
            0.0,
            0.0,
            100.0,
            100.0,
        );
        invalid_project.compositions[0].layers[0]
            .masks
            .extend([mask.clone(), mask]);
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0].layers[0].transform.position =
            crate::core::property::Animatable::new_constant([f32::NAN, 0.0]);
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0].layers[0].transform.opacity =
            crate::core::property::Animatable::new_constant(1.1);
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0].layers[0].time_remap =
            Some(crate::core::property::Animatable::new_constant(f32::NAN));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0].layers[0].time_remap =
            Some(crate::core::property::Animatable::new_constant(-1.0));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0].layers[0]
            .transform_3d
            .position = crate::core::property::Animatable::new_constant([0.0, f32::INFINITY, 0.0]);
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        if let LayerType::Text { font_size, .. } =
            &mut invalid_project.compositions[0].layers[1].layer_type
        {
            *font_size = 0;
        }
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        if let LayerType::Text { align, .. } =
            &mut invalid_project.compositions[0].layers[1].layer_type
        {
            *align = 3;
        }
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        if let LayerType::Text { font_size, .. } =
            &mut invalid_project.compositions[0].layers[1].layer_type
        {
            *font_size = 16_385;
        }
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        if let LayerType::Text { font_family, .. } =
            &mut invalid_project.compositions[0].layers[1].layer_type
        {
            *font_family = "   ".into();
        }
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0]
            .layers
            .push(crate::core::timeline::Layer::new(
                "invalid-gradient-stops".into(),
                "Invalid Gradient Stops".into(),
                LayerType::Shape {
                    shape_type: crate::core::timeline::ShapeType::Rectangle {
                        width: crate::core::property::Animatable::new_constant(100.0),
                        height: crate::core::property::Animatable::new_constant(100.0),
                        corner_radius: crate::core::property::Animatable::new_constant(0.0),
                    },
                    color: [1.0, 1.0, 1.0, 1.0],
                    stroke_color: [0.0, 0.0, 0.0, 1.0],
                    stroke_width: 0.0,
                    fill_type: crate::core::timeline::ShapeFillType::LinearGradient {
                        start: [0.0, 0.0],
                        end: [100.0, 100.0],
                        colors: vec![[0.0, 0.0, 0.0, 1.0], [1.0, f32::NAN, 1.0, 1.0]],
                        stops: vec![0.0],
                    },
                    extrusion_depth: 0.0,
                    bevel_depth: 0.0,
                },
                300,
            ));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0]
            .layers
            .push(crate::core::timeline::Layer::new(
                "invalid-shape-animation".into(),
                "Invalid Shape Animation".into(),
                LayerType::Shape {
                    shape_type: crate::core::timeline::ShapeType::Star {
                        points: crate::core::property::Animatable::new_constant(f32::NAN),
                        inner_radius: crate::core::property::Animatable::new_constant(10.0),
                        outer_radius: crate::core::property::Animatable::new_constant(20.0),
                    },
                    color: [1.0, 1.0, 1.0, 1.0],
                    stroke_color: [0.0, 0.0, 0.0, 1.0],
                    stroke_width: 0.0,
                    fill_type: Default::default(),
                    extrusion_depth: 0.0,
                    bevel_depth: 0.0,
                },
                300,
            ));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0]
            .layers
            .push(crate::core::timeline::Layer::new(
                "invalid-video".into(),
                "Invalid Video".into(),
                LayerType::Video {
                    source: "video.mp4".into(),
                    frames_dir: "frames".into(),
                    frame_count: 0,
                    audio_wav: None,
                    speed: 1.0,
                },
                300,
            ));
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        let duplicate_camera = invalid_project.compositions[0].active_camera.clone();
        invalid_project.compositions[0]
            .cameras
            .push(duplicate_camera);
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0].active_camera.fov_degrees = f32::NAN;
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        invalid_project.compositions[0]
            .lights
            .push(crate::core::timeline::Light3D {
                id: "bad-light".into(),
                name: "Bad Light".into(),
                intensity: -1.0,
                ..Default::default()
            });
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        let duration_frames = invalid_project.compositions[0].duration_frames;
        invalid_project.compositions[0]
            .markers
            .push(crate::core::timeline::TimelineMarker {
                frame: duration_frames,
                label: "  ".into(),
                color: [0.0, 0.0, 0.0],
            });
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        let mut invalid_mask = crate::core::mask::Mask::new_rect(
            "invalid-mask".into(),
            "Mask".into(),
            0.0,
            0.0,
            100.0,
            100.0,
        );
        if let crate::core::property::Animatable::Constant(vertices) =
            &mut invalid_mask.path.vertices
        {
            vertices[0][0] = f32::NAN;
        }
        invalid_project.compositions[0].layers[0]
            .masks
            .push(invalid_mask);
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        let mut invalid_mask = crate::core::mask::Mask::new_rect(
            "invalid-mask-animation".into(),
            "Mask".into(),
            0.0,
            0.0,
            100.0,
            100.0,
        );
        invalid_mask.opacity = crate::core::property::Animatable::new_constant(f32::NAN);
        invalid_project.compositions[0].layers[0]
            .masks
            .push(invalid_mask);
        assert!(ProductionDocument::new(invalid_project).validate().is_err());

        let mut invalid_project = Project::default();
        let mut invalid_mask = crate::core::mask::Mask::new_rect(
            "invalid-mask-opacity".into(),
            "Mask".into(),
            0.0,
            0.0,
            100.0,
            100.0,
        );
        invalid_mask.opacity = crate::core::property::Animatable::new_constant(-0.1);
        invalid_project.compositions[0].layers[0]
            .masks
            .push(invalid_mask);
        assert!(ProductionDocument::new(invalid_project).validate().is_err());
    }

    #[test]
    fn rejects_unbounded_audio_and_binding_collections() {
        let mut document = ProductionDocument::new(Project::default());
        document.audio.channels.resize(
            ProductionDocument::MAX_AUDIO_CHANNELS + 1,
            MixerChannel::default(),
        );
        assert!(document.validate().is_err());

        let mut document = ProductionDocument::new(Project::default());
        document.bindings.resize(
            ProductionDocument::MAX_BINDINGS + 1,
            AutomationBinding {
                source: "audio.x".into(),
                target: "vfx.y".into(),
                curve: crate::core::automation_binding::AutomationCurve {
                    points: vec![crate::core::automation_binding::AutomationPoint {
                        time: crate::core::unified_time::Time::ZERO,
                        value: 0.0,
                    }],
                },
                input_min: 0.0,
                input_max: 1.0,
                output_min: 0.0,
                output_max: 1.0,
            },
        );
        assert!(document.validate().is_err());
    }

    #[test]
    fn gradient_stops_require_matching_sorted_unit_interval_values() {
        let colors = [[0.0, 0.0, 0.0, 1.0], [1.0, 1.0, 1.0, 1.0]];
        assert!(valid_gradient_stops(&colors, &[0.0, 1.0]));
        assert!(!valid_gradient_stops(&colors, &[0.8, 0.2]));
        assert!(!valid_gradient_stops(&colors, &[-0.1, 1.0]));
        assert!(!valid_gradient_stops(&colors, &[0.0, 1.1]));
        assert!(!valid_gradient_stops(&colors, &[0.0]));
        assert!(!valid_gradient_stops(&[[0.0, 0.0, 0.0, 1.0]], &[0.0]));
    }

    #[test]
    fn effect_finite_check_does_not_match_field_names() {
        assert!(effect_debug_is_finite("Bezier { influence: 0.333 }"));
        assert!(!effect_debug_is_finite("GaussianBlur { radius: NaN }"));
        assert!(!effect_debug_is_finite("GaussianBlur { radius: -inf }"));
    }

    #[test]
    fn particle_emitter_validation_rejects_invalid_ranges_and_curves() {
        let mut emitter = crate::core::particle_system::ParticleEmitter::default();
        assert!(particle_emitter_is_valid(&emitter));
        emitter.spread_degrees = 361.0;
        assert!(!particle_emitter_is_valid(&emitter));
        emitter.spread_degrees = 0.0;
        emitter.gravity_curve.0[0] = f32::NAN;
        assert!(!particle_emitter_is_valid(&emitter));
        emitter.gravity_curve.0[0] = 1.0;
        emitter.collision_enabled = true;
        emitter.collision_bounds = [10.0, 10.0, 0.0, 0.0];
        assert!(!particle_emitter_is_valid(&emitter));
        emitter.collision_bounds = [0.0, 0.0, 10.0, 10.0];
        emitter.color_start[3] = 1.1;
        assert!(!particle_emitter_is_valid(&emitter));
    }

    #[test]
    fn spot_light_validation_bounds_cone_settings() {
        assert!(light_type_is_valid(
            &crate::core::timeline::LightType::Spot {
                cone_angle_deg: 45.0,
                cone_feather_pct: 50.0,
            }
        ));
        assert!(!light_type_is_valid(
            &crate::core::timeline::LightType::Spot {
                cone_angle_deg: 181.0,
                cone_feather_pct: 50.0,
            }
        ));
        assert!(!light_type_is_valid(
            &crate::core::timeline::LightType::Spot {
                cone_angle_deg: 45.0,
                cone_feather_pct: -1.0,
            }
        ));
    }

    #[test]
    fn atomic_save_and_load_preserve_contract() {
        let directory =
            std::env::temp_dir().join(format!("aevfx_production_document_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("session.aura");
        let document = ProductionDocument::new(Project::default());

        document.save_atomic(&path).unwrap();
        let loaded = ProductionDocument::load(&path).unwrap();
        assert_eq!(loaded.schema_version, document.schema_version);
        assert!(path.exists());
        assert!(!directory.join("session.production.tmp").exists());

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn failed_atomic_replace_removes_temporary_document() {
        let directory = std::env::temp_dir().join(format!(
            "aevfx_production_document_failure_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let target = directory.join("session.aura");
        std::fs::create_dir(&target).unwrap();

        let document = ProductionDocument::new(Project::default());
        assert!(document.save_atomic(&target).is_err());

        let temporary_files = std::fs::read_dir(&directory)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name())
            .filter(|name| name.to_string_lossy().starts_with("session.production."))
            .collect::<Vec<_>>();
        assert!(
            temporary_files.is_empty(),
            "temporary files: {temporary_files:?}"
        );

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn upgrades_legacy_project_json_with_safe_defaults() {
        let legacy =
            crate::core::project_migration::save_project_versioned(&Project::default()).unwrap();
        let document = ProductionDocument::from_legacy_project_json(&legacy).unwrap();

        assert_eq!(document.project().compositions.len(), 1);
        assert_eq!(document.audio.sample_rate, 48_000);
        assert!(document.bindings.is_empty());
    }

    #[test]
    fn legacy_migration_rejects_unusable_project() {
        let mut project = Project::default();
        project.active_composition_idx = project.compositions.len();
        let legacy = crate::core::project_migration::save_project_versioned(&project).unwrap();
        assert!(ProductionDocument::from_legacy_project_json(&legacy).is_err());
    }

    #[test]
    fn partial_production_documents_receive_domain_defaults() {
        let mut value: serde_json::Value = serde_json::from_str(
            &serde_json::to_string(&ProductionDocument::new(Project::default())).unwrap(),
        )
        .unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("audio");
        object.remove("tempo");
        object.remove("bindings");
        let document =
            ProductionDocument::from_json(&serde_json::to_string(&value).unwrap()).unwrap();
        assert_eq!(document.audio.sample_rate, 48_000);
        assert_eq!(
            document
                .tempo
                .beat_at(crate::core::unified_time::Time::new(1, 1)),
            2.0
        );
        assert!(document.bindings.is_empty());
    }
}
