# Project plan

## Scope

Build a Linux typing application and a layout generator sharing the same chord
and action model. Optimize for few, comfortable actions per character, including
the cost of Shift, layers, and transitions between characters.

Supported controls are the D-pad, A/B/X/Y, L1/R1, and L2/R2. All physically
achievable combinations are in scope when enabled by the profile. Analog L2/R2
are modifier-only by default; ordinary typing uses A/B/X/Y/L1/R1 and the D-pad.
Do not impose a universal two- or three-button limit. Allow individual users to
forbid uncomfortable combinations or set a chord-size limit.

The search finds the best candidates under a stated model and budget. Measured
typing performance determines whether a candidate is an improvement; a heuristic
result is not a proof of global optimality.

## Initial assumptions

- Linux first, using Rust and evdev/uinput, with editor and terminal checks under
  Wayland and X11.
- A configured US keyboard layout for initial character output. Other keyboard
  layouts and general Unicode input require explicit output-backend support.
- English prose is the provisional default corpus. Corpus selection and weights
  are configurable, including programming text and shortcut tasks.
- D-pad positions are neutral, cardinal directions, and diagonals when enabled
  by the controller profile. Opposite directions are not valid positions.
- L2/R2 are analog controls, reserved for held Shift or layers by default.
  Unassigned triggers remain unused. Ordinary trigger chords require an explicit
  controller-profile opt-in.

## Controller model

Normalize physical controls before chord recognition. Store face buttons by
physical position internally; show the controller's A/B/X/Y labels in the UI.
Handle D-pads reported as buttons or hat axes without counting both reports.

Represent a chord as a D-pad position and a set of ordinary buttons. With all
eight buttons ordinary and all nine D-pad positions enabled, there are
`9 * 2^8 - 1 = 2303` nonempty nominal combinations. Filter these against the
controller profile before searching. This count describes available input
combinations, not usable or desirable bindings.

With L2/R2 reserved for modifiers, the direct baseline instead has
`9 * 2^6 - 1 = 575` nominal combinations. Modifier-only controls remain available
for held roles; disabled controls cannot be used in either role.

For analog L2/R2, use calibrated press and release thresholds with hysteresis.
Button mappings, thresholds, disabled diagonals, and forbidden combinations
belong to the controller profile, separate from character assignments.

Before accepting a layout, require a calibration exercise that checks its used
combinations on the actual controller. Start with a manually editable feasibility
profile; calibration can refine it over time.

## Chord boundaries

The first recognizer uses release-to-commit and requires ordinary chord controls
to return to neutral between strokes. Held modifier controls are exempt from that
neutral requirement. This intentionally simple baseline can be measured before
introducing rollover.

Process complete input reports before interpreting state changes, so event order
inside one hardware report does not change the recognized chord.

| State | Behavior |
| --- | --- |
| Idle | Wait for an ordinary button press or D-pad departure from neutral. Snapshot the active layer and Shift state when the stroke starts. |
| Collecting | Accept additional ordinary presses. Buttons must overlap before the first release. Use the current D-pad position, allowing movement into a diagonal if enabled. |
| Releasing | On the first ordinary button or D-pad direction release, freeze the preceding chord. Wait for all ordinary controls to become neutral. |
| Commit | Emit the frozen binding exactly once, then return to Idle. |
| Cancelled | Emit nothing and wait for ordinary controls to become neutral. |

A new ordinary press during Releasing cancels the stroke. Invalid D-pad states,
forbidden combinations, and unsupported transitions cancel it too. An unmapped
chord produces feedback and no keyboard output. A D-pad change directly between
opposite directions cancels the stroke.

Press order before the first release does not matter. Release order does not
change the frozen chord. There is no fixed chord-collection timeout in the first
version; analog threshold filtering is separate from chord timing.

A diagonal freezes when either component direction releases. For example,
Up+Left → Up → neutral commits Up+Left. Adding another control after that first
direction release cancels the stroke, just as adding a button during release does.

For example, if both `A` and `L1+A` are mapped, pressing A produces no output yet.
Adding L1 before either button is released selects `L1+A`. The binding emits when
both buttons have been released. A alone emits after A is released.

