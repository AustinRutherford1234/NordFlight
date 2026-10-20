"""Reconstruct the baseline and align diagnostics, without production optimizations."""
import hashlib
import json
from pathlib import Path
import shutil
import tarfile

ROOT=Path(__file__).resolve().parents[2]
DEST=ROOT/"tests/optimization3/baseline-source"

def replace(path,old,new):
    text=path.read_text()
    if text.count(old)!=1: raise RuntimeError(f"Expected one marker in {path}: {old[:80]}")
    path.write_text(text.replace(old,new))

def between(text,start,end): return text[text.index(start):text.index(end,text.index(start))]

if __name__=="__main__":
    if DEST.exists():raise SystemExit("Baseline source already exists; preserve it")
    DEST.mkdir()
    with tarfile.open(ROOT/"tests/optimization3/baseline/instrumented-source.tar.gz") as archive:
        archive.extractall(DEST,filter="data")
    for name in ["performance.rs","benchmark.rs"]:shutil.copy2(ROOT/"src"/name,DEST/"src"/name)
    original=DEST/"src/main.rs";candidate=(ROOT/"src/main.rs").read_text()
    replace(original,"mod aero_fx;","mod aero_fx;\nmod benchmark;")
    replace(original,"    planner.capture_context(&initial_flight, &initial_view);","    planner.capture_context(&initial_flight, &initial_view);\n    let initial_warp=benchmark::configured_warp(render_benchmark.is_some());")
    replace(original,".insert_resource(warp::Warp::load())",".insert_resource(initial_warp)\n        .init_resource::<benchmark::Session>()")
    replace(original,"        .add_plugins(display_upload::DisplayUploadPlugin)","        .add_plugins(display_upload::DisplayUploadPlugin)\n        .add_systems(PreUpdate, benchmark::neutral_inputs.after(bevy::input::InputSystems).before(bevy::ui::UiSystems::Focus))")
    replace(original,"                    sound::ui,","                    sound::ui,\n                    benchmark::exercise,")
    replace(original,"                    sound::update,\n                    sound::diagnostics,\n                    draw_guidance,","                    (sound::update, sound::diagnostics, draw_guidance).chain(),")
    text=original.read_text()
    a=between(text,"fn benchmark_render(","fn setup_view(")
    b=between(candidate,"fn benchmark_render(","fn setup_view(")
    replace(original,a,b)
    replace(original,"        if advanced < requested - 1e-6 {","        performance::advanced(advanced);\n        if advanced < requested - 1e-6 {")
    replace(original,"    clock.accumulator = warp.timestep * warp.effective;","    clock.accumulator = warp.timestep * warp.effective;\n    let requested=clock.accumulator;")
    replace(original,"        clock.accumulator -= dt;\n    }","        clock.accumulator -= dt;\n    }\n    performance::advanced(requested);")
    replace(original,"    let density = earth::sky_density(f.altitude());","    let _measure=performance::span(8);\n    let density = earth::sky_density(f.altitude());")
    policy=between(candidate,"                        compute: bevy::app::TaskPoolThreadAssignmentPolicy {","                        io: bevy::app::TaskPoolThreadAssignmentPolicy {")
    replace(original,"task_pool_options: bevy::app::TaskPoolOptions {","task_pool_options: bevy::app::TaskPoolOptions {\n"+policy)
    signatures={
        "collision.rs":("pub fn check_airframe(f: &mut Flight, previous: DVec3, previous_rotation: bevy::math::DQuat) {",12),
        "combat/ai.rs":("pub fn fly(t: &mut Target, e: &Environment, desired: DVec3, speed: f64, d: Difficulty, dt: f64) {",13),
        "terrain.rs":("fn build(tile: Tile) -> Built {",14),
        "warp.rs":("pub fn coast(f: &mut Flight, e: &mut Environment, duration: f64) -> f64 {",15),
        "combat/dogfight.rs":("    pub fn tick(&mut self, f: &mut Flight, dt: f64) {",2),
    }
    for name,(signature,index) in signatures.items(): replace(DEST/"src"/name,signature,signature+f"\n    let _measure=crate::performance::span({index});")
    replace(DEST/"src/world.rs","    let basis = frame(&f).conjugate();","    let _measure=crate::performance::span(9);\n    let basis = frame(&f).conjugate();")
    # These methods are called solely by the opt-in shared input script.
    combat=(ROOT/"src/combat.rs").read_text()
    helpers=between(combat,"    /// Input emulation for the opt-in validation driver", "    pub(crate) fn sound_impacts")
    replace(DEST/"src/combat.rs","impl Combat {","impl Combat {\n"+helpers)
    hashes={str(p.relative_to(DEST)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((DEST/"src").rglob("*")) if p.is_file()}
    (DEST/"diagnostic-source-hashes.json").write_text(json.dumps(hashes,indent=2))
    print(DEST)
