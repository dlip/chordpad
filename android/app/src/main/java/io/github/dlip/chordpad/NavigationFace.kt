package io.github.dlip.chordpad

class NavigationFace {
    private enum class Phase { IDLE, COLLECTING, RELEASING, CANCELLED }
    private var phase = Phase.IDLE
    private var previous = 0
    private var frozen = 0
    private var held = 0
    private var modifiers = 0
    private var previousHeld = 0
    private var previousModifiers = 0
    private var repeated = false
    private var repeating: Action? = null
    private var at = 0L

    private fun resolve(mask: Int, held: Int, modifiers: Int, layout: ChordLayout): Action? {
        if (!layout.allows(mask or held, 0)) return null
        val key = layout.navigationFace(mask) ?: return null
        val action = Action.key(key)
        return if (action.consumerKey()) action else action.modified(
            modifiers or if (held and L2 != 0) Action.SHIFT else 0
        )
    }

    fun update(mask: Int, held: Int, modifiers: Int, layout: ChordLayout, now: Long): List<Action> {
        val newModifier = held and previousHeld.inv() != 0 || modifiers and previousModifiers.inv() != 0
        previousHeld = held
        previousModifiers = modifiers
        val output = mutableListOf<Action>()
        if (phase == Phase.IDLE && mask != 0) {
            phase = Phase.COLLECTING
            this.held = held
            this.modifiers = modifiers
            repeated = false
        } else if (phase == Phase.COLLECTING || phase == Phase.RELEASING) {
            val added = mask and previous.inv() != 0
            val released = previous and mask.inv() != 0
            if (newModifier || (added && (released || phase == Phase.RELEASING || repeated))) {
                phase = Phase.CANCELLED
            } else {
                if (phase == Phase.COLLECTING && released) {
                    frozen = previous
                    phase = Phase.RELEASING
                }
                if (phase == Phase.RELEASING && mask == 0) {
                    val action = resolve(frozen, this.held, this.modifiers, layout)
                    if (!repeated && action != null) output.add(action)
                    phase = Phase.IDLE
                }
            }
        }
        if (phase == Phase.CANCELLED && mask == 0) phase = Phase.IDLE
        if (phase != Phase.COLLECTING || Integer.bitCount(mask) != 1) repeating = null
        else if (repeating == null) {
            repeating = resolve(mask, this.held, this.modifiers, layout)
            at = now + 400
        }
        previous = mask
        return output
    }

    fun repeat(now: Long): List<Action> {
        val action = repeating ?: return emptyList()
        if (now < at) return emptyList()
        repeated = true
        at = now + 80
        return listOf(action)
    }
}