Releasing all ordinary controls is required for repeated letters as well. Holding
a chord does not automatically repeat it. Backspace and navigation repeat need
an explicit later policy, tested separately from chord collection.

## Shift and layers

Each candidate layout assigns a fixed role to every button: ordinary chord
member or held modifier. A modifier can select a layer or Shift state. Its role
must not change during a stroke. A button used as a held modifier is unavailable
as an ordinary chord member in that candidate.

Held modifiers must be active before the stroke starts, or in its first complete
input report. Releasing one during the stroke leaves the snapshotted context in
effect for that stroke. Pressing a new modifier during an active stroke cancels
the stroke. This prevents a late layer change from silently changing its output.

Start by comparing direct chords on A/B/X/Y/L1/R1 and the D-pad, L2 as Shift and
R2 as a layer, the reversed assignment, and each trigger assigned individually.
Make the candidate role assignments configurable. Triggers do not participate in
ordinary chords unless the profile explicitly removes their modifier-only role.

Explicit mapped actions can also set one-shot Shift or select a persistent mode.
Every persistent mode must have a validated path back to the base mode. Defer
more elaborate modifier tap/hold behavior until the baseline has been measured.

Compare direct access, held layers, and one-shot actions using the full input
sequence. Include entering and leaving a layer; repeated characters on a held
layer do not pay the entry cost again.

## Output and runtime behavior

Bindings emit character actions, keyboard keys/shortcuts, or internal mode
actions. Keep character intent separate from keyboard keycodes so output support
can expand beyond the initial configured US layout.

Create a virtual keyboard through uinput. Emit balanced press/release sequences;
internal gamepad modifiers do not leave keyboard modifiers held between actions.
Treat controller access and uinput access as explicit installation requirements.
Keep the ordinary application process unprivileged with suitable device access.

On disconnect, input resynchronization, pause, or layout replacement, discard
pending strokes, clear internal modifiers and modes, and release any emitted keys
still held. Resume only after controls return to neutral. Detect device loss and
exclude the application's virtual keyboard from controller discovery.

Provide a pause command outside the generated layout and a configurable reserved
controller gesture. The reserved gesture must consume its input without also
typing a character and must be unavailable to generated bindings. Do not enable
exclusive device grabbing by default; expose it for users who need it.

## Optimization model

Search both which chords to use and where characters/actions are assigned. Keep
user-pinned assignments fixed. Hard constraints cover required outputs, controller
feasibility, reserved gestures, and reachability of every mode.

For a corpus of N target characters, evaluate:

```text
J = wt * normalized(total predicted time / N)
  + wp * normalized(total button/direction activations / N)
  + we * normalized(total ergonomic effort / N)
  + wr * normalized(predicted correction cost / N)
  + wm * normalized(learning complexity)
```

Normalize against a recorded baseline so the terms have comparable scales.
Record the weights and normalization values in every result. They are tunable
preferences, not universal constants.

Time estimates cover chord formation, release, and movement from the preceding
chord. Transition terms must not count effort already included in chord formation
twice. Ergonomic effort covers simultaneous load, thumb movement, shoulder use,
and hand balance. Penalize large chords without excluding them automatically.

Learning complexity counts bindings in context and extra mode rules. The same
physical chord with different meanings on different layers adds learning cost.
Initially use explicit estimated costs. Only label error and timing estimates as
measured when calibration data supports them; show unmeasured terms separately.

If a character has multiple bindings, score a valid sequence through the layout's
states, not its cheapest chord in isolation. Use sequence evaluation that tracks
the previous chord, held modifiers, and persistent/one-shot mode state. Include
mode cleanup at task boundaries and model all presses and releases. Represent
shortcuts/navigation as explicit tasks alongside text, with separate weights.

Start with a frequency-based baseline and simulated annealing with multiple
seeds. Compare candidate modifier configurations under the same search budget.
Mutation operations include reassignment, swapping bindings, and changing
direct/layer access while preserving required outputs and pinned assignments.

Evaluate on held-out text and report a small set of candidates showing efficiency,
comfort, and learning tradeoffs. Keep seeds, corpus hashes, controller profiles,
cost-model versions, search budgets, and baseline scores for reproducibility.
Never automatically replace an active layout after optimization.

