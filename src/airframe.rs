//! Compound convex collision skin in the same metre/body frame as the visual model.
//! Convex pieces preserve the empty space beside the nose and between lifting surfaces.
use crate::{
    control_surfaces::{self, SurfaceKind},
    physics::{Flight, Vehicle},
};
use bevy::math::{DQuat, DVec3};
use std::sync::OnceLock;

pub fn distance_to_skin(point: DVec3) -> f64 {
    distance_to_aircraft(point,crate::aircraft::Aircraft::Gripen)
}
pub fn distance_to_aircraft(point:DVec3,aircraft:crate::aircraft::Aircraft)->f64 {
    aircraft_hulls(aircraft)
        .iter()
        .map(|h| {
            // Dykstra projection onto each convex skin's bounding half-spaces.
            let mut p = point;
            let mut correction = vec![DVec3::ZERO; h.normals.len() * 2];
            for _ in 0..64 {
                let old = p;
                for (i, &(n, limit)) in h.planes().iter().enumerate() {
                    let trial = p + correction[i];
                    p = trial - n * (trial.dot(n) - limit).max(0.0);
                    correction[i] = trial - p;
                }
                if p.distance_squared(old) < 1e-14 {
                    break;
                }
            }
            point.distance(p)
        })
        .fold(f64::INFINITY, f64::min)
}

/// Continuous SAT for translating convex skins, at the supplied orientations.
#[cfg(test)]
pub fn aircraft_contact(start: DVec3, end: DVec3, qa: DQuat, qb: DQuat) -> Option<f64> {
    let f = Flight::runway();
    aircraft_contact_state(start, end, qa, qb, &f, &f)
}
pub fn aircraft_contact_state(
    start: DVec3,
    end: DVec3,
    qa: DQuat,
    qb: DQuat,
    fa: &Flight,
    fb: &Flight,
) -> Option<f64> {
    struct Prepared {
        vertices: Vec<DVec3>,
        normals: Vec<DVec3>,
        edges: Vec<DVec3>,
        min: DVec3,
        max: DVec3,
    }
    let prepare = |f: &Flight, q: DQuat| {
        for_flight(f)
            .iter()
            .filter(|h| h.enabled(f))
            .map(|h| {
                let (pose, offset) = h.pose(f);
                let rotation = q * pose;
                let offset = q * offset;
                let vertices: Vec<_> = h.vertices.iter().map(|p| rotation * *p + offset).collect();
                let min = vertices
                    .iter()
                    .copied()
                    .fold(DVec3::splat(f64::INFINITY), DVec3::min);
                let max = vertices
                    .iter()
                    .copied()
                    .fold(DVec3::splat(f64::NEG_INFINITY), DVec3::max);
                Prepared {
                    vertices,
                    normals: h.normals.iter().map(|n| rotation * *n).collect(),
                    edges: h.edges.iter().map(|e| rotation * *e).collect(),
                    min,
                    max,
                }
            })
            .collect::<Vec<_>>()
    };
    let left = prepare(fa, qa);
    let right = prepare(fb, qb);
    let mut earliest: Option<f64> = None;
    for a in &left {
        for b in &right {
            if (b.max + start.max(end)).cmplt(a.min).any()
                || (b.min + start.min(end)).cmpgt(a.max).any()
            {
                continue;
            }
            let cross = a.edges.iter().flat_map(|ea| {
                b.edges.iter().filter_map(move |eb| {
                    let axis = ea.cross(*eb);
                    (axis.length_squared() > 1e-12).then(|| axis.normalize())
                })
            });
            let axes = a
                .normals
                .iter()
                .chain(b.normals.iter())
                .copied()
                .chain(cross);
            let mut enter: f64 = 0.0;
            let mut leave: f64 = 1.0;
            for axis in axes {
                let amin = a
                    .vertices
                    .iter()
                    .map(|p| p.dot(axis))
                    .fold(f64::INFINITY, f64::min);
                let amax = a
                    .vertices
                    .iter()
                    .map(|p| p.dot(axis))
                    .fold(f64::NEG_INFINITY, f64::max);
                let bmin = b
                    .vertices
                    .iter()
                    .map(|p| p.dot(axis))
                    .fold(f64::INFINITY, f64::min);
                let bmax = b
                    .vertices
                    .iter()
                    .map(|p| p.dot(axis))
                    .fold(f64::NEG_INFINITY, f64::max);
                let x = start.dot(axis);
                let speed = (end - start).dot(axis);
                if speed.abs() < 1e-10 {
                    if x + bmax < amin || x + bmin > amax {
                        enter = 2.0;
                        break;
                    }
                } else {
                    let t0 = (amin - bmax - x) / speed;
                    let t1 = (amax - bmin - x) / speed;
                    enter = enter.max(t0.min(t1));
                    leave = leave.min(t0.max(t1));
                    if enter > leave {
                        break;
                    }
                }
            }
            if enter <= leave && enter <= 1.0 && leave >= 0.0 {
                earliest = Some(earliest.map_or(enter, |x| x.min(enter)));
            }
        }
    }
    earliest
}

