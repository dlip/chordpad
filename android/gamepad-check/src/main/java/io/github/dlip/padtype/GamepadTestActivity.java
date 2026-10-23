package io.github.dlip.padtype;

import android.app.Activity;
import android.os.Bundle;
import android.view.InputDevice;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.widget.TextView;
import java.util.ArrayList;
import java.util.List;
import io.github.dlip.padtype.gamepadcheck.R;

public final class GamepadTestActivity extends Activity {
    final List<String> keys = new ArrayList<>();
    int motions;
    float x, y, rx, ry, l2, r2, hat;
    private TextView output;

    @Override public void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        output = new TextView(this); output.setTextSize(24); output.setPadding(32,32,32,32);
        setContentView(output); refresh();
    }
    @Override public boolean dispatchKeyEvent(KeyEvent event) {
        if (event.isFromSource(InputDevice.SOURCE_GAMEPAD) || event.isFromSource(InputDevice.SOURCE_DPAD)) {
            keys.add(event.getKeyCode() + ":" + event.getAction()); refresh(); return true;
        }
        return super.dispatchKeyEvent(event);
    }
    @Override public boolean onGenericMotionEvent(MotionEvent event) {
        if (event.isFromSource(InputDevice.SOURCE_JOYSTICK)) {
            motions++; x = event.getAxisValue(MotionEvent.AXIS_X); y = event.getAxisValue(MotionEvent.AXIS_Y);
            rx = event.getAxisValue(MotionEvent.AXIS_Z); ry = event.getAxisValue(MotionEvent.AXIS_RZ);
            l2 = event.getAxisValue(MotionEvent.AXIS_LTRIGGER); r2 = event.getAxisValue(MotionEvent.AXIS_RTRIGGER);
            hat = event.getAxisValue(MotionEvent.AXIS_HAT_X); refresh(); return true;
        }
        return super.onGenericMotionEvent(event);
    }
    private void refresh() { output.setText(getString(R.string.gamepad_status,keys.size(),motions)); }
}
