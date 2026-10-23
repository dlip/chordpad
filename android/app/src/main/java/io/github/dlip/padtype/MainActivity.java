package io.github.dlip.padtype;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.Intent;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.os.Bundle;
import android.provider.Settings;
import android.text.InputType;
import android.view.Gravity;
import android.view.InputDevice;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.View;
import android.view.WindowInsets;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.RadioButton;
import android.widget.RadioGroup;
import android.widget.ScrollView;
import android.widget.Switch;
import android.widget.TextView;
import android.widget.Toast;
import java.nio.charset.StandardCharsets;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

public final class MainActivity extends Activity {
    private static final int INK = 0xff182b34, MUTED = 0xff586a73, BLUE = 0xff1557b0;
    private AppState state;
    private TextView connection, status, serviceStatus, profile, action;
    private Button start, pause;
    private Switch hideSoftKeyboard;
    private ControllerView controller;
    private LinearLayout directCard, chordCard;
    private final Map<String, Button> mappingButtons = new LinkedHashMap<>();
    private final Runnable refresh = this::refresh;

    @Override public void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        state = AppState.get(this);
        ScrollView scroll = new ScrollView(this); scroll.setFillViewport(true); scroll.setBackgroundColor(0xfff6f5f0);
        LinearLayout page = column(); page.setPadding(dp(24),dp(20),dp(24),dp(32));
        scroll.addView(page); setContentView(scroll);
        scroll.setOnApplyWindowInsetsListener((view, insets) -> {
            var bars = insets.getInsets(WindowInsets.Type.systemBars() | WindowInsets.Type.displayCutout());
            scroll.setPadding(bars.left,bars.top,bars.right,bars.bottom); return insets;
        });
        TextView title = text("padtype.",36,INK); title.setTypeface(null,Typeface.BOLD); page.addView(title);
        page.addView(text("A controller, a keyboard, your layout.",16,MUTED));

        LinearLayout live = card(page);
        connection = text("",20,INK); live.addView(connection);
        boolean wide = getResources().getConfiguration().screenWidthDp >= 840;
        LinearLayout liveBody = wide ? row() : column(); live.addView(liveBody);
        controller = new ControllerView(this,state);
        liveBody.addView(controller,new LinearLayout.LayoutParams(wide ? 0 : -1,-2,wide ? 1.2f : 0));
        LinearLayout controls = column(); liveBody.addView(controls,new LinearLayout.LayoutParams(wide ? 0 : -1,-2,wide ? 1 : 0));
        LinearLayout row = row(); controls.addView(row);
        start = button("Start remapping", v -> {
            if (state.service == null) { accessibility(); return; }
            if (state.device == null) { toast("Connect a game controller first"); return; }
            state.setEnabled(!state.enabled);
        }); start.setId(R.id.start_remapping); row.addView(start,new LinearLayout.LayoutParams(0,dp(52),1));
        pause = button("Pause",v -> state.setPaused(!state.paused)); row.addView(pause);
        status = text("",16,BLUE); status.setTypeface(null,Typeface.BOLD); controls.addView(status);
        action = text("",14,MUTED); controls.addView(action);
        controls.addView(button("Choose controller",v -> chooseController()));

        LinearLayout setup = card(page);
        setup.addView(text("Controller input",20,INK));
        serviceStatus = text("",14,MUTED); setup.addView(serviceStatus);
        setup.addView(button("Enable controller input…",v -> accessibility()));
        setup.addView(text("Sends text and keys to a focused text field in this or another app. Games and apps without a text input connection may not accept mapped keys.",14,MUTED));
        hideSoftKeyboard = new Switch(this); hideSoftKeyboard.setId(R.id.hide_soft_keyboard);
        hideSoftKeyboard.setText(R.string.hide_soft_keyboard); hideSoftKeyboard.setChecked(state.hideSoftKeyboard);
        hideSoftKeyboard.setOnCheckedChangeListener((button,checked) -> { if (checked != state.hideSoftKeyboard) state.setHideSoftKeyboard(checked); });
        setup.addView(hideSoftKeyboard);
        setup.addView(text(getString(R.string.hide_soft_keyboard_description),14,MUTED));