## Shared data and architecture

Use a versioned layout format containing button roles, layers, chord bindings,
internal actions, output assumptions, and recognition settings. Validate layouts
before activation. Store controller calibration and optimizer run metadata
separately, referencing the layout and profile versions they used.

```text
physical controller -> normalization -> chord engine -> layout -> keyboard output
                                            |
                                         feedback

corpus + controller profile + cost model -> generator -> candidate layouts
practice session -> timing/error measurements -> cost-model calibration
```

The runtime, inspector, trainer, and optimizer share the same chord semantics and
layout validation. The first interface is a CLI plus live terminal feedback.
Add a graphical trainer after the input model works. Practice logging is explicit
and local; ordinary typing is not collected as training data.

## Milestones

1. Layout generator. Produce reproducible, valid candidates from a corpus and
   controller profile. Report comparisons against the baseline on training and
   held-out text, including input effort and learning complexity. The first
   implementation searches direct chords, US Shift pairs, and one held layer.
   Timing/error calibration, one-shot/persistent modes, multiple bindings per
   output, and explicit shortcut/navigation tasks remain follow-up work.
2. Controller inspector and chord tester. Identify all required controls, calibrate
   triggers, display live input and recognized chords, and replay recorded input
   traces. Complete when press/release order, diagonals, nested chords, held
   modifiers, cancellation, disconnect, and resynchronization behave as specified.
3. Linux typing baseline. Load a generated layout and type letters, corrections,
   capitals, numbers, symbols, and shortcuts in an editor and terminal. Verify
   pause/resume and device loss without stuck keyboard keys, and test both Wayland
   and X11 sessions before claiming support for both.
4. Trainer and calibration. Run explicit practice tasks, measure corrected typing
   speed and accuracy, and use the results to revise predicted costs. Compare
   candidates after a stated practice period instead of comparing a familiar
   layout with an unfamiliar one without qualification.
5. Packaging and daily use. Add installation/device-access instructions, a user
   service, layout switching, and a readable active-layout display. Document
   controller and desktop combinations that have actually been tested.

## Implementation status

The generator, controller inspector/chord tester, and Linux typing runtime are
implemented. The visual exporter produces a self-contained HTML reference with
SVG controller diagrams, base-control annotations, chord cards grouped by held
state, exact character/action search, and full-size chord previews. It prints all
bindings regardless of screen filters, with separate modifier-state sections.
The document uses the same layout validation as the runtime and generator.

The inspector provides device discovery, read-only evdev input, trigger endpoint
calibration with hysteresis, release-to-commit recognition, optional generated
layout lookup, JSONL recording, and deterministic replay. Replay fixtures and
tests cover press/release order, diagonals, held modifiers, cancellation,
disconnect/reset behavior, and dropped-report boundaries. A live Kishi Ultra
session verified the six digital buttons, cardinal D-pad directions, full trigger
travel, a three-button chord, held layer/Shift lookup, and identical replay.
The endpoint-calibration wizard and live diagonal/chord feasibility exercise
still need a coordinated hardware test.

The typing runtime sends printable ASCII, Enter/Tab, keyboard keys, and explicit
shortcuts through uinput using balanced press/release sequences. It starts paused,
offers dry-run mode and optional exclusive controller access, and supports a
configurable reserved toggle gesture plus SIGUSR1/SIGUSR2 pause/resume. Runtime
validation rejects gesture collisions; generator profiles can reserve the same
set with forbidden_combinations. Normal typing is not recorded. Tests cover
modifier cleanup, partial output failures, pending-stroke cancellation, and
neutral requirements after pause/reset. Linux integration checks passed with an
exclusively captured virtual keyboard (101 actions, no held keys) and an
exclusively grabbed virtual gamepad (signals, toggle gesture, shifted actions,
and disconnect during a stroke). The Kishi also passed runtime dry-run startup,
signal pause/resume, and SIGTERM shutdown. These checks do not validate desktop
text entry.

The next validation is typing and correcting text in an editor and terminal,
including capitals, numbers, symbols and shortcuts, and checking desktop behavior
under Wayland and X11 before claiming support for both. Trainer/calibration work
follows that hands-on baseline.
