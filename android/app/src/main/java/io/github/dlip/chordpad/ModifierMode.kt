package io.github.dlip.chordpad

class ModifierMode {
    private class Side {
        var armed = false
        var selecting = false
        var blocked = false
        var gesture = false
        var used = false
        var suppressPair = false
        var modifiers = 0
    }
    private val sides = Array(2) { Side() }
    private var previous = Frame.NEUTRAL
    var suppressTaps = 0; private set

    fun modifiers() = sides.fold(0) { mask, side -> mask or side.modifiers }

    fun status(): List<String> = sides.mapIndexedNotNull { index, side ->
        if (side.armed) "${if (index == 0) "D-pad" else "Face"} modifiers: hold direction"
        else if (side.modifiers != 0) listOf(Action.CTRL to "Ctrl", Action.ALT to "Alt", Action.SHIFT to "Shift", Action.META to "Command")
            .filter { side.modifiers and it.first != 0 }.joinToString(" + ") { it.second } + " (held)"
        else null
    }

    private fun direction(frame: Frame, index: Int): Int {
        if (index == 0) return frame.dpad
        var bits = 0
        for (i in 0..3) if (frame.buttons and AlphaBankLayout.FACES[i] != 0) bits = bits or AlphaBankLayout.DIRECTIONS[i]
        return bits
    }

    fun filter(raw: Frame, repeatingBumpers: Int): Pair<Frame, List<Int>> {
        var filtered = raw
        val consumed = mutableListOf<Int>()
        suppressTaps = 0
        val typing = (raw.dpad != 0 && !sides[0].selecting && !sides[0].blocked) ||
            (raw.buttons and 15 != 0 && !sides[1].selecting && !sides[1].blocked) || raw.menu != 0
        for (i in 0..1) {
            val side = sides[i]
            val bumper = if (i == 0) L1 else R1
            val trigger = if (i == 0) L2 else R2
            val current = direction(raw, i)
            val bumperDown = raw.buttons and bumper != 0
            val triggerDown = raw.buttons and trigger != 0
            val pair = bumperDown && triggerDown
            val wasPair = previous.buttons and (bumper or trigger) == bumper or trigger
            if (pair && !wasPair && !side.gesture && repeatingBumpers and bumper == 0) {
                side.gesture = true
                side.used = typing
                side.suppressPair = false
            }
            if (side.gesture) {
                side.used = side.used || typing
                suppressTaps = suppressTaps or trigger
                consumed.add(8 + i)
                if (!pair && !side.used && !side.suppressPair) {
                    side.armed = !(side.armed || side.selecting)
                    side.selecting = false
                    side.modifiers = 0
                    side.blocked = side.blocked || current != 0
                    side.suppressPair = true
                }
                if (side.suppressPair) filtered = filtered.copy(buttons = filtered.buttons and (bumper or trigger).inv())
                if (!bumperDown && !triggerDown) side.gesture = false
            }
            if (side.blocked && current == 0) side.blocked = false
            if (side.armed && !side.blocked && !pair && current != 0) {
                side.armed = false
                side.selecting = true
            }
            val suppress = side.blocked || side.selecting
            if (side.selecting) {
                if (current and 3 == 3 || current and 12 == 12) {
                    side.modifiers = 0
                    side.selecting = false
                    side.blocked = true
                } else {
                    val x = (if (current and 2 != 0) 1 else 0) - (if (current and 1 != 0) 1 else 0)
                    val y = (if (current and 8 != 0) 1 else 0) - (if (current and 4 != 0) 1 else 0)
                    side.modifiers = ControllerInput.modifiers(x, y, i == 0)
                    if (current == 0) side.selecting = false
                }
            }
            if (suppress) filtered = if (i == 0) filtered.copy(dpad = 0) else filtered.copy(buttons = filtered.buttons and 15.inv())
        }
        previous = raw
        return filtered to consumed
    }
}
