# Start screen and branding

The app opens into an animated hangar. Choose Atmospheric Flight or Spaceflight,
then choose a vehicle, configuration and starting scenario. Fly (or Enter)
starts a new scenario through the existing flight reset path. Spaceflight
currently offers one capsule/lander vehicle, with Earth orbit, lunar orbit and
lunar landing approach starts. Atmospheric flight retains all five aircraft,
cold-and-dark, takeoff, approach, target practice, three air-combat difficulties
and all four BVR force sizes with difficulty and objective choices.

The selected atmospheric model appears in a lit hangar with a moving camera.
The spacecraft appears against a slowly turning Earth, Moon and star backdrop.
This is a separate rendering layer. Its wall-time animation does not integrate
the player's vehicle, advance fuel, change damage or advance orbital time.
Preview snapshots reuse the existing mesh/material handles. The authoritative
Flight resource is restored immediately after capturing preview poses.

Escape during flight opens the pause menu. Resume restores the previous pause
state. Main Menu opens new-mission setup while retaining the previous flight
for Resume. Launching a new scenario intentionally replaces that flight.
Settings exposes the two existing graphics presets and persistent master audio
controls; Controls lists flight, weapons, map and time-warp shortcuts.
Mouse wheel scrolls the menu on smaller windows. F6 still opens the existing
in-flight aircraft hangar. Quiet mechanical ambience and UI switches use the
existing audio mixer and respect master volume/mute.

Diagnostic/benchmark command lines bypass the start screen. Developer native
captures can opt in with `NORD_MENU_CAPTURE=atmosphere`, `space`, `settings` or
`controls`. The new app icon, menu branding and README use the user-approved
Northern Lights illustration with the silver/gold title from Option 1. See
[the image provenance and revision prompt](../assets/branding/README.md).

The macOS build script now copies the icon, Info.plist and runtime assets into
the app bundle. Asset loading checks bundled resources before the development
checkout, so Finder launch does not require a particular working directory.
The Windows script copies runtime assets alongside the executable; this does
not constitute a Windows runtime validation.
