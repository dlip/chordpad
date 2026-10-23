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
            assertEquals(146,RecognitionVectors.check(layout(),cases));
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
}
