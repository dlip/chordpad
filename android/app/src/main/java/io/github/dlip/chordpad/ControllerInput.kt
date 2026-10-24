package io.github.dlip.chordpad

import android.view.InputDevice
import android.view.KeyEvent
import android.view.MotionEvent

class ControllerInput(private val device: InputDevice) {
    private var buttons = 0
    private var dpad = 0
    private var menu = 0
    private var lx = 0
    private var ly = 0
    private var rx = 0
    private var ry = 0
    var leftX = 0f; private set
    var leftY = 0f; private set
    var rightX = 0f; private set
    var rightY = 0f; private set
    var l2 = 0f; private set
    var r2 = 0f; private set
    private val hats = range(MotionEvent.AXIS_HAT_X) && range(MotionEvent.AXIS_HAT_Y)
    private val analogLeft = range(MotionEvent.AXIS_LTRIGGER) || range(MotionEvent.AXIS_BRAKE)
    private val analogRight = range(MotionEvent.AXIS_RTRIGGER) || range(MotionEvent.AXIS_GAS)

    private fun range(axis: Int) = device.getMotionRange(axis, InputDevice.SOURCE_JOYSTICK) != null

    fun key(event: KeyEvent): Boolean {
        val button = when (event.keyCode) {
            KeyEvent.KEYCODE_BUTTON_A -> A; KeyEvent.KEYCODE_BUTTON_B -> B
            KeyEvent.KEYCODE_BUTTON_X -> X; KeyEvent.KEYCODE_BUTTON_Y -> Y
            KeyEvent.KEYCODE_BUTTON_L1 -> L1; KeyEvent.KEYCODE_BUTTON_R1 -> R1
            KeyEvent.KEYCODE_BUTTON_L2 -> L2; KeyEvent.KEYCODE_BUTTON_R2 -> R2
            else -> 0
        }
        val direction = when (event.keyCode) {
            KeyEvent.KEYCODE_DPAD_LEFT -> 1; KeyEvent.KEYCODE_DPAD_RIGHT -> 2
            KeyEvent.KEYCODE_DPAD_UP -> 4; KeyEvent.KEYCODE_DPAD_DOWN -> 8
            else -> 0
        }
        val extra = when (event.keyCode) {
            KeyEvent.KEYCODE_BUTTON_START -> 1; KeyEvent.KEYCODE_BUTTON_SELECT -> 2
            KeyEvent.KEYCODE_BUTTON_THUMBL -> 4; KeyEvent.KEYCODE_BUTTON_THUMBR -> 8
            else -> 0
        }
        if (button == 0 && direction == 0 && extra == 0) return false
        if (event.repeatCount != 0 || event.action !in listOf(KeyEvent.ACTION_UP, KeyEvent.ACTION_DOWN)) return true
        val down = event.action == KeyEvent.ACTION_DOWN
        if (!((button == L2 && analogLeft) || (button == R2 && analogRight))) {
            buttons = if (down) buttons or button else buttons and button.inv()
        }
        if (!hats) dpad = if (down) dpad or direction else dpad and direction.inv()
        menu = if (down) menu or extra else menu and extra.inv()
        return true
    }

    private fun axis(event: MotionEvent, axis: Int, history: Int) =
        if (history < 0) event.getAxisValue(axis) else event.getHistoricalAxisValue(axis, history)

    fun motion(event: MotionEvent, history: Int) {
        leftX = axis(event, MotionEvent.AXIS_X, history)
        leftY = axis(event, MotionEvent.AXIS_Y, history)
        rightX = axis(event, if (range(MotionEvent.AXIS_Z)) MotionEvent.AXIS_Z else MotionEvent.AXIS_RX, history)
        rightY = axis(event, if (range(MotionEvent.AXIS_RZ)) MotionEvent.AXIS_RZ else MotionEvent.AXIS_RY, history)
        lx = direction(leftX, lx); ly = direction(leftY, ly)
        rx = direction(rightX, rx); ry = direction(rightY, ry)
        if (hats) {
            val x = axis(event, MotionEvent.AXIS_HAT_X, history)
            val y = axis(event, MotionEvent.AXIS_HAT_Y, history)
            dpad = (if (x < -.5f) 1 else if (x > .5f) 2 else 0) or
                (if (y < -.5f) 4 else if (y > .5f) 8 else 0)
        }
        if (analogLeft) {
            l2 = axis(event, if (range(MotionEvent.AXIS_LTRIGGER)) MotionEvent.AXIS_LTRIGGER else MotionEvent.AXIS_BRAKE, history)
            buttons = trigger(buttons, L2, l2)
        }
        if (analogRight) {
            r2 = axis(event, if (range(MotionEvent.AXIS_RTRIGGER)) MotionEvent.AXIS_RTRIGGER else MotionEvent.AXIS_GAS, history)
            buttons = trigger(buttons, R2, r2)
        }
    }

    fun frame() = Frame(buttons, dpad, modifiers(lx, ly, true) or modifiers(rx, ry, false), menu, lx, ly, rx, ry)

    companion object {
        fun controller(device: InputDevice?) = device != null &&
            (device.supportsSource(InputDevice.SOURCE_GAMEPAD) || device.supportsSource(InputDevice.SOURCE_JOYSTICK))

        fun direction(value: Float, previous: Int): Int = when {
            value >= .55f -> 1
            value <= -.55f -> -1
            previous > 0 && value > .35f -> 1
            previous < 0 && value < -.35f -> -1
            else -> 0
        }

        fun trigger(buttons: Int, bit: Int, value: Float): Int {
            val active = if (buttons and bit != 0) value > .35f else value >= .65f
            return if (active) buttons or bit else buttons and bit.inv()
        }

        fun modifiers(x: Int, y: Int, left: Boolean): Int {
            if (if (left) x > 0 else x < 0) return Action.ALT or Action.META or
                (if (y > 0) 0 else Action.CTRL) or (if (y < 0) Action.SHIFT else 0)
            return (if (x != 0) Action.CTRL else 0) or (if (y < 0) Action.ALT else if (y > 0) Action.META else 0)
        }
    }
}
