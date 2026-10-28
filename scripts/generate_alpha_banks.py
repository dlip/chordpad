#!/usr/bin/env python3
"""Generate an experimental four-bank alpha layout and offline reference."""

import argparse
import html
import json
import subprocess
from pathlib import Path

from alpha_bank_model import (
    BANKS,
    CONTROLS,
    EDGE,
    TRIGGER_TAP_MAX_MS,
    GROUPS,
    HOLD,
    LABELS,
    LATE_BUMPER_WINDOW_MS,
    LOAD_EDGE,
    NUMBER_BASE,
    PINS,
    SHIFT_PAIRS,
    SIZE,
    SKIP_WEIGHTS,
    SYMBOLS,
    WEIGHTS,
    corpus_stats,
    keyboard_modifiers,
    metrics,
    objective,
    slot_info,
    validate,
)

ROOT = Path(__file__).resolve().parents[1]


def native_search(stats, seeds, iterations, pins, prefer_rolls):
    slots = [slot_info(i) for i in range(SIZE)]
    problem = {
        "counts": stats["counts"], "unary": HOLD,
        "edge_costs": EDGE if prefer_rolls else LOAD_EDGE,
        "edges": stats["edges"], "skips": stats["skips"],
        "slots": [{"bank": b, "thumb": t, "position": p} for b, t, p in slots],
        "groups": [[SYMBOLS.index(c) for c in group] for group in GROUPS],
        "available": [list(range(24)), list(range(24, 32))],
        "pins": {SYMBOLS.index(c): s for c, s in pins.items()},
        "index_offset": stats["index_offset"],
        "balance_weights": [WEIGHTS["thumb_imbalance_squared"], WEIGHTS["index_imbalance_squared"]],
        "travel_weight": WEIGHTS["thumb_travel"],
        "seeds": seeds, "iterations": iterations,
    }
    result = subprocess.run([str(ROOT / "target/release/examples/alpha_bank_search")],
                            input=json.dumps(problem), text=True, stdout=subprocess.PIPE, check=True)
    return json.loads(result.stdout)


def label(c):
    return {"\t": "Tab", "MEDIA_PLAY_PAUSE": "Play", "MEDIA_NEXT": "Next",
            "DELETE": "Del", "PRINT_SCREEN": "PrtSc", "UP": "↑", "RIGHT": "→",
            "DOWN": "↓", "LEFT": "←", "PAGE_UP": "PgUp", "PAGE_DOWN": "PgDn",
            "HOME": "Home", "END": "End", "VOLUME_UP": "Vol+", "VOLUME_DOWN": "Vol−",
            "MEDIA_PREVIOUS": "Prev", "MEDIA_STOP": "Stop", "MUTE": "Mute"}.get(c, c)


def layout_banks(assignment):
    banks = []
    for bank, held in enumerate(BANKS):
        characters = ([SYMBOLS[assignment.index(slot)] for slot in range(bank * 8, (bank + 1) * 8)]
                      if bank < 4 else NUMBER_BASE[(bank - 4) * 8:(bank - 3) * 8])
        banks.append({"held": held, "bindings": [
            {"control": CONTROLS[i], "character": c,
             **({"shifted_character": SHIFT_PAIRS[c]} if c in SHIFT_PAIRS else {})}
            for i, c in enumerate(characters)
        ]})
    return banks


def function_banks():
    keys = {str(i): f"f{i}" for i in range(1, 10)} | {"0": "f10", "-": "f11", "=": "f12"}
    banks = [{"held": ["R2"] + (["L1"] if bank else []), "bindings": [
        {"control": CONTROLS[i], "key": keys[c]}
        for i, c in enumerate(NUMBER_BASE[bank * 8:(bank + 1) * 8]) if c in keys
    ]} for bank in range(2)]
    return banks


def navigation_banks():
    banks = [{"held": ["R1", "R2"] + (["L1"] if bank else []), "bindings": [
        {"control": control, "key": key} for control, key in zip(
            CONTROLS, (["volume_up", "media_next", "volume_down", "media_previous",
                        "print_screen", "media_stop", "media_play_pause", "delete"] if bank else
                       ["up", "right", "down", "left", "page_up", "end", "page_down", "home"])
        )
    ]} for bank in range(2)]
    return banks


