# Fresh player aircraft

Carrier Defense's tactical-map panel includes SPAWN F/A-18 / SHIFT+N. Shift+N is also available without opening the map. N/K/J and B/V/L retain their existing infinite-fuel, wind, camera, brakes, countermeasures and lighting bindings.

The action creates a fully fueled/rearmed player Hornet on the current moving carrier using the shared radio-respawn path. It is available even if the existing player aircraft is alive. Battle time, enemy aircraft, live weapons, fleet ammunition and damage remain intact. An ended mission or unusable carrier deck rejects spawning. The map button closes the map after a successful spawn. The radio menu's existing loss-only respawn option remains available.

Validation: full suite passed 733 tests before adding the hotkey. Focused hotkey/spawn tests check Shift+N input consumption, plain N preservation, deck eligibility, moving-carrier velocity and persistent battle state. Rebuilt the Mac app. No additional native-app session interruption was requested for this control change.
