package io.github.dlip.chordpad

import org.json.JSONArray
import org.json.JSONException
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import java.nio.file.Files
import java.nio.file.Path

class ControllerTest {
    private fun layout() = Files.readAllBytes(Path.of(System.getProperty("chordpad.assets"), "default-layout.json")).toString(Charsets.UTF_8)

    @Test fun rustRecognitionVectorsMatch() {
        javaClass.getResourceAsStream("/chord-vectors.json").use { stream ->
            assertNotNull(stream)
            val cases = JSONArray(checkNotNull(stream).readBytes().toString(Charsets.UTF_8))
            assertEquals(361, RecognitionVectors.check(layout(), cases))
        }
    }
    @Test fun analogThresholdsHaveHysteresis() {
        assertEquals(0, ControllerInput.direction(.54f, 0))
        assertEquals(1, ControllerInput.direction(.55f, 0))
        assertEquals(1, ControllerInput.direction(.4f, 1))
        assertEquals(0, ControllerInput.direction(.35f, 1))
        assertEquals(-1, ControllerInput.direction(-.7f, 1))
        assertEquals(0, ControllerInput.trigger(0, 64, .64f))
        assertEquals(64, ControllerInput.trigger(0, 64, .65f))
        assertEquals(64, ControllerInput.trigger(64, 64, .4f))
        assertEquals(0, ControllerInput.trigger(64, 64, .35f))
    }
    @Test fun appSwitcherShortcutsWorkWithAndWithoutAnEditor() {
        for (editor in listOf(false, true)) {
            for (modifiers in intArrayOf(Action.META, Action.ALT)) {
                val expected = if (modifiers == Action.ALT) "last_app" else "recents"
                assertEquals(expected, Action.text("\t").modified(modifiers).systemAction(editor))
                assertEquals(expected, Action.key("tab").modified(modifiers).systemAction(editor))
            }
            assertEquals("back", Action("system", "back").systemAction(editor))
            for (modifiers in intArrayOf(0, Action.SHIFT, Action.CTRL, Action.META or Action.CTRL, 15)) {
                assertNull(Action.text("\t").modified(modifiers).systemAction(editor))
                assertNull(Action.key("tab").modified(modifiers).systemAction(editor))
            }
            assertNull(Action.key("a").modified(Action.META).systemAction(editor))
        }
    }
    @Test fun globalNavigationOnlyAppliesWithoutAnEditorOrKeyboardModifiers() {
        for (key in listOf("up", "down", "left", "right", "enter", "escape")) {
            val expected = when (key) { "enter" -> "dpad_center"; "escape" -> "back"; else -> "dpad_$key" }
            assertEquals(expected, Action.key(key).systemAction(false))
            assertNull(Action.key(key).systemAction(true))
            for (modifier in intArrayOf(Action.SHIFT, Action.CTRL, Action.ALT, Action.META, 15)) {
                assertNull(Action.key(key).modified(modifier).systemAction(false))
            }
        }
        assertEquals("dpad_center", Action.text("\n").systemAction(false))
        assertNull(Action.text("\n").systemAction(true))
        assertNull(Action.text("\n").modified(Action.SHIFT).systemAction(false))
        assertNull(Action.text("a").systemAction(false))
        assertNull(Action.key("backspace").systemAction(false))
    }
    @Test fun directDirectionsRepeatAndReleaseAndModifiersStayBalanced() {
        val mapper = DirectMapper()
        val up = Frame(0, 0, Action.ALT, 0, 0, -1, 0, 0)
        assertEquals(listOf(Action.key("up")), mapper.update(up, 0))
        assertTrue(mapper.update(up, 100).isEmpty())
        assertTrue(mapper.repeat(399).isEmpty())
        assertEquals(listOf(Action.key("up")), mapper.repeat(400))
        mapper.update(Frame.NEUTRAL, 420)
        assertTrue(mapper.repeat(900).isEmpty())
        val shifted = Frame(64 or 1, 0, 0, 0, 0, 0, 0, 0)
        assertEquals(listOf(Action.text(" ").modified(Action.SHIFT)), mapper.update(shifted, 1000))
        mapper.reset(); assertTrue(mapper.repeat(2000).isEmpty())
    }
    @Test fun invalidImportDoesNotConstructALayout() {
        var json = JSONObject(layout())
        json.getJSONObject("strategy").put("shift", "R2")
        assertTrue(assertThrows(JSONException::class.java) { ChordLayout(json.toString()) }.message.orEmpty().contains("independent"))
        json = JSONObject(layout())
        json.getJSONArray("bindings").put(json.getJSONArray("bindings").getJSONObject(0))
        assertTrue(assertThrows(JSONException::class.java) { ChordLayout(json.toString()) }.message.orEmpty().contains("Duplicate"))
    }
    @Test fun functionLayerRequiresDigitsAndDerivesTheirChords() {
        val layout = ChordLayout(layout())
        assertEquals(15, layout.functionBindings.size)
        for (digit in 0..9) {
            val source = layout.bindings.first { it.action == Action.text(digit.toString()) }
            val output = layout.functionBindings.first { it.action == Action.key("f${if (digit == 0) 10 else digit}") }
            assertEquals(source.buttons, output.buttons); assertEquals(source.dpad, output.dpad)
        }
        val json = JSONObject(layout())
        val bindings = json.getJSONArray("bindings")
        for (i in bindings.length() - 1 downTo 0) {
            if (bindings.getJSONObject(i).getJSONObject("action").optString("value") == "0") bindings.remove(i)
        }
        assertTrue(assertThrows(JSONException::class.java) { ChordLayout(json.toString()) }.message.orEmpty().contains("ten number"))
    }
    @Test fun oldFunctionDefaultsAndCustomExtraCombosRemainUsable() {
        val old = JSONObject(layout()); old.getJSONObject("profile").remove("extra_function_chords")
        val bindings = old.getJSONArray("bindings")
        for (i in bindings.length() - 1 downTo 0) {
            val binding = bindings.getJSONObject(i)
            val action = binding.getJSONObject("action")
            val value = action.getString("value")
            if (action.getString("type") != "character" || value !in "0123456789" || value.length != 1) { bindings.remove(i); continue }
            if (value in listOf("5", "7", "9")) binding.put("chord", JSONObject().put("buttons", JSONArray().put(when (value) { "5" -> "A"; "7" -> "X"; else -> "Y" })))
        }
        val legacy = ChordLayout(old.toString())
        assertEquals(Action.key("f11"), legacy.resolveFunction(0, 1, 128, 0))
        assertEquals(Action.key("f12"), legacy.resolveFunction(0, 8, 128, 0))
        val custom = JSONObject(layout())
        custom.getJSONObject("profile").put("function_volume", false)
        val extras = custom.getJSONObject("profile").getJSONArray("extra_function_chords")
        extras.getJSONObject(0).put("buttons", JSONArray().put("A").put("X"))
        val engine = ChordEngine(ChordLayout(custom.toString()))
        engine.update(Frame.NEUTRAL, 0)
        engine.update(Frame(128 or 32, 0, 0, 0, 0, 0, 0, 0), 1)
        engine.update(Frame(128 or 32 or 1, 0, 0, 0, 0, 0, 0, 0), 2)
        engine.update(Frame(128 or 32 or 1 or 4, 0, 0, 0, 0, 0, 0, 0), 3)
        assertTrue(engine.update(Frame(128 or 32 or 1, 0, 0, 0, 0, 0, 0, 0), 4).isEmpty())
        assertEquals(listOf(Action.key("f11")), engine.update(Frame(128 or 32, 0, 0, 0, 0, 0, 0, 0), 5))
        extras.put(1, extras.getJSONObject(0))
        assertTrue(assertThrows(JSONException::class.java) { ChordLayout(custom.toString()) }.message.orEmpty().contains("must differ"))
    }
    @Test fun mediaAndVolumeOptionsKeepOldLayoutsCompatibleAndRejectConflicts() {
        var json = JSONObject(layout())
        json.getJSONObject("profile").remove("navigation_media")
        json.getJSONObject("profile").remove("function_volume")
        val legacy = ChordLayout(json.toString())
        assertFalse(legacy.navigationMedia); assertFalse(legacy.functionVolume)
        assertEquals(12, legacy.functionBindings.size)
        val engine = ChordEngine(legacy)
        engine.update(Frame.NEUTRAL, 0)
        engine.update(Frame(128 or 16, 0, 0, 0, 0, 0, 0, 0), 1)
        assertEquals(listOf(Action.key("page_down"), Action.key("home")), engine.update(Frame(128 or 16 or 1 or 4, 0, 0, 0, 0, 0, 0, 0), 2))
        json = JSONObject(layout())
        json.getJSONObject("profile").getJSONArray("extra_function_chords").put(0, JSONObject().put("buttons", JSONArray().put("A").put("X")))
        assertTrue(assertThrows(JSONException::class.java) { ChordLayout(json.toString()) }.message.orEmpty().contains("volume"))
        val current = ChordLayout(layout())
        assertEquals(Action.key("volume_down"), current.resolveFunction(1 or 4, 0, 128 or 64, Action.CTRL))
    }
}