def candidate_layout(assignment):
    return {
        "format": "chordpad-alpha-bank-candidate-v2",
        "runtime_supported": True,
        "runtime_platforms": ["linux", "android"],
        "recognition": {
            "late_bumper_window_ms": LATE_BUMPER_WINDOW_MS,
            "freeze": "Next thumb press, first release, or window expiry, whichever happens first.",
            "late_additions": "New bumper presses join only the newest eligible held thumb stroke.",
            "bumper_release": "Preserve each stroke's captured bank.",
            "output_order": "First outputs retain stroke-start order; held repeats can interleave with later typing. No extra delay to finish the collection window.",
            "tap_consumption": "Each bumper used by any thumb stroke consumes its tap.",
            "hold_repeat": "Every mapped output repeats after a 300 ms hold, then every 80 ms, including characters, function/navigation/media keys and menu/click shortcuts. Capture the bank, mode and modifiers; releasing the output control, pausing, or resetting stops repetition without an extra release action. Modifier and layer selectors do not repeat as actions.",
            "bumper_repeat": "A standalone L1/Backspace or R1/Space hold repeats after 300 ms without another control press, then every 80 ms. A control joining before 300 ms cancels repetition for that hold. Once repeating, that bumper acts only as its key until released. Preheld keyboard modifiers are supported. Release, pause, reset, and disconnect stop repetition.",
            "same_thumb_rolling": "Each thumb button press starts its own stroke, including overlapping adjacent buttons.",
            "number_layer": "Hold R2 for momentary numbers or tap it once for the next key. R2 has no late-join window. Navigation requires physically held R1+R2.",
            "trigger_tap_max_ms": TRIGGER_TAP_MAX_MS,
            "function_layer": "Unused R2 taps cycle numbers, functions, off on release, with no inter-tap deadline. Each tap must last at most 250 ms. The next key consumes the selected layer and returns to alpha. Used holds do not advance the cycle. L1 selects the second bank.",
            "one_shot_shift": "An unused L2 tap of at most 250 ms arms Shift for the next key; a second unused tap cancels it. A used or longer hold does not rearm it. Keyboard modifiers are captured at stroke start.",
            "held_shift": "L2 also works as momentary Shift. Linux sends held modifiers to the OS immediately. Android supports Shift-click/tap in focused editors and global mouse modifiers through the optional ADB helper. Fixed L3/R3 shortcuts ignore held and one-shot modifiers.",
            "button_modifiers": "Tap and release an unused R1+R2 pair to arm face-button modifiers, or L1+L2 for the D-pad. Hold a mirrored stick direction, including adjacent pairs for diagonals, across typed keys or mouse actions. Releasing the directional controls ends that side's mode. Repeat an unused pair tap to cancel. Pairs used with typing keep their layers, including R1+R2 navigation. Unused pairs consume Space/Backspace and sticky Shift/numbers. Reset clears both sides.",
            "navigation_layer": "Physically hold R1+R2; L1 selects editing/media. Navigation overrides and consumes either one-shot R2 layer on the next key and consumes R1's Space tap and R2's layer tap. All mapped outputs repeat while held.",
            "reset": "Pause, focus change, disconnect, or reset clears both one-shot selections and requires neutral controls.",
        },
        "banks": layout_banks(assignment),
        "function_banks": function_banks(),
        "navigation_banks": navigation_banks(),
        "held_shift_buttons": ["L2"],
        "proposed_controls": {"R1_tap": "Space", "L1_tap": "Backspace",
                              "L1_R1_unused_hold": "Backspace / Space repeat after 300 ms, every 80 ms",
                              "L2_tap": "Shift next key; tap again to cancel",
                              "L2_hold": "Shift", "R2_tap": "Numbers → functions → off; next key clears selection", "R2_hold": "Momentary numbers",
                              "R1_R2_hold": "Navigation", "Start": "Enter", "Select": "Escape",
                              "R1_R2_tap": "Arm face-button modifiers until direction release", "L1_L2_tap": "Arm D-pad modifiers until direction release",
                              "L3": "Command+D", "R3": "Command+Shift+D",
                              "A_B_X_Y": "Pause/resume"},
    }


