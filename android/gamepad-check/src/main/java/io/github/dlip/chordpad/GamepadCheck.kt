package io.github.dlip.chordpad

import android.accessibilityservice.AccessibilityServiceInfo
import android.app.Activity
import android.app.Instrumentation
import android.app.UiAutomation
import android.content.Intent
import android.os.Bundle
import android.os.ParcelFileDescriptor
import android.os.SystemClock
import android.view.InputDevice
import android.view.KeyEvent
import android.view.accessibility.AccessibilityNodeInfo
import java.io.IOException
import java.io.PrintWriter
import java.io.StringWriter
import kotlin.math.abs

class GamepadCheck : Instrumentation() {
    private lateinit var automation: UiAutomation
    private var pad: VirtualGamepad? = null
    private var game: GamepadTestActivity? = null
    private val layers = intArrayOf(KeyEvent.KEYCODE_BUTTON_L1, KeyEvent.KEYCODE_BUTTON_R1, KeyEvent.KEYCODE_BUTTON_L2, KeyEvent.KEYCODE_BUTTON_R2)

    override fun onCreate(arguments: Bundle?) { super.onCreate(arguments); start() }
    override fun onStart() {
        val result = Bundle()
        try {
            checkPassthrough()
            result.putString("stream", "Gamepad passthrough and four-button resume passed.\n")
            finish(Activity.RESULT_OK, result)
        } catch (error: Throwable) {
            val trace = StringWriter(); error.printStackTrace(PrintWriter(trace))
            result.putString("stream", trace.toString()); finish(Activity.RESULT_CANCELED, result)
        }
    }
    private fun require(condition: Boolean, message: String) { if (!condition) throw AssertionError(message) }
    private fun waitFor(condition: () -> Boolean, message: String) {
        val end = SystemClock.uptimeMillis() + 5000
        while (SystemClock.uptimeMillis() < end) { if (condition()) return; SystemClock.sleep(40) }
        throw AssertionError(message)
    }
    private fun startButton(): AccessibilityNodeInfo? {
        val root = automation.rootInActiveWindow ?: return null
        if (root.packageName?.toString() != "io.github.dlip.chordpad") return null
        return root.findAccessibilityNodeInfosByViewId("io.github.dlip.chordpad:id/start_remapping").firstOrNull()
    }
    private fun openChordpad() {
        try {
            ParcelFileDescriptor.AutoCloseInputStream(automation.executeShellCommand("am start -f 0x10008000 -n io.github.dlip.chordpad/.MainActivity"))
                .bufferedReader().use { reader -> reader.lineSequence().forEach { } }
        } catch (error: IOException) { throw IllegalStateException(error) }
        waitFor({ startButton() != null }, "chordpad did not open")
    }
    private fun chooseController(name: String) { click("Choose controller"); click(name); idle() }
    private fun click(text: String) {
        waitFor({
            automation.rootInActiveWindow?.findAccessibilityNodeInfosByText(text)?.any {
                it.text?.toString() == text && it.performAction(AccessibilityNodeInfo.ACTION_CLICK)
            } == true
        }, "Button not found: $text")
    }
    private fun key(code: Int, action: Int) { checkNotNull(pad).key(code, action); SystemClock.sleep(20) }
    private fun tap(code: Int) { key(code, KeyEvent.ACTION_DOWN); key(code, KeyEvent.ACTION_UP) }
    private fun motion(x: Float, y: Float, l2: Float, r2: Float, hat: Float) {
        checkNotNull(pad).motion(x, y, l2, r2, hat); SystemClock.sleep(20)
    }
    private fun keys(): List<String> {
        val copy = mutableListOf<String>(); runOnMainSync { copy.addAll(checkNotNull(game).keys) }; return copy
    }
    private fun idle() { SystemClock.sleep(200); waitForIdleSync() }

