//! Bounded wall-clock scheduling. Flight and environment consume identical ticks;
//! render frequency changes the number of ticks, never their simulated duration.
use bevy::prelude::*;

pub const MAX_FRAME_WALL_SECONDS: f64 = 0.1;
pub const MAX_STEPS_PER_FRAME: usize = 256;
pub const MAX_PHYSICS_STEP: f64 = 1. / 120.;
pub const GROUND_PRIORITY_WALL_SECONDS: f64 = 0.25;
pub const GROUND_PRIORITY_STEPS: usize = 2560;

#[derive(Resource, Default)]
pub struct SimClock {
    ground_priority: bool,
    accumulator: f64,
    last_elapsed: Option<f64>,
    generation: Option<u64>,
    was_paused: bool,
    dropped_wall_seconds: f64,
    dropped_simulated_seconds: f64,
}
impl SimClock {
    pub fn prioritize_ground(&mut self, active: bool) { self.ground_priority = active; }
    /// Discard paused time, scene-cut residue and long host stalls. In particular,
    /// resuming a suspended process cannot accumulate seconds of physics debt.
    pub fn wall_duration(
        &mut self,
        wall: f64,
        elapsed: f64,
        paused: bool,
        generation: Option<u64>,
    ) -> f64 {
        let reset = self.last_elapsed.is_some_and(|old| elapsed < old)
            || self
                .generation
                .zip(generation)
                .is_some_and(|(old, new)| old != new);
        let resuming = self.was_paused;
        self.was_paused = paused;
        self.last_elapsed = Some(elapsed);
        self.generation = generation;
        if paused || resuming || reset || !wall.is_finite() || wall <= 0. {
            if paused || resuming || reset {
                self.accumulator = 0.;
            }
            return 0.;
        }
        let bounded = wall.min(if self.ground_priority {GROUND_PRIORITY_WALL_SECONDS} else {MAX_FRAME_WALL_SECONDS});
        self.dropped_wall_seconds += wall - bounded;
        bounded
    }
    /// Use the configured interval as a maximum tick size, additionally bounded
    /// by the existing 120 Hz physics limit. Retain fractional ticks across frames.
    pub fn steps(&mut self, wall: f64, rate: f64, configured: f64) -> (usize, f64) {
        let h = if configured.is_finite() {
            configured.clamp(0.001, MAX_PHYSICS_STEP)
        } else {
            MAX_PHYSICS_STEP
        };
        if !wall.is_finite() || !rate.is_finite() || wall < 0. || rate < 0. {
            return (0, h);
        }
        self.accumulator += wall * rate;
        let max_steps = if self.ground_priority {GROUND_PRIORITY_STEPS} else {MAX_STEPS_PER_FRAME};
        let limit = h * max_steps as f64;
        if self.accumulator > limit {
            self.dropped_simulated_seconds += self.accumulator - limit;
            self.accumulator = limit;
        }
        let count = ((self.accumulator + 1e-10) / h).floor() as usize;
        let count = count.min(max_steps);
        self.accumulator = (self.accumulator - count as f64 * h).max(0.);
        (count, h)
    }
    pub fn discard_remainder(&mut self) {
        self.accumulator = 0.;
    }
    pub fn report(&self) -> serde_json::Value {
        serde_json::json!({"mode":"bounded_wall_clock", "max_frame_wall_s":MAX_FRAME_WALL_SECONDS,
            "ground_priority":self.ground_priority,"active_max_frame_wall_s":if self.ground_priority {GROUND_PRIORITY_WALL_SECONDS}else{MAX_FRAME_WALL_SECONDS},
            "active_max_steps_per_frame":if self.ground_priority {GROUND_PRIORITY_STEPS}else{MAX_STEPS_PER_FRAME},
            "max_physics_step_s":MAX_PHYSICS_STEP,"max_steps_per_frame":MAX_STEPS_PER_FRAME,
            "remainder_s":self.accumulator,"whole_run_dropped_wall_s":self.dropped_wall_seconds,
            "whole_run_dropped_simulated_s":self.dropped_simulated_seconds})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ground_priority_tracks_ten_x_at_low_display_rates_without_enlarging_ticks() {
        for fps in [5,10,15,30,60] {
            for interval in [0.001,1./240.,1./60.] {
                let mut c=SimClock::default();c.prioritize_ground(true);
                let mut elapsed=0.;
                for _ in 0..fps*2 {
                    let wall=c.wall_duration(1./fps as f64,elapsed,false,Some(0));
                    let (n,h)=c.steps(wall,10.,interval);
                    assert!(n<=GROUND_PRIORITY_STEPS&&h<=MAX_PHYSICS_STEP);
                    elapsed+=n as f64*h;
                }
                assert!((elapsed-20.).abs()<0.0011,"{fps} Hz, {interval}: {elapsed}");
                assert_eq!(c.dropped_wall_seconds,0.);assert_eq!(c.dropped_simulated_seconds,0.);
            }
        }
    }
    #[test]
    fn priority_stalls_remain_bounded_and_normal_mode_restores_limits() {
        let mut c=SimClock::default();c.prioritize_ground(true);
        let wall=c.wall_duration(10.,0.,false,None);assert_eq!(wall,0.25);
        let (n,h)=c.steps(wall,10.,0.001);assert_eq!(n,2500);assert_eq!(h,0.001);
        assert_eq!(c.steps(0.,10.,0.001).0,0);
        c.prioritize_ground(false);
        assert_eq!(c.wall_duration(10.,2.5,false,None),MAX_FRAME_WALL_SECONDS);
        assert_eq!(c.steps(0.1,10.,0.001).0,MAX_STEPS_PER_FRAME);
    }
    #[test]
    fn rates_and_tick_settings_are_independent_of_render_frequency() {
        for fps in [24, 30, 40, 60, 120, 144, 240] {
            for rate in [0.1, 0.25, 0.5, 0.75, 1., 2., 4.] {
                for interval in [0.001, 1. / 240., 1. / 60., 0.05] {
                    let mut clock = SimClock::default();
                    let mut simulated = 0.;
                    for _ in 0..fps * 5 {
                        let wall = clock.wall_duration(1. / fps as f64, simulated, false, Some(0));
                        let (n, h) = clock.steps(wall, rate, interval);
                        assert!(n <= MAX_STEPS_PER_FRAME && h <= MAX_PHYSICS_STEP);
                        simulated += n as f64 * h;
                    }
                    assert!(
                        (simulated - 5. * rate).abs() < 0.0011,
                        "fps {fps}, rate {rate}, interval {interval}, simulated {simulated}"
                    );
                }
            }
        }
    }
    #[test]
    fn pause_resume_and_reset_discard_fractional_debt() {
        let mut c = SimClock::default();
        let wall = c.wall_duration(0.004, 0., false, Some(0));
        assert_eq!(c.steps(wall, 1., 0.02).0, 0);
        assert_eq!(c.wall_duration(10., 0., true, Some(0)), 0.);
        assert_eq!(c.wall_duration(10., 0., false, Some(0)), 0.);
        assert_eq!(c.steps(0.004, 1., 0.02).0, 0);
        assert_eq!(c.wall_duration(0.02, 0., false, Some(1)), 0.);
        assert_eq!(c.steps(0.004, 1., 0.02).0, 0);
        c.wall_duration(0.001, 2., false, Some(1));
        assert_eq!(c.wall_duration(0.02, 0., false, Some(1)), 0.);
        assert_eq!(c.steps(0.004, 1., 0.02).0, 0);
    }
    #[test]
    fn stalls_and_small_tick_catchup_have_finite_work_limits() {
        let mut c = SimClock::default();
        let wall = c.wall_duration(60., 0., false, None);
        assert_eq!(wall, MAX_FRAME_WALL_SECONDS);
        let (n, h) = c.steps(wall, 4., 0.001);
        assert_eq!(n, MAX_STEPS_PER_FRAME);
        assert!((n as f64 * h - 0.256).abs() < 1e-12);
        assert!((c.dropped_wall_seconds - 59.9).abs() < 1e-10);
        assert!((c.dropped_simulated_seconds - 0.144).abs() < 1e-10);
        assert_eq!(c.steps(0., 1., 0.001).0, 0);
    }
}
