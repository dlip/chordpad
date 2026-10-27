@file:Suppress("DEPRECATION")

package io.github.dlip.chordpad

import android.app.UiAutomation
import android.content.Intent
import android.os.SystemClock
import android.test.InstrumentationTestCase
import android.view.InputDevice
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.accessibility.AccessibilityNodeInfo
import org.json.JSONArray

class DeviceTest : InstrumentationTestCase() {
    private fun deviceId(): Int {
        for (id in InputDevice.getDeviceIds()) {
            val device = InputDevice.getDevice(id) ?: continue
            if (ControllerInput.controller(device) && device.name.contains("DualSense")) return id
        }
        throw AssertionError("Connect the DualSense before running device checks")
    }

    private fun motion(leftTrigger: Float, rightTrigger: Float, rightX: Float): MotionEvent {
        val pointer = MotionEvent.PointerProperties().apply { id = 0 }
        val coordinate = MotionEvent.PointerCoords().apply {
            setAxisValue(MotionEvent.AXIS_LTRIGGER, leftTrigger)
            setAxisValue(MotionEvent.AXIS_RTRIGGER, rightTrigger)
            setAxisValue(MotionEvent.AXIS_Z, rightX)
        }
        val time = SystemClock.uptimeMillis()
        return MotionEvent.obtain(time, time, MotionEvent.ACTION_MOVE, 1, arrayOf(pointer), arrayOf(coordinate),
            0, 0, 1f, 1f, deviceId(), 0, InputDevice.SOURCE_JOYSTICK, 0)
    }

    fun testRecognitionVectorsOnAndroid() {
        val layout = instrumentation.targetContext.assets.open("legacy-layout.json").use { it.readBytes().toString(Charsets.UTF_8) }
        instrumentation.context.assets.open("chord-vectors.json").use {
            val cases = JSONArray(it.readBytes().toString(Charsets.UTF_8))
            assertEquals(cases.length(), RecognitionVectors.check(layout, cases))
        }
    }

    fun testConsumerKeyCodes() {
        val keys = arrayOf("media_play_pause", "media_next", "media_previous", "media_stop", "volume_up", "volume_down", "mute")
        val codes = intArrayOf(KeyEvent.KEYCODE_MEDIA_PLAY_PAUSE, KeyEvent.KEYCODE_MEDIA_NEXT, KeyEvent.KEYCODE_MEDIA_PREVIOUS,
            KeyEvent.KEYCODE_MEDIA_STOP, KeyEvent.KEYCODE_VOLUME_UP, KeyEvent.KEYCODE_VOLUME_DOWN, KeyEvent.KEYCODE_VOLUME_MUTE)
        for (i in keys.indices) {
            assertTrue(Action.key(keys[i]).consumerKey())
            assertEquals(codes[i], KeyboardOutput.keyCode(keys[i]))
            KeyboardOutput.validate(Action.key(keys[i]))
        }
        assertFalse(Action.key("f1").consumerKey())
        assertEquals(android.content.pm.PackageManager.PERMISSION_GRANTED,
            instrumentation.targetContext.checkSelfPermission(android.Manifest.permission.MODIFY_AUDIO_SETTINGS))
    }

