use super::{controls::targets, database, elastic, elements, landing, plant, trim::condition};
use crate::physics::*;
use bevy::math::DVec3;
#[test]
fn geometry_partitions_and_database_are_finite_continuous_and_symmetric() {
    let g = elements::geometry();
    assert_eq!(g.len(), 30);
    for sum in [
        g.iter().map(|g| g.normal_share).sum::<f64>(),
        g.iter().map(|g| g.drag_share).sum(),
        g.iter().map(|g| g.side_share).sum(),
    ] {
        assert!((sum - 1.).abs() < 1e-12);
    }
    for m in [0., 0.6, 0.9, 1., 1.1, 2., 3.2, 4.] {
        for a in [-3., -1.5, -0.5, 0., 0.2, 0.5, 1.5, 3.] {
            let p = database::lookup(m, a, 0.1);
            let n = database::lookup(m, a, -0.1);
            assert!(p.lift.is_finite() && p.drag >= 0. && p.pitch.is_finite());
            assert!((p.side + n.side).abs() < 1e-10);
            assert!((p.yaw + n.yaw).abs() < 1e-10);
        }
    }
    for m in database::MACH_AXIS {
        let a = database::lookup(m - 1e-7, 0.1, 0.);
        let b = database::lookup(m + 1e-7, 0.1, 0.);
        assert!((a.lift - b.lift).abs() < 1e-5);
    }
}
#[test]
fn local_rates_and_asymmetric_deformation_create_local_load_differences() {
    let mut c = condition(0.1, 10000., 300.);
    let b = DVec3::new(0., -300. * c.alpha.sin(), -300. * c.alpha.cos());
    let mut s = plant::State::default();
    s.elastic_enabled = false;
    plant::prepare(
        &mut s,
        c,
        b,
        0.22,
        landing::Surface::free(),
        &AircraftDamage::default(),
        DVec3::ZERO,
        DVec3::ZERO,
        0.,
        true,
    );
    for i in 0..6 {
        assert!((s.local[i].force.y - s.local[i + 6].force.y).abs() < 1e-7);
    }
    c.rates.z = 0.2;
    plant::prepare(
        &mut s,
        c,
        b,
        0.22,
        landing::Surface::free(),
        &AircraftDamage::default(),
        DVec3::ZERO,
        DVec3::ZERO,
        0.,
        true,
    );
    assert!(s.local[4].alpha > s.local[10].alpha);
    assert!((s.local[4].force - s.local[10].force).length() > 100.);
}
#[test]
fn supporting_plane_and_landing_configuration_affect_loads() {
    let c = condition(0.12, 1000., 120.);
    let b = DVec3::new(0., -120. * c.alpha.sin(), -120. * c.alpha.cos());
    let d = AircraftDamage::default();
    let mut far = plant::State::default();
    far.elastic_enabled = false;
    plant::prepare(
        &mut far,
        c,
        b,
        0.22,
        landing::Surface::free(),
        &d,
        DVec3::ZERO,
        DVec3::ZERO,
        0.,
        true,
    );
    let mut close = plant::State::default();
    close.elastic_enabled = false;
    close.gear_extension = 1.;
    close.gear_door = 0.5;
    let surface = landing::Surface {
        normal_body: DVec3::Y,
        distance_m: 2.,
        relative_velocity_body: DVec3::new(0., -3., -120.),
    };
    plant::prepare(
        &mut close,
        c,
        b,
        0.22,
        surface,
        &d,
        DVec3::ZERO,
        DVec3::ZERO,
        0.,
        true,
    );
    assert!(close.mean_pressure > 0.5);
    assert!(close.base.force.y > far.base.force.y);
    assert!(close.coefficients.parasite > far.coefficients.parasite + 0.05);
    assert!(
        landing::Surface {
            normal_body: DVec3::new(0.2, 1., 0.).normalize(),
            ..surface
        }
        .clearance(DVec3::X * 5.)
            > surface.clearance(DVec3::X * 5.)
    );
}
#[test]
fn no_airflow_produces_no_aerodynamic_force_or_moment() {
    let c = condition(0., 5000., 0.);
    let s = plant::steady(
        c,
        DVec3::ZERO,
        0.22,
        &AircraftDamage::default(),
        targets(DVec3::ZERO),
        [0.; 2],
        landing::Surface::free(),
        true,
    );
    assert_eq!(s.total.force, DVec3::ZERO);
    assert_eq!(s.total.moment, DVec3::ZERO);
}
#[test]
fn elastic_free_response_dissipates_energy_and_section_strain_matches_constitutive_law() {
    let mut s = elastic::State::default();
    s.modes[0].bend_m = 0.1;
    s.modes[0].twist_rad = 0.01;
    let empty = [elastic::PointLoad::default(); elastic::LOAD_COUNT];
    s.advance(
        &empty,
        DVec3::ZERO,
        DVec3::ZERO,
        DVec3::ZERO,
        1. / 240.,
        false,
    );
    let initial = s.energy_j;
    for _ in 0..2400 {
        s.advance(
            &empty,
            DVec3::ZERO,
            DVec3::ZERO,
            DVec3::ZERO,
            1. / 240.,
            false,
        );
    }
    assert!(s.energy_j < initial * 1e-4);
    let mut loads = empty;
    loads[0] = elastic::PointLoad {
        assembly: Some(0),
        point: elements::root(0) - DVec3::X * 4.,
        force: DVec3::Y * 10000.,
        moment: DVec3::ZERO,
    };
    s.advance(&loads, DVec3::ZERO, DVec3::ZERO, DVec3::ZERO, 0., true);
    let root = s.sections[0][0];
    assert!((root.bending_nm - 40000.).abs() < 1e-8);
    assert!(
        (root.normal_strain
            - root.normal_stress_pa / super::airframe_config::ELASTIC.young_modulus_pa)
            .abs()
            < 1e-12
    );
    assert!(s.modes[0].bend_m > 0.);
}

