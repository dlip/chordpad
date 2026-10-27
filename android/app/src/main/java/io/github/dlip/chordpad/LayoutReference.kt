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
        layout.alphaBanks?.let { return alphaLayers(it) }
        val layers = mutableListOf<Layer>()
        val typing = layout.bindings.toMutableList()
        for (binding in layout.bindings) {
            if (binding.held == 0 && binding.dpad == 0 && binding.buttons in listOf(L1, R1) && layout.allows(binding.buttons or L2, 0)) {
                typing.add(binding.copy(held = L2))
            }
        }
        val paired = "Pairs show normal / L2 Shift. A single label applies to both. Press adjacent controls together for diagonals. In shoulder maps, the center is the shoulder alone. · means unassigned."
        layers.add(layer("Base: letters and punctuation", paired, typing, 0, L2))
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

    private fun alphaLayers(layout: AlphaBankLayout): List<Layer> {
        val titles = listOf("Base: letters and punctuation", "L1: alpha", "R1: alpha", "L1 + R1: alpha",
            "R2: numbers 1–4", "R2 + L1: numbers 5–8", "R1 + R2: navigation", "Add L1: editing / media",
            "Tap R2 twice: function keys", "Add L1: function keys")
        return (layout.banks + layout.navigation + layout.functions).mapIndexed { index, bank ->
            val bindings = bank.actions.map { (control, action) -> Binding(
                if (control < 4) 0 else AlphaBankLayout.FACES[control - 4],
                if (control < 4) AlphaBankLayout.DIRECTIONS[control] else 0, bank.held, action) }
            val note = when {
                index < 4 -> "Use one cardinal or face button. Hold the bank bumpers or add them within ${layout.lateWindow} ms, before release. Hold any output for 300 ms to repeat every 80 ms; L1/R1 must start alone to repeat Backspace/Space. Hold L2 for Shift, or tap ≤${layout.tapMax} ms for one-shot Shift; tap again to cancel."
                index < 6 -> "Hold or tap R2 for numbers; add L1 for 5–8. R2 taps cycle numbers → functions → off for the next key. Small outputs use L2 Shift."
                index < 8 -> "Hold R1+R2; add L1 for editing/media. All outputs repeat after 300 ms, every 80 ms. L2 adds Shift."
                else -> "Tap and release unused R2 twice for one-shot functions; a third unused tap cancels. Each tap lasts at most ${layout.tapMax} ms; gaps have no limit. Add L1 for F5–F8."
            }
            val result = if (index >= 6) layer(titles[index], note, bindings, bank.held)
            else {
                val lower = "abcdefghijklmnopqrstuvwxyz`1234567890-=[]\\;',./"
                val upper = "ABCDEFGHIJKLMNOPQRSTUVWXYZ~!@#\$%^&*()_+{}|:\"<>?"
                val shifted = bindings.map { binding ->
                    val position = lower.indexOf(binding.action.value)
                    binding.copy(held = binding.held or L2, action = if (position >= 0) Action.text(upper[position].toString()) else binding.action)
                }
                layer(titles[index], note, bindings + shifted, bank.held, bank.held or L2)
            }
            result.copy(maps = result.maps.map { map ->
                map.copy(cells = map.cells.mapIndexed { i, cell -> if (i in listOf(0, 2, 4, 6, 8)) Cell("", "") else cell })
            })
        }
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
            if (i == 4) Cell(if (left) "Command + D" else "Command + Shift + D", if (left) "Click L3" else "Click R3")
            else Cell(modifiers(modifiers), arrows[i])
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
