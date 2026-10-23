package io.github.dlip.padtype;

import android.accessibilityservice.AccessibilityService;
import android.accessibilityservice.AccessibilityServiceInfo;
import android.accessibilityservice.InputMethod;
import android.os.Handler;
import android.os.Looper;
import android.view.InputDevice;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.accessibility.AccessibilityEvent;
import android.view.inputmethod.EditorInfo;

public final class ControllerService extends AccessibilityService {
    private AppState state;
    private final Handler outputHandler = new Handler(Looper.getMainLooper());
    private final LastAppSwitch lastAppSwitch = new LastAppSwitch(
            () -> state != null && state.enabled && !state.paused && state.device != null,
            () -> performGlobalAction(GLOBAL_ACTION_RECENTS),
            task -> outputHandler.postDelayed(task,150));
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
