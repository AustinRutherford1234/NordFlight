# F-22 weathered gray nozzle panels — October 19, 2026

The subsequent [aft-tip correction](F22_AFT_NOZZLE_TIP.md) reverses the
triangular extension described below: the triangle belongs at the aft end,
away from the nose. The gray material remains unchanged.

The moving exit panels now have a rectangular aft section and an actual
triangular forward extension in their perimeter. They are five-sided solids,
rather than square slabs with triangles placed on top. Their worn edge trim
follows the complete outline. A small exterior lap clears the forward backing
panel during articulation. The existing separate forward/aft motion and fixed
lateral sidewalls are retained.

The near-black outer-panel finish is replaced by lighter gray weathered metal.
A deterministic 512 × 512 opaque sRGB texture adds mottling, heat stains and
fine wear. Mesh UVs map that texture across the panels; neutral vertex shading
replaces the older brown heat tint. The moving flaps, forward covers, fixed
boat-tail and exposed lateral facets share this finish. The recessed duct and
burner backing remain dark. The material retains a metallic response, with
greater roughness and some diffuse contribution so exterior panels remain
readable outside direct reflections.

The shape and finish use the previously inspected
[F-22 nozzle close-up](https://commons.wikimedia.org/wiki/File:F-22_thrust_vectoring_nozzles_at_SIAF-2022.jpg)
and [motion footage](https://makeagif.com/gif/f-22-thrust-vectoring-nozzles-C0mygK)
as visual references. Public references do not establish exact panel dimensions
or linkage schedules; those remain procedural approximations. No aircraft
physics, engine thrust, nozzle commands or control laws were edited.

All **346 application tests pass**, including the existing articulation and
aperture sweep plus the new triangular-outline/opaque-texture regression.
The GPU-free preview exports the exact runtime mesh builder and update output;
native captures check the textured material under actual scene lighting.
Captures are sampled poses, not a performance benchmark or per-frame video.

Starting source/executable snapshots, final source hashes, numerical flight
study comparisons, release signature verification and final native capture
identities are recorded under `tests/f22-gray-panels/`, with the accepted
installed build summarized in `delivery.json`.
