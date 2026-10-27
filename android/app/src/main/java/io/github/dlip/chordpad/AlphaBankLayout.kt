package io.github.dlip.chordpad

import org.json.JSONException
import org.json.JSONObject

class AlphaBankLayout(json: JSONObject) {
    data class Bank(val held: Int, val actions: Map<Int, Action>)
    val lateWindow: Long
    val tapMax: Long
    val banks: List<Bank>
    val functions: List<Bank>
    val navigation: List<Bank>

    init {
        if (json.getString("format") != "chordpad-alpha-bank-candidate-v2") throw JSONException("Unsupported four-bank layout format")
        val recognition = json.getJSONObject("recognition")
        lateWindow = recognition.getLong("late_bumper_window_ms")
        tapMax = recognition.optLong("trigger_tap_max_ms", recognition.optLong("function_double_tap_ms", 250))
        if (lateWindow !in 1..1000 || tapMax !in 1..1000) throw JSONException("Timing windows must be 1–1000 ms")
        if (ChordLayout.buttons(json.getJSONArray("held_shift_buttons")) != L2) throw JSONException("Held Shift must use L2")
        fun read(name: String, selectors: List<Int>): List<Bank> {
            val array = json.getJSONArray(name)
            if (array.length() != selectors.size) throw JSONException("Incorrect $name count")
            return selectors.mapIndexed { index, held ->
                val item = array.getJSONObject(index)
                if (ChordLayout.buttons(item.getJSONArray("held")) != held) throw JSONException("Unexpected $name selector")
                val actions = linkedMapOf<Int, Action>()
                val bindings = item.getJSONArray("bindings")
                val controls = if (name == "function_banks" && index == 1) 0..3 else 0..7
                for (i in 0 until bindings.length()) {
                    val binding = bindings.getJSONObject(i)
                    val control = CONTROLS.indexOf(binding.getString("control"))
                    if (control !in controls || control in actions) throw JSONException("Invalid or duplicate thumb control")
                    if (binding.has("character") == binding.has("key")) throw JSONException("Bind exactly one character or key")
                    val action = if (binding.has("character")) {
                        val character = binding.getString("character")
                        if (character.length != 1 || (character[0] !in ' '..'~' && character !in listOf("\t", "\n"))) {
                            throw JSONException("Four-bank characters must use ASCII, Tab, or Enter")
                        }
                        Action.text(character)
                    } else Action.key(binding.getString("key"))
                    if (action.type == "key" && action.value !in KEYS && !action.value.matches(Regex("[a-z]|f([1-9]|1[0-2])"))) {
                        throw JSONException("Unknown keyboard key: ${action.value}")
                    }
                    actions[control] = action
                }
                if (actions.keys != controls.toSet()) throw JSONException("Missing thumb bindings in $name")
                Bank(held, actions)
            }
        }
        banks = read("banks", listOf(0, L1, R1, L1 or R1, R2, R2 or L1))
        functions = read("function_banks", listOf(R2, R2 or L1))
        navigation = read("navigation_banks", listOf(R1 or R2, R1 or R2 or L1))
    }

    fun action(mode: Int, bank: Int, control: Int): Action? = when (mode) {
        0 -> banks[bank]; 1 -> banks[4 + bank]; 2 -> navigation[bank]; else -> functions[bank]
    }.actions[control]

    fun bindings(): List<ChordLayout.Binding> = (banks + navigation + functions).flatMap { bank ->
        bank.actions.map { (control, action) ->
            ChordLayout.Binding(if (control < 4) 0 else FACES[control - 4],
                if (control < 4) DIRECTIONS[control] else 0, bank.held, action)
        }
    }

    companion object {
        private val KEYS = setOf("backspace", "escape", "enter", "tab", "space", "delete", "insert",
            "left", "right", "up", "down", "home", "end", "page_up", "page_down", "print_screen",
            "media_play_pause", "media_next", "media_previous", "media_stop", "volume_up", "volume_down", "mute")
        val CONTROLS = listOf("↑", "→", "↓", "←", "Y / △", "B / ○", "A / ×", "X / □")
        val DIRECTIONS = intArrayOf(4, 2, 8, 1)
        val FACES = intArrayOf(Y, B, A, X)
    }
}
