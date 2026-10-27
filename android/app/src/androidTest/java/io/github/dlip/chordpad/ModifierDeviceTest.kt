@file:Suppress("DEPRECATION")
package io.github.dlip.chordpad

import android.app.UiAutomation
import android.os.ParcelFileDescriptor
import android.os.SystemClock
import android.test.InstrumentationTestCase
import android.view.InputDevice
import android.view.KeyEvent

class ModifierDeviceTest : InstrumentationTestCase() {
    private fun waitFor(message: String, condition: () -> Boolean) {
        val deadline = SystemClock.uptimeMillis() + 5000
        while (SystemClock.uptimeMillis() < deadline) {
            if (condition()) return
            SystemClock.sleep(20)
        }
        fail(message)
    }

    fun testVirtualKeyboardModifierCombinations() {
        val automation = instrumentation.getUiAutomation(UiAutomation.FLAG_DONT_SUPPRESS_ACCESSIBILITY_SERVICES)
        lateinit var state: AppState
        instrumentation.runOnMainSync { state = AppState.get(instrumentation.targetContext) }
        val apk = instrumentation.targetContext.applicationInfo.sourceDir
        val pipe = automation.executeShellCommand("app_process -Djava.class.path=$apk /system/bin --nice-name=chordpad-modifiers io.github.dlip.chordpad.ModifierHelper")
        try {
            waitFor("ADB helper did not connect") { state.globalModifiers.connected }
            val all = KeyEvent.META_CTRL_ON or KeyEvent.META_ALT_ON or KeyEvent.META_SHIFT_ON or KeyEvent.META_META_ON
            fun meta(): Int {
                val output = automation.executeShellCommand("dumpsys input")
                val dump = ParcelFileDescriptor.AutoCloseInputStream(output).bufferedReader().use { it.readText() }
                val match = Regex("""Device \d+: Chordpad modifiers[\s\S]*?MetaState: 0x([0-9a-f]+)""").find(dump)
                return match?.groupValues?.get(1)?.toInt(16)?.and(all) ?: -1
            }
            val bits = intArrayOf(KeyEvent.META_CTRL_ON, KeyEvent.META_ALT_ON, KeyEvent.META_SHIFT_ON, KeyEvent.META_META_ON)
            for (mask in (0..15).toList() + listOf(15, 0)) {
                val expected = bits.indices.filter { mask and (1 shl it) != 0 }.fold(0) { value, i -> value or bits[i] }
                state.globalModifiers.set(mask)
                waitFor("Android global meta state did not match mask $mask") { meta() == expected }
            }
            assertTrue(InputDevice.getDeviceIds().any { InputDevice.getDevice(it)?.name == ModifierProtocol.DEVICE })
        } finally {
            state.globalModifiers.set(0)
            pipe.close()
        }
    }
}
