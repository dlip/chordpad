# Four-bank layout: design and evaluation

Open the [portrait PDF](reference.pdf) or [offline HTML reference](reference.html),
including both mirrored thumbstick modifier maps.
Every character uses one D-pad cardinal or one face button. L1/R1 select four
alpha banks; R2 adds two fixed banks for numbers and other symbols. Letters and
symbols avoid diagonals, face pairs, and same-side shoulder holds. Hold R1+R2 for navigation; tap R2 twice for one-shot function keys.

The [JSON](layout.json) works with Linux typing and the Android app built from
this checkout. This layout is the default on Linux and Android; imported legacy layouts remain supported.
The root [layout JSON](../../layout.json), [HTML](../../layout.html), and
[PDF](../../layout.pdf) are the release references.

## Try on Linux

Build the app and find your controller:

```sh
cargo build --release --locked
target/release/chordpad devices
```

Use the event path shown by `devices`:

```sh
target/release/chordpad type --device /dev/input/eventN \
  --layout experiments/four-bank-alpha/layout.json
```

It starts paused. Press **A+B+X+Y**, then release all controls to start typing.
Repeat to pause; Ctrl-C stops it. The four-shoulder gesture is available for
Shift plus the second navigation bank, so this layout uses the four face buttons
for pausing by default. Optional `--dry-run --start-active` prints actions without
typing into applications. The existing Linux calibration, grab, pause signals,
and reconnection options work; see the [Linux guide](../../docs/linux.md).
Output assumes a US keyboard layout with Caps Lock off.

The `inspect`, `replay`, and `render` commands support this layout. The generic
`score` command uses the legacy search model; use the Python full-text scorer
below to evaluate the four-bank model.

Hold **L2 for Shift**, including text selection with navigation. An unused L2
tap of at most 250 ms arms Shift for the next key; a second unused tap cancels it. Core typing, Shift, navigation, and function keys
need no thumbsticks. For mirrored Ctrl/Alt/Meta directions without sticks, tap
an unused **R1+R2** pair for face-button modifiers or **L1+L2** for the D-pad.
Hold a direction, including adjacent pairs for diagonals, across keys or mouse
clicks; releasing the direction exits modifier mode. Repeat the tap to cancel.
Pairs used while typing retain their layers. Controllers with sticks also
provide held Ctrl/Alt/Meta directions. Click **L3 for Command+D** or **R3 for
Command+Shift+D** (Command = Meta / Super on Linux). Short click shortcuts send once on release in every layer and ignore held
modifiers and stick directions. Holding the click repeats the shortcut.
You can map L3's Command+D to **Dictate** and R3's Command+Shift+D to
**Cancel Dictation** in your computer's shortcut settings.

| Bank | ↑ | → | ↓ | ← | Y / △ | B / ○ | A / × | X / □ |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Base | s | r | n | t | i | o | a | e |
| L1 | b | m | d | l | u | p | y | g |
| R1 | w | f | v | c | x | j | k | h |
| L1 + R1 | / | q | z | Tab | ' | ; | , | . |
| R2 | 1 | 2 | 3 | 4 | = | 0 | 9 | - |
| R2 + L1 | 5 | 6 | 7 | 8 | [ | ] | Backslash | Backtick |

The pins preserve the original banks and thumb clusters: U on L1 + Y/Triangle,
D on L1 + Down, L on L1 + Left, R on base Right, and O on base B/Circle.

## Stroke collection

Hold a bank before pressing a thumb button, or add its bumper within **100 ms**
while the thumb button is still down. Only the newest stroke accepts late
bumpers. Starting another thumb stroke freezes every older stroke on either
hand. A stroke also freezes on its first release or at the 100 ms deadline.
Releasing a bumper preserves the captured bank. A bumper overlapping a thumb
stroke consumes its tap even if it arrived too late to change that stroke.

Each button press starts a separate stroke, including adjacent buttons rolling
on one thumb. Short strokes commit on release without waiting out the window. Hold a mapped
output for **300 ms** to start repeating it every **80 ms**, including letters,
numbers, function keys, navigation, media, Enter/Escape, and stick-click
shortcuts. Modifier and layer selectors stay held as selectors.

First outputs retain press order, so a completed stroke may wait for an earlier
one. Once the earlier stroke starts repeating, completed later strokes can
commit without waiting for its release. Multiple held outputs can repeat;
releasing each one stops it without an extra tap. Banks, modes, and modifiers
stay captured for the held key. Pause, reset, and disconnect stop all repeats.

Start a base left-thumb letter, start a right-thumb letter, then add L1 within
its window: only the right-thumb letter acquires L1. Adding L1 before the second
stroke starts can still change the first letter; the rule cannot infer intent.

The Linux recognizer is [alpha_banks.rs](../../src/alpha_banks.rs).
`StrokeCollector` in [alpha_bank_model.py](../../scripts/alpha_bank_model.py)
is the reference prototype for captured thumb banks, modes, and modifiers.
Tests cover both types of overlap, reversed release order, early bumper releases,
and the exact 99/100 ms boundary.

