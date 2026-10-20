# F-22 moving nozzle panel volume — October 20, 2026

Both upper and lower moving nozzle assemblies now have closed, shaped volume.
The forward rectangular sections and aft chevron sections have a longitudinal
outer ridge, broad sloping faces, narrow side bevels and a thin trailing edge.
The aft section tapers from its full-width rectangular base into the existing
rear-pointing triangle. The upper and lower shapes mirror away from the gas
passage; the previous flat inner-liner clearance is retained.

The shape was inspected against the [National Museum of the USAF F119
photograph](https://commons.wikimedia.org/wiki/File:P%26W_F119.jpg) and an
[installed-aircraft rear photograph](https://1.bp.blogspot.com/-ghLzGwajPMk/XupB8FOAibI/AAAAAAABMY8/4Zy2DVhQ1hk4PbZPtiAK7i4s3JO1SDumQCLcBGAsYHQ/s1600/f22_6_187.jpg).
These are visual proportions inferred from photographs, not calibrated
production-engine dimensions or a reconstruction of internal mechanisms.

The existing separate shoulder/flap poses, throttle aperture and differential
TVC commands remain. Fixed sidewalls and forward fairings remain fixed. The
gray materials and removed external actuator hardware remain as delivered.
No authoritative flight model or engine control code changes.

All **353 tests pass**. New mesh coverage checks upper/lower mirrored volume,
finite normals, nondegenerate triangles, closed edges and positive signed
volume. Existing hinge, aperture and flight tests continue to pass. The exact
runtime export is also plotted at neutral and both vector limits; this plot
checks geometry, while native captures check materials in the game lighting.

Validation artifacts, baseline/release roll-diagnostic comparison, source
hashes, signature verification and native capture identities are recorded in
`tests/f22-solid-nozzle/delivery.json`. The native view set covers top, side,
both vector limits above, and downward vectoring underneath. These are sampled
pose inspections rather than continuous motion or full CAD interference proof.
