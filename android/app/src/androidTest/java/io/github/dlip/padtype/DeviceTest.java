package io.github.dlip.padtype;

import android.app.UiAutomation;
import android.content.Intent;
import android.os.SystemClock;
import android.test.InstrumentationTestCase;
import android.view.InputDevice;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.accessibility.AccessibilityNodeInfo;
import org.json.JSONArray;
import java.nio.charset.StandardCharsets;
import java.util.function.BooleanSupplier;

@SuppressWarnings("deprecation")
public class DeviceTest extends InstrumentationTestCase {
    private int deviceId() {
        for (int id : InputDevice.getDeviceIds()) {
            InputDevice device = InputDevice.getDevice(id);
            if (ControllerInput.controller(device) && device.getName().contains("DualSense")) return id;
        }
        throw new AssertionError("Connect the DualSense before running device checks");
    }
    private MotionEvent motion(float leftTrigger,float rightTrigger,float rightX) {
        MotionEvent.PointerProperties pointer = new MotionEvent.PointerProperties(); pointer.id = 0;
        MotionEvent.PointerCoords coordinate = new MotionEvent.PointerCoords();
        coordinate.setAxisValue(MotionEvent.AXIS_LTRIGGER,leftTrigger);
        coordinate.setAxisValue(MotionEvent.AXIS_RTRIGGER,rightTrigger);
        coordinate.setAxisValue(MotionEvent.AXIS_Z,rightX);
        long time = SystemClock.uptimeMillis();
        return MotionEvent.obtain(time,time,MotionEvent.ACTION_MOVE,1,new MotionEvent.PointerProperties[]{pointer},new MotionEvent.PointerCoords[]{coordinate},0,0,1,1,deviceId(),0,InputDevice.SOURCE_JOYSTICK,0);
    }
    public void testRecognitionVectorsOnAndroid() throws Exception {
        String layout;
        try (var stream = getInstrumentation().getTargetContext().getAssets().open("default-layout.json")) { layout = new String(stream.readAllBytes(),StandardCharsets.UTF_8); }
        try (var stream = getInstrumentation().getContext().getAssets().open("chord-vectors.json")) {
            assertEquals(341,RecognitionVectors.check(layout,new JSONArray(new String(stream.readAllBytes(),StandardCharsets.UTF_8))));
        }
    }
    public void testConsumerKeyCodes() {
        String[] keys = {"media_play_pause","media_next","media_previous","media_stop","volume_up","volume_down","mute"};
        int[] codes = {KeyEvent.KEYCODE_MEDIA_PLAY_PAUSE,KeyEvent.KEYCODE_MEDIA_NEXT,KeyEvent.KEYCODE_MEDIA_PREVIOUS,KeyEvent.KEYCODE_MEDIA_STOP,KeyEvent.KEYCODE_VOLUME_UP,KeyEvent.KEYCODE_VOLUME_DOWN,KeyEvent.KEYCODE_VOLUME_MUTE};
        for (int i = 0; i < keys.length; i++) {
            assertTrue(Action.key(keys[i]).consumerKey());
            assertEquals(codes[i],KeyboardOutput.keyCode(keys[i]));
            KeyboardOutput.validate(Action.key(keys[i]));
        }
        assertFalse(Action.key("f1").consumerKey());
        assertEquals(android.content.pm.PackageManager.PERMISSION_GRANTED,getInstrumentation().getTargetContext().checkSelfPermission(android.Manifest.permission.MODIFY_AUDIO_SETTINGS));
    }
    public void testDualSenseAnalogAndButtonNormalization() {
        ControllerInput input = new ControllerInput(InputDevice.getDevice(deviceId()));
        input.key(new KeyEvent(0,0,KeyEvent.ACTION_DOWN,KeyEvent.KEYCODE_BUTTON_A,0,0,deviceId(),0,0,InputDevice.SOURCE_GAMEPAD));
        assertEquals(1,input.frame().buttons());
        MotionEvent event = motion(.8f,.8f,-.8f);
        input.motion(event,-1); event.recycle();
        assertEquals(1|64|128,input.frame().buttons());
        assertEquals(Action.ALT|Action.META,input.frame().modifiers());
        input.key(new KeyEvent(0,0,KeyEvent.ACTION_UP,KeyEvent.KEYCODE_BUTTON_L2,0,0,deviceId(),0,0,InputDevice.SOURCE_GAMEPAD));
        assertTrue((input.frame().buttons()&64)!=0);
        event = motion(.4f,.4f,-.4f); input.motion(event,-1); event.recycle();
        assertEquals(1|64|128,input.frame().buttons());
        event = motion(.35f,.35f,-.35f); input.motion(event,-1); event.recycle();
        assertEquals(1,input.frame().buttons()); assertEquals(0,input.frame().modifiers());
    }
    private void waitFor(BooleanSupplier condition,String message) {
        long until = SystemClock.uptimeMillis()+5000;
        while (SystemClock.uptimeMillis()<until) {
            if (condition.getAsBoolean()) return;
            SystemClock.sleep(40);
        }
        fail(message);
    }
    private String editorText(UiAutomation automation) {
        AccessibilityNodeInfo root = automation.getRootInActiveWindow();
        if (root == null || !"io.github.dlip.padtype.test".contentEquals(root.getPackageName())) return "<not test editor>";
        AccessibilityNodeInfo editor = root.findFocus(AccessibilityNodeInfo.FOCUS_INPUT);
        return editor == null ? "<no focus>" : String.valueOf(editor.getText() == null ? "" : editor.getText());
    }
    private void press(UiAutomation automation,int code) {
        long time = SystemClock.uptimeMillis();
        for (int action : new int[]{KeyEvent.ACTION_DOWN,KeyEvent.ACTION_UP}) {
            assertTrue(automation.injectInputEvent(new KeyEvent(time,SystemClock.uptimeMillis(),action,code,0,0,deviceId(),0,0,InputDevice.SOURCE_GAMEPAD),true));
        }
    }
    public void testTypingIntoSeparateApp() throws Exception {
        UiAutomation automation = getInstrumentation().getUiAutomation(UiAutomation.FLAG_DONT_SUPPRESS_ACCESSIBILITY_SERVICES);
        AppState state = AppState.get(getInstrumentation().getTargetContext());
        waitFor(() -> state.service != null,"Enable padtype controller input in accessibility settings first");
        String oldMode = state.mode;
        Action oldMapping = state.direct.mappings.get("Right stick Right");
        try {
            Intent editor = new Intent().setClassName("io.github.dlip.padtype.test","io.github.dlip.padtype.TestEditorActivity").addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TASK);
            getInstrumentation().getContext().startActivity(editor);
            waitFor(() -> editorText(automation).equals(""),"Test editor did not become focused");
            getInstrumentation().runOnMainSync(() -> { state.setMode("chords"); state.setEnabled(true); });
            MotionEvent neutral = motion(0,0,0); automation.injectInputEvent(neutral,true); neutral.recycle();
            press(automation,KeyEvent.KEYCODE_BUTTON_A);
            waitFor(() -> editorText(automation).equals("i"),"Cross did not type i; editor="+editorText(automation));
            MotionEvent shift = motion(1,0,0); automation.injectInputEvent(shift,true); shift.recycle();
            press(automation,KeyEvent.KEYCODE_BUTTON_A);
            neutral = motion(0,0,0); automation.injectInputEvent(neutral,true); neutral.recycle();
            waitFor(() -> editorText(automation).equals("iI"),"L2+Cross did not type uppercase I");
            press(automation,KeyEvent.KEYCODE_BUTTON_L1);
            press(automation,KeyEvent.KEYCODE_BUTTON_R1);
            press(automation,KeyEvent.KEYCODE_BUTTON_START);
            waitFor(() -> editorText(automation).equals("i \n"),"Backspace/Space/Options did not reach the other app");
            getInstrumentation().runOnMainSync(() -> {
                state.setMode("direct");
                try { state.map("Right stick Right",Action.text("z")); } catch (Exception error) { throw new AssertionError(error); }
            });
            MotionEvent right = motion(0,0,1); automation.injectInputEvent(right,true); right.recycle();
            neutral = motion(0,0,0); automation.injectInputEvent(neutral,true); neutral.recycle();
            waitFor(() -> editorText(automation).equals("i \nz"),"Remapped right stick did not type z");
        } finally {
            getInstrumentation().runOnMainSync(() -> {
                state.setEnabled(false); state.setMode(oldMode);
                try { state.map("Right stick Right",oldMapping); } catch (Exception error) { throw new AssertionError(error); }
            });
        }
    }
}
