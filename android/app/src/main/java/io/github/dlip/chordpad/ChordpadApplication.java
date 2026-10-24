package io.github.dlip.chordpad;

import android.app.Application;

public final class ChordpadApplication extends Application {
    private AppState state;
    synchronized AppState state() {
        if (state == null) state = new AppState(this);
        return state;
    }
}
