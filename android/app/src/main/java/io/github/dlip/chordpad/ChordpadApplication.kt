package io.github.dlip.chordpad

import android.app.Application

class ChordpadApplication : Application() {
    private val appState by lazy { AppState(this) }
    fun state() = appState
}
