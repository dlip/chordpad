package io.github.dlip.padtype;

import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import static io.github.dlip.padtype.ChordLayout.*;

final class LayoutReference {
    record Cell(String output, String controls) {}
    record Map(String title, List<Cell> cells) {}
    record Layer(String title, String note, List<Map> maps, List<Cell> chords) {}
    private static final int[] DIRECTIONS = {5,4,6,1,0,2,9,8,10};
    private static final int[] FACES = {X|Y,Y,Y|B,X,0,B,A|X,A,A|B};
    private static final String[] ARROWS = {"↖","↑","↗","←","●","→","↙","↓","↘"};

    static List<Layer> layers(ChordLayout layout) {
        List<Layer> layers = new ArrayList<>();
        List<Binding> typing = new ArrayList<>(layout.bindings);
        for (Binding binding : layout.bindings) {
            if (binding.held() == 0 && binding.dpad() == 0 && (binding.buttons() == L1 || binding.buttons() == R1)
                    && layout.allows(binding.buttons() | L2,0)) {
                typing.add(new Binding(binding.buttons(),0,L2,binding.action()));
            }
        }
        String paired = "Pairs show normal / L2 Shift. A single label applies to both. Press adjacent controls together for diagonals. In shoulder maps, the center is the shoulder alone. · means unassigned.";
        layers.add(layer("Base + L2: Shift",paired,typing,0,L2));
        if (typing.stream().anyMatch(b -> (b.held() & R2) != 0)) {
            layers.add(layer("R2: numbers and symbols","Hold R2. " + paired,typing,R2,R2|L2));
        }
        if (layout.navigationLayer) {
            int held = R2|L1;
            List<Binding> navigation = new ArrayList<>();
            for (int dpad : new int[]{4,8,1,2}) {
                if (layout.allows(held,dpad)) navigation.add(new Binding(0,dpad,held,Action.key(switch (dpad) {
                    case 4 -> "up"; case 8 -> "down"; case 1 -> "left"; default -> "right";
                })));
            }
            for (int mask = 1; mask < 16; mask++) {
                String key = layout.navigationFace(mask);
                if (key != null && layout.allows(held|mask,0)) navigation.add(new Binding(mask,0,held,Action.key(key)));
            }
            String note = "Hold R2 + L1. Add L2 for Shift selection. D-pad arrows send immediately and repeat while held; D-pad diagonals are unassigned.";
            if (layout.navigationMedia) note += " Face taps send on release. Hold a single face button to repeat; adjacent pairs send Previous, Next, Delete, and Play/Pause once. Media controls ignore modifiers.";
            layers.add(layer("R2 + L1: navigation",note,navigation,held));
        }
        if (layout.functionLayer) {
            String note = "Hold R2 + R1. Use the number combos for F1–F10. Release the ordinary controls to send; keep the layer held for sequential inputs. Add L2 for Shift.";
            if (layout.functionVolume) note += " Volume pairs send once and ignore modifiers.";
            layers.add(layer("R2 + R1: function keys",note,layout.functionBindings,R2|R1));
        }
        return layers;
    }

    private static Layer layer(String title, String note, List<Binding> bindings, int... contexts) {
        List<Binding> selected = bindings.stream().filter(b -> {
            for (int held : contexts) if (b.held() == held) return true;
            return false;
        }).toList();
        List<Map> maps = new ArrayList<>();
        for (int shoulder : new int[]{0,L1,R1}) {
            if (shoulder != 0 && selected.stream().noneMatch(b -> (b.buttons() & shoulder) != 0)) continue;
            for (boolean face : new boolean[]{false,true}) {
                if ((shoulder == L1 && face) || (shoulder == R1 && !face)) continue;
                List<Cell> cells = new ArrayList<>();
                for (int i = 0; i < 9; i++) {
                    int buttons = shoulder | (face ? FACES[i] : 0), dpad = face ? 0 : DIRECTIONS[i];
                    String controls = face ? faceControls(FACES[i]) : ARROWS[i];
                    if (shoulder != 0) controls = i == 4 ? buttons(shoulder) + " alone" : controls + " + " + buttons(shoulder);
                    cells.add(new Cell(outputs(selected,buttons,dpad,contexts),controls));
                }
                maps.add(new Map((face ? "Face buttons" : "D-pad") + (shoulder == 0 ? "" : " + " + buttons(shoulder)),cells));
            }
        }
        java.util.Map<Integer,Binding> combos = new LinkedHashMap<>();
        for (Binding binding : selected) combos.putIfAbsent(binding.buttons() | (binding.dpad() << 8),binding);
        List<Cell> chords = new ArrayList<>();
        for (Binding binding : combos.values()) {
            String controls = controls(binding.buttons(),binding.dpad());
            int held = contexts[0];
            if (held != 0) controls = "Hold " + buttons(held) + "; " + controls;
            chords.add(new Cell(outputs(selected,binding.buttons(),binding.dpad(),contexts),controls));
        }
        return new Layer(title,note,maps,chords);
    }

