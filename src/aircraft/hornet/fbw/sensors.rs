//! Four independent simulated sensor lanes. Voters use consistency and range
//! checks, never the plant's true value to decide which disagreeing lane wins.
use super::*;
#[derive(Clone, Copy, Debug)]
pub struct Fault {
    pub rates_valid: bool,
    pub air_valid: bool,
    pub load_valid: bool,
    pub frozen: bool,
    pub rate_bias: DVec3,
    pub alpha_bias: f64,
    pub beta_bias: f64,
    pub load_bias: f64,
    pub speed_scale: f64,
}
impl Default for Fault {
    fn default() -> Self {
        Self {
            rates_valid: true,
            air_valid: true,
            load_valid: true,
            frozen: false,
            rate_bias: DVec3::ZERO,
            alpha_bias: 0.,
            beta_bias: 0.,
            load_bias: 0.,
            speed_scale: 1.,
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Sample {
    pub rates: DVec3,
    pub alpha: f64,
    pub beta: f64,
    pub load: f64,
    pub speed: f64,
    pub mach: f64,
}
#[derive(Clone, Debug)]
pub struct Sensors {
    pub faults: [Fault; 4],
    pub samples: [Sample; 4],
    pub voted: Sample,
    pub counts: [usize; 3],
    pub rejected: [bool; 4],
    initialized: bool,
}
impl Default for Sensors {
    fn default() -> Self {
        Self {
            faults: [Fault::default(); 4],
            samples: [Sample::default(); 4],
            voted: Sample::default(),
            counts: [0; 3],
            rejected: [false; 4],
            initialized: false,
        }
    }
}
/// Unique largest mutually consistent group. A 2-vs-2 split returns no quorum.
pub(super) fn cohort(valid: [bool; 4], distance: impl Fn(usize, usize) -> bool) -> u8 {
    let (mut best, mut count, mut tie) = (0, 0, false);
    for mask in 1u8..16 {
        let n = mask.count_ones();
        if n < 2 {
            continue;
        }
        let mut okay = true;
        for i in 0..4 {
            if mask & (1 << i) == 0 {
                continue;
            }
            if !valid[i] {
                okay = false;
                break;
            }
            for j in i + 1..4 {
                if mask & (1 << j) != 0 && !distance(i, j) {
                    okay = false;
                }
            }
        }
        if okay && n > count {
            best = mask;
            count = n;
            tie = false;
        } else if okay && n == count && mask != best {
            tie = true;
        }
    }
    if tie {
        0
    } else {
        best
    }
}
fn average(mask: u8, value: impl Fn(usize) -> f64) -> f64 {
    let mut sum = 0.;
    for i in 0..4 {
        if mask & (1 << i) != 0 {
            sum += value(i);
        }
    }
    sum / (mask.count_ones().max(1) as f64)
}
impl Sensors {
    pub fn sample(&mut self, f: &Flight, dt: f64) {
        let truth = Sample {
            rates: f.rates,
            alpha: f.alpha,
            beta: f.beta,
            load: f.g_load,
            speed: f.airspeed,
            mach: f.mach,
        };
        for i in 0..4 {
            let fault = self.faults[i];
            if !fault.frozen || !self.initialized {
                self.samples[i] = Sample {
                    rates: truth.rates + fault.rate_bias,
                    alpha: truth.alpha + fault.alpha_bias,
                    beta: truth.beta + fault.beta_bias,
                    load: truth.load + fault.load_bias,
                    speed: truth.speed * fault.speed_scale,
                    mach: truth.mach * fault.speed_scale,
                };
            }
        }
        let s = &self.samples;
        // Independent threshold estimates connect existing avionics damage to
        // sensor-lane availability. No random or truth-based failure detector.
        let power = super::powered(f);
        let health = f.damage.avionics;
        let rate_alive = [0.03, 0.18, 0.38, 0.68].map(|limit| power && health > limit);
        let air_alive = [0.07, 0.22, 0.42, 0.72].map(|limit| power && health > limit);
        let load_alive = [0.05, 0.20, 0.40, 0.70].map(|limit| power && health > limit);

        let rate = cohort(
            std::array::from_fn(|i| {
                self.faults[i].rates_valid
                    && rate_alive[i]
                    && s[i].rates.is_finite()
                    && s[i].rates.abs().max_element() < 10.
            }),
            |i, j| (s[i].rates - s[j].rates).abs().max_element() < 0.15,
        );
        let air = cohort(
            std::array::from_fn(|i| {
                self.faults[i].air_valid
                    && air_alive[i]
                    && s[i].alpha.is_finite()
                    && s[i].alpha.abs() <= std::f64::consts::PI
                    && s[i].beta.is_finite()
                    && s[i].beta.abs() < 1.6
                    && s[i].speed.is_finite()
                    && (0. ..1200.).contains(&s[i].speed)
                    && s[i].mach.is_finite()
            }),
            |i, j| {
                (s[i].alpha - s[j].alpha).abs() < 0.08
                    && (s[i].beta - s[j].beta).abs() < 0.08
                    && (s[i].speed - s[j].speed).abs() < 10. + s[i].speed.abs() * 0.12
            },
        );
        let load = cohort(
            std::array::from_fn(|i| {
                self.faults[i].load_valid
                    && load_alive[i]
                    && s[i].load.is_finite()
                    && (-30. ..35.).contains(&s[i].load)
            }),
            |i, j| (s[i].load - s[j].load).abs() < 0.6,
        );
        self.counts = [
            rate.count_ones() as usize,
            air.count_ones() as usize,
            load.count_ones() as usize,
        ];
        self.rejected = std::array::from_fn(|i| {
            rate & (1 << i) == 0 || air & (1 << i) == 0 || load & (1 << i) == 0
        });
        let filter = if self.initialized {
            1. - (-dt / 0.035).exp()
        } else {
            1.
        };
        if rate != 0 {
            let rates = DVec3::new(
                average(rate, |i| s[i].rates.x),
                average(rate, |i| s[i].rates.y),
                average(rate, |i| s[i].rates.z),
            );
            self.voted.rates += (rates - self.voted.rates) * filter;
        }
        if air != 0 {
            self.voted.alpha += (average(air, |i| s[i].alpha) - self.voted.alpha) * filter;
            self.voted.beta += (average(air, |i| s[i].beta) - self.voted.beta) * filter;
            self.voted.speed += (average(air, |i| s[i].speed) - self.voted.speed) * filter;
            self.voted.mach += (average(air, |i| s[i].mach) - self.voted.mach) * filter;
        }
        if load != 0 {
            self.voted.load += (average(load, |i| s[i].load) - self.voted.load) * filter;
        }
        self.initialized = true;
    }
}
