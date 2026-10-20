# Local Bevy PBR 0.18.1 patch

Upstream crate: `bevy_pbr` 0.18.1, copied from the locked crates.io package.
The upstream MIT and Apache-2.0 license files are retained.

`src/atmosphere/node.rs` differs from upstream production code. It caches
the camera-independent transmittance and multiscattering lookup-table outputs.
The original WGSL, resolutions, sample counts, bind groups, sky/aerial tables,
environment lighting, shadows and materials are retained.

Reuse requires exact atmospheric/settings float bits, medium texture-view IDs,
both output texture-view IDs, and both prepared compute-pipeline IDs. Track
each output independently so texture-cache reassignment between views cannot
reuse stale contents. Shader reloads, medium edits, resize/quality changes and
new textures invalidate reuse. The cache owns no GPU textures and retains at
most 62 output records. Pipelines must be ready before recording a result.

Set `BEVY_DISABLE_STATIC_ATMOSPHERE_CACHE=1` for same-executable comparisons with
the original dispatch behavior. This diagnostic override is read when the node
is created; it does not change shader quality.

Run the focused invalidation/aliasing regressions with:

```sh
cargo test --locked -p bevy_pbr --lib static_lut_tests
```

`src/light_probe/generate.rs` also reuses the existing environment-generation
bind groups, texture views and constant buffers when their inputs are identical.
The key includes all five texture identities, the intermediate default view,
all five prepared layout identities, the sampler, source/noise dimensions and
the combined/split layout setting. Missing prepared bindings force a rebuild.
The key and bindings are installed together on the live render entity, so this
cache owns no global history. Resource replacements, resize, view recreation,
blue-noise changes and layout/sampler changes invalidate it. Updating pixels
inside the same textures requires no rebinding. All dispatches retain their
original constants. No lighting update is delayed or approximated.

Set `BEVY_DISABLE_ENV_BIND_GROUP_CACHE=1` for same-executable comparisons with
the original resource-preparation behavior. Run its focused tests with:

```sh
cargo test --locked -p bevy_pbr --lib binding_cache_tests
```

The atmosphere producer additionally records each actual GPU source-texture
write using the ID in its bound output group. Downsampling snapshots the source
revision actually copied into its intermediate chain. Filtering can reuse a completed
result when that copied revision, all resource identities/constants and all five
prepared pipeline identities match. It does not reuse partial/unprepared output.
This is restricted to known atmosphere-owned sources and exactly one extracted
3D view. Generic environment maps and multiple-view scenes keep original work.
Copy/downsampling still run; radiance and irradiance are omitted only when their
complete inputs have not changed. The convolution shaders have no frame/time
input. Atmosphere movement, lighting changes or re-entry write the source again
and immediately invalidate reuse. No active lighting update frequency changes.

The revision registry owns no GPU resources and is pruned to live source IDs;
each output's last-result key lives on its render entity. Never opt an additional
GPU writer into this ownership contract without recording all its writes.
The producer may run between copying and filtering; reading the live source
revision at filter time would incorrectly tag an older intermediate. A focused
regression exercises a final producer write in this ordering followed by a coast.
The copied version forces the necessary final recomputation on the next copy.
Set `BEVY_DISABLE_ENV_CONTENT_CACHE=1` to retain the original filtering dispatches.
Opt-in benchmark counters report hits/misses/actual source writes/bypasses.

The engine remains pinned to 0.18.1. Re-audit all static shader dependencies
before carrying this patch to another Bevy version.
# October 17: coherent atmospheric environment updates

The application may opt in with the extracted `generate::EnvironmentMapScene`
camera component. It must version every application-controlled atmosphere/light
input in `source` and discontinuities in `generation`; other applications without
this component keep complete filtering. Camera orientation is retained in this
application's source inputs because the LUT parameterization uses it.

Graph edges enforce LUT → cubemap → downsampling → filtering → main-pass order.
Probe bindings refresh when actual GPU lookup textures change. Source reuse also
keys the environment/LUT pipeline identities and medium texture views.

Persistent back textures retain complete original sample counts and mip chains.
One immutable downsampled snapshot feeds all faces; diffuse/specular front maps
are updated together only after all faces complete. A cut, target replacement or
pipeline change resets pending work and requires a full first update. Six/three
face stages and the full-update reference are selected by NORD_ENV_STAGES. The
existing content-cache bypass disables slicing. Generic generated maps retain
full updates; the one-view ownership restriction remains.

NORD_ENV_LEGACY_ORDER restores the formerly disconnected cubemap node for
diagnostic reproduction only. Do not use it in a packaged launch configuration.

