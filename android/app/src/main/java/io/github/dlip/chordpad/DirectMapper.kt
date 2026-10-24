package io.github.dlip.chordpad

import java.util.Locale

class DirectMapper {
    val mappings = defaults()
    private var previous = mutableSetOf<String>()
    private val repeatAt = linkedMapOf<String, Long>()
    private var modifiers = 0

    fun reset() { previous.clear(); repeatAt.clear(); modifiers = 0 }

    fun update(frame: Frame, now: Long): List<Action> {
        val active = active(frame)
        modifiers = 0
        for (control in active) {
            val action = mappings[control]
            if (action?.type == "modifier") modifiers = modifiers or Action.modifier(action.value)
        }
        val output = mutableListOf<Action>()
        repeatAt.keys.retainAll(active)
        for (control in CONTROLS) {
            val action = mappings[control]
            if (control in active && control !in previous && action != null && action.type != "modifier") {
                output.add(action.modified(modifiers))
                if (repeatable(action)) repeatAt[control] = now + 400
            }
        }
        previous = active
        return output
    }

    fun repeat(now: Long): List<Action> {
        val output = mutableListOf<Action>()
        for (entry in repeatAt.entries) {
            if (now >= entry.value) {
                mappings[entry.key]?.let { output.add(it.modified(modifiers)) }
                entry.setValue(now + 80)
            }
        }
        return output
    }

    companion object {
        val CONTROLS = arrayOf("A", "B", "X", "Y", "L1", "R1", "L2", "R2",
            "Start", "Select", "L3", "R3", "D-pad Up", "D-pad Down", "D-pad Left", "D-pad Right",
            "Left stick Up", "Left stick Down", "Left stick Left", "Left stick Right",
            "Right stick Up", "Right stick Down", "Right stick Left", "Right stick Right")

        fun defaults(): LinkedHashMap<String, Action> {
            val result = linkedMapOf(
                "A" to Action.text(" "), "B" to Action.key("escape"),
                "X" to Action.key("backspace"), "Y" to Action.key("enter"),
                "L1" to Action.key("tab"), "R1" to Action.text(" "),
                "L2" to Action("modifier", "shift"), "R2" to Action("modifier", "ctrl"),
                "Start" to Action.key("enter"), "Select" to Action.key("escape")
            )
            for (direction in listOf("Up", "Down", "Left", "Right")) {
                result["D-pad $direction"] = Action.key(direction.lowercase(Locale.ROOT))
                result["Left stick $direction"] = Action.key(direction.lowercase(Locale.ROOT))
            }
            result["Right stick Up"] = Action.text("w")
            result["Right stick Down"] = Action.text("s")
            result["Right stick Left"] = Action.text("a")
            result["Right stick Right"] = Action.text("d")
            return result
        }

        private fun repeatable(action: Action) = action.type == "character" ||
            action.value in setOf("up", "down", "left", "right", "backspace", "delete")

        fun active(frame: Frame): MutableSet<String> {
            val active = mutableSetOf<String>()
            for (i in 0..7) if (frame.buttons and (1 shl i) != 0) active.add(CONTROLS[i])
            for (i in 0..3) if (frame.menu and (1 shl i) != 0) active.add(CONTROLS[8 + i])
            if (frame.dpad and 4 != 0) active.add("D-pad Up")
            if (frame.dpad and 8 != 0) active.add("D-pad Down")
            if (frame.dpad and 1 != 0) active.add("D-pad Left")
            if (frame.dpad and 2 != 0) active.add("D-pad Right")
            if (frame.leftY < 0) active.add("Left stick Up")
            if (frame.leftY > 0) active.add("Left stick Down")
            if (frame.leftX < 0) active.add("Left stick Left")
            if (frame.leftX > 0) active.add("Left stick Right")
            if (frame.rightY < 0) active.add("Right stick Up")
            if (frame.rightY > 0) active.add("Right stick Down")
            if (frame.rightX < 0) active.add("Right stick Left")
            if (frame.rightX > 0) active.add("Right stick Right")
            return active
        }
    }
}
