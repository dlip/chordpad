# Layout generation

Use the bundled [layout.json](../layout.json) to start typing without generating
anything. This guide covers creating and evaluating custom layouts. Run commands
from the repository root with Rust and Cargo installed; on Nix, enter
`nix-shell` first.

The bundled layout has 97 bindings and uses the independent-halves preset,
trained on the Leipzig English web corpus described below. Generating a layout
writes to `layout.json` by default; use `--output custom-layout.json` to keep
the bundled layout.

## Generate a layout

With Rust and Cargo available, run:

```sh
cargo run --release -- generate
```

On Nix, enter `nix-shell` first to get the compiler and development tools.

The default command uses a small demonstration corpus, compares seven modifier
strategies with 1,000 annealing proposals per restart and three restarts, then
runs the systematic finishing passes. It writes `layout.json` plus
`layout.report.json`. Use representative text for a useful result:

```sh
cargo run --release -- generate \
  --corpus writing.txt \
  --held-out other-writing.txt \
  --seed 42 \
  --iterations 1000 \
  --restarts 3
```

For a larger general-English baseline, prepare the Leipzig Corpora Collection's
2014 English web sample (10,000 sentences):

```sh
python3 scripts/prepare_english_corpus.py
cargo run --release -- generate --config examples/independent.json \
  --corpus corpora/english-web/train.txt \
  --held-out corpora/english-web/held-out.txt \
  --output layout.english.json
cargo run -- render --layout layout.english.json --output layout.english.html
```

