//! Editing-assist algorithms used by automatic timeline cleanup commands.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelBounds {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeRange {
    pub start_seconds: f64,
    pub end_seconds: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReframePoint {
    pub frame: usize,
    pub center: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LetterboxInsets {
    pub top: u32,
    pub bottom: u32,
    pub left: u32,
    pub right: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LoopCandidate {
    pub start_frame: usize,
    pub end_frame: usize,
    pub error: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExposureReport {
    pub shadow_clipped_fraction: f32,
    pub highlight_clipped_fraction: f32,
}

/// Finds the smallest rectangle containing pixels whose alpha exceeds `threshold`.
pub fn transparent_content_bounds(
    rgba: &[u8],
    width: u32,
    height: u32,
    threshold: u8,
) -> Option<PixelBounds> {
    let expected = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    if width == 0 || height == 0 || rgba.len() != expected {
        return None;
    }

    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;
    for y in 0..height {
        for x in 0..width {
            let alpha = rgba[((y as usize * width as usize + x as usize) * 4) + 3];
            if alpha > threshold {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                found = true;
            }
        }
    }
    found.then_some(PixelBounds {
        x: min_x,
        y: min_y,
        width: max_x - min_x + 1,
        height: max_y - min_y + 1,
    })
}

/// Detects hard cuts by comparing compact RGB histograms of consecutive frames.
pub fn detect_scene_cuts(
    frames: &[&[u8]],
    width: u32,
    height: u32,
    sensitivity: f32,
) -> Vec<usize> {
    let Some(frame_len) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|v| v.checked_mul(4))
    else {
        return Vec::new();
    };
    if frame_len == 0 || frames.len() < 2 || frames.iter().any(|f| f.len() != frame_len) {
        return Vec::new();
    }
    let threshold = if sensitivity.is_finite() {
        sensitivity.clamp(0.0, 1.0)
    } else {
        0.35
    };
    let histograms: Vec<_> = frames.iter().map(|frame| rgb_histogram(frame)).collect();
    histograms
        .windows(2)
        .enumerate()
        .filter_map(|(index, pair)| {
            let distance = pair[0]
                .iter()
                .zip(&pair[1])
                .map(|(a, b)| a.abs_diff(*b) as f32)
                .sum::<f32>()
                / (2.0 * width as f32 * height as f32 * 3.0);
            (distance >= threshold).then_some(index + 1)
        })
        .collect()
}

fn rgb_histogram(frame: &[u8]) -> [u32; 48] {
    let mut histogram = [0; 48];
    for pixel in frame.chunks_exact(4) {
        histogram[(pixel[0] >> 4) as usize] += 1;
        histogram[16 + (pixel[1] >> 4) as usize] += 1;
        histogram[32 + (pixel[2] >> 4) as usize] += 1;
    }
    histogram
}

/// Finds runs of visually unchanged frames using mean absolute RGB error.
pub fn detect_frozen_frames(
    frames: &[&[u8]],
    width: u32,
    height: u32,
    tolerance: f32,
    minimum_frames: usize,
) -> Vec<std::ops::RangeInclusive<usize>> {
    let Some(frame_len) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|v| v.checked_mul(4))
    else {
        return Vec::new();
    };
    if frame_len == 0
        || frames.len() < 2
        || minimum_frames < 2
        || frames.iter().any(|frame| frame.len() != frame_len)
    {
        return Vec::new();
    }
    let tolerance = if tolerance.is_finite() {
        tolerance.clamp(0.0, 255.0)
    } else {
        0.0
    };
    let mut ranges = Vec::new();
    let mut run_start = 0;
    for index in 1..frames.len() {
        let error = frames[index - 1]
            .chunks_exact(4)
            .zip(frames[index].chunks_exact(4))
            .map(|(a, b)| {
                (a[0].abs_diff(b[0]) as f32
                    + a[1].abs_diff(b[1]) as f32
                    + a[2].abs_diff(b[2]) as f32)
                    / 3.0
            })
            .sum::<f32>()
            / (width as f32 * height as f32);
        if error > tolerance {
            if index - run_start >= minimum_frames {
                ranges.push(run_start..=index - 1);
            }
            run_start = index;
        }
    }
    if frames.len() - run_start >= minimum_frames {
        ranges.push(run_start..=frames.len() - 1);
    }
    ranges
}

/// Finds nearly black frames while allowing a small number of bright pixels.
pub fn detect_black_frames(
    frames: &[&[u8]],
    width: u32,
    height: u32,
    luminance_threshold: u8,
    required_dark_fraction: f32,
) -> Vec<usize> {
    let Some(frame_len) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|value| value.checked_mul(4))
    else {
        return Vec::new();
    };
    if frame_len == 0 || frames.iter().any(|frame| frame.len() != frame_len) {
        return Vec::new();
    }
    let required = if required_dark_fraction.is_finite() {
        required_dark_fraction.clamp(0.0, 1.0)
    } else {
        0.99
    };
    frames
        .iter()
        .enumerate()
        .filter_map(|(index, frame)| {
            let dark = frame
                .chunks_exact(4)
                .filter(|pixel| {
                    let luminance =
                        (pixel[0] as u32 * 54 + pixel[1] as u32 * 183 + pixel[2] as u32 * 19) / 256;
                    luminance <= luminance_threshold as u32
                })
                .count();
            (dark as f32 / (width as f32 * height as f32) >= required).then_some(index)
        })
        .collect()
}

