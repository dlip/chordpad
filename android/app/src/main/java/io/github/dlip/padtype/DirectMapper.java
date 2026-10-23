package io.github.dlip.padtype;

import java.util.ArrayList;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;

public final class DirectMapper {
    public static final String[] CONTROLS = {"A", "B", "X", "Y", "L1", "R1", "L2", "R2",
            "Start", "Select", "L3", "R3", "D-pad Up", "D-pad Down", "D-pad Left", "D-pad Right",
            "Left stick Up", "Left stick Down", "Left stick Left", "Left stick Right",
            "Right stick Up", "Right stick Down", "Right stick Left", "Right stick Right"};
    public final Map<String, Action> mappings = defaults();
    private Set<String> previous = new HashSet<>();
    private final Map<String, Long> repeatAt = new LinkedHashMap<>();
    private int modifiers;

    public static LinkedHashMap<String, Action> defaults() {
        LinkedHashMap<String, Action> result = new LinkedHashMap<>();
        result.put("A", Action.text(" "));
        result.put("B", Action.key("escape"));
        result.put("X", Action.key("backspace"));
        result.put("Y", Action.key("enter"));
        result.put("L1", Action.key("tab"));
        result.put("R1", Action.text(" "));
        result.put("L2", new Action("modifier", "shift", 0));
        result.put("R2", new Action("modifier", "ctrl", 0));
        result.put("Start", Action.key("enter"));
        result.put("Select", Action.key("escape"));
        for (String direction : new String[]{"Up", "Down", "Left", "Right"}) {
            result.put("D-pad " + direction, Action.key(direction.toLowerCase(java.util.Locale.ROOT)));
            result.put("Left stick " + direction, Action.key(direction.toLowerCase(java.util.Locale.ROOT)));
        }
        result.put("Right stick Up", Action.text("w"));
        result.put("Right stick Down", Action.text("s"));
        result.put("Right stick Left", Action.text("a"));
        result.put("Right stick Right", Action.text("d"));
        return result;
    }

    public void reset() { previous.clear(); repeatAt.clear(); modifiers = 0; }
    public List<Action> update(Frame frame, long now) {
        Set<String> active = active(frame);
        modifiers = 0;
        for (String control : active) {
            Action action = mappings.get(control);
            if (action != null && action.type().equals("modifier")) modifiers |= Action.modifier(action.value());
        }
        List<Action> output = new ArrayList<>();
        repeatAt.keySet().retainAll(active);
        for (String control : CONTROLS) {
            Action action = mappings.get(control);
            if (active.contains(control) && !previous.contains(control) && action != null && !action.type().equals("modifier")) {
                output.add(action.modified(modifiers));
                if (repeatable(action)) repeatAt.put(control, now + 400);
            }
        }
        previous = active;
        return output;
    }
    public List<Action> repeat(long now) {
        List<Action> output = new ArrayList<>();
        for (Map.Entry<String, Long> entry : repeatAt.entrySet()) {
            if (now >= entry.getValue()) {
                Action action = mappings.get(entry.getKey());
                if (action != null) output.add(action.modified(modifiers));
                entry.setValue(now + 80);
            }
        }
        return output;
    }
    private static boolean repeatable(Action action) {
        return action.type().equals("character") || Set.of("up", "down", "left", "right", "backspace", "delete").contains(action.value());
    }
    public static Set<String> active(Frame frame) {
        Set<String> active = new HashSet<>();
        for (int i = 0; i < 8; i++) if ((frame.buttons() & (1 << i)) != 0) active.add(CONTROLS[i]);
        for (int i = 0; i < 4; i++) if ((frame.menu() & (1 << i)) != 0) active.add(CONTROLS[8 + i]);
        if ((frame.dpad() & 4) != 0) active.add("D-pad Up");
        if ((frame.dpad() & 8) != 0) active.add("D-pad Down");
        if ((frame.dpad() & 1) != 0) active.add("D-pad Left");
        if ((frame.dpad() & 2) != 0) active.add("D-pad Right");
        if (frame.leftY() < 0) active.add("Left stick Up");
        if (frame.leftY() > 0) active.add("Left stick Down");
        if (frame.leftX() < 0) active.add("Left stick Left");
        if (frame.leftX() > 0) active.add("Left stick Right");
        if (frame.rightY() < 0) active.add("Right stick Up");
        if (frame.rightY() > 0) active.add("Right stick Down");
        if (frame.rightX() < 0) active.add("Right stick Left");
        if (frame.rightX() > 0) active.add("Right stick Right");
        return active;
    }
}