#[test]
fn dynamic_separation_has_hysteresis_vortex_convection_and_recovery() {
    let mut state = super::unsteady::State::equilibrium(0.05, 0., 0.5);
    let mut up = 0.;
    let mut down = 0.;
    let mut peak_vortex: f64 = 0.;
    for step in 0..240 * 8 {
        let t = step as f64 / 240.;
        let alpha = if t < 1. {
            0.05 + 0.65 * t
        } else if t < 3. {
            0.70
        } else if t < 4. {
            0.70 - 0.65 * (t - 3.)
        } else {
            0.05
        };
        state.advance(alpha, 0., 0.5, 150., 6., 1. / 240.);
        peak_vortex = peak_vortex.max(state.vortex.abs());
        if (t - 0.4).abs() < 1. / 480. {
            up = state.attached;
        }
        if (t - 3.6).abs() < 1. / 480. {
            down = state.attached;
        }
        assert!(state.attached.is_finite() && (0.0..=1.0).contains(&state.attached));
        assert!(state.vortex.abs() <= super::airframe_config::UNSTEADY.vortex_limit + 1e-9);
    }
    assert!(down < up, "no separation hysteresis: up={up}, down={down}");
    assert!(peak_vortex > 0.01);
    assert!(state.attached > 0.98);
}
#[test]
fn coupled_airframe_converges_at_genuinely_different_internal_steps() {
    let mut results = Vec::new();
    for hz in [240, 480, 960] {
        let mut c = condition(0.1, 6000., 250.);
        let b = DVec3::new(0., -250. * c.alpha.sin(), -250. * c.alpha.cos());
        let damage = AircraftDamage::default();
        let mut s = plant::steady(
            c,
            b,
            0.22,
            &damage,
            targets(DVec3::ZERO),
            [0.; 2],
            landing::Surface::free(),
            true,
        );
        let mut peak_force: f64 = 0.;
        let mut peak_bend: f64 = 0.;
        for step in 0..hz * 6 {
            let t = (step + 1) as f64 / hz as f64;
            let freq = std::f64::consts::TAU * 0.7;
            c.alpha = 0.1 + 0.025 * (freq * t).sin();
            c.rates.x = 0.025 * freq * (freq * t).cos();
            let angular = DVec3::X * (-0.025 * freq * freq * (freq * t).sin());
            let b = DVec3::new(0., -250. * c.alpha.sin(), -250. * c.alpha.cos());
            plant::prepare(
                &mut s,
                c,
                b,
                0.22,
                landing::Surface::free(),
                &damage,
                DVec3::Y * 9.8,
                angular,
                1. / hz as f64,
                false,
            );
            let control = plant::control_loads(
                &s,
                &damage,
                targets(DVec3::new(0.03 * (freq * t).sin(), 0., 0.02)),
                [0.; 2],
            );
            plant::finish(&mut s, &control);
            assert!(s.structure.valid && s.total.force.is_finite() && s.total.moment.is_finite());
            peak_force = peak_force.max(s.total.force.length());
            peak_bend = peak_bend.max(s.structure.modes[0].bend_m.abs());
        }
        println!(
            "SR71 AIRFRAME INTERNAL {hz}Hz force={:?} bend={} peak_force={peak_force} peak_bend={peak_bend}",
            s.total.force, s.structure.modes[0].bend_m
        );
        results.push((s, peak_force, peak_bend));
    }
    for (s, peak_force, peak_bend) in &results[..2] {
        let reference = &results[2];
        assert!(
            s.total.force.distance(reference.0.total.force) / reference.0.total.force.length()
                < 0.02
        );
        assert!((peak_force - reference.1).abs() / reference.1 < 0.02);
        assert!((peak_bend - reference.2).abs() / reference.2 < 0.02);
    }
}
#[test]
fn stress_strain_scale_with_load_and_progressive_overload_reduces_stiffness() {
    let mut loads = [elastic::PointLoad::default(); elastic::LOAD_COUNT];
    loads[0] = elastic::PointLoad {
        assembly: Some(1),
        point: elements::root(1) + DVec3::X * 5.,
        force: DVec3::Y * 10000.,
        moment: DVec3::ZERO,
    };
    let mut a = elastic::State::default();
    a.advance(&loads, DVec3::ZERO, DVec3::ZERO, DVec3::ZERO, 0., true);
    let section = a.sections[1][0];
    let bend = a.modes[1].bend_m;
    loads[0].force *= 2.;
    let mut b = elastic::State::default();
    b.advance(&loads, DVec3::ZERO, DVec3::ZERO, DVec3::ZERO, 0., true);
    assert!((b.sections[1][0].normal_strain / section.normal_strain - 2.).abs() < 1e-9);
    assert!((b.modes[1].bend_m / bend - 2.).abs() < 1e-9);
    loads[0].force *= 100.;
    for _ in 0..240 {
        b.advance(
            &loads,
            DVec3::ZERO,
            DVec3::ZERO,
            DVec3::ZERO,
            1. / 240.,
            false,
        );
    }
    assert!(b.damage[1] > 0.1);
    assert!(b.sections[1][0].utilization > 1.);
    assert!(b.valid);
}
#[test]
fn frozen_actuator_and_surface_release_keep_distributed_control_budget_consistent() {
    let c = condition(0.12, 8000., 280.);
    let b = DVec3::new(0., -280. * c.alpha.sin(), -280. * c.alpha.cos());
    let mut d = AircraftDamage::default();
    let s = plant::steady(
        c,
        b,
        0.22,
        &d,
        targets(DVec3::ZERO),
        [0.; 2],
        landing::Surface::free(),
        true,
    );
    d.surface_state[2] = crate::combat::damage::SurfaceState::Jammed(0.10);
    let a = plant::control_loads(&s, &d, targets(DVec3::new(0.2, 0.1, -0.1)), [0.05; 2]);
    let f = a.surfaces.iter().map(|l| l.force).sum::<DVec3>();
    let m = a.surfaces.iter().map(|l| l.moment).sum::<DVec3>();
    assert!((f - a.force).length() < 1e-7);
    assert!((m - a.moment).length() < 1e-7);
    assert_eq!(a.surfaces[2].deflection, 0.10);
}
#[test]
fn unassisted_high_alpha_sideslip_and_reversals_remain_finite() {
    let mut f = super::study::airborne(10000., 250.);
    f.assist = false;
    f.throttle = 0.6;
    let env = Environment {
        wind_enabled: false,
        ..Default::default()
    };
    for step in 0..240 * 20 {
        let t = step as f64 / 240.;
        f.controls = if t < 2. {
            DVec3::new(0.35, 0.1, 0.)
        } else if t < 4. {
            DVec3::new(-0.2, -0.1, 0.2)
        } else {
            DVec3::ZERO
        };
        simulate(&mut f, &env, 1. / 240.);
        assert!(
            f.position.is_finite()
                && f.velocity.is_finite()
                && f.rates.is_finite()
                && f.systems.sr71.airframe.structure.valid
        );
        if f.crashed {
            break;
        }
    }
}

