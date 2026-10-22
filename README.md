# padtype

Gamepad typing for Linux, with a layout generator that searches for comfortable,
efficient chords.

Inputs are the D-pad, A/B/X/Y, L1/R1, and L2/R2. Direct typing chords use the
D-pad and A/B/X/Y/L1/R1 by default. The analog L2/R2 triggers are reserved for
held Shift or layers. The generator chooses combinations within those roles.

The layout generator is implemented. The Linux typing app is planned.

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
Unicode is supported as a generator output symbol; the future typing app will
need an appropriate output backend to type it.

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
combination, with held modifiers included in its button list. Reserve a future
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

## Development

```sh
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

See [PLAN.md](PLAN.md) for chord recognition rules and the remaining milestones.
