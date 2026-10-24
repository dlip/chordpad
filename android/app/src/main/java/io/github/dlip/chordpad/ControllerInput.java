package io.github.dlip.chordpad;

import android.view.InputDevice;
import android.view.KeyEvent;
import android.view.MotionEvent;
import static io.github.dlip.chordpad.ChordLayout.*;

public final class ControllerInput {
    private int buttons, dpad, menu, lx, ly, rx, ry;
    public float leftX, leftY, rightX, rightY, l2, r2;
    private final InputDevice device;
    private final boolean hats, analogLeft, analogRight;

    public ControllerInput(InputDevice device) {
        this.device = device;
        hats = range(MotionEvent.AXIS_HAT_X) && range(MotionEvent.AXIS_HAT_Y);
        analogLeft = range(MotionEvent.AXIS_LTRIGGER) || range(MotionEvent.AXIS_BRAKE);
        analogRight = range(MotionEvent.AXIS_RTRIGGER) || range(MotionEvent.AXIS_GAS);
    }
    private boolean range(int axis) { return device.getMotionRange(axis, InputDevice.SOURCE_JOYSTICK) != null; }
    public static boolean controller(InputDevice device) {
        return device != null && (device.supportsSource(InputDevice.SOURCE_GAMEPAD) || device.supportsSource(InputDevice.SOURCE_JOYSTICK));
    }
    public boolean key(KeyEvent event) {
        int code = event.getKeyCode();
        int button = switch (code) {
            case KeyEvent.KEYCODE_BUTTON_A -> A; case KeyEvent.KEYCODE_BUTTON_B -> B;
            case KeyEvent.KEYCODE_BUTTON_X -> X; case KeyEvent.KEYCODE_BUTTON_Y -> Y;
            case KeyEvent.KEYCODE_BUTTON_L1 -> L1; case KeyEvent.KEYCODE_BUTTON_R1 -> R1;
            case KeyEvent.KEYCODE_BUTTON_L2 -> L2; case KeyEvent.KEYCODE_BUTTON_R2 -> R2;
            default -> 0;
        };
        int direction = switch (code) {
            case KeyEvent.KEYCODE_DPAD_LEFT -> 1; case KeyEvent.KEYCODE_DPAD_RIGHT -> 2;
            case KeyEvent.KEYCODE_DPAD_UP -> 4; case KeyEvent.KEYCODE_DPAD_DOWN -> 8;
            default -> 0;
        };
        int extra = switch (code) {
            case KeyEvent.KEYCODE_BUTTON_START -> 1; case KeyEvent.KEYCODE_BUTTON_SELECT -> 2;
            case KeyEvent.KEYCODE_BUTTON_THUMBL -> 4; case KeyEvent.KEYCODE_BUTTON_THUMBR -> 8;
            default -> 0;
        };
        if (button == 0 && direction == 0 && extra == 0) return false;
        if (event.getRepeatCount() != 0 || (event.getAction() != KeyEvent.ACTION_UP && event.getAction() != KeyEvent.ACTION_DOWN)) return true;
        boolean down = event.getAction() == KeyEvent.ACTION_DOWN;
        if (!((button == L2 && analogLeft) || (button == R2 && analogRight))) buttons = down ? buttons | button : buttons & ~button;
        if (!hats) dpad = down ? dpad | direction : dpad & ~direction;
        menu = down ? menu | extra : menu & ~extra;
        return true;
    }
    private float axis(MotionEvent event, int axis, int history) {
        return history < 0 ? event.getAxisValue(axis) : event.getHistoricalAxisValue(axis, history);
    }
    public void motion(MotionEvent event, int history) {
        leftX = axis(event, MotionEvent.AXIS_X, history);
        leftY = axis(event, MotionEvent.AXIS_Y, history);
        rightX = axis(event, range(MotionEvent.AXIS_Z) ? MotionEvent.AXIS_Z : MotionEvent.AXIS_RX, history);
        rightY = axis(event, range(MotionEvent.AXIS_RZ) ? MotionEvent.AXIS_RZ : MotionEvent.AXIS_RY, history);
        lx = direction(leftX, lx); ly = direction(leftY, ly);
        rx = direction(rightX, rx); ry = direction(rightY, ry);
        if (hats) {
            float x = axis(event, MotionEvent.AXIS_HAT_X, history), y = axis(event, MotionEvent.AXIS_HAT_Y, history);
            dpad = (x < -.5f ? 1 : x > .5f ? 2 : 0) | (y < -.5f ? 4 : y > .5f ? 8 : 0);
        }
        if (analogLeft) {
            l2 = axis(event, range(MotionEvent.AXIS_LTRIGGER) ? MotionEvent.AXIS_LTRIGGER : MotionEvent.AXIS_BRAKE, history);
            buttons = trigger(buttons, L2, l2);
        }
        if (analogRight) {
            r2 = axis(event, range(MotionEvent.AXIS_RTRIGGER) ? MotionEvent.AXIS_RTRIGGER : MotionEvent.AXIS_GAS, history);
            buttons = trigger(buttons, R2, r2);
        }
    }
    static int direction(float value, int previous) {
        if (value >= .55f) return 1;
        if (value <= -.55f) return -1;
        if (previous > 0 && value > .35f) return 1;
        if (previous < 0 && value < -.35f) return -1;
        return 0;
    }
    static int trigger(int buttons, int bit, float value) {
        boolean active = (buttons & bit) != 0 ? value > .35f : value >= .65f;
        return active ? buttons | bit : buttons & ~bit;
    }
    static int modifiers(int x, int y, boolean left) {
        if (left ? x > 0 : x < 0) return Action.ALT | Action.META | (y > 0 ? 0 : Action.CTRL) | (y < 0 ? Action.SHIFT : 0);
        return (x != 0 ? Action.CTRL : 0) | (y < 0 ? Action.ALT : y > 0 ? Action.META : 0);
    }
    public Frame frame() { return new Frame(buttons, dpad, modifiers(lx, ly, true) | modifiers(rx, ry, false), menu, lx, ly, rx, ry); }
}
