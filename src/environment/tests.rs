use super::*;
#[test]
fn geographic_round_trips_cover_poles_and_wrap() {
    for r in [6371000., 6500000.] {
        for (lon, lat) in [
            (12.711111, 58.428333),
            (-179.9, -70.),
            (179.9, 80.),
            (0., 90.),
            (0., -90.),
        ] {
            let p = geo_to_local(lon, lat, 321., r);
            let (l, b) = local_to_geo(p);
            assert!((p.length() - r - 321.).abs() < 1e-7);
            assert!((b - lat).abs() < 1e-6);
            if lat.abs() < 89. {
                assert!((l - lon).abs() < 1e-8);
            }
        }
    }
}
#[test]
fn custom_atmospheres_are_hydrostatic_and_bounded() {
    let mut p = EnvironmentSnapshot::legacy().preset.planet;
    p.atmosphere = AtmosphereProfile::Dry {
        temperature: 280.,
        pressure: 100000.,
        gas_constant: 287.,
        gamma: 1.4,
        top: 100000.,
    };
    p.validate().unwrap();
    let a = p.atmosphere(0.);
    let b = p.atmosphere(1000.);
    assert!(b.1 < a.1 && b.2 < a.2);
    assert!((a.1 - a.2 * 287. * a.0).abs() < 1e-6);
    assert_eq!(p.atmosphere(100000.).1, 0.);
    p.atmosphere = AtmosphereProfile::Vacuum;
    assert_eq!(p.atmosphere(0.).2, 0.);
    p.mu *= 0.5;
    assert!(p.surface_gravity() < 5.);
}
#[test]
fn bad_presets_and_graphics_fail_before_flight() {
    let mut p = EnvironmentSnapshot::legacy().preset;
    p.planet.radius = f64::NAN;
    assert!(p.validate().is_err());
    p = EnvironmentSnapshot::legacy().preset;
    p.packs = vec!["../outside".into()];
    assert!(p.validate().is_err());
    let mut g = GraphicsSettings::default();
    g.render_scale = f32::NAN;
    assert!(g.validate().is_err());
}
#[test]
fn grid_interpolation_is_continuous_and_classes_are_categorical() {
    let g = pack::Grid {
        n: 2,
        heights: vec![10, 20, 30, 40],
        cover: vec![10, 80, 30, 40],
        cover_n: 2,
    };
    assert_eq!(g.sample(0.5, 0.5).0, 25.);
    assert_eq!(g.sample(1., 0.).1, 80);
    assert!((g.sample(0.5 - 1e-6, 0.5).0 - g.sample(0.5 + 1e-6, 0.5).0).abs() < 0.001);
}
#[test]
fn installed_packs_are_real_and_fit_budget() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/environment/packs");
    let data = pack::TerrainData::load(
        &root,
        &["sweden-west".into(), "norway-mountains".into()],
        384 * 1024 * 1024,
    )
    .unwrap();
    assert!(!data.global.is_empty());
    assert_eq!(data.manifests.len(), 2);
    let (h, c, d) = data.sample(7.5, 61.5);
    assert!(d && h > 100., "{h} {c}");
    assert!(data.bytes < 384 * 1024 * 1024);
}
#[test]
fn deterministic_noise_has_no_short_repeat_or_cell_steps() {
    assert_eq!(noise(-31., 72., 100., 3), noise(-31., 72., 100., 3));
    assert!((noise(100. - 1e-4, 0., 100., 3) - noise(100. + 1e-4, 0., 100., 3)).abs() < 1e-8);
    assert_ne!(noise(0., 0., 100., 3), noise(10000., 0., 100., 3));
}

#[test]
fn neighboring_installed_tiles_share_identical_borders() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/environment/packs");
    let data = pack::TerrainData::load(
        &root,
        &["sweden-west".into(), "norway-mountains".into()],
        384 * 1024 * 1024,
    )
    .unwrap();
    for (&(x, y), a) in &data.grids {
        if let Some(b) = data.grids.get(&(x + 1, y)) {
            for j in 0..a.cover_n {
                assert_eq!(
                    a.cover[j * a.cover_n + a.cover_n - 1],
                    b.cover[j * b.cover_n]
                );
            }
            for j in 0..a.n {
                assert_eq!(a.heights[j * a.n + a.n - 1], b.heights[j * b.n]);
            }
        }
        if let Some(b) = data.grids.get(&(x, y + 1)) {
            for i in 0..a.cover_n {
                assert_eq!(a.cover[i], b.cover[(b.cover_n - 1) * b.cover_n + i]);
            }
            for i in 0..a.n {
                assert_eq!(a.heights[i], b.heights[(b.n - 1) * b.n + i]);
            }
        }
    }
}
#[test]
fn earth_snapshot_keeps_geography_separate_from_render_quality() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/environment/packs");
    let mut snapshot = EnvironmentSnapshot::legacy();
    snapshot.preset.legacy = false;
    snapshot.data =
        pack::TerrainData::load(&root, &["sweden-west".into()], 384 * 1024 * 1024).unwrap();
    let p = geo_to_local(12.85, 58.41, 0., snapshot.preset.planet.radius);
    let low = snapshot.sample_surface(p);
    snapshot.graphics = GraphicsSettings::high();
    let high = snapshot.sample_surface(p);
    assert_eq!(low.ground_height, high.ground_height);
    assert_eq!(low.normal, high.normal);
    assert_eq!(low.land_cover, high.land_cover);
    let lake = snapshot.sample_surface(geo_to_local(13., 58.7, 0., snapshot.preset.planet.radius));
    assert!(lake.water_surface.is_some());
    assert!(lake.water_surface.unwrap() > 30.);
    assert!(lake.ground_height < lake.water_surface.unwrap());
}

#[test]
fn lifted_mountain_vertices_sample_the_same_surface_as_collision_queries() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/environment/packs");
    let mut world = EnvironmentSnapshot::legacy();
    world.preset.legacy = false;
    world.data =
        pack::TerrainData::load(&root, &["norway-mountains".into()], 384 * 1024 * 1024).unwrap();
    for (lon, lat) in [(7.7, 61.25), (7.4, 61.7), (8.3, 62.1)] {
        let p = geo_to_local(lon, lat, 0., world.preset.planet.radius);
        let h = world.height_xz(p.x, p.z);
        let lifted = point(p.x, p.z, h, world.preset.planet.radius);
        assert!((world.raw(lifted).0 - h).abs() < 0.01, "{lon} {lat}");
    }
}

#[test]
fn airport_grade_includes_the_visible_parking_apron() {
    let mut world = EnvironmentSnapshot::legacy();
    world.preset.legacy = false;
    for (x, z) in [(0., 1050.), (150., 700.), (250., 620.), (280., 400.)] {
        let sample = world.sample_surface(point(
            x,
            z,
            crate::physics::RUNWAY_HEIGHT,
            world.preset.planet.radius,
        ));
        assert_eq!(sample.ground_height, crate::physics::RUNWAY_HEIGHT);
        assert!(sample.water_surface.is_none());
    }
}
