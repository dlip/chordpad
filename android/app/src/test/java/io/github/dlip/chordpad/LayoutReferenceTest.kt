package io.github.dlip.chordpad

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import java.nio.file.Files
import java.nio.file.Path

class LayoutReferenceTest {
    private fun bundled() = JSONObject(Files.readAllBytes(Path.of(System.getProperty("chordpad.assets"), "default-layout.json")).toString(Charsets.UTF_8))
    private fun layers() = LayoutReference.layers(ChordLayout(bundled().toString()))
    private fun outputs(map: LayoutReference.Map) = map.cells.map { it.output }

    @Test fun bundledLayersKeepShiftPairsAndEveryTextChord() {
        val layers = layers()
        assertEquals(4, layers.size)
        assertEquals(listOf(4, 2, 2, 2), layers.map { it.maps.size })
        assertEquals(listOf(34, 16, 12, 16), layers.map { it.chords.size })
        val base = layers[0].maps.flatMap { outputs(it) }
        for (output in listOf("m / M", ", / <", "Space", "Backspace")) assertTrue(output, output in base)
        val numbers = layers[1].maps
        assertEquals(listOf("8 / *", "1 / !", "2 / @", "7 / &", "·", "3 / #", "6 / ^", "5 / %", "4 / $"), outputs(numbers[0]))
        for (output in listOf("9 / (", "0 / )", "- / _", "= / +", "] / }")) assertTrue(output, output in outputs(numbers[1]))
    }
    @Test fun derivedLayersShowNavigationMediaFunctionsAndVolume() {
        val layers = layers()
        assertEquals(listOf("·", "↑", "·", "←", "·", "→", "·", "↓", "·"), outputs(layers[2].maps[0]))
        assertEquals(listOf("Previous", "Page Up", "Next", "Home", "·", "End", "Delete", "Page Down", "Play/Pause"), outputs(layers[2].maps[1]))
        assertEquals(listOf("Volume Up", "F12", "Print Screen", "F11", "·", "F10", "Volume Down", "F9", "Mute"), outputs(layers[3].maps[1]))
    }
    @Test fun importedPositionsAndOptionalLayersComeFromActiveLayout() {
        val json = bundled()
        val bindings = json.getJSONArray("bindings")
        for (i in 0 until bindings.length()) {
            val binding = bindings.getJSONObject(i)
            val value = binding.getJSONObject("action").getString("value")
            if (value in listOf("-", "_", "=", "+", "9", "(")) {
                binding.getJSONObject("chord").put("buttons", JSONArray().put(when (value) { "9", "(" -> "Y"; "-", "_" -> "A"; else -> "X" }))
            }
        }
        json.getJSONObject("profile").put("navigation_layer", false).put("navigation_media", false)
            .put("function_layer", false).put("function_volume", false).put("function_print_screen", false)
        val layers = LayoutReference.layers(ChordLayout(json.toString()))
        assertEquals(2, layers.size)
        assertEquals("= / +", layers[1].maps[1].cells[3].output)
        assertEquals("- / _", layers[1].maps[1].cells[7].output)
    }
    @Test fun stickMapsMirrorTheRuntimeModifiers() {
        val maps = LayoutReference.sticks()
        assertEquals("Ctrl + Alt + Shift + Command", maps[0].cells[2].output)
        assertEquals("Ctrl + Alt + Shift + Command", maps[1].cells[0].output)
        assertEquals("Alt + Command", maps[0].cells[8].output)
        assertEquals("Alt + Command", maps[1].cells[6].output)
        assertEquals("Ctrl + Alt + Command", maps[0].cells[5].output)
        assertEquals("Ctrl + Alt + Command", maps[1].cells[3].output)
    }
    @Test fun textReferenceKeepsImportedCombosOutsideTheGrid() {
        val json = bundled()
        json.getJSONObject("profile").put("exclude_opposite_face_pairs", false)
        val bindings = json.getJSONArray("bindings")
        for (i in 0 until bindings.length()) {
            val binding = bindings.getJSONObject(i)
            if (binding.getJSONObject("action").getString("value") in listOf("e", "E")) {
                binding.put("chord", JSONObject().put("buttons", JSONArray().put("A").put("Y")))
            }
        }
        val base = LayoutReference.layers(ChordLayout(json.toString()))[0]
        assertTrue(base.chords.any { it.output == "e / E" && it.controls == "A + Y" })
        assertFalse(base.maps.flatMap { it.cells }.any { it.output == "e / E" })
    }
}
