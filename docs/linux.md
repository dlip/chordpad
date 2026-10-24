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

It starts paused. Press **L1 + R1 + L2 + R2**, then release all controls to enable
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
including temporary keyboard modifiers. General Unicode, other keyboard layouts,
automatic key repeat, and persistent/one-shot modes are not implemented.

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
Keyboard events press and release the modifiers around each committed action,
so holding a stick alone does not hold an OS modifier down between keystrokes.
Pause, disconnect, and dropped-input resets discard pending actions and require
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

Open `layout.html` in a browser. It is a self-contained document with 8-way
D-pad and face-button maps, so it works offline without a server, external fonts,
or images. Base and Shift share one set of maps: pairs such as `m / M` and
`, / <` show the normal output followed by its shifted output. Numbers and
shifted symbols share another set. Navigation and function keys have their own
maps and layer filters.

Shoulder maps show chords with L1/R1, with the shoulder alone in the center.
The maps use the layout's actual bindings. Hover over a cell for its full chord
instructions. Search also finds combinations outside the maps, including
opposite face buttons and chords with more than two face buttons, and shows
their instructions beside the result count.

Search for a literal character to find its exact chord: `m` and `M` select the
same paired chord, while a literal space selects Space. Action names such as
`Backspace`, `F11`, and `Ctrl + C` also work. Search highlights matching map cells;
select a layer filter to narrow the results. Maps also work without JavaScript.

Use **Print / Save PDF** to produce compact reference sheets. Printing includes
all layer maps even if the screen is filtered and omits the text chord
references. Layers share pages on A4 portrait paper; browser print settings can
change paper size or scale.

Re-run the command after generating a new layout. The document is derived from
the validated bindings and records their strategy name. Existing HTML output is
updated atomically; invalid layouts leave it intact. The output path cannot
resolve to the input layout, including through symlinks.

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
to commit it once. L2/R2 may remain held between characters. New modifier presses
during a stroke or new ordinary presses during release cancel that stroke.
Diagonals freeze when either direction releases, so Up+Left → Up → neutral
still commits Up+Left.

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
This optional map also accepts custom event codes or an empty object to disable
both buttons; recordings without it retain their original behavior.
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
range. Press activates at 65% travel and release deactivates at 35%, so small
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