#[test]
fn section_corner_stress_includes_edge_bending_and_tension_compression() {
    let mut loads = [elastic::PointLoad::default(); elastic::LOAD_COUNT];
    loads[0] = elastic::PointLoad {
        assembly: Some(1),
        point: elements::root(1) + DVec3::X * 4.,
        force: DVec3::new(-3000., 10000., 5000.),
        moment: DVec3::X * 2000.,
    };
    let mut state = elastic::State::default();
    state.advance(&loads, DVec3::ZERO, DVec3::ZERO, DVec3::ZERO, 0., true);
    let s = state.sections[1][0];
    assert!(s.edge_bending_nm.abs() > 1000. && s.axial_n < 0. && s.torsion_nm.abs() > 1000.);
    assert!(
        s.corner_stress_pa.iter().any(|s| *s > 0.) && s.corner_stress_pa.iter().any(|s| *s < 0.)
    );
    for i in 0..4 {
        assert!(
            (s.corner_strain[i]
                - s.corner_stress_pa[i] / super::airframe_config::ELASTIC.young_modulus_pa)
                .abs()
                < 1e-12
        );
    }
    assert!(state.modes[1].edge_m.abs() > 0.);
}
#[test]
fn wing_inertia_includes_completed_ground_contact_acceleration() {
    let mut f = Flight::runway();
    crate::aircraft::select(&mut f, crate::aircraft::Aircraft::Sr71);
    f.engine = false;
    f.throttle = 0.;
    f.spool = 0.;
    f.paused = false;
    let env = Environment {
        wind_enabled: false,
        ..Default::default()
    };
    for _ in 0..1200 {
        simulate(&mut f, &env, 1. / 240.);
        assert!(f.position.is_finite() && f.systems.sr71.airframe.structure.valid);
    }
    assert!(f.systems.sr71.airframe.last_specific_acceleration.y > 5.);
    assert!(f.systems.sr71.airframe.structure.modes[0].bend_m < 0.);
    assert!(f.systems.sr71.airframe.structure.inertial_root_nm[0].abs() > 100.);
}
#[test]
fn distributed_indicial_state_handles_angle_wrap_without_a_rate_spike() {
    let pi = std::f64::consts::PI;
    let mut s = super::unsteady::State::equilibrium(pi - 0.001, 0., 0.6);
    s.advance(-pi + 0.001, 0., 0.6, 200., 5., 0.01);
    assert!(s.alpha_rate.abs() < 0.21);
    assert!(s.vortex.abs() < 0.01);
}

