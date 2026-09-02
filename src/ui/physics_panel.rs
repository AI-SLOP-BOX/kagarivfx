use crate::core::keyframe::{InterpolationType, Keyframe};
use crate::core::physics::{
    bake_physics_simulation_to_keyframes, calc_spring_overshoot, PhysicsWorld, RigidBody,
    RigidBodyType,
};
use crate::core::property::Animatable;
use crate::ui::custom_widgets;
use crate::ui::theme::colors;
use crate::AfterEffectsApp;
use eframe::egui;

/// 2D Rigid Body Physics & Spring Dynamics Panel for AE Motion Graphics.
pub fn draw_physics_panel(app: &mut AfterEffectsApp, ui: &mut egui::Ui) {
    ui.heading("⚛ Physics & Dynamics");
    ui.label(
        egui::RichText::new("2D Rigid Body Collision Simulation & Keyframe Baker")
            .small()
            .color(colors::TEXT_SECONDARY),
    );
    ui.separator();

    let gravity_id = egui::Id::new("ae_physics_gravity_y");
    let mut gravity_y: f32 = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(gravity_id, || 980.0));

    let bounciness_id = egui::Id::new("ae_physics_bounciness");
    let mut bounciness: f32 = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(bounciness_id, || 0.6));

    let friction_id = egui::Id::new("ae_physics_friction");
    let mut friction: f32 = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(friction_id, || 0.3));

    let floor_id = egui::Id::new("ae_physics_floor_enabled");
    let mut floor_enabled: bool = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(floor_id, || true));

    let collider_shape_id = egui::Id::new("ae_physics_collider_shape");
    let mut collider_shape_idx: usize = ui
        .ctx()
        .data_mut(|d| *d.get_temp_mut_or_insert_with(collider_shape_id, || 0));

    ui.horizontal(|ui| {
        ui.label("Gravity Y:");
        if ui
            .add(egui::Slider::new(&mut gravity_y, -2000.0..=3000.0).suffix(" px/s²"))
            .changed()
        {
            ui.ctx().data_mut(|d| d.insert_temp(gravity_id, gravity_y));
        }
    });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label("Bounciness:");
        if ui
            .add(
                egui::Slider::new(&mut bounciness, 0.0..=1.0)
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
            )
            .changed()
        {
            ui.ctx()
                .data_mut(|d| d.insert_temp(bounciness_id, bounciness));
        }
    });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label("Friction:");
        if ui
            .add(egui::Slider::new(&mut friction, 0.0..=1.0))
            .changed()
        {
            ui.ctx().data_mut(|d| d.insert_temp(friction_id, friction));
        }
    });

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label("Collider Shape:");
        if ui
            .selectable_value(&mut collider_shape_idx, 0, "📦 Box")
            .clicked()
        {
            ui.ctx()
                .data_mut(|d| d.insert_temp(collider_shape_id, collider_shape_idx));
        }
        if ui
            .selectable_value(&mut collider_shape_idx, 1, "⚪ Circle")
            .clicked()
        {
            ui.ctx()
                .data_mut(|d| d.insert_temp(collider_shape_id, collider_shape_idx));
        }
    });

    ui.add_space(4.0);
    if ui
        .checkbox(&mut floor_enabled, "Floor Boundary at Bottom of Comp")
        .changed()
    {
        ui.ctx()
            .data_mut(|d| d.insert_temp(floor_id, floor_enabled));
    }

    ui.separator();

    let comp = app.history.current().active_composition();
    let comp_w = comp.width as f32;
    let comp_h = comp.height as f32;
    let dur = comp.duration_frames;
    let fps = comp.fps as f32;
    let selected_layer_indices: Vec<usize> = app.selection.selected_layers.iter().copied().collect();

    if selected_layer_indices.is_empty() {
        ui.colored_label(
            colors::TEXT_SECONDARY,
            "Select one or more layers in the Timeline to simulate.",
        );
    } else {
        ui.label(format!(
            "🎯 {} layer(s) selected for physics simulation",
            selected_layer_indices.len()
        ));

        ui.add_space(8.0);
        if custom_widgets::ae_button(ui, "🚀 Simulate & Bake to Keyframes").clicked() {
            let mut temp_proj = app.history.current().clone();
            let active_comp = temp_proj.active_composition_mut();

            let mut world = PhysicsWorld::new();
            world.gravity = [0.0, gravity_y];

            // 1. Add static floor if enabled
            if floor_enabled {
                let floor_body = RigidBody::new_box(
                    None,
                    [comp_w * 0.5, comp_h + 20.0],
                    comp_w * 2.0,
                    40.0,
                    0.0,
                    RigidBodyType::Static,
                );
                world.add_body(floor_body);
            }

            // 2. Add selected layers as rigid bodies
            for &idx in &selected_layer_indices {
                if let Some(layer) = active_comp.layers.get(idx) {
                    let pos = layer.transform.position.evaluate(0);
                    let body = if collider_shape_idx == 1 {
                        let mut b = RigidBody::new_circle(
                            Some(idx),
                            pos,
                            30.0,
                            1.0,
                            RigidBodyType::Dynamic,
                        );
                        b.restitution = bounciness;
                        b.friction = friction;
                        b
                    } else {
                        let mut b = RigidBody::new_box(
                            Some(idx),
                            pos,
                            60.0,
                            60.0,
                            1.0,
                            RigidBodyType::Dynamic,
                        );
                        b.restitution = bounciness;
                        b.friction = friction;
                        b
                    };
                    world.add_body(body);
                }
            }

            // 3. Bake simulation
            let baked =
                bake_physics_simulation_to_keyframes(&mut world, 0, dur.saturating_sub(1), fps);

            // 4. Apply baked keyframes to composition layers
            for (layer_idx, (pos_kfs, rot_kfs)) in baked {
                if let Some(layer) = active_comp.layers.get_mut(layer_idx) {
                    layer.transform.position = Animatable::Animated(pos_kfs);
                    layer.transform.rotation = Animatable::Animated(rot_kfs);
                }
            }

            app.history.commit(temp_proj);
            crate::core::frame_cache::bump_version();
            app.toasts.info("Physics simulation baked to keyframes!");
        }
    }

    ui.separator();
    ui.label(egui::RichText::new("Inertia / Bounce Dynamic Keyframes").strong());
    if custom_widgets::ae_button(ui, "⚡ Bake Inertial Bounce on Scale").clicked() {
        if selected_layer_indices.is_empty() {
            app.toasts.error("Select a layer first");
            return;
        }

        let mut temp_proj = app.history.current().clone();
        let active_comp = temp_proj.active_composition_mut();
        for &idx in &selected_layer_indices {
            if let Some(layer) = active_comp.layers.get_mut(idx) {
                let start_f = layer.in_frame;
                let end_f = (start_f + 45).min(layer.out_frame);
                let base_scale = layer.transform.scale.evaluate(start_f);

                let mut kfs = Vec::new();
                for f in start_f..=end_f {
                    let t = (f - start_f) as f32 / fps.max(1.0);
                    let bounce = calc_spring_overshoot(t, 3.5, 6.0, 25.0);
                    let scale_val = [base_scale[0] + bounce, base_scale[1] + bounce];
                    kfs.push(Keyframe::new(f, scale_val, InterpolationType::Linear));
                }
                layer.transform.scale = Animatable::Animated(kfs);
            }
        }

        app.history.commit(temp_proj);
        crate::core::frame_cache::bump_version();
        app.toasts.info("Baked inertial bounce keyframes to Scale!");
    }
}
