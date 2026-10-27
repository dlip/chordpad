"""Full-text scoring and experimental stroke collection for the alpha candidate."""

import hashlib
import math
import random
import string
from collections import Counter, deque
from dataclasses import dataclass

ALPHA = string.ascii_lowercase + "',.;/\t"
NUMBER_BASE = "1234=09-5678[]\\`"
SYMBOLS = ALPHA + " \0" + NUMBER_BASE + "\n"
SPACE, BOUNDARY, ENTER = 32, 33, 50
RARE = "qz',.;/\t"
GROUPS = ["".join(c for c in string.ascii_lowercase if c not in "qz"), RARE]
PINS = {"u": 12, "d": 10, "l": 11, "r": 1, "o": 5}
LATE_BUMPER_WINDOW_MS = 100
TRIGGER_TAP_MAX_MS = 250
SKIP_WEIGHTS = (0.5, 0.25, 0.125)
CONTROLS = ["↑", "→", "↓", "←", "Y / △", "B / ○", "A / ×", "X / □"]
BANKS = [[], ["L1"], ["R1"], ["L1", "R1"], ["R2"], ["R2", "L1"]]
LABELS = ["Base", "Hold L1", "Hold R1", "Hold L1 + R1", "Hold R2", "Hold R2 + L1"]
POINTS = [(0, -1), (1, 0), (0, 1), (-1, 0)]
WEIGHTS = {
    "same_thumb": 1.0,
    "thumb_travel": 0.4,
    "roll_bonus": 0.6,
    "awkward_roll_fraction": 0.5,
    "bumper_press": 1.1,
    "trigger_press": 1.4,
    "bumper_change": 0.35,
    "bumper_hold_per_character": 0.165,
    "trigger_hold_per_character": 0.21,
    "space_rearm": 0.35,
    "thumb_imbalance_squared": 2.0,
    "index_imbalance_squared": 0.5,
}
SHIFT_PAIRS = dict(zip("1234567890-=[]\\`;'/.,", "!@#$%^&*()_+{}|~:\"?><"))
SHIFT_PAIRS.update({c: c.upper() for c in string.ascii_lowercase})
CHARACTERS = {c: (i, False) for i, c in enumerate(SYMBOLS) if c != "\0"}
CHARACTERS.update({shift: (SYMBOLS.index(base), True) for base, shift in SHIFT_PAIRS.items()})
SIZE = len(SYMBOLS)


def slot_info(slot):
    if slot < 32:
        return slot // 8, (slot % 8) // 4, POINTS[slot % 4]
    if 34 <= slot < 50:
        position = slot - 34
        return 4 + position // 8, (position % 8) // 4, POINTS[position % 4]
    return 0, None, (0, 0)


def roll_quality(a, b):
    bank_a, thumb_a, point_a = slot_info(a)
    bank_b, thumb_b, point_b = slot_info(b)
    if bank_a != bank_b or thumb_a is None or thumb_a != thumb_b:
        return 0.0
    if point_a[0] * point_b[0] + point_a[1] * point_b[1] != 0:
        return 0.0
    pair = frozenset([POINTS.index(point_a), POINTS.index(point_b)])
    comfortable = [{0, 3}, {1, 2}] if thumb_a == 0 else [{0, 1}, {2, 3}]
    return 1.0 if pair in comfortable else WEIGHTS["awkward_roll_fraction"]


def modifier_plan(before, after, shifted=False, space=False):
    """Canonical release-before-press gesture, with Shift clear of held L1."""
    events = []
    if shifted:
        if before & 1:
            events.append(("release", "L1"))
            before &= ~1
        events.extend([("press", "L2"), ("release", "L2")])
    for bit, name in [(1, "L1"), (2, "R1"), (4, "R2")]:
        if before & ~after & bit:
            events.append(("release", name))
    for bit, name in [(1, "L1"), (2, "R1"), (4, "R2")]:
        if after & ~before & bit:
            events.append(("press", name))
    if space:
        events.extend([("press", "R1"), ("release", "R1")])
    return events


def edge_cost(a, b, prefer_rolls=True, shifted=False):
    bank_a, thumb_a, point_a = slot_info(a)
    bank_b, thumb_b, point_b = slot_info(b)
    same = thumb_a is not None and thumb_a == thumb_b
    gestures = modifier_plan(bank_a, bank_b, shifted, b == SPACE)
    modifier_cost = sum(
        WEIGHTS["bumper_change"]
        + (WEIGHTS["trigger_press"] if name in ("L2", "R2") else WEIGHTS["bumper_press"])
        * (action == "press")
        for action, name in gestures
    )
    return (
        WEIGHTS["same_thumb"] * same
        + WEIGHTS["thumb_travel"] * math.dist(point_a, point_b) * same
        + modifier_cost
        + WEIGHTS["space_rearm"] * (b == SPACE and bool(bank_a & 2))
        - (WEIGHTS["roll_bonus"] * roll_quality(a, b) if prefer_rolls and not shifted else 0.0)
    )


