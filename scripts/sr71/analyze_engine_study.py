#!/usr/bin/env python3
"""Validate the production --study-sr71-engines export (standard library only)."""
import csv
import json
import pathlib
import sys

p = pathlib.Path(sys.argv[1] if len(sys.argv)>1 else 'tests/sr71-engine-study')
rows = list(csv.DictReader((p/'steady.csv').open()))
runs = json.loads((p/'transients.json').read_text())
issues = []
if len(rows)!=150 or len(runs)!=30:
    issues.append('Incomplete steady/transient study')
wet = next(r for r in rows if (r['alt_m'],r['mach'],r['throttle'])==('0','0','1'))
reference = 32500*4.4482216152605
rating_error = (float(wet['net_thrust_n'])-reference)/reference
if abs(rating_error)>.10:
    issues.append('Static afterburning rating outside initial 10 percent sanity tolerance')
convergence = {}
for case in dict.fromkeys(r['case'] for r in runs):
    group = {r['hz']:r for r in runs if r['case']==case}
    if set(group)!={240,480,960}:
        issues.append(f'Missing independent timestep: {case}')
        continue
    ref = group[960]
    errors = {hz:{key:abs(group[hz][key]-ref[key])/max(abs(ref[key]),1.)
                  for key in ('fuel_kg','impulse_ns','peak_thrust_n')} for hz in (240,480)}
    convergence[case] = errors
    if max(e for x in errors.values() for e in x.values())>.02:
        issues.append(f'Integrated impulse/fuel/peak thrust differs by over 2 percent: {case}')
    for hz,run in group.items():
        s = run['final_state']; o = s['output']
        if abs(o['gas_energy_residual_w'])>1e-5 or abs(o['air_mass_residual_kg_s'])>1e-8:
            issues.append(f'Gas energy/mass budget: {case}/{hz}')
        if abs(s['shaft_balance_residual_j'])>1e-6:
            issues.append(f'Shaft work budget: {case}/{hz}')
        if case in ('ground_start','throttle_step','cruise','bleed_jam','windmill_restart','inlet_unstart') and s['mode']!='Running':
            issues.append(f'Expected healthy/recovered running state: {case}/{hz}')
        if case in ('failed_ignition','nozzle_jam','pump_failure','fuel_exhaustion') and s['thrust_n']!=0:
            issues.append(f'Powered thrust after failure/shutdown: {case}/{hz}')
        if case=='inlet_unstart' and (s['inlet']['unstarts']==0 or s['inlet']['mode']!='Started'):
            issues.append(f'No unstart/recovery evidence: {hz}')
        if case in ('ground_start','throttle_step','cruise') and (s['surges'] or s['flameouts']):
            issues.append(f'Unsolicited nominal failure: {case}/{hz}')
# Positive nozzle residual is spare geometric capacity: the modeled core jet
# occupies less than the open nozzle area. Only a negative residual is shortfall.
for row in csv.DictReader((p/'transients.csv').open()):
    for key in ('rpm','thrust_n','drag_n','main_fuel_kg_s','ab_fuel_kg_s'):
        if float(row[key])<0:
            issues.append(f'Negative bounded state: {row["case"]}/{key}')
result = {'passed':not issues,'issues':issues,'steady_points':len(rows),'independent_transient_runs':len(runs),
          'static_wet_thrust_n_per_engine':float(wet['net_thrust_n']),'reference_wet_thrust_n':reference,
          'static_rating_relative_difference':rating_error,
          'infeasible_stress_grid_points':[{'alt_m':float(r['alt_m']),'mach':float(r['mach']),'throttle':float(r['throttle'])}
                                           for r in rows if r['equilibrium_feasible']=='false'],
          'independent_convergence':convergence,'performance':json.loads((p/'performance.json').read_text()),
          'limits':'Estimated component maps; arithmetic/trajectory consistency is not OEM or flight validation.'}
(p/'analysis.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
raise SystemExit(0 if result['passed'] else 1)
