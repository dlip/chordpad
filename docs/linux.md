# Linux usage and controller setup

Start with the [README](../README.md#linux) to build Chordpad and begin typing
with the bundled layout. This guide covers runtime options, controller setup,
and troubleshooting. Run commands from the repository root.

## Type on Linux

Build the app and start it with your layout and a readable gamepad event path:

```sh
cargo build --release
target/release/chordpad type --device /dev/input/event28 --layout layout.json
```

It starts paused. Press **A + B + X + Y**, then release all controls to enable
typing. Repeat that gesture to pause. Each action commits when its ordinary
controls are released; triggers assigned to Shift or layer can stay held.
Ctrl-C or SIGTERM stops the process. It does not record normal typing.

Test the runtime without sending keyboard events:

```sh
target/release/chordpad type --device /dev/input/event28 \
  --layout layout.json --dry-run --start-active --duration 30
```

`--dry-run` prints `WOULD TYPE` actions and never opens uinput. `--start-active`
skips the initial pause, while still requiring neutral controls before typing.
`--calibration controller.json` uses saved trigger settings. `--grab` requests
exclusive controller access; the default leaves the gamepad available to other
applications. On device loss, Chordpad discards pending strokes, releases keyboard
keys, and waits for the controller to return at the supplied path. Reconnecting
reloads its mapping, restores `--grab`, and preserves the pause state. Release all
controls, including triggers and sticks, before typing resumes. Pause/resume
signals and Ctrl-C/SIGTERM still work while waiting; `--duration` includes the
time disconnected.

Use a stable `/dev/input/by-id/*-event-joystick` or `/dev/input/by-path/` link so
reconnection works if Linux assigns a new event number. A raw `/dev/input/eventN`
path is retried as given. A device with a different vendor/product or name at
that path is ignored. The controller must be present when the command starts.

The keyboard backend uses [uinput](https://docs.kernel.org/input/uinput.html),
needs read/write access to `/dev/uinput`, and assumes the
desktop uses a **US keyboard layout**, with Caps Lock off and no other keyboard
modifiers held. It emits complete press/release sequences for every action,
including temporary keyboard modifiers. General Unicode and other keyboard
layouts are not implemented. The default four-bank layout
supports hold-to-repeat and one-shot L2 Shift; unused R2 taps cycle numbers →
functions → off. Selections clear after the next key. Used holds stay momentary.
Pause and disconnect clear them.

The PID printed at startup supports pause/resume from another terminal:

```sh
kill -USR1 PID   # pause
kill -USR2 PID   # resume after controls return to neutral
```

Signals discard pending input and refresh controller state. Resume cannot finish
a stroke that started while paused. The controller toggle consumes its whole
gesture, including release; no character is emitted. `--pause-buttons A,B,L1,R1`
changes the reserved button set. Its supersets are reserved in every layer and
D-pad position. Layouts with conflicting bindings or disabled gesture buttons
are rejected before device access.

For custom layouts, see [pause gestures and shortcut bindings](layout-generation.md#pause-gestures-and-shortcut-bindings).

## Default four-bank layout

The default [four-bank layout](../layout.json)
uses single thumb buttons, a 100 ms late-bumper window, R1+R2 navigation,
and one-shot L2 Shift. R2 taps cycle numbers → function keys → off. Build this checkout, then run:

```sh
target/release/chordpad type --device /dev/input/eventN \
  --layout layout.json
```

Use the controller path from `chordpad devices`. It starts paused; press all four
face buttons, then release everything to start or pause. This leaves all four
shoulders available for Shift plus the second navigation bank. Add
`--dry-run --start-active` to print recognized actions without typing.
Every mapped output in the four-bank layout repeats after a 300 ms hold,
then every 80 ms. This includes letters, numbers, navigation/function/media
keys, menus, and stick-click shortcuts. Layer and modifier selectors stay
selectors. Release stops the held output without an extra tap. First outputs
retain press order; repeating keys do not block later completed input.

Hold L1/Backspace or R1/Space alone for 300 ms to start repeating every 80 ms.
Another control pressed before the deadline prevents repeat for that hold.
Once repeating, that bumper acts as its key until release and does not select a
bank. Release stops it without an extra tap.
See the [portrait reference](../layout.pdf).

## Thumbstick click shortcuts

On Linux, **L3 sends Command+D** and **R3 sends Command+Shift+D**. Command uses
Meta / Super. Short clicks fire once on release in every layer and ignore held keyboard
modifiers and stick directions. In the four-bank layout, holding a click repeats
its shortcut after 300 ms, every 80 ms. Pause, disconnect, or reset
cancels pending clicks and requires all controls to return to neutral.
You can assign L3's Command+D to **Dictate** and R3's Command+Shift+D to
**Cancel Dictation** in your computer's shortcut settings.

Automatic mappings detect L3/R3 at Linux button codes 317/318 when advertised.
For an older calibration file, add `"L3": 317, "R3": 318` under `menu_buttons`,
or export a fresh calibration.

## Thumbstick keyboard modifiers

Both sticks provide the same modifiers, mirrored around the controller center:

| Direction | Keyboard modifiers |
| --- | --- |
| Up | Alt |
| Down | Command (Meta / Super on Linux) |
| Outward | Ctrl |
| Outward-up | Ctrl + Alt |
| Outward-down | Ctrl + Command |
| Inward | Ctrl + Alt + Command |
| Inward-up | Command + Ctrl + Alt + Shift |
| Inward-down | Alt + Command |

Inward is right on the left stick and left on the right stick. Stick modifiers
combine with each other, L2 Shift, and the R2 numbers/symbols layer. They apply to
characters, editing keys, and explicit shortcuts on either half. The modifier
set is captured at stroke start; releasing it early preserves that stroke.
Held Shift and stick modifiers press real OS modifier keys immediately and
release them when the controls return to neutral. Hold L2 while clicking the
mouse for Shift-click, or hold a Ctrl stick direction for Ctrl-click. This works
without typing a controller key. Ordinary typing keeps shared modifiers down;
fixed shortcuts and media keys temporarily use their own modifier set, then
restore the held selection. Releasing a modifier before a captured stroke
finishes still preserves that stroke's modifiers.

One-shot Shift remains a next-key selection; it does not hold an OS modifier
between keys. In the four-bank layout, an unused L2 tap must last at most 250 ms;
a longer hold does not arm one-shot Shift on release. Pause, disconnect, invalid
input, dropped-input resets, and exit release held modifiers. Reset requires
full neutral before rearming.

Automatic Linux mappings detect the left stick at ABS_X/ABS_Y (0/1) and the right
stick at ABS_RX/ABS_RY (3/4). Center comes from the advertised range midpoint.
Stick directions have per-axis hysteresis: engage at 55% travel and disengage at
35%. Calibration JSON can override axis codes, minimum/center/maximum, inversion,
and thresholds under `thumbsticks.left` and `thumbsticks.right`. Existing mapping
files without `thumbsticks` leave this feature disabled; use automatic detection
or export a fresh calibration. Inspector/replay show stick positions and the
captured keyboard modifiers. Visual references include both directional maps.

## Create a visual layout reference

```sh
cargo run -- render --layout layout.json --output layout.html
```

Open `layout.html` in a browser. The offline reference shows each alpha, number,
navigation, editing/media, and function bank, with mirrored thumbstick maps.
Use your browser's Print / Save PDF command for a one-page A4 portrait reference.
The checked-in [PDF](../layout.pdf) has the same layout.

Legacy independent layouts render as searchable eight-way maps, with layer
filters and a Print / Save PDF button. Their search highlights exact characters,
action names, and shortcuts.

The document uses the validated bindings. Existing HTML output is updated
atomically; invalid layouts leave it intact. Output cannot resolve to the input
layout, including through symlinks.

## Inspect and test a controller

List readable gamepads, then use the event path shown by `devices`:

```sh
cargo run -- devices
cargo run -- inspect --device /dev/input/event28 --layout layout.json
```

Prefer a stable `/dev/input/by-id/*-event-joystick` path when available. Device
numbers can change after reconnecting. Read access to the selected event node is
required; discovery reports unreadable nodes. The inspector opens the device
read-only, leaves it available to other applications, and displays controls and
recognized actions. It does not create a virtual keyboard or type into apps.

The display includes the D-pad, buttons, trigger travel, and recognition state.
Release all controls to arm it. Form a chord, then release its ordinary controls
to commit it once. L2/R2 may remain held between characters. The default layout
allows a bumper to join the newest held letter within 100 ms. Each thumb press
starts its own stroke. Legacy independent layouts retain their diagonal and
chord-cancellation rules.

`--layout` uses the generated layout's roles, physical constraints, and action
bindings. Without a layout, L2 is Shift and R2 is a held layer, and chords are
displayed without character assignments. `--duration 10` stops after ten seconds;
otherwise Ctrl-C stops the session. `--json` emits machine-readable views.

Standard Linux mappings use physical face-button positions (A south, B east,
X west, Y north). Automatic detection identifies the kernel driver through sysfs.
For [`xpad`](https://github.com/torvalds/linux/blob/master/drivers/input/joystick/xpad.c)
controllers, Xbox X/Y use the driver's [legacy codes](https://github.com/torvalds/linux/blob/master/include/uapi/linux/input-event-codes.h)
(307/308), which reverse the generic geometric mapping. `devices --json` includes the detected
driver. Explicit calibration files keep their supplied button codes.
Start/Select are detected when the controller advertises their standard Linux
codes (315/314). To enable them in an older saved calibration, add
`"menu_buttons": {"Start": 315, "Select": 314}` or export a fresh calibration.
This optional map also accepts L3/R3, custom event codes, or an empty object to disable
these buttons; recordings without it retain their original behavior.
Hat-axis D-pad reports take precedence over duplicate button reports. Digital D-pads and trigger buttons are also supported. Controllers must
expose the standard face, shoulder, and D-pad capabilities. Keyboard devices
advertising gamepad button codes are excluded from discovery. A calibration file
can override individual raw event codes for device-specific mappings.

### Calibrate L2/R2

```sh
cargo run -- calibrate --device /dev/input/event28 --output controller.json
cargo run -- inspect --device /dev/input/event28 \
  --layout layout.json --calibration controller.json
```

Calibration first samples two seconds at rest, then asks you to fully press and
release both triggers for eight seconds. It saves measured endpoints and the
controller's vendor/product identity. Movement during the rest phase or
insufficient travel rejects the calibration. `--rest-seconds` and `--sweep-seconds`
change those intervals. Existing output files are preserved.

Without a calibration file, trigger endpoints come from the driver's advertised
range. Automatic mapping prefers ABS_BRAKE/ABS_GAS over ABS_Z/ABS_RZ when both
pairs are advertised, as on the GameSir Pocket; Z/RZ can describe the right stick.
Press activates at 65% travel and release deactivates at 35%, so small
movements around one threshold do not repeatedly toggle a modifier. Calibration
also supports reversed analog axes. Edit the saved threshold fractions or event
codes to adjust a controller's mapping; `--calibration` can reuse that mapping
for another calibration run.

Calibration is stored separately from generator costs and layout bindings. It
measures physical trigger endpoints; it does not claim to measure chord comfort
or typing speed.

### Record and replay

```sh
cargo run -- inspect --device /dev/input/event28 --layout layout.json \
  --calibration controller.json --record session.jsonl
cargo run -- replay --input session.jsonl
cargo run -- replay --input examples/recordings/chords.jsonl --json
```

Recordings contain a versioned header with the mapping and optional layout,
followed by timestamped complete input reports, resets, and an end marker. Replay
uses the same normalizer and chord engine without hardware or timing delays.
It reproduces the corresponding live views. Recording requires a new file and
captures only the selected controller, during this explicit session.

Dropped input reports discard the pending chord and resynchronize from the
device's current state. Startup and resynchronization require a neutral state
before accepting strokes. Disconnect or stopping discards pending strokes and
ends the recording; reconnecting requires starting a new inspector session.
Incomplete, malformed, out-of-order, or unsupported recordings are rejected.

The synthetic recordings under [examples/recordings](../examples/recordings) cover
staggered chords, held layers and Shift, analog trigger hysteresis, cancellation,
and resynchronization. They contain no captured personal input.
