package io.github.dlip.padtype;

import org.json.JSONArray;
import org.json.JSONObject;
import org.junit.Test;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import static org.junit.Assert.*;

public class ControllerTest {
    private String layout() throws Exception {
        return new String(Files.readAllBytes(Path.of(System.getProperty("padtype.assets"),"default-layout.json")),StandardCharsets.UTF_8);
    }
    @Test public void rustRecognitionVectorsMatch() throws Exception {
        try (var stream = getClass().getResourceAsStream("/chord-vectors.json")) {
            assertNotNull(stream);
            JSONArray cases = new JSONArray(new String(stream.readAllBytes(),StandardCharsets.UTF_8));
            assertEquals(361,RecognitionVectors.check(layout(),cases));
        }
    }
    @Test public void analogThresholdsHaveHysteresis() {
        assertEquals(0,ControllerInput.direction(.54f,0));
        assertEquals(1,ControllerInput.direction(.55f,0));
        assertEquals(1,ControllerInput.direction(.4f,1));
        assertEquals(0,ControllerInput.direction(.35f,1));
        assertEquals(-1,ControllerInput.direction(-.7f,1));
        assertEquals(0,ControllerInput.trigger(0,64,.64f));
        assertEquals(64,ControllerInput.trigger(0,64,.65f));
        assertEquals(64,ControllerInput.trigger(64,64,.4f));
        assertEquals(0,ControllerInput.trigger(64,64,.35f));
    }
    @Test public void appSwitcherShortcutsWorkWithAndWithoutAnEditor() {
        for (boolean editor : new boolean[]{false,true}) {
            for (int modifiers : new int[]{Action.META,Action.ALT}) {
                String expected = modifiers == Action.ALT ? "last_app" : "recents";
                assertEquals(expected,Action.text("\t").modified(modifiers).systemAction(editor));
                assertEquals(expected,Action.key("tab").modified(modifiers).systemAction(editor));
            }
            assertEquals("back",new Action("system","back",0).systemAction(editor));
            for (int modifiers : new int[]{0,Action.SHIFT,Action.CTRL,Action.META|Action.CTRL,15}) {
                assertNull(Action.text("\t").modified(modifiers).systemAction(editor));
                assertNull(Action.key("tab").modified(modifiers).systemAction(editor));
            }
            assertNull(Action.key("a").modified(Action.META).systemAction(editor));
        }
    }
    @Test public void globalNavigationOnlyAppliesWithoutAnEditorOrKeyboardModifiers() {
        for (String key : new String[]{"up","down","left","right","enter","escape"}) {
            String expected = switch (key) { case "enter" -> "dpad_center"; case "escape" -> "back"; default -> "dpad_" + key; };
            assertEquals(expected,Action.key(key).systemAction(false));
            assertNull(Action.key(key).systemAction(true));
            for (int modifier : new int[]{Action.SHIFT,Action.CTRL,Action.ALT,Action.META,15}) {
                assertNull(Action.key(key).modified(modifier).systemAction(false));
            }
        }
        assertEquals("dpad_center",Action.text("\n").systemAction(false));
        assertNull(Action.text("\n").systemAction(true));
        assertNull(Action.text("\n").modified(Action.SHIFT).systemAction(false));
        assertNull(Action.text("a").systemAction(false));
        assertNull(Action.key("backspace").systemAction(false));
    }
    @Test public void directDirectionsRepeatAndReleaseAndModifiersStayBalanced() {
        DirectMapper mapper = new DirectMapper();
        Frame up = new Frame(0,0,Action.ALT,0,0,-1,0,0);
        assertEquals(List.of(Action.key("up")),mapper.update(up,0));
        assertTrue(mapper.update(up,100).isEmpty());
        assertTrue(mapper.repeat(399).isEmpty());
        assertEquals(List.of(Action.key("up")),mapper.repeat(400));
        mapper.update(Frame.NEUTRAL,420);
        assertTrue(mapper.repeat(900).isEmpty());
        Frame shifted = new Frame(64|1,0,0,0,0,0,0,0);
        assertEquals(List.of(Action.text(" ").modified(Action.SHIFT)),mapper.update(shifted,1000));
        mapper.reset(); assertTrue(mapper.repeat(2000).isEmpty());
    }
    @Test public void invalidImportDoesNotConstructALayout() throws Exception {
        JSONObject json = new JSONObject(layout());
        json.getJSONObject("strategy").put("shift","R2");
        try { new ChordLayout(json.toString()); fail("Accepted swapped Shift/layer roles"); }
        catch (org.json.JSONException expected) { assertTrue(expected.getMessage().contains("independent")); }
        json = new JSONObject(layout());
        JSONObject binding = json.getJSONArray("bindings").getJSONObject(0);
        json.getJSONArray("bindings").put(binding);
        try { new ChordLayout(json.toString()); fail("Accepted a duplicate binding"); }
        catch (org.json.JSONException expected) { assertTrue(expected.getMessage().contains("Duplicate")); }
    }
    @Test public void functionLayerRequiresDigitsAndDerivesTheirChords() throws Exception {
        ChordLayout layout = new ChordLayout(layout());
        assertEquals(15,layout.functionBindings.size());
        for (int digit = 0; digit < 10; digit++) {
            String number = Integer.toString(digit), key = "f" + (digit == 0 ? 10 : digit);
            ChordLayout.Binding source = layout.bindings.stream().filter(b -> b.action().equals(Action.text(number))).findFirst().orElseThrow();
            ChordLayout.Binding output = layout.functionBindings.stream().filter(b -> b.action().equals(Action.key(key))).findFirst().orElseThrow();
            assertEquals(source.buttons(),output.buttons()); assertEquals(source.dpad(),output.dpad());
        }
        JSONObject json = new JSONObject(layout());
        org.json.JSONArray bindings = json.getJSONArray("bindings");
        for (int i = bindings.length()-1; i >= 0; i--) if (bindings.getJSONObject(i).getJSONObject("action").optString("value").equals("0")) bindings.remove(i);
        try { new ChordLayout(json.toString()); fail("Accepted missing number 0"); }
        catch (org.json.JSONException expected) { assertTrue(expected.getMessage().contains("ten number")); }
    }
    @Test public void oldFunctionDefaultsAndCustomExtraCombosRemainUsable() throws Exception {
        JSONObject old = new JSONObject(layout()); old.getJSONObject("profile").remove("extra_function_chords");
        JSONArray bindings = old.getJSONArray("bindings");
        for (int i = bindings.length()-1; i >= 0; i--) {
            JSONObject binding = bindings.getJSONObject(i), action = binding.getJSONObject("action");
            String value = action.getString("value");
            if (!action.getString("type").equals("character") || !"0123456789".contains(value) || value.length() != 1) { bindings.remove(i); continue; }
            if (value.equals("5") || value.equals("7")) binding.put("chord",new JSONObject().put("buttons",new JSONArray().put(value.equals("5") ? "A" : "X")));
        }
        ChordLayout legacy = new ChordLayout(old.toString());
        assertEquals(Action.key("f11"),legacy.resolveFunction(0,1,128,0));
        assertEquals(Action.key("f12"),legacy.resolveFunction(0,8,128,0));
        JSONObject custom = new JSONObject(layout());
        custom.getJSONObject("profile").put("function_volume",false);
        JSONArray extras = custom.getJSONObject("profile").getJSONArray("extra_function_chords");
        extras.getJSONObject(0).put("buttons",new JSONArray().put("A").put("X"));
        ChordEngine engine = new ChordEngine(new ChordLayout(custom.toString()));
        engine.update(Frame.NEUTRAL,0);
        engine.update(new Frame(128|32,0,0,0,0,0,0,0),1);
        engine.update(new Frame(128|32|1,0,0,0,0,0,0,0),2);
        engine.update(new Frame(128|32|1|4,0,0,0,0,0,0,0),3);
        assertTrue(engine.update(new Frame(128|32|1,0,0,0,0,0,0,0),4).isEmpty());
        assertEquals(List.of(Action.key("f11")),engine.update(new Frame(128|32,0,0,0,0,0,0,0),5));
        extras.put(1,extras.getJSONObject(0));
        try { new ChordLayout(custom.toString()); fail("Accepted duplicate extra function combos"); }
        catch (org.json.JSONException expected) { assertTrue(expected.getMessage().contains("must differ")); }
    }
    @Test public void mediaAndVolumeOptionsKeepOldLayoutsCompatibleAndRejectConflicts() throws Exception {
        JSONObject json = new JSONObject(layout());
        json.getJSONObject("profile").remove("navigation_media");
        json.getJSONObject("profile").remove("function_volume");
        ChordLayout legacy = new ChordLayout(json.toString());
        assertFalse(legacy.navigationMedia); assertFalse(legacy.functionVolume);
        assertEquals(12,legacy.functionBindings.size());
        ChordEngine engine = new ChordEngine(legacy);
        engine.update(Frame.NEUTRAL,0);
        engine.update(new Frame(128|16,0,0,0,0,0,0,0),1);
        assertEquals(List.of(Action.key("page_down"),Action.key("home")),engine.update(new Frame(128|16|1|4,0,0,0,0,0,0,0),2));
        json = new JSONObject(layout());
        json.getJSONObject("profile").getJSONArray("extra_function_chords").put(0,new JSONObject().put("buttons",new JSONArray().put("A").put("X")));
        try { new ChordLayout(json.toString()); fail("Accepted volume/F11 conflict"); }
        catch (org.json.JSONException expected) { assertTrue(expected.getMessage().contains("volume")); }
        ChordLayout current = new ChordLayout(layout());
        assertEquals(Action.key("volume_down"),current.resolveFunction(1|4,0,128|64,Action.CTRL));
    }
}