def stick_reference():
    directions = [("up-left", "↖"), ("up", "↑"), ("up-right", "↗"),
                  ("left", "←"), ("neutral", "·"), ("right", "→"),
                  ("down-left", "↙"), ("down", "↓"), ("down-right", "↘")]
    sections = []
    for side in ["left", "right"]:
        button, shortcut, purpose = (("L3", "Command+D", "Dictate") if side == "left" else
                                     ("R3", "Command+Shift+D", "Cancel Dictation"))
        keys = []
        for direction, arrow in directions:
            if direction == "neutral":
                keys.append(f'<div class="key"><small>Click {button}</small>'
                            f'<strong>{html.escape(shortcut)}</strong></div>')
                continue
            modifiers = [m.title() for m in keyboard_modifiers(**{side: direction})]
            lines = [" + ".join(modifiers[i:i + 2]) for i in range(0, len(modifiers), 2)]
            text = "<br>+ ".join(lines) or "None"
            keys.append(f'<div class="key"><small>{arrow}</small><strong>{text}</strong></div>')
        sections.append(f'<section><h2>{side.title()} stick: hold for modifiers</h2>'
                        f'<div class="stick-map">{"".join(keys)}</div>'
                        f'<p>You can map {button} to {purpose} on your computer.</p></section>')
    return "".join(sections)