pub struct Hull {
    pub vertices: Vec<DVec3>,
    normals: Vec<DVec3>,
    edges: Vec<DVec3>,
    hinge: Option<(SurfaceKind, DVec3, DVec3)>,
    fleet_surface: Option<usize>,
    // A rigid local approximation to a small portion of a flexible surface.
    flex_surface: Option<usize>,
    planes: OnceLock<Vec<(DVec3, f64)>>,
    bounds: OnceLock<(DVec3, DVec3)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn broken_f22_wing_removes_only_its_collision_pieces() {
        let mut f = Flight::runway();
        crate::aircraft::select(&mut f, crate::aircraft::Aircraft::F22);
        f.damage.wing_broken[0] = true;
        for i in [1, 2, 9] { f.damage.surface_state[i] = crate::combat::damage::SurfaceState::Detached; }
        let hulls = aircraft_hulls(f.aircraft);
        assert!(hulls.iter().any(|h| h.flex_surface == Some(crate::structural::LEFT_WING) && !h.enabled(&f)));
        assert!(hulls.iter().filter(|h| h.flex_surface == Some(crate::structural::RIGHT_WING)).all(|h| h.enabled(&f)));
        assert!(hulls.iter().find(|h| h.fleet_surface == Some(0)).unwrap().enabled(&f));
    }
    #[test]
    fn raptor_collision_segments_follow_elastic_state() {
        use crate::{aircraft::Aircraft, structural::RIGHT_WING};
        let mut f = Flight::runway();
        crate::aircraft::select(&mut f, Aircraft::F22);
        let pieces: Vec<_> = aircraft_hulls(Aircraft::F22).iter()
            .filter(|h| h.flex_surface == Some(RIGHT_WING) && h.fleet_surface.is_none())
            .collect();
        assert!(pieces.len() >= 4);
        let tip = pieces.iter().max_by(|a, b| {
            let x = |h: &&Hull| h.vertices.iter().map(|p| p.x).sum::<f64>() / h.vertices.len() as f64;
            x(a).total_cmp(&x(b))
        }).unwrap();
        let center = tip.vertices.iter().copied().sum::<DVec3>() / tip.vertices.len() as f64;
        f.structure.surfaces[RIGHT_WING].bend = 0.4;
        let (rotation, offset) = tip.pose(&f);
        assert!((rotation * center + offset).y > center.y + 0.1);
        assert!(pieces.iter().all(|h| h.vertices.iter().all(|p| p.is_finite())
            && h.normals.iter().all(|p| p.is_finite())));
        let panel = aircraft_hulls(Aircraft::F22).iter().find(|h| h.fleet_surface == Some(6)).unwrap();
        let center = panel.vertices.iter().copied().sum::<DVec3>() / panel.vertices.len() as f64;
        let (before_q, before_offset) = panel.pose(&f);
        f.structure.panel_modes[6].bend = 0.15;
        let (after_q, after_offset) = panel.pose(&f);
        assert!((after_q * center + after_offset).distance(before_q * center + before_offset) > 0.001);
    }
    #[test]
    fn raptor_collision_covers_the_new_belly_and_engine_housings() {
        use crate::aircraft::{Aircraft, raptor};
        for (z, width, _, cy) in raptor::sections().into_iter().skip(1).take(12) {
            for x in [-0.82_f32, -0.48, 0., 0.48, 0.82] {
                let point = DVec3::new(width * x as f64, cy + raptor::lower_profile(x, z as f32) as f64, z);
                assert!(distance_to_aircraft(point, Aircraft::F22) < 0.025, "missing belly collision at {point:?}");
            }
        }
    }
    fn overlaps(f: &Flight, point: DVec3) -> bool {
        hulls(f.vehicle).iter().any(|h| {
            let (q, offset) = h.pose(f);
            h.sweep_box(
                offset,
                DVec3::ZERO,
                q,
                DVec3::ZERO,
                point - DVec3::splat(0.03),
                point + DVec3::splat(0.03),
                0.0,
            )
            .is_some()
        })
    }
    #[test]
    fn skin_covers_belly_wing_roots_canards_fin_and_nozzle() {
        let f = Flight::runway();
        for p in [
            DVec3::new(0.0, -0.63, 0.0),
            DVec3::new(2.0, -0.15, 1.0),
            DVec3::new(1.35, 0.32, -2.7),
            DVec3::new(0.0, 2.7, 4.3),
            DVec3::new(0.0, 0.4, 6.0),
        ] {
            assert!(overlaps(&f, p), "missing skin at {p:?}");
        }
        // A single giant box would incorrectly occupy these empty spaces.
        assert!(!overlaps(&f, DVec3::new(3.0, 0.0, -5.0)));
        assert!(!overlaps(&f, DVec3::new(3.0, 2.0, 1.0)));
    }
    #[test]
    fn spacecraft_uses_its_own_skin_including_solar_arrays() {
        let f = Flight::spacecraft();
        assert!(overlaps(&f, DVec3::new(6.0, 0.0, 1.0)));
        assert!(overlaps(&f, DVec3::new(0.0, 1.1, 0.0)));
        assert!(!overlaps(&f, DVec3::new(4.0, 0.0, -2.0)));
    }
    #[test]
    fn prepared_sweeps_preserve_symmetry_and_contact_time_for_failed_surfaces() {
        use crate::combat::damage::SurfaceState;
        let a = Flight::runway();
        let mut b = Flight::runway();
        b.damage.surface_state[0] = SurfaceState::Jammed(0.3);
        b.damage.surface_state[6] = SurfaceState::Detached;
        let mut contacts = 0;
        let mut misses = 0;
        for i in 0..48 {
            let angle = i as f64 * 2.399963229728653;
            let qa = DQuat::from_euler(bevy::math::EulerRot::XYZ, angle * 0.2, angle, 0.3);
            let qb = DQuat::from_euler(bevy::math::EulerRot::XYZ, -0.4, angle * 0.7, angle);
            let start = DVec3::new(angle.cos() * 24.0, angle.sin() * 24.0, -40.0);
            let end = DVec3::new(-start.x, -start.y, 40.0);
            let offset = DVec3::X * (i % 3) as f64 * 18.0;
            let start = start + offset;
            let end = end + offset;
            let forward = aircraft_contact_state(start, end, qa, qb, &a, &b);
            let reversed = aircraft_contact_state(-start, -end, qb, qa, &b, &a);
            assert_eq!(forward.is_some(), reversed.is_some());
            if let (Some(t), Some(other)) = (forward, reversed) {
                contacts += 1;
                assert!((t - other).abs() < 1e-10, "asymmetric contact: {t}/{other}");
                // Truncating the same path must preserve its first contact location.
                let stop = (t + 0.01).min(1.0);
                let shortened =
                    aircraft_contact_state(start, start.lerp(end, stop), qa, qb, &a, &b)
                        .expect("truncated sweep lost contact");
                assert!((shortened * stop - t).abs() < 1e-10);
            } else {
                misses += 1;
            }
        }
        assert!(contacts > 0 && misses > 0);
    }
    #[test]
    fn thin_surface_sweep_cannot_tunnel_through_a_small_obstacle() {
        let f = Flight::runway();
        let point = DVec3::new(2.0, -0.15, 1.0);
        assert!(hulls(f.vehicle).iter().any(|h| {
            h.sweep_box(
                -DVec3::Z * 100.0,
                DVec3::Z * 200.0,
                DQuat::IDENTITY,
                DVec3::ZERO,
                point - DVec3::splat(0.02),
                point + DVec3::splat(0.02),
                0.0,
            )
            .is_some()
        }));
    }
}
impl Hull {
    /// Immutable local bounds support conservative ground rejection without
    /// weakening any of the detailed vertex or swept-contact tests.
    pub fn bounds(&self) -> (DVec3, DVec3) {
        *self.bounds.get_or_init(|| {
            self.vertices.iter().copied().fold(
                (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY)),
                |(min, max), p| (min.min(p), max.max(p)),
            )
        })
    }
    fn planes(&self) -> &[(DVec3, f64)] {
        self.planes.get_or_init(|| {
            self.normals
                .iter()
                .flat_map(|n| [*n, -*n])
                .map(|n| {
                    (
                        n,
                        self.vertices
                            .iter()
                            .map(|v| v.dot(n))
                            .fold(f64::NEG_INFINITY, f64::max),
                    )
                })
                .collect()
        })
    }
    pub fn prism(points: &[DVec3], thickness: DVec3) -> Self {
        let n = points.len();
        let vertices: Vec<_> = points
            .iter()
            .map(|p| *p - thickness)
            .chain(points.iter().map(|p| *p + thickness))
            .collect();
        let mut normals = vec![
            (points[1] - points[0])
                .cross(points[2] - points[0])
                .normalize(),
        ];
        let mut edges = vec![thickness.normalize()];
        for i in 0..n {
            let edge = points[(i + 1) % n] - points[i];
            edges.push(edge.normalize());
            normals.push(edge.cross(thickness).normalize());
        }
        Self {
            vertices,
            normals,
            edges,
            hinge: None,
            fleet_surface: None,
            flex_surface: None,
            planes: OnceLock::new(),
            bounds: OnceLock::new(),
        }
    }
    pub fn box_at(center: DVec3, half: DVec3) -> Self {
        Self::prism(
            &[
                center + DVec3::new(-half.x, -half.y, 0.0),
                center + DVec3::new(half.x, -half.y, 0.0),
                center + DVec3::new(half.x, half.y, 0.0),
                center + DVec3::new(-half.x, half.y, 0.0),
            ],
            DVec3::Z * half.z,
        )
    }
    pub fn section(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64)) -> Self {
        let mut vertices = Vec::new();
        // Circumscribed 16-sided skin avoids slipping through the rendered elliptical body.
        let skin = 1.0 / (std::f64::consts::PI / 16.0).cos();
        for (z, w, h, y) in [a, b] {
            for i in 0..16 {
                let angle = i as f64 * std::f64::consts::TAU / 16.0;
                vertices.push(DVec3::new(
                    w * angle.cos() * skin,
                    y + h * angle.sin() * skin,
                    z,
                ));
            }
        }
        let mut normals = vec![DVec3::Z];
        let mut edges = Vec::new();
        for i in 0..16 {
            let ring = vertices[(i + 1) % 16] - vertices[i];
            let length = vertices[i + 16] - vertices[i];
            normals.push(ring.cross(length).normalize());
            edges.extend([ring.normalize(), length.normalize()]);
        }
        Self {
            vertices,
            normals,
            edges,
            hinge: None,
            fleet_surface: None,
            flex_surface: None,
            planes: OnceLock::new(),
            bounds: OnceLock::new(),
        }
    }
    fn raptor_section(a: (f64, f64, f64, f64), b: (f64, f64, f64, f64), side: f64) -> Self {
        use crate::aircraft::raptor::{lower_profile, upper_profile};
        // Separate convex halves preserve the shallow channel between engine
        // housings instead of filling it with a single enlarged elliptical hull.
        let mut vertices = Vec::new();
        for (z, w, h, y) in [a, b] {
            for x in [0., 0.48, 0.58, 0.88, 1.] {
                vertices.push(DVec3::new(side * w * x, y + h * upper_profile(x as f32) as f64 + 0.012 * (1. - x), z));
            }
            for x in [0.82, 0.66, 0.48, 0.25, 0.] {
                vertices.push(DVec3::new(side * w * x, y + lower_profile(x as f32, z as f32) as f64 - 0.012 * (1. - x), z));
            }
        }
        let n = vertices.len() / 2;
        let mut normals = vec![DVec3::Z];
        let mut edges = Vec::new();
        for i in 0..n {
            let ring = vertices[(i + 1) % n] - vertices[i];
            let length = vertices[i + n] - vertices[i];
            normals.push(ring.cross(length).normalize());
            edges.extend([ring.normalize(), length.normalize()]);
        }
        Self { vertices, normals, edges, hinge: None, fleet_surface: None, flex_surface: None, planes: OnceLock::new(), bounds: OnceLock::new() }
    }
    pub fn pose(&self, f: &Flight) -> (DQuat, DVec3) {
        if let Some(i)=self.fleet_surface {
            let p=f.aircraft.panels()[i];
            let q=crate::aircraft::surface_rotation(f.aircraft,i,control_surfaces::effective_angle(&f.damage,i,crate::aircraft::angles(f)[i]));
            if let Some(surface) = self.flex_surface {
                let config = crate::structural::Config::f22();
                let frame = f.structure.frame_rotation(&config, surface, p.center);
                let center = f.structure.deform_point(&config, surface, p.center);
                // Each panel also has its own local bending and twist. Sample
                // its convex hull at the centroid and compose that local frame
                // with the hinge and the supporting surface frame.
                let sample = self.vertices.iter().copied().sum::<DVec3>() / self.vertices.len() as f64;
                let panel_rotation = f.structure.panel_frame_rotation(&config, i, sample);
                let displaced = f.structure.deform_panel_point(&config, i, sample);
                let rotation = frame * q * panel_rotation;
                let moved = center + frame * q * (displaced - p.center);
                return (rotation, moved - rotation * sample);
            }
            return(q,p.center-q*p.center);
        }
        if let Some(surface) = self.flex_surface {
            let config = crate::structural::Config::f22();
            let center = self.vertices.iter().copied().sum::<DVec3>() / self.vertices.len() as f64;
            let rotation = f.structure.frame_rotation(&config, surface, center);
            let moved = f.structure.deform_point(&config, surface, center);
            return (rotation, moved - rotation * center);
        }
        if let Some((kind, origin, axis)) = self.hinge {
            let q = DQuat::from_axis_angle(
                axis.normalize(),
                control_surfaces::effective_angle(
                    &f.damage,
                    self.surface_index().unwrap(),
                    control_surfaces::deflection(kind, f.surface, f.flaps, f.airbrake),
                ),
            );
            (q, origin - q * origin)
        } else {
            (DQuat::IDENTITY, DVec3::ZERO)
        }
    }
    fn surface_index(&self) -> Option<usize> {
        if self.fleet_surface.is_some(){return self.fleet_surface;}
        self.hinge.map(|(kind, origin, _)| match kind {
            SurfaceKind::Canard => {
                if origin.x < 0.0 {
                    0
                } else {
                    4
                }
            }
            SurfaceKind::Airbrake => {
                if origin.x < 0.0 {
                    3
                } else {
                    7
                }
            }
            _ => control_surfaces::index(kind),
        })
    }
    pub fn enabled(&self, f: &Flight) -> bool {
        if f.aircraft == crate::aircraft::Aircraft::F22 && self.fleet_surface.is_none() {
            if self.flex_surface == Some(crate::structural::LEFT_WING) && f.damage.wing_broken[0] { return false; }
            if self.flex_surface == Some(crate::structural::RIGHT_WING) && f.damage.wing_broken[1] { return false; }
        }
        self.surface_index().is_none_or(|i| {
            f.damage.surface_state[i] != crate::combat::damage::SurfaceState::Detached
        })
    }
    /// Continuous separating-axis test of a convex piece translated past an AABB.
    /// Continuous convex-versus-triangle SAT in a common frame. The margin bounds
    /// rotational motion between attitude samples; translation is swept exactly.
    pub fn sweep_triangle(&self,origin:DVec3,delta:DVec3,rotation:DQuat,tri:[DVec3;3],margin:f64)->Option<(f64,DVec3)> {
        let edges=[tri[1]-tri[0],tri[2]-tri[1],tri[0]-tri[2]];
        let face=edges[0].cross(-edges[2]).normalize_or_zero();
        let mut enter=0f64;let mut leave=1f64;let mut normal=face;
        let mut test=|axis:DVec3| {
            if axis.length_squared()<1e-16 {return true;}
            let axis=axis.normalize();let local=rotation.conjugate()*axis;
            let shift=origin.dot(axis);
            let mut lo=f64::INFINITY;let mut hi=f64::NEG_INFINITY;
            for p in &self.vertices {let v=p.dot(local)+shift;lo=lo.min(v);hi=hi.max(v);}
            let tlo=tri.iter().map(|p|p.dot(axis)).fold(f64::INFINITY,f64::min)-margin;
            let thi=tri.iter().map(|p|p.dot(axis)).fold(f64::NEG_INFINITY,f64::max)+margin;
            let speed=delta.dot(axis);
            if speed.abs()<1e-12 {return lo<=thi&&hi>=tlo;}
            let a=(tlo-hi)/speed;let b=(thi-lo)/speed;
            if a.min(b)>enter {enter=a.min(b);normal=-axis*speed.signum();}
            leave=leave.min(a.max(b));enter<=leave
        };
        if !test(face){return None;}
        for n in &self.normals {if !test(rotation* *n){return None;}}
        for e in &self.edges {for t in edges {if !test((rotation* *e).cross(t)){return None;}}}
        (enter<=1.&&leave>=0.).then_some((enter,if normal.dot(delta)>0.{-normal}else{normal}))
    }
    pub fn sweep_box(
        &self,
        origin: DVec3,
        delta: DVec3,
        rotation: DQuat,
        offset: DVec3,
        min: DVec3,
        max: DVec3,
        margin: f64,
    ) -> Option<(f64, DVec3)> {
        let center = (min + max) * 0.5;
        let half = (max - min) * 0.5 + DVec3::splat(margin);
        let mut enter = 0.0_f64;
        let mut leave = 1.0_f64;
        let mut normal = DVec3::ZERO;
        let mut axis_test = |axis: DVec3| -> bool {
            if axis.length_squared() < 1e-16 {
                return true;
            }
            let axis = axis.normalize();
            let local_axis = rotation.conjugate() * axis;
            let translation = (origin + rotation * offset - center).dot(axis);
            let mut lo = f64::INFINITY;
            let mut hi = f64::NEG_INFINITY;
            for v in &self.vertices {
                let p = v.dot(local_axis) + translation;
                lo = lo.min(p);
                hi = hi.max(p);
            }
            let radius = half.dot(axis.abs());
            let speed = delta.dot(axis);
            if speed.abs() < 1e-12 {
                return lo <= radius && hi >= -radius;
            }
            let mut t0 = (-radius - hi) / speed;
            let mut t1 = (radius - lo) / speed;
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }
            if t0 > enter {
                enter = t0;
                normal = -axis * speed.signum();
            }
            leave = leave.min(t1);
            enter <= leave
        };
        for axis in [DVec3::X, DVec3::Y, DVec3::Z] {
            if !axis_test(axis) {
                return None;
            }
        }
        for n in &self.normals {
            if !axis_test(rotation * *n) {
                return None;
            }
        }
        for edge in &self.edges {
            for axis in [DVec3::X, DVec3::Y, DVec3::Z] {
                if !axis_test((rotation * *edge).cross(axis)) {
                    return None;
                }
            }
        }
        (enter <= 1.0 && leave >= 0.0).then_some((
            enter,
            if normal == DVec3::ZERO {
                -delta.normalize_or_zero()
            } else {
                normal
            },
        ))
    }
}
pub fn for_flight(f:&Flight)-> &'static [Hull]{if f.vehicle==Vehicle::Spacecraft{hulls(f.vehicle)}else{aircraft_hulls(f.aircraft)}}