    private fun checkPassthrough() {
        automation = getUiAutomation(UiAutomation.FLAG_DONT_SUPPRESS_ACCESSIBILITY_SERVICES)
        val info = automation.serviceInfo
        info.flags = info.flags or AccessibilityServiceInfo.FLAG_REPORT_VIEW_IDS
        automation.serviceInfo = info
        openChordpad()
        var original: String? = null
        for (id in InputDevice.getDeviceIds()) {
            val device = InputDevice.getDevice(id) ?: continue
            if (device.supportsSource(InputDevice.SOURCE_GAMEPAD) &&
                automation.rootInActiveWindow.findAccessibilityNodeInfosByText(device.name).isNotEmpty()) original = device.name
        }
        if (original == null) {
            for (id in InputDevice.getDeviceIds()) {
                val device = InputDevice.getDevice(id) ?: continue
                if (device.supportsSource(InputDevice.SOURCE_GAMEPAD) && device.name != VirtualGamepad.NAME) original = device.name
            }
            require(original != null, "Connect and select a controller before running this check")
            chooseController(checkNotNull(original))
        }
        val originalController = checkNotNull(original)
        var failure: Throwable? = null
        try {
            if (startButton()?.text?.toString() == "Stop remapping") { startButton()?.performAction(AccessibilityNodeInfo.ACTION_CLICK); idle() }
            pad = VirtualGamepad(automation)
            waitFor({ InputDevice.getDeviceIds().any { InputDevice.getDevice(it)?.name == VirtualGamepad.NAME } }, "uinput did not create the test gamepad")
            chooseController(VirtualGamepad.NAME)
            require(checkNotNull(startButton()).performAction(AccessibilityNodeInfo.ACTION_CLICK), "Could not start remapping")
            waitFor({ startButton()?.text?.toString() == "Stop remapping" }, "Enable chordpad's accessibility service first")
            game = startActivitySync(Intent(targetContext, GamepadTestActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) as GamepadTestActivity
            idle(); motion(0f, 0f, 0f, 0f, 0f); idle()
            tap(KeyEvent.KEYCODE_BUTTON_A); idle()
            require(keys().isEmpty(), "Typing mode leaked gamepad buttons to the game")
            for (code in layers) key(code, KeyEvent.ACTION_DOWN)
            motion(0f, 0f, 1f, 1f, 0f); idle()
            for (code in layers) key(code, KeyEvent.ACTION_UP)
            motion(0f, 0f, 0f, 0f, 0f); idle()
            require(keys().isEmpty(), "Pausing leaked unmatched layer-button releases: ${keys()}")
            val buttons = intArrayOf(KeyEvent.KEYCODE_BUTTON_A, KeyEvent.KEYCODE_BUTTON_B, KeyEvent.KEYCODE_BUTTON_X, KeyEvent.KEYCODE_BUTTON_Y,
                KeyEvent.KEYCODE_BUTTON_L1, KeyEvent.KEYCODE_BUTTON_R1, KeyEvent.KEYCODE_BUTTON_L2, KeyEvent.KEYCODE_BUTTON_R2)
            val expected = mutableListOf<String>()
            for (code in buttons) { tap(code); expected.add("$code:${KeyEvent.ACTION_DOWN}"); expected.add("$code:${KeyEvent.ACTION_UP}") }
            idle(); require(expected == keys(), "Paused game did not receive balanced buttons: ${keys()}")
            motion(.75f, -.75f, .4f, .8f, 1f); idle()
            var axesReceived = false
            runOnMainSync {
                val game = checkNotNull(game)
                axesReceived = game.motions > 0 && abs(game.x - .75f) < .01f && abs(game.y + .75f) < .01f &&
                    abs(game.rx - .75f) < .01f && abs(game.ry + .75f) < .01f &&
                    abs(game.l2 - .4f) < .01f && abs(game.r2 - .8f) < .01f && game.hat == 1f
            }
            require(axesReceived, "Paused game did not receive sticks, analog triggers, and D-pad axes")
            motion(0f, 0f, 0f, 0f, 0f); idle()
            key(KeyEvent.KEYCODE_BUTTON_A, KeyEvent.ACTION_DOWN); expected.add("${KeyEvent.KEYCODE_BUTTON_A}:${KeyEvent.ACTION_DOWN}")
            for (code in layers) { key(code, KeyEvent.ACTION_DOWN); expected.add("$code:${KeyEvent.ACTION_DOWN}") }
            motion(0f, 0f, 1f, 1f, 0f); idle()
            for (code in layers) { key(code, KeyEvent.ACTION_UP); expected.add("$code:${KeyEvent.ACTION_UP}") }
            motion(0f, 0f, 0f, 0f, 0f)
            key(KeyEvent.KEYCODE_BUTTON_A, KeyEvent.ACTION_UP); expected.add("${KeyEvent.KEYCODE_BUTTON_A}:${KeyEvent.ACTION_UP}")
            idle(); require(expected == keys(), "Resuming left gamepad buttons unbalanced: ${keys()}")
            tap(KeyEvent.KEYCODE_BUTTON_B); idle()
            require(expected == keys(), "Four-button shortcut did not resume typing")
            openChordpad()
            val field = automation.rootInActiveWindow.findAccessibilityNodeInfosByViewId("io.github.dlip.chordpad:id/test_field")[0]
            field.performAction(AccessibilityNodeInfo.AccessibilityAction.ACTION_SHOW_ON_SCREEN.id)
            require(field.performAction(AccessibilityNodeInfo.ACTION_FOCUS), "Could not focus the typing field"); idle()
            tap(KeyEvent.KEYCODE_BUTTON_R1)
            waitFor({
                automation.rootInActiveWindow?.findAccessibilityNodeInfosByViewId("io.github.dlip.chordpad:id/test_field")?.firstOrNull()?.text?.toString() == " "
            }, "Typing did not work after resuming from gamepad mode")
        } catch (error: Throwable) { failure = error; throw error }
        finally {
            try {
                if (game != null) {
                    for (code in layers) key(code, KeyEvent.ACTION_UP)
                    key(KeyEvent.KEYCODE_BUTTON_A, KeyEvent.ACTION_UP); motion(0f, 0f, 0f, 0f, 0f); idle()
                }
                openChordpad()
                if (startButton()?.text?.toString() == "Stop remapping") startButton()?.performAction(AccessibilityNodeInfo.ACTION_CLICK)
                chooseController(originalController)
            } catch (error: Throwable) {
                val previous = failure
                if (previous != null) previous.addSuppressed(error) else throw error
            } finally {
                pad?.close()
                game?.let { runOnMainSync { it.finish() } }
            }
        }
    }
}