/// Detects uniformly dark bars touching each edge of a frame.
pub fn detect_letterbox_insets(
    rgba: &[u8],
    width: u32,
    height: u32,
    luminance_threshold: u8,
    required_dark_fraction: f32,
) -> Option<LetterboxInsets> {
    let expected = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    if width == 0 || height == 0 || rgba.len() != expected {
        return None;
    }
    let required = if required_dark_fraction.is_finite() {
        required_dark_fraction.clamp(0.5, 1.0)
    } else {
        0.98
    };
    let row_dark = |y: u32| {
        (0..width)
            .filter(|x| pixel_is_dark(rgba, width, *x, y, luminance_threshold))
            .count() as f32
            / width as f32
            >= required
    };
    let column_dark = |x: u32| {
        (0..height)
            .filter(|y| pixel_is_dark(rgba, width, x, *y, luminance_threshold))
            .count() as f32
            / height as f32
            >= required
    };
    let top = (0..height).take_while(|y| row_dark(*y)).count() as u32;
    let bottom = (0..height).rev().take_while(|y| row_dark(*y)).count() as u32;
    let left = (0..width).take_while(|x| column_dark(*x)).count() as u32;
    let right = (0..width).rev().take_while(|x| column_dark(*x)).count() as u32;
    if top + bottom >= height || left + right >= width {
        return None;
    }
    Some(LetterboxInsets {
        top,
        bottom,
        left,
        right,
    })
}

fn pixel_is_dark(rgba: &[u8], width: u32, x: u32, y: u32, threshold: u8) -> bool {
    let index = (y as usize * width as usize + x as usize) * 4;
    let luminance =
        (rgba[index] as u32 * 54 + rgba[index + 1] as u32 * 183 + rgba[index + 2] as u32 * 19)
            / 256;
    luminance <= threshold as u32
}

/// Reports the fraction of pixels clipped near black and white.
pub fn analyze_exposure_clipping(
    rgba: &[u8],
    width: u32,
    height: u32,
    shadow_threshold: u8,
    highlight_threshold: u8,
) -> Option<ExposureReport> {
    let expected = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    if expected == 0 || rgba.len() != expected || shadow_threshold >= highlight_threshold {
        return None;
    }
    let mut shadows = 0usize;
    let mut highlights = 0usize;
    for pixel in rgba.chunks_exact(4) {
        let minimum = pixel[0].min(pixel[1]).min(pixel[2]);
        let maximum = pixel[0].max(pixel[1]).max(pixel[2]);
        shadows += usize::from(maximum <= shadow_threshold);
        highlights += usize::from(minimum >= highlight_threshold);
    }
    let pixels = width as f32 * height as f32;
    Some(ExposureReport {
        shadow_clipped_fraction: shadows as f32 / pixels,
        highlight_clipped_fraction: highlights as f32 / pixels,
    })
}

/// Finds abrupt one-frame luminance spikes or drops that may indicate flashes.
pub fn detect_flash_frames(
    frames: &[&[u8]],
    width: u32,
    height: u32,
    minimum_luminance_jump: f32,
) -> Vec<usize> {
    let Some(frame_len) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|value| value.checked_mul(4))
    else {
        return Vec::new();
    };
    if frame_len == 0 || frames.len() < 3 || frames.iter().any(|frame| frame.len() != frame_len) {
        return Vec::new();
    }
    let jump = if minimum_luminance_jump.is_finite() {
        minimum_luminance_jump.clamp(0.0, 255.0)
    } else {
        80.0
    };
    let averages: Vec<f32> = frames
        .iter()
        .map(|frame| {
            frame
                .chunks_exact(4)
                .map(|pixel| {
                    (pixel[0] as f32 * 0.2126 + pixel[1] as f32 * 0.7152 + pixel[2] as f32 * 0.0722)
                })
                .sum::<f32>()
                / (width as f32 * height as f32)
        })
        .collect();
    averages
        .windows(3)
        .enumerate()
        .filter_map(|(index, values)| {
            let left = values[1] - values[0];
            let right = values[2] - values[1];
            (left.abs() >= jump && right.abs() >= jump && left.signum() != right.signum())
                .then_some(index + 1)
        })
        .collect()
}

/// Returns frame indices to retain after removing consecutive visual duplicates.
pub fn deduplicate_frame_indices(
    frames: &[&[u8]],
    width: u32,
    height: u32,
    tolerance: f32,
) -> Vec<usize> {
    let Some(frame_len) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|value| value.checked_mul(4))
    else {
        return Vec::new();
    };
    if frame_len == 0 || frames.iter().any(|frame| frame.len() != frame_len) {
        return Vec::new();
    }
    if frames.is_empty() {
        return Vec::new();
    }
    let tolerance = if tolerance.is_finite() {
        tolerance.clamp(0.0, 255.0)
    } else {
        0.0
    };
    let mut retained = vec![0];
    let mut reference = 0;
    for index in 1..frames.len() {
        let error = frames[reference]
            .chunks_exact(4)
            .zip(frames[index].chunks_exact(4))
            .map(|(a, b)| {
                (a[0].abs_diff(b[0]) as f32
                    + a[1].abs_diff(b[1]) as f32
                    + a[2].abs_diff(b[2]) as f32)
                    / 3.0
            })
            .sum::<f32>()
            / (width as f32 * height as f32);
        if error > tolerance {
            retained.push(index);
            reference = index;
        }
    }
    retained
}

