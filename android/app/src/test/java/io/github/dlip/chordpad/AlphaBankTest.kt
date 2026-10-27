package io.github.dlip.chordpad

import org.json.JSONArray
import org.json.JSONObject
import org.json.JSONException
import org.junit.Assert.*
import org.junit.Test
import java.nio.file.Files
import java.nio.file.Path

class AlphaBankTest {
    private fun layout() = Files.readAllBytes(Path.of(System.getProperty("chordpad.assets"), "default-layout.json")).toString(Charsets.UTF_8)
    @Test fun linuxVectorsMatchAndroid() {
        val cases = JSONArray(checkNotNull(javaClass.getResourceAsStream("/alpha-bank-vectors.json")).bufferedReader().readText())
        assertEquals(cases.length(), RecognitionVectors.check(layout(), cases))
    }
    @Test fun referenceUsesCurrentBanksAndStickClicks() {
        val layers = LayoutReference.layers(ChordLayout(layout()))
        assertEquals(10, layers.size)
        assertEquals("1 / !", layers[4].maps[0].cells[1].output)
        assertEquals("2 / @", layers[4].maps[0].cells[5].output)
        assertEquals("5 / %", layers[5].maps[0].cells[1].output)
        assertEquals("F1", layers[8].maps[0].cells[1].output)
        assertEquals("F5", layers[9].maps[0].cells[1].output)
        assertEquals("Volume Up", layers[7].maps[0].cells[1].output)
        assertEquals("Stop", layers[7].maps[1].cells[5].output)
        assertEquals("Command + D", LayoutReference.sticks()[0].cells[4].output)
        assertEquals("Click R3", LayoutReference.sticks()[1].cells[4].controls)
    }
    @Test fun invalidCandidateImportsFail() {
        for (field in listOf("late_bumper_window_ms", "trigger_tap_max_ms")) {
            for (window in listOf(0, 1001)) {
                val json = JSONObject(layout())
                json.getJSONObject("recognition").put(field, window)
                assertThrows(JSONException::class.java) { ChordLayout(json.toString()) }
            }
        }
        val json = JSONObject(layout())
        json.getJSONArray("banks").getJSONObject(0).getJSONArray("bindings").getJSONObject(0).put("control", "invalid")
        assertThrows(JSONException::class.java) { ChordLayout(json.toString()) }
    }
}
