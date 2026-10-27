import math
import string
import tempfile
import unittest
from pathlib import Path

from alpha_bank_model import (
    CHARACTERS,
    GROUPS,
    PINS,
    SIZE,
    SPACE,
    SYMBOLS,
    StrokeCollector,
    corpus_stats,
    edge_cost,
    keyboard_modifiers,
    modifier_plan,
    objective,
    optimize,
    roll_quality,
    skip_cost,
    validate,
)
from generate_alpha_banks import function_banks, layout_banks, navigation_banks


class AlphaBanksTest(unittest.TestCase):
    def stats(self, text):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "corpus.txt"
            path.write_text(text, encoding="utf-8")
            return corpus_stats(path)

    def test_corpus_preserves_case_and_shifted_punctuation_without_dropping_text(self):
        stats = self.stats("ABCDEFGHIJKLMNOPQRSTUVWXYZ!ab")
        self.assertEqual(stats["raw_counts"][0], 2)
        self.assertEqual(stats["uppercase_count"], 26)
        self.assertEqual(stats["shifted_symbol_count"], 1)
        self.assertEqual(stats["unsupported_characters"], 0)
        self.assertEqual(stats["modeled_characters"], 29)
        pairs = {(a, b) for a, b, _ in stats["pairs"]}
        self.assertNotIn((SYMBOLS.index("z"), SYMBOLS.index("a")), pairs)
        self.assertIn((SYMBOLS.index("z"), SYMBOLS.index("1")), pairs)
        self.assertIn((SYMBOLS.index("1"), SYMBOLS.index("a")), pairs)

    def test_every_printable_ascii_character_tab_and_enter_is_evaluated(self):
        text = string.ascii_letters + string.digits + string.punctuation + " \t\n"
        stats = self.stats(text)
        self.assertEqual(stats["modeled_characters"], 97)
        self.assertEqual(stats["uppercase_count"], 26)
        self.assertEqual(stats["shifted_symbol_count"], 21)
        self.assertEqual(stats["unsupported_characters"], 0)
        self.assertEqual(set(text), set(CHARACTERS))

    def test_unsupported_text_fails_instead_of_silently_changing_the_corpus(self):
        with self.assertRaisesRegex(ValueError, "Unsupported corpus character"):
            self.stats(string.ascii_lowercase + "é")

    def test_skipgram_charges_the_next_thumb_movement_across_the_other_thumb(self):
        assignment = list(range(SIZE))
        self.assertAlmostEqual(skip_cost([0, 4, 2], assignment), 0.8)
        self.assertAlmostEqual(skip_cost([0, 4, 5, 1], assignment), 0.4 * math.sqrt(2))
        self.assertEqual(skip_cost([0, 1, 2], assignment), 0)
        self.assertEqual(skip_cost([0, 4, 5], assignment), 0)

    def test_space_after_r1_requires_release_then_a_new_tap(self):
        self.assertEqual(modifier_plan(2, 0, space=True),
                         [("release", "R1"), ("press", "R1"), ("release", "R1")])
        self.assertAlmostEqual(edge_cost(16, SPACE) - edge_cost(0, SPACE), 0.7)

    def test_shift_gestures_release_l1_and_never_stack_shoulders_on_one_side(self):
        self.assertEqual(modifier_plan(1, 1, shifted=True), [
            ("release", "L1"), ("press", "L2"), ("release", "L2"), ("press", "L1")
        ])
        bits = {"L1": 1, "R1": 2, "R2": 4, "L2": 8}
        for before in range(6):
            for after in range(6):
                held = before
                for action, button in modifier_plan(before, after, shifted=True):
                    held = held | bits[button] if action == "press" else held & ~bits[button]
                    self.assertNotEqual(held & 9, 9)
                    self.assertNotEqual(held & 6, 6)
                self.assertEqual(held, after)

    def test_switching_bumpers_costs_more_than_keeping_one_held(self):
        self.assertGreater(edge_cost(8, 20), edge_cost(8, 12))
        self.assertGreater(edge_cost(0, 12), edge_cost(0, 4))

    def test_reusing_a_thumb_costs_more_than_alternating(self):
        self.assertGreater(edge_cost(0, 1), edge_cost(0, 5))

    def test_roll_credit_requires_adjacent_buttons_on_one_thumb_in_the_same_bank(self):
        for a, b in [(0, 0), (0, 2), (0, 4), (0, 9), (0, 32), (32, 33)]:
            with self.subTest(a=a, b=b):
                self.assertEqual(roll_quality(a, b), 0)
                self.assertEqual(edge_cost(a, b), edge_cost(a, b, prefer_rolls=False))

    def test_comfortable_rolls_receive_more_credit_than_awkward_rolls(self):
        for comfortable, awkward in [((0, 3), (0, 1)), ((4, 5), (4, 7))]:
            self.assertEqual(roll_quality(*comfortable), 1)
            self.assertEqual(roll_quality(*awkward), 0.5)
            self.assertLess(edge_cost(*comfortable), edge_cost(*awkward))
            self.assertEqual(edge_cost(*comfortable), edge_cost(*reversed(comfortable)))

    def test_roll_credit_keeps_thumb_load_and_never_makes_a_transition_negative(self):
        for a in range(34):
            for b in range(34):
                self.assertGreaterEqual(edge_cost(a, b), 0)
        self.assertGreater(edge_cost(0, 3), edge_cost(0, 4))
        self.assertGreater(edge_cost(0, 3), 0.9)
        self.assertLess(edge_cost(0, 3), edge_cost(0, 2))

    def test_search_is_reproducible_and_finishes_at_a_local_minimum(self):
        stats = self.stats("abcdefghijklmnopqrstuvwxyz " * 3 + "the other thing, a sunny day. " * 10)
        assignment = optimize(stats, seed=42, iterations=100)
        self.assertEqual(assignment, optimize(stats, seed=42, iterations=100))
        validate(assignment)
        self.assertEqual(len([slot for slot in assignment[:26] if slot < 8]), 8)
        score = objective(assignment, stats)
        for group in GROUPS:
            for i, a in enumerate(group):
                for b in group[i + 1:]:
                    x, y = SYMBOLS.index(a), SYMBOLS.index(b)
                    candidate = assignment[:]
                    candidate[x], candidate[y] = candidate[y], candidate[x]
                    self.assertGreaterEqual(objective(candidate, stats) + 1e-10, score)

    def test_sparse_corpus_fails_instead_of_generating_untrained_letters(self):
        with self.assertRaises(ValueError):
            self.stats("only some letters")

    def test_validation_rejects_duplicate_slots_and_letters_in_wrong_banks(self):
        assignment = [None] * 32 + list(range(32, SIZE))
        for group, slots in zip(GROUPS, [range(24), range(24, 32)]):
            for character, slot in zip(group, slots):
                assignment[SYMBOLS.index(character)] = slot
        validate(assignment)
        candidate = assignment[:]
        candidate[0] = candidate[1]
        with self.assertRaises(ValueError):
            validate(candidate)
        a, q = SYMBOLS.index("a"), SYMBOLS.index("q")
        assignment[a], assignment[q] = assignment[q], assignment[a]
        with self.assertRaises(ValueError):
            validate(assignment)

    def test_pins_survive_search_while_other_letters_can_change_banks(self):
        stats = self.stats("abcdefghijklmnopqrstuvwxyz " * 3 + "the other thing, a sunny day. " * 10)
        assignment = optimize(stats, seed=42, iterations=100, pins=PINS)
        validate(assignment, PINS)
        self.assertEqual([assignment[SYMBOLS.index(c)] for c in "udlro"], [12, 10, 11, 1, 5])
        candidate = assignment[:]
        u, o = SYMBOLS.index("u"), SYMBOLS.index("o")
        candidate[u], candidate[o] = candidate[o], candidate[u]
        validate(candidate)
        with self.assertRaises(ValueError):
            validate(candidate, PINS)

    def test_exported_banks_cover_ascii_with_standard_shift_pairs(self):
        stats = self.stats(string.ascii_lowercase)
        assignment = optimize(stats, seed=42, iterations=1, pins=PINS)
        banks = layout_banks(assignment)
        covered = {" ", "\n"}
        for bank in banks:
            for binding in bank["bindings"]:
                covered.add(binding["character"])
                if "shifted_character" in binding:
                    covered.add(binding["shifted_character"])
        self.assertEqual(covered, set(CHARACTERS))
        self.assertEqual(banks[4]["held"], ["R2"])
        self.assertEqual(banks[5]["held"], ["R2", "L1"])
        self.assertEqual([b["character"] for b in banks[4]["bindings"][:4]], list("1234"))
        self.assertEqual([b["character"] for b in banks[5]["bindings"][:4]], list("5678"))

    def test_function_keys_keep_number_positions(self):
        banks = function_banks()
        self.assertEqual(banks[0]["held"], ["R2"])
        self.assertEqual(banks[1]["held"], ["R2", "L1"])
        keys = [b["key"] for bank in banks for b in bank["bindings"]]
        self.assertCountEqual(keys, [f"f{i}" for i in range(1, 13)])
        self.assertEqual({b["control"]: b["key"] for b in banks[0]["bindings"]}, {
            "↑": "f1", "→": "f2", "↓": "f3", "←": "f4",
            "Y / △": "f12", "B / ○": "f10", "A / ×": "f9", "X / □": "f11",
        })
        self.assertEqual({b["control"]: b["key"] for b in banks[1]["bindings"]}, {
            "↑": "f5", "→": "f6", "↓": "f7", "←": "f8",
        })


