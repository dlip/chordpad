package io.github.dlip.padtype;

import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

public final class ChordLayout {
    public static final int A = 1, B = 2, X = 4, Y = 8, L1 = 16, R1 = 32, L2 = 64, R2 = 128;
    public record Binding(int buttons, int dpad, int held, Action action) {}
    public final List<Binding> bindings = new ArrayList<>();
    public final boolean navigationLayer;
    public final boolean functionLayer;
    public final List<Binding> functionBindings = new ArrayList<>();
    private final Map<Integer,Action> functionActions = new HashMap<>();
    private final Map<Integer, Action> actions = new HashMap<>();
    private final Set<Integer> forbiddenChords = new HashSet<>();
    private final List<Integer> forbiddenCombinations = new ArrayList<>();
    private final boolean diagonals, excludeOpposites;
    private final int disabled, modifierOnly, maxControls;

    public ChordLayout(String json) throws JSONException {
        JSONObject object = new JSONObject(json);
        int version = object.getInt("schema_version");
        if (version != 2 || object.getInt("cost_model_version") != 2) {
            throw new JSONException("Use a version 2 independent padtype layout");
        }
        JSONObject strategy = object.getJSONObject("strategy");
        if (!strategy.optBoolean("independent") || !"L2".equals(strategy.optString("shift"))
                || !"R2".equals(strategy.optString("layer"))
                || !"independent-release".equals(object.getString("recognition"))) {
            throw new JSONException("This app supports independent layouts with L2 Shift and R2 symbols");
        }
        JSONObject profile = object.getJSONObject("profile");
        navigationLayer = profile.optBoolean("navigation_layer");
        functionLayer = profile.optBoolean("function_layer");
        int[] extraButtons = {0,0}, extraDpad = {1,8};
        JSONArray extras = profile.optJSONArray("extra_function_chords");
        if (profile.has("extra_function_chords") && (extras == null || extras.length() != 2)) throw new JSONException("F11/F12 require two extra function combos");
        if (extras != null) for (int i = 0; i < 2; i++) {
            JSONObject chord = extras.getJSONObject(i);
            extraButtons[i] = buttons(chord.has("buttons") ? chord.getJSONArray("buttons") : null); extraDpad[i] = direction(chord.optString("dpad","neutral"));
        }
        for (int i = 0; i < 2; i++) if ((extraButtons[i] & ~0x0f) != 0 || (extraDpad[i] != 0 && extraButtons[i] != 0) || (extraDpad[i] == 0 && extraButtons[i] == 0)) throw new JSONException("F11/F12 require D-pad or face-button combos without shoulders");
        if (extraButtons[0] == extraButtons[1] && extraDpad[0] == extraDpad[1]) throw new JSONException("F11/F12 combos must differ");
        diagonals = profile.optBoolean("diagonals", true);
        excludeOpposites = profile.optBoolean("exclude_opposite_face_pairs");
        disabled = buttons(profile.optJSONArray("disabled_buttons"));
        modifierOnly = profile.has("modifier_only_buttons") ? buttons(profile.getJSONArray("modifier_only_buttons")) : L2 | R2;
        maxControls = profile.optInt("max_chord_size", 0);
        JSONArray combinations = profile.optJSONArray("forbidden_combinations");
        if (combinations != null) for (int i = 0; i < combinations.length(); i++) {
            forbiddenCombinations.add(buttons(combinations.getJSONArray(i)));
        }
        JSONArray chords = profile.optJSONArray("forbidden_chords");
        if (chords != null) for (int i = 0; i < chords.length(); i++) {
            JSONObject chord = chords.getJSONObject(i);
            forbiddenChords.add(slot(buttons(chord.optJSONArray("buttons")), direction(chord.optString("dpad", "neutral")), 0));
        }
        Set<String> punctuation = new HashSet<>();
        JSONArray base = profile.optJSONArray("base_punctuation");
        if (base != null) for (int i = 0; i < base.length(); i++) punctuation.add(base.getString(i));
        Map<String, Binding> groups = new HashMap<>();
        Set<String> outputs = new HashSet<>();
        JSONArray array = object.getJSONArray("bindings");
        if (array.length() == 0 || array.length() > 4096) throw new JSONException("Invalid binding count");
        for (int i = 0; i < array.length(); i++) {
            JSONObject item = array.getJSONObject(i), chord = item.getJSONObject("chord");
            int mask = buttons(chord.optJSONArray("buttons"));
            int dpad = direction(chord.optString("dpad", "neutral"));
            int held = buttons(item.optJSONArray("held"));
            Action action = Action.read(item.getJSONObject("action"));
            if (!action.type().equals("key") && !action.type().equals("character")) throw new JSONException("Invalid chord action");
            if ((held & ~(L2 | R2)) != 0 || !item.optString("held_dpad", "neutral").equals("neutral")
                    || (mask & (L2 | R2 | modifierOnly)) != 0 || (mask == 0 && dpad == 0)
                    || (dpad != 0 && (mask & ~L1) != 0) || ((mask & L1) != 0 && (mask & ~L1) != 0)
                    || (navigationLayer && (held & R2) != 0 && (mask & L1) != 0)
                    || (functionLayer && (held & R2) != 0 && (mask & R1) != 0)
                    || !allows(mask | held, dpad)) throw new JSONException("Forbidden or mixed-half chord at binding " + i);
            String plain = action.value();
            boolean shifted = false;
            if (action.type().equals("character")) {
                if (plain.codePointCount(0, plain.length()) != 1) throw new JSONException("A character binding must contain one character");
                String lower = "abcdefghijklmnopqrstuvwxyz`1234567890-=[]\\;',./";
                String upper = "ABCDEFGHIJKLMNOPQRSTUVWXYZ~!@#$%^&*()_+{}|:\"<>?";
                int index = upper.indexOf(plain);
                if (index >= 0) { shifted = true; plain = lower.substring(index, index + 1); }
                if (Character.isAlphabetic(action.value().codePointAt(0)) && (held & R2) != 0) throw new JSONException("Letters must use the base layer");
                if (!Character.isWhitespace(action.value().codePointAt(0)) && !Character.isISOControl(action.value().codePointAt(0))
                        && !Character.isAlphabetic(action.value().codePointAt(0)) && !punctuation.contains(plain) && (held & R2) == 0) {
                    throw new JSONException("Base punctuation is not permitted by the profile");
                }
            }
            if (((held & L2) != 0) != shifted) throw new JSONException("Invalid Shift pairing");
            Action fixed = fixed(mask, dpad);
            if (fixed != null && (!fixed.equals(action) || held != 0)) throw new JSONException("L1 is Backspace and R1 is Space");
            if (fixed == null && (action.equals(Action.text(" ")) || action.equals(Action.key("backspace")) || action.equals(Action.key("space")))) {
                throw new JSONException("Space and Backspace must use R1 and L1");
            }
            Binding binding = new Binding(mask, dpad, held, action);
            String group = action.type() + ":" + plain + ":" + action.modifiers();
            Binding previous = groups.putIfAbsent(group, binding);
            if (previous != null && (previous.buttons != mask || previous.dpad != dpad || (previous.held & R2) != (held & R2))) {
                throw new JSONException("Shift pairs must share a chord and layer");
            }
            if (!outputs.add(action.type() + ":" + action.value() + ":" + action.modifiers())
                    || actions.putIfAbsent(slot(mask, dpad, held), action) != null) throw new JSONException("Duplicate action or chord");
            bindings.add(binding);
        }
        if (functionLayer) {
            for (int digit = 0; digit < 10; digit++) {
                Binding number = null;
                for (Binding binding : bindings) if (binding.held() == R2 && binding.action().equals(Action.text(Integer.toString(digit)))) number = binding;
                if (number == null || (number.buttons() & (L1 | R1)) != 0
                        || (number.buttons() == extraButtons[0] && number.dpad() == extraDpad[0])
                        || (number.buttons() == extraButtons[1] && number.dpad() == extraDpad[1])) {
                    throw new JSONException("The function layer requires all ten number combos without L1/R1 or F11/F12 conflicts");
                }
                functionBinding(number.buttons(),number.dpad(),"f" + (digit == 0 ? 10 : digit));
            }
            functionBinding(extraButtons[0],extraDpad[0],"f11"); functionBinding(extraButtons[1],extraDpad[1],"f12");
        }
    }

