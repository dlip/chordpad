package io.github.dlip.padtype;

import android.accessibilityservice.AccessibilityService;
import android.accessibilityservice.AccessibilityServiceInfo;
import android.accessibilityservice.InputMethod;
import android.view.InputDevice;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.accessibility.AccessibilityEvent;
import android.view.inputmethod.EditorInfo;

public final class ControllerService extends AccessibilityService {
    private AppState state;
    @Override protected void onServiceConnected() {
        state = AppState.get(this); state.service = this;
        updateCapture(); state.reset("Service connected; start remapping when ready");
    }
    public void updateCapture() {
        AccessibilityServiceInfo info = getServiceInfo();
        if (info == null) return;
        info.flags |= AccessibilityServiceInfo.FLAG_REQUEST_FILTER_KEY_EVENTS | AccessibilityServiceInfo.FLAG_INPUT_METHOD_EDITOR;
        info.setMotionEventSources(state.enabled ? InputDevice.SOURCE_JOYSTICK : 0);
        setServiceInfo(info);
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
        if (state != null) { state.service = null; state.editor = false; state.setEnabled(false); }
        super.onDestroy();
    }
    public boolean send(Action action) {
        if (action.type().equals("system")) return performGlobalAction(switch (action.value()) {
            case "back" -> GLOBAL_ACTION_BACK; case "home" -> GLOBAL_ACTION_HOME; case "recents" -> GLOBAL_ACTION_RECENTS;
            default -> throw new IllegalArgumentException("Unknown navigation action");
        });
        InputMethod method = getInputMethod();
        InputMethod.AccessibilityInputConnection connection = method == null ? null : method.getCurrentInputConnection();
        if (connection == null || !method.getCurrentInputStarted()) return false;
        KeyboardOutput.send(connection, action); return true;
    }
}
