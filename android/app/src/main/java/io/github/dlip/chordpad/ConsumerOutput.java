package io.github.dlip.chordpad;

import android.content.Context;
import android.media.AudioManager;
import android.os.SystemClock;
import android.view.KeyEvent;

final class ConsumerOutput {
    private ConsumerOutput() {}

    static boolean send(Context context, Action action) {
        AudioManager audio = context.getSystemService(AudioManager.class);
        if (audio == null) return false;
        switch (action.value()) {
            case "volume_up" -> audio.adjustStreamVolume(AudioManager.STREAM_MUSIC,AudioManager.ADJUST_RAISE,AudioManager.FLAG_SHOW_UI);
            case "volume_down" -> audio.adjustStreamVolume(AudioManager.STREAM_MUSIC,AudioManager.ADJUST_LOWER,AudioManager.FLAG_SHOW_UI);
            case "mute" -> audio.adjustStreamVolume(AudioManager.STREAM_MUSIC,AudioManager.ADJUST_TOGGLE_MUTE,AudioManager.FLAG_SHOW_UI);
            default -> {
                int key = KeyboardOutput.keyCode(action.value());
                long now = SystemClock.uptimeMillis();
                try { audio.dispatchMediaKeyEvent(new KeyEvent(now,now,KeyEvent.ACTION_DOWN,key,0)); }
                finally { audio.dispatchMediaKeyEvent(new KeyEvent(now,SystemClock.uptimeMillis(),KeyEvent.ACTION_UP,key,0)); }
            }
        }
        return true;
    }
}
