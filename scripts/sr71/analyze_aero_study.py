#!/usr/bin/env python3
"""Check a --study-sr71 export without launching the renderer or importing dependencies."""
import argparse
import json
import math
from pathlib import Path


def analyze(directory):
    directory = Path(directory)
    summaries = json.loads((directory / 'summary.json').read_text())
    issues = []
    expected = {(case, hz) for case in (
        'neutral', 'doublet', 'left_engine_out', 'hydraulic_loss',
        'jammed_elevon', 'detached_elevons', 'fuel_exhaustion')
        for hz in (60, 120, 240)}
    keyed = {(row['scenario'], row['hz']): row for row in summaries}
    if set(keyed) != expected or len(summaries) != len(expected):
        issues.append('Missing or duplicate study case/rate combination')
    for row in summaries:
        if not row['finite']:
            issues.append(f"Nonfinite trajectory: {row['scenario']} at {row['hz']} Hz")
        if row['peak_moment_closure_nm'] > 1e-6:
            issues.append(f"Moment budget did not close: {row['scenario']} at {row['hz']} Hz")
    convergence = {}
    for case in ('neutral', 'doublet'):
        reference = keyed.get((case, 240))
        if not reference:
            continue
        differences = []
        for hz in (60, 120):
            row = keyed.get((case, hz))
            if not row:
                continue
            altitude_error = abs(row['altitude_change_m'] - reference['altitude_change_m'])
            mach_error = abs(row['final_mach'] - reference['final_mach']) / reference['final_mach']
            differences.append({'hz': hz, 'altitude_difference_m': altitude_error,
                                'relative_mach_difference': mach_error})
            if altitude_error > max(1., abs(reference['altitude_change_m']) * .02) or mach_error > .02:
                issues.append(f'{case} failed nominal 2% convergence at {hz} Hz')
            if row['crashed'] or row['integrity'] < .99:
                issues.append(f'{case} lost aircraft integrity at {hz} Hz')
        convergence[case] = differences
    max_force_closure = 0.
    samples = 0
    for line in (directory / 'snapshots.jsonl').open():
        sample = json.loads(line)['snapshot']
        channels = ('baseline_force_body_n', 'control_force_body_n',
                    'damage_force_body_n', 'engine_force_body_n')
        residual = [sum(sample[channel][axis] for channel in channels)
                    - sample['total_specific_force_body_n'][axis] for axis in range(3)]
        error = math.sqrt(sum(value * value for value in residual))
        max_force_closure = max(max_force_closure, error)
        samples += 1
    if not samples or max_force_closure > 1e-6:
        issues.append('Missing snapshots or force budget failed to close')
    result = {'passed': not issues, 'issues': issues, 'flight_cases': len(summaries),
              'snapshot_samples': samples, 'max_force_closure_n': max_force_closure,
              'nominal_convergence': convergence,
              'note': 'Fault-case aircraft losses are reported, not treated as numerical failures.'}
    (directory / 'analysis.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))
    return not issues


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', nargs='?', default='tests/sr71-aero-study')
    raise SystemExit(0 if analyze(parser.parse_args().directory) else 1)
