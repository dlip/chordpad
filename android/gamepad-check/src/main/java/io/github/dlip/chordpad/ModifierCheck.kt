package io.github.dlip.chordpad

import android.app.Activity
import android.app.Instrumentation
import android.app.UiAutomation
import android.content.Intent
import android.os.Bundle
import android.os.ParcelFileDescriptor
import android.os.SystemClock
import android.view.InputDevice
import android.view.MotionEvent
import java.io.PrintWriter
import java.io.StringWriter

class ModifierCheck : Instrumentation() {
    private var withHelper = true
    private var web = false
    private var pointer = "mouse"
    override fun onCreate(arguments: Bundle?) {
        super.onCreate(arguments)
        withHelper = arguments?.getString("helper") != "false"
        web = arguments?.getString("web") == "true"
        pointer = arguments?.getString("pointer") ?: "mouse"
        start()
    }
    private fun waitFor(message: String, condition: () -> Boolean) {
        val end = SystemClock.uptimeMillis() + 5000
        while (SystemClock.uptimeMillis() < end) { if (condition()) return; SystemClock.sleep(20) }
        throw AssertionError(message)
    }
    private lateinit var automation: UiAutomation
    private var pad: VirtualGamepad? = null
    private fun startButton() = automation.rootInActiveWindow
        ?.findAccessibilityNodeInfosByViewId("io.github.dlip.chordpad:id/start_remapping")?.firstOrNull()
    private fun click(text: String) {
        waitFor("Button not found: $text") {
            automation.rootInActiveWindow?.findAccessibilityNodeInfosByText(text)?.any {
                it.text?.toString() == text && it.performAction(android.view.accessibility.AccessibilityNodeInfo.ACTION_CLICK)
            } == true
        }
        SystemClock.sleep(200)
    }
    private fun openChordpad() {
        ParcelFileDescriptor.AutoCloseInputStream(automation.executeShellCommand("am start -f 0x10008000 -n io.github.dlip.chordpad/.MainActivity"))
            .bufferedReader().use { it.readText() }
        waitFor("Chordpad did not open") { automation.rootInActiveWindow?.packageName?.toString() == "io.github.dlip.chordpad" }
        SystemClock.sleep(200)
    }
    private fun shift(down: Boolean) {
        checkNotNull(pad).key(android.view.KeyEvent.KEYCODE_BUTTON_L2,
            if (down) android.view.KeyEvent.ACTION_DOWN else android.view.KeyEvent.ACTION_UP)
        checkNotNull(pad).motion(0f, 0f, if (down) 1f else 0f, 0f, 0f)
    }
    override fun onStart() {
        val result = Bundle()
        var resultCode = Activity.RESULT_CANCELED
        var activity: ModifierTestActivity? = null
        var keyboard: ParcelFileDescriptor.AutoCloseOutputStream? = null
        var reader: ParcelFileDescriptor? = null
        var original: String? = null
        try {
            automation = getUiAutomation(UiAutomation.FLAG_DONT_SUPPRESS_ACCESSIBILITY_SERVICES)
            automation.serviceInfo = automation.serviceInfo.apply {
                flags = flags or android.accessibilityservice.AccessibilityServiceInfo.FLAG_REPORT_VIEW_IDS
            }
            original = InputDevice.getDeviceIds().asIterable().mapNotNull { InputDevice.getDevice(it) }
                .firstOrNull { it.supportsSource(InputDevice.SOURCE_GAMEPAD) && it.name != VirtualGamepad.NAME }?.name
            openChordpad()
            if (startButton()?.text?.toString() == "Stop remapping") {
                check(startButton()?.performAction(android.view.accessibility.AccessibilityNodeInfo.ACTION_CLICK) == true)
                waitFor("Could not stop remapping") { startButton()?.text?.toString() == "Start remapping" }
            }
            pad = VirtualGamepad(automation)
            waitFor("Test controller did not register") { InputDevice.getDeviceIds().any { InputDevice.getDevice(it)?.name == VirtualGamepad.NAME } }
            click("Choose controller"); click(VirtualGamepad.NAME)
            waitFor("Controller service did not connect") {
                automation.rootInActiveWindow?.findAccessibilityNodeInfosByText("Controller service connected")?.isNotEmpty() == true
            }
            check(startButton()?.performAction(android.view.accessibility.AccessibilityNodeInfo.ACTION_CLICK) == true)
            waitFor("Could not start remapping") { startButton()?.text?.toString() == "Stop remapping" }
            activity = startActivitySync(Intent(targetContext, ModifierTestActivity::class.java).putExtra("web", web).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) as ModifierTestActivity
            waitForIdleSync()
            if (web) waitFor("Web editor did not load") { activity.webReady }
            fun js(expression: String): String {
                val done = java.util.concurrent.CountDownLatch(1)
                var answer = "null"
                runOnMainSync { checkNotNull(activity.web).evaluateJavascript(expression) { answer = it; done.countDown() } }
                check(done.await(5, java.util.concurrent.TimeUnit.SECONDS))
                return answer
            }
            fun selection(): Pair<Int, Int> {
                if (web) {
                    val value = org.json.JSONArray(js("[getSelection().anchorOffset,getSelection().focusOffset]"))
                    return value.getInt(0) to value.getInt(1)
                }
                var value = -1 to -1
                runOnMainSync { value = activity.editor.selectionStart to activity.editor.selectionEnd }
                return value
            }
            if (web) {
                val point = FloatArray(2)
                runOnMainSync {
                    val location = IntArray(2); checkNotNull(activity.web).getLocationOnScreen(location)
                    point[0] = location[0] + 60 * targetContext.resources.displayMetrics.density
                    point[1] = location[1] + 44 * targetContext.resources.displayMetrics.density
                }
                val time = SystemClock.uptimeMillis()
                for (action in intArrayOf(MotionEvent.ACTION_DOWN, MotionEvent.ACTION_UP)) {
                    val properties = arrayOf(MotionEvent.PointerProperties().apply { id = 0; toolType = MotionEvent.TOOL_TYPE_FINGER })
                    val coords = arrayOf(MotionEvent.PointerCoords().apply { x = point[0]; y = point[1]; pressure = 1f })
                    val event = MotionEvent.obtain(time, SystemClock.uptimeMillis(), action, 1, properties, coords,
                        0, 0, 1f, 1f, 0, 0, InputDevice.SOURCE_TOUCHSCREEN, 0)
                    check(automation.injectInputEvent(event, true)); event.recycle()
                }
                js("c.focus();let r=document.createRange();r.setStart(c.firstChild,2);r.collapse(true);let s=getSelection();s.removeAllRanges();s.addRange(r)")
                SystemClock.sleep(400)
            }
            val pipes = automation.executeShellCommandRw("uinput -")
            reader = pipes[0]
            keyboard = ParcelFileDescriptor.AutoCloseOutputStream(pipes[1])
            fun send(command: String) { keyboard.write((command + "\n").toByteArray()); keyboard.flush() }
            fun inject(id: Int, events: String) {
                send("{\"id\":$id,\"command\":\"updateTimeBase\"}")
                send("{\"id\":$id,\"command\":\"inject\",\"events\":[$events,0,0,0]}")
            }
            send("""{"id":2,"command":"register","name":"chordpad modifier check mouse","vid":1,"pid":2,"bus":"usb","configuration":[{"type":"UI_SET_EVBIT","data":["EV_KEY","EV_REL"]},{"type":"UI_SET_KEYBIT","data":[272]},{"type":"UI_SET_RELBIT","data":[0,1]}]}""")
            if (withHelper) waitFor("Virtual keyboard did not register") {
                InputDevice.getDeviceIds().any { InputDevice.getDevice(it)?.name == "Chordpad modifiers" }
            } else check(InputDevice.getDeviceIds().none { InputDevice.getDevice(it)?.name == "Chordpad modifiers" })
            SystemClock.sleep(500)
            waitFor("Test activity is not focused") { activity.focused }
            fun globalShift(): Boolean {
                val pipe = automation.executeShellCommand("dumpsys input")
                val state = ParcelFileDescriptor.AutoCloseInputStream(pipe).bufferedReader().use { it.readText() }
                val match = Regex("""Device \d+: Chordpad modifiers[\s\S]*?MetaState: 0x([0-9a-f]+)""").find(state)
                return match != null && match.groupValues[1].toInt(16) and 1 != 0
            }
            fun activated(message: String) {
                if (withHelper) waitFor(message) { globalShift() } else SystemClock.sleep(100)
            }
            val point = FloatArray(2)
            if (web) {
                val value = org.json.JSONArray(js("(()=>{let r=document.createRange();r.setStart(c.firstChild,10);r.collapse(true);let b=r.getBoundingClientRect();return [b.x,(b.top+b.bottom)/2,devicePixelRatio]})()"))
                runOnMainSync {
                    val location = IntArray(2); checkNotNull(activity.web).getLocationOnScreen(location)
                    point[0] = location[0] + (value.getDouble(0)*value.getDouble(2)).toFloat()
                    point[1] = location[1] + (value.getDouble(1)*value.getDouble(2)).toFloat()
                }
            } else runOnMainSync {
                val location = IntArray(2); activity.editor.getLocationOnScreen(location)
                point[0] = location[0] + activity.editor.totalPaddingLeft + activity.editor.layout.getPrimaryHorizontal(10)
                point[1] = location[1] + activity.editor.totalPaddingTop + (activity.editor.layout.getLineTop(0) + activity.editor.layout.getLineBottom(0)).toFloat() / 2
            }
            shift(true)
            activated("Shift did not activate in Android input reader")
            inject(2,"2,0,-10000,2,1,-10000")
            SystemClock.sleep(60)
            for (i in 0..200) {
                val dx = if (activity.mouseHover) ((point[0] - activity.mouseX) / 3).toInt().coerceIn(-10,10) else 10
                val dy = if (activity.mouseHover) ((point[1] - activity.mouseY) / 3).toInt().coerceIn(-10,10) else 20
                if (activity.mouseHover && kotlin.math.abs(activity.mouseX-point[0]) < 4 && kotlin.math.abs(activity.mouseY-point[1]) < 4) break
                inject(2,"2,0,$dx,2,1,$dy")
                SystemClock.sleep(20)
            }
            check(activity.mouseHover && kotlin.math.abs(activity.mouseX-point[0]) < 5 && kotlin.math.abs(activity.mouseY-point[1]) < 5) { "Mouse did not reach text target: ${activity.mouseX},${activity.mouseY} vs ${point.toList()}" }
            if (pointer == "mouse") {
                inject(2,"1,272,1"); SystemClock.sleep(40); inject(2,"1,272,0")
            } else {
                val time = SystemClock.uptimeMillis()
                val deviceId = InputDevice.getDeviceIds().first { InputDevice.getDevice(it)?.name == VirtualGamepad.NAME }
                val properties = arrayOf(MotionEvent.PointerProperties().apply { id = 0; toolType = MotionEvent.TOOL_TYPE_FINGER })
                val coords = arrayOf(MotionEvent.PointerCoords().apply { x = point[0]; y = point[1]; pressure = 1f })
                val click = pointer == "touchpad_click"
                val actions = if (click) intArrayOf(MotionEvent.ACTION_DOWN, MotionEvent.ACTION_BUTTON_PRESS, MotionEvent.ACTION_BUTTON_RELEASE, MotionEvent.ACTION_UP)
                    else intArrayOf(MotionEvent.ACTION_DOWN, MotionEvent.ACTION_UP)
                for (action in actions) {
                    val buttons = if (click && action in intArrayOf(MotionEvent.ACTION_DOWN, MotionEvent.ACTION_BUTTON_PRESS)) MotionEvent.BUTTON_PRIMARY else 0
                    val event = MotionEvent.obtain(time, SystemClock.uptimeMillis(), action, 1, properties, coords,
                        if (withHelper) android.view.KeyEvent.META_SHIFT_ON else 0, buttons, 1f, 1f, deviceId, 0, InputDevice.SOURCE_MOUSE, 0)
                    if (action == MotionEvent.ACTION_BUTTON_PRESS || action == MotionEvent.ACTION_BUTTON_RELEASE) MotionEvent::class.java.getDeclaredMethod("setActionButton", Int::class.javaPrimitiveType).invoke(event, MotionEvent.BUTTON_PRIMARY)
                    check(automation.injectInputEvent(event, true)); event.recycle(); SystemClock.sleep(30)
                }
            }
            if (withHelper) waitFor("Mouse click did not carry real global Shift") { activity.mouseShift }
            SystemClock.sleep(200)
            val (mouseStart, mouseEnd) = selection()
            check(mouseStart != mouseEnd && minOf(mouseStart,mouseEnd) == 2) { "Shift-click did not extend selection: $mouseStart..$mouseEnd" }
            shift(false)
            waitFor("Shift release did not clear Android input state") { !globalShift() }
            if (web) js("c.focus();let r=document.createRange();r.setStart(c.firstChild,2);r.collapse(true);let s=getSelection();s.removeAllRanges();s.addRange(r)")
            else runOnMainSync { activity.editor.setSelection(2) }
            SystemClock.sleep(400)
            shift(true)
            activated("Shift did not reactivate")
            val time = SystemClock.uptimeMillis()
            for (action in intArrayOf(MotionEvent.ACTION_DOWN, MotionEvent.ACTION_UP)) {
                val properties = arrayOf(MotionEvent.PointerProperties().apply { id = 0; toolType = MotionEvent.TOOL_TYPE_FINGER })
                val coords = arrayOf(MotionEvent.PointerCoords().apply { x = point[0]; y = point[1]; pressure = 1f })
                val event = MotionEvent.obtain(time, SystemClock.uptimeMillis(), action, 1, properties, coords,
                    0, 0, 1f, 1f, 0, 0, InputDevice.SOURCE_TOUCHSCREEN, 0)
                check(automation.injectInputEvent(event, true))
                event.recycle(); SystemClock.sleep(30)
            }
            SystemClock.sleep(200)
            val (start, end) = selection()
            check(start != end && minOf(start,end) == 2) { "Shift-tap did not extend selection: $start..$end" }
            shift(false)
            waitFor("Shift remained active after release") { !globalShift() }
            shift(true); activated("Shift did not activate before disconnect")
            checkNotNull(pad).close(); pad = null
            waitFor("Controller disconnect left Shift active") { !globalShift() }
            result.putString("stream", "Shift-click selection: $mouseStart..$mouseEnd; Shift-tap selection: $start..$end; global helper: $withHelper.\n")
            resultCode = Activity.RESULT_OK
        } catch (error: Throwable) {
            val text = StringWriter(); error.printStackTrace(PrintWriter(text))
            result.putString("stream", "focused=${activity?.focused}\n"+text.toString())
        } finally {
            keyboard?.close(); reader?.close(); pad?.close()
            try {
                openChordpad()
                if (startButton()?.text?.toString() == "Stop remapping") {
                    startButton()?.performAction(android.view.accessibility.AccessibilityNodeInfo.ACTION_CLICK)
                    waitFor("Could not stop remapping after test") { startButton()?.text?.toString() == "Start remapping" }
                }
                original?.let { click("Choose controller"); click(it) }
            } catch (error: Throwable) {
                resultCode = Activity.RESULT_CANCELED
                result.putString("stream", (result.getString("stream") ?: "") + "Cleanup failed: $error\n")
            }
            activity?.let { runOnMainSync { it.finish() } }
        }
        finish(resultCode, result)
    }
}
