package io.github.dlip.chordpad

import java.util.ArrayDeque

class ChordEngine(private val layout: ChordLayout) {
    private val alpha = layout.alphaBanks?.let { AlphaBankEngine(it) }
    private enum class Phase { IDLE, COLLECTING, RELEASING, CANCELLED }
    private class Pending(var done: Boolean = false, var action: Action? = null)
    private class Half {
        var phase = Phase.IDLE
        var buttons = 0
        var dpad = 0
        var frozenButtons = 0
        var frozenDpad = 0
        var held = 0
        var modifiers = 0
        var pending: Pending? = null
        var function = false
    }
    private val queue = ArrayDeque<Pending>()
    private var halves = arrayOf(Half(), Half())
    private var menu = arrayOfNulls<Pending>(4)
    private var cancelledMenu = BooleanArray(4)
    private var previousHeld = 0
    private var previousModifiers = 0
    private var previousMenuModifiers = 0
    private var waiting = true
    private var layers = HeldLayers()
    var status = "Release all controls"
        get() = alpha?.status ?: field
        private set

    fun ready() = alpha?.ready() ?: !waiting
    fun heldModifiers(frame: Frame) = alpha?.heldModifiers ?: (frame.modifiers or if (frame.buttons and L2 != 0) Action.SHIFT else 0)

    fun reset() {
        alpha?.reset()
        queue.clear()
        layers = HeldLayers()
        halves = arrayOf(Half(), Half())
        menu = arrayOfNulls(4)
        cancelledMenu = BooleanArray(4)
        previousHeld = 0
        previousModifiers = 0
        previousMenuModifiers = 0
        waiting = true
        status = "Release all controls"
    }

    fun repeat(now: Long = now()): List<Action> = alpha?.repeat(now) ?: if (waiting || queue.isNotEmpty()) emptyList() else layers.repeat(now)

    fun update(input: Frame, now: Long = now()): List<Action> {
        alpha?.let { return it.update(input, now) }
        if (!input.valid()) { reset(); status = "Invalid D-pad position"; return emptyList() }
        if (waiting) {
            if (input.neutral()) { waiting = false; status = "Ready" }
            return emptyList()
        }
        val held = input.buttons and (L2 or R2)
        val layerChange = if (layout.navigationLayer || layout.functionLayer) layers.update(input, layout, now)
            else HeldLayers.Change(false, false, input, emptyList())
        if (layerChange.changed) {
            for (half in halves) half.pending?.let { it.action = null; it.done = true }
            halves = arrayOf(Half(), Half())
        }
        val frame = layerChange.frame
        val newModifier = held and previousHeld.inv() != 0 || frame.modifiers and previousModifiers.inv() != 0
        previousHeld = held
        previousModifiers = frame.modifiers
        for (side in 0..1) {
            val half = halves[side]
            val buttons = frame.buttons and if (side == 0) L1 else A or B or X or Y or R1
            val dpad = if (side == 0) frame.dpad else 0
            val current = buttons or (dpad shl 8)
            val previous = half.buttons or (half.dpad shl 8)
            if (half.phase == Phase.IDLE && current != 0) {
                if (queue.size == 64) return overflow()
                val pending = Pending()
                half.pending = pending
                queue.add(pending)
                half.held = held
                half.function = layerChange.function
                half.modifiers = frame.modifiers
                half.phase = Phase.COLLECTING
            } else if (half.phase == Phase.COLLECTING || half.phase == Phase.RELEASING) {
                val pending = checkNotNull(half.pending)
                val added = current and previous.inv() != 0
                val released = previous and current.inv() != 0
                val opposite = (half.dpad and 1 != 0 && dpad and 2 != 0) ||
                    (half.dpad and 2 != 0 && dpad and 1 != 0) ||
                    (half.dpad and 4 != 0 && dpad and 8 != 0) ||
                    (half.dpad and 8 != 0 && dpad and 4 != 0)
                if (newModifier || opposite || (half.phase == Phase.RELEASING && added) || (released && added)) {
                    pending.done = true
                    half.pending = null
                    half.phase = Phase.CANCELLED
                    status = "Cancelled stroke; release that half"
                } else {
                    if (half.phase == Phase.COLLECTING && released) {
                        half.frozenButtons = half.buttons
                        half.frozenDpad = half.dpad
                        half.phase = Phase.RELEASING
                    }
                    if (half.phase == Phase.RELEASING && current == 0) {
                        pending.action = if (half.function) layout.resolveFunction(half.frozenButtons, half.frozenDpad, half.held, half.modifiers)
                            else layout.resolve(half.frozenButtons, half.frozenDpad, half.held, half.modifiers)
                        if (pending.action == null) status = "Unmapped or forbidden chord"
                        pending.done = true
                        half.pending = null
                        half.phase = Phase.IDLE
                    }
                }
            }
            if (half.phase == Phase.CANCELLED && current == 0) half.phase = Phase.IDLE
            half.buttons = buttons
            half.dpad = dpad
            if (side == 0) for (arrow in layerChange.actions) {
                if (queue.size == 64) return overflow()
                queue.add(Pending(true, arrow))
            }
        }
        val menuModifiers = frame.modifiers or if (held and L2 != 0) Action.SHIFT else 0
        val addedMenuModifier = menuModifiers and previousMenuModifiers.inv() != 0
        previousMenuModifiers = menuModifiers
        for (i in 0..3) {
            val down = frame.menu and (1 shl i) != 0
            val pending = menu[i]
            if (pending != null && addedMenuModifier && i < 2) cancelledMenu[i] = true
            if (down && pending == null) {
                if (queue.size == 64) return overflow()
                val next = Pending(action = AlphaBankEngine.menuAction(i).modified(if (i < 2) menuModifiers else 0))
                menu[i] = next
                queue.add(next)
            } else if (!down && pending != null) {
                if (cancelledMenu[i]) pending.action = null
                pending.done = true
                menu[i] = null
                cancelledMenu[i] = false
            }
        }
        val actions = mutableListOf<Action>()
        while (queue.peek()?.done == true) {
            queue.remove().action?.let { actions.add(it); status = "Ready" }
        }
        return actions
    }

    private fun overflow(): List<Action> {
        reset()
        status = "Too many pending strokes; release all controls"
        return emptyList()
    }

    companion object {
        fun now() = System.nanoTime() / 1_000_000
    }
}
