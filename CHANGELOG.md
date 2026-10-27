# Changelog

## 1.1.0

- Make the four-bank layout the default on Linux and Android, with matching JSON, offline HTML, and a one-page portrait PDF. Keep legacy layout imports supported.
- Type each character with one D-pad direction or face button. Use L1/R1 for alpha banks, a 100 ms late-bumper window, and overlapping thumb strokes. Keep mnemonic U/D/L/R/O positions and modest rolling preferences in layout generation.
- Tap L2 for Shift on the next key; tap again to cancel. R2 taps cycle numbers, function keys, and off for one key. Hold L2 or R2 for momentary Shift or numbers. Group numbers 1–4 and 5–8 on their D-pad banks.
- Use R1+R2 for navigation and add L1 for editing/media. Put volume and track controls on the D-pad, A Play/Pause, B Stop, Y Print Screen, and X Delete. Keep the function banks separate in the reference.
- Repeat every mapped output after a 300 ms hold, then every 80 ms. Repeat standalone Backspace/Space holds if no other control joins within 300 ms. Keep captured banks and modifiers, and allow later input while earlier keys repeat.
- Pause/resume with A+B+X+Y. Map L3 to Command+D and R3 to Command+Shift+D for optional Dictate/Cancel Dictation shortcuts.
- Apply held Linux modifiers to mouse actions. On Android, extend text selections with held Shift and a click or tap, including DualSense touchpad taps in browser editors. Preserve holds across same-editor input restarts. Add an optional ADB helper for global mouse modifiers, with disconnect and heartbeat cleanup.
- Allow stopping Android remapping after the controller disconnects. Support the default layout in Linux inspection, recording/replay, and reference rendering.

## 1.0.3

- Move `l` to D-pad ← + L1 in the default layout.
- Prefer L1 + a cardinal direction over bare D-pad diagonals in the default effort costs, and regenerate the layout and references.

## 1.0.2

- Pin `a` to A/Cross and `o` to B/Circle, and regenerate the default layout and chord references.
- Pin `l` to D-pad ↙, `u` to Y/Triangle + R1, and `d` to D-pad ↓ + L1.
- Pin `r` to D-pad →.
- Add Print Screen on function-layer Y + B, with screenshot support on Android.

## 1.0.1

- Convert the Android app, tests, and gamepad test helper to Kotlin.
- Add a one-page portrait PDF chord reference and highlight search results on HTML maps.
- Put usage instructions before installation and link the PDF from the README.
- Use this changelog for GitHub release notes.

## 1.0.0

- Type on Android 14 and newer with DualSense chords or editable direct mappings.
- Type on Linux with a virtual keyboard, controller calibration, and recording/replay tools.
- Use independent typing halves, held Shift and symbol layers, navigation, function keys, media controls, and thumbstick modifiers.
- Generate custom layouts from text and browse offline eight-way chord reference maps.
- Download a signed Android APK from GitHub releases, built automatically for each tag.
