#!/usr/bin/env python3
"""Run the production J58 component tests without linking the Bevy application.
Requires dependencies from one successful `cargo test --locked` build.
No production physics is copied into this harness.
"""
import pathlib
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
DEPS = ROOT / 'target/debug/deps'
with tempfile.TemporaryDirectory(prefix='nordflight-engine-') as directory:
    directory = pathlib.Path(directory)
    modules = ('config', 'gaspath', 'inlet', 'state')
    source = ''.join(f'#[path="{ROOT}/src/aircraft/sr71/propulsion/{name}.rs"] mod {name};\n' for name in modules)
    source += f'#[path="{ROOT}/src/aircraft/sr71/propulsion/tests.rs"] mod tests;\n'
    (directory / 'harness.rs').write_text(source)
    command = ['rustc', '--edition=2024', '--test', '-O', '-A', 'dead_code', str(directory/'harness.rs'), '-o', str(directory/'engines'), '-L', f'dependency={DEPS}']
    for name in ('serde', 'serde_json'):
        candidates = list(DEPS.glob(f'lib{name}-*.rlib'))
        if not candidates:
            raise SystemExit('Build the Cargo test dependencies first: cargo test --locked')
        library = max(candidates, key=lambda p:p.stat().st_mtime)
        command += ['--extern', f'{name}={library}']
    subprocess.run(command, cwd=ROOT, check=True)
    subprocess.run([str(directory/'engines'), '--nocapture'], cwd=ROOT, check=True)