`keyboard_modifiers` mirrors the production stick map and adds L2 Shift.
The collector captures modifiers and modes at each stroke start. Early releases
preserve them for that stroke. `release_strokes` returns complete strokes;
`release_thumb` retains the simpler control/bank projection. Tests cover the
250 ms R2 tap boundary, used R2 presses, held Shift, and shifted navigation.

## Shift, symbols, and Space

Tap and release L2 within 250 ms for Shift on the next key; tap it again before typing to
cancel. The selection clears as that key starts, so rolling into another
letter leaves the second letter unshifted. Letters
become uppercase; symbols use standard US pairs, including digits, brackets,
apostrophe/double quote, and comma/less-than. The JSON records every shifted
character, and the R2 maps show shifted symbols in small type. Release L1 before
tapping L2, then reapply it if needed. The canonical gesture plan avoids holding
L1+L2 or R1+R2 together.

Hold R2 before a number stroke for momentary numbers, or tap it to keep numbers
selected for the next key. R2 taps cycle **numbers → functions → off** before
that key consumes the selection. R2 has no late-join window. Its two banks use
no R1 selector. D-pad directions ↑, →, ↓, ← give 1–4 on R2 and 5–8 on R2+L1.
A remains 9 and B remains 0 on R2. The recorded scores use one-shot Shift and
momentary number entry; they do not measure one-shot number entry.

Tap R1 alone for Space or L1 alone for Backspace. A thumb stroke consumes the
bumper's tap. Hold either bumper alone for **300 ms** without another control
press to start repeating its key, then repeat every **80 ms**. Once repeating,
that hold acts as Space or Backspace until release; later letters use the other
bank selectors. Joining another control before the 300 ms deadline cancels
repeat for the entire hold, even after that control is released. Keyboard
modifiers held before the bumper still apply. Release stops repeat without an
extra tap; pause, disconnect, and reset stop it too.

Space after an R1-bank character explicitly releases the used R1,
presses it again, then releases it. The scorer counts those events and adds a
0.35 rearming coordination cost. Start and Select retain Enter and Escape.

## Navigation and held modifiers

Hold **R1 + R2** before a thumb stroke for navigation. The prefix consumes R1's
Space tap even without a thumb stroke. Add L1 for the second bank:

| Hold | ↑ | → | ↓ | ← | Y / △ | B / ○ | A / × | X / □ |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| R1 + R2 | Up | Right | Down | Left | Page Up | End | Page Down | Home |
| R1 + R2 + L1 | Volume up | Next track | Volume down | Previous track | Print Screen | Stop | Play/pause | Delete |

Navigation and editing/media outputs all use the same 300 ms hold delay and
80 ms repeat interval as letters. Hold L2 for Shift selection: L2 + R1 + R2 +
D-pad Right sends Shift+Right. L2 also combines with function and editing keys.
Using held Shift consumes its tap; releasing it does not arm another Shift.
An unused tap arms Shift for one key; another tap before typing cancels it.
Using a held modifier does not rearm its one-shot selection.

