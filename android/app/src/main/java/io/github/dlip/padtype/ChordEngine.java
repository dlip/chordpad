package io.github.dlip.padtype;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.List;
import static io.github.dlip.padtype.ChordLayout.*;

public final class ChordEngine {
    private static final int IDLE = 0, COLLECTING = 1, RELEASING = 2, CANCELLED = 3;
    private static final class Pending { boolean done; Action action; }
    private static final class Half {
        int phase, buttons, dpad, frozenButtons, frozenDpad, held, modifiers;
        Pending pending;
    }
    private final ChordLayout layout;
    private final ArrayDeque<Pending> queue = new ArrayDeque<>();
    private Half[] halves;
    private Pending[] menu;
    private boolean[] cancelledMenu;
    private int previousHeld, previousModifiers, previousMenuModifiers;
    private boolean waiting;
    public String status = "Release all controls";

    public ChordEngine(ChordLayout layout) { this.layout = layout; reset(); }
    public boolean ready() { return !waiting; }
    public void reset() {
        queue.clear();
        halves = new Half[]{new Half(), new Half()};
        menu = new Pending[2];
        cancelledMenu = new boolean[2];
        previousHeld = previousModifiers = previousMenuModifiers = 0;
        waiting = true;
        status = "Release all controls";
    }

    public List<Action> update(Frame frame) {
        if (!frame.valid()) { reset(); status = "Invalid D-pad position"; return List.of(); }
        if (waiting) {
            if (frame.neutral()) { waiting = false; status = "Ready"; }
            return List.of();
        }
        int held = frame.buttons() & (L2 | R2);
        boolean newModifier = (held & ~previousHeld) != 0 || (frame.modifiers() & ~previousModifiers) != 0;
        previousHeld = held;
        previousModifiers = frame.modifiers();
        for (int side = 0; side < 2; side++) {
            Half half = halves[side];
            int buttons = frame.buttons() & (side == 0 ? L1 : A | B | X | Y | R1);
            int dpad = side == 0 ? frame.dpad() : 0;
            int current = buttons | (dpad << 8), previous = half.buttons | (half.dpad << 8);
            if (half.phase == IDLE && current != 0) {
                if (queue.size() == 64) return overflow();
                half.pending = new Pending();
                queue.add(half.pending);
                half.held = held;
                half.modifiers = frame.modifiers();
                half.phase = COLLECTING;
            } else if (half.phase == COLLECTING || half.phase == RELEASING) {
                boolean added = (current & ~previous) != 0, released = (previous & ~current) != 0;
                boolean opposite = ((half.dpad & 1) != 0 && (dpad & 2) != 0)
                        || ((half.dpad & 2) != 0 && (dpad & 1) != 0)
                        || ((half.dpad & 4) != 0 && (dpad & 8) != 0)
                        || ((half.dpad & 8) != 0 && (dpad & 4) != 0);
                if (newModifier || opposite || (half.phase == RELEASING && added) || (released && added)) {
                    half.pending.done = true;
                    half.pending = null;
                    half.phase = CANCELLED;
                    status = "Cancelled stroke; release that half";
                } else {
                    if (half.phase == COLLECTING && released) {
                        half.frozenButtons = half.buttons;
                        half.frozenDpad = half.dpad;
                        half.phase = RELEASING;
                    }
                    if (half.phase == RELEASING && current == 0) {
                        half.pending.action = layout.resolve(half.frozenButtons, half.frozenDpad, half.held, half.modifiers);
                        if (half.pending.action == null) status = "Unmapped or forbidden chord";
                        half.pending.done = true;
                        half.pending = null;
                        half.phase = IDLE;
                    }
                }
            }
            if (half.phase == CANCELLED && current == 0) half.phase = IDLE;
            half.buttons = buttons;
            half.dpad = dpad;
        }
        int menuModifiers = frame.modifiers() | ((held & L2) != 0 ? Action.SHIFT : 0);
        boolean addedMenuModifier = (menuModifiers & ~previousMenuModifiers) != 0;
        previousMenuModifiers = menuModifiers;
        for (int i = 0; i < 2; i++) {
            boolean down = (frame.menu() & (1 << i)) != 0;
            if (menu[i] != null && addedMenuModifier) cancelledMenu[i] = true;
            if (down && menu[i] == null) {
                if (queue.size() == 64) return overflow();
                menu[i] = new Pending();
                menu[i].action = Action.key(i == 0 ? "enter" : "escape").modified(menuModifiers);
                queue.add(menu[i]);
            } else if (!down && menu[i] != null) {
                if (cancelledMenu[i]) menu[i].action = null;
                menu[i].done = true;
                menu[i] = null;
                cancelledMenu[i] = false;
            }
        }
        List<Action> actions = new ArrayList<>();
        while (!queue.isEmpty() && queue.peek().done) {
            Action action = queue.remove().action;
            if (action != null) { actions.add(action); status = "Ready"; }
        }
        return actions;
    }

    private List<Action> overflow() { reset(); status = "Too many pending strokes; release all controls"; return List.of(); }
}
