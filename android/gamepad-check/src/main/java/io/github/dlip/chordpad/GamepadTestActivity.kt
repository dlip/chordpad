package io.github.dlip.chordpad

import android.app.Activity
import android.os.Bundle
import android.view.InputDevice
import android.view.KeyEvent
import android.view.MotionEvent
import android.widget.TextView
import io.github.dlip.chordpad.gamepadcheck.R

class GamepadTestActivity : Activity() {
    val keys = mutableListOf<String>()
    var motions = 0
    var x = 0f
    var y = 0f
    var rx = 0f
    var ry = 0f
    var l2 = 0f
    var r2 = 0f
    var hat = 0f
    private lateinit var output: TextView

    override fun onCreate(bundle: Bundle?) {
        super.onCreate(bundle)
        output = TextView(this).apply { textSize = 24f; setPadding(32, 32, 32, 32) }
        setContentView(output)
        refresh()
    }
    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        if (event.isFromSource(InputDevice.SOURCE_GAMEPAD) || event.isFromSource(InputDevice.SOURCE_DPAD)) {
            keys.add("${event.keyCode}:${event.action}"); refresh(); return true
        }
        return super.dispatchKeyEvent(event)
    }
    override fun onGenericMotionEvent(event: MotionEvent): Boolean {
        if (event.isFromSource(InputDevice.SOURCE_JOYSTICK)) {
            motions++
            x = event.getAxisValue(MotionEvent.AXIS_X); y = event.getAxisValue(MotionEvent.AXIS_Y)
            rx = event.getAxisValue(MotionEvent.AXIS_Z); ry = event.getAxisValue(MotionEvent.AXIS_RZ)
            l2 = event.getAxisValue(MotionEvent.AXIS_LTRIGGER); r2 = event.getAxisValue(MotionEvent.AXIS_RTRIGGER)
            hat = event.getAxisValue(MotionEvent.AXIS_HAT_X)
            refresh(); return true
        }
        return super.onGenericMotionEvent(event)
    }
    private fun refresh() { output.text = getString(R.string.gamepad_status, keys.size, motions) }
}