The prepared training split has 8,926 sentences / 1,067,910 characters; the
held-out split has 917 sentences / 108,574 characters. A checksum pins the
[source archive](https://downloads.wortschatz-leipzig.de/corpora/eng_web_2014_10K.tar.gz).
The script removes sentence IDs, maps curly quotes and dashes to ASCII, applies
Unicode NFKC, and excludes 157 sentences with remaining unsupported characters.
Case, spaces, numbers, and ASCII punctuation are retained. Normalized sentences
are deduplicated and assigned by a stable sentence hash, so the splits do not
overlap. `manifest.json` records normalization, source IDs, hashes, and provider
[terms](https://wortschatz.uni-leipzig.de/en/usage). Downloaded and prepared data
stays in the ignored `corpora/` directory; the helper requires only Python's
standard library. An existing archive can be supplied with `--archive PATH`.

This is web text from 2014, with independent sentences rather than complete
documents. Each sentence becomes one line; n-grams crossing those line boundaries
approximate unrelated sentence transitions. It supplies actual text for the full
Shift, grip, and thumb-return model, without reconstructing text from frequency
tables. Preparing it does not change the checked-in default layout.

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

[examples/comfortable.json](../examples/comfortable.json) disables diagonals,
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

## Independent halves, Shift, and numbers/symbols

```sh
cargo run --release -- generate \
  --independent \
  --exclude-opposite-face-pairs \
  --same-hand-penalty 1 \
  --output independent.json
cargo run -- render --layout independent.json --output independent.html
```

[examples/independent.json](../examples/independent.json) pins `a` to A, `o` to B,
`l` to D-pad Left + L1, `u` to Y + R1, `d` to D-pad Down + L1, and `r` to
D-pad Right (→). L2 adds uppercase letters. It also pins punctuation, numbers,
symbols, and Tab, and enables finger and thumb comfort costs. Use it with
`--config examples/independent.json` and your own corpus.

Regenerate the checked-in default and refresh its reference:

```sh
python3 scripts/prepare_english_corpus.py
cargo run --release -- generate --config examples/independent.json \
  --corpus corpora/english-web/train.txt \
  --held-out corpora/english-web/held-out.txt
cargo run -- render --layout layout.json --output layout.html
```

Copy `layout.json` to `android/app/src/main/assets/default-layout.json` when
updating the Android preset.

The preset uses `profile.costs.same_finger_transition: 1.0` instead of a whole-hand
reuse penalty. D-pad inputs use the left thumb and face buttons use the right.
On each side, either shoulder alone uses the index finger. With both L1+L2 or
R1+R2 active, the index presses L1/R1 and the middle presses L2/R2. Held Shift and
layer triggers affect this assignment, but holding a modifier across strokes does
not count as repeatedly pressing it. For example, a face button followed by R1
Space reuses no finger, while two face-button strokes reuse the right thumb.
Each reused finger adds the configured penalty. The older `same_hand_transition`
setting remains available as a separate, additive preference.

`profile.costs.grip_change` charges each middle-finger recruitment from controller
support and its return. The preset uses `0.7`. Keeping both shoulders active across
consecutive strokes keeps the middle recruited without another change penalty;
entering or leaving that grip costs once per side, including the start and end
of the text. The existing `same_side_shoulders` cost still accounts for the load
while both shoulders are active. Both new finger costs default to zero when omitted.

`profile.costs.skipgram_weights` adds movement costs for a thumb's next input
across one, two, or three intervening characters that do not use that thumb.
The preset uses `[0.5, 0.25, 0.125]`: more intervening inputs give more time to
reposition, so their penalties are smaller. For example, D-pad Up → face button →
D-pad Down adds half the Up-to-Down distance to the movement metric. A same-thumb
input in the middle prevents that longer skipgram from being counted. These costs
add to adjacent movement; they do not add presses, modifier changes, or hand-reuse
penalties. Both thumbs are scored separately, including when Shift/layers are held.

The preset sets `profile.costs.diagonal_extra` to `2.0`, making L1 plus a cardinal
direction cheaper in the per-chord effort model than any bare D-pad diagonal,
with or without L2 Shift. Base effort is `2.28` for L1 + a cardinal, `3.18` for
↖/↘, and `3.78` for ↗/↙. With Shift, those costs are `3.73`, `3.93`, and `4.53`.
Both chord types count as two activations. Movement, finger reuse, and grip
changes still affect the total score across a sequence.

`profile.costs.awkward_diagonal_extra` adds effort to inner-up and outer-down
positions. The preset uses `0.6`. On the left these are D-pad ↗/↙; on the right
they are X+Y and A+B. Inner-down/outer-up (D-pad ↘/↖ and A+X/Y+B) retain their
usual costs. The face-pair penalty also applies when a shoulder is part of the
chord. All values are adjustable estimates. Both new settings default to zero
when omitted, preserving existing layout scores and bindings. Generate a new
layout to apply them; rendering or running an existing layout does not rearrange it.

This preset enforces these rules during generation and layout validation:

| Control | Role |
| --- | --- |
| D-pad, A/B/X/Y, L1/R1 combinations | Digital typing chords; all letters use the base bank |
| R1 alone | Space; with R2, hold the function layer when enabled |
| L1 alone | Backspace; with R2, hold the navigation layer when enabled |
| Start | Enter in every layer |
| Select | Escape in every layer |
| L2 | Held Shift; uppercase uses the lowercase chord |
| R2 | Held numbers/symbols layer; combine with L2 for shifted symbols |

The example configuration enables `profile.navigation_layer`, a third, held layer:

| Hold R2 + L1, then press | Output |
| --- | --- |
| D-pad Up / Down / Left / Right | Arrow keys |
| Y / Triangle | Page Up |
| A / Cross | Page Down |
| X / Square | Home |
| B / Circle | End |

The bundled preset also enables `profile.navigation_media` for adjacent face pairs:

| Navigation face pair | Output |
| --- | --- |
| X + Y (Square + Triangle) | Previous track |
| Y + B (Triangle + Circle) | Next track |
| A + X (Cross + Square) | Delete |
| A + B (Cross + Circle) | Play/pause |

Add L2 for Shift selection or a stick modifier for modified navigation. D-pad
arrows fire on press and repeat after 400 ms, every 80 ms. Face taps send on full
chord release, so adding a second button does not leak a page key. Holding one
face button starts repeating after 400 ms; its release adds no extra output.
Adjacent pairs fire once on full release and never repeat. Delete supports L2
Shift and thumbstick modifiers; media controls ignore modifiers. Adding a button
after a single-button repeat cancels the remaining stroke. D-pad diagonals produce
no arrows.
Without `navigation_media`, older layouts retain immediate independent face keys.
Release either R2 or L1 to stop navigation; held navigation controls must release
before typing again. Entering navigation discards unfinished typing strokes, and
L1 does not send Backspace when used as the layer key.

R2 alone retains the numbers/symbols layer, including D-pad chords. Generation
reserves R2 + L1 and relocates symbols that would conflict with the navigation
modifier. Base letters and punctuation are unchanged in the bundled layout.
For a new layout, use `--independent --navigation-layer` or enable
`profile.navigation_layer` in its generation configuration. Existing layouts
need regeneration to relocate conflicting symbols; layouts without this option
retain their existing behavior.

The example pins numbers clockwise around the D-pad while R2 is held:

```text
8  1  2
7  ·  3
6  5  4
```

Cross/A is 9 and Circle/B is 0. L2 keeps the usual shifted number symbols on
the same combos. Square/X is `-`; Triangle/Y is `=`. Add L2 for `_` and `+`.
Cross+Square/A+X is `]` (L2 adds `}`). Base letters and punctuation are unchanged.

The example also enables `profile.function_layer`. Hold **R2 + R1** and use the
same combos: **1–9 → F1–F9**, **0 → F10**. Square/X is F11 and Triangle/Y is F12.
`profile.extra_function_chords` specifies the F11 and F12 combos, in that order;
the example uses `[{"buttons":["X"]},{"buttons":["Y"]}]`. They are reserved against
number placement. Older layouts without this field retain D-pad Left/Down for
F11/F12. Number combos avoid L1/R1 so they work with the held function prefix.
Function keys send once when the whole ordinary chord releases; they do not
repeat. The prefix can stay held for sequential function keys. L2 adds Shift,
and thumbstick modifiers apply. Releasing the prefix early preserves a function
stroke already started; R1 does not send Space as a layer modifier.

With `profile.function_volume` enabled (as in the bundled preset), R2 + R1 also
provides volume controls on adjacent face pairs:

| Function face pair | Output |
| --- | --- |
| X + Y (Square + Triangle) | Volume up |
| A + X (Cross + Square) | Volume down |
| A + B (Cross + Circle) | Mute/unmute |

Volume controls fire once on full chord release. Media and volume keys ignore
keyboard modifiers. These optional additions require their parent layers and
reserve their combos against numbers and F11/F12; incompatible custom bindings
are rejected. Older layouts without the two options retain their behavior.

With `profile.function_print_screen` enabled (as in the bundled preset),
Y + B (Triangle + Circle) sends Print Screen from the function layer. It sends
once on full chord release, supports keyboard modifiers, and reserves its combo
against numbers and F11/F12. Older layouts without this option keep their
existing function bindings.

Use `--independent --function-layer` or `profile.function_layer` for new layouts.
Generation includes all ten digits even with `--alphabet corpus` and derives
F1–F10 from their final positions. Regenerate layouts with R2 + R1 symbols to
move the conflicts. Changing between navigation and functions cancels unfinished
strokes; holding both L1 and R1 with R2 suppresses typing until one is released.
The normal L1 + R1 + L2 + R2 pause gesture still takes precedence. Navigation and
function tasks do not add separate frequencies to the text corpus score.

`profile.base_punctuation` permits selected unshifted ASCII punctuation on the
base bank, with its usual L2 Shift pair. Other numbers and symbols require R2.
The example configuration permits apostrophe, comma, period, semicolon, and
forward slash and pins:

| Base chord | Output | With L2 Shift |
| --- | --- | --- |
| R1 + A + B | `'` | `"` |
| R1 + A + X | `,` | `<` |
| R1 + B + Y | `.` | `>` |
| L1 + D-pad Up Left | `;` | `:` |
| R1 + X + Y | `/` | `?` |

Start and Select provide Enter and Escape. Independent generation uses these
dedicated controls without allocating typing chords. Explicit Enter/Escape pins
in older configurations remain supported.

Both triggers work with either hand and can stay held across sequential strokes,
including Space and Backspace. Hold modifiers before starting an ordinary stroke,
or in its first hardware report. Releasing a modifier early preserves the context
latched at stroke start. Adding or re-pressing a modifier during an active stroke
cancels that stroke. Space and Backspace also accept keyboard modifiers, such as
Ctrl+Backspace or Shift+Space.

The left half types with D-pad/L1; the right half types with A/B/X/Y/R1. The first
ordinary release freezes that half's chord; full ordinary release commits it.
The halves collect and release independently. Output follows stroke-start order,
even when a later stroke finishes first, including Start and Select. Same-report
starts are ordered left, right, Start, then Select. Attempting a 65th pending
stroke discards the queue and waits for all controls, including sticks and
triggers, to return to neutral.

`--exclude-opposite-face-pairs` rejects A+Y and B+X, including their supersets.
Adjacent pairs remain eligible. All three- and four-face-button combinations
contain an opposite pair and are excluded by this option. D-pad diagonals remain
available through `profile.diagonals`.

`--same-hand-penalty` sets `profile.costs.same_hand_transition` (default zero).
It charges consecutive outputs that share any occupied hand, including a hand
holding a trigger. Reports show `same_hand_transition_rate`, the fraction of
consecutive corpus pairs sharing a hand (zero for a one-character corpus).
This is a bigram preference without a timing or overlap speed simulation.
The text corpus does not model thumbstick shortcut tasks. Space and Backspace
are scored at their base bindings, so modifier-switch costs across them are
conservative estimates even though the runtime permits modifiers to stay held.

This preset is opt-in; the default search still compares the seven original
strategies. Existing version 1 layouts remain usable. New layout exports use
schema and cost model version 2. Pass the generated layout to `inspect --layout`
or `type --layout` to use independent typing.

## Understand the results

The search uses simulated annealing with swaps and replacement chords, beginning
with a frequency-based baseline. After each restart, it takes that run's best
layout and checks every legal move to an unused chord and every legal pair swap.
It applies the best improvement and repeats until none improves the score beyond
a relative `1e-12` numerical tolerance. These finishing passes are additional to
`--iterations`, which counts annealing proposals. Every legal chord remains
eligible, including chords outside the annealer's preferred shortlist. Pins stay
fixed. The best result across the finished runs wins; it retains the baseline if
it finds no improvement. This finds a local optimum for single moves and swaps,
without guaranteeing a global optimum.

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
Independent layouts score Start/Select as fixed controls with one activation and
unit effort. Menu-button movement uses the neutral thumb positions as an
estimate; these controls do not consume generated chord slots. Newlines in the
corpus are scored through Start when there is no explicit Enter chord.

The report records baseline and optimized scores, held-out scores, all candidate
layouts, costs, configuration, seeds, budgets, corpus hashes, and dependency lock
hash. Selection uses training scores only. Repeat a command with the same inputs,
configuration, seed, and locked dependencies to reproduce it. The winning layout
embeds its profile, weights, recognition policy, and normalization for rescoring.

```sh
cargo run --release -- score --layout layout.json --corpus other-writing.txt
```

Costs are estimates. Results are candidates for controller testing, not measured
typing speeds or a guarantee of global optimality. The search has one
binding per output. Legacy strategies use systematic US Shift pairs and at most
one held layer. Independent layouts constrain letters to base and numbers/symbols
to the R2 layer, with L2 Shift and configurable base punctuation.
It does not yet search one-shot/persistent modes, multiple access paths per output,
or explicit shortcut/navigation tasks.

## Pause gestures and shortcut bindings

For new layouts, reserve the gesture in the generator's profile. The example
also pins Ctrl+C to L1+X and Ctrl+V to L1+B:

```sh
cargo run --release -- generate --config examples/typing.json --corpus writing.txt
```

[examples/typing.json](../examples/typing.json) uses
`"forbidden_combinations": [["L1", "R1", "L2", "R2"]]` to reserve the default
gesture. Existing layouts work if none of their bindings use that combination.
Generating with a different pause gesture requires reserving that set instead.

Shortcut actions use physical keyboard keys, for example:

```json
{"type":"shortcut","value":{"modifiers":["ctrl","shift"],"key":"c"}}
```

Modifiers must be nonempty, unique, and ordered `ctrl`, `alt`, `shift`, `meta`.
Supported keys include letters `a` through `z`, Backspace, Escape, Enter, Tab,
Space, Delete, Insert, arrows, Home/End, Page Up/Down, F1 through F12, and Print Screen (`print_screen`). Multiword
key names use snake case (`page_up`). A key action uses the same names, such as
`{"type":"key","value":"left"}`. Pin shortcuts/navigation actions in the
generator configuration to include them in a generated layout. Plain text
corpora do not measure their usage frequency.

See the [Linux guide](linux.md) for typing, controller setup, and visual
references, or return to the [README](../README.md).
