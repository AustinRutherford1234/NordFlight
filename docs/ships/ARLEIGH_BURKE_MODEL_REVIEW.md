# Arleigh Burke escorts: model review and proposed work

Status: **approved by the user for integration** through the built-in review question. The initial review downloaded and inspected the free candidate in a standalone Blender scene before any game integration. This document records the approved proposal; implementation and validation are documented separately in `ARLEIGH_BURKE_IMPLEMENTATION.md`.

## Recommended base and two ships

Use [USS Jack H. Lucas DDG-125 by SB-129](https://sketchfab.com/3d-models/uss-jack-h-lucas-ddg-125-1c9229b49915454c8ff22605f8cc7b66), a free model listed under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). The live download dialog offers OBJ, glTF, USDZ and GLB; the 2K GLB was downloaded successfully. Its embedded asset metadata also identifies SB-129, the source page and CC-BY-4.0. Preserve those credits, source link, license and modification history with every distributed derivative.

Represent **USS Jack H. Lucas (DDG 125)** and **USS Ted Stevens (DDG 128)** using a shared Flight III base, hull-specific markings and photo-supported fitting differences. The Navy identifies Jack H. Lucas as the first Flight III destroyer; Navy/NAVSEA sources identify Ted Stevens as Flight III as well. These ship names suit this model family; exact individual fittings still require the reference pass below.

- [Navy: delivery of Jack H. Lucas](https://www.navy.mil/Press-Office/News-Stories/display-news/Article/3441141/navy-accepts-delivery-of-future-uss-jack-h-lucas/)
- [NAVSEA: delivery of Ted Stevens](https://www.navsea.navy.mil/Media/News/Article-View/Article/4368261/navy-accepts-delivery-of-future-uss-ted-stevens/)
- [Official Ted Stevens ship page](https://www.surflant.usff.navy.mil/Organization/Operational-Forces/Destroyers/USS-Ted-Stevens-DDG-128/Welcome/)

This is the strongest candidate inspected locally, rather than a claim that every free model on the internet has been audited.

## Free alternatives considered

| Candidate | Public listing | Assessment |
|---|---|---|
| SB-129, Jack H. Lucas DDG-125 | Free, CC BY 4.0; listing about 79.9k triangles | Recommended. Actual GLB downloaded, parsed, imported and rendered; see measured results below. |
| [waelXcm, Alreigh Burke Destroyer](https://sketchfab.com/3d-models/alreigh-burke-destroyer-52a04129e8f64134ae26d365c4e82ce7) | Free, CC Attribution; 80.6k triangles; author describes working gun/CIWS | Backup listing only; file not downloaded or audited. Page carries a NoAI tag, which must be resolved for the intended workflow before adopting it. Its class configuration must also be identified from the geometry. |
| [ammar_ssr, USS Arleigh Burke Destroyer](https://sketchfab.com/3d-models/uss-arleigh-burke-destroyer-952c093b8709495bb96e7c8e0ce55c8b) | Free, CC Attribution; 363k triangles | Not recommended: uploader explicitly says it is not theirs, leaving the original attribution/provenance unresolved. Higher triangle count alone does not establish better usable detail. |

Paid listings were excluded. Flight I print-oriented STL alternatives were not selected because they would require extensive material work and a configuration change to match the requested helicopter hangars.

## Measured candidate inspection

Source download: `/Users/Austin/Downloads/uss_jack_h._lucas_ddg-125.glb`.

- SHA-256: `89752908a8d2cd198256636d13d76be616032ce82cd8c53a79013c6b53010290`.
- File size: **13,207,124 bytes**.
- Blender import: **77,672 triangles**, **22 mesh objects**, **11 materials**, **11 embedded images**. The listing's approximate count differs from this downloaded conversion; use the measured count for budgets.
- Texture sizes: seven 2048×2048 images, one 1024×1024, two 512×512, one 256×1024.
- Materials contain base-color textures, no normal maps, no roughness/metalness textures, and no emissive textures/factors. Metallic factors are zero. All source materials are double-sided and use opaque alpha mode.
- No glTF animations or supplied LOD chain were found.
- Source object names are generic `Object_*`, rather than usable gun/radar/door component names.
- The import is not in usable metre units: the longest dimension is about 10,368 source units, with the bow/stern axis along Blender X. It must be aligned, scaled and placed at its waterline before integration. The review renders use a provisional 155.3 m overall length solely for inspection; final scale and waterline must be checked against the selected ship references.
- Geometry already includes the hull, bridge, radar faces, mast/rigging, exhaust structures, gun, launcher hatch arrays, boats, deck fittings and helicopter facilities. The source includes a **parked helicopter**, which must be removed to satisfy the requested clear flight deck.

All eleven textures imported. The standalone scene rendered successfully from four angles. This verifies file accessibility and Blender usability; Bevy import, moving collision integration, final appearance and runtime performance remain future validation.

## Review images and real references

These are renders of the downloaded candidate with neutral review lighting, before any model repairs. They are not in-game captures or AI-generated concept art. The helicopter and source markings visible in them are part of the downloaded model.

- [Bow quarter](../../tests/escort-model-review/bow-quarter.png)
- [Stern quarter](../../tests/escort-model-review/stern-quarter.png)
- [Port profile](../../tests/escort-model-review/port-profile.png)
- [Bridge/mast/deck detail](../../tests/escort-model-review/deck-detail.png)
- [Inspection measurements](../../tests/escort-model-review/inspection.json)

Use these primary references to judge proportions and rebuild details, rather than treating the asset title as proof of accuracy:

| View/reference | What to check |
|---|---|
| [U.S. Navy/DVIDS: DDG 125 transiting San Diego Bay, 12 November 2025](https://www.dvidshub.net/image/9434185/navys-flight-iii-destroyer-brings-significant-combatant-capabilities) — [reference photo](https://d1ldvf68ux039x.cloudfront.net/thumbs/photos/2512/9434185/1000w_q95.jpg) | Overall hull/superstructure proportions, radar faces, bridge, mast and boats. Photo: MC1 Sara L. Eshleman. |
| [Official Navy virtual tour of DDG 125](https://www.youtube.com/watch?v=wUssm0X_ELw) | Close fittings, passageways, deck equipment and ship-specific visual references. |
| [U.S. Navy: DDG 128 arrival at Norfolk, 15 May 2026](https://www.surflant.usff.navy.mil/Press-Room/Photo-Gallery/igphoto/2003932531/) | Bow shape, hull number, bridge, mast and side details. Photo: MC1 Sophie A. Pinkham. |
| [HII: DDG 128 sail-away gallery, 8 May 2026](https://www.hii.com/news/flight-iii-destroyer-ted-stevens-ddg-128-sails-aways-from-hiis-ingalls-shipbuilding) | Both side layouts, stern/aviation area, radar/mast configuration and builder reference views. |

Additional port/starboard, stern, overhead and night photographs must be added wherever these views do not establish a fitting. Do not invent a specific real-ship antenna or weapon fit to fill an unclear image. Close-up comparison against those references is an implementation task, not a completed claim in this review.

## Proposed model work

### 1. Prepare a consistent source asset

Keep the downloaded file untouched; create an editable Blender project and derivative GLBs. Apply metre scale, align bow with the simulator's forward convention, and set an explicit waterline origin. Check hull length, visible beam, deck heights and mast height independently rather than scaling to length alone. Name meaningful assemblies and preserve attachment points for later animation/system work.

### 2. Repair and detail for 20–50 m flybys

Remove the parked helicopter and its associated separate geometry; retain a clear marked aviation deck and closed hangar doors. Identify geometry by location/connected components before deleting, because the current objects are material partitions with generic names.

Work evenly across the ship:

- Hull: bow/sonar/anchor forms, hawse openings, deck edge, stern transom, waterline stripe, drainage details and clean normals.
- Bridge/superstructure: window recesses, bridge wings, door frames, ladders, vents, radar-face borders and exhaust openings.
- Masts: proper supports, platforms, safety rails, antenna/dome shapes and readable rigging without disconnected floating fragments.
- Deck: believable railing thickness and attachment, bollards, fairleads, liferaft racks, boats/davits, lockers, hoses, closed hangar-door seams and flight-deck markings.
- Static fittings: main gun, VLS hatch geometry, CIWS and hull-appropriate visible mounts. Compare the selected hulls' dated photographs before retaining or adding a specific fitting.
- Crew: a small number of static Navy deck figures and equipment in safe, plausible locations. Reuse suitable project crew assets where practical; keep geometry modest and clear of the helicopter deck.

Check from above, below and both sides. Fix reversed faces, flat-shaded curved surfaces, overlapping panels and incorrectly double-sided closed metal. Preserve true thin surfaces that need two-sided rendering. The outcome should fit the existing carrier's detail and material style.

### 3. Materials and two distinct hull identities

Use moderately weathered Navy gray: restrained panel variation, runoff at drains, exhaust staining and local rust around exposed fittings. Replace obvious repeated/checker-like texture patterns where they do not match photographs. Add shared normal and packed roughness/metalness detail where it improves close views.

Share the base geometry and most textures between both ships. Remove or mask source numbers before adding clean per-ship `125` and `128` markings; correct stern nameboards and flight-deck identifiers too. Add only photo-supported small fitting differences, rather than presenting two repaints as exact replicas of different current configurations.

### 4. Night lighting

Add correct-side navigation colors, masthead/stern fixtures, selective bridge-window illumination, doorway/bulkhead lighting and subdued hangar-exterior/aviation-deck lights. Doors stay closed; do not make light visibly pass through solid hangar doors. Shallow interior inserts behind lit bridge windows can avoid a flat glowing sheet.

Use shared emissive masks and small fixtures. Limit actual lights/shadows to nearby views; emissive materials must use a render path that actually applies emission. Validate dusk/night exposure so the lights remain legible without turning the ships into luminous blocks. This is new work: the downloaded model currently has no working night-light setup.

### 5. Distance-based simplification and performance

Initial budgets to test, not measured performance promises:

| Level | Initial range | Proposed geometry budget per ship |
|---|---|---|
| LOD0 | Up to about 300 m / large projected size | Approximately 100–150k triangles after targeted detail additions |
| LOD1 | About 300 m–1 km | Approximately 35–50k triangles |
| LOD2 | About 1–3 km | Approximately 8–15k triangles |
| LOD3 | Beyond about 3 km / small projected size | Approximately 1–3k triangles; preserve silhouette |

Use projected size as well as distance and hysteresis to prevent toggling. Preserve the mast/radar/hull silhouette through transitions. Batch compatible rigid details; share meshes, textures and materials across both instances. Cull tiny crew and fixtures at distance. Avoid allocating/changing materials every frame.

Compressed download size is not GPU memory usage: the source images would occupy about **119 MiB as decoded RGBA8 before mipmaps**, shared by both instances, with actual formats/mipmap behavior still to measure. Budget additional maps carefully. Profile the actual Mac with the carrier and both escorts visible in cockpit/external views after integration; request permission before runtime diagnostic/soak sessions.

### 6. Formation and model-derived colliders

Place one escort **500 m port** and the other **500 m starboard**, measured centre to centre in the carrier's frame. Start abeam, on the same heading, maintaining the existing straight 25-knot course. This leaves the existing approach/launch lane clear. Show both whenever the carrier is active, including future carrier modes, and use the existing water presentation initially.

Generalize the existing carrier collision skin/BVH so it can query three ship instances, with independent transforms and shared geometry per class. Export colliders from the final corrected destroyer geometry in the same frame as its render asset. All contact consumers must choose the earliest hit across the carrier and escorts; an escort must not accidentally use the carrier's origin or deck outline. Retain swept aircraft, debris, ejection, store and projectile contacts, including ship-relative velocities. Do not add carrier-style arrest/catapult behavior to the destroyers.

Keep named visual mounts/attachment points suitable for later SAM/naval-combat work. This milestone implements static visible weapon geometry, not SAM detection, guidance, firing or combat AI.

### 7. Acceptance and delivery after approval

- Compare both hulls against dated reference views; verify scale, waterline, clear flight decks, closed doors, markings and fixture attachment at 20, 50 and 500 m.
- Inspect daylight, dusk and night in the actual app, in cockpit and external views.
- Sweep contacts against both destroyers' bows, sides, sterns, superstructures and masts at different simulation times; verify no phantom walls between ships.
- Test reset/mode changes, formation spacing over time, all-ship projectile/debris contacts and existing carrier landing/arrest/launch regressions.
- Inspect LOD transitions and measure whole-scene frame-time/memory behavior against a carrier-only baseline on this Mac; report any material regression before acceptance.
- Rebuild/sign the Mac app, include attribution and evidence, and push the complete authorized source/assets after the integrated model review.

## Review decision

Recommended: approve the SB-129 Flight III base, with **DDG 125 port / DDG 128 starboard**, and the work above. Approval authorizes the model/detail, lighting, LOD, formation and collider integration. The user approved this model and integration plan; model/formation/collider integration may proceed.
