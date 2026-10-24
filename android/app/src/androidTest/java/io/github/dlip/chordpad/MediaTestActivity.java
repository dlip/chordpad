package io.github.dlip.chordpad;

import android.app.Activity;
import android.content.Intent;
import android.media.AudioAttributes;
import android.media.AudioFormat;
import android.media.AudioTrack;
import android.media.session.MediaSession;
import android.media.session.PlaybackState;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;
import android.view.KeyEvent;
import android.widget.TextView;

@SuppressWarnings("deprecation")
public class MediaTestActivity extends Activity {
    private MediaSession session;
    private AudioTrack track;
    private TextView output;
    private final StringBuilder events = new StringBuilder();

    @Override public void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        if (getIntent().getBooleanExtra("finish",false)) { finish(); return; }
        output = new TextView(this); output.setTextSize(24); output.setPadding(32,32,32,32);
        output.setText("chordpad media output test"); output.setContentDescription("chordpad media output test"); setContentView(output);
        session = new MediaSession(this,"chordpad output test");
        session.setFlags(MediaSession.FLAG_HANDLES_MEDIA_BUTTONS | MediaSession.FLAG_HANDLES_TRANSPORT_CONTROLS);
        session.setCallback(new MediaSession.Callback() {
            @Override public boolean onMediaButtonEvent(Intent intent) {
                KeyEvent event = intent.getParcelableExtra(Intent.EXTRA_KEY_EVENT,KeyEvent.class);
                if (event != null) {
                    String name = KeyEvent.keyCodeToString(event.getKeyCode()) + ":" + event.getAction();
                    if (events.length() != 0) events.append(",");
                    events.append(name); output.setContentDescription(events.toString()); output.setText(events.toString());
                    Log.i("ChordpadMediaTest",name);
                }
                return true;
            }
        },new Handler(Looper.getMainLooper()));
        session.setPlaybackState(new PlaybackState.Builder().setActions(PlaybackState.ACTION_PLAY_PAUSE | PlaybackState.ACTION_SKIP_TO_NEXT | PlaybackState.ACTION_SKIP_TO_PREVIOUS | PlaybackState.ACTION_STOP)
                .setState(PlaybackState.STATE_PLAYING,0,1).build());
        session.setActive(true);
        track = new AudioTrack.Builder().setAudioAttributes(new AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_MEDIA).build())
                .setAudioFormat(new AudioFormat.Builder().setSampleRate(8000).setEncoding(AudioFormat.ENCODING_PCM_16BIT).setChannelMask(AudioFormat.CHANNEL_OUT_MONO).build())
                .setBufferSizeInBytes(16000).setTransferMode(AudioTrack.MODE_STATIC).build();
        track.write(new byte[16000],0,16000); track.setLoopPoints(0,8000,-1); track.setVolume(0); track.play();
    }
    @Override protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        if (intent.getBooleanExtra("finish",false)) finish();
    }
    @Override protected void onDestroy() {
        if (track != null) { track.stop(); track.release(); }
        if (session != null) session.release();
        super.onDestroy();
    }
}