/// Chooses a subtitle/overlay rectangle with the least overlap with occupied regions.
pub fn find_safe_overlay_position(
    frame_size: [u32; 2],
    overlay_size: [u32; 2],
    occupied: &[PixelBounds],
    margin: u32,
) -> Option<PixelBounds> {
    if frame_size.contains(&0)
        || overlay_size.contains(&0)
        || overlay_size[0].saturating_add(margin.saturating_mul(2)) > frame_size[0]
        || overlay_size[1].saturating_add(margin.saturating_mul(2)) > frame_size[1]
    {
        return None;
    }
    let center_x = (frame_size[0] - overlay_size[0]) / 2;
    let center_y = (frame_size[1] - overlay_size[1]) / 2;
    let candidates = [
        [center_x, frame_size[1] - margin - overlay_size[1]],
        [center_x, margin],
        [margin, center_y],
        [frame_size[0] - margin - overlay_size[0], center_y],
        [center_x, center_y],
    ];
    candidates
        .into_iter()
        .map(|position| PixelBounds {
            x: position[0],
            y: position[1],
            width: overlay_size[0],
            height: overlay_size[1],
        })
        .min_by_key(|candidate| {
            occupied
                .iter()
                .map(|region| intersection_area(*candidate, *region))
                .sum::<u64>()
        })
}

fn intersection_area(a: PixelBounds, b: PixelBounds) -> u64 {
    let left = a.x.max(b.x);
    let top = a.y.max(b.y);
    let right = a.x.saturating_add(a.width).min(b.x.saturating_add(b.width));
    let bottom =
        a.y.saturating_add(a.height)
            .min(b.y.saturating_add(b.height));
    right.saturating_sub(left) as u64 * bottom.saturating_sub(top) as u64
}

/// Estimates image sharpness using variance of the luminance Laplacian.
pub fn sharpness_score(rgba: &[u8], width: u32, height: u32) -> Option<f32> {
    let expected = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    if width < 3 || height < 3 || rgba.len() != expected {
        return None;
    }
    let luminance = |x: u32, y: u32| {
        let index = (y as usize * width as usize + x as usize) * 4;
        rgba[index] as f32 * 0.2126
            + rgba[index + 1] as f32 * 0.7152
            + rgba[index + 2] as f32 * 0.0722
    };
    let mut values = Vec::with_capacity(((width - 2) * (height - 2)) as usize);
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            values.push(
                luminance(x - 1, y)
                    + luminance(x + 1, y)
                    + luminance(x, y - 1)
                    + luminance(x, y + 1)
                    - 4.0 * luminance(x, y),
            );
        }
    }
    let mean = values.iter().sum::<f32>() / values.len() as f32;
    Some(
        values
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f32>()
            / values.len() as f32,
    )
}

/// Measures high-frequency camera shake from tracked frame-to-frame positions.
pub fn camera_shake_score(tracked_centers: &[[f32; 2]]) -> f32 {
    if tracked_centers.len() < 3
        || tracked_centers
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
    {
        return 0.0;
    }
    let acceleration_energy = tracked_centers
        .windows(3)
        .map(|points| {
            let acceleration = [
                points[2][0] - 2.0 * points[1][0] + points[0][0],
                points[2][1] - 2.0 * points[1][1] + points[0][1],
            ];
            acceleration[0] * acceleration[0] + acceleration[1] * acceleration[1]
        })
        .sum::<f32>();
    (acceleration_energy / (tracked_centers.len() - 2) as f32).sqrt()
}

