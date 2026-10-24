package io.github.dlip.chordpad

import android.content.Context
import android.media.AudioManager
import android.os.SystemClock
import android.view.KeyEvent

object ConsumerOutput {
    fun send(context: Context, action: Action): Boolean {
        val audio = context.getSystemService(AudioManager::class.java) ?: return false
        when (action.value) {
            "volume_up" -> audio.adjustStreamVolume(AudioManager.STREAM_MUSIC, AudioManager.ADJUST_RAISE, AudioManager.FLAG_SHOW_UI)
            "volume_down" -> audio.adjustStreamVolume(AudioManager.STREAM_MUSIC, AudioManager.ADJUST_LOWER, AudioManager.FLAG_SHOW_UI)
            "mute" -> audio.adjustStreamVolume(AudioManager.STREAM_MUSIC, AudioManager.ADJUST_TOGGLE_MUTE, AudioManager.FLAG_SHOW_UI)
            else -> {
                val key = KeyboardOutput.keyCode(action.value)
                val now = SystemClock.uptimeMillis()
                try { audio.dispatchMediaKeyEvent(KeyEvent(now, now, KeyEvent.ACTION_DOWN, key, 0)) }
                finally { audio.dispatchMediaKeyEvent(KeyEvent(now, SystemClock.uptimeMillis(), KeyEvent.ACTION_UP, key, 0)) }
            }
        }
        return true
    }
}
