package io.github.dlip.padtype;

import org.json.JSONObject;
import org.junit.Test;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import static org.junit.Assert.*;

public class LayoutReferenceTest {
    private JSONObject bundled() throws Exception {
        return new JSONObject(new String(Files.readAllBytes(Path.of(System.getProperty("padtype.assets"),"default-layout.json")),StandardCharsets.UTF_8));
    }
    private List<LayoutReference.Layer> layers() throws Exception { return LayoutReference.layers(new ChordLayout(bundled().toString())); }
    private List<String> outputs(LayoutReference.Map map) { return map.cells().stream().map(LayoutReference.Cell::output).toList(); }

    @Test public void bundledLayersKeepShiftPairsAndEveryTextChord() throws Exception {
        var layers = layers();
        assertEquals(4,layers.size());
        assertEquals(List.of(4,2,2,2),layers.stream().map(l -> l.maps().size()).toList());
        assertEquals(List.of(34,16,12,15),layers.stream().map(l -> l.chords().size()).toList());
        List<String> base = layers.get(0).maps().stream().flatMap(m -> outputs(m).stream()).toList();
        for (String output : List.of("m / M",", / <","Space","Backspace")) assertTrue(output,base.contains(output));
        var numbers = layers.get(1).maps();
        assertEquals(List.of("8 / *","1 / !","2 / @","7 / &","·","3 / #","6 / ^","5 / %","4 / $"),outputs(numbers.get(0)));
        for (String output : List.of("9 / (","0 / )","- / _","= / +","] / }")) assertTrue(output,outputs(numbers.get(1)).contains(output));
    }

    @Test public void derivedLayersShowNavigationMediaFunctionsAndVolume() throws Exception {
        var layers = layers();
        assertEquals(List.of("·","↑","·","←","·","→","·","↓","·"),outputs(layers.get(2).maps().get(0)));
        assertEquals(List.of("Previous","Page Up","Next","Home","·","End","Delete","Page Down","Play/Pause"),outputs(layers.get(2).maps().get(1)));
        assertEquals(List.of("Volume Up","F9","·","F12","·","F10","Volume Down","F11","Mute"),outputs(layers.get(3).maps().get(1)));
    }

    @Test public void importedPositionsAndOptionalLayersComeFromActiveLayout() throws Exception {
        JSONObject json = bundled();
        var bindings = json.getJSONArray("bindings");
        for (int i = 0; i < bindings.length(); i++) {
            var binding = bindings.getJSONObject(i);
            String value = binding.getJSONObject("action").getString("value");
            if (List.of("-","_","=","+").contains(value)) {
                binding.getJSONObject("chord").put("buttons",new org.json.JSONArray().put(value.equals("-") || value.equals("_") ? "A" : "X"));
            }
        }
        json.getJSONObject("profile").put("navigation_layer",false).put("navigation_media",false)
                .put("function_layer",false).put("function_volume",false);
        var layers = LayoutReference.layers(new ChordLayout(json.toString()));
        assertEquals(2,layers.size());
        assertEquals("= / +",layers.get(1).maps().get(1).cells().get(3).output());
        assertEquals("- / _",layers.get(1).maps().get(1).cells().get(7).output());
    }

    @Test public void stickMapsMirrorTheRuntimeModifiers() {
        var maps = LayoutReference.sticks();
        assertEquals("Ctrl + Alt + Shift + Command",maps.get(0).cells().get(2).output());
        assertEquals("Ctrl + Alt + Shift + Command",maps.get(1).cells().get(0).output());
        assertEquals("Alt + Command",maps.get(0).cells().get(8).output());
        assertEquals("Alt + Command",maps.get(1).cells().get(6).output());
        assertEquals("Ctrl + Alt + Command",maps.get(0).cells().get(5).output());
        assertEquals("Ctrl + Alt + Command",maps.get(1).cells().get(3).output());
    }

    @Test public void textReferenceKeepsImportedCombosOutsideTheGrid() throws Exception {
        JSONObject json = bundled();
        json.getJSONObject("profile").put("exclude_opposite_face_pairs",false);
        var bindings = json.getJSONArray("bindings");
        for (int i = 0; i < bindings.length(); i++) {
            var binding = bindings.getJSONObject(i);
            if (List.of("e","E").contains(binding.getJSONObject("action").getString("value"))) {
                binding.put("chord",new JSONObject().put("buttons",new org.json.JSONArray().put("A").put("Y")));
            }
        }
        var base = LayoutReference.layers(new ChordLayout(json.toString())).get(0);
        assertTrue(base.chords().stream().anyMatch(c -> c.output().equals("e / E") && c.controls().equals("A + Y")));
        assertFalse(base.maps().stream().flatMap(m -> m.cells().stream()).anyMatch(c -> c.output().equals("e / E")));
    }
}
