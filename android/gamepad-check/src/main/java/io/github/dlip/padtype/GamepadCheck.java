package io.github.dlip.padtype;

import android.app.Activity;
import android.app.Instrumentation;
import android.app.UiAutomation;
import android.accessibilityservice.AccessibilityServiceInfo;
import android.content.Intent;
import android.os.Bundle;
import android.os.ParcelFileDescriptor;
import android.os.SystemClock;
import android.view.InputDevice;
import android.view.KeyEvent;
import android.view.accessibility.AccessibilityNodeInfo;
import java.io.PrintWriter;
import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.io.StringWriter;
import java.util.ArrayList;
import java.util.List;
import java.util.function.BooleanSupplier;

public final class GamepadCheck extends Instrumentation {
    private UiAutomation automation;
    private VirtualGamepad pad;
    private GamepadTestActivity game;
    private static final int[] LAYERS = {KeyEvent.KEYCODE_BUTTON_L1,KeyEvent.KEYCODE_BUTTON_R1,KeyEvent.KEYCODE_BUTTON_L2,KeyEvent.KEYCODE_BUTTON_R2};

    @Override public void onCreate(Bundle arguments) { super.onCreate(arguments); start(); }
    @Override public void onStart() {
        Bundle result = new Bundle();
        try {
            checkPassthrough(); result.putString("stream","Gamepad passthrough and four-button resume passed.\n");
            finish(Activity.RESULT_OK,result);
        } catch (Throwable error) {
            StringWriter trace = new StringWriter(); error.printStackTrace(new PrintWriter(trace));
            result.putString("stream",trace.toString()); finish(Activity.RESULT_CANCELED,result);
        }
    }
    private void require(boolean condition,String message) { if (!condition) throw new AssertionError(message); }
    private void waitFor(BooleanSupplier condition,String message) {
        long end = SystemClock.uptimeMillis()+5000;
        while (SystemClock.uptimeMillis()<end) { if (condition.getAsBoolean()) return; SystemClock.sleep(40); }
        throw new AssertionError(message);
    }
    private AccessibilityNodeInfo startButton() {
        AccessibilityNodeInfo root = automation.getRootInActiveWindow();
        if (root == null || !"io.github.dlip.padtype".contentEquals(root.getPackageName())) return null;
        var nodes = root.findAccessibilityNodeInfosByViewId("io.github.dlip.padtype:id/start_remapping");
        return nodes.isEmpty() ? null : nodes.get(0);
    }
    private void openPadtype() {
        try (var reader = new BufferedReader(new InputStreamReader(new ParcelFileDescriptor.AutoCloseInputStream(
                automation.executeShellCommand("am start -f 0x10008000 -n io.github.dlip.padtype/.MainActivity"))))) {
            while (reader.readLine() != null) {}
        } catch (java.io.IOException error) { throw new IllegalStateException(error); }
        waitFor(() -> startButton() != null,"padtype did not open");
    }
    private void chooseController(String name) {
        click("Choose controller"); click(name); idle();
    }
    private void click(String text) {
        waitFor(() -> {
            var root = automation.getRootInActiveWindow();
            if (root != null) for (var node : root.findAccessibilityNodeInfosByText(text)) {
                if (text.contentEquals(node.getText()) && node.performAction(AccessibilityNodeInfo.ACTION_CLICK)) return true;
            }
            return false;
        },"Button not found: "+text);
    }
    private void key(int code,int action) throws Exception { pad.key(code,action); SystemClock.sleep(20); }
    private void tap(int code) throws Exception { key(code,KeyEvent.ACTION_DOWN); key(code,KeyEvent.ACTION_UP); }
    private void motion(float x,float y,float l2,float r2,float hat) throws Exception {
        pad.motion(x,y,l2,r2,hat); SystemClock.sleep(20);
    }
    private List<String> keys() {
        List<String> copy = new ArrayList<>(); runOnMainSync(() -> copy.addAll(game.keys)); return copy;
    }
    private void idle() { SystemClock.sleep(200); waitForIdleSync(); }
    private void checkPassthrough() throws Exception {
        automation = getUiAutomation(UiAutomation.FLAG_DONT_SUPPRESS_ACCESSIBILITY_SERVICES);
        var info = automation.getServiceInfo(); info.flags |= AccessibilityServiceInfo.FLAG_REPORT_VIEW_IDS; automation.setServiceInfo(info);
        openPadtype();
        String original = null;
        for (int id : InputDevice.getDeviceIds()) {
            InputDevice device = InputDevice.getDevice(id);
            if (device.supportsSource(InputDevice.SOURCE_GAMEPAD) && !automation.getRootInActiveWindow().findAccessibilityNodeInfosByText(device.getName()).isEmpty()) original = device.getName();
        }
        if (original == null) {
            for (int id : InputDevice.getDeviceIds()) {
                InputDevice device = InputDevice.getDevice(id);
                if (device.supportsSource(InputDevice.SOURCE_GAMEPAD) && !VirtualGamepad.NAME.equals(device.getName())) original = device.getName();
            }
            require(original != null,"Connect and select a controller before running this check");
            chooseController(original);
        }
        Throwable failure = null;
        try {
            if ("Stop remapping".contentEquals(startButton().getText())) { startButton().performAction(AccessibilityNodeInfo.ACTION_CLICK); idle(); }
            pad = new VirtualGamepad(automation);
            waitFor(() -> {
                for (int id : InputDevice.getDeviceIds()) if (VirtualGamepad.NAME.equals(InputDevice.getDevice(id).getName())) return true;
                return false;
            },"uinput did not create the test gamepad");
            chooseController(VirtualGamepad.NAME);
            require(startButton().performAction(AccessibilityNodeInfo.ACTION_CLICK),"Could not start remapping");
            waitFor(() -> startButton() != null && "Stop remapping".contentEquals(startButton().getText()),"Enable padtype's accessibility service first");
            game = (GamepadTestActivity)startActivitySync(new Intent(getTargetContext(),GamepadTestActivity.class).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)); idle();
            motion(0,0,0,0,0); idle();
            tap(KeyEvent.KEYCODE_BUTTON_A); idle();
            require(keys().isEmpty(),"Typing mode leaked gamepad buttons to the game");
            for (int code : LAYERS) key(code,KeyEvent.ACTION_DOWN);
            motion(0,0,1,1,0); idle();
            for (int code : LAYERS) key(code,KeyEvent.ACTION_UP);
            motion(0,0,0,0,0); idle();
            require(keys().isEmpty(),"Pausing leaked unmatched layer-button releases: " + keys());
            int[] buttons = {KeyEvent.KEYCODE_BUTTON_A,KeyEvent.KEYCODE_BUTTON_B,KeyEvent.KEYCODE_BUTTON_X,KeyEvent.KEYCODE_BUTTON_Y,
                    KeyEvent.KEYCODE_BUTTON_L1,KeyEvent.KEYCODE_BUTTON_R1,KeyEvent.KEYCODE_BUTTON_L2,KeyEvent.KEYCODE_BUTTON_R2};
            List<String> expected = new ArrayList<>();
            for (int code : buttons) { tap(code); expected.add(code+":"+KeyEvent.ACTION_DOWN); expected.add(code+":"+KeyEvent.ACTION_UP); }
            idle(); require(expected.equals(keys()),"Paused game did not receive balanced buttons: " + keys());
            motion(.75f,-.75f,.4f,.8f,1); idle();
            boolean[] axesReceived = new boolean[1];
            runOnMainSync(() -> axesReceived[0] = game.motions > 0 && Math.abs(game.x-.75f)<.01f && Math.abs(game.y+.75f)<.01f
                    && Math.abs(game.rx-.75f)<.01f && Math.abs(game.ry+.75f)<.01f
                    && Math.abs(game.l2-.4f)<.01f && Math.abs(game.r2-.8f)<.01f && game.hat == 1);
            require(axesReceived[0],"Paused game did not receive sticks, analog triggers, and D-pad axes");
            motion(0,0,0,0,0); idle();
            key(KeyEvent.KEYCODE_BUTTON_A,KeyEvent.ACTION_DOWN); expected.add(KeyEvent.KEYCODE_BUTTON_A+":"+KeyEvent.ACTION_DOWN);
            for (int code : LAYERS) { key(code,KeyEvent.ACTION_DOWN); expected.add(code+":"+KeyEvent.ACTION_DOWN); }
            motion(0,0,1,1,0); idle();
            for (int code : LAYERS) { key(code,KeyEvent.ACTION_UP); expected.add(code+":"+KeyEvent.ACTION_UP); }
            motion(0,0,0,0,0);
            key(KeyEvent.KEYCODE_BUTTON_A,KeyEvent.ACTION_UP); expected.add(KeyEvent.KEYCODE_BUTTON_A+":"+KeyEvent.ACTION_UP);
            idle(); require(expected.equals(keys()),"Resuming left gamepad buttons unbalanced: " + keys());
            tap(KeyEvent.KEYCODE_BUTTON_B); idle();
            require(expected.equals(keys()),"Four-button shortcut did not resume typing");
            openPadtype();
            AccessibilityNodeInfo field = automation.getRootInActiveWindow().findAccessibilityNodeInfosByViewId("io.github.dlip.padtype:id/test_field").get(0);
            field.performAction(AccessibilityNodeInfo.AccessibilityAction.ACTION_SHOW_ON_SCREEN.getId());
            require(field.performAction(AccessibilityNodeInfo.ACTION_FOCUS),"Could not focus the typing field"); idle();
            tap(KeyEvent.KEYCODE_BUTTON_R1);
            waitFor(() -> {
                var root = automation.getRootInActiveWindow();
                if (root == null) return false;
                var fields = root.findAccessibilityNodeInfosByViewId("io.github.dlip.padtype:id/test_field");
                return !fields.isEmpty() && " ".contentEquals(fields.get(0).getText());
            },"Typing did not work after resuming from gamepad mode");
        } catch (Exception | AssertionError error) { failure = error; throw error; }
        finally {
            try {
                if (game != null) {
                    for (int code : LAYERS) key(code,KeyEvent.ACTION_UP);
                    key(KeyEvent.KEYCODE_BUTTON_A,KeyEvent.ACTION_UP); motion(0,0,0,0,0); idle();
                }
                openPadtype();
                if ("Stop remapping".contentEquals(startButton().getText())) startButton().performAction(AccessibilityNodeInfo.ACTION_CLICK);
                chooseController(original);
            } catch (Exception | AssertionError error) {
                if (failure != null) failure.addSuppressed(error);
                else throw error;
            } finally {
                if (pad != null) pad.close();
                if (game != null) runOnMainSync(game::finish);
            }
        }
    }
}