Optional stick modifiers still combine with Shift. For example, hold a stick
outward for Ctrl, hold L2 for Shift, then use navigation for Ctrl+Shift+Arrow.
The [mirrored stick map](../../docs/linux.md#thumbstick-keyboard-modifiers),
including inward-up Hyper, is preserved. No stick click is required.

On Linux, holding L2 or a stick modifier presses its OS key immediately, so it
also works with your mouse: hold L2 for Shift-click or a Ctrl direction for
Ctrl-click. Releasing the control releases the OS modifier; typing keeps shared
held modifiers down. Pause, disconnect, reset, and exit release them too.
One-shot Shift still applies only to the next typed key. An unused L2 hold
longer than 250 ms does not arm it on release. Android supports Shift-click/tap selection in focused editors, including browser
editors. Its [optional ADB helper](../../android/README.md#global-mouse-modifiers)
adds system-wide mouse modifiers.

## Function keys

**Tap and release unused R2 twice for one-shot function keys; tap a third time
before typing to cancel.** Each tap lasts at most **250 ms**; there is no limit between
taps. The next key uses the selected layer and clears it. Used holds do not advance
the cycle. The 100 ms late-bumper window for letters is separate.

| After two R2 taps | ↑ | → | ↓ | ← | Y / △ | B / ○ | A / × | X / □ |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| R2 | F1 | F2 | F3 | F4 | F12 | F10 | F9 | F11 |
| R2 + L1 | F5 | F6 | F7 | F8 | — | — | — | — |

These keep the number positions: 1–9 become F1–F9, 0 becomes F10, and minus/equal
become F11/F12. L1 selects the second bank; its face buttons are unassigned.
Media controls, Print Screen, and Delete are on the editing/media bank:
R1+R2+L1. The reference labels volume Vol+/Vol−, tracks Prev/Next, and Play/pause Play.
The reference shows the function banks separately from the number banks.

Holding R1+R2 selects navigation over either one-shot R2 layer. That navigation
key also consumes the one-shot selections. R1 Space and L1 Backspace
remain available while a layer is selected and R2 is released. A used R2 hold
does not rearm a selection. An unused navigation prefix or R2 hold longer than
250 ms leaves an armed selection unchanged. Modes are captured at stroke start,
so adding R1 after a number stroke starts does not convert it into navigation;
early releases preserve
the captured mode. Pause, disconnect, and focus resets clear one-shot selections.

The Linux runtime implements this capture and mode selection, keyboard output,
and hold-to-repeat after 300 ms, then every 80 ms while held. The Python
collector models capture without emitting keyboard events. The full-text scorer still uses one-shot Shift for capitals
and symbols; it excludes navigation, functions, and held shortcuts. Recorded typing scores and letter positions are unchanged; the scores have not
been reevaluated for one-shot number entry.

## Search and full-text evaluation

The search results below used the earlier odd/even number banks. Numbers have
since been grouped as 1–4 and 5–8, keeping the alpha positions. The report's
`current_layout_evaluation` rescores the current layout with these number banks;
the remaining results preserve the original search and comparisons.

The existing Leipzig English web splits are evaluated with case preserved.
All printable ASCII, Tab, and Enter are covered; unsupported characters fail
instead of being removed. CRLF becomes LF. The complete 1,067,910 training and
108,574 held-out characters are scored. Held-out text includes 3,371 capitals
and 500 shifted symbols, requiring 3,871 one-shot Shift taps.

The 24 letters other than Q/Z may move between base and single-bumper banks.
Q/Z, five alpha punctuation characters, and Tab use both bumpers. The five pins
and R2 banks stay fixed. Pinned, unpinned, and pinned-without-roll-preference
searches each use seeds 42–46 and 20,000 proposals per seed, followed by improving
swaps until no allowed swap improves the score. The Rust helper evaluates exact
affected factors and runs independent seeds in parallel. Every returned score
is checked with Python. Selection uses training text only. Seed 42 wins the
pinned search; seed 46 wins the unpinned search.

| Held-out result | Value |
| --- | ---: |
| Letters on base | 65.20% |
| Letters on L1 / R1 / both | 20.75% / 13.84% / 0.21% |
| Left / right thumb letter load | 50.18% / 49.82% |
| Adjacent-letter alternation | 70.82% |
| Same-bank adjacent roll opportunities | 6.85% |
| Comfortable roll opportunities | 4.00% |
| Space rearms per 100 characters | 2.39 |
| Corpus character coverage | 100% |

| Same full-text model | Previous alpha positions | Regenerated |
| --- | ---: | ---: |
| Objective, lower is better | 2.148386 | 2.148163 |
| Load objective, without rolling credit | 2.169327 | 2.169107 |
| Weighted skipgram travel cost | 0.102915 | 0.102802 |

[baseline-alpha.json](baseline-alpha.json) preserves the previous alpha positions.
Both candidates use the same earlier odd/even number banks and full-text rules in this table.
The regeneration makes a small improvement. Absolute scores are not comparable
with v1/v2 reports, which omitted characters, Shift, skipgrams, and constant costs.
The [report](report.json) records assignments, seeds, weights, hashes, and comparisons.

Pins cost 0.15% on training and 0.14% on held-out text versus the unpinned search.
With the same pins, disabling the roll preference gives 6.61% held-out roll
opportunities and 2.86% comfortable opportunities; enabling it gives 6.85% and
4.00%. Alternation is 70.82% in both. Opportunities in text are not measured rolls.

## What the score measures

Model `four-bank-alpha-v3` includes ordinary presses, held-bank load, modifier
presses/releases, one-shot Shift, thumb reuse, and adjacent travel. It adds each
thumb's next movement across one, two, or three intervening characters, weighted
0.5, 0.25, and 0.125. An intervening use of that thumb prevents double-counting
a longer skipgram. Newlines end skipgram history.

Same-bank adjacent rolls get a bounded 0.6 rebate; existing awkward inner-up and
outer-down pairs get half credit. Repeated buttons, opposite buttons, and bank
changes get none. A new one-shot Shift also interrupts a roll. Thumb reuse and
travel remain charged, so alternation is preferred. `load_objective` excludes
the rebate. Squared penalties balance thumb use and index engagement, including
Space, R2, and one-shot Shift. Shift releases held L1 first; bank changes release
outgoing modifiers before pressing incoming ones. This plan avoids same-side
shoulder holds.

This is an estimated cost model. Hold load is per character, not elapsed time
or force; the 100 ms window has no measured timing cost. The canonical typing plan
releases bank modifiers for Space and Enter. Retaining L1 across Space or banks
across Enter is not optimized. Menu-key travel, corrections, Caps Lock,
function-key usage, navigation, and shortcuts are excluded. Scores do not predict speed, fatigue,
or memorability.

## Reproduce

From the repository root with Rust and Cargo available; on Nix enter `nix-shell`
first:

```sh
python3 scripts/prepare_english_corpus.py
python3 scripts/generate_alpha_banks.py
python3 -m unittest discover -s scripts -p 'test_*.py'
cargo test --example alpha_bank_search --locked
```

The generator builds its Rust helper and writes JSON, a report, and offline
HTML. Open the HTML and print on A4 portrait to refresh the PDF.
