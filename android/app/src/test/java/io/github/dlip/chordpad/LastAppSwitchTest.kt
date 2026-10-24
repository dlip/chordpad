package io.github.dlip.chordpad

import org.junit.Assert.*
import org.junit.Test

class LastAppSwitchTest {
    @Test fun eachSwitchTogglesRecentsTwiceAndIgnoresOverlappingRequests() {
        val scheduled = mutableListOf<Runnable>()
        var toggles = 0
        val output = LastAppSwitch({ true }, { toggles++; true }, { scheduled.add(it) })
        assertTrue(output.start()); assertTrue(output.start())
        assertEquals(1, toggles); assertEquals(1, scheduled.size)
        scheduled[0].run(); assertEquals(2, toggles)
        assertTrue(output.start()); scheduled[1].run(); assertEquals(4, toggles)
    }
    @Test fun cancelledCallbacksCannotCompleteAReplacementSwitch() {
        val scheduled = mutableListOf<Runnable>()
        var toggles = 0
        val output = LastAppSwitch({ true }, { toggles++; true }, { scheduled.add(it) })
        output.start(); output.cancel(); output.start()
        scheduled[0].run(); assertEquals(2, toggles)
        scheduled[1].run(); assertEquals(3, toggles)
        scheduled[1].run(); assertEquals(3, toggles)
    }
    @Test fun inactiveRemappingAndFailedActionsDoNotScheduleOrFinishSwitches() {
        val scheduled = mutableListOf<Runnable>()
        var active = false
        var toggles = 0
        val output = LastAppSwitch({ active }, { toggles++; true }, { scheduled.add(it) })
        assertFalse(output.start()); assertEquals(0, toggles); assertTrue(scheduled.isEmpty())
        active = true; assertTrue(output.start())
        active = false; scheduled[0].run(); assertEquals(1, toggles)
        active = true; assertTrue(output.start()); assertEquals(2, toggles)
        val failed = LastAppSwitch({ true }, { false }, { scheduled.add(it) })
        val count = scheduled.size; assertFalse(failed.start()); assertEquals(count, scheduled.size)
    }
}
