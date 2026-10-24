package io.github.dlip.chordpad

import io.github.dlip.chordpad.ChordLayout.Binding
import java.util.Locale

object LayoutReference {
    data class Cell(val output: String, val controls: String)
    data class Map(val title: String, val cells: List<Cell>)
    data class Layer(val title: String, val note: String, val maps: List<Map>, val chords: List<Cell>)
    private val directions = intArrayOf(5, 4, 6, 1, 0, 2, 9, 8, 10)
    private val faces = intArrayOf(X or Y, Y, Y or B, X, 0, B, A or X, A, A or B)
    private val arrows = arrayOf("↖", "↑", "↗", "←", "●", "→", "↙", "↓", "↘")

    fun layers(layout: ChordLayout): List<Layer> {
        val layers = mutableListOf<Layer>()
        val typing = layout.bindings.toMutableList()
        for (binding in layout.bindings) {
            if (binding.held == 0 && binding.dpad == 0 && binding.buttons in listOf(L1, R1) && layout.allows(binding.buttons or L2, 0)) {
                typing.add(binding.copy(held = L2))
            }
        }
        val paired = "Pairs show normal / L2 Shift. A single label applies to both. Press adjacent controls together for diagonals. In shoulder maps, the center is the shoulder alone. · means unassigned."
        layers.add(layer("Base + L2: Shift", paired, typing, 0, L2))
        if (typing.any { it.held and R2 != 0 }) layers.add(layer("R2: numbers and symbols", "Hold R2. $paired", typing, R2, R2 or L2))
        if (layout.navigationLayer) {
            val held = R2 or L1
            val navigation = mutableListOf<Binding>()
            for (dpad in intArrayOf(4, 8, 1, 2)) {
                if (layout.allows(held, dpad)) navigation.add(Binding(0, dpad, held, Action.key(when (dpad) {
                    4 -> "up"; 8 -> "down"; 1 -> "left"; else -> "right"
                })))
            }
            for (mask in 1..15) {
                val key = layout.navigationFace(mask)
                if (key != null && layout.allows(held or mask, 0)) navigation.add(Binding(mask, 0, held, Action.key(key)))
            }
            var note = "Hold R2 + L1. Add L2 for Shift selection. D-pad arrows send immediately and repeat while held; D-pad diagonals are unassigned."
            if (layout.navigationMedia) note += " Face taps send on release. Hold a single face button to repeat; adjacent pairs send Previous, Next, Delete, and Play/Pause once. Media controls ignore modifiers."
            layers.add(layer("R2 + L1: navigation", note, navigation, held))
        }
        if (layout.functionLayer) {
            var note = "Hold R2 + R1. Use the number combos for F1–F10. Release the ordinary controls to send; keep the layer held for sequential inputs. Add L2 for Shift."
            if (layout.functionVolume) note += " Volume pairs send once and ignore modifiers."
            if (layout.functionPrintScreen) note += " Y + B sends Print Screen."
            layers.add(layer("R2 + R1: function keys", note, layout.functionBindings, R2 or R1))
        }
        return layers
    }

    private fun layer(title: String, note: String, bindings: List<Binding>, vararg contexts: Int): Layer {
        val selected = bindings.filter { it.held in contexts }
        val maps = mutableListOf<Map>()
        for (shoulder in intArrayOf(0, L1, R1)) {
            if (shoulder != 0 && selected.none { it.buttons and shoulder != 0 }) continue
            for (face in listOf(false, true)) {
                if ((shoulder == L1 && face) || (shoulder == R1 && !face)) continue
                val cells = (0..8).map { i ->
                    val buttons = shoulder or if (face) faces[i] else 0
                    val dpad = if (face) 0 else directions[i]
                    var controls = if (face) faceControls(faces[i]) else arrows[i]
                    if (shoulder != 0) controls = if (i == 4) "${buttons(shoulder)} alone" else "$controls + ${buttons(shoulder)}"
                    Cell(outputs(selected, buttons, dpad, contexts), controls)
                }
                maps.add(Map((if (face) "Face buttons" else "D-pad") + (if (shoulder == 0) "" else " + ${buttons(shoulder)}"), cells))
            }
        }
        val combos = linkedMapOf<Int, Binding>()
        for (binding in selected) combos.putIfAbsent(binding.buttons or (binding.dpad shl 8), binding)
        val chords = combos.values.map { binding ->
            var controls = controls(binding.buttons, binding.dpad)
            if (contexts[0] != 0) controls = "Hold ${buttons(contexts[0])}; $controls"
            Cell(outputs(selected, binding.buttons, binding.dpad, contexts), controls)
        }
        return Layer(title, note, maps, chords)
    }

    private fun outputs(bindings: List<Binding>, buttons: Int, dpad: Int, contexts: IntArray): String {
        val labels = contexts.map { held ->
            bindings.firstOrNull { it.buttons == buttons && it.dpad == dpad && it.held == held }?.let { action(it.action) } ?: "·"
        }
        return if (labels.distinct().size == 1) labels[0] else labels.joinToString(" / ")
    }

    fun sticks(): List<Map> = listOf(true, false).map { left ->
        val cells = directions.mapIndexed { i, d ->
            val x = if (d and 1 != 0) -1 else if (d and 2 != 0) 1 else 0
            val y = if (d and 4 != 0) -1 else if (d and 8 != 0) 1 else 0
            val modifiers = ControllerInput.modifiers(x, y, left)
            Cell(if (modifiers == 0) "Neutral" else modifiers(modifiers), arrows[i])
        }
        Map(if (left) "Left stick" else "Right stick", cells)
    }

    private fun modifiers(mask: Int): String = listOf(
        Action.CTRL to "Ctrl", Action.ALT to "Alt", Action.SHIFT to "Shift", Action.META to "Command"
    ).filter { mask and it.first != 0 }.joinToString(" + ") { it.second }

    private fun action(action: Action): String {
        var name = when (action.value) {
            "up" -> "↑"; "down" -> "↓"; "left" -> "←"; "right" -> "→"
            "page_up" -> "Page Up"; "page_down" -> "Page Down"
            "media_play_pause" -> "Play/Pause"; "media_previous" -> "Previous"
            "media_next" -> "Next"; "media_stop" -> "Stop"
            "volume_up" -> "Volume Up"; "volume_down" -> "Volume Down"; "mute" -> "Mute"
            else -> action.copy(modifiers = 0).label()
        }
        if (action.type == "key" && action.value.matches(Regex("f[0-9]+"))) name = action.value.uppercase(Locale.ROOT)
        else if (action.type == "key" && name.isNotEmpty()) name = name[0].uppercaseChar() + name.substring(1)
        return if (action.modifiers == 0) name else "${modifiers(action.modifiers)} + $name"
    }

    private fun faceControls(mask: Int): String = when (mask) {
        A -> "A / ×"; B -> "B / ○"; X -> "X / □"; Y -> "Y / △"; 0 -> "●"; else -> buttons(mask)
    }

    private fun controls(mask: Int, dpad: Int): String {
        val direction = when (dpad) {
            1 -> "Left"; 2 -> "Right"; 4 -> "Up"; 8 -> "Down"
            5 -> "Up Left"; 6 -> "Up Right"; 9 -> "Down Left"; 10 -> "Down Right"; else -> ""
        }
        return (if (dpad == 0) "" else "D-pad $direction${if (mask == 0) "" else " + "}") + buttons(mask)
    }

    private fun buttons(mask: Int): String = arrayOf("A", "B", "X", "Y", "L1", "R1", "L2", "R2")
        .filterIndexed { bit, _ -> mask and (1 shl bit) != 0 }.joinToString(" + ")
}
