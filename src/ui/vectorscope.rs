//! 📊 Vectorscope + RGB Parade overlay (Shift+F4)
//!
//! Renders the active composition at a reduced resolution through the CPU
//! reference path, then plots either a BT.709 Cb/Cr vectorscope cloud or
//! per-channel parade columns. Pure math helpers are unit-tested below.

use crate::core::software_renderer;
use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

/// BT.709 RGB(0..1) → (Cb, Cr), each normalized to ±0.5.
pub fn rgb_to_cbcr(r: f32, g: f32, b: f32) -> [f32; 2] {
    let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    [(b - y) * 0.565, (r - y) * 0.713]
}

/// One plotted sample: normalized chroma position + source color for tinting.
#[derive(Debug, Clone, Copy)]
pub struct ScopeSample {
    /// Cb, Cr in ±0.5
    pub cbcr: [f32; 2],
    /// Source RGB (0..255) used to tint the plotted dot
    pub rgb: [u8; 3],
}

/// Collects chroma samples for every non-transparent pixel.
pub fn scope_samples(pixels: &[u8], out: &mut Vec<ScopeSample>) {
    out.clear();
    for px in pixels.chunks_exact(4) {
        if px[3] == 0 {
            continue;
        }
        let cbcr = rgb_to_cbcr(
            px[0] as f32 / 255.0,
            px[1] as f32 / 255.0,
            px[2] as f32 / 255.0,
        );
        out.push(ScopeSample {
            cbcr,
            rgb: [px[0], px[1], px[2]],
        });
    }
}

/// Per-channel histogram buckets (R, G, B), values bucketed into 64 bins.
pub const PARADE_BUCKETS: usize = 64;
pub type ParadeCounts = [[u32; PARADE_BUCKETS]; 3];

pub fn parade_buckets(pixels: &[u8], counts: &mut ParadeCounts) {
    *counts = [[0; PARADE_BUCKETS]; 3];
    for px in pixels.chunks_exact(4) {
        if px[3] == 0 {
            continue; // fully transparent → no signal
        }
        for (bucket, chan) in counts.iter_mut().zip(px[..3].iter()) {
            let idx = (*chan as usize * PARADE_BUCKETS) / 256;
            bucket[idx.min(PARADE_BUCKETS - 1)] += 1;
        }
    }
}

const MODE_ID: &str = "vectorscope_mode";

pub fn draw_vectorscope_window(app: &mut KagariApp, ctx: &egui::Context) {
    if !app.show_vectorscope {
        return;
    }
    let mut mode: u8 =
        ctx.data_mut(|d| *d.get_temp_mut_or_insert_with(egui::Id::new(MODE_ID), || 0u8));

    egui::Window::new("📊 Vectorscope / RGB Parade")
        .open(&mut app.show_vectorscope)
        .default_width(260.0)
        .show(ctx, |ui| {
            let comp = app.history.current().active_composition();
            let w = 192u32;
            let h = ((w * comp.height.max(1)) / comp.width.max(1)).clamp(16, 240);
            // Scopes conventionally show the unmanaged signal: exposure/LUT 0.
            let pixels =
                software_renderer::render_frame_to_pixels(comp, app.playback.current_frame, w, h, 0.0, 0);

            ui.horizontal(|ui| {
                ui.radio_value(&mut mode, 0u8, "Vectorscope");
                ui.radio_value(&mut mode, 1u8, "RGB Parade");
                if pixels.is_empty() {
                    ui.weak("(empty)");
                }
            });

            let side = ui.available_width().min(240.0);
            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(side, side * 0.75), egui::Sense::hover());
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(10, 10, 12));
            painter.rect_stroke(rect, 4.0, egui::Stroke::new(1.0, colors::BORDER_MEDIUM));

            if pixels.is_empty() {
                return;
            }

            match mode {
                0 => draw_scope(painter, rect, &pixels),
                _ => draw_parade(painter, rect, &pixels),
            }
        });

    ctx.data_mut(|d| d.insert_temp(egui::Id::new(MODE_ID), mode));
}

