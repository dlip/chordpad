package io.github.dlip.chordpad

import org.json.JSONArray
import org.json.JSONException
import org.json.JSONObject

const val A = 1
const val B = 2
const val X = 4
const val Y = 8
const val L1 = 16
const val R1 = 32
const val L2 = 64
const val R2 = 128

class ChordLayout(json: String) {
    data class Binding(val buttons: Int, val dpad: Int, val held: Int, val action: Action)
    val bindings = mutableListOf<Binding>()
    val navigationLayer: Boolean
    val functionLayer: Boolean
    val navigationMedia: Boolean
    val functionVolume: Boolean
    val functionBindings = mutableListOf<Binding>()
    private val functionActions = mutableMapOf<Int, Action>()
    private val actions = mutableMapOf<Int, Action>()
    private val forbiddenChords = mutableSetOf<Int>()
    private val forbiddenCombinations = mutableListOf<Int>()
    private val diagonals: Boolean
    private val excludeOpposites: Boolean
    private val disabled: Int
    private val modifierOnly: Int
    private val maxControls: Int

    init {
        val jsonObject = JSONObject(json)
        if (jsonObject.getInt("schema_version") != 2 || jsonObject.getInt("cost_model_version") != 2) {
            throw JSONException("Use a version 2 independent chordpad layout")
        }
        val strategy = jsonObject.getJSONObject("strategy")
        if (!strategy.optBoolean("independent") || strategy.optString("shift") != "L2" ||
            strategy.optString("layer") != "R2" || jsonObject.getString("recognition") != "independent-release") {
            throw JSONException("This app supports independent layouts with L2 Shift and R2 symbols")
        }
        val profile = jsonObject.getJSONObject("profile")
        navigationLayer = profile.optBoolean("navigation_layer")
        functionLayer = profile.optBoolean("function_layer")
        navigationMedia = profile.optBoolean("navigation_media")
        functionVolume = profile.optBoolean("function_volume")
        if ((navigationMedia && !navigationLayer) || (functionVolume && !functionLayer)) {
            throw JSONException("Media/volume controls require their parent layer")
        }
        val extraButtons = intArrayOf(0, 0)
        val extraDpad = intArrayOf(1, 8)
        val extras = profile.optJSONArray("extra_function_chords")
        if (profile.has("extra_function_chords") && (extras == null || extras.length() != 2)) {
            throw JSONException("F11/F12 require two extra function combos")
        }
        if (extras != null) for (i in 0..1) {
            val chord = extras.getJSONObject(i)
            extraButtons[i] = buttons(if (chord.has("buttons")) chord.getJSONArray("buttons") else null)
            extraDpad[i] = direction(chord.optString("dpad", "neutral"))
        }
        for (i in 0..1) {
            if (extraButtons[i] and 0x0f.inv() != 0 || (extraDpad[i] != 0 && extraButtons[i] != 0) ||
                (extraDpad[i] == 0 && extraButtons[i] == 0)) {
                throw JSONException("F11/F12 require D-pad or face-button combos without shoulders")
            }
        }
        if (extraButtons[0] == extraButtons[1] && extraDpad[0] == extraDpad[1]) {
            throw JSONException("F11/F12 combos must differ")
        }
        if (functionVolume) for (i in 0..1) {
            if (extraDpad[i] == 0 && extraButtons[i] in setOf(X or Y, A or X, A or B)) {
                throw JSONException("F11/F12 conflict with volume combos")
            }
        }
        diagonals = profile.optBoolean("diagonals", true)
        excludeOpposites = profile.optBoolean("exclude_opposite_face_pairs")
        disabled = buttons(profile.optJSONArray("disabled_buttons"))
        modifierOnly = if (profile.has("modifier_only_buttons")) buttons(profile.getJSONArray("modifier_only_buttons")) else L2 or R2
        maxControls = profile.optInt("max_chord_size", 0)
        val combinations = profile.optJSONArray("forbidden_combinations")
        if (combinations != null) for (i in 0 until combinations.length()) {
            forbiddenCombinations.add(buttons(combinations.getJSONArray(i)))
        }
        val chords = profile.optJSONArray("forbidden_chords")
        if (chords != null) for (i in 0 until chords.length()) {
            val chord = chords.getJSONObject(i)
            forbiddenChords.add(slot(buttons(chord.optJSONArray("buttons")), direction(chord.optString("dpad", "neutral")), 0))
        }
        val punctuation = mutableSetOf<String>()
        val base = profile.optJSONArray("base_punctuation")
        if (base != null) for (i in 0 until base.length()) punctuation.add(base.getString(i))
        val groups = mutableMapOf<String, Binding>()
        val outputs = mutableSetOf<String>()
        val array = jsonObject.getJSONArray("bindings")
        if (array.length() !in 1..4096) throw JSONException("Invalid binding count")
        for (i in 0 until array.length()) {
            val item = array.getJSONObject(i)
            val chord = item.getJSONObject("chord")
            val mask = buttons(chord.optJSONArray("buttons"))
            val dpad = direction(chord.optString("dpad", "neutral"))
            val held = buttons(item.optJSONArray("held"))
            val action = Action.read(item.getJSONObject("action"))
            if (action.type != "key" && action.type != "character") throw JSONException("Invalid chord action")
            if (held and (L2 or R2).inv() != 0 || item.optString("held_dpad", "neutral") != "neutral" ||
                mask and (L2 or R2 or modifierOnly) != 0 || (mask == 0 && dpad == 0) ||
                (dpad != 0 && mask and L1.inv() != 0) || (mask and L1 != 0 && mask and L1.inv() != 0) ||
                (navigationLayer && held and R2 != 0 && mask and L1 != 0) ||
                (functionLayer && held and R2 != 0 && mask and R1 != 0) || !allows(mask or held, dpad)) {
                throw JSONException("Forbidden or mixed-half chord at binding $i")
            }
            var plain = action.value
            var shifted = false
            if (action.type == "character") {
                if (plain.codePointCount(0, plain.length) != 1) throw JSONException("A character binding must contain one character")
                val lower = "abcdefghijklmnopqrstuvwxyz`1234567890-=[]\\;',./"
                val upper = "ABCDEFGHIJKLMNOPQRSTUVWXYZ~!@#\$%^&*()_+{}|:\"<>?"
                val index = upper.indexOf(plain)
                if (index >= 0) { shifted = true; plain = lower.substring(index, index + 1) }
                val codePoint = action.value.codePointAt(0)
                if (Character.isAlphabetic(codePoint) && held and R2 != 0) throw JSONException("Letters must use the base layer")
                if (!Character.isWhitespace(codePoint) && !Character.isISOControl(codePoint) &&
                    !Character.isAlphabetic(codePoint) && plain !in punctuation && held and R2 == 0) {
                    throw JSONException("Base punctuation is not permitted by the profile")
                }
            }
            if ((held and L2 != 0) != shifted) throw JSONException("Invalid Shift pairing")
            val fixed = fixed(mask, dpad)
            if (fixed != null && (fixed != action || held != 0)) throw JSONException("L1 is Backspace and R1 is Space")
            if (fixed == null && action in listOf(Action.text(" "), Action.key("backspace"), Action.key("space"))) {
                throw JSONException("Space and Backspace must use R1 and L1")
            }
            val binding = Binding(mask, dpad, held, action)
            val previous = groups.putIfAbsent("${action.type}:$plain:${action.modifiers}", binding)
            if (previous != null && (previous.buttons != mask || previous.dpad != dpad || previous.held and R2 != held and R2)) {
                throw JSONException("Shift pairs must share a chord and layer")
            }
            if (!outputs.add("${action.type}:${action.value}:${action.modifiers}") ||
                actions.putIfAbsent(slot(mask, dpad, held), action) != null) throw JSONException("Duplicate action or chord")
            bindings.add(binding)
        }
        if (functionLayer) {
            for (digit in 0..9) {
                val number = bindings.lastOrNull { it.held == R2 && it.action == Action.text(digit.toString()) }
                if (number == null || number.buttons and (L1 or R1) != 0 ||
                    (number.buttons == extraButtons[0] && number.dpad == extraDpad[0]) ||
                    (number.buttons == extraButtons[1] && number.dpad == extraDpad[1]) ||
                    (functionVolume && number.dpad == 0 && number.buttons in setOf(X or Y, A or X, A or B))) {
                    throw JSONException("The function layer requires all ten number combos without L1/R1 or F11/F12 conflicts")
                }
                functionBinding(number.buttons, number.dpad, "f${if (digit == 0) 10 else digit}")
            }
            functionBinding(extraButtons[0], extraDpad[0], "f11")
            functionBinding(extraButtons[1], extraDpad[1], "f12")
            if (functionVolume) {
                functionBinding(X or Y, 0, "volume_up")
                functionBinding(A or X, 0, "volume_down")
                functionBinding(A or B, 0, "mute")
            }
        }
    }

