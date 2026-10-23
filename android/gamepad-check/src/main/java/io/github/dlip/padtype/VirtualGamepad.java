package io.github.dlip.padtype;

import android.app.UiAutomation;
import android.os.ParcelFileDescriptor;
import android.view.KeyEvent;
import java.io.IOException;
import java.io.OutputStream;
import java.nio.charset.StandardCharsets;

final class VirtualGamepad implements AutoCloseable {
    static final String NAME = "padtype test gamepad";
    private final ParcelFileDescriptor output;
    private final OutputStream commands;

    VirtualGamepad(UiAutomation automation) throws IOException {
        ParcelFileDescriptor[] pipes = automation.executeShellCommandRw("uinput -");
        output = pipes[0]; commands = new ParcelFileDescriptor.AutoCloseOutputStream(pipes[1]);
        send("""
                {"id":1,"command":"register","name":"padtype test gamepad","vid":1356,"pid":3302,"bus":"usb",
                 "configuration":[
                   {"type":"UI_SET_EVBIT","data":["EV_KEY","EV_ABS"]},
                   {"type":"UI_SET_KEYBIT","data":[304,305,307,308,310,311,312,313,314,315,317,318]},
                   {"type":"UI_SET_ABSBIT","data":[0,1,2,3,4,5,16,17]}],
                 "abs_info":[
                   {"code":0,"info":{"value":128,"minimum":0,"maximum":255}},
                   {"code":1,"info":{"value":128,"minimum":0,"maximum":255}},
                   {"code":2,"info":{"value":0,"minimum":0,"maximum":255}},
                   {"code":3,"info":{"value":128,"minimum":0,"maximum":255}},
                   {"code":4,"info":{"value":128,"minimum":0,"maximum":255}},
                   {"code":5,"info":{"value":0,"minimum":0,"maximum":255}},
                   {"code":16,"info":{"value":0,"minimum":-1,"maximum":1}},
                   {"code":17,"info":{"value":0,"minimum":-1,"maximum":1}}]}
                """);
    }
    private void send(String command) throws IOException {
        commands.write((command+"\n").getBytes(StandardCharsets.UTF_8)); commands.flush();
    }
    private void inject(String events) throws IOException {
        send("{\"id\":1,\"command\":\"updateTimeBase\"}");
        send("{\"id\":1,\"command\":\"inject\",\"events\":["+events+",0,0,0]}");
    }
    void key(int code,int action) throws IOException {
        int scan = switch (code) {
            case KeyEvent.KEYCODE_BUTTON_A -> 304; case KeyEvent.KEYCODE_BUTTON_B -> 305;
            case KeyEvent.KEYCODE_BUTTON_X -> 308; case KeyEvent.KEYCODE_BUTTON_Y -> 307;
            case KeyEvent.KEYCODE_BUTTON_L1 -> 310; case KeyEvent.KEYCODE_BUTTON_R1 -> 311;
            case KeyEvent.KEYCODE_BUTTON_L2 -> 312; case KeyEvent.KEYCODE_BUTTON_R2 -> 313;
            default -> throw new IllegalArgumentException("Unsupported test button");
        };
        inject("1,"+scan+","+(action == KeyEvent.ACTION_DOWN ? 1 : 0));
    }
    void motion(float x,float y,float l2,float r2,float hat) throws IOException {
        inject("3,0,"+Math.round((x+1)*127.5f)+",3,1,"+Math.round((y+1)*127.5f)
                +",3,3,"+Math.round((x+1)*127.5f)+",3,4,"+Math.round((y+1)*127.5f)
                +",3,2,"+Math.round(l2*255)+",3,5,"+Math.round(r2*255)+",3,16,"+Math.round(hat));
    }
    @Override public void close() throws IOException { try { commands.close(); } finally { output.close(); } }
}
