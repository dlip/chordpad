package io.github.dlip.chordpad

import android.view.KeyEvent

class GamepadKeys {
    data class Result(val consumed: Boolean, val layerGesture: Boolean)
    private val held = mutableMapOf<Int, Boolean>()
    private var layers = 0
    private var latched = false

    fun update(code: Int, action: Int, repeat: Int, consume: Boolean): Result {
        val layer = when (code) {
            KeyEvent.KEYCODE_BUTTON_A -> 1
            KeyEvent.KEYCODE_BUTTON_B -> 2
            KeyEvent.KEYCODE_BUTTON_X -> 4
            KeyEvent.KEYCODE_BUTTON_Y -> 8
            else -> 0
        }
        var gesture = false
        val consumed = when (action) {
            KeyEvent.ACTION_DOWN -> {
                if (repeat == 0 && code !in held) {
                    held[code] = consume
                    layers = layers or layer
                    if (layers == 15 && !latched) {
                        gesture = true
                        latched = true
                    }
                }
                held[code] == true
            }
            KeyEvent.ACTION_UP -> {
                val routed = held.remove(code) == true
                layers = layers and layer.inv()
                if (layers == 0) latched = false
                routed
            }
            else -> false
        }
        return Result(consumed, gesture)
    }

    fun suppressHeldGesture() { if (layers != 0) latched = true }
    fun hasConsumedPresses() = held.containsValue(true)
    fun hasHeldPresses() = held.isNotEmpty()
    fun reset() { held.clear(); layers = 0; latched = false }
}
