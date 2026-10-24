package io.github.dlip.chordpad

import android.app.Activity
import android.content.Intent
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import android.media.session.MediaSession
import android.media.session.PlaybackState
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.KeyEvent
import android.widget.TextView

@Suppress("DEPRECATION")
class MediaTestActivity : Activity() {
    private var session: MediaSession? = null
    private var track: AudioTrack? = null
    private lateinit var output: TextView
    private val events = StringBuilder()

    override fun onCreate(bundle: Bundle?) {
        super.onCreate(bundle)
        if (intent.getBooleanExtra("finish", false)) { finish(); return }
        output = TextView(this).apply {
            textSize = 24f; setPadding(32, 32, 32, 32)
            text = "chordpad media output test"; contentDescription = "chordpad media output test"
        }
        setContentView(output)
        session = MediaSession(this, "chordpad output test").apply {
            setFlags(MediaSession.FLAG_HANDLES_MEDIA_BUTTONS or MediaSession.FLAG_HANDLES_TRANSPORT_CONTROLS)
            setCallback(object : MediaSession.Callback() {
                override fun onMediaButtonEvent(intent: Intent): Boolean {
                    intent.getParcelableExtra(Intent.EXTRA_KEY_EVENT, KeyEvent::class.java)?.let { event ->
                        val name = "${KeyEvent.keyCodeToString(event.keyCode)}:${event.action}"
                        if (events.isNotEmpty()) events.append(",")
                        events.append(name); output.contentDescription = events.toString(); output.text = events.toString()
                        Log.i("ChordpadMediaTest", name)
                    }
                    return true
                }
            }, Handler(Looper.getMainLooper()))
            setPlaybackState(PlaybackState.Builder().setActions(PlaybackState.ACTION_PLAY_PAUSE or PlaybackState.ACTION_SKIP_TO_NEXT or PlaybackState.ACTION_SKIP_TO_PREVIOUS or PlaybackState.ACTION_STOP)
                .setState(PlaybackState.STATE_PLAYING, 0, 1f).build())
            isActive = true
        }
        track = AudioTrack.Builder().setAudioAttributes(AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_MEDIA).build())
            .setAudioFormat(AudioFormat.Builder().setSampleRate(8000).setEncoding(AudioFormat.ENCODING_PCM_16BIT).setChannelMask(AudioFormat.CHANNEL_OUT_MONO).build())
            .setBufferSizeInBytes(16000).setTransferMode(AudioTrack.MODE_STATIC).build().apply {
                write(ByteArray(16000), 0, 16000); setLoopPoints(0, 8000, -1); setVolume(0f); play()
            }
    }
    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        if (intent.getBooleanExtra("finish", false)) finish()
    }
    override fun onDestroy() {
        track?.let { it.stop(); it.release() }
        session?.release()
        super.onDestroy()
    }
}
