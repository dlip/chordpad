package io.github.dlip.chordpad;

import org.json.JSONArray;
import org.json.JSONObject;
import java.util.ArrayList;
import java.util.List;

public final class RecognitionVectors {
    private RecognitionVectors() {}
    private static int[] stick(String direction) throws Exception {
        int bits = ChordLayout.direction(direction);
        return new int[]{(bits&1)!=0 ? -1 : (bits&2)!=0 ? 1 : 0,(bits&4)!=0 ? -1 : (bits&8)!=0 ? 1 : 0};
    }
    public static int check(String layout, JSONArray cases) throws Exception {
        for (int i = 0; i < cases.length(); i++) {
            JSONObject test = cases.getJSONObject(i);
            ChordEngine engine = new ChordEngine(new ChordLayout(layout));
            List<Action> actual = new ArrayList<>(), expected = new ArrayList<>();
            JSONArray frames = test.getJSONArray("frames");
            for (int j = 0; j < frames.length(); j++) {
                JSONObject frame = frames.getJSONObject(j);
                if (frame.optBoolean("reset")) { engine.reset(); continue; }
                long now = frame.optLong("at_ms",0);
                if (frame.optBoolean("tick")) { actual.addAll(engine.repeat(now)); continue; }
                int[] left = stick(frame.optString("left_stick","neutral")), right = stick(frame.optString("right_stick","neutral"));
                int modifiers = ControllerInput.modifiers(left[0],left[1],true) | ControllerInput.modifiers(right[0],right[1],false);
                int menu = 0;
                JSONArray buttons = frame.optJSONArray("menu_buttons");
                if (buttons != null) for (int k = 0; k < buttons.length(); k++) menu |= buttons.getString(k).equals("Start") ? 1 : 2;
                actual.addAll(engine.update(new Frame(ChordLayout.buttons(frame.getJSONArray("buttons")),frame.isNull("dpad") ? 3 : ChordLayout.direction(frame.getString("dpad")),modifiers,menu,left[0],left[1],right[0],right[1]),now));
            }
            JSONArray actions = test.getJSONArray("actions");
            for (int j = 0; j < actions.length(); j++) expected.add(Action.read(actions.getJSONObject(j)));
            if (!expected.equals(actual)) throw new AssertionError(test.getString("name") + ": expected " + expected + ", received " + actual);
        }
        return cases.length();
    }
}
