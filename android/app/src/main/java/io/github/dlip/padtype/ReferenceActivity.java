package io.github.dlip.padtype;

import android.app.Activity;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.os.Bundle;
import android.view.Gravity;
import android.view.View;
import android.view.WindowInsets;
import android.widget.AdapterView;
import android.widget.ArrayAdapter;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.Spinner;
import android.widget.TextView;
import java.util.ArrayList;
import java.util.List;

public final class ReferenceActivity extends Activity {
    private static final int INK = 0xff182b34, MUTED = 0xff586a73, BLUE = 0xff1557b0;
    private final List<View> sections = new ArrayList<>();
    private Spinner filter;

    @Override public void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        LinearLayout root = column(); root.setBackgroundColor(0xfff6f5f0);
        setContentView(root);
        root.setOnApplyWindowInsetsListener((view,insets) -> {
            var bars = insets.getInsets(WindowInsets.Type.systemBars() | WindowInsets.Type.displayCutout());
            root.setPadding(bars.left,bars.top,bars.right,bars.bottom); return insets;
        });
        LinearLayout toolbar = new LinearLayout(this); toolbar.setGravity(Gravity.CENTER_VERTICAL);
        toolbar.setPadding(dp(16),dp(8),dp(16),0); root.addView(toolbar);
        TextView title = text("Chord reference",24,INK); title.setTypeface(null,Typeface.BOLD);
        toolbar.addView(title,new LinearLayout.LayoutParams(0,-2,1));
        Button done = button("Done"); done.setOnClickListener(v -> finish()); toolbar.addView(done);
        List<LayoutReference.Layer> layers = LayoutReference.layers(AppState.get(this).layout);
        List<String> choices = new ArrayList<>(); choices.add("All layers");
        for (var layer : layers) choices.add(layer.title());
        choices.add("Thumbstick modifiers");
        filter = new Spinner(this); filter.setContentDescription("Reference layer");
        ArrayAdapter<String> adapter = new ArrayAdapter<>(this,android.R.layout.simple_spinner_item,choices);
        adapter.setDropDownViewResource(android.R.layout.simple_spinner_dropdown_item); filter.setAdapter(adapter);
        LinearLayout.LayoutParams filterParams = new LinearLayout.LayoutParams(-1,dp(56));
        filterParams.setMargins(dp(16),0,dp(16),0); root.addView(filter,filterParams);

