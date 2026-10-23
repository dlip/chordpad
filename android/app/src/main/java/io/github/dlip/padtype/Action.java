package io.github.dlip.padtype;

import org.json.JSONException;
import org.json.JSONObject;

public record Action(String type, String value, int modifiers) {
    public static final int CTRL = 1, ALT = 2, SHIFT = 4, META = 8;

    public static Action text(String text) { return new Action("character", text, 0); }
    public static Action key(String key) { return new Action("key", key, 0); }
    public Action modified(int mask) { return new Action(type, value, modifiers | mask); }

    public static Action read(JSONObject object) throws JSONException {
        String type = object.getString("type");
        if (type.equals("shortcut")) {
            JSONObject shortcut = object.getJSONObject("value");
            int modifiers = 0;
            for (int i = 0; i < shortcut.getJSONArray("modifiers").length(); i++) {
                modifiers |= modifier(shortcut.getJSONArray("modifiers").getString(i));
            }
            return key(shortcut.getString("key")).modified(modifiers);
        }
        if (!type.equals("key") && !type.equals("character") && !type.equals("modifier") && !type.equals("system")) {
            throw new JSONException("Unsupported action: " + type);
        }
        return new Action(type, object.getString("value"), object.optInt("modifiers", 0));
    }

    public JSONObject json() throws JSONException {
        return new JSONObject().put("type", type).put("value", value).put("modifiers", modifiers);
    }

    public static int modifier(String name) {
        return switch (name) {
            case "ctrl" -> CTRL;
            case "alt" -> ALT;
            case "shift" -> SHIFT;
            case "meta" -> META;
            default -> throw new IllegalArgumentException("Unknown modifier: " + name);
        };
    }

    public String label() {
        String label = switch (value) {
            case " " -> "Space";
            case "\n", "enter" -> "Enter";
            case "\t", "tab" -> "Tab";
            case "escape" -> "Escape";
            case "backspace" -> "Backspace";
            default -> value;
        };
        StringBuilder prefix = new StringBuilder();
        for (String name : new String[]{"ctrl", "alt", "shift", "meta"}) {
            if ((modifiers & modifier(name)) != 0) prefix.append(name).append(" + ");
        }
        return prefix + label;
    }
}
