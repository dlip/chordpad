package io.github.dlip.chordpad

import android.accessibilityservice.InputMethod.AccessibilityInputConnection
import android.os.SystemClock
import android.view.InputDevice
import android.view.KeyCharacterMap
import android.view.KeyEvent
import java.util.Locale

object KeyboardOutput {
    fun keyCode(key: String): Int = when (key) {
        "backspace" -> KeyEvent.KEYCODE_DEL
        "delete" -> KeyEvent.KEYCODE_FORWARD_DEL
        "up" -> KeyEvent.KEYCODE_DPAD_UP
        "down" -> KeyEvent.KEYCODE_DPAD_DOWN
        "left" -> KeyEvent.KEYCODE_DPAD_LEFT
        "right" -> KeyEvent.KEYCODE_DPAD_RIGHT
        "home" -> KeyEvent.KEYCODE_MOVE_HOME
        "end" -> KeyEvent.KEYCODE_MOVE_END
        "mute" -> KeyEvent.KEYCODE_VOLUME_MUTE
        else -> KeyEvent.keyCodeFromString("KEYCODE_${key.uppercase(Locale.ROOT)}")
    }

    fun validate(action: Action) {
        require(action.modifiers and 15.inv() == 0) { "Invalid keyboard modifiers" }
        when (action.type) {
            "character" -> require(action.value.isNotEmpty()) { "Enter some text" }
            "key" -> require(keyCode(action.value) != KeyEvent.KEYCODE_UNKNOWN) { "Unknown key: ${action.value}" }
            "modifier" -> Action.modifier(action.value)
            "system" -> require(action.value in listOf("back", "home", "recents")) { "Unknown navigation action" }
            else -> throw IllegalArgumentException("Unsupported action")
        }
    }

    fun send(connection: AccessibilityInputConnection, action: Action) {
        if (action.type == "character" && action.modifiers == 0 && action.value != "\n" && action.value != "\t") {
            connection.commitText(action.value, 1, null)
            return
        }
        var modifiers = action.modifiers
        val code: Int
        if (action.type == "character") {
            require(action.value.length == 1) { "Modified text must be one ASCII character" }
            var character = action.value[0]
            code = when (character) {
                '\n' -> KeyEvent.KEYCODE_ENTER
                '\t' -> KeyEvent.KEYCODE_TAB
                else -> {
                    val lower = "abcdefghijklmnopqrstuvwxyz`1234567890-=[]\\;',./ "
                    val upper = "ABCDEFGHIJKLMNOPQRSTUVWXYZ~!@#\$%^&*()_+{}|:\"<>? "
                    val index = upper.indexOf(character)
                    if (index >= 0 && character != ' ') {
                        character = lower[index]
                        modifiers = modifiers or Action.SHIFT
                    }
                    val keys = KeyCharacterMap.load(KeyCharacterMap.VIRTUAL_KEYBOARD).getEvents(charArrayOf(character))
                    keys?.firstOrNull { it.action == KeyEvent.ACTION_DOWN && !KeyEvent.isModifierKey(it.keyCode) }?.keyCode
                        ?: throw IllegalArgumentException("This character has no keyboard shortcut key")
                }
            }
        } else code = keyCode(action.value)
        require(code != KeyEvent.KEYCODE_UNKNOWN) { "Unsupported keyboard key" }
        val bits = intArrayOf(Action.CTRL, Action.ALT, Action.SHIFT, Action.META)
        val codes = intArrayOf(KeyEvent.KEYCODE_CTRL_LEFT, KeyEvent.KEYCODE_ALT_LEFT, KeyEvent.KEYCODE_SHIFT_LEFT, KeyEvent.KEYCODE_META_LEFT)
        val states = intArrayOf(KeyEvent.META_CTRL_ON or KeyEvent.META_CTRL_LEFT_ON,
            KeyEvent.META_ALT_ON or KeyEvent.META_ALT_LEFT_ON, KeyEvent.META_SHIFT_ON or KeyEvent.META_SHIFT_LEFT_ON,
            KeyEvent.META_META_ON or KeyEvent.META_META_LEFT_ON)
        val pressed = mutableListOf<Int>()
        var meta = 0
        val time = SystemClock.uptimeMillis()
        try {
            for (i in bits.indices) if (modifiers and bits[i] != 0) {
                meta = meta or states[i]
                pressed.add(codes[i])
                connection.sendKeyEvent(event(time, KeyEvent.ACTION_DOWN, codes[i], meta))
            }
            pressed.add(code)
            connection.sendKeyEvent(event(time, KeyEvent.ACTION_DOWN, code, meta))
        } finally {
            for (key in pressed.asReversed()) {
                for (j in codes.indices) if (codes[j] == key) meta = meta and states[j].inv()
                connection.sendKeyEvent(event(time, KeyEvent.ACTION_UP, key, meta))
            }
        }
    }

    private fun event(downTime: Long, action: Int, code: Int, meta: Int) = KeyEvent(
        downTime, SystemClock.uptimeMillis(), action, code, 0, meta, KeyCharacterMap.VIRTUAL_KEYBOARD,
        0, KeyEvent.FLAG_SOFT_KEYBOARD or KeyEvent.FLAG_KEEP_TOUCH_MODE, InputDevice.SOURCE_KEYBOARD
    )
}
