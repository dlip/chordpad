package io.github.dlip.chordpad

import java.util.ArrayDeque

class AlphaBankEngine(private val layout: AlphaBankLayout) {
    private data class Stroke(
        val control: Int, val mode: Int, var bank: Int, val started: Long, var modifiers: Int,
        var released: Boolean = false, var frozen: Boolean = false, var consumed: Boolean = false,
        var repeatAt: Long = started + 300, var repeatEligible: Boolean = false
    )
    private val queue = ArrayDeque<Stroke>()
    private val repeating = mutableListOf<Stroke>()
    private var waiting = true
    private var previous = Frame.NEUTRAL
    private var oneShotShift = false
    private var shiftUsed = false
    private var shiftStarted = 0L
    private var r2Started = 0L
    private var r2Used = false
    private var oneShotR2 = 0
    var status = "Release all controls"; private set
    fun ready() = !waiting

    fun reset() {
        queue.clear(); repeating.clear(); waiting = true; previous = Frame.NEUTRAL
        oneShotShift = false; shiftUsed = false; r2Used = false; oneShotR2 = 0
        status = "Release all controls"
    }

    private fun thumbs(frame: Frame): Int {
        var mask = 0
        for (i in 0..3) {
            if (frame.dpad and AlphaBankLayout.DIRECTIONS[i] != 0) mask = mask or (1 shl i)
            if (frame.buttons and AlphaBankLayout.FACES[i] != 0) mask = mask or (1 shl (i + 4))
        }
        return mask
    }
    private fun consume(control: Int) {
        for (s in queue) if (s.control == control && !s.released) { s.consumed = true; s.released = true }
    }
    private fun clearOneShots() { oneShotShift = false; oneShotR2 = 0 }
    private fun bankButtons(frame: Frame): Int {
        var held = frame.buttons
        for (s in repeating) held = held and when (s.control) { 8 -> L1.inv(); 9 -> R1.inv(); else -> -1 }
        return held
    }
    private fun controlDown(control: Int, frame: Frame) = when (control) {
        in 0..7 -> thumbs(frame) and (1 shl control) != 0
        8 -> frame.buttons and L1 != 0
        9 -> frame.buttons and R1 != 0
        else -> frame.menu and (1 shl (control - 10)) != 0
    }
    private fun prepareBumper(s: Stroke) {
        if (oneShotShift) s.modifiers = s.modifiers or Action.SHIFT
        clearOneShots()
    }
    private fun repeatStrokes(frame: Frame, now: Long): List<Action> {
        val output = mutableListOf<Action>()
        repeating.removeAll { !controlDown(it.control, frame) }
        for (s in repeating) if (now >= s.repeatAt) {
            action(s)?.let { output.add(it) }
            s.repeatAt = now + 80
        }
        while (true) {
            val s = queue.peek() ?: break
            if (!s.repeatEligible || s.released || now < s.repeatAt || !controlDown(s.control, frame)) break
            if (s.control < 8 && layout.action(s.mode, s.bank, s.control) == null) break
            queue.remove()
            if (s.control in 8..9) prepareBumper(s)
            s.frozen = true
            action(s)?.let { output.add(it) }
            s.repeatAt = now + 80; repeating.add(s)
            output.addAll(flush())
        }
        return output
    }
    private fun press(control: Int, frame: Frame, now: Long) {
        for (s in queue) if (s.control < 8) s.frozen = true
        val held = bankButtons(frame)
        val mode = if (held and (R1 or R2) == (R1 or R2)) 2 else if (oneShotR2 == 2) 3 else if (oneShotR2 == 1 || held and R2 != 0) 1 else 0
        val bank = (if (held and L1 != 0) 1 else 0) + (if (mode == 0 && held and R1 != 0) 2 else 0)
        if (control < 8) {
            if (held and L1 != 0) consume(8)
            if (held and R1 != 0) consume(9)
        }
        r2Used = r2Used || held and R2 != 0
        val usesOneShots = control !in 8..9 && control < 12
        val modifiers = when {
            control >= 12 -> 0
            else -> {
                val mask = frame.modifiers or if (held and L2 != 0 || (usesOneShots && oneShotShift)) Action.SHIFT else 0
                shiftUsed = shiftUsed || held and L2 != 0
                mask
            }
        }
        if (usesOneShots) clearOneShots()
        queue.add(Stroke(control, mode, bank, now, modifiers,
            repeatEligible = control !in 8..9 || (frame.menu == 0 &&
                frame.buttons and (L1 or R1 or R2) == if (control == 8) L1 else R1)))
    }
    private fun release(control: Int) { queue.firstOrNull { it.control == control && !it.released }?.let { it.released = true; it.frozen = true } }
    private fun action(s: Stroke): Action? {
        val action = when (s.control) {
            in 0..7 -> layout.action(s.mode, s.bank, s.control)
            8 -> Action.key("backspace"); 9 -> Action.key("space")
            else -> menuAction(s.control - 10)
        } ?: return null
        return if (action.consumerKey()) action else action.modified(s.modifiers)
    }
    private fun flush(): List<Action> {
        val result = mutableListOf<Action>()
        while (queue.peek()?.released == true) {
            val s = queue.remove()
            if (!s.consumed) {
                if (s.control in 8..9) {
                    prepareBumper(s)
                }
                action(s)?.let { result.add(it) }
            }
        }
        updateStatus()
        return result
    }

