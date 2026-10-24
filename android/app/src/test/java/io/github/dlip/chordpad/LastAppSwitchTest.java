package io.github.dlip.chordpad;

import org.junit.Test;
import java.util.ArrayList;
import java.util.List;
import static org.junit.Assert.*;

public class LastAppSwitchTest {
    @Test public void eachSwitchTogglesRecentsTwiceAndIgnoresOverlappingRequests() {
        List<Runnable> scheduled = new ArrayList<>();
        int[] toggles = {0};
        LastAppSwitch output = new LastAppSwitch(() -> true,() -> { toggles[0]++; return true; },scheduled::add);
        assertTrue(output.start()); assertTrue(output.start());
        assertEquals(1,toggles[0]); assertEquals(1,scheduled.size());
        scheduled.get(0).run(); assertEquals(2,toggles[0]);
        assertTrue(output.start()); scheduled.get(1).run(); assertEquals(4,toggles[0]);
    }
    @Test public void cancelledCallbacksCannotCompleteAReplacementSwitch() {
        List<Runnable> scheduled = new ArrayList<>();
        int[] toggles = {0};
        LastAppSwitch output = new LastAppSwitch(() -> true,() -> { toggles[0]++; return true; },scheduled::add);
        output.start(); output.cancel(); output.start();
        scheduled.get(0).run(); assertEquals(2,toggles[0]);
        scheduled.get(1).run(); assertEquals(3,toggles[0]);
        scheduled.get(1).run(); assertEquals(3,toggles[0]);
    }
    @Test public void inactiveRemappingAndFailedActionsDoNotScheduleOrFinishSwitches() {
        List<Runnable> scheduled = new ArrayList<>();
        boolean[] active = {false}; int[] toggles = {0};
        LastAppSwitch output = new LastAppSwitch(() -> active[0],() -> { toggles[0]++; return true; },scheduled::add);
        assertFalse(output.start()); assertEquals(0,toggles[0]); assertTrue(scheduled.isEmpty());
        active[0] = true; assertTrue(output.start());
        active[0] = false; scheduled.get(0).run(); assertEquals(1,toggles[0]);
        active[0] = true; assertTrue(output.start()); assertEquals(2,toggles[0]);
        LastAppSwitch failed = new LastAppSwitch(() -> true,() -> false,scheduled::add);
        int count = scheduled.size(); assertFalse(failed.start()); assertEquals(count,scheduled.size());
    }
}
