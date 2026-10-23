package io.github.dlip.padtype;

import java.util.ArrayList;
import java.util.List;
import static io.github.dlip.padtype.ChordLayout.*;

final class NavigationFace {
    private static final int IDLE = 0, COLLECTING = 1, RELEASING = 2, CANCELLED = 3;
    private int phase, previous, frozen, held, modifiers, previousHeld, previousModifiers;
    private boolean repeated;
    private Action repeating;
    private long at;

    private static Action resolve(int mask, int held, int modifiers, ChordLayout layout) {
        if (!layout.allows(mask | held,0)) return null;
        String key = layout.navigationFace(mask);
        if (key == null) return null;
        Action action = Action.key(key);
        return action.consumerKey() ? action : action.modified(modifiers | ((held & L2) != 0 ? Action.SHIFT : 0));
    }

    List<Action> update(int mask, int held, int modifiers, ChordLayout layout, long now) {
        boolean newModifier = (held & ~previousHeld) != 0 || (modifiers & ~previousModifiers) != 0;
        previousHeld = held; previousModifiers = modifiers;
        List<Action> output = new ArrayList<>();
        if (phase == IDLE && mask != 0) {
            phase = COLLECTING; this.held = held; this.modifiers = modifiers; repeated = false;
        } else if (phase == COLLECTING || phase == RELEASING) {
            boolean added = (mask & ~previous) != 0, released = (previous & ~mask) != 0;
            if (newModifier || (added && (released || phase == RELEASING || repeated))) phase = CANCELLED;
            else {
                if (phase == COLLECTING && released) { frozen = previous; phase = RELEASING; }
                if (phase == RELEASING && mask == 0) {
                    Action action = resolve(frozen,this.held,this.modifiers,layout);
                    if (!repeated && action != null) output.add(action);
                    phase = IDLE;
                }
            }
        }
        if (phase == CANCELLED && mask == 0) phase = IDLE;
        if (phase != COLLECTING || Integer.bitCount(mask) != 1) repeating = null;
        else if (repeating == null) { repeating = resolve(mask,this.held,this.modifiers,layout); at = now + 400; }
        previous = mask;
        return output;
    }

    List<Action> repeat(long now) {
        if (repeating == null || now < at) return List.of();
        repeated = true; at = now + 80;
        return List.of(repeating);
    }
}
