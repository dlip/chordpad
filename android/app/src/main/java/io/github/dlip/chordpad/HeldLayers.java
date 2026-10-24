package io.github.dlip.chordpad;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import static io.github.dlip.chordpad.ChordLayout.*;

final class HeldLayers {
    record Change(boolean changed, boolean function, Frame frame, List<Action> actions) {}
    private record Repeating(Action action, long at) {}
    private enum Mode { NORMAL, NAVIGATION, FUNCTION, AMBIGUOUS }
    private Mode mode = Mode.NORMAL;
    private boolean blockedDpad;
    private int previousButtons, previousDpad, blockedButtons;
    private Map<String,Repeating> active = new LinkedHashMap<>();
    private NavigationFace face = new NavigationFace();

    Change update(Frame frame, ChordLayout layout, long now) {
        int buttons = frame.buttons(), direction = frame.dpad();
        boolean r2 = (buttons & R2) != 0;
        boolean navigation = layout.navigationLayer && r2 && (buttons & L1) != 0;
        boolean function = layout.functionLayer && r2 && (buttons & R1) != 0;
        Mode nextMode = navigation && function ? Mode.AMBIGUOUS : navigation ? Mode.NAVIGATION : function ? Mode.FUNCTION : Mode.NORMAL;
        boolean changed = nextMode != mode;
        boolean functionExit = mode == Mode.FUNCTION && nextMode == Mode.NORMAL;
        if (changed) face = new NavigationFace();
        blockedButtons &= buttons;
        if (direction == 0) blockedDpad = false;
        if (changed && !functionExit) {
            blockedButtons |= previousButtons & buttons & 0x3f;
            blockedDpad = previousDpad != 0 && direction != 0;
            if (mode != Mode.NORMAL) {
                blockedButtons |= buttons & 0x0f;
                blockedDpad = direction != 0;
            }
        }
        if (nextMode == Mode.NAVIGATION) blockedButtons |= L1;
        if (nextMode == Mode.FUNCTION) blockedButtons |= R1;
        if (nextMode == Mode.AMBIGUOUS) blockedButtons |= L1 | R1;
        mode = nextMode; previousButtons = buttons; previousDpad = direction;
        List<Action> output = new ArrayList<>();
        Map<String,Repeating> next = new LinkedHashMap<>();
        if (mode == Mode.NAVIGATION) {
            int held = (R2 | L1) | (buttons & L2);
            int modifiers = frame.modifiers() | ((buttons & L2) != 0 ? Action.SHIFT : 0);
            if (!blockedDpad && layout.allows(held,direction)) {
                String key = switch (direction) { case 1 -> "left"; case 2 -> "right"; case 4 -> "up"; case 8 -> "down"; default -> null; };
                if (key != null) add(next,output,key,modifiers,now);
            }
            String[] keys = {"page_down","end","home","page_up"};
            if (layout.navigationMedia) output.addAll(face.update(buttons & ~blockedButtons & 0x0f,held,frame.modifiers(),layout,now));
            for (int i = 0; i < 4; i++) if (!layout.navigationMedia && (buttons & ~blockedButtons & (1 << i)) != 0 && layout.allows(held | (1 << i),0)) {
                add(next,output,keys[i],modifiers,now);
            }
        }
        active = next;
        Frame filtered = new Frame(buttons & ~blockedButtons & (mode == Mode.NAVIGATION ? ~0x0f : mode == Mode.AMBIGUOUS ? ~0x3f : 0xff),
                mode == Mode.NAVIGATION || mode == Mode.AMBIGUOUS || blockedDpad ? 0 : direction,frame.modifiers(),frame.menu(),frame.leftX(),frame.leftY(),frame.rightX(),frame.rightY());
        return new Change(changed && !functionExit,mode == Mode.FUNCTION,filtered,output);
    }
    private void add(Map<String,Repeating> next,List<Action> output,String key,int modifiers,long now) {
        Action action = Action.key(key).modified(modifiers);
        Repeating previous = active.get(key);
        if (previous == null) output.add(action);
        next.put(key,new Repeating(action,previous == null ? now + 400 : previous.at()));
    }
    List<Action> repeat(long now) {
        List<Action> output = new ArrayList<>();
        active.replaceAll((key,entry) -> {
            if (now < entry.at()) return entry;
            output.add(entry.action()); return new Repeating(entry.action(),now + 80);
        });
        output.addAll(face.repeat(now));
        return output;
    }
}