    private fun functionBinding(mask: Int, dpad: Int, key: String) {
        if (!allows(mask or R1 or R2, dpad)) throw JSONException("A function combo is forbidden by the profile")
        val action = Action.key(key)
        functionBindings.add(Binding(mask, dpad, R1 or R2, action))
        functionActions[slot(mask, dpad, 0)] = action
    }

    fun resolveFunction(mask: Int, dpad: Int, held: Int, modifiers: Int): Action? {
        if (!allows(mask or held or R1 or R2, dpad)) return null
        val action = functionActions[slot(mask, dpad, 0)] ?: return null
        return if (action.consumerKey()) action else action.modified(modifiers or if (held and L2 != 0) Action.SHIFT else 0)
    }

    fun navigationFace(mask: Int): String? = when (mask) {
        A -> "page_down"; B -> "end"; X -> "home"; Y -> "page_up"
        X or Y -> if (navigationMedia) "media_previous" else null
        Y or B -> if (navigationMedia) "media_next" else null
        A or X -> if (navigationMedia) "delete" else null
        A or B -> if (navigationMedia) "media_play_pause" else null
        else -> null
    }

    fun allows(mask: Int, dpad: Int): Boolean {
        if (mask and disabled != 0 || (!diagonals && Integer.bitCount(dpad) > 1) ||
            (maxControls > 0 && Integer.bitCount(mask) + Integer.bitCount(dpad) > maxControls) ||
            (excludeOpposites && (mask and 9 == 9 || mask and 6 == 6))) return false
        for (combination in forbiddenCombinations) if (mask and combination == combination) return false
        return slot(mask, dpad, 0) !in forbiddenChords
    }

