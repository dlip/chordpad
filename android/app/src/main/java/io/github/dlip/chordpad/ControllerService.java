package io.github.dlip.chordpad;

import android.accessibilityservice.AccessibilityService;
import android.accessibilityservice.AccessibilityServiceInfo;
import android.accessibilityservice.AccessibilityButtonController;
import android.accessibilityservice.InputMethod;
import android.os.Handler;
import android.os.Looper;
import android.view.InputDevice;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.accessibility.AccessibilityEvent;
import android.view.inputmethod.EditorInfo;
import android.widget.Toast;

public final class ControllerService extends AccessibilityService {
    private AppState state;
    private int previousKeyboardMode = -1;
    private final AccessibilityButtonController.AccessibilityButtonCallback buttonCallback = new AccessibilityButtonController.AccessibilityButtonCallback() {
        @Override public void onClicked(AccessibilityButtonController controller) {
            if (state == null) return;
            state.setEnabled(!state.enabled);
            int message = !state.enabled ? R.string.controller_input_off
                    : state.device == null ? R.string.controller_input_waiting : R.string.controller_input_on;
            Toast.makeText(ControllerService.this,message,Toast.LENGTH_SHORT).show();
        }
    };
    private final Handler outputHandler = new Handler(Looper.getMainLooper());
    private final LastAppSwitch lastAppSwitch = new LastAppSwitch(
            () -> state != null && state.enabled && !state.paused && state.device != null,
            () -> performGlobalAction(GLOBAL_ACTION_RECENTS),
            task -> outputHandler.postDelayed(task,150));
    @Override protected void onServiceConnected() {
        state = AppState.get(this); state.service = this;
        getAccessibilityButtonController().registerAccessibilityButtonCallback(buttonCallback);
        updateCapture(); state.reset("Service connected; start remapping when ready");
    }
    public void updateCapture() {
        AccessibilityServiceInfo info = getServiceInfo();
        if (info == null) return;
        int flags = info.flags | AccessibilityServiceInfo.FLAG_REQUEST_FILTER_KEY_EVENTS | AccessibilityServiceInfo.FLAG_INPUT_METHOD_EDITOR;
        int sources = state.capturingMotion() ? InputDevice.SOURCE_JOYSTICK : 0;
        if (info.flags == flags && info.getMotionEventSources() == sources) return;
        info.flags = flags; info.setMotionEventSources(sources);
        setServiceInfo(info);
    }
    public void updateSoftKeyboard() {
        if (state != null && state.hideSoftKeyboard && state.enabled && !state.paused && state.device != null) {
            if (previousKeyboardMode != -1) return;
            SoftKeyboardController keyboard = getSoftKeyboardController();
            int mode = keyboard.getShowMode();
            if (mode != SHOW_MODE_HIDDEN && keyboard.setShowMode(SHOW_MODE_HIDDEN)) previousKeyboardMode = mode;
        } else restoreSoftKeyboard();
    }
    private void restoreSoftKeyboard() {
        if (previousKeyboardMode == -1) return;
        SoftKeyboardController keyboard = getSoftKeyboardController();
        if (keyboard.getShowMode() != SHOW_MODE_HIDDEN || keyboard.setShowMode(
                previousKeyboardMode == SHOW_MODE_IGNORE_HARD_KEYBOARD ? SHOW_MODE_IGNORE_HARD_KEYBOARD : SHOW_MODE_AUTO)) previousKeyboardMode = -1;
    }
    @Override public InputMethod onCreateInputMethod() {
        if (state == null) state = AppState.get(this);
        return new InputMethod(this) {
            @Override public void onStartInput(EditorInfo info, boolean restarting) {
                if (state == null) return;
                state.editor = true; state.focusChanged();
            }
            @Override public void onFinishInput() {
                if (state == null) return;
                state.editor = false; state.focusChanged();
            }
        };
    }
    @Override protected boolean onKeyEvent(KeyEvent event) { return state != null && state.key(event); }
    @Override public void onMotionEvent(MotionEvent event) { if (state != null) state.motion(event); }
    @Override public void onAccessibilityEvent(AccessibilityEvent event) {
        if (state != null) state.focusChanged();
    }
    @Override public void onInterrupt() { if (state != null) state.setEnabled(false); }
    @Override public void onDestroy() {
        restoreSoftKeyboard();
        getAccessibilityButtonController().unregisterAccessibilityButtonCallback(buttonCallback);
        cancelLastAppSwitch();
        if (state != null) { state.service = null; state.editor = false; state.setEnabled(false); }
        super.onDestroy();
    }
    public void cancelLastAppSwitch() {
        lastAppSwitch.cancel(); outputHandler.removeCallbacksAndMessages(null);
    }
    public boolean send(Action action) {
        InputMethod method = getInputMethod();
        InputMethod.AccessibilityInputConnection connection = method == null ? null : method.getCurrentInputConnection();
        boolean editorAvailable = connection != null && method.getCurrentInputStarted();
        String system = action.systemAction(editorAvailable);
        if ("last_app".equals(system)) return lastAppSwitch.start();
        if (system != null) {
            cancelLastAppSwitch();
            return performGlobalAction(switch (system) {
                case "back" -> GLOBAL_ACTION_BACK; case "home" -> GLOBAL_ACTION_HOME; case "recents" -> GLOBAL_ACTION_RECENTS;
                case "dpad_up" -> GLOBAL_ACTION_DPAD_UP; case "dpad_down" -> GLOBAL_ACTION_DPAD_DOWN;
                case "dpad_left" -> GLOBAL_ACTION_DPAD_LEFT; case "dpad_right" -> GLOBAL_ACTION_DPAD_RIGHT;
                case "dpad_center" -> GLOBAL_ACTION_DPAD_CENTER;
                default -> throw new IllegalArgumentException("Unknown navigation action");
            });
        }
        if (action.consumerKey()) return ConsumerOutput.send(this,action);
        if (!editorAvailable) return false;
        KeyboardOutput.send(connection, action); return true;
    }
}