// Split a convex wing into narrow spanwise hulls. Each hull follows the local
// elastic frame, retaining convex SAT and the existing broad-phase cache.
fn clip_span(points: &[DVec3], side: f64, bound: f64, keep_outboard: bool) -> Vec<DVec3> {
    let distance = |p: DVec3| if keep_outboard { side * p.x - bound } else { bound - side * p.x };
    let mut output = Vec::new();
    let Some(mut previous) = points.last().copied() else { return output; };
    let mut before = distance(previous);
    for &point in points {
        let after = distance(point);
        if (before >= 0.) != (after >= 0.) {
            output.push(previous + (point - previous) * (before / (before - after)));
        }
        if after >= 0. { output.push(point); }
        previous = point;
        before = after;
    }
    output.dedup_by(|a, b| a.distance_squared(*b) < 1e-16);
    if output.len() > 1 && output[0].distance_squared(*output.last().unwrap()) < 1e-16 { output.pop(); }
    output
}

pub fn aircraft_hulls(a:crate::aircraft::Aircraft)-> &'static [Hull]{
    use crate::aircraft::Aircraft;
    static FLEET:OnceLock<[Vec<Hull>;4]>=OnceLock::new();
    if a==Aircraft::Gripen{return hulls(Vehicle::Gripen);}
    let all=FLEET.get_or_init(||[Aircraft::Su27,Aircraft::F35B,Aircraft::F22,Aircraft::Hornet].map(|a|{
        let mut h=Vec::new();
        let sections=crate::aircraft::visual::sections(a);
        for p in sections.windows(2){
            if a==Aircraft::F22 {for side in [-1.,1.] {h.push(Hull::raptor_section(p[0],p[1],side));}}
            else {h.push(Hull::section(p[0],p[1]));}
        }
        let d=a.cabin_offset().as_dvec3();let canopy=[(d.z-4.08,0.06,0.10,0.66),(d.z-3.4,0.49,0.88,0.66),(d.z-2.5,0.55,0.95,0.66),(d.z-1.35,0.06,0.10,0.66)];for p in canopy.windows(2){h.push(Hull::section(p[0],p[1]));}
        for side in [-1.,1.] {
            let wing = if side < 0. { crate::structural::LEFT_WING } else { crate::structural::RIGHT_WING };
            let fin_surface = if side < 0. { crate::structural::LEFT_FIN } else { crate::structural::RIGHT_FIN };
            for piece in crate::aircraft::visual::fixed_wing(a,side) {
                let points = piece.iter().map(|p|p.as_dvec3()).collect::<Vec<_>>();
                if a == Aircraft::F22 {
                    for bounds in [0.0, 2.93, 4.21, 5.49, 7.0].windows(2) {
                        let clipped = clip_span(&clip_span(&points,side as f64,bounds[0],true),side as f64,bounds[1],false);
                        if clipped.len() >= 3 {
                            let area = (clipped[1]-clipped[0]).cross(clipped[2]-clipped[0]).length_squared();
                            if area > 1e-14 {
                                let mut hull = Hull::prism(&clipped,DVec3::Y*0.045);
                                hull.flex_surface = Some(wing);
                                h.push(hull);
                            }
                        }
                    }
                } else { h.push(Hull::prism(&points,DVec3::Y*0.045)); }
            }
            let(fin,thickness)=crate::aircraft::visual::fin(a,side);
            let mut hull = Hull::prism(&fin.map(|p|p.as_dvec3()),thickness.as_dvec3());
            if a == Aircraft::F22 { hull.flex_surface = Some(fin_surface); }
            h.push(hull);
        }
        for(i,p)in a.panels().iter().enumerate().filter(|(_,p)|p.area>0.){
            let (points,thickness)=crate::aircraft::visual::panel_polygon(*p);
            let mut hull=Hull::prism(&points.map(|p|p.as_dvec3()),thickness.as_dvec3());hull.fleet_surface=Some(i);
            if a == Aircraft::F22 {
                hull.flex_surface = match i {
                    0 => Some(crate::structural::LEFT_TAIL), 4 => Some(crate::structural::RIGHT_TAIL),
                    1 | 2 | 9 => Some(crate::structural::LEFT_WING), 5 | 6 | 10 => Some(crate::structural::RIGHT_WING),
                    3 => Some(crate::structural::LEFT_FIN), 7 => Some(crate::structural::RIGHT_FIN), _ => None,
                };
            }
            h.push(hull);
        }
        for p in a.engine_positions().iter().take(a.spec().engines){h.push(Hull::box_at(*p,DVec3::new(0.65,0.65,1.2)));}
        h
    }));
    &all[a as usize-1]
}
pub fn hulls(vehicle: Vehicle) -> &'static [Hull] {
    static JET: OnceLock<Vec<Hull>> = OnceLock::new();
    static CRAFT: OnceLock<Vec<Hull>> = OnceLock::new();
    if vehicle == Vehicle::Spacecraft {
        return CRAFT.get_or_init(|| {
            let mut pieces = Vec::new();
            let sections = [
                (-4.2, 0.02, 0.02, 0.0),
                (-3.85, 0.85, 0.85, 0.0),
                (-3.0, 1.2, 1.2, 0.0),
                (3.0, 1.2, 1.2, 0.0),
                (3.85, 0.85, 0.85, 0.0),
                (4.85, 0.7, 0.7, 0.0),
            ];
            for pair in sections.windows(2) {
                pieces.push(Hull::section(pair[0], pair[1]));
            }
            for side in [-1.0, 1.0] {
                pieces.push(Hull::box_at(
                    DVec3::new(side * 4.0, 0.0, 1.0),
                    DVec3::new(2.5, 0.08, 1.5),
                ));
            }
            pieces.push(Hull::box_at(
                DVec3::new(0.0, 0.7, -0.6),
                DVec3::new(0.55, 0.15, 0.65),
            ));
            for z in [-2.5, 2.5] {
                for p in [
                    DVec3::new(0.0, -1.2, z),
                    DVec3::new(0.0, 1.2, z),
                    DVec3::new(-1.2, 0.0, z),
                    DVec3::new(1.2, 0.0, z),
                ] {
                    pieces.push(Hull::box_at(p, DVec3::new(0.25, 0.25, 0.20)));
                }
            }
            pieces
        });
    }
    JET.get_or_init(|| {
        let sections = [
            (-7.5, 0.015, 0.015, 0.02),
            (-6.8, 0.20, 0.18, 0.035),
            (-5.6, 0.41, 0.36, 0.04),
            (-4.0, 0.58, 0.56, 0.04),
            (-2.8, 0.69, 0.62, 0.02),
            (-1.3, 0.81, 0.64, -0.02),
            (0.8, 0.85, 0.66, -0.04),
            (2.8, 0.76, 0.62, 0.0),
            (4.6, 0.64, 0.56, 0.0),
            (5.5, 0.56, 0.51, 0.0),
            (6.3, 0.57, 0.57, 0.0),
        ];
        let mut pieces = Vec::new();
        for pair in sections.windows(2) {
            pieces.push(Hull::section(pair[0], pair[1]));
        }
        pieces.push(Hull::box_at(
            DVec3::new(0.0, 0.70, -2.8),
            DVec3::new(0.48, 0.36, 1.55),
        ));
        pieces.push(Hull::box_at(
            DVec3::new(0.0, 0.63, 0.35),
            DVec3::new(0.24, 0.08, 1.8),
        ));
        pieces.push(Hull::box_at(
            DVec3::new(0.0, 0.02, -7.8),
            DVec3::new(0.018, 0.018, 0.35),
        ));
        for side in [-1.0, 1.0] {
            pieces.push(Hull::prism(
                &[
                    DVec3::new(side * 0.65, -0.12, -1.55),
                    DVec3::new(side * 4.15, -0.20, 2.2),
                    DVec3::new(side * 3.96, -0.20, 2.75),
                    DVec3::new(side * 0.65, -0.12, 2.32),
                ],
                DVec3::Y * 0.13,
            ));
            pieces.push(Hull::box_at(
                DVec3::new(side * 0.86, -0.2, -1.2),
                DVec3::new(0.275, 0.3, 1.0),
            ));
            pieces.push(Hull::prism(
                &[
                    DVec3::new(side * 0.64, -0.5, -1.65),
                    DVec3::new(side * 1.16, -0.48, -1.65),
                    DVec3::new(side * 1.15, 0.16, -1.65),
                    DVec3::new(side * 0.65, 0.23, -1.65),
                ],
                DVec3::Z * 0.65,
            ));
            pieces.push(Hull::box_at(
                DVec3::new(side * 4.1, -0.08, 2.5),
                DVec3::new(0.065, 0.065, 1.0),
            ));
            let mut canard = Hull::prism(
                &[
                    DVec3::new(side * 0.72, 0.3, -3.28),
                    DVec3::new(side * 2.08, 0.35, -2.5),
                    DVec3::new(side * 1.85, 0.35, -1.84),
                    DVec3::new(side * 0.72, 0.3, -2.23),
                ],
                DVec3::Y * 0.035,
            );
            canard.hinge = Some((
                SurfaceKind::Canard,
                DVec3::new(side * 0.72, 0.3, -2.7),
                DVec3::X,
            ));
            pieces.push(canard);
            for (inner, x0, x1) in [(true, 0.87, 2.28), (false, 2.32, 3.90)] {
                let hinge = |x: f64| 2.32 + (x - 0.65) * 0.13;
                let y = |x: f64| -0.12 - (x - 0.65) * 0.024;
                let origin = DVec3::new(side * x0, y(x0), hinge(x0));
                let end = DVec3::new(side * x1, y(x1), hinge(x1));
                let mut elevon = Hull::prism(
                    &[
                        origin,
                        end,
                        DVec3::new(side * x1, y(x1), 3.25),
                        DVec3::new(side * x0, y(x0), 2.94 + (x0 - 0.87) * 0.1),
                    ],
                    DVec3::Y * 0.035,
                );
                elevon.hinge = Some((
                    SurfaceKind::Elevon {
                        side: side as f32,
                        inner,
                    },
                    origin,
                    (end - origin) * side,
                ));
                pieces.push(elevon);
            }
            let origin = DVec3::new(side * 0.45, 0.46, 3.55);
            let mut brake = Hull::prism(
                &[
                    origin,
                    DVec3::new(side * 0.77, 0.32, 3.55),
                    DVec3::new(side * 0.72, 0.32, 4.45),
                    DVec3::new(side * 0.43, 0.46, 4.45),
                ],
                DVec3::Y * 0.025,
            );
            brake.hinge = Some((SurfaceKind::Airbrake, origin, DVec3::X));
            pieces.push(brake);
        }
        pieces.push(Hull::prism(
            &[
                DVec3::new(0.0, 0.45, 2.65),
                DVec3::new(0.0, 3.0, 4.15),
                DVec3::new(0.0, 2.95, 4.75),
                DVec3::new(0.0, 0.5, 4.95),
            ],
            DVec3::X * 0.055,
        ));
        let bottom = DVec3::new(0.0, 0.52, 4.98);
        let top = DVec3::new(0.0, 2.94, 4.78);
        let mut rudder = Hull::prism(
            &[
                bottom,
                top,
                DVec3::new(0.0, 2.9, 5.42),
                DVec3::new(0.0, 0.52, 5.65),
            ],
            DVec3::X * 0.045,
        );
        rudder.hinge = Some((SurfaceKind::Rudder, bottom, top - bottom));
        pieces.push(rudder);
        pieces
    })
}
