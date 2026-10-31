//! Estimated compartment damage and hydrostatics, not a naval engineering model.
use super::*;
#[derive(Clone, Copy, Debug, Default)]
pub struct Compartment {
    pub damage: f64,
    pub fire: f64,
    pub flood: f64,
    pub breach: f64,
    pub magazine: bool,
    pub exploded: bool,
}
#[derive(Clone, Debug)]
pub struct Ship {
    pub compartments: [Compartment; 8],
    pub radar: f64,
    pub power: f64,
    pub launcher: f64,
    pub propulsion: f64,
    pub hull: f64,
    pub sink: f64,
    pub sink_rate: f64,
    pub list: f64,
    pub trim: f64,
    pub sunk: bool,
    pub standard: u16,
    pub essm: u16,
    pub gun: u32,
    pub next_launch: f64,
    pub next_burst: f64,
    pub gun_yaw: f64,
    pub gun_pitch: f64,
    pub index: usize,
}
impl Ship {
    pub fn new(index: usize) -> Self {
        let mut compartments = [Compartment::default(); 8];
        compartments[1].magazine = true;
        compartments[5].magazine = true;
        Self {
            compartments,
            radar: 1.,
            power: 1.,
            launcher: 1.,
            propulsion: 1.,
            hull: 1.,
            sink: 0.,
            sink_rate: 0.,
            list: 0.,
            trim: 0.,
            sunk: false,
            standard: if index == 0 { 0 } else { 24 },
            essm: if index == 0 { 32 } else { 32 },
            gun: 3100,
            next_launch: 0.,
            next_burst: 0.,
            gun_yaw: 0.,
            gun_pitch: 0.,
            index,
        }
    }
    pub fn deck_usable(&self) -> bool {
        !self.sunk
            && self.hull > 0.35
            && self.sink < 1.5
            && self.list.abs() < 0.035
            && self.trim.abs() < 0.035
            && self.power > 0.25
    }
    pub fn hit(&mut self, p: DVec3, energy: f64) {
        let half = if self.index == 0 { 170. } else { 80. };
        let section = (((p.z + half) / (half * 2.) * 4.).floor() as usize).min(3);
        let side = usize::from(p.x > 0.);
        let i = section * 2 + side;
        let c = &mut self.compartments[i];
        let scale = if self.index == 0 { 0.55 } else { 1. };
        let severity = (energy * scale).clamp(0., 1.5);
        c.damage = (c.damage + severity).min(1.);
        c.fire = (c.fire + severity * 0.6).min(1.);
        if p.y < 5. {
            c.breach = (c.breach + severity * 0.7).min(1.);
        }
        self.hull = (self.hull - severity * 0.15).max(0.);
        if p.y > 15. {
            self.radar = (self.radar - severity * 0.8).max(0.);
        }
        if section == 1 {
            self.launcher = (self.launcher - severity * 0.7).max(0.);
        }
        if section >= 2 {
            self.power = (self.power - severity * 0.3).max(0.);
            self.propulsion = (self.propulsion - severity * 0.4).max(0.);
        }
    }
    pub fn advance(&mut self, dt: f64) {
        let previous = self.compartments;
        let mut port = 0.;
        let mut starboard = 0.;
        let mut fore = 0.;
        let mut aft = 0.;
        for i in 0..8 {
            let c = &mut self.compartments[i];
            let neighbor = previous[i ^ 1].fire * 0.003
                + if i >= 2 {
                    previous[i - 2].fire * 0.003
                } else {
                    0.
                };
            c.fire = (c.fire
                + dt * (neighbor + c.fire * 0.006 - 0.014 * self.power * (1. - c.damage * 0.6)))
                .clamp(0., 1.);
            c.damage = (c.damage + c.fire * 0.0005 * dt).min(1.);
            c.flood = (c.flood
                + dt * (c.breach * 0.012 * (1. + self.sink * 0.1) - 0.002 * self.power))
                .clamp(0., 1.);
            if c.magazine && !c.exploded && c.fire > 0.8 && c.damage > 0.7 {
                c.exploded = true;
                c.damage = 1.;
                c.breach = 1.;
                self.hull = (self.hull - 0.25).max(0.);
                self.launcher *= 0.4;
            }
            if i % 2 == 0 {
                port += c.flood;
            } else {
                starboard += c.flood;
            }
            if i < 4 {
                fore += c.flood;
            } else {
                aft += c.flood;
            }
        }
        let flooding = (port + starboard) / 8.;
        self.hull = (self.hull
            - dt * self.compartments.iter().map(|c| c.fire).sum::<f64>() * 0.00006)
            .max(0.);
        let target_list = ((port - starboard) * 0.06).clamp(-0.6, 0.6);
        let target_trim = ((aft - fore) * 0.035).clamp(-0.3, 0.3);
        self.list += (target_list - self.list) * (1. - (-dt / 8.).exp());
        self.trim += (target_trim - self.trim) * (1. - (-dt / 10.).exp());
        self.sunk |= self.hull <= 0.05 || flooding > 0.65;
        self.sink_rate = if self.sunk {
            0.6
        } else {
            ((flooding * 8. - self.sink) / 20.).max(0.)
        };
        self.sink = (self.sink + self.sink_rate * dt).min(90.);
        if self.sunk {
            self.radar = 0.;
            self.power = 0.;
            self.launcher = 0.;
            self.propulsion = 0.;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flooding_lists_and_sinks() {
        let mut s = Ship::new(1);
        s.compartments[0].breach = 1.;
        for _ in 0..1000 {
            s.advance(0.1);
        }
        assert!(s.list > 0.01 && s.sink > 0.);
        for c in &mut s.compartments {
            c.breach = 1.;
        }
        for _ in 0..2000 {
            s.advance(0.1);
        }
        assert!(s.sunk && s.power == 0.);
    }
    #[test]
    fn magazine_is_one_shot_and_suppression_works() {
        let mut s = Ship::new(1);
        s.compartments[1].fire = 0.9;
        s.compartments[1].damage = 0.9;
        s.advance(0.1);
        assert!(s.compartments[1].exploded);
        let h = s.hull;
        s.advance(0.1);
        assert!(h - s.hull < 0.001);
        let mut clean = Ship::new(0);
        clean.compartments[0].fire = 0.2;
        for _ in 0..300 {
            clean.advance(0.1);
        }
        assert_eq!(clean.compartments[0].fire, 0.);
    }
}
