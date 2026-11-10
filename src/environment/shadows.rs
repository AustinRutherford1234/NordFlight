use bevy::{
    light::{Cascades, DirectionalLight, SimulationLightSystems, cascade::Cascade},
    math::{DMat3, DQuat, DVec3},
    prelude::*,
};

pub(super) fn install(app: &mut App) {
    app.add_systems(
        PostUpdate,
        anchor_sun_cascades
            .after(SimulationLightSystems::UpdateDirectionalLightCascades)
            .before(SimulationLightSystems::UpdateLightFrusta),
    );
}

pub(crate) fn sun_rotation(direction: DVec3) -> DQuat {
    // Choose a body-fixed roll so the grid does not rotate when local up
    // changes at a floating-origin boundary.
    let right = DVec3::Y.cross(direction).normalize_or(DVec3::X);
    DQuat::from_mat3(&DMat3::from_cols(right, direction.cross(right), direction))
}

fn anchor_sun_cascades(
    flight: Res<crate::physics::Flight>,
    environment: Res<crate::physics::Environment>,
    mut suns: Query<(&DirectionalLight, &mut Cascades), With<crate::world::Sun>>,
) {
    for (light, mut cascades) in &mut suns {
        if !light.shadows_enabled {
            continue;
        }
        let origin = crate::world::render_origin(&flight) - DVec3::Y * super::radius();
        let origin_in_light = sun_rotation(environment.sun_direction()).conjugate() * origin;
        for cascade in cascades.cascades.values_mut().flatten() {
            anchor_cascade(cascade, origin_in_light);
        }
    }
}

fn anchor_cascade(cascade: &mut Cascade, origin_in_light: DVec3) {
    let texel = f64::from(cascade.texel_size);
    // Bevy has already snapped the center on the render-local grid. Translate
    // that grid to the nearest world texel without changing its depth range.
    let phase = |coordinate: f64| {
        let offset = coordinate / texel;
        ((offset.round() - offset) * texel) as f32
    };
    let x = phase(origin_in_light.x);
    let y = phase(origin_in_light.y);
    let world = &mut cascade.world_from_cascade;
    world.w_axis += world.x_axis * x + world.y_axis * y;
    cascade.clip_from_world.w_axis.x -= cascade.clip_from_cascade.x_axis.x * x;
    cascade.clip_from_world.w_axis.y -= cascade.clip_from_cascade.y_axis.y * y;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fixed_point_keeps_its_shadow_texel_phase_across_rebases() {
        let point = DVec3::new(322_419.37, 453_992.15, 0.);
        for texel in [0.18_f32, 2.7] {
            let diameter = texel * 1024.;
            let clip = Mat4::from_scale(Vec3::new(2. / diameter, 2. / diameter, 0.001));
            let mut phases = Vec::new();
            for origin in [
                DVec3::new(321_825.22, 452_996.46, 0.),
                DVec3::new(323_786.73, 454_386.67, 0.),
            ] {
                let mut cascade = Cascade {
                    world_from_cascade: Mat4::IDENTITY,
                    clip_from_cascade: clip,
                    clip_from_world: clip,
                    texel_size: texel,
                };
                anchor_cascade(&mut cascade, origin);
                let uv = cascade
                    .clip_from_world
                    .transform_point3((point - origin).as_vec3());
                phases.push(uv.xy() * 512.);
                assert!(
                    (cascade.clip_from_world - clip * cascade.world_from_cascade.inverse())
                        .abs_diff_eq(Mat4::ZERO, 1e-7)
                );
            }
            let shift = phases[0] - phases[1];
            assert!(shift.distance(shift.round()) < 0.002, "{shift:?}");
        }
    }
}
