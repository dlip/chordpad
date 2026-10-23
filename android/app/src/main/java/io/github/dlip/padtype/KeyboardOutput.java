package io.github.dlip.padtype;

import android.accessibilityservice.InputMethod.AccessibilityInputConnection;
import android.os.SystemClock;
import android.view.InputDevice;
import android.view.KeyCharacterMap;
import android.view.KeyEvent;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;

public final class KeyboardOutput {
    private KeyboardOutput() {}
    public static int keyCode(String key) {
        return switch (key) {
            case "backspace" -> KeyEvent.KEYCODE_DEL;
            case "delete" -> KeyEvent.KEYCODE_FORWARD_DEL;
            case "up" -> KeyEvent.KEYCODE_DPAD_UP;
            case "down" -> KeyEvent.KEYCODE_DPAD_DOWN;
            case "left" -> KeyEvent.KEYCODE_DPAD_LEFT;
            case "right" -> KeyEvent.KEYCODE_DPAD_RIGHT;
            case "home" -> KeyEvent.KEYCODE_MOVE_HOME;
            case "end" -> KeyEvent.KEYCODE_MOVE_END;
            case "mute" -> KeyEvent.KEYCODE_VOLUME_MUTE;
            default -> KeyEvent.keyCodeFromString("KEYCODE_" + key.toUpperCase(Locale.ROOT));
        };
    }
    public static void validate(Action action) {
        if ((action.modifiers() & ~15) != 0) throw new IllegalArgumentException("Invalid keyboard modifiers");
        switch (action.type()) {
            case "character" -> { if (action.value().isEmpty()) throw new IllegalArgumentException("Enter some text"); }
            case "key" -> { if (keyCode(action.value()) == KeyEvent.KEYCODE_UNKNOWN) throw new IllegalArgumentException("Unknown key: " + action.value()); }
            case "modifier" -> Action.modifier(action.value());
            case "system" -> { if (!List.of("back", "home", "recents").contains(action.value())) throw new IllegalArgumentException("Unknown navigation action"); }
            default -> throw new IllegalArgumentException("Unsupported action");
        }
    }
    public static void send(AccessibilityInputConnection connection, Action action) {
        if (action.type().equals("character") && action.modifiers() == 0
                && !action.value().equals("\n") && !action.value().equals("\t")) {
            connection.commitText(action.value(), 1, null);
            return;
        }
        int code, modifiers = action.modifiers();
        if (action.type().equals("character")) {
            if (action.value().length() != 1) throw new IllegalArgumentException("Modified text must be one ASCII character");
            char character = action.value().charAt(0);
            if (character == '\n') code = KeyEvent.KEYCODE_ENTER;
            else if (character == '\t') code = KeyEvent.KEYCODE_TAB;
            else {
                String lower = "abcdefghijklmnopqrstuvwxyz`1234567890-=[]\\;',./ ";
                String upper = "ABCDEFGHIJKLMNOPQRSTUVWXYZ~!@#$%^&*()_+{}|:\"<>? ";
                int index = upper.indexOf(character);
                if (index >= 0 && character != ' ') { character = lower.charAt(index); modifiers |= Action.SHIFT; }
                KeyEvent[] keys = KeyCharacterMap.load(KeyCharacterMap.VIRTUAL_KEYBOARD).getEvents(new char[]{character});
                code = KeyEvent.KEYCODE_UNKNOWN;
                if (keys != null) for (KeyEvent event : keys) if (event.getAction() == KeyEvent.ACTION_DOWN && !KeyEvent.isModifierKey(event.getKeyCode())) { code = event.getKeyCode(); break; }
                if (code == KeyEvent.KEYCODE_UNKNOWN) throw new IllegalArgumentException("This character has no keyboard shortcut key");
            }
        } else code = keyCode(action.value());
        if (code == KeyEvent.KEYCODE_UNKNOWN) throw new IllegalArgumentException("Unsupported keyboard key");
        int[] bits = {Action.CTRL, Action.ALT, Action.SHIFT, Action.META};
        int[] codes = {KeyEvent.KEYCODE_CTRL_LEFT, KeyEvent.KEYCODE_ALT_LEFT, KeyEvent.KEYCODE_SHIFT_LEFT, KeyEvent.KEYCODE_META_LEFT};
        int[] states = {KeyEvent.META_CTRL_ON | KeyEvent.META_CTRL_LEFT_ON, KeyEvent.META_ALT_ON | KeyEvent.META_ALT_LEFT_ON,
                KeyEvent.META_SHIFT_ON | KeyEvent.META_SHIFT_LEFT_ON, KeyEvent.META_META_ON | KeyEvent.META_META_LEFT_ON};
        List<Integer> pressed = new ArrayList<>();
        int meta = 0;
        long time = SystemClock.uptimeMillis();
        try {
            for (int i = 0; i < bits.length; i++) if ((modifiers & bits[i]) != 0) {
                meta |= states[i]; pressed.add(codes[i]);
                connection.sendKeyEvent(event(time, KeyEvent.ACTION_DOWN, codes[i], meta));
            }
            pressed.add(code);
            connection.sendKeyEvent(event(time, KeyEvent.ACTION_DOWN, code, meta));
        } finally {
            for (int i = pressed.size() - 1; i >= 0; i--) {
                int key = pressed.get(i);
                for (int j = 0; j < codes.length; j++) if (codes[j] == key) meta &= ~states[j];
                connection.sendKeyEvent(event(time, KeyEvent.ACTION_UP, key, meta));
            }
        }
    }
    private static KeyEvent event(long downTime, int action, int code, int meta) {
        return new KeyEvent(downTime, SystemClock.uptimeMillis(), action, code, 0, meta,
                KeyCharacterMap.VIRTUAL_KEYBOARD, 0, KeyEvent.FLAG_SOFT_KEYBOARD | KeyEvent.FLAG_KEEP_TOUCH_MODE, InputDevice.SOURCE_KEYBOARD);
    }
}