/// Builds an aspect-ratio crop centered around the combined important regions.
pub fn smart_aspect_crop(
    frame_size: [u32; 2],
    target_aspect: f32,
    important_regions: &[PixelBounds],
) -> Option<PixelBounds> {
    if frame_size.contains(&0) || !target_aspect.is_finite() || target_aspect <= 0.0 {
        return None;
    }
    let source_aspect = frame_size[0] as f32 / frame_size[1] as f32;
    let (crop_width, crop_height) = if target_aspect >= source_aspect {
        (
            frame_size[0],
            (frame_size[0] as f32 / target_aspect).round() as u32,
        )
    } else {
        (
            (frame_size[1] as f32 * target_aspect).round() as u32,
            frame_size[1],
        )
    };
    let crop_width = crop_width.clamp(1, frame_size[0]);
    let crop_height = crop_height.clamp(1, frame_size[1]);
    let frame_bounds = PixelBounds {
        x: 0,
        y: 0,
        width: frame_size[0],
        height: frame_size[1],
    };
    let valid: Vec<_> = important_regions
        .iter()
        .filter_map(|region| intersect_bounds(*region, frame_bounds))
        .collect();
    let center = if valid.is_empty() {
        [frame_size[0] as f64 * 0.5, frame_size[1] as f64 * 0.5]
    } else {
        let total_weight = valid
            .iter()
            .map(|region| region.width as f64 * region.height as f64)
            .sum::<f64>();
        let weighted = valid.iter().fold([0.0, 0.0], |mut sum, region| {
            let weight = region.width as f64 * region.height as f64;
            sum[0] += (region.x as f64 + region.width as f64 * 0.5) * weight;
            sum[1] += (region.y as f64 + region.height as f64 * 0.5) * weight;
            sum
        });
        [weighted[0] / total_weight, weighted[1] / total_weight]
    };
    let x = (center[0] - crop_width as f64 * 0.5)
        .round()
        .clamp(0.0, (frame_size[0] - crop_width) as f64) as u32;
    let y = (center[1] - crop_height as f64 * 0.5)
        .round()
        .clamp(0.0, (frame_size[1] - crop_height) as f64) as u32;
    Some(PixelBounds {
        x,
        y,
        width: crop_width,
        height: crop_height,
    })
}

fn intersect_bounds(a: PixelBounds, b: PixelBounds) -> Option<PixelBounds> {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    let right = a.x.saturating_add(a.width).min(b.x.saturating_add(b.width));
    let bottom =
        a.y.saturating_add(a.height)
            .min(b.y.saturating_add(b.height));
    if right <= x || bottom <= y {
        return None;
    }
    Some(PixelBounds {
        x,
        y,
        width: right - x,
        height: bottom - y,
    })
}

/// Suggests visually seamless frame pairs, ordered by lowest RGB error.
pub fn find_loop_candidates(
    frames: &[&[u8]],
    width: u32,
    height: u32,
    minimum_length: usize,
    maximum_results: usize,
) -> Vec<LoopCandidate> {
    let Some(frame_len) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|value| value.checked_mul(4))
    else {
        return Vec::new();
    };
    if frame_len == 0
        || minimum_length == 0
        || maximum_results == 0
        || frames.iter().any(|frame| frame.len() != frame_len)
    {
        return Vec::new();
    }
    let pixels = width as f32 * height as f32;
    let mut candidates = Vec::new();
    for start in 0..frames.len() {
        for end in start.saturating_add(minimum_length)..frames.len() {
            let error = frames[start]
                .chunks_exact(4)
                .zip(frames[end].chunks_exact(4))
                .map(|(a, b)| {
                    (a[0].abs_diff(b[0]) as f32
                        + a[1].abs_diff(b[1]) as f32
                        + a[2].abs_diff(b[2]) as f32)
                        / 3.0
                })
                .sum::<f32>()
                / pixels;
            let candidate = LoopCandidate {
                start_frame: start,
                end_frame: end,
                error,
            };
            if candidates.len() < maximum_results {
                candidates.push(candidate);
            } else {
                let worst = candidates
                    .iter()
                    .enumerate()
                    .max_by(|(_, left), (_, right)| compare_loop_candidates(left, right));
                if let Some((worst_index, worst)) = worst {
                    if compare_loop_candidates(&candidate, worst).is_lt() {
                        candidates[worst_index] = candidate;
                    }
                }
            }
        }
    }
    candidates.sort_by(compare_loop_candidates);
    candidates
}

fn compare_loop_candidates(a: &LoopCandidate, b: &LoopCandidate) -> std::cmp::Ordering {
    a.error
        .total_cmp(&b.error)
        .then_with(|| b.end_frame.cmp(&a.end_frame))
        .then_with(|| a.start_frame.cmp(&b.start_frame))
}

/// Returns silent regions using windowed RMS, merging adjacent silent windows.
pub fn detect_silence(
    samples: &[f32],
    sample_rate: u32,
    threshold_db: f32,
    minimum_seconds: f64,
) -> Vec<TimeRange> {
    if samples.is_empty()
        || sample_rate == 0
        || !threshold_db.is_finite()
        || !minimum_seconds.is_finite()
        || minimum_seconds < 0.0
    {
        return Vec::new();
    }
    let window = (sample_rate as usize / 100).max(1);
    let amplitude_threshold = 10.0f32.powf(threshold_db.clamp(-120.0, 0.0) / 20.0);
    let mut ranges = Vec::new();
    let mut silent_start = None;
    for (window_index, chunk) in samples.chunks(window).enumerate() {
        let rms = (chunk
            .iter()
            .map(|sample| {
                if sample.is_finite() {
                    sample * sample
                } else {
                    1.0
                }
            })
            .sum::<f32>()
            / chunk.len() as f32)
            .sqrt();
        let start = window_index * window;
        if rms <= amplitude_threshold {
            silent_start.get_or_insert(start);
        } else if let Some(range_start) = silent_start.take() {
            push_silence_range(
                &mut ranges,
                range_start,
                start,
                sample_rate,
                minimum_seconds,
            );
        }
    }
    if let Some(range_start) = silent_start {
        push_silence_range(
            &mut ranges,
            range_start,
            samples.len(),
            sample_rate,
            minimum_seconds,
        );
    }
    ranges
}

