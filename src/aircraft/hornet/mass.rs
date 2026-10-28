use super::*;
fn points(f: &Flight) -> [(f64, DVec3); 21] {
    let mut p = [(0., DVec3::ZERO); 21];
    p[..3].copy_from_slice(&[
        (config::EMPTY - 2100., DVec3::ZERO),
        (config::CREW, DVec3::new(0., 0.5, -4.)),
        (config::PYLONS, DVec3::new(0., -0.5, 0.)),
    ]);
    let mut index = 3;
    for side in 0..2 {
        let sign = if side == 0 { -1. } else { 1. };
        p[index] = (
            if f.damage.wing_broken[side] { 0. } else { 750. },
            DVec3::new(sign * 3.8, 0., 0.6),
        );
        index += 1;
        for (j, at) in [
            (side * 4, DVec3::new(sign * 2.6, 0., 6.)),
            (side * 4 + 3, DVec3::new(sign * 1.7, 1.7, 4.8)),
        ] {
            p[index] = (
                if f.damage.surface_state[j] == crate::combat::damage::SurfaceState::Detached {
                    0.
                } else {
                    150.
                },
                at,
            );
            index += 1;
        }
    }
    for (fraction, at) in [
        (0.35, DVec3::new(0., 0., -1.4)),
        (0.25, DVec3::new(0., 0., 1.8)),
        (0.2, DVec3::new(-2., 0., 0.)),
        (0.2, DVec3::new(2., 0., 0.)),
    ] {
        p[index] = (f.fuel * fraction, at);
        index += 1;
    }
    for (i, on) in f.systems.stores.iter().enumerate().take(8) {
        if *on {
            p[index] = (
                if i < 2 { 85. } else { 152. },
                crate::aircraft::station_for(f, i),
            );
            index += 1;
        }
    }
    p
}
pub fn total(f: &Flight) -> f64 {
    points(f).iter().map(|p| p.0).sum()
}
pub fn properties(f: &Flight) -> (DVec3, DMat3) {
    let points = points(f);
    let mass = total(f);
    let cg = points.iter().map(|(m, p)| *p * *m).sum::<DVec3>() / mass;
    let mut inertia = DMat3::from_diagonal(DVec3::new(140000., 175000., 20000.));
    for (m, p) in points {
        let r = p - cg;
        inertia += (DMat3::IDENTITY * r.length_squared()
            - DMat3::from_cols(r * r.x, r * r.y, r * r.z))
            * m;
    }
    (cg, inertia)
}
