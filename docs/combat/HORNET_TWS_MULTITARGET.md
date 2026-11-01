# Hornet TWS missile support

The local-combat Hornet previously required `selected == missile.target` for every supported missile. Selecting another aircraft therefore removed datalink support from previously launched AIM-120s even when their radar tracks were fresh. BVR used a separate mode-aware rule.

Local Hornet and BVR guidance now share a radar support policy:

- TWS supplies independent midcourse updates to missiles against any fresh tracked target. HUD designation/Tab selection does not own the guidance channel.
- STT supports the selected target only, matching its dedicated beam behavior.
- Search mode supplies no weapon updates.
- Radar-off, failed aircraft power/radar, track age above three seconds, invalid tracks, target visibility/return and missile support geometry retain their existing restrictions.
- Once a missile's seeker becomes active, it continues to use its own seeker logic; losing aircraft support does not force an active missile back to datalink.

Use TWS for a multi-target salvo: select a contact, acquire/release, then Tab or select another tracked contact. Enter/LOCK explicitly selects STT; use UNLOCK or the mode control to return to TWS for multiple simultaneous targets. No radar range, missile kinematics, weapon inventory or seeker susceptibility is increased by this correction.

Validation: a deterministic regression acquires four distinct targets, launches four radar missiles, changes designation and checks the actual missile guidance modes in local combat and BVR. It also checks independent track aging, STT, search and radar power gates. Final regression run: 742 passed, 0 failed, 84.91 seconds test execution. The map mode button now routes through the same radar command handler in BVR and local/naval scenarios. The existing Mac app was rebuilt and its ad-hoc signature verified. No additional live-session interruption is part of this correction.