def edge_table(prefer_rolls=True):
    return [[[edge_cost(a, b, prefer_rolls, shifted) for b in range(SIZE)]
             for a in range(SIZE)] for shifted in [False, True]]


EDGE, LOAD_EDGE = edge_table(), edge_table(False)
HOLD = [
    (1.0 if s not in (SPACE, BOUNDARY) else 0.0)
    + (slot_info(s)[0] & 3).bit_count() * WEIGHTS["bumper_hold_per_character"]
    + bool(slot_info(s)[0] & 4) * WEIGHTS["trigger_hold_per_character"]
    for s in range(SIZE)
]


def corpus_stats(path):
    raw = path.read_bytes()
    text = raw.decode("utf-8").replace("\r\n", "\n")
    if not text:
        raise ValueError("Corpus is empty")
    try:
        actions = [CHARACTERS[c] for c in text]
    except KeyError as error:
        raise ValueError(f"Unsupported corpus character {error.args[0]!r}") from error
    tokens = [i for i, _ in actions]
    counts = Counter(tokens)
    if any(not counts[SYMBOLS.index(c)] for c in string.ascii_lowercase):
        raise ValueError("Training and evaluation text must contain every ASCII letter")
    pairs = Counter(zip([BOUNDARY] + tokens, tokens + [BOUNDARY]))
    action_pairs = Counter((a[0], b[0], int(b[1])) for a, b in zip(
        [(BOUNDARY, False)] + actions, actions + [(BOUNDARY, False)]
    ))
    skips = Counter()
    for gap, weight in enumerate(SKIP_WEIGHTS, 1):
        windows = Counter(tuple(tokens[i:i + gap + 2]) for i in range(len(tokens) - gap - 1)
                          if ENTER not in tokens[i:i + gap + 2])
        for window, count in windows.items():
            skips[window] += count * weight
    total = len(text)
    return {
        "sha256": hashlib.sha256(raw).hexdigest(),
        "counts": [counts[i] / total for i in range(SIZE)],
        "raw_counts": [counts[i] for i in range(SIZE)],
        "pairs": [(a, b, n / total) for (a, b), n in sorted(pairs.items())],
        "edges": [(a, b, shifted, n / total) for (a, b, shifted), n in sorted(action_pairs.items())],
        "skips": [(list(window), n / total) for window, n in sorted(skips.items())],
        "index_offset": (sum(shift for _, shift in actions) - counts[SPACE]) / total,
        "uppercase_count": sum(c in string.ascii_uppercase for c in text),
        "shifted_symbol_count": sum(shift and i >= 26 for i, shift in actions),
        "modeled_characters": total,
        "unsupported_characters": 0,
    }


def balance(assignment, frequencies, index_offset=0.0):
    thumb_difference = 0.0
    index_difference = index_offset
    for symbol, slot in enumerate(assignment):
        bank, thumb, _ = slot_info(slot)
        frequency = frequencies[symbol]
        if thumb is not None:
            thumb_difference += frequency * (1 if thumb == 0 else -1)
        index_difference += frequency * (bool(bank & 1) - bool(bank & 6))
    return (WEIGHTS["thumb_imbalance_squared"] * thumb_difference**2
            + WEIGHTS["index_imbalance_squared"] * index_difference**2)


def skip_cost(window, assignment):
    first, last = slot_info(assignment[window[0]]), slot_info(assignment[window[-1]])
    thumb = first[1]
    if thumb is None or thumb != last[1]:
        return 0.0
    if any(slot_info(assignment[i])[1] == thumb for i in window[1:-1]):
        return 0.0
    return WEIGHTS["thumb_travel"] * math.dist(first[2], last[2])


def objective(assignment, stats, prefer_rolls=True):
    edges = EDGE if prefer_rolls else LOAD_EDGE
    return (
        sum(stats["counts"][i] * HOLD[s] for i, s in enumerate(assignment))
        + sum(f * edges[shift][assignment[a]][assignment[b]] for a, b, shift, f in stats["edges"])
        + sum(f * skip_cost(window, assignment) for window, f in stats["skips"])
        + balance(assignment, stats["counts"], stats["index_offset"])
    )