        ScrollView scroll = new ScrollView(this); root.addView(scroll,new LinearLayout.LayoutParams(-1,0,1));
        LinearLayout page = column(); page.setPadding(dp(16),0,dp(16),dp(24)); scroll.addView(page);
        page.addView(text("A / Cross × · B / Circle ○ · X / Square □ · Y / Triangle △\nOptions: Enter · Share: Escape in every layer. Hold modifiers before starting a chord; release ordinary controls to type.",14,MUTED));
        for (var layer : layers) {
            LinearLayout section = section(page,layer.title(),layer.note());
            maps(section,layer.maps(),false);
            Button toggle = button("Show text chord reference"); section.addView(toggle);
            LinearLayout details = column(); details.setVisibility(View.GONE); section.addView(details);
            for (var chord : layer.chords()) {
                TextView line = text(chord.output() + "    ←    " + chord.controls(),16,INK);
                line.setTextIsSelectable(true); line.setPadding(0,dp(8),0,dp(8)); details.addView(line);
            }
            toggle.setOnClickListener(v -> {
                boolean show = details.getVisibility() != View.VISIBLE;
                details.setVisibility(show ? View.VISIBLE : View.GONE);
                toggle.setText(show ? "Hide text chord reference" : "Show text chord reference");
            });
        }
        LinearLayout sticks = section(page,"Thumbstick modifiers","Hold either stick while typing. Inward points toward the controller center. Command sends Meta / Super. Stick modifiers combine with L2 Shift and R2 layers.");
        maps(sticks,LayoutReference.sticks(),true);
        filter.setOnItemSelectedListener(new AdapterView.OnItemSelectedListener() {
            @Override public void onItemSelected(AdapterView<?> parent, View view, int position, long id) {
                for (int i = 0; i < sections.size(); i++) sections.get(i).setVisibility(position == 0 || position == i+1 ? View.VISIBLE : View.GONE);
                scroll.scrollTo(0,0);
            }
            @Override public void onNothingSelected(AdapterView<?> parent) {}
        });
        if (bundle != null) filter.setSelection(bundle.getInt("layer",0));
    }

    @Override protected void onSaveInstanceState(Bundle bundle) {
        bundle.putInt("layer",filter.getSelectedItemPosition()); super.onSaveInstanceState(bundle);
    }

    private LinearLayout section(LinearLayout page, String title, String note) {
        LinearLayout section = column(); section.setPadding(0,dp(20),0,0); page.addView(section); sections.add(section);
        TextView heading = text(title,22,INK); heading.setTypeface(null,Typeface.BOLD); heading.setAccessibilityHeading(true); section.addView(heading);
        section.addView(text(note,15,MUTED)); return section;
    }

    private void maps(LinearLayout section, List<LayoutReference.Map> maps, boolean sticks) {
        var config = getResources().getConfiguration();
        int columns = config.screenWidthDp / config.fontScale >= 720 ? 2 : 1;
        for (int i = 0; i < maps.size(); i += columns) {
            LinearLayout row = new LinearLayout(this); row.setLayoutDirection(View.LAYOUT_DIRECTION_LTR); row.setBaselineAligned(false); section.addView(row);
            for (int j = i; j < Math.min(i+columns,maps.size()); j++) {
                var map = maps.get(j);
                LinearLayout panel = column();
                LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(0,-2,1);
                if (j != i) params.leftMargin = dp(16); row.addView(panel,params);
                TextView heading = text(map.title(),18,INK); heading.setTypeface(null,Typeface.BOLD); heading.setAccessibilityHeading(true); panel.addView(heading);
                for (int r = 0; r < 3; r++) {
                    LinearLayout cells = new LinearLayout(this); cells.setBaselineAligned(false); panel.addView(cells);
                    for (int c = 0; c < 3; c++) {
                        var cell = map.cells().get(r*3+c);
                        LinearLayout box = column(); box.setGravity(Gravity.CENTER); box.setPadding(dp(6),dp(10),dp(6),dp(10)); box.setMinimumHeight(dp(94));
                        GradientDrawable background = new GradientDrawable(); background.setColor(0xfff6f5f0); background.setCornerRadius(dp(8)); background.setStroke(dp(1),0xffd7dfdf); box.setBackground(background);
                        TextView output = text(sticks ? cell.controls() : cell.output(),sticks ? 24 : 20,BLUE);
                        output.setGravity(Gravity.CENTER); output.setTypeface(null,Typeface.BOLD); box.addView(output,new LinearLayout.LayoutParams(-1,-2));
                        TextView controls = text(sticks ? cell.output() : cell.controls(),sticks ? 14 : 12,INK);
                        controls.setGravity(Gravity.CENTER); box.addView(controls,new LinearLayout.LayoutParams(-1,-2));
                        box.setContentDescription(map.title() + ": " + cell.controls() + ", " + (cell.output().equals("·") ? "Unassigned" : cell.output()));
                        box.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_YES);
                        output.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO); controls.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO);
                        LinearLayout.LayoutParams cellParams = new LinearLayout.LayoutParams(0,-1,1);
                        cellParams.setMargins(0,0,c == 2 ? 0 : dp(6),dp(6)); cells.addView(box,cellParams);
                    }
                }
            }
        }
    }

    private LinearLayout column() { LinearLayout view = new LinearLayout(this); view.setOrientation(LinearLayout.VERTICAL); return view; }
    private TextView text(String value, float size, int color) {
        TextView text = new TextView(this); text.setText(value); text.setTextSize(size); text.setTextColor(color); text.setPadding(0,dp(4),0,dp(8)); return text;
    }
    private Button button(String label) { Button button = new Button(this); button.setText(label); button.setAllCaps(false); button.setTextColor(BLUE); button.setMinHeight(dp(48)); return button; }
    private int dp(float value) { return Math.round(value * getResources().getDisplayMetrics().density); }
}