        LinearLayout modes = card(page); modes.addView(text("Input mode",20,INK));
        RadioGroup group = new RadioGroup(this); group.setOrientation(LinearLayout.HORIZONTAL);
        RadioButton chords = new RadioButton(this); chords.setText(R.string.mode_chords_label); chords.setId(R.id.mode_chords); group.addView(chords);
        RadioButton direct = new RadioButton(this); direct.setText(R.string.mode_direct_label); direct.setId(R.id.mode_direct); group.addView(direct);
        group.check(state.mode.equals("chords") ? R.id.mode_chords : R.id.mode_direct);
        group.setOnCheckedChangeListener((g,id) -> state.setMode(id == R.id.mode_chords ? "chords" : "direct")); modes.addView(group);

        chordCard = card(page); chordCard.addView(text("Typing layout",20,INK));
        profile = text("",14,MUTED); chordCard.addView(profile);
        chordCard.addView(text("Cross = A · Circle = B · Square = X · Triangle = Y\nL2: Shift · R2: numbers and symbols\nBundled numbers: clockwise D-pad 1–8; Triangle 9, Circle 0; Square -, Cross =. Add L2 for _ and +.\nBundled layout: hold R2 + L1 for navigation. D-pad arrows; Triangle/Cross Page Up/Down; Square/Circle Home/End. Add L2 to select text.\nHold R2 + R1: function keys using number combos (0 = F10); Cross/Square = F11/F12.\nAdjacent face pairs add media in navigation and volume in functions.\nOptions: Enter · Share: Escape\nSticks: up Alt, down Command, outward Ctrl, inward Ctrl + Alt + Command.\nInward-up: Command + Ctrl + Alt + Shift; inward-down: Alt + Command.",15,INK));
        chordCard.addView(button("View chord reference",v -> reference()));
        LinearLayout files = row(); chordCard.addView(files);
        files.addView(button("Import layout…",v -> {
            Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT).setType("*/*").addCategory(Intent.CATEGORY_OPENABLE);
            startActivityForResult(intent,10);
        }));
        files.addView(button("Export…",v -> startActivityForResult(new Intent(Intent.ACTION_CREATE_DOCUMENT).setType("application/json").putExtra(Intent.EXTRA_TITLE,"padtype-layout.json"),11)));
        chordCard.addView(button("Restore bundled layout",v -> new AlertDialog.Builder(this).setTitle("Restore bundled layout?")
                .setMessage("Replace the imported layout with the included 97-binding padtype layout.")
                .setNegativeButton("Cancel",null).setPositiveButton("Restore",(dialog,which) -> {
                    try { state.importLayout(state.defaultLayout()); } catch (Exception error) { failure(error); }
                }).show()));

        directCard = card(page); directCard.addView(text("Direct mappings",20,INK));
        directCard.addView(text("Tap an input to change its text, key, modifier, or Android navigation action. Hold a stick direction to repeat. L2 is Shift and R2 is Ctrl by default.",14,MUTED));
        for (String control : DirectMapper.CONTROLS) {
            Button entry = button("",v -> editMapping(control)); directCard.addView(entry); mappingButtons.put(control,entry);
        }

        LinearLayout test = card(page); test.addView(text("Try typing",20,INK));
        test.addView(text("Start remapping, tap the field, and release all controls. L1 + R1 + L2 + R2 pauses or resumes.",14,MUTED));
        EditText field = new EditText(this); field.setId(R.id.test_field); field.setHint("Type here with the DualSense…");
        field.setMinLines(4); field.setGravity(Gravity.TOP); field.setTextSize(18);
        field.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_MULTI_LINE | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS);
        test.addView(field,new LinearLayout.LayoutParams(-1,dp(180)));
        test.addView(button("Focus test field",v -> field.requestFocus()));
        page.addView(text("Android 14+ · Stored on this device · No network access",12,MUTED));
        refresh();
    }

    @Override protected void onResume() { super.onResume(); state.listen(refresh); refresh(); }
    @Override protected void onPause() { state.unlisten(refresh); super.onPause(); }
    private void refresh() {
        connection.setText(state.device == null ? "Waiting for a controller" : state.device.getName());
        start.setText(state.enabled ? "Stop remapping" : "Start remapping");
        pause.setEnabled(state.enabled); pause.setText(state.paused ? "Resume" : "Pause");
        hideSoftKeyboard.setChecked(state.hideSoftKeyboard);
        status.setText(!state.enabled ? "Remapping is off" : state.status);
        action.setText(getString(R.string.last_output,state.lastAction));
        serviceStatus.setText(state.service == null ? "Enable padtype in Android accessibility settings to type across apps." : "Controller service connected" + (state.editor ? " · Text field connected" : " · Focus a text field to type"));
        profile.setText(getResources().getQuantityString(R.plurals.binding_count,state.layout.bindings.size(),state.layout.bindings.size()));
        chordCard.setVisibility(state.mode.equals("chords") ? View.VISIBLE : View.GONE);
        directCard.setVisibility(state.mode.equals("direct") ? View.VISIBLE : View.GONE);
        for (Map.Entry<String,Button> entry : mappingButtons.entrySet()) {
            Action output = state.direct.mappings.get(entry.getKey());
            String value = getString(R.string.mapping_label,label(entry.getKey()),output == null ? "Unassigned" : output.label());
            if (!entry.getValue().getText().toString().equals(value)) entry.getValue().setText(value);
        }
        controller.invalidate();
    }
    private void accessibility() {
        startActivity(new Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS));
    }
    private void chooseController() {
        List<InputDevice> devices = state.controllers();
        if (devices.isEmpty()) { toast("No game controllers connected"); return; }
        new AlertDialog.Builder(this).setTitle("Choose controller")
                .setItems(devices.stream().map(InputDevice::getName).toArray(String[]::new),(dialog,index) -> state.choose(devices.get(index))).show();
    }
    private void reference() {
        startActivity(new Intent(this,ReferenceActivity.class));
    }
    static String direction(int bits) { return ((bits&4)!=0 ? "↑" : (bits&8)!=0 ? "↓" : "") + ((bits&1)!=0 ? "←" : (bits&2)!=0 ? "→" : ""); }
    static String label(String control) {
        return switch (control) { case "A" -> "Cross ×"; case "B" -> "Circle ○"; case "X" -> "Square □"; case "Y" -> "Triangle △"; case "Start" -> "Options"; case "Select" -> "Share"; default -> control; };
    }
    private void editMapping(String control) {
        String[] options = {"Text…","Key…","Hold modifier…","Android navigation…","Unassigned"};
        new AlertDialog.Builder(this).setTitle(label(control)).setItems(options,(dialog,index) -> {
            if (index == 0) {
                EditText input = new EditText(this); input.setSingleLine(true); input.setHint("Text to type");
                Action old = state.direct.mappings.get(control); if (old != null && old.type().equals("character")) input.setText(old.value());
                new AlertDialog.Builder(this).setTitle("Text for " + label(control)).setView(input).setNegativeButton("Cancel",null)
                        .setPositiveButton("Save",(d,w) -> save(control,Action.text(input.getText().toString()))).show();
            } else if (index == 1) {
                String[] keys = {"enter","escape","space","backspace","delete","tab","up","down","left","right","home","end","page_up","page_down","a","b","c","d","e","f","g","h","i","j","k","l","m","n","o","p","q","r","s","t","u","v","w","x","y","z","f1","f2","f3","f4","f5","f6","f7","f8","f9","f10","f11","f12","media_play_pause","media_next","media_previous","media_stop","volume_up","volume_down","mute"};
                new AlertDialog.Builder(this).setTitle("Output key").setItems(keys,(d,w) -> save(control,Action.key(keys[w]))).show();
            } else if (index == 2) {
                String[] modifiers = {"ctrl","alt","shift","meta"};
                new AlertDialog.Builder(this).setTitle("Held modifier").setItems(modifiers,(d,w) -> save(control,new Action("modifier",modifiers[w],0))).show();
            } else if (index == 3) {
                String[] actions = {"back","home","recents"};
                new AlertDialog.Builder(this).setTitle("Android navigation").setItems(actions,(d,w) -> save(control,new Action("system",actions[w],0))).show();
            } else save(control,null);
        }).show();
    }
    private void save(String control,Action action) { try { state.map(control,action); } catch (Exception error) { failure(error); } }
    @Override protected void onActivityResult(int request,int result,Intent data) {
        super.onActivityResult(request,result,data);
        if (result != RESULT_OK || data == null || data.getData() == null) return;
        try {
            if (request == 10) {
                try (var stream = getContentResolver().openInputStream(data.getData())) {
                    if (stream == null) throw new java.io.IOException("Cannot open layout");
                    byte[] bytes = stream.readNBytes(1024*1024+1);
                    if (bytes.length > 1024*1024) throw new java.io.IOException("Layout exceeds 1 MiB");
                    state.importLayout(new String(bytes,StandardCharsets.UTF_8)); toast("Layout imported");
                }
            } else if (request == 11) {
                try (var stream = getContentResolver().openOutputStream(data.getData(),"wt")) {
                    if (stream == null) throw new java.io.IOException("Cannot write layout");
                    stream.write(state.layoutJson.getBytes(StandardCharsets.UTF_8)); toast("Layout exported");
                }
            }
        } catch (Exception error) { failure(error); }
    }
    @Override public boolean dispatchKeyEvent(KeyEvent event) {
        if (state.device != null && event.getDeviceId() == state.device.getId()
                && (KeyEvent.isGamepadButton(event.getKeyCode()) || (event.getKeyCode() >= KeyEvent.KEYCODE_DPAD_UP && event.getKeyCode() <= KeyEvent.KEYCODE_DPAD_RIGHT))) {
            if (state.service == null && state.key(event)) return true;
        }
        return super.dispatchKeyEvent(event);
    }
    @Override public boolean dispatchGenericMotionEvent(MotionEvent event) {
        if (state.device != null && event.getDeviceId() == state.device.getId() && event.isFromSource(InputDevice.SOURCE_JOYSTICK) && state.motion(event)) return true;
        return super.dispatchGenericMotionEvent(event);
    }
    private LinearLayout column() { LinearLayout view = new LinearLayout(this); view.setOrientation(LinearLayout.VERTICAL); return view; }
    private LinearLayout row() { LinearLayout view = new LinearLayout(this); view.setGravity(Gravity.CENTER_VERTICAL); return view; }
    private LinearLayout card(LinearLayout page) {
        LinearLayout card = column(); card.setPadding(dp(20),dp(16),dp(20),dp(16));
        GradientDrawable background = new GradientDrawable(); background.setColor(0xfffefefd); background.setCornerRadius(dp(16)); background.setStroke(dp(1),0xffd7dfdf); card.setBackground(background);
        LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(-1,-2); params.topMargin = dp(18); page.addView(card,params); return card;
    }
    private TextView text(String value,float size,int color) { TextView text = new TextView(this); text.setText(value); text.setTextSize(size); text.setTextColor(color); text.setPadding(0,dp(4),0,dp(8)); return text; }
    private Button button(String label,View.OnClickListener listener) {
        Button button = new Button(this); button.setText(label); button.setTextColor(BLUE); button.setAllCaps(false); button.setMinHeight(dp(48)); button.setOnClickListener(listener); return button;
    }
    private int dp(float value) { return Math.round(value * getResources().getDisplayMetrics().density); }
    private void toast(String text) { Toast.makeText(this,text,Toast.LENGTH_LONG).show(); }
    private void failure(Exception error) { new AlertDialog.Builder(this).setTitle("Could not apply mapping").setMessage(error.getMessage()).setPositiveButton("OK",null).show(); }
}
