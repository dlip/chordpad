package io.github.dlip.chordpad

import android.app.Activity
import android.os.Bundle
import android.text.InputType
import android.view.Gravity
import android.widget.EditText

class TestEditorActivity : Activity() {
    override fun onCreate(bundle: Bundle?) {
        super.onCreate(bundle)
        val editor = EditText(this).apply {
            contentDescription = "chordpad test editor"
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE or InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
            gravity = Gravity.TOP; textSize = 24f; showSoftInputOnFocus = false
        }
        setContentView(editor)
        editor.requestFocus()
    }
}
