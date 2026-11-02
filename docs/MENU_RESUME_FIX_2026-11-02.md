# In-flight Settings / Controls resume repair

The shared menu stored its return page every time Settings or Controls was clicked. Reopening Settings changed its return page to Settings itself; switching between Settings and Controls created a two-page return loop. Back and Escape could therefore fail to reach the pause menu. The same problem affected navigation from the preflight setup menu.

Settings and Controls now retain their original parent (Pause or Setup) throughout sibling-page navigation. Opening the pause menu and returning to the main menu explicitly establish that parent. Both submenus expose Back and, when an existing flight is available, Resume Flight near the top. The current footer tab is highlighted. Resume restores the pause state saved when the menu opened, so a flight that was already paused stays paused. Keyboard/mouse state is cleared on resume; a completed resume or launch stops processing additional same-frame menu actions.

## Regression evidence

Before changing production menu handling, the new in-flight repeated/sibling-navigation and preflight sibling-navigation tests both failed against the old code. The existing three menu tests passed. The repair adds coverage for repeated Settings/Controls clicks, cross-navigation, Back, Escape, direct resume from either submenu, input clearing, reopening, preflight navigation and preserving both originally running and originally paused flights. State comparisons check position, velocity, attitude, simulation time and fuel when returning through the menu.

## Review steps

1. During flight, press Escape, then Settings. Click Settings again; Back should return to Flight Paused. Resume Flight should return to the same flight.
2. Open the pause menu again. Switch repeatedly between Settings and Controls; Back should still return to Flight Paused. Escape should do the same, and a second Escape should close the pause menu.
3. From either submenu, use the Resume Flight button near the top to return directly.
4. Repeat after pausing with P first: leaving the menu should retain that pre-existing pause. P then resumes simulation.
5. Before starting a flight, switch between Settings and Controls: Back should return to setup, and no Resume Flight action should appear.

These are the intended native review steps. Automated tests exercise the real Bevy menu-control system headlessly; this task does not interrupt a running user flight or claim a completed native visual review.

## Recorded results

`cargo test --locked`: **799 passed, zero failures, one opt-in timing diagnostic ignored**, in 24.11 seconds. All seven menu tests passed. No flight physics or rendering-quality changes are included in this fix.

The existing `Nord Flight.app` was rebuilt with `./build-mac.sh dev` and passed `codesign --verify --deep --strict`. Executable SHA-256: `9db6f740a4f31f21bc4321fd2f0a78cd2d4400c51adb192a9794fb3df6cbd9b6`. The running user session was left untouched; quit and reopen the app to load this executable.