    private void functionBinding(int mask,int dpad,String key) throws JSONException {
        if (!allows(mask | R1 | R2,dpad)) throw new JSONException("A function combo is forbidden by the profile");
        Action action = Action.key(key);
        functionBindings.add(new Binding(mask,dpad,R1 | R2,action));
        functionActions.put(slot(mask,dpad,0),action);
    }
    public Action resolveFunction(int mask,int dpad,int held,int modifiers) {
        if (!allows(mask | held | R1 | R2,dpad)) return null;
        Action action = functionActions.get(slot(mask,dpad,0));
        return action == null ? null : action.modified(modifiers | ((held & L2) != 0 ? Action.SHIFT : 0));
    }

    public boolean allows(int mask, int dpad) {
        if ((mask & disabled) != 0 || (!diagonals && Integer.bitCount(dpad) > 1)
                || (maxControls > 0 && Integer.bitCount(mask) + Integer.bitCount(dpad) > maxControls)
                || (excludeOpposites && ((mask & 9) == 9 || (mask & 6) == 6))) return false;
        for (int combination : forbiddenCombinations) if ((mask & combination) == combination) return false;
        return !forbiddenChords.contains(slot(mask, dpad, 0));
    }

    public Action resolve(int mask, int dpad, int held, int modifiers) {
        if (!allows(mask | held, dpad)) return null;
        Action fixed = fixed(mask, dpad);
        if (fixed != null) return actions.containsKey(slot(mask, dpad, 0))
                ? fixed.modified(modifiers | ((held & L2) != 0 ? Action.SHIFT : 0)) : null;
        Action action = actions.get(slot(mask, dpad, held));
        return action == null ? null : action.modified(modifiers);
    }

    private static Action fixed(int mask, int dpad) {
        if (dpad != 0) return null;
        return mask == L1 ? Action.key("backspace") : mask == R1 ? Action.text(" ") : null;
    }
    private static int slot(int mask, int dpad, int held) { return mask | (dpad << 8) | (held << 12); }
    public static int buttons(JSONArray array) throws JSONException {
        int mask = 0;
        if (array == null) return mask;
        for (int i = 0; i < array.length(); i++) {
            int bit = switch (array.getString(i)) {
                case "A" -> A; case "B" -> B; case "X" -> X; case "Y" -> Y;
                case "L1" -> L1; case "R1" -> R1; case "L2" -> L2; case "R2" -> R2;
                default -> throw new JSONException("Unknown controller button");
            };
            if ((mask & bit) != 0) throw new JSONException("Duplicate controller button");
            mask |= bit;
        }
        return mask;
    }
    public static int direction(String name) throws JSONException {
        return switch (name) {
            case "neutral" -> 0; case "left" -> 1; case "right" -> 2; case "up" -> 4; case "down" -> 8;
            case "up-left" -> 5; case "up-right" -> 6; case "down-left" -> 9; case "down-right" -> 10;
            default -> throw new JSONException("Unknown D-pad direction: " + name);
        };
    }
}