def optimize(stats, seed, iterations, pins=None, prefer_rolls=True):
    """Reference search for small corpora; the CLI uses the equivalent Rust search."""
    pins = pins or {}
    rng = random.Random(seed)
    assignment = list(range(SIZE))
    for c, s in pins.items():
        assignment[SYMBOLS.index(c)] = s
    groups = [[SYMBOLS.index(c) for c in group if c not in pins] for group in GROUPS]
    for symbols, slots in zip(groups, [[s for s in range(24) if s not in pins.values()], list(range(24, 32))]):
        rng.shuffle(slots)
        for symbol, slot in zip(symbols, slots):
            assignment[symbol] = slot
    swaps = [(a, b) for group in groups for i, a in enumerate(group) for b in group[i + 1:]]
    current = objective(assignment, stats, prefer_rolls)
    best, best_score = assignment[:], current
    for step in range(iterations):
        a, b = rng.choice(swaps)
        assignment[a], assignment[b] = assignment[b], assignment[a]
        score = objective(assignment, stats, prefer_rolls)
        temperature = 0.025 * (0.00002 / 0.025) ** (step / max(iterations - 1, 1))
        if score < current or rng.random() < math.exp((current - score) / temperature):
            current = score
            if score < best_score:
                best, best_score = assignment[:], score
        else:
            assignment[a], assignment[b] = assignment[b], assignment[a]
    assignment = best
    while True:
        choices = []
        for a, b in swaps:
            assignment[a], assignment[b] = assignment[b], assignment[a]
            choices.append((objective(assignment, stats, prefer_rolls), a, b))
            assignment[a], assignment[b] = assignment[b], assignment[a]
        score, a, b = min(choices)
        if score >= best_score - 1e-12:
            return assignment
        assignment[a], assignment[b] = assignment[b], assignment[a]
        best_score = score


def validate(assignment, pins=None):
    if sorted(assignment) != list(range(SIZE)) or assignment[32:] != list(range(32, SIZE)):
        raise ValueError("Every position must be occupied once; non-alpha positions are fixed")
    for group, lower, upper in zip(GROUPS, [0, 24], [24, 32]):
        if any(not lower <= assignment[SYMBOLS.index(c)] < upper for c in group):
            raise ValueError("A character is outside its allowed alpha bank")
    if any(assignment[SYMBOLS.index(c)] != slot for c, slot in (pins or {}).items()):
        raise ValueError("A pinned character moved")


def metrics(assignment, stats):
    counts = stats["raw_counts"]
    letters = sum(counts[:26])
    banks, thumbs = [0] * 4, [0, 0]
    for i in range(26):
        bank, thumb, _ = slot_info(assignment[i])
        banks[bank] += counts[i]
        thumbs[thumb] += counts[i]
    pairs = [(a, b, f) for a, b, f in stats["pairs"] if a < 26 and b < 26]
    total_pairs = sum(f for _, _, f in pairs)
    roll_pairs = [(a, b, f) for a, b, shift, f in stats["edges"] if a < 26 and b < 26 and not shift]
    return {
        "objective": objective(assignment, stats),
        "load_objective": objective(assignment, stats, False),
        "skipgram_travel_cost": sum(f * skip_cost(w, assignment) for w, f in stats["skips"]),
        "letter_count": letters,
        "letter_bank_percent": {LABELS[i]: 100 * n / letters for i, n in enumerate(banks)},
        "letter_thumb_percent": {side: 100 * n / letters for side, n in zip(["left", "right"], thumbs)},
        "adjacent_letter_alternation_percent": 100 * sum(
            f for a, b, f in pairs if slot_info(assignment[a])[1] != slot_info(assignment[b])[1]
        ) / total_pairs,
        "adjacent_letter_roll_opportunity_percent": 100 * sum(
            f for a, b, f in roll_pairs if roll_quality(assignment[a], assignment[b]) > 0
        ) / total_pairs,
        "adjacent_letter_comfortable_roll_percent": 100 * sum(
            f for a, b, f in roll_pairs if roll_quality(assignment[a], assignment[b]) == 1
        ) / total_pairs,
        "space_rearms_per_100_characters": 100 * sum(
            f for a, b, _, f in stats["edges"] if b == SPACE and slot_info(assignment[a])[0] & 2
        ),
        "one_shot_shift_count": stats["uppercase_count"] + stats["shifted_symbol_count"],
        "uppercase_count": stats["uppercase_count"],
        "shifted_symbol_count": stats["shifted_symbol_count"],
        "thumb_button_presses_per_letter": 1.0,
        "same_side_shoulder_pairs_per_character_in_canonical_plan": 0,
        "modeled_characters": stats["modeled_characters"],
        "unsupported_characters": 0,
    }


