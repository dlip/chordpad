# Chordpad

Type with a game controller on Linux or Android. Press a button combination
(a chord), then release it to type a letter or key. The included layout is ready
to use, with letters, numbers, punctuation, navigation, and keyboard shortcuts.

See the [changelog](CHANGELOG.md) for release notes.

## Learn the chords

Download the [one-page chord reference (PDF)](layout.pdf) for the included layout.

On Android, open **View chord reference**. On Linux, create an offline reference
for the included layout:

```sh
target/release/chordpad render --layout layout.json --output layout.html
```

Open `layout.html` in a browser. Search for a character or key name to find its
chord, filter by layer, or use **Print / Save PDF** for reference sheets.

The default layout lets each hand type independently: the D-pad and L1 form the
left half, and the face buttons and R1 form the right half. Press the controls
for a chord together and release them to send it. Hold Shift or a layer before
starting a chord; those controls can stay held across characters.

Button names follow physical positions: A/Cross is south, B/Circle east,
X/Square west, and Y/Triangle north.

| Control | Action |
| --- | --- |
| A / Cross | `a`; hold L2 for `A` |
| B / Circle | `o`; hold L2 for `O` |
| R1 / L1 alone | Space / Backspace |
| Start (Options) / Select (Share) | Enter / Escape in every layer |
| Hold L2 | Shift: uppercase letters and shifted punctuation |
| Hold R2 | Numbers and symbols |
| Hold R2 + L1 | Navigation: D-pad arrows; Y Page Up, A Page Down, X Home, B End |
| Hold R2 + R1 | Function keys: number chords 1–9 give F1–F9, 0 gives F10; X gives F11, Y gives F12 |
| L1 + R1 + L2 + R2 | Pause or resume typing |

With R2 held, numbers run clockwise around the D-pad:

```text
8  1  2
7  ·  3
6  5  4
```

A/Cross is 9 and B/Circle is 0. X/Square is `-` and Y/Triangle is `=`.
Add L2 for their usual shifted symbols.

Either thumbstick adds keyboard modifiers: up is Alt, down is Command/Meta,
and outward is Ctrl. Inward means toward the controller center. See the
[full modifier map](docs/linux.md#thumbstick-keyboard-modifiers) for combined
modifiers. Hold a modifier before starting the chord, for example Ctrl with
the chord for `c` to copy.

The reference also includes punctuation, media, and volume chords. Navigation
arrows repeat while held; ordinary typing chords send once on release.

## Android

The [Android app](android/README.md) supports Android 14 and newer, with a live
DualSense preview, a chord reference, and editable controller mappings.

1. Download `chordpad.apk` from [GitHub Releases](https://github.com/dlip/chordpad/releases/latest)
   and install it on your device, or [build it yourself](android/README.md#install-and-use).
2. Pair your controller in Android's Bluetooth settings.
3. Open **Chordpad → Enable controller input…** and enable **Chordpad controller
   input** in accessibility settings.
4. Choose **Chordpad chords**, tap **Start remapping**, and focus a text field.
   Release all controls before typing.

Use **Direct mapping** if you want to assign individual buttons or stick
directions instead of typing chords. **View chord reference** shows the active
layout. Pause or stop remapping to use the controller in games.

See the [Android guide](android/README.md) for the accessibility button,
app switching, importing layouts, and controller settings.

## Linux

You need Rust and Cargo, a connected gamepad, read access to its input device,
and read/write access to `/dev/uinput`. Use a **US keyboard layout** with Caps
Lock off. On Nix, enter `nix-shell` first.

Run these commands from the repository root:

1. Build Chordpad and find your controller:

   ```sh
   cargo build --release
   target/release/chordpad devices
   ```

2. Use the event path shown by `devices` to preview chords without typing into
   any app. Replace `/dev/input/event28` with your controller's path:

   ```sh
   target/release/chordpad inspect --device /dev/input/event28 --layout layout.json
   ```

   Release all controls, try a few chords, then press Ctrl-C to exit.

3. Start typing with the included layout:

   ```sh
   target/release/chordpad type --device /dev/input/event28 --layout layout.json
   ```

   Chordpad starts paused. Focus a text field, press **L1 + R1 + L2 + R2**,
   and release all controls to enable typing. Repeat the gesture to pause.
   Ctrl-C stops Chordpad.

Prefer a stable `/dev/input/by-id/*-event-joystick` path when available, so
Chordpad can find the controller after reconnecting. It waits for a disconnected
controller to return; release all controls before typing resumes.

See the [Linux guide](docs/linux.md) for permissions, trigger calibration,
runtime options, and recording/replay. Desktop editor, terminal, and Wayland/X11
compatibility still need hands-on verification.

## Custom layouts and development

- [Layout generation](docs/layout-generation.md): train on your own text,
  change bindings and comfort settings, and evaluate layouts.
- [Linux guide](docs/linux.md): controller setup, calibration, runtime options,
  and visual references.
- [Development](docs/development.md): checks, runtime validation, and project plans.
- [Android guide](android/README.md): app usage, build requirements, and checks.
