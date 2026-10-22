# padtype

Gamepad typing for Linux, with a layout generator that searches for comfortable,
efficient chords.

Inputs are the D-pad, A/B/X/Y, L1/R1, and L2/R2. Direct typing chords use the
D-pad and A/B/X/Y/L1/R1 by default. The analog L2/R2 triggers are reserved for
held Shift or layers. The generator chooses combinations within those roles.

The layout generator, controller inspector, and Linux virtual-keyboard runtime
are implemented. Desktop editor and terminal testing is still pending.

## Generate a layout

With Rust and Cargo available, run:

```sh
cargo run --release -- generate
```

On Nix, enter `nix-shell` first to get the compiler and development tools.

The default command uses a small demonstration corpus, compares seven modifier
strategies, and writes `layout.json` plus `layout.report.json`. Use representative
text for a useful result:

```sh
cargo run --release -- generate \
  --corpus writing.txt \
  --held-out other-writing.txt \
  --seed 42 \
  --iterations 20000 \
  --restarts 3
```

Layout and report output paths must differ and cannot resolve to a corpus,
held-out, or configuration input file, including through symlinks.

The default alphabet covers printable ASCII, Enter, Tab, Backspace, Escape, and
any additional characters in the training corpus. `--alphabet corpus` limits
character coverage to the training text; `--include '0123456789'` adds specific
characters. Evaluation fails on uncovered characters instead of ignoring them.
Pinned actions are also included in the required outputs.

Case, whitespace, and Unicode characters are preserved. CRLF is normalized to LF.
Unicode is supported as a generator output symbol. The initial typing backend
supports printable ASCII, Enter and Tab; it rejects unsupported characters before
opening a controller or virtual keyboard.

## Configure the search

```sh
cargo run -- defaults > config.json
cargo run --release -- generate --config config.json --corpus writing.txt
```

Edit the configuration to change estimated button costs, scoring weights,
diagonals, disabled or modifier-only buttons, maximum chord size, forbidden
combinations, or pinned bindings. Nested sections use defaults for omitted fields.
If overriding the `buttons` cost map, provide all eight button costs. Unknown
fields are rejected.

[examples/comfortable.json](examples/comfortable.json) disables diagonals,
forbids two opposite-face combinations, and pins Space to A. Those restrictions
are examples, not defaults. A forbidden button combination also forbids its
supersets, including held modifiers. `forbidden_chords` excludes an exact physical
combination, with held modifiers included in its button list. Reserve the runtime's
pause gesture here so it cannot be assigned to a character.

By default, the search compares a direct-chord baseline without triggers, either
trigger as Shift, either trigger as a held layer, and both Shift/layer trigger
assignments. An unassigned trigger stays unused. Custom strategies can assign
those roles to other buttons.
`--strategy direct` restricts the search to one named strategy; repeat the flag
to choose several. Infeasible strategies are reported and skipped. A configuration
with no feasible strategy fails without exporting a layout.

`profile.modifier_only_buttons` defaults to `["L2", "R2"]`: these controls can
be held as modifiers but never appear in ordinary typing chords. Unlike
`disabled_buttons`, this setting permits Shift/layer use. Set it to `[]` to
allow all eight buttons as ordinary chord members in strategies that leave them
unassigned. This restores the full 2,303-combination search space when diagonals
are enabled; the default direct baseline has 575 nominal combinations.

L1/R1/L2/R2 are distinct controls. In a modifier strategy, the assigned button
is reserved for that role. Held Shift shares ordinary chords between standard US
character pairs, such as `a`/`A` and `1`/`!`. A held layer adds a second assignment
bank. Modifiers can stay held across characters; ordinary chord controls must
fully release between them.

## Understand the results

The search uses simulated annealing with swaps and replacement chords, beginning
with a frequency-based baseline. Every legal chord remains eligible. It retains
the baseline if it finds no improvement and keeps pinned assignments fixed.

All strategies use the same normalization and weights. Lower objective is better:

| Term | Meaning |
| --- | --- |
| Effort | Estimated button, D-pad, simultaneous-control, and held-modifier effort |
| Activations | Ordinary controls pressed per character, plus modifier entries |
| Movement | Estimated thumb repositioning between consecutive chords |
| Coverage | Uniform access cost over all required outputs, including unobserved ones |
| Learning | Canonical character/key groups plus a penalty for modifier rules |

A diagonal counts as two direction activations and two controls for chord-size
limits. Activations count presses; ordinary release requirements are part of the
input model, and modifier releases have an effort cost. Each corpus file is one
continuous task with modifier entry at the start and cleanup at the end.

