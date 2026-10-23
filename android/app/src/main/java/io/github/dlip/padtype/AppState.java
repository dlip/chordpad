package io.github.dlip.padtype;

import android.content.Context;
import android.content.SharedPreferences;
import android.hardware.input.InputManager;
import android.os.Handler;
import android.os.Looper;
import android.os.SystemClock;
import android.view.InputDevice;
import android.view.KeyEvent;
import android.view.MotionEvent;
import org.json.JSONObject;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

public final class AppState implements InputManager.InputDeviceListener {
    public static AppState get(Context context) { return ((PadtypeApplication) context.getApplicationContext()).state(); }
    private final Context context;
    private final SharedPreferences preferences;
    private final InputManager manager;
    private final Handler handler = new Handler(Looper.getMainLooper());
    private final Set<Runnable> listeners = new HashSet<>();
    public volatile ControllerService service;
    public ControllerInput input;
    public InputDevice device;
    public ChordLayout layout;
    public ChordEngine engine;
    public final DirectMapper direct = new DirectMapper();
    public boolean enabled, paused, ready, editor, hideSoftKeyboard;
    public String mode, status = "Remapping is off", lastAction = "None", layoutJson;
    private boolean gesture;
    private Frame latest = Frame.NEUTRAL;

    AppState(Context context) {
        this.context = context;
        preferences = context.getSharedPreferences("padtype", Context.MODE_PRIVATE);
        hideSoftKeyboard = preferences.getBoolean("hide_soft_keyboard", true);
        manager = context.getSystemService(InputManager.class);
        mode = preferences.getString("mode", "chords");
        try {
            layoutJson = preferences.getString("layout", defaultLayout());
            layout = new ChordLayout(layoutJson);
        } catch (Exception error) {
            try { layoutJson = defaultLayout(); layout = new ChordLayout(layoutJson); }
            catch (Exception fatal) { throw new IllegalStateException("Bundled layout is invalid", fatal); }
        }
        engine = new ChordEngine(layout);
        try {
            JSONObject saved = new JSONObject(preferences.getString("direct", "{}"));
            for (String control : DirectMapper.CONTROLS) if (saved.has(control)) {
                if (saved.isNull(control)) direct.mappings.remove(control);
                else { Action action = Action.read(saved.getJSONObject(control)); KeyboardOutput.validate(action); direct.mappings.put(control, action); }
            }
        } catch (Exception error) { status = "Could not restore direct mappings; using defaults"; direct.mappings.clear(); direct.mappings.putAll(DirectMapper.defaults()); }
        manager.registerInputDeviceListener(this, handler);
        reconnect();
    }
    public String defaultLayout() throws IOException {
        try (var stream = context.getAssets().open("default-layout.json")) {
            return new String(stream.readAllBytes(), StandardCharsets.UTF_8);
        }
    }
    public List<InputDevice> controllers() {
        List<InputDevice> devices = new ArrayList<>();
        for (int id : InputDevice.getDeviceIds()) {
            InputDevice candidate = InputDevice.getDevice(id);
            if (ControllerInput.controller(candidate)) devices.add(candidate);
        }
        return devices;
    }
    public void choose(InputDevice device) {
        preferences.edit().putString("controller", device.getDescriptor()).apply();
        this.device = device;
        input = new ControllerInput(device);
        latest = input.frame();
        reset("Release all controls");
    }
    private void reconnect() {
        String descriptor = preferences.getString("controller", "");
        InputDevice found = null;
        for (InputDevice candidate : controllers()) {
            if (candidate.getDescriptor().equals(descriptor)) { found = candidate; break; }
            if (descriptor.isEmpty() && (found == null || candidate.getName().contains("DualSense"))) found = candidate;
        }
        if (found != null) choose(found);
        else { device = null; input = null; latest = Frame.NEUTRAL; reset("Waiting for controller"); }
    }
    @Override public void onInputDeviceAdded(int id) { if (device == null) reconnect(); }
    @Override public void onInputDeviceRemoved(int id) { if (device != null && device.getId() == id) reconnect(); }
    @Override public void onInputDeviceChanged(int id) { if (device != null && device.getId() == id) reconnect(); }

