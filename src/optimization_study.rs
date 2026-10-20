//! Focused, renderer-free workloads. Timings are separate from behavioral checksums.
use crate::{collision, combat::guidance, physics::*};
use bevy::math::DVec3;
use std::{hint::black_box, time::Instant};
pub fn run() {
    let start = Instant::now();
    let mut visible = 0_u64;
    for step in 0..500 {
        let positions: [DVec3; 12] = std::array::from_fn(|i| {
            surface_point(
                100000. + i as f64 * 2300.,
                -60000. + step as f64 * 2. + (i % 4) as f64 * 38000.,
                9000. + (i % 3) as f64 * 1200.,
            )
        });
        for _ in 0..3 {
            for a in 0..12 {
                for b in 0..12 {
                    if a != b {
                        visible += guidance::terrain_visible(
                            black_box(positions[a]),
                            black_box(positions[b]),
                        ) as u64;
                    }
                }
            }
        }
    }
    let los = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let mut collisions = 0;
    for bank in [0., 0.12, 0.6] {
        let mut f = Flight::runway();
        f.orientation *= bevy::math::DQuat::from_rotation_z(bank);
        for _ in 0..3000 {
            let mut trial = f.clone();
            collision::check_airframe(&mut trial, f.position, f.orientation);
            collisions += trial.crashed as u64;
            black_box(trial);
        }
    }
    let collision = start.elapsed().as_secs_f64();
    let report = serde_json::json!({"profile":if cfg!(debug_assertions){"dev"}else{"release"},"checks":{"visible":visible,"collisions":collisions},"seconds":{"line_of_sight":los,"ground_collision":collision}});
    std::fs::write(
        "tests/optimization2-workloads.json",
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{report}");
}