def render(assignment, report):
    sections = []
    reference_banks = list(zip(LABELS[:4] + ["R2: hold or tap once", "Add L1: numbers 5–8"], layout_banks(assignment)))
    reference_banks.extend(zip(["R1 + R2: navigation", "Add L1: editing / media"], navigation_banks()))
    reference_banks.extend(zip(["R2 tap twice: function", "Add L1: function"], function_banks()))
    for bank, (title, data) in enumerate(reference_banks):
        bindings = {b["control"]: b for b in data["bindings"]}
        maps = []
        for side in range(2):
            keys = []
            for position, area in enumerate(["north", "east", "south", "west"]):
                control = CONTROLS[side * 4 + position]
                binding = bindings.get(control, {})
                c = binding.get("character", binding.get("key", "·").upper())
                shifted = (f'<em>{html.escape(SHIFT_PAIRS[c])}</em>'
                           if 4 <= bank < 6 else '')
                key_class = "longer" if len(label(c)) > 3 else "long" if len(label(c)) > 2 else ""
                keys.append(f'<div class="key {area}"><small>{control}</small>'
                            f'<strong class="{key_class}">{html.escape(label(c))}{shifted}</strong></div>')
            maps.append(f'<div class="hand"><p>{["D-pad", "Face buttons"][side]}</p>'
                        f'<div class="pad">{"".join(keys)}</div></div>')
        sections.append(f'<section><h2>{title}</h2><div class="hands">{"".join(maps)}</div></section>')
    return ((ROOT / "src/render/four-bank.html").read_text(encoding="utf-8")
            .replace("@@BANKS@@", "".join(sections)).replace("@@STICKS@@", stick_reference())
            .replace("@@WINDOW_MS@@", str(LATE_BUMPER_WINDOW_MS)))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", type=Path, default=ROOT / "corpora/english-web/train.txt")
    parser.add_argument("--held-out", type=Path, default=ROOT / "corpora/english-web/held-out.txt")
    parser.add_argument("--output-dir", type=Path, default=ROOT / "experiments/four-bank-alpha")
    parser.add_argument("--iterations", type=int, default=20000)
    parser.add_argument("--seeds", type=int, nargs="+", default=[42, 43, 44, 45, 46])
    parser.add_argument("--baseline", type=Path, default=ROOT / "experiments/four-bank-alpha/baseline-alpha.json")
    args = parser.parse_args()
    if args.iterations < 1:
        parser.error("--iterations must be positive")
    train, held = corpus_stats(args.corpus), corpus_stats(args.held_out)
    subprocess.run(["cargo", "build", "--release", "--example", "alpha_bank_search", "--locked"], cwd=ROOT, check=True)
    candidates = {}
    for name, pins, prefer_rolls in [("unpinned", {}, True), ("pinned", PINS, True),
                                   ("without_roll_preference", PINS, False)]:
        results = []
        print(f"Searching {name}; {len(train['skips'])} skipgram factors", flush=True)
        for run in native_search(train, args.seeds, args.iterations, pins, prefer_rolls):
            seed, assignment = run["seed"], run["assignment"]
            validate(assignment, pins)
            score = objective(assignment, train, prefer_rolls)
            if abs(score - run["objective"]) > 1e-9:
                raise ValueError("Native and reference scores disagree")
            results.append((score, seed, assignment))
        _, seed, assignment = min(results)
        candidates[name] = {
            "optimized_metric": "objective" if prefer_rolls else "load_objective",
            "selected_seed": seed,
            "runs": [{"seed": s, "training_objective": score} for score, s, _ in results],
            "assignment": assignment,
            "training": metrics(assignment, train),
            "held_out": metrics(assignment, held),
        }
    pinned, unpinned = candidates["pinned"], candidates["unpinned"]
    without_rolls = candidates["without_roll_preference"]
    seed, assignment = pinned["selected_seed"], pinned["assignment"]
    baseline_bytes = args.baseline.read_bytes()
    baseline = json.loads(baseline_bytes)
    previous = list(range(SIZE))
    for bank, data in enumerate(baseline["banks"][:4]):
        for binding in data["bindings"]:
            previous[SYMBOLS.index(binding["character"])] = bank * 8 + CONTROLS.index(binding["control"])
    validate(previous, PINS)
    report = {
        "model": "four-bank-alpha-v3",
        "number_banks": [NUMBER_BASE[:8], NUMBER_BASE[8:]],
        "search_space": "24 letters may move between base and single-bumper banks; Q/Z and punctuation use both bumpers.",
        "weights": WEIGHTS,
        "late_bumper_window_ms": LATE_BUMPER_WINDOW_MS,
        "timing_scored": False,
        "skipgram_weights": SKIP_WEIGHTS,
        "text": "Case preserved; full printable ASCII, Tab and Enter; unsupported characters fail.",
        "caps_lock_scored": False,
        "function_keys_scored": False,
        "held_modifiers_scored": False,
        "navigation_scored": False,
        "rolling": "Same-bank adjacent moves on one thumb get a bounded rebate; awkward diagonals get half credit. Alternation remains cheaper.",
        "pins": PINS,
        "selection": "Lowest training objective; held-out text is evaluation only.",
        "iterations_per_seed": args.iterations,
        "selected_seed": seed,
        "runs": pinned["runs"],
        "corpus_sha256": {"training": train["sha256"], "held_out": held["sha256"]},
        "training": pinned["training"],
        "held_out": pinned["held_out"],
        "unpinned_comparison": unpinned,
        "without_roll_preference_comparison": without_rolls,
        "previous_alpha_comparison": {
            "description": "Previous alpha positions rescored with the same fixed number banks and full-text model.",
            "assignment": previous,
            "training": metrics(previous, train),
            "held_out": metrics(previous, held),
        },
        "load_objective_increase_percent": {
            split: 100 * (pinned[split]["load_objective"] / without_rolls[split]["load_objective"] - 1)
            for split in ["training", "held_out"]
        },
        "objective_increase_percent": {
            split: 100 * (pinned[split]["objective"] / unpinned[split]["objective"] - 1)
            for split in ["training", "held_out"]
        },
    }
    layout = candidate_layout(assignment)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    for name, data in [("layout.json", layout), ("report.json", report)]:
        (args.output_dir / name).write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    (args.output_dir / "reference.html").write_text(render(assignment, report), encoding="utf-8")
    print(f"Selected seed {seed}; wrote candidate and reference to {args.output_dir}")


if __name__ == "__main__":
    main()