def keyboard_modifiers(left="neutral", right="neutral", held_shift=()):
    directions = {
        "neutral": (0, 0), "up": (0, -1), "down": (0, 1),
        "left": (-1, 0), "right": (1, 0), "up-left": (-1, -1),
        "up-right": (1, -1), "down-left": (-1, 1), "down-right": (1, 1),
    }
    if set(held_shift) - {"L2"}:
        raise ValueError("Held Shift uses L2")
    modifiers = {"shift"} if held_shift else set()
    for side, direction in [("left", left), ("right", right)]:
        x, y = directions[direction]
        inward = x > 0 if side == "left" else x < 0
        if inward:
            modifiers.update({-1: {"ctrl", "alt", "shift", "meta"},
                              0: {"ctrl", "alt", "meta"}, 1: {"alt", "meta"}}[y])
        else:
            if x:
                modifiers.add("ctrl")
            if y < 0:
                modifiers.add("alt")
            if y > 0:
                modifiers.add("meta")
    return tuple(m for m in ("ctrl", "alt", "shift", "meta") if m in modifiers)


@dataclass
class Stroke:
    control: int
    bank: int
    started: int
    modifiers: tuple[str, ...] = ()
    mode: str = "alpha"
    frozen: bool = False
    released: bool = False


class StrokeCollector:
    """Prototype alpha stroke capture; app integration and output stay separate."""

    def __init__(self, window_ms=LATE_BUMPER_WINDOW_MS, tap_max_ms=TRIGGER_TAP_MAX_MS):
        self.window_ms = window_ms
        self.tap_max_ms = tap_max_ms
        self.held = 0
        self.active = {}
        self.pending = deque()
        self.shift_buttons = set()
        self.sticks = {"left": "neutral", "right": "neutral"}
        self.shift_used = False
        self.shift_started = 0
        self.one_shot_shift = False
        self.r2_down = False
        self.r2_started = 0
        self.r2_used = False
        self.one_shot_r2 = 0

    def press_shift(self, button, now=0):
        keyboard_modifiers(held_shift=(button,))
        if button in self.shift_buttons:
            return
        self.shift_buttons.add(button)
        self.shift_used = False
        self.shift_started = now

    def release_shift(self, button, now=0):
        if button in self.shift_buttons and not self.shift_used and now - self.shift_started <= self.tap_max_ms:
            self.one_shot_shift = not self.one_shot_shift
        self.shift_buttons.discard(button)

    def set_stick(self, side, direction):
        keyboard_modifiers(**{side: direction})
        self.sticks[side] = direction

    def press_r2(self, now):
        if self.r2_down:
            return
        self.r2_down = True
        self.r2_started = now
        self.r2_used = bool(self.active) or bool(self.held & 2)
        for stroke in self.active.values():
            stroke.frozen = True

    def release_r2(self, now):
        if not self.r2_down:
            return
        if not self.r2_used and now - self.r2_started <= self.tap_max_ms:
            self.one_shot_r2 = (self.one_shot_r2 + 1) % 3
        self.r2_down = False

    def press_thumb(self, control, now):
        if control not in range(8) or control in self.active:
            raise ValueError("Thumb button is invalid or already down")
        for stroke in self.active.values():
            stroke.frozen = True
        mode = "alpha"
        if self.r2_down and self.held & 2:
            mode = "navigation"
        elif self.one_shot_r2 == 2:
            mode = "function"
        elif self.r2_down or self.one_shot_r2 == 1:
            mode = "number"
        shift = self.shift_buttons | ({"L2"} if self.one_shot_shift else set())
        stroke = Stroke(control, self.held if mode == "alpha" else self.held & 1, now,
                        keyboard_modifiers(**self.sticks, held_shift=shift), mode=mode)
        self.shift_used |= bool(self.shift_buttons)
        self.r2_used |= self.r2_down
        self.one_shot_shift = False
        self.one_shot_r2 = 0
        self.active[control] = stroke
        self.pending.append(stroke)

    def press_bumper(self, bumper, now):
        bit = {"L1": 1, "R1": 2}[bumper]
        if self.held & bit:
            return
        self.held |= bit
        self.shift_used |= bool(self.shift_buttons)
        if self.r2_down:
            self.r2_used = True
        if self.pending:
            stroke = self.pending[-1]
            if not stroke.released and not stroke.frozen and (stroke.mode == "alpha" or bumper == "L1"):
                if now - stroke.started < self.window_ms:
                    stroke.bank |= bit
                else:
                    stroke.frozen = True

    def release_bumper(self, bumper):
        self.held &= ~{"L1": 1, "R1": 2}[bumper]

    def release_thumb(self, control):
        return [(stroke.control, stroke.bank) for stroke in self.release_strokes(control)]

    def release_strokes(self, control):
        stroke = self.active.pop(control)
        stroke.frozen = stroke.released = True
        completed = []
        while self.pending and self.pending[0].released:
            stroke = self.pending.popleft()
            completed.append(stroke)
        return completed