#[test]
fn structural_health_is_not_counted_twice_after_shared_damage_update() {
    let c = condition(0.1, 10000., 300.);
    let b = DVec3::new(0., -300. * c.alpha.sin(), -300. * c.alpha.cos());
    let mut clean = plant::State::default();
    clean.elastic_enabled = false;
    plant::prepare(
        &mut clean,
        c,
        b,
        0.22,
        landing::Surface::free(),
        &AircraftDamage::default(),
        DVec3::ZERO,
        DVec3::ZERO,
        0.,
        true,
    );
    let mut damaged = plant::State::default();
    damaged.elastic_enabled = false;
    damaged.structure.damage[0] = 0.5;
    let mut d = AircraftDamage::default();
    d.wings[0] = 0.5;
    plant::prepare(
        &mut damaged,
        c,
        b,
        0.22,
        landing::Surface::free(),
        &d,
        DVec3::ZERO,
        DVec3::ZERO,
        0.,
        true,
    );
    for i in 0..6 {
        let lift_axis = DVec3::X.cross(b.normalize()).normalize();
        assert!(
            (damaged.local[i].force - clean.local[i].force * 0.5)
                .dot(lift_axis)
                .abs()
                < 1e-7
        );
    }
}

#[test]
fn induced_drag_reaches_structural_elements_once_and_preserves_force_budget() {
    let c = condition(0.12, 8000., 280.);
    let b = DVec3::new(0., -280. * c.alpha.sin(), -280. * c.alpha.cos());
    let damage = AircraftDamage::default();
    let mut s = plant::State::default();
    s.elastic_enabled = false;
    plant::prepare(
        &mut s,
        c,
        b,
        0.22,
        landing::Surface::free(),
        &damage,
        DVec3::ZERO,
        DVec3::ZERO,
        0.,
        true,
    );
    let baseline = s.local.iter().map(|l| l.force).sum::<DVec3>();
    assert!((baseline - s.base.force).length() < 1e-7);
    let loads = plant::control_loads(&s, &damage, targets(DVec3::new(0.3, 0., 0.1)), [0.; 2]);
    plant::finish(&mut s, &loads);
    let structural = s.last_loads.iter().map(|l| l.force).sum::<DVec3>();
    assert!((structural - s.total.force).length() < 1e-7);
    let moment = s
        .last_loads
        .iter()
        .map(|l| (l.point - s.cg).cross(l.force) + l.moment)
        .sum::<DVec3>();
    let unresolved = s.base.moment - s.local.iter().map(|l| l.moment).sum::<DVec3>();
    assert!((moment + unresolved - s.total.moment).length() < 1e-6);
}
