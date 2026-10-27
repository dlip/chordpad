package io.github.dlip.chordpad

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ShiftSelectionTest {
    @Test fun pointerSelectionKeepsAnchorInEitherDirection() {
        val selection = ShiftSelection()
        selection.start(2, false); selection.setShift(true)
        assertEquals(2 to 10, selection.update(2, 10, 10, 100))
        assertNull(selection.update(10, 2, 10, 110))
        assertEquals(0 to 2, selection.update(2, 0, 0, 120))
        selection.setShift(false)
        assertNull(selection.update(0, 8, 8, 130))
    }

    @Test fun editorRestartKeepsHeldAnchorAndNativeRanges() {
        val selection = ShiftSelection()
        selection.start(2, false); selection.setShift(true)
        selection.start(10, true)
        assertEquals(2 to 10, selection.update(2, 10, 10, 100))
        assertNull(selection.update(2, 2, 12, 110))
    }

    @Test fun typedOutputDoesNotSelectItsOwnCursorChange() {
        val selection = ShiftSelection()
        selection.start(2, false); selection.setShift(true); selection.output(100)
        assertNull(selection.update(2, 3, 3, 120))
        assertNull(selection.update(3, 4, 4, 150))
        assertEquals(4 to 10, selection.update(4, 10, 10, 220))
    }

    @Test fun unmovedOutputDoesNotBlockLaterPointerSelection() {
        val selection = ShiftSelection()
        selection.start(2, false); selection.setShift(true); selection.output(100)
        assertEquals(2 to 10, selection.update(2, 10, 10, 500))
        selection.setShift(false); selection.finish()
        selection.start(-1, false); selection.setShift(true)
        assertEquals(2 to 10, selection.update(2, 10, 10, 600))
    }
}