    public void listen(Runnable listener) { listeners.add(listener); }
    public void unlisten(Runnable listener) { listeners.remove(listener); }
    public void changed() { for (Runnable listener : new ArrayList<>(listeners)) listener.run(); }
    public void reset(String reason) {
        if (service != null) service.updateSoftKeyboard();
        if (service != null && (!enabled || paused || device == null)) service.cancelLastAppSwitch();
        ready = false; gesture = true; engine.reset(); direct.reset(); status = reason;
        changed();
    }
    public void setEnabled(boolean enabled) {
        this.enabled = enabled; paused = false;
        reset(enabled ? "Release all controls" : "Remapping is off");
        if (service != null) service.updateCapture();
        if (enabled) { process(); handler.removeCallbacks(repeater); handler.postDelayed(repeater, 80); }
        else handler.removeCallbacks(repeater);
        changed();
    }
    public void setPaused(boolean paused) { this.paused = paused; reset(paused ? "Paused" : "Release all controls"); process(); }
    public void setHideSoftKeyboard(boolean hide) {
        hideSoftKeyboard = hide; preferences.edit().putBoolean("hide_soft_keyboard", hide).apply();
        if (service != null) service.updateSoftKeyboard();
        changed();
    }
    public void setMode(String mode) {
        this.mode = mode; preferences.edit().putString("mode", mode).apply(); reset("Release all controls"); process();
    }
    public void importLayout(String json) throws Exception {
        ChordLayout next = new ChordLayout(json);
        for (ChordLayout.Binding binding : next.bindings) KeyboardOutput.validate(binding.action());
        layout = next; layoutJson = json; engine = new ChordEngine(next);
        preferences.edit().putString("layout", json).apply(); reset("Layout loaded; release all controls");
        process();
    }
    public void map(String control, Action action) throws Exception {
        if (action == null) direct.mappings.remove(control);
        else { KeyboardOutput.validate(action); direct.mappings.put(control, action); }
        JSONObject saved = new JSONObject();
        for (String name : DirectMapper.CONTROLS) {
            Action value = direct.mappings.get(name);
            saved.put(name, value == null ? JSONObject.NULL : value.json());
        }
        preferences.edit().putString("direct", saved.toString()).apply(); reset("Mapping saved; release all controls");
        process();
    }
    public void focusChanged() { reset(editor ? "Text field connected; release all controls" : "Focus a text field to type"); process(); }
    public boolean key(KeyEvent event) {
        if (device == null || event.getDeviceId() != device.getId()) return false;
        boolean handled = input.key(event);
        if (handled) { latest = input.frame(); process(); changed(); }
        return handled && enabled;
    }
    public boolean motion(MotionEvent event) {
        if (device == null || event.getDeviceId() != device.getId() || !event.isFromSource(InputDevice.SOURCE_JOYSTICK)) return false;
        for (int i = 0; i < event.getHistorySize(); i++) { input.motion(event, i); latest = input.frame(); process(); }
        input.motion(event, -1); latest = input.frame(); process(); changed(); return enabled;
    }
    public Frame frame() { return latest; }
    private void process() {
        if (!enabled || device == null) return;
        if (gesture && latest.neutral()) gesture = false;
        if (!gesture && (latest.buttons() & 0xf0) == 0xf0) { paused = !paused; reset(paused ? "Paused" : "Release all controls"); return; }
        if (paused) return;
        if (!ready) {
            if (!latest.neutral()) return;
            ready = true; engine.update(latest); status = "Ready"; return;
        }
        List<Action> actions = mode.equals("chords") ? engine.update(latest) : direct.update(latest, SystemClock.uptimeMillis());
        if (mode.equals("chords")) { status = engine.status; ready = engine.ready(); }
        emit(actions);
    }
    private void emit(List<Action> actions) {
        for (Action action : actions) {
            try {
                if (service == null || !service.send(action)) { status = "Focus a text field to type"; direct.reset(); break; }
                lastAction = action.label();
            } catch (RuntimeException error) { reset("Output stopped: " + error.getMessage()); break; }
        }
    }
    private final Runnable repeater = new Runnable() {
        @Override public void run() {
            if (!enabled) return;
            if (!paused && ready && mode.equals("direct")) emit(direct.repeat(SystemClock.uptimeMillis()));
            if (!paused && ready && mode.equals("chords")) emit(engine.repeat());
            handler.postDelayed(this, 80);
        }
    };
}