    fun testMediaKeysReachPlayerWithoutEditor() {
        val automation = instrumentation.getUiAutomation(UiAutomation.FLAG_DONT_SUPPRESS_ACCESSIBILITY_SERVICES)
        val player = Intent().setClassName("io.github.dlip.chordpad.test", "io.github.dlip.chordpad.MediaTestActivity")
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK)
        try {
            instrumentation.context.startActivity(player)
            waitFor({
                automation.rootInActiveWindow?.let { it.packageName?.toString() == "io.github.dlip.chordpad.test" &&
                    it.findAccessibilityNodeInfosByText("chordpad media output test").isNotEmpty() } == true
            }, "Test player did not open")
            SystemClock.sleep(500)
            val expected = mutableListOf<String>()
            for (key in listOf("media_play_pause", "media_next", "media_previous", "media_stop")) {
                assertTrue(ConsumerOutput.send(instrumentation.targetContext, Action.key(key)))
                for (action in intArrayOf(KeyEvent.ACTION_DOWN, KeyEvent.ACTION_UP)) {
                    expected.add("${KeyEvent.keyCodeToString(KeyboardOutput.keyCode(key))}:$action")
                }
            }
            waitFor({ automation.rootInActiveWindow?.findAccessibilityNodeInfosByText(expected.joinToString(","))?.isNotEmpty() == true },
                "Media keys did not reach the player with balanced presses and releases")
        } finally {
            instrumentation.context.startActivity(player.addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP).putExtra("finish", true))
        }
    }

    fun testDualSenseAnalogAndButtonNormalization() {
        val input = ControllerInput(checkNotNull(InputDevice.getDevice(deviceId())))
        input.key(KeyEvent(0, 0, KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_BUTTON_A, 0, 0, deviceId(), 0, 0, InputDevice.SOURCE_GAMEPAD))
        assertEquals(1, input.frame().buttons)
        var event = motion(.8f, .8f, -.8f)
        input.motion(event, -1); event.recycle()
        assertEquals(1 or 64 or 128, input.frame().buttons)
        assertEquals(Action.CTRL or Action.ALT or Action.META, input.frame().modifiers)
        input.key(KeyEvent(0, 0, KeyEvent.ACTION_UP, KeyEvent.KEYCODE_BUTTON_L2, 0, 0, deviceId(), 0, 0, InputDevice.SOURCE_GAMEPAD))
        assertTrue(input.frame().buttons and 64 != 0)
        event = motion(.4f, .4f, -.4f); input.motion(event, -1); event.recycle()
        assertEquals(1 or 64 or 128, input.frame().buttons)
        event = motion(.35f, .35f, -.35f); input.motion(event, -1); event.recycle()
        assertEquals(1, input.frame().buttons); assertEquals(0, input.frame().modifiers)
    }

    private fun waitFor(condition: () -> Boolean, message: String) {
        val until = SystemClock.uptimeMillis() + 5000
        while (SystemClock.uptimeMillis() < until) {
            if (condition()) return
            SystemClock.sleep(40)
        }
        fail(message)
    }

    private fun editorText(automation: UiAutomation): String {
        val root = automation.rootInActiveWindow
        if (root == null || root.packageName?.toString() != "io.github.dlip.chordpad.test") return "<not test editor>"
        val editor = root.findFocus(AccessibilityNodeInfo.FOCUS_INPUT) ?: return "<no focus>"
        return editor.text?.toString() ?: ""
    }

    private fun press(automation: UiAutomation, code: Int) {
        val time = SystemClock.uptimeMillis()
        for (action in intArrayOf(KeyEvent.ACTION_DOWN, KeyEvent.ACTION_UP)) {
            assertTrue(automation.injectInputEvent(KeyEvent(time, SystemClock.uptimeMillis(), action, code, 0, 0,
                deviceId(), 0, 0, InputDevice.SOURCE_GAMEPAD), true))
        }
    }

    fun testTypingIntoSeparateApp() {
        val automation = instrumentation.getUiAutomation(UiAutomation.FLAG_DONT_SUPPRESS_ACCESSIBILITY_SERVICES)
        val state = AppState.get(instrumentation.targetContext)
        waitFor({ state.service != null }, "Enable chordpad controller input in accessibility settings first")
        val oldMode = state.mode
        val oldMapping = state.direct.mappings["Right stick Right"]
        try {
            val editor = Intent().setClassName("io.github.dlip.chordpad.test", "io.github.dlip.chordpad.TestEditorActivity")
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK)
            instrumentation.context.startActivity(editor)
            waitFor({ editorText(automation) == "" }, "Test editor did not become focused")
            instrumentation.runOnMainSync { state.setMode("chords"); state.setEnabled(true) }
            fun inject(event: MotionEvent) { automation.injectInputEvent(event, true); event.recycle() }
            inject(motion(0f, 0f, 0f))
            press(automation, KeyEvent.KEYCODE_BUTTON_A)
            waitFor({ editorText(automation) == "a" }, "Cross did not type a; editor=${editorText(automation)}")
            inject(motion(1f, 0f, 0f)); press(automation, KeyEvent.KEYCODE_BUTTON_A); inject(motion(0f, 0f, 0f))
            waitFor({ editorText(automation) == "aA" }, "L2+Cross did not type uppercase A")
            press(automation, KeyEvent.KEYCODE_BUTTON_L1)
            press(automation, KeyEvent.KEYCODE_BUTTON_R1)
            press(automation, KeyEvent.KEYCODE_BUTTON_START)
            waitFor({ editorText(automation) == "a \n" }, "Backspace/Space/Options did not reach the other app")
            instrumentation.runOnMainSync { state.setMode("direct"); state.map("Right stick Right", Action.text("z")) }
            inject(motion(0f, 0f, 1f)); inject(motion(0f, 0f, 0f))
            waitFor({ editorText(automation) == "a \nz" }, "Remapped right stick did not type z")
        } finally {
            instrumentation.runOnMainSync { state.setEnabled(false); state.setMode(oldMode); state.map("Right stick Right", oldMapping) }
        }
    }
}
