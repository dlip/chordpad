package io.github.dlip.chordpad

import org.json.JSONException
import org.json.JSONObject

data class Action(val type: String, val value: String, val modifiers: Int = 0) {
    fun modified(mask: Int) = copy(modifiers = modifiers or mask)

    fun systemAction(editorAvailable: Boolean): String? {
        if (type == "system") return value
        if ((modifiers == META || modifiers == ALT) &&
            ((type == "key" && value == "tab") || (type == "character" && value == "\t"))) {
            return if (modifiers == ALT) "last_app" else "recents"
        }
        if (!editorAvailable && modifiers == 0) {
            if (type == "character" && value == "\n") return "dpad_center"
            if (type == "key") return when (value) {
                "up", "down", "left", "right" -> "dpad_$value"
                "enter" -> "dpad_center"
                "escape" -> "back"
                else -> null
            }
        }
        return null
    }

    fun consumerKey() = type == "key" && value in setOf(
        "media_play_pause", "media_next", "media_previous", "media_stop",
        "volume_up", "volume_down", "mute"
    )

    fun json(): JSONObject = JSONObject().put("type", type).put("value", value).put("modifiers", modifiers)

    fun label(): String {
        val label = when (value) {
            " " -> "Space"
            "\n", "enter" -> "Enter"
            "\t", "tab" -> "Tab"
            "escape" -> "Escape"
            "backspace" -> "Backspace"
            else -> value
        }
        val prefix = listOf("ctrl", "alt", "shift", "meta")
            .filter { modifiers and modifier(it) != 0 }.joinToString("") { "$it + " }
        return prefix + label
    }

    companion object {
        const val CTRL = 1
        const val ALT = 2
        const val SHIFT = 4
        const val META = 8

        fun text(text: String) = Action("character", text)
        fun key(key: String) = Action("key", key)

        fun read(json: JSONObject): Action {
            val type = json.getString("type")
            if (type == "shortcut") {
                val shortcut = json.getJSONObject("value")
                val names = shortcut.getJSONArray("modifiers")
                var modifiers = 0
                for (i in 0 until names.length()) modifiers = modifiers or modifier(names.getString(i))
                return key(shortcut.getString("key")).modified(modifiers)
            }
            if (type !in setOf("key", "character", "modifier", "system")) {
                throw JSONException("Unsupported action: $type")
            }
            return Action(type, json.getString("value"), json.optInt("modifiers", 0))
        }

        fun modifier(name: String): Int = when (name) {
            "ctrl" -> CTRL
            "alt" -> ALT
            "shift" -> SHIFT
            "meta" -> META
            else -> throw IllegalArgumentException("Unknown modifier: $name")
        }
    }
}