    private fun updateStatus() {
        status = listOfNotNull("Ready", if (oneShotShift) "Shift (next key)" else null, when (oneShotR2) { 1 -> "Numbers (next key)"; 2 -> "Functions (next key)"; else -> null }).joinToString(" · ")
    }

    fun update(frame: Frame, now: Long): List<Action> {
        if (!frame.valid()) { reset(); status = "Invalid D-pad position"; return emptyList() }
        if (waiting) {
            if (frame.neutral()) { waiting = false; previous = frame; status = "Ready" }
            return emptyList()
        }
        val output = repeatStrokes(frame, now).toMutableList()
        val held = frame.buttons
        val pressed = held and previous.buttons.inv()
        val released = previous.buttons and held.inv()
        val currentThumbs = thumbs(frame)
        val previousThumbs = thumbs(previous)
        val otherPress = currentThumbs and previousThumbs.inv() != 0 || frame.menu and previous.menu.inv() != 0 ||
            ((frame.leftX != 0 || frame.leftY != 0) && (frame.leftX != previous.leftX || frame.leftY != previous.leftY)) ||
            ((frame.rightX != 0 || frame.rightY != 0) && (frame.rightX != previous.rightX || frame.rightY != previous.rightY))
        for (s in queue) if (s.control in 8..9) {
            val button = if (s.control == 8) L1 else R1
            if (otherPress || pressed and button.inv() != 0) s.repeatEligible = false
        }
        for (i in 0..7) if (previousThumbs and currentThumbs.inv() and (1 shl i) != 0) release(i)
        if (pressed and L2 != 0) { shiftUsed = false; shiftStarted = now }
        if (released and L2 != 0 && !shiftUsed && now - shiftStarted <= layout.tapMax) {
            oneShotShift = !oneShotShift
        }
        if (pressed and R2 != 0) {
            r2Started = now
            r2Used = bankButtons(frame) and R1 != 0 || queue.any { !it.released && it.control < 8 }
            for (s in queue) s.frozen = true
        }
        if (released and R2 != 0 && !r2Used && now - r2Started <= layout.tapMax) {
            oneShotR2 = (oneShotR2 + 1) % 3
        }
        if (bankButtons(frame) and (R1 or R2) == R1 or R2) { consume(9); r2Used = true }
        for ((i, button) in listOf(L1, R1).withIndex()) if (released and button != 0) {
            if (held and L2 != 0) shiftUsed = true
            release(8 + i)
        }
        for (i in 0..7) if (currentThumbs and previousThumbs.inv() and (1 shl i) != 0) press(i, frame, now)
        for ((i, button) in listOf(L1, R1).withIndex()) if (pressed and button != 0) {
            r2Used = r2Used || held and R2 != 0
            val s = queue.lastOrNull { it.control < 8 }
            val joined = s != null && !s.released && !s.frozen && (s.mode == 0 || button == L1) && now - s.started < layout.lateWindow
            if (joined) checkNotNull(s).bank = s.bank or (1 shl i)
            if (!joined && held and R2 == 0 && currentThumbs == 0 && previousThumbs == 0) press(8 + i, frame, now)
        }
        for (i in 0..3) {
            val down = frame.menu and (1 shl i) != 0
            val wasDown = previous.menu and (1 shl i) != 0
            if (down && !wasDown) press(10 + i, frame, now)
            if (!down && wasDown) release(10 + i)
        }
        previous = frame
        if (queue.size > 64) { reset(); status = "Too many pending strokes; release all controls"; return emptyList() }
        output.addAll(flush())
        return output
    }

    fun repeat(now: Long): List<Action> {
        if (waiting) return emptyList()
        val actions = repeatStrokes(previous, now)
        updateStatus()
        return actions
    }

    companion object {
        fun menuAction(index: Int) = when (index) {
            0 -> Action.key("enter"); 1 -> Action.key("escape")
            2 -> Action.key("d").modified(Action.META)
            else -> Action.key("d").modified(Action.META or Action.SHIFT)
        }
    }
}
