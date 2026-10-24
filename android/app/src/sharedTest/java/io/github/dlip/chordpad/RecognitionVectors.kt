package io.github.dlip.chordpad

import org.json.JSONArray

object RecognitionVectors {
    private fun stick(direction: String): IntArray {
        val bits = ChordLayout.direction(direction)
        return intArrayOf(if (bits and 1 != 0) -1 else if (bits and 2 != 0) 1 else 0,
            if (bits and 4 != 0) -1 else if (bits and 8 != 0) 1 else 0)
    }

    fun check(layout: String, cases: JSONArray): Int {
        for (i in 0 until cases.length()) {
            val test = cases.getJSONObject(i)
            val engine = ChordEngine(ChordLayout(layout))
            val actual = mutableListOf<Action>()
            val expected = mutableListOf<Action>()
            val frames = test.getJSONArray("frames")
            for (j in 0 until frames.length()) {
                val frame = frames.getJSONObject(j)
                if (frame.optBoolean("reset")) { engine.reset(); continue }
                val now = frame.optLong("at_ms", 0)
                if (frame.optBoolean("tick")) { actual.addAll(engine.repeat(now)); continue }
                val left = stick(frame.optString("left_stick", "neutral"))
                val right = stick(frame.optString("right_stick", "neutral"))
                val modifiers = ControllerInput.modifiers(left[0], left[1], true) or ControllerInput.modifiers(right[0], right[1], false)
                var menu = 0
                val buttons = frame.optJSONArray("menu_buttons")
                if (buttons != null) for (k in 0 until buttons.length()) menu = menu or if (buttons.getString(k) == "Start") 1 else 2
                actual.addAll(engine.update(Frame(ChordLayout.buttons(frame.getJSONArray("buttons")),
                    if (frame.isNull("dpad")) 3 else ChordLayout.direction(frame.getString("dpad")),
                    modifiers, menu, left[0], left[1], right[0], right[1]), now))
            }
            val actions = test.getJSONArray("actions")
            for (j in 0 until actions.length()) expected.add(Action.read(actions.getJSONObject(j)))
            if (expected != actual) throw AssertionError("${test.getString("name")}: expected $expected, received $actual")
        }
        return cases.length()
    }
}