fn push_silence_range(
    ranges: &mut Vec<TimeRange>,
    start: usize,
    end: usize,
    sample_rate: u32,
    minimum_seconds: f64,
) {
    let start_seconds = start as f64 / sample_rate as f64;
    let end_seconds = end as f64 / sample_rate as f64;
    if end_seconds - start_seconds >= minimum_seconds {
        ranges.push(TimeRange {
            start_seconds,
            end_seconds,
        });
    }
}

/// Returns transient/beat candidates in seconds using positive short-time energy changes.
pub fn detect_audio_beats(
    samples: &[f32],
    sample_rate: u32,
    sensitivity: f32,
    minimum_interval_seconds: f64,
) -> Vec<f64> {
    if samples.is_empty()
        || sample_rate == 0
        || !minimum_interval_seconds.is_finite()
        || minimum_interval_seconds < 0.0
    {
        return Vec::new();
    }
    let window = (sample_rate as usize / 100).max(1);
    let energies: Vec<f64> = samples
        .chunks(window)
        .map(|chunk| {
            chunk
                .iter()
                .map(|sample| {
                    if sample.is_finite() {
                        f64::from(*sample) * f64::from(*sample)
                    } else {
                        0.0
                    }
                })
                .sum::<f64>()
                / chunk.len() as f64
        })
        .collect();
    if energies.len() < 3 {
        return Vec::new();
    }
    let onsets: Vec<f64> = energies
        .windows(2)
        .map(|pair| (pair[1] - pair[0]).max(0.0))
        .collect();
    let mean = onsets.iter().sum::<f64>() / onsets.len() as f64;
    let variance = onsets
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / onsets.len() as f64;
    let sensitivity = if sensitivity.is_finite() {
        f64::from(sensitivity.clamp(0.0, 4.0))
    } else {
        1.5f64
    };
    let threshold = mean + variance.sqrt() * sensitivity;
    let minimum_samples = (minimum_interval_seconds * sample_rate as f64) as usize;
    let mut last_sample = None;
    let mut beats = Vec::new();
    for index in 1..onsets.len().saturating_sub(1) {
        let onset = onsets[index];
        if onset > threshold && onset >= onsets[index - 1] && onset >= onsets[index + 1] {
            let sample = (index + 1) * window;
            if last_sample.is_none_or(|last| sample.saturating_sub(last) >= minimum_samples) {
                beats.push(sample as f64 / sample_rate as f64);
                last_sample = Some(sample);
            }
        }
    }
    beats
}

/// Computes a safe gain toward a target RMS level without exceeding a peak ceiling.
pub fn normalization_gain_db(samples: &[f32], target_rms_db: f32, peak_ceiling_db: f32) -> f32 {
    if samples.is_empty() || !target_rms_db.is_finite() || !peak_ceiling_db.is_finite() {
        return 0.0;
    }
    let finite: Vec<f32> = samples
        .iter()
        .copied()
        .filter(|sample| sample.is_finite())
        .collect();
    if finite.is_empty() {
        return 0.0;
    }
    let sum_squares = finite
        .iter()
        .map(|sample| f64::from(*sample) * f64::from(*sample))
        .sum::<f64>();
    let rms = (sum_squares / finite.len() as f64).sqrt();
    let peak = finite
        .iter()
        .map(|sample| f64::from(sample.abs()))
        .fold(0.0, f64::max);
    if !rms.is_finite() || !peak.is_finite() || rms <= f64::EPSILON || peak <= f64::EPSILON {
        return 0.0;
    }
    let rms_db = 20.0 * rms.log10();
    let peak_db = 20.0 * peak.log10();
    ((f64::from(target_rms_db) - rms_db)
        .min(f64::from(peak_ceiling_db) - peak_db)
        .clamp(-60.0, 60.0)) as f32
}

/// Returns ranges where audio remains at or above a clipping threshold.
pub fn detect_audio_clipping(
    samples: &[f32],
    sample_rate: u32,
    threshold: f32,
    minimum_samples: usize,
) -> Vec<TimeRange> {
    if samples.is_empty() || sample_rate == 0 || !threshold.is_finite() || minimum_samples == 0 {
        return Vec::new();
    }
    let threshold = threshold.abs().clamp(0.0, 1.0);
    let mut ranges = Vec::new();
    let mut start = None;
    for (index, sample) in samples.iter().enumerate() {
        let clipped = sample.is_finite() && sample.abs() >= threshold;
        if clipped {
            start.get_or_insert(index);
        } else if let Some(range_start) = start.take() {
            if index - range_start >= minimum_samples {
                ranges.push(TimeRange {
                    start_seconds: range_start as f64 / sample_rate as f64,
                    end_seconds: index as f64 / sample_rate as f64,
                });
            }
        }
    }
    if let Some(range_start) = start {
        if samples.len() - range_start >= minimum_samples {
            ranges.push(TimeRange {
                start_seconds: range_start as f64 / sample_rate as f64,
                end_seconds: samples.len() as f64 / sample_rate as f64,
            });
        }
    }
    ranges
}

