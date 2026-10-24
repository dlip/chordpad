package io.github.dlip.chordpad

import android.view.KeyEvent
import org.junit.Assert.*
import org.junit.Test

class GamepadKeysTest {
    private val layers = intArrayOf(KeyEvent.KEYCODE_BUTTON_L1, KeyEvent.KEYCODE_BUTTON_R1, KeyEvent.KEYCODE_BUTTON_L2, KeyEvent.KEYCODE_BUTTON_R2)
    private val keys = GamepadKeys()
    private fun down(code: Int, typing: Boolean) = keys.update(code, KeyEvent.ACTION_DOWN, 0, typing)
    private fun up(code: Int, typing: Boolean) = keys.update(code, KeyEvent.ACTION_UP, 0, typing)

    @Test fun pressesKeepTheirRoutingAcrossPauseAndResume() {
        assertTrue(down(KeyEvent.KEYCODE_BUTTON_A, true).consumed)
        assertTrue(keys.hasConsumedPresses())
        assertTrue(keys.update(KeyEvent.KEYCODE_BUTTON_A, KeyEvent.ACTION_DOWN, 1, false).consumed)
        assertTrue(up(KeyEvent.KEYCODE_BUTTON_A, false).consumed)
        assertFalse(keys.hasConsumedPresses())
        assertFalse(down(KeyEvent.KEYCODE_BUTTON_A, false).consumed)
        assertFalse(keys.hasConsumedPresses())
        assertFalse(keys.update(KeyEvent.KEYCODE_BUTTON_A, KeyEvent.ACTION_DOWN, 1, true).consumed)
        assertFalse(up(KeyEvent.KEYCODE_BUTTON_A, true).consumed)
        assertFalse(up(KeyEvent.KEYCODE_BUTTON_B, true).consumed)
        assertFalse(keys.update(KeyEvent.KEYCODE_BUTTON_B, KeyEvent.ACTION_DOWN, 1, true).consumed)
    }
    @Test fun fourDigitalButtonsResumeWithoutReceivingMotionEvents() {
        for (i in layers.indices) {
            val result = down(layers[i], false)
            assertFalse(result.consumed)
            assertEquals(i == 3, result.layerGesture)
        }
        assertFalse(down(layers[3], true).layerGesture)
        for (code in layers) assertFalse(up(code, true).consumed)
        for (i in 3 downTo 0) assertEquals(i == 0, down(layers[i], false).layerGesture)
    }
    @Test fun partialReleaseCannotRetriggerTheGesture() {
        for (code in layers) down(code, false)
        up(layers[0], true)
        assertFalse(down(layers[0], true).layerGesture)
        for (code in layers) up(code, true)
        for (i in layers.indices) assertEquals(i == 3, down(layers[i], true).layerGesture)
    }
    @Test fun analogPauseBeforeTriggerButtonsCannotImmediatelyResume() {
        down(layers[0], true); down(layers[1], true)
        keys.suppressHeldGesture()
        assertFalse(down(layers[2], false).layerGesture)
        assertFalse(down(layers[3], false).layerGesture)
        assertTrue(up(layers[0], false).consumed)
        assertTrue(up(layers[1], false).consumed)
        assertFalse(up(layers[2], false).consumed)
        assertFalse(up(layers[3], false).consumed)
        for (i in layers.indices) assertEquals(i == 3, down(layers[i], false).layerGesture)
    }
    @Test fun pausingWithNoButtonsHeldAllowsTheNextGesture() {
        keys.suppressHeldGesture()
        for (code in intArrayOf(KeyEvent.KEYCODE_BUTTON_A, KeyEvent.KEYCODE_BUTTON_B, KeyEvent.KEYCODE_BUTTON_X, KeyEvent.KEYCODE_BUTTON_Y)) {
            assertFalse(down(code, false).layerGesture); up(code, false)
        }
        for (i in layers.indices) assertEquals(i == 3, down(layers[i], false).layerGesture)
    }
    @Test fun reconnectDiscardsOwnershipAndPartialGestures() {
        for (code in layers) down(code, true)
        keys.reset()
        for (code in layers) assertFalse(up(code, true).consumed)
        for (i in layers.indices) assertEquals(i == 3, down(layers[i], false).layerGesture)
    }
    @Test fun resumeWaitsForOtherButtonsTheGameAlreadyReceived() {
        down(KeyEvent.KEYCODE_BUTTON_A, false)
        for (code in layers) down(code, false)
        for (code in layers) up(code, true)
        assertTrue(keys.hasHeldPresses()); assertFalse(keys.hasConsumedPresses())
        assertFalse(up(KeyEvent.KEYCODE_BUTTON_A, true).consumed)
        assertFalse(keys.hasHeldPresses())
        assertTrue(down(KeyEvent.KEYCODE_BUTTON_A, true).consumed)
        assertTrue(up(KeyEvent.KEYCODE_BUTTON_A, true).consumed)
    }
}