Movement uses D-pad coordinates and the centroid of pressed face buttons, with
Xbox-style positions: A south, B east, X west, Y north. The learning term gives
Shift pairs a shared rule; it is an estimated learning proxy, not a measurement.
Backspace and Escape are covered but have no usage frequency in plain text.

The report records baseline and optimized scores, held-out scores, all candidate
layouts, costs, configuration, seeds, budgets, corpus hashes, and dependency lock
hash. Selection uses training scores only. Repeat a command with the same inputs,
configuration, seed, and locked dependencies to reproduce it. The winning layout
embeds its profile, weights, recognition policy, and normalization for rescoring.

```sh
cargo run --release -- score --layout layout.json --corpus other-writing.txt
```

Costs are estimates. Results are candidates for controller testing, not measured
typing speeds or a guarantee of global optimality. This first version has one
binding per output, systematic US Shift pairs, and at most one held layer. It
does not yet search one-shot/persistent modes, multiple access paths per output,
or explicit shortcut/navigation tasks.

## Type on Linux

Build the app and start it with your layout and a readable gamepad event path:

```sh
cargo build --release
target/release/padtype type --device /dev/input/event28 --layout layout.json
```

It starts paused. Press **L1 + R1 + L2 + R2**, then release all controls to enable
typing. Repeat that gesture to pause. Each action commits when its ordinary
controls are released; triggers assigned to Shift or layer can stay held.
Ctrl-C or SIGTERM stops the process. It does not record normal typing.

Test the runtime without sending keyboard events:

```sh
target/release/padtype type --device /dev/input/event28 \
  --layout layout.json --dry-run --start-active --duration 30
```

`--dry-run` prints `WOULD TYPE` actions and never opens uinput. `--start-active`
skips the initial pause, while still requiring neutral controls before typing.
`--calibration controller.json` uses saved trigger settings. `--grab` requests
exclusive controller access; the default leaves the gamepad available to other
applications. Device loss ends the session, discards a pending stroke, and
releases keyboard keys. Restart the command after reconnecting.

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

For new layouts, reserve the gesture in the generator's profile. The example
also pins Ctrl+C to L1+X and Ctrl+V to L1+B:

```sh
cargo run --release -- generate --config examples/typing.json --corpus writing.txt
```

[examples/typing.json](examples/typing.json) uses
`"forbidden_combinations": [["L1", "R1", "L2", "R2"]]` to reserve the default
gesture. Existing layouts work if none of their bindings use that combination.
Generating with a different pause gesture requires reserving that set instead.

Shortcut actions use physical keyboard keys, for example:

```json
{"type":"shortcut","value":{"modifiers":["ctrl","shift"],"key":"c"}}
```

Modifiers must be nonempty, unique, and ordered `ctrl`, `alt`, `shift`, `meta`.
Supported keys include letters `a` through `z`, Backspace, Escape, Enter, Tab,
Space, Delete, Insert, arrows, Home/End, Page Up/Down, and F1 through F12. Multiword
key names use snake case (`page_up`). A key action uses the same names, such as
`{"type":"key","value":"left"}`. Pin shortcuts/navigation actions in the
generator configuration to include them in a generated layout. Plain text
corpora do not measure their usage frequency.

### Runtime validation

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo test --locked --test typing -- --ignored --nocapture --test-threads=1
```

The last command requires uinput and input-node permissions. It captures its own
virtual keyboard exclusively before emitting events, then checks a dry-run
typing subprocess using an exclusively grabbed virtual gamepad. These checks
cover real evdev/uinput events without typing into desktop applications.

Kernel-synthesized release reports and forced report splits cancel pending
strokes and refresh controller state. The input core emits these boundaries
during [device teardown and reset](https://github.com/torvalds/linux/blob/master/drivers/input/input.c);
they must not commit a typing chord.

The Kishi Ultra has passed a live inspector test for all six digital buttons,
four cardinal D-pad directions, full-range triggers, a three-button chord, and
held Shift/layer lookup. Its live recording replayed identically. Runtime dry-run
startup, signal pause/resume, and SIGTERM shutdown also passed. Editor/terminal
typing and Wayland/X11 session compatibility still need hands-on verification.

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
X west, Y north). Hat-axis D-pad reports take precedence over duplicate button
reports. Digital D-pads and trigger buttons are also supported. Controllers must
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

The synthetic recordings under [examples/recordings](examples/recordings) cover
staggered chords, held layers and Shift, analog trigger hysteresis, cancellation,
and resynchronization. They contain no captured personal input.

## Development

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

See [PLAN.md](PLAN.md) for chord recognition rules and the remaining milestones.