/// Produces a smoothed, crop-safe center path from tracked subject positions.
pub fn build_auto_reframe_path(
    tracked_centers: &[[f32; 2]],
    source_size: [u32; 2],
    output_size: [u32; 2],
    smoothing: f32,
) -> Vec<ReframePoint> {
    if source_size.contains(&0)
        || output_size.contains(&0)
        || output_size[0] > source_size[0]
        || output_size[1] > source_size[1]
        || tracked_centers
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
    {
        return Vec::new();
    }
    let smoothing = if smoothing.is_finite() {
        smoothing.clamp(0.0, 0.99)
    } else {
        0.8
    };
    let half = [output_size[0] as f32 * 0.5, output_size[1] as f32 * 0.5];
    let max = [
        source_size[0] as f32 - half[0],
        source_size[1] as f32 - half[1],
    ];
    let mut previous = None;
    tracked_centers
        .iter()
        .enumerate()
        .map(|(frame, center)| {
            let target = [
                center[0].clamp(half[0], max[0]),
                center[1].clamp(half[1], max[1]),
            ];
            let smoothed = previous.map_or(target, |old: [f32; 2]| {
                [
                    old[0] * smoothing + target[0] * (1.0 - smoothing),
                    old[1] * smoothing + target[1] * (1.0 - smoothing),
                ]
            });
            previous = Some(smoothed);
            ReframePoint {
                frame,
                center: smoothed,
            }
        })
        .collect()
}

/// Reduces 2D motion samples while keeping endpoints and the requested error bound.
pub fn reduce_motion_path(points: &[[f32; 2]], tolerance: f32) -> Vec<usize> {
    if points.len() <= 2 {
        return (0..points.len()).collect();
    }
    if points.iter().flatten().any(|v| !v.is_finite()) {
        return (0..points.len()).collect();
    }
    let tolerance = if tolerance.is_finite() {
        tolerance.max(0.0)
    } else {
        0.0
    };
    let mut keep = vec![false; points.len()];
    keep[0] = true;
    keep[points.len() - 1] = true;
    simplify_range(points, 0, points.len() - 1, tolerance, &mut keep);
    keep.into_iter()
        .enumerate()
        .filter_map(|(index, retain)| retain.then_some(index))
        .collect()
}

fn simplify_range(
    points: &[[f32; 2]],
    start: usize,
    end: usize,
    tolerance: f32,
    keep: &mut [bool],
) {
    let mut farthest = 0.0;
    let mut farthest_index = None;
    for index in start + 1..end {
        let distance = point_segment_distance(points[index], points[start], points[end]);
        if distance > farthest {
            farthest = distance;
            farthest_index = Some(index);
        }
    }
    if farthest > tolerance {
        let index = farthest_index.expect("positive distance has an index");
        keep[index] = true;
        simplify_range(points, start, index, tolerance, keep);
        simplify_range(points, index, end, tolerance, keep);
    }
}

