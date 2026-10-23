package io.github.dlip.padtype;

import android.view.KeyEvent;
import org.junit.Test;
import static org.junit.Assert.*;

public class GamepadKeysTest {
    private static final int[] LAYERS = {KeyEvent.KEYCODE_BUTTON_L1,KeyEvent.KEYCODE_BUTTON_R1,KeyEvent.KEYCODE_BUTTON_L2,KeyEvent.KEYCODE_BUTTON_R2};
    private final GamepadKeys keys = new GamepadKeys();
    private GamepadKeys.Result down(int code,boolean typing) { return keys.update(code,KeyEvent.ACTION_DOWN,0,typing); }
    private GamepadKeys.Result up(int code,boolean typing) { return keys.update(code,KeyEvent.ACTION_UP,0,typing); }

    @Test public void pressesKeepTheirRoutingAcrossPauseAndResume() {
        assertTrue(down(KeyEvent.KEYCODE_BUTTON_A,true).consumed());
        assertTrue(keys.hasConsumedPresses());
        assertTrue(keys.update(KeyEvent.KEYCODE_BUTTON_A,KeyEvent.ACTION_DOWN,1,false).consumed());
        assertTrue(up(KeyEvent.KEYCODE_BUTTON_A,false).consumed());
        assertFalse(keys.hasConsumedPresses());
        assertFalse(down(KeyEvent.KEYCODE_BUTTON_A,false).consumed());
        assertFalse(keys.hasConsumedPresses());
        assertFalse(keys.update(KeyEvent.KEYCODE_BUTTON_A,KeyEvent.ACTION_DOWN,1,true).consumed());
        assertFalse(up(KeyEvent.KEYCODE_BUTTON_A,true).consumed());
        assertFalse(up(KeyEvent.KEYCODE_BUTTON_B,true).consumed());
        assertFalse(keys.update(KeyEvent.KEYCODE_BUTTON_B,KeyEvent.ACTION_DOWN,1,true).consumed());
    }
    @Test public void fourDigitalButtonsResumeWithoutReceivingMotionEvents() {
        for (int i = 0; i < LAYERS.length; i++) {
            GamepadKeys.Result result = down(LAYERS[i],false);
            assertFalse(result.consumed());
            assertEquals(i == 3,result.layerGesture());
        }
        assertFalse(down(LAYERS[3],true).layerGesture());
        for (int code : LAYERS) assertFalse(up(code,true).consumed());
        for (int i = 3; i >= 0; i--) assertEquals(i == 0,down(LAYERS[i],false).layerGesture());
    }
    @Test public void partialReleaseCannotRetriggerTheGesture() {
        for (int code : LAYERS) down(code,false);
        up(LAYERS[0],true);
        assertFalse(down(LAYERS[0],true).layerGesture());
        for (int code : LAYERS) up(code,true);
        for (int i = 0; i < LAYERS.length; i++) assertEquals(i == 3,down(LAYERS[i],true).layerGesture());
    }
    @Test public void analogPauseBeforeTriggerButtonsCannotImmediatelyResume() {
        down(LAYERS[0],true); down(LAYERS[1],true);
        keys.suppressHeldGesture();
        assertFalse(down(LAYERS[2],false).layerGesture());
        assertFalse(down(LAYERS[3],false).layerGesture());
        assertTrue(up(LAYERS[0],false).consumed());
        assertTrue(up(LAYERS[1],false).consumed());
        assertFalse(up(LAYERS[2],false).consumed());
        assertFalse(up(LAYERS[3],false).consumed());
        for (int i = 0; i < LAYERS.length; i++) assertEquals(i == 3,down(LAYERS[i],false).layerGesture());
    }
    @Test public void pausingWithNoButtonsHeldAllowsTheNextGesture() {
        keys.suppressHeldGesture();
        for (int code : new int[]{KeyEvent.KEYCODE_BUTTON_A,KeyEvent.KEYCODE_BUTTON_B,KeyEvent.KEYCODE_BUTTON_X,KeyEvent.KEYCODE_BUTTON_Y}) {
            assertFalse(down(code,false).layerGesture()); up(code,false);
        }
        for (int i = 0; i < LAYERS.length; i++) assertEquals(i == 3,down(LAYERS[i],false).layerGesture());
    }
    @Test public void reconnectDiscardsOwnershipAndPartialGestures() {
        for (int code : LAYERS) down(code,true);
        keys.reset();
        for (int code : LAYERS) assertFalse(up(code,true).consumed());
        for (int i = 0; i < LAYERS.length; i++) assertEquals(i == 3,down(LAYERS[i],false).layerGesture());
    }
    @Test public void resumeWaitsForOtherButtonsTheGameAlreadyReceived() {
        down(KeyEvent.KEYCODE_BUTTON_A,false);
        for (int code : LAYERS) down(code,false);
        for (int code : LAYERS) up(code,true);
        assertTrue(keys.hasHeldPresses()); assertFalse(keys.hasConsumedPresses());
        assertFalse(up(KeyEvent.KEYCODE_BUTTON_A,true).consumed());
        assertFalse(keys.hasHeldPresses());
        assertTrue(down(KeyEvent.KEYCODE_BUTTON_A,true).consumed());
        assertTrue(up(KeyEvent.KEYCODE_BUTTON_A,true).consumed());
    }
}
