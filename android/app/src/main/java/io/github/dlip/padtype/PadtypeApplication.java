package io.github.dlip.padtype;

import android.app.Application;

public final class PadtypeApplication extends Application {
    private AppState state;
    synchronized AppState state() {
        if (state == null) state = new AppState(this);
        return state;
    }
}
