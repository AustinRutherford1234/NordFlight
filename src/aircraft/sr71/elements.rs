//! Fixed spatial discretization; normal/drag/side shares each sum to one.
use super::{airframe_config::SPATIAL, config::*};
use bevy::math::DVec3;
use std::sync::OnceLock;
pub const COUNT: usize = 30;
pub const SECTIONS: usize = 6;
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub enum Kind {
    Wing,
    Chine,
    Body,
    Nacelle,
    Fin,
}
#[derive(Clone, Copy, Debug)]
pub struct Element {
    pub kind: Kind,
    pub at: DVec3,
    pub chord: f64,
    pub normal_share: f64,
    pub drag_share: f64,
    pub side_share: f64,
    pub assembly: Option<usize>,
    pub span_fraction: f64,
}
pub fn geometry() -> &'static [Element; COUNT] {
    static G: OnceLock<[Element; COUNT]> = OnceLock::new();
    G.get_or_init(|| {
        let empty = Element {
            kind: Kind::Body,
            at: DVec3::ZERO,
            chord: CHORD,
            normal_share: 0.,
            drag_share: 0.,
            side_share: 0.,
            assembly: None,
            span_fraction: 0.,
        };
        let mut g = [empty; COUNT];
        let mut n = 0;
        for (side_index, side) in [-1., 1.].into_iter().enumerate() {
            for j in 0..6 {
                let fraction = (j as f64 + 0.5) / 6.;
                let weight = [0.23, 0.22, 0.20, 0.16, 0.12, 0.07][j];
                g[n] = Element {
                    kind: Kind::Wing,
                    at: DVec3::new(side * (1. + 7.2 * fraction), 0.15, 2.8 + 7.8 * fraction),
                    chord: 13. - 10. * fraction,
                    normal_share: SPATIAL.wing_lift_share * weight * 0.5,
                    drag_share: SPATIAL.wing_drag_share * weight * 0.5,
                    side_share: 0.,
                    assembly: Some(side_index),
                    span_fraction: fraction,
                };
                n += 1;
            }
        }
        for side in [-1., 1.] {
            for j in 0..3 {
                g[n] = Element {
                    kind: Kind::Chine,
                    at: DVec3::new(side * (0.8 + 0.25 * j as f64), 0.1, -10. + 4. * j as f64),
                    chord: 5.,
                    normal_share: SPATIAL.chine_lift_share / 6.,
                    drag_share: SPATIAL.chine_drag_share / 6.,
                    ..empty
                };
                n += 1;
            }
        }
        for j in 0..4 {
            g[n] = Element {
                kind: Kind::Body,
                at: DVec3::new(0., 0., -11. + 7. * j as f64),
                chord: 7.,
                normal_share: SPATIAL.body_lift_share / 4.,
                drag_share: SPATIAL.body_drag_share / 4.,
                side_share: SPATIAL.body_side_share / 4.,
                ..empty
            };
            n += 1;
        }
        for side in [-1., 1.] {
            for j in 0..2 {
                g[n] = Element {
                    kind: Kind::Nacelle,
                    at: DVec3::new(side * 4.3, 0.2, 2. + 8. * j as f64),
                    chord: 8.,
                    normal_share: SPATIAL.nacelle_lift_share / 4.,
                    drag_share: SPATIAL.nacelle_drag_share / 4.,
                    side_share: SPATIAL.nacelle_side_share / 4.,
                    ..empty
                };
                n += 1;
            }
        }
        for (side_index, side) in [-1., 1.].into_iter().enumerate() {
            for j in 0..2 {
                let f = (j as f64 + 0.5) / 2.;
                g[n] = Element {
                    kind: Kind::Fin,
                    at: DVec3::new(side * (4. - 0.55 * f), 1.28 + 2.4 * f, 10.4 + f),
                    chord: 3.8,
                    normal_share: 0.,
                    drag_share: 0.,
                    side_share: SPATIAL.fin_side_share / 4.,
                    assembly: Some(2 + side_index),
                    span_fraction: f,
                    ..empty
                };
                n += 1;
            }
        }
        assert_eq!(n, COUNT);
        g
    })
}
/// Estimated elastic-axis roots aligned to the imported aircraft's pivots.
pub fn root(a: usize) -> DVec3 {
    if a < 2 {
        DVec3::new(if a == 0 { -1. } else { 1. }, 0.15, 3.)
    } else {
        DVec3::new(if a == 2 { -4. } else { 4. }, 1.28, 10.4)
    }
}
pub fn span(a: usize) -> DVec3 {
    if a < 2 {
        DVec3::X * if a == 0 { -1. } else { 1. }
    } else {
        DVec3::new(if a == 2 { 0.22 } else { -0.22 }, 1., 0.05).normalize()
    }
}
pub fn normal(a: usize) -> DVec3 {
    if a < 2 {
        DVec3::Y
    } else {
        DVec3::new(1., if a == 2 { -0.22 } else { 0.22 }, 0.).normalize()
    }
}
pub fn length(a: usize) -> f64 {
    if a < 2 { 7.2 } else { 2.4 }
}

pub fn register() -> serde_json::Value {
    serde_json::Value::Array(geometry().iter().map(|e|serde_json::json!({"kind":e.kind,"at_m":e.at.to_array(),"chord_m":e.chord,"normal_share":e.normal_share,"drag_share":e.drag_share,"side_share":e.side_share,"assembly":e.assembly.map_or(-1,|a|a as i32),"span_fraction":e.span_fraction})).collect())
}