fn draw_scope(painter: egui::Painter, rect: egui::Rect, pixels: &[u8]) {
    let center = rect.center();
    let radius = rect.width().min(rect.height()) * 0.47;

    // Graticule: outer ring (100%), inner ring (75%), cross axes, skin line.
    painter.circle_stroke(center, radius, egui::Stroke::new(1.0, colors::GRID_LINE));
    painter.circle_stroke(
        center,
        radius * 0.75,
        egui::Stroke::new(0.7, colors::GRID_LINE),
    );
    painter.line_segment(
        [
            egui::pos2(center.x - radius, center.y),
            egui::pos2(center.x + radius, center.y),
        ],
        egui::Stroke::new(0.7, colors::GRID_LINE),
    );
    painter.line_segment(
        [
            egui::pos2(center.x, center.y - radius),
            egui::pos2(center.x, center.y + radius),
        ],
        egui::Stroke::new(0.7, colors::GRID_LINE),
    );
    let skin_dir = {
        let c = rgb_to_cbcr(1.0, 0.80, 0.60);
        let len = (c[0] * c[0] + c[1] * c[1]).sqrt().max(1e-4);
        [c[0] / len, c[1] / len]
    };
    painter.line_segment(
        [
            egui::pos2(
                center.x - skin_dir[0] * radius,
                center.y - skin_dir[1] * radius,
            ),
            egui::pos2(
                center.x + skin_dir[0] * radius,
                center.y + skin_dir[1] * radius,
            ),
        ],
        egui::Stroke::new(0.8, egui::Color32::from_rgb(120, 90, 60)),
    );

    let mut samples = Vec::with_capacity(4096);
    scope_samples(pixels, &mut samples);
    let stride = (samples.len() / 15000).max(1);
    for s in samples.iter().step_by(stride) {
        let x = center.x + s.cbcr[0] * 2.0 * radius;
        let y = center.y - s.cbcr[1] * 2.0 * radius;
        let col = egui::Color32::from_rgba_unmultiplied(s.rgb[0], s.rgb[1], s.rgb[2], 110);
        painter.circle_filled(egui::pos2(x, y), 1.0, col);
    }
}

fn draw_parade(painter: egui::Painter, rect: egui::Rect, pixels: &[u8]) {
    let mut counts: ParadeCounts = [[0; PARADE_BUCKETS]; 3];
    parade_buckets(pixels, &mut counts);

    let band_w = rect.width() / 3.0;
    let labels = ["R", "G", "B"];
    let tints = [
        egui::Color32::from_rgb(220, 70, 70),
        egui::Color32::from_rgb(80, 210, 110),
        egui::Color32::from_rgb(90, 130, 230),
    ];
    for ch in 0..3 {
        let x0 = rect.left() + band_w * ch as f32;
        let band = egui::Rect::from_min_size(
            egui::pos2(x0, rect.top()),
            egui::vec2(band_w, rect.height()),
        );
        painter.rect_stroke(band, 0.0, egui::Stroke::new(0.6, colors::GRID_LINE));
        let max = counts[ch].iter().copied().max().unwrap_or(1).max(1);
        for &cnt in counts[ch].iter() {
            if cnt == 0 {
                continue;
            }
            let bar_h = rect.height() * (cnt as f32 / max as f32);
            let bw = band_w - 2.0;
            let bx = band.left() + 1.0;
            let by = band.bottom() - bar_h;
            painter.rect_filled(
                egui::Rect::from_min_size(egui::pos2(bx, by), egui::vec2(bw, bar_h)),
                0.0,
                tints[ch].linear_multiply(180.0 / 255.0),
            );
        }
        painter.text(
            egui::pos2(band.center().x, band.top() + 8.0),
            egui::Align2::CENTER_CENTER,
            labels[ch],
            egui::FontId::proportional(10.0),
            colors::TEXT_MUTED,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_and_black_map_to_center() {
        let w = rgb_to_cbcr(1.0, 1.0, 1.0);
        assert!(w[0].abs() < 1e-4 && w[1].abs() < 1e-4, "white: {:?}", w);
        let k = rgb_to_cbcr(0.0, 0.0, 0.0);
        assert!(k[0].abs() < 1e-6 && k[1].abs() < 1e-6);
    }

    #[test]
    fn primaries_land_in_expected_quadrants() {
        let red = rgb_to_cbcr(1.0, 0.0, 0.0);
        assert!(red[1] > 0.5 && red[0] < 0.0, "red: {:?}", red);
        let blue = rgb_to_cbcr(0.0, 0.0, 1.0);
        assert!(blue[0] > 0.5 && blue[1] < 0.0, "blue: {:?}", blue);
        let green = rgb_to_cbcr(0.0, 1.0, 0.0);
        assert!(green[0] < 0.0 && green[1] < 0.0, "green: {:?}", green);
    }

    #[test]
    fn transparent_pixels_skipped_counts_match() {
        let px = vec![
            128u8, 128, 128, 255, 0, 0, 0, 0, // skipped
            255, 255, 255, 255,
        ];
        let mut samples = Vec::new();
        scope_samples(&px, &mut samples);
        assert_eq!(samples.len(), 2);

        let mut counts: ParadeCounts = [[0; PARADE_BUCKETS]; 3];
        parade_buckets(&px, &mut counts);
        for (ch, bucket) in counts.iter().enumerate() {
            let total: u32 = bucket.iter().sum();
            assert_eq!(total, 2, "channel {} counted opaque pixels", ch);
        }
        assert_eq!(counts[0][32], 1); // gray 128 → bucket 32
        assert_eq!(counts[0][63], 1); // white → last bucket
    }
}