    private static String outputs(List<Binding> bindings, int buttons, int dpad, int[] contexts) {
        List<String> labels = new ArrayList<>();
        for (int held : contexts) {
            labels.add(bindings.stream().filter(b -> b.buttons() == buttons && b.dpad() == dpad && b.held() == held)
                    .findFirst().map(b -> action(b.action())).orElse("·"));
        }
        return labels.stream().distinct().count() == 1 ? labels.get(0) : String.join(" / ",labels);
    }

    static List<Map> sticks() {
        List<Map> maps = new ArrayList<>();
        for (boolean left : new boolean[]{true,false}) {
            List<Cell> cells = new ArrayList<>();
            for (int i = 0; i < 9; i++) {
                int d = DIRECTIONS[i], x = (d&1)!=0 ? -1 : (d&2)!=0 ? 1 : 0, y = (d&4)!=0 ? -1 : (d&8)!=0 ? 1 : 0;
                int modifiers = ControllerInput.modifiers(x,y,left);
                cells.add(new Cell(modifiers == 0 ? "Neutral" : modifiers(modifiers),ARROWS[i]));
            }
            maps.add(new Map(left ? "Left stick" : "Right stick",cells));
        }
        return maps;
    }

    private static String modifiers(int mask) {
        List<String> names = new ArrayList<>();
        if ((mask & Action.CTRL) != 0) names.add("Ctrl");
        if ((mask & Action.ALT) != 0) names.add("Alt");
        if ((mask & Action.SHIFT) != 0) names.add("Shift");
        if ((mask & Action.META) != 0) names.add("Command");
        return String.join(" + ",names);
    }

    private static String action(Action action) {
        String name = switch (action.value()) {
            case "up" -> "↑"; case "down" -> "↓"; case "left" -> "←"; case "right" -> "→";
            case "page_up" -> "Page Up"; case "page_down" -> "Page Down";
            case "media_play_pause" -> "Play/Pause"; case "media_previous" -> "Previous";
            case "media_next" -> "Next"; case "media_stop" -> "Stop";
            case "volume_up" -> "Volume Up"; case "volume_down" -> "Volume Down"; case "mute" -> "Mute";
            default -> new Action(action.type(),action.value(),0).label();
        };
        if (action.type().equals("key") && action.value().matches("f[0-9]+")) name = action.value().toUpperCase(java.util.Locale.ROOT);
        else if (action.type().equals("key") && !name.isEmpty()) name = Character.toUpperCase(name.charAt(0)) + name.substring(1);
        return action.modifiers() == 0 ? name : modifiers(action.modifiers()) + " + " + name;
    }

    private static String faceControls(int mask) {
        return switch (mask) {
            case A -> "A / ×"; case B -> "B / ○"; case X -> "X / □"; case Y -> "Y / △";
            case 0 -> "●"; default -> buttons(mask);
        };
    }

    private static String controls(int mask, int dpad) {
        String direction = switch (dpad) {
            case 1 -> "Left"; case 2 -> "Right"; case 4 -> "Up"; case 8 -> "Down";
            case 5 -> "Up Left"; case 6 -> "Up Right"; case 9 -> "Down Left"; case 10 -> "Down Right";
            default -> "";
        };
        return (dpad == 0 ? "" : "D-pad " + direction + (mask == 0 ? "" : " + ")) + buttons(mask);
    }

    private static String buttons(int mask) {
        List<String> names = new ArrayList<>();
        String[] labels = {"A","B","X","Y","L1","R1","L2","R2"};
        for (int bit = 0; bit < labels.length; bit++) if ((mask & (1 << bit)) != 0) names.add(labels[bit]);
        return String.join(" + ",names);
    }
}
