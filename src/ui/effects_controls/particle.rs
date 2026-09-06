use crate::ui::theme::colors;
use crate::KagariApp;
use eframe::egui;

pub fn draw_particle_emitter_controls(app: &mut KagariApp, ui: &mut egui::Ui) {
    use crate::core::particle_system::{EmitterShape, ParticleEmitter};
    use crate::core::timeline::LayerType;

    let Some(idx) = app.selection.selected_layer_idx else {
        return;
    };
    let is_particle = matches!(
        app.history.current().active_composition().layers.get(idx),
        Some(l) if matches!(l.layer_type, LayerType::Particle { .. })
    );
    if !is_particle {
        return;
    }

    ui.add_space(6.0);
    ui.separator();
    ui.label(
        egui::RichText::new("Particle Emitter")
            .strong()
            .color(colors::ACCENT_CYAN),
    );

    // Local working copy so sliders feel responsive; committed on change.
    let mut em: ParticleEmitter = match app.history.current().active_composition().layers.get(idx) {
        Some(l) => match &l.layer_type {
            LayerType::Particle { emitter } => emitter.clone(),
            _ => return,
        },
        None => return,
    };
    let mut changed = false;

    ui.collapsing("Emission", |ui| {
        changed |= row_f32(ui, "Rate (p/s)", &mut em.rate, 0.0..=500.0, 1.0);
        changed |= row_u32(ui, "Max particles", &mut em.max_particles, 1..=20000);
        changed |= row_f32(ui, "Lifetime (s)", &mut em.lifetime, 0.1..=30.0, 0.1);
        changed |= row_f32(
            ui,
            "Lifetime var",
            &mut em.lifetime_variance,
            0.0..=1.0,
            0.01,
        );
        changed |= row_f32(ui, "Speed", &mut em.speed, 0.0..=1000.0, 1.0);
        changed |= row_f32(ui, "Speed var", &mut em.speed_variance, 0.0..=2.0, 0.01);
        changed |= row_f32(ui, "Spread (°)", &mut em.spread_degrees, 0.0..=360.0, 1.0);
    });

    ui.collapsing("Shape", |ui| {
        let shapes = [
            ("Point", EmitterShape::Point),
            ("Box", EmitterShape::Box),
            ("Circle", EmitterShape::Circle),
            ("Line", EmitterShape::Line),
            ("Ring", EmitterShape::Ring),
        ];
        ui.horizontal(|ui| {
            for (label, shape) in shapes {
                let selected = em.shape == shape;
                if ui.selectable_label(selected, label).clicked() {
                    em.shape = shape;
                    changed = true;
                }
            }
        });
        changed |= row_f32(ui, "Size X / Ø", &mut em.emitter_size[0], 1.0..=4000.0, 1.0);
        if em.shape == EmitterShape::Box {
            changed |= row_f32(ui, "Size Y", &mut em.emitter_size[1], 1.0..=4000.0, 1.0);
        }
    });

    ui.collapsing("Forces", |ui| {
        ui.horizontal(|ui| {
            ui.label("Gravity:");
            changed |= drag2(ui, &mut em.gravity);
            ui.label("Wind:");
            changed |= drag2(ui, &mut em.wind);
        });
        changed |= row_f32(
            ui,
            "Gust strength",
            &mut em.wind_gust_strength,
            0.0..=300.0,
            1.0,
        );
        changed |= row_f32(
            ui,
            "Gust freq (Hz)",
            &mut em.wind_gust_frequency,
            0.0..=10.0,
            0.05,
        );
        changed |= row_f32(ui, "Turbulence", &mut em.turbulence, 0.0..=300.0, 1.0);
        changed |= row_f32(ui, "Air drag", &mut em.drag, 0.0..=20.0, 0.05);
    });

    ui.collapsing("Look", |ui| {
        changed |= row_f32(ui, "Size start", &mut em.size_start, 0.0..=200.0, 0.5);
        changed |= row_f32(ui, "Size end", &mut em.size_end, 0.0..=200.0, 0.5);
    });

    ui.collapsing("Collisions", |ui| {
        if ui
            .checkbox(&mut em.collision_enabled, "Boundary collisions")
            .changed()
        {
            changed = true;
        }
        if em.collision_enabled {
            ui.horizontal(|ui| {
                ui.label("Bounds x0/y0/x1/y1:");
                for v in &mut em.collision_bounds {
                    if ui.add(egui::DragValue::new(v).speed(1.0)).changed() {
                        changed = true;
                    }
                }
            });
            changed |= row_f32(ui, "Restitution", &mut em.restitution, 0.0..=1.0, 0.01);
            changed |= row_f32(ui, "Friction", &mut em.surface_friction, 0.0..=1.0, 0.01);
        }
        if ui
            .checkbox(&mut em.particle_collisions, "Particle ↔ particle")
            .changed()
        {
            changed = true;
        }
        if em.particle_collisions {
            changed |= row_f32(
                ui,
                "Contact Ø (px)",
                &mut em.particle_diameter,
                0.5..=200.0,
                0.5,
            );
        }
    });

    if changed {
        app.modify_project(move |p| {
            let comp = p.active_composition_mut();
            if let Some(layer) = comp.layers.get_mut(idx) {
                if let LayerType::Particle { emitter } = &mut layer.layer_type {
                    *emitter = em.clone();
                }
            }
        });
        crate::core::frame_cache::bump_version();
    }
}

fn row_f32(
    ui: &mut egui::Ui,
    label: &str,
    val: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    speed: f32,
) -> bool {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::DragValue::new(val).speed(speed).range(range))
            .changed()
    })
    .inner
}

fn row_u32(
    ui: &mut egui::Ui,
    label: &str,
    val: &mut u32,
    range: std::ops::RangeInclusive<u32>,
) -> bool {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::DragValue::new(val).range(range)).changed()
    })
    .inner
}

fn drag2(ui: &mut egui::Ui, v: &mut [f32; 2]) -> bool {
    let mut ch = false;
    for c in v.iter_mut() {
        if ui.add(egui::DragValue::new(c).speed(1.0)).changed() {
            ch = true;
        }
    }
    ch
}
