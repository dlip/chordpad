package io.github.dlip.chordpad

/** Keep the original cursor when a pointer collapses a selection while Shift is held. */
class ShiftSelection {
    private var held = false
    private var cursor = -1
    private var anchor = -1
    private var ignoreUntil = 0L

    fun start(start: Int, restarting: Boolean) {
        cursor = start
        if (!restarting) { anchor = -1; ignoreUntil = 0 }
    }

    fun finish() { cursor = -1; anchor = -1 }

    fun setShift(active: Boolean) {
        if (held == active) return
        held = active
        anchor = if (active) cursor else -1
    }

    fun output(now: Long = ChordEngine.now()) {
        ignoreUntil = now + 100
    }

    fun update(oldStart: Int, start: Int, end: Int, now: Long = ChordEngine.now()): Pair<Int, Int>? {
        cursor = start
        if (now < ignoreUntil) {
            anchor = start
            return null
        }
        if (!held || start < 0 || start != end) return null
        if (anchor < 0) anchor = oldStart
        if (anchor < 0 || start == anchor) return null
        return minOf(anchor, start) to maxOf(anchor, start)
    }
}
