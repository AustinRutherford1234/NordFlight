# Hornet HUD acquisition review

The reported symptom was failure to acquire or maintain locks across both missile types and combat scenarios. Controlled tests using the existing acquisition logic already acquire and hold locks in Target Practice, Training, Balanced, Hard and BVR. A universal seeker/backend failure was not reproduced. The player nose is pointed at the designated target in these tests; they are not a claim to validate a manually flown engagement or the full guidance envelope.

Confirmed presentation/workflow problems corrected:

- Hornet combat scenario resets previously left the HUD in NAV. Scenarios 4 through 8 now start in A/A. The existing F7 mode controls remain available afterward.
- The IR seeker circle was always fixed at boresight, including while the selected contact was being tracked. It now follows that contact when acquisition is valid, with distinct acquiring/locked sizes.
- The off-screen target arrow used azimuth alone. It now includes elevation, so above/below contacts cue in the appropriate direction.
- A/A showed only NO LOCK for every inhibition. It now shows the actual existing cue rejection, acquisition percentage, selected target identity/Tab reminder and consistent lock/readiness state. Paused flights show RESUME TO FIRE and do not falsely show RDY. Invalid cue conditions cannot display a lock diamond even if dwell state is stale.

The acquisition thresholds, ranges, signal models and radar operating rules are unchanged. In BVR, being outside the modeled launch/seeker range, having Search mode selected, or lacking a fresh radar track remains a valid inhibition. The HUD now exposes the reason directly.

## Automated evidence

`cargo test --locked`: 677 passed, zero failed. The new coverage includes both missiles in all five combat scenarios, acquisition through the actual timer followed by an actual launch, radar standby and folded-wing inhibition with stale dwell, paused readiness, combat-mode reset selection and vertical/off-axis target cues. In the controlled four-second cases, the AIM-9X reaches its 0.8-second dwell threshold and the AIM-120 its 1.3-second threshold; BVR radar acquisition includes the initial wait for a fresh track.

The approved native review uses `scripts/fa18f/capture_lock_review.py` to pause/resume the existing app and capture four disposable app instances. Diagnostic presets advance the normal combat acquisition timer rather than forcing a completed lock. The final render is paused for inspection. All four native presets completed, were inspected, and the previous app session was resumed:

| Preset | Actual dwell | HUD state | Cue rejection |
|---|---:|---|---|
| AIM-9X acquiring | 0.25 s / 31.25% | ACQ 31% | None |
| AIM-9X locked | 0.8 s / 100% | LOCK / RESUME TO FIRE | None |
| AIM-120 locked | 1.3 s / 100% | LOCK / RESUME TO FIRE | None |
| AIM-120 radar off | 0 s / 0% | NO LOCK / RADAR STANDBY | Radar standby |

Local captures and logs are under `tests/hornet-lock-review/`. No application errors or assertion failures were recorded. These are controlled native display checks; the user's exact manually flown acquisition failure has not been reproduced.

`./build-mac.sh dev` updated Nord Flight.app. Strict deep signature verification passed, and the build/packaged ARM64 UUIDs match: `6B48548E-8260-31AB-8EAE-0EB647005742`.
