package io.github.dlip.chordpad

class HeldLayers {
    data class Change(val changed: Boolean, val function: Boolean, val frame: Frame, val actions: List<Action>)
    private data class Repeating(val action: Action, val at: Long)
    private enum class Mode { NORMAL, NAVIGATION, FUNCTION, AMBIGUOUS }
    private var mode = Mode.NORMAL
    private var blockedDpad = false
    private var previousButtons = 0
    private var previousDpad = 0
    private var blockedButtons = 0
    private var active = linkedMapOf<String, Repeating>()
    private var face = NavigationFace()

    fun update(frame: Frame, layout: ChordLayout, now: Long): Change {
        val buttons = frame.buttons
        val direction = frame.dpad
        val r2 = buttons and R2 != 0
        val navigation = layout.navigationLayer && r2 && buttons and L1 != 0
        val function = layout.functionLayer && r2 && buttons and R1 != 0
        val nextMode = when {
            navigation && function -> Mode.AMBIGUOUS
            navigation -> Mode.NAVIGATION
            function -> Mode.FUNCTION
            else -> Mode.NORMAL
        }
        val changed = nextMode != mode
        val functionExit = mode == Mode.FUNCTION && nextMode == Mode.NORMAL
        if (changed) face = NavigationFace()
        blockedButtons = blockedButtons and buttons
        if (direction == 0) blockedDpad = false
        if (changed && !functionExit) {
            blockedButtons = blockedButtons or (previousButtons and buttons and 0x3f)
            blockedDpad = previousDpad != 0 && direction != 0
            if (mode != Mode.NORMAL) {
                blockedButtons = blockedButtons or (buttons and 0x0f)
                blockedDpad = direction != 0
            }
        }
        blockedButtons = blockedButtons or when (nextMode) {
            Mode.NAVIGATION -> L1
            Mode.FUNCTION -> R1
            Mode.AMBIGUOUS -> L1 or R1
            else -> 0
        }
        mode = nextMode
        previousButtons = buttons
        previousDpad = direction
        val output = mutableListOf<Action>()
        val next = linkedMapOf<String, Repeating>()
        if (mode == Mode.NAVIGATION) {
            val held = R2 or L1 or (buttons and L2)
            val modifiers = frame.modifiers or if (buttons and L2 != 0) Action.SHIFT else 0
            if (!blockedDpad && layout.allows(held, direction)) {
                val key = when (direction) { 1 -> "left"; 2 -> "right"; 4 -> "up"; 8 -> "down"; else -> null }
                if (key != null) add(next, output, key, modifiers, now)
            }
            val keys = arrayOf("page_down", "end", "home", "page_up")
            if (layout.navigationMedia) output.addAll(face.update(buttons and blockedButtons.inv() and 0x0f, held, frame.modifiers, layout, now))
            for (i in 0..3) {
                if (!layout.navigationMedia && buttons and blockedButtons.inv() and (1 shl i) != 0 && layout.allows(held or (1 shl i), 0)) {
                    add(next, output, keys[i], modifiers, now)
                }
            }
        }
        active = next
        val filtered = frame.copy(
            buttons = buttons and blockedButtons.inv() and when (mode) {
                Mode.NAVIGATION -> 0x0f.inv()
                Mode.AMBIGUOUS -> 0x3f.inv()
                else -> 0xff
            },
            dpad = if (mode == Mode.NAVIGATION || mode == Mode.AMBIGUOUS || blockedDpad) 0 else direction
        )
        return Change(changed && !functionExit, mode == Mode.FUNCTION, filtered, output)
    }

    private fun add(next: MutableMap<String, Repeating>, output: MutableList<Action>, key: String, modifiers: Int, now: Long) {
        val action = Action.key(key).modified(modifiers)
        val previous = active[key]
        if (previous == null) output.add(action)
        next[key] = Repeating(action, previous?.at ?: (now + 400))
    }

    fun repeat(now: Long): List<Action> {
        val output = mutableListOf<Action>()
        active.replaceAll { _, entry ->
            if (now < entry.at) entry else {
                output.add(entry.action)
                Repeating(entry.action, now + 80)
            }
        }
        output.addAll(face.repeat(now))
        return output
    }
}