fn point_segment_distance(point: [f32; 2], start: [f32; 2], end: [f32; 2]) -> f32 {
    let delta = [end[0] - start[0], end[1] - start[1]];
    let length_squared = delta[0] * delta[0] + delta[1] * delta[1];
    if length_squared <= f32::EPSILON {
        return ((point[0] - start[0]).powi(2) + (point[1] - start[1]).powi(2)).sqrt();
    }
    let t = (((point[0] - start[0]) * delta[0] + (point[1] - start[1]) * delta[1])
        / length_squared)
        .clamp(0.0, 1.0);
    let projected = [start[0] + t * delta[0], start[1] + t * delta[1]];
    ((point[0] - projected[0]).powi(2) + (point[1] - projected[1]).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transparent_bounds_find_single_pixel() {
        let mut pixels = vec![0; 4 * 3 * 4];
        pixels[(2 * 4 + 1) * 4 + 3] = 255;
        assert_eq!(
            transparent_content_bounds(&pixels, 4, 3, 0),
            Some(PixelBounds {
                x: 1,
                y: 2,
                width: 1,
                height: 1
            })
        );
    }

    #[test]
    fn transparent_bounds_reject_bad_buffer() {
        assert_eq!(transparent_content_bounds(&[0; 3], 1, 1, 0), None);
    }

    #[test]
    fn scene_cuts_detect_black_to_white() {
        let black = vec![0; 4 * 4 * 4];
        let white = vec![255; 4 * 4 * 4];
        assert_eq!(
            detect_scene_cuts(&[&black, &black, &white], 4, 4, 0.5),
            vec![2]
        );
    }

    #[test]
    fn scene_cuts_ignore_identical_frames() {
        let frame = vec![42; 4 * 4 * 4];
        assert!(detect_scene_cuts(&[&frame, &frame], 4, 4, 0.1).is_empty());
    }

    #[test]
    fn frozen_frames_find_multiple_runs() {
        let a = vec![10; 2 * 2 * 4];
        let b = vec![200; 2 * 2 * 4];
        assert_eq!(
            detect_frozen_frames(&[&a, &a, &a, &b, &b], 2, 2, 0.0, 2),
            vec![0..=2, 3..=4]
        );
    }

    #[test]
    fn frozen_frames_reject_mismatched_frame() {
        assert!(detect_frozen_frames(&[&[0; 16], &[0; 3]], 2, 2, 0.0, 2).is_empty());
    }

    #[test]
    fn black_frame_detection_allows_small_bright_area() {
        let black = vec![0; 10 * 10 * 4];
        let mut almost_black = black.clone();
        almost_black[0..4].fill(255);
        let bright = vec![255; 10 * 10 * 4];
        assert_eq!(
            detect_black_frames(&[&black, &almost_black, &bright], 10, 10, 8, 0.98),
            vec![0, 1]
        );
    }

    #[test]
    fn letterbox_detects_top_and_bottom_bars() {
        let mut frame = vec![255; 6 * 6 * 4];
        for y in [0, 5] {
            for x in 0..6 {
                frame[(y * 6 + x) * 4..(y * 6 + x + 1) * 4].fill(0);
            }
        }
        assert_eq!(
            detect_letterbox_insets(&frame, 6, 6, 4, 1.0),
            Some(LetterboxInsets {
                top: 1,
                bottom: 1,
                left: 0,
                right: 0
            })
        );
    }

    #[test]
    fn letterbox_rejects_fully_black_frame() {
        assert_eq!(detect_letterbox_insets(&[0; 4 * 4 * 4], 4, 4, 8, 1.0), None);
    }

    #[test]
    fn loop_candidates_prefer_matching_frames() {
        let a = vec![10; 4 * 4 * 4];
        let b = vec![100; 4 * 4 * 4];
        let candidates = find_loop_candidates(&[&a, &b, &a], 4, 4, 2, 1);
        assert_eq!(candidates[0].start_frame, 0);
        assert_eq!(candidates[0].end_frame, 2);
        assert_eq!(candidates[0].error, 0.0);
    }

    #[test]
    fn exposure_report_counts_only_neutral_clipping() {
        let frame = [0, 0, 0, 255, 255, 255, 255, 255, 255, 0, 0, 255];
        let report = analyze_exposure_clipping(&frame, 3, 1, 4, 250).unwrap();
        assert!((report.shadow_clipped_fraction - 1.0 / 3.0).abs() < 0.001);
        assert!((report.highlight_clipped_fraction - 1.0 / 3.0).abs() < 0.001);
    }

    #[test]
    fn flash_detection_finds_single_frame_spike() {
        let dark = vec![10; 2 * 2 * 4];
        let bright = vec![240; 2 * 2 * 4];
        assert_eq!(
            detect_flash_frames(&[&dark, &bright, &dark], 2, 2, 100.0),
            vec![1]
        );
    }

    #[test]
    fn flash_detection_ignores_gradual_change() {
        let a = vec![10; 2 * 2 * 4];
        let b = vec![100; 2 * 2 * 4];
        let c = vec![200; 2 * 2 * 4];
        assert!(detect_flash_frames(&[&a, &b, &c], 2, 2, 50.0).is_empty());
    }

    #[test]
    fn deduplication_keeps_only_visual_changes() {
        let a = vec![10; 2 * 2 * 4];
        let b = vec![20; 2 * 2 * 4];
        assert_eq!(
            deduplicate_frame_indices(&[&a, &a, &b, &b, &a], 2, 2, 0.0),
            vec![0, 2, 4]
        );
    }

    #[test]
    fn safe_overlay_avoids_occupied_bottom() {
        let occupied = [PixelBounds {
            x: 0,
            y: 60,
            width: 100,
            height: 40,
        }];
        let result = find_safe_overlay_position([100, 100], [40, 20], &occupied, 10).unwrap();
        assert_eq!(
            result,
            PixelBounds {
                x: 30,
                y: 10,
                width: 40,
                height: 20
            }
        );
    }

    #[test]
    fn safe_overlay_rejects_oversized_content() {
        assert!(find_safe_overlay_position([100, 100], [100, 20], &[], 1).is_none());
    }

    #[test]
    fn sharpness_score_distinguishes_edge_from_flat_frame() {
        let flat = vec![100; 5 * 5 * 4];
        let mut edge = flat.clone();
        for y in 0..5 {
            for x in 2..5 {
                edge[(y * 5 + x) * 4..(y * 5 + x) * 4 + 3].fill(255);
            }
        }
        assert!(sharpness_score(&edge, 5, 5).unwrap() > sharpness_score(&flat, 5, 5).unwrap());
    }

    #[test]
    fn shake_score_ignores_constant_velocity() {
        assert_eq!(
            camera_shake_score(&[[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]]),
            0.0
        );
        assert!(camera_shake_score(&[[0.0, 0.0], [2.0, 0.0], [0.0, 0.0]]) > 0.0);
    }

    #[test]
    fn smart_crop_keeps_target_aspect_and_focus() {
        let focus = [PixelBounds {
            x: 80,
            y: 20,
            width: 20,
            height: 20,
        }];
        let crop = smart_aspect_crop([100, 100], 0.5, &focus).unwrap();
        assert_eq!([crop.width, crop.height], [50, 100]);
        assert_eq!(crop.x, 50);
    }

    #[test]
    fn smart_crop_defaults_to_center() {
        assert_eq!(
            smart_aspect_crop([100, 50], 1.0, &[]),
            Some(PixelBounds {
                x: 25,
                y: 0,
                width: 50,
                height: 50
            })
        );
    }

    #[test]
    fn smart_crop_ignores_regions_outside_frame() {
        let crop = smart_aspect_crop(
            [100, 100],
            1.0,
            &[PixelBounds {
                x: 10_000,
                y: 10_000,
                width: 20,
                height: 20,
            }],
        )
        .unwrap();
        assert_eq!(
            crop,
            PixelBounds {
                x: 0,
                y: 0,
                width: 100,
                height: 100
            }
        );
    }

    #[test]
    fn silence_detection_merges_windows() {
        let mut samples = vec![0.0; 100];
        samples.extend(vec![1.0; 100]);
        let ranges = detect_silence(&samples, 100, -40.0, 0.5);
        assert_eq!(
            ranges,
            vec![TimeRange {
                start_seconds: 0.0,
                end_seconds: 1.0
            }]
        );
    }

    #[test]
    fn nonfinite_audio_is_not_silence() {
        assert!(detect_silence(&[f32::NAN; 100], 100, -40.0, 0.1).is_empty());
    }

    #[test]
    fn beat_detection_finds_energy_transients() {
        let mut samples = vec![0.0; 1_000];
        for start in [200, 600] {
            samples[start..start + 10].fill(1.0);
        }
        let beats = detect_audio_beats(&samples, 1_000, 0.5, 0.1);
        assert_eq!(beats, vec![0.2, 0.6]);
    }

    #[test]
    fn beat_detection_rejects_invalid_rate() {
        assert!(detect_audio_beats(&[1.0], 0, 1.0, 0.1).is_empty());
    }

    #[test]
    fn normalization_gain_targets_rms() {
        let gain = normalization_gain_db(&[0.1; 100], -6.0, 0.0);
        assert!((gain - 14.0).abs() < 0.01);
    }

    #[test]
    fn normalization_gain_respects_peak_ceiling() {
        let mut samples = vec![0.0; 100];
        samples[0] = 1.0;
        let gain = normalization_gain_db(&samples, -6.0, -1.0);
        assert!((gain + 1.0).abs() < 0.01);
    }

    #[test]
    fn normalization_gain_ignores_nonfinite_samples() {
        assert_eq!(normalization_gain_db(&[f32::NAN], -12.0, -1.0), 0.0);
    }

    #[test]
    fn normalization_gain_handles_extreme_finite_samples() {
        let gain = normalization_gain_db(&[f32::MAX, -f32::MAX], -12.0, -1.0);
        assert!(gain.is_finite());
        assert!((-60.0..=60.0).contains(&gain));
    }

    #[test]
    fn audio_clipping_merges_consecutive_samples() {
        let ranges = detect_audio_clipping(&[0.0, 1.0, 1.0, 0.0, -1.0, -1.0], 10, 0.99, 2);
        assert_eq!(
            ranges,
            vec![
                TimeRange {
                    start_seconds: 0.1,
                    end_seconds: 0.3
                },
                TimeRange {
                    start_seconds: 0.4,
                    end_seconds: 0.6
                },
            ]
        );
    }

    #[test]
    fn audio_clipping_ignores_nonfinite_samples() {
        assert!(detect_audio_clipping(&[f32::NAN, 1.0], 10, 0.99, 2).is_empty());
    }

    #[test]
    fn auto_reframe_clamps_to_crop_safe_region() {
        let path = build_auto_reframe_path(&[[0.0, 0.0], [100.0, 50.0]], [100, 50], [40, 20], 0.0);
        assert_eq!(path[0].center, [20.0, 10.0]);
        assert_eq!(path[1].center, [80.0, 40.0]);
    }

    #[test]
    fn auto_reframe_smooths_motion() {
        let path = build_auto_reframe_path(&[[20.0, 20.0], [80.0, 20.0]], [100, 40], [20, 20], 0.5);
        assert_eq!(path[1].center, [50.0, 20.0]);
    }

    #[test]
    fn auto_reframe_rejects_impossible_crop() {
        assert!(build_auto_reframe_path(&[[1.0, 1.0]], [10, 10], [20, 20], 0.5).is_empty());
    }

    #[test]
    fn straight_motion_path_keeps_endpoints() {
        let points = [[0.0, 0.0], [1.0, 1.0], [2.0, 2.0], [3.0, 3.0]];
        assert_eq!(reduce_motion_path(&points, 0.01), vec![0, 3]);
    }

    #[test]
    fn motion_path_keeps_corner() {
        let points = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]];
        assert_eq!(reduce_motion_path(&points, 0.1), vec![0, 1, 2]);
    }

    #[test]
    fn invalid_motion_path_is_preserved() {
        let points = [[0.0, 0.0], [f32::NAN, 1.0], [2.0, 2.0]];
        assert_eq!(reduce_motion_path(&points, 1.0), vec![0, 1, 2]);
    }
}
