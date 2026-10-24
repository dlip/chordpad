package io.github.dlip.chordpad;

import android.app.Activity;
import android.os.Bundle;
import android.text.InputType;
import android.view.Gravity;
import android.widget.EditText;

public class TestEditorActivity extends Activity {
    @Override public void onCreate(Bundle bundle) {
        super.onCreate(bundle);
        EditText editor = new EditText(this);
        editor.setContentDescription("chordpad test editor");
        editor.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_MULTI_LINE | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS);
        editor.setGravity(Gravity.TOP); editor.setTextSize(24); editor.setShowSoftInputOnFocus(false);
        setContentView(editor); editor.requestFocus();
    }
}
