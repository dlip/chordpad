package io.github.dlip.padtype;

import android.view.KeyEvent;
import java.util.HashMap;
import java.util.Map;

final class GamepadKeys {
    record Result(boolean consumed, boolean layerGesture) {}
    private final Map<Integer,Boolean> held = new HashMap<>();
    private int layers;
    private boolean latched;

    Result update(int code,int action,int repeat,boolean consume) {
        int layer = switch (code) {
            case KeyEvent.KEYCODE_BUTTON_L1 -> 1; case KeyEvent.KEYCODE_BUTTON_R1 -> 2;
            case KeyEvent.KEYCODE_BUTTON_L2 -> 4; case KeyEvent.KEYCODE_BUTTON_R2 -> 8;
            default -> 0;
        };
        boolean gesture = false, consumed = false;
        if (action == KeyEvent.ACTION_DOWN) {
            if (repeat == 0 && !held.containsKey(code)) {
                held.put(code,consume); layers |= layer;
                if (layers == 15 && !latched) { gesture = true; latched = true; }
            }
            consumed = Boolean.TRUE.equals(held.get(code));
        } else if (action == KeyEvent.ACTION_UP) {
            consumed = Boolean.TRUE.equals(held.remove(code)); layers &= ~layer;
            if (layers == 0) latched = false;
        }
        return new Result(consumed,gesture);
    }
    void suppressHeldGesture() { if (layers != 0) latched = true; }
    boolean hasConsumedPresses() { return held.containsValue(true); }
    boolean hasHeldPresses() { return !held.isEmpty(); }
    void reset() { held.clear(); layers = 0; latched = false; }
}