    fun resolve(mask: Int, dpad: Int, held: Int, modifiers: Int): Action? {
        if (!allows(mask or held, dpad)) return null
        val fixed = fixed(mask, dpad)
        if (fixed != null) return if (slot(mask, dpad, 0) in actions) {
            fixed.modified(modifiers or if (held and L2 != 0) Action.SHIFT else 0)
        } else null
        return actions[slot(mask, dpad, held)]?.modified(modifiers)
    }

    companion object {
        private fun fixed(mask: Int, dpad: Int): Action? = if (dpad != 0) null else when (mask) {
            L1 -> Action.key("backspace"); R1 -> Action.text(" "); else -> null
        }
        private fun slot(mask: Int, dpad: Int, held: Int) = mask or (dpad shl 8) or (held shl 12)

        fun buttons(array: JSONArray?): Int {
            var mask = 0
            if (array == null) return mask
            for (i in 0 until array.length()) {
                val bit = when (array.getString(i)) {
                    "A" -> A; "B" -> B; "X" -> X; "Y" -> Y
                    "L1" -> L1; "R1" -> R1; "L2" -> L2; "R2" -> R2
                    else -> throw JSONException("Unknown controller button")
                }
                if (mask and bit != 0) throw JSONException("Duplicate controller button")
                mask = mask or bit
            }
            return mask
        }

        fun direction(name: String): Int = when (name) {
            "neutral" -> 0; "left" -> 1; "right" -> 2; "up" -> 4; "down" -> 8
            "up-left" -> 5; "up-right" -> 6; "down-left" -> 9; "down-right" -> 10
            else -> throw JSONException("Unknown D-pad direction: $name")
        }
    }
}