class StrokeCollectorTest(unittest.TestCase):
    def test_navigation_banks_keep_arrows_and_face_shortcuts(self):
        banks = navigation_banks()
        self.assertEqual([b["held"] for b in banks], [["R1", "R2"], ["R1", "R2", "L1"]])
        self.assertEqual([b["key"] for b in banks[0]["bindings"]],
                         ["up", "right", "down", "left", "page_up", "end", "page_down", "home"])
        self.assertEqual([b["key"] for b in banks[1]["bindings"]],
                         ["volume_up", "media_next", "volume_down", "media_previous",
                          "print_screen", "media_stop", "media_play_pause", "delete"])

    def test_shift_tap_applies_to_one_stroke_and_second_tap_cancels(self):
        collector = StrokeCollector()
        collector.press_shift("L2")
        collector.release_shift("L2")
        collector.press_thumb(0, 0)
        collector.press_thumb(1, 10)
        self.assertEqual(collector.release_strokes(1), [])
        self.assertEqual([s.modifiers for s in collector.release_strokes(0)], [("shift",), ()])
        for _ in range(2):
            collector.press_shift("L2")
            collector.release_shift("L2")
        collector.press_thumb(0, 20)
        self.assertEqual(collector.release_strokes(0)[0].modifiers, ())
        collector.press_shift("L2")
        for now in [30, 40]:
            collector.press_thumb(0, now)
            self.assertEqual(collector.release_strokes(0)[0].modifiers, ("shift",))
        collector.release_shift("L2")
        collector.press_thumb(0, 50)
        self.assertEqual(collector.release_strokes(0)[0].modifiers, ())

    def test_long_unused_shift_hold_does_not_arm_one_shot(self):
        for duration, modifiers in [(250, ("shift",)), (251, ())]:
            collector = StrokeCollector()
            collector.press_shift("L2", 10)
            collector.release_shift("L2", 10 + duration)
            collector.press_thumb(0, 20 + duration)
            self.assertEqual(collector.release_strokes(0)[0].modifiers, modifiers)

    def test_r2_taps_cycle_until_one_stroke_consumes_the_selection(self):
        for taps, mode in [(1, "number"), (2, "function"), (3, "alpha")]:
            collector = StrokeCollector()
            for tap in range(taps):
                at = tap * 1000
                collector.press_r2(at)
                collector.release_r2(at + 20)
            collector.press_thumb(0, 4000)
            collector.press_thumb(1, 4010)
            self.assertEqual(collector.release_strokes(1), [])
            self.assertEqual([s.mode for s in collector.release_strokes(0)], [mode, "alpha"])

    def test_held_r2_works_across_strokes_without_arming_a_tap(self):
        collector = StrokeCollector()
        collector.press_r2(0)
        for now in [10, 20]:
            collector.press_thumb(0, now)
            self.assertEqual(collector.release_strokes(0)[0].mode, "number")
        collector.release_r2(30)
        collector.press_thumb(0, 40)
        self.assertEqual(collector.release_strokes(0)[0].mode, "alpha")

    def test_first_r2_press_used_for_number_or_navigation_cannot_arm_functions(self):
        for use in ["number", "navigation_before", "navigation_after"]:
            with self.subTest(use=use):
                collector = StrokeCollector()
                if use == "navigation_before":
                    collector.press_bumper("R1", 0)
                collector.press_r2(10)
                if use == "number":
                    collector.press_thumb(0, 20)
                    collector.release_strokes(0)
                elif use == "navigation_after":
                    collector.press_bumper("R1", 20)
                collector.release_r2(30)
                collector.release_bumper("R1")
                collector.press_r2(40)
                collector.press_thumb(0, 50)
                self.assertEqual(collector.release_strokes(0)[0].mode, "number")

    def test_navigation_overrides_one_shot_functions_and_strokes_keep_captured_mode(self):
        collector = StrokeCollector()
        for at in [0, 1000]:
            collector.press_r2(at)
            collector.release_r2(at + 20)
        collector.press_thumb(0, 1030)
        collector.press_r2(1040)
        collector.press_bumper("R1", 1050)
        collector.press_bumper("L1", 1060)
        collector.press_thumb(4, 1070)
        collector.release_r2(1080)
        collector.release_bumper("L1")
        self.assertEqual(collector.release_strokes(4), [])
        self.assertEqual([(s.mode, s.bank) for s in collector.release_strokes(0)],
                         [("function", 0), ("navigation", 1)])
        collector.release_bumper("R1")
        collector.press_thumb(0, 1090)
        self.assertEqual(collector.release_strokes(0)[0].mode, "alpha")

    def test_r2_tap_duration_boundary_has_no_inter_tap_deadline(self):
        for duration, gap, expected in [(20, 250, "function"), (20, 10000, "function"),
                                        (250, 20, "function"), (251, 20, "number")]:
            with self.subTest(duration=duration, gap=gap):
                collector = StrokeCollector()
                collector.press_r2(0)
                collector.release_r2(duration)
                collector.press_r2(duration + gap)
                collector.release_r2(duration + gap + 10)
                collector.press_thumb(0, duration + gap + 20)
                self.assertEqual(collector.release_strokes(0)[0].mode, expected)

    def test_late_r1_does_not_convert_existing_number_stroke_to_navigation(self):
        collector = StrokeCollector()
        collector.press_r2(0)
        collector.press_thumb(0, 10)
        collector.press_bumper("R1", 20)
        collector.press_thumb(4, 30)
        collector.release_r2(40)
        self.assertEqual(collector.release_strokes(4), [])
        self.assertEqual([(s.mode, s.bank) for s in collector.release_strokes(0)],
                         [("number", 0), ("navigation", 0)])

    def test_shift_navigation_and_secondary_bank_work_without_sticks(self):
        collector = StrokeCollector()
        collector.press_shift("L2")
        collector.press_bumper("R1", 0)
        collector.press_r2(10)
        collector.press_thumb(0, 20)
        collector.release_shift("L2")
        collector.release_r2(30)
        stroke = collector.release_strokes(0)[0]
        self.assertEqual((stroke.mode, stroke.bank, stroke.modifiers), ("navigation", 0, ("shift",)))
        collector.press_r2(40)
        collector.press_bumper("L1", 50)
        collector.press_thumb(4, 60)
        stroke = collector.release_strokes(4)[0]
        self.assertEqual((stroke.mode, stroke.bank, stroke.modifiers), ("navigation", 1, ()))

    def test_l2_adds_shift_to_optional_mirrored_stick_modifiers(self):
        for side, outward in [("left", "left"), ("right", "right")]:
            for direction, expected in [(outward, ("ctrl", "shift")),
                                        ("up", ("alt", "shift")), ("down", ("shift", "meta"))]:
                self.assertEqual(keyboard_modifiers(**{side: direction}, held_shift=("L2",)), expected)
        self.assertEqual(keyboard_modifiers(held_shift=("L2",)), ("shift",))
        self.assertEqual(keyboard_modifiers(left="up-right", held_shift=("L2",)),
                         ("ctrl", "alt", "shift", "meta"))

    def test_held_shift_and_stick_modifiers_survive_early_release_for_that_stroke(self):
        collector = StrokeCollector()
        collector.set_stick("right", "right")
        collector.press_shift("L2")
        collector.press_thumb(0, 0)
        collector.release_shift("L2")
        collector.set_stick("right", "neutral")
        self.assertEqual(collector.release_strokes(0)[0].modifiers, ("ctrl", "shift"))
        collector.press_thumb(1, 10)
        self.assertEqual(collector.release_strokes(1)[0].modifiers, ())

    def test_releasing_shift_preserves_ctrl_and_keeps_overlapping_strokes_ordered(self):
        collector = StrokeCollector()
        collector.set_stick("left", "left")
        collector.press_shift("L2")
        collector.press_thumb(4, 0)
        collector.release_shift("L2")
        collector.press_thumb(5, 20)
        self.assertEqual(collector.release_strokes(5), [])
        self.assertEqual([s.modifiers for s in collector.release_strokes(4)],
                         [("ctrl", "shift"), ("ctrl",)])

    def test_late_bumper_changes_only_the_newest_stroke_and_preserves_output_order(self):
        collector = StrokeCollector()
        collector.press_thumb(0, 0)
        collector.press_thumb(4, 20)
        collector.press_bumper("L1", 30)
        self.assertEqual(collector.release_thumb(4), [])
        self.assertEqual(collector.release_thumb(0), [(0, 0), (4, 1)])

    def test_new_same_thumb_press_freezes_the_previous_letter(self):
        collector = StrokeCollector()
        collector.press_thumb(0, 0)
        collector.press_thumb(1, 10)
        collector.press_bumper("R1", 20)
        self.assertEqual(collector.release_thumb(0), [(0, 0)])
        self.assertEqual(collector.release_thumb(1), [(1, 2)])

    def test_100_ms_boundary_and_early_release(self):
        for delay, expected in [(99, 1), (100, 0), (101, 0)]:
            collector = StrokeCollector()
            collector.press_thumb(0, 0)
            collector.press_bumper("L1", delay)
            self.assertEqual(collector.release_thumb(0), [(0, expected)])
        collector = StrokeCollector()
        collector.press_thumb(0, 0)
        self.assertEqual(collector.release_thumb(0), [(0, 0)])
        collector.press_bumper("L1", 20)
        self.assertEqual(list(collector.pending), [])

    def test_held_bank_survives_early_release_and_frozen_older_strokes_stay_frozen(self):
        collector = StrokeCollector()
        collector.press_bumper("L1", 0)
        collector.press_thumb(0, 10)
        collector.release_bumper("L1")
        collector.press_thumb(4, 20)
        self.assertEqual(collector.release_thumb(4), [])
        collector.press_bumper("R1", 30)
        self.assertEqual(collector.release_thumb(0), [(0, 1), (4, 0)])


if __name__ == "__main__":
    unittest.main()
