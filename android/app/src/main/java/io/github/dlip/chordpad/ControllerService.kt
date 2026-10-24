package io.github.dlip.chordpad

import android.accessibilityservice.AccessibilityButtonController
import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.AccessibilityServiceInfo
import android.accessibilityservice.InputMethod
import android.os.Handler
import android.os.Looper
import android.view.InputDevice
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.accessibility.AccessibilityEvent
import android.view.inputmethod.EditorInfo
import android.widget.Toast

class ControllerService : AccessibilityService() {
    private var state: AppState? = null
    private var previousKeyboardMode = -1
    private val buttonCallback = object : AccessibilityButtonController.AccessibilityButtonCallback() {
        override fun onClicked(controller: AccessibilityButtonController) {
            val state = state ?: return
            state.setEnabled(!state.enabled)
            val message = when {
                !state.enabled -> R.string.controller_input_off
                state.device == null -> R.string.controller_input_waiting
                else -> R.string.controller_input_on
            }
            Toast.makeText(this@ControllerService, message, Toast.LENGTH_SHORT).show()
        }
    }
    private val outputHandler = Handler(Looper.getMainLooper())
    private val lastAppSwitch = LastAppSwitch(
        { state?.let { it.enabled && !it.paused && it.device != null } == true },
        { performGlobalAction(GLOBAL_ACTION_RECENTS) },
        { task -> outputHandler.postDelayed(task, 150) }
    )

    override fun onServiceConnected() {
        val state = AppState.get(this)
        this.state = state
        state.service = this
        accessibilityButtonController.registerAccessibilityButtonCallback(buttonCallback)
        updateCapture()
        state.reset("Service connected; start remapping when ready")
    }

    fun updateCapture() {
        val state = state ?: return
        val info = serviceInfo ?: return
        val flags = info.flags or AccessibilityServiceInfo.FLAG_REQUEST_FILTER_KEY_EVENTS or AccessibilityServiceInfo.FLAG_INPUT_METHOD_EDITOR
        val sources = if (state.capturingMotion()) InputDevice.SOURCE_JOYSTICK else 0
        if (info.flags == flags && info.motionEventSources == sources) return
        info.flags = flags
        info.motionEventSources = sources
        serviceInfo = info
    }

    fun updateSoftKeyboard() {
        val state = state
        if (state != null && state.hideSoftKeyboard && state.enabled && !state.paused && state.device != null) {
            if (previousKeyboardMode != -1) return
            val keyboard = softKeyboardController
            val mode = keyboard.showMode
            if (mode != SHOW_MODE_HIDDEN && keyboard.setShowMode(SHOW_MODE_HIDDEN)) previousKeyboardMode = mode
        } else restoreSoftKeyboard()
    }

    private fun restoreSoftKeyboard() {
        if (previousKeyboardMode == -1) return
        val keyboard = softKeyboardController
        if (keyboard.showMode != SHOW_MODE_HIDDEN || keyboard.setShowMode(
                if (previousKeyboardMode == SHOW_MODE_IGNORE_HARD_KEYBOARD) SHOW_MODE_IGNORE_HARD_KEYBOARD else SHOW_MODE_AUTO)) {
            previousKeyboardMode = -1
        }
    }

    override fun onCreateInputMethod(): InputMethod {
        if (state == null) state = AppState.get(this)
        return object : InputMethod(this) {
            override fun onStartInput(info: EditorInfo, restarting: Boolean) {
                state?.let { it.editor = true; it.focusChanged() }
            }
            override fun onFinishInput() {
                state?.let { it.editor = false; it.focusChanged() }
            }
        }
    }

    override fun onKeyEvent(event: KeyEvent) = state?.key(event) == true
    override fun onMotionEvent(event: MotionEvent) { state?.motion(event) }
    override fun onAccessibilityEvent(event: AccessibilityEvent?) { state?.focusChanged() }
    override fun onInterrupt() { state?.setEnabled(false) }

    override fun onDestroy() {
        restoreSoftKeyboard()
        accessibilityButtonController.unregisterAccessibilityButtonCallback(buttonCallback)
        cancelLastAppSwitch()
        state?.let { it.service = null; it.editor = false; it.setEnabled(false) }
        super.onDestroy()
    }

    fun cancelLastAppSwitch() { lastAppSwitch.cancel(); outputHandler.removeCallbacksAndMessages(null) }

    fun send(action: Action): Boolean {
        val method = inputMethod
        val connection = method?.currentInputConnection
        val editorAvailable = connection != null && method.currentInputStarted
        val system = action.systemAction(editorAvailable)
        if (system == "last_app") return lastAppSwitch.start()
        if (system != null) {
            cancelLastAppSwitch()
            return performGlobalAction(when (system) {
                "back" -> GLOBAL_ACTION_BACK; "home" -> GLOBAL_ACTION_HOME; "recents" -> GLOBAL_ACTION_RECENTS
                "dpad_up" -> GLOBAL_ACTION_DPAD_UP; "dpad_down" -> GLOBAL_ACTION_DPAD_DOWN
                "dpad_left" -> GLOBAL_ACTION_DPAD_LEFT; "dpad_right" -> GLOBAL_ACTION_DPAD_RIGHT
                "dpad_center" -> GLOBAL_ACTION_DPAD_CENTER
                "screenshot" -> GLOBAL_ACTION_TAKE_SCREENSHOT
                else -> throw IllegalArgumentException("Unknown navigation action")
            })
        }
        if (action.consumerKey()) return ConsumerOutput.send(this, action)
        if (!editorAvailable) return false
        KeyboardOutput.send(connection, action)
        return true
    }
}
