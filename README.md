# Chordpad

Type with a game controller on Linux or Android. Press a button combination
(a chord), then release it to type a letter or key. The included layout is ready
to use, with letters, numbers, punctuation, navigation, and keyboard shortcuts.

See the [changelog](CHANGELOG.md) for release notes.

## Learn the chords

Download the [one-page PDF](layout.pdf) or open the [offline HTML reference](layout.html).
Both show the default layout used on Linux and Android. On Android, choose
**View chord reference** to see your active layout.

Press one D-pad cardinal or one face button per letter. Hold L1, R1, or both
for another bank. You can add the bumper up to **100 ms** after pressing the
letter button, before releasing it. Short presses type on release; holding any
mapped output for **300 ms** repeats it every **80 ms**.

Button names follow physical positions: A/Cross is south, B/Circle east,
X/Square west, and Y/Triangle north.

| Bank | ↑ | → | ↓ | ← | Y / △ | B / ○ | A / × | X / □ |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Base | s | r | n | t | i | o | a | e |
| L1 | b | m | d | l | u | p | y | g |
| R1 | w | f | v | c | x | j | k | h |
| L1 + R1 | / | q | z | Tab | ' | ; | , | . |

| Control | Action |
| --- | --- |
| L1 / R1 alone | Backspace / Space; hold alone 300 ms to repeat |
| Hold L2 | Shift: uppercase letters, shifted punctuation, and selection |
| Tap L2 | Shift for the next key; tap again to cancel |
| Hold R2 or tap once | Numbers and symbols; D-pad ↑ → ↓ ← gives 1–4 |
| R2 + L1 | D-pad 5–8; A is 9 and B is 0 on the first number bank |
| R1 + R2 | Navigation: arrows; Y Page Up, A Page Down, X Home, B End |
| R1 + R2 + L1 | Editing/media: ↑/↓ volume, ←/→ previous/next track; A Play/Pause, B Stop, Y Print Screen, X Delete |
| Tap R2 twice | Function keys for the next key; a third tap cancels |
| Start / Select | Enter / Escape |
| Tap R1 + R2 / L1 + L2 | Arm held modifiers on face buttons / D-pad |
| A + B + X + Y | Pause or resume typing |
| L3 / R3 click | Command+D / Command+Shift+D (Meta/Super) |

L2 and R2 taps last at most 250 ms. R2 taps have no limit between them; the next
key clears the selection. Function keys use the number positions, with L1 for
F5–F8. Hold Shift while using navigation to select text.

Either thumbstick adds keyboard modifiers: up is Alt, down is Command/Meta,
and outward is Ctrl. See the [full modifier map](docs/linux.md#thumbstick-keyboard-modifiers)
for combinations. L3/R3 shortcuts can be assigned to Dictate and Cancel Dictation
on your computer.

Without sticks, tap and release **R1 + R2** to arm face-button modifiers,
or **L1 + L2** for the D-pad. Hold a mirrored stick direction: right-side B is
Ctrl, Y is Alt, A is Command; left-side ← is Ctrl, ↑ is Alt, ↓ is Command.
Adjacent pairs provide diagonal combinations. Type several keys or use your
mouse while holding the direction; releasing it ends modifier mode.
Repeat the unused shoulder-pair tap to cancel. Holding a pair and typing keeps
its existing layer behavior, including R1+R2 navigation. Unused pair taps do not
send Space/Backspace or arm sticky Shift/numbers.

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
layout. Android and Linux share the [four-bank layout](layout.json):
one thumb button per letter, with shared L1/R1 banks. Pause or stop remapping to use
the controller in games. Hold L2 and click or tap another position to extend a
selection in a focused text editor. For other mouse modifier actions, use the
[optional ADB helper](android/README.md#global-mouse-modifiers).

See the [Android guide](android/README.md) for the accessibility button,
app switching, importing layouts, and controller settings.

## Linux

Hold L2 or a modifier stick direction to use it with your mouse, such as
Shift-click or Ctrl-click. Release the control to release the modifier.

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

   Chordpad starts paused. Focus a text field, press **A + B + X + Y**,
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
