package io.github.dlip.chordpad

import android.app.Activity
import android.graphics.Typeface
import android.os.Bundle
import android.view.KeyEvent
import android.view.MotionEvent
import android.widget.EditText

class ModifierTestActivity : Activity() {
    lateinit var editor: EditText
    var web: android.webkit.WebView? = null
    @Volatile var webReady = false
    @Volatile var focused = false
    @Volatile var mouseHover = false
    @Volatile var mouseX = 0f
    @Volatile var mouseY = 0f
    @Volatile var mouseShift = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.addFlags(android.view.WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        if (intent.getBooleanExtra("web", false)) {
            web = android.webkit.WebView(this).apply {
                settings.javaScriptEnabled = true
                webViewClient = object : android.webkit.WebViewClient() {
                    override fun onPageFinished(view: android.webkit.WebView, url: String) {
                        view.evaluateJavascript("c.focus();let r=document.createRange();r.setStart(c.firstChild,2);r.collapse(true);let s=getSelection();s.removeAllRanges();s.addRange(r)") { webReady = true }
                    }
                }
                loadDataWithBaseURL("https://chordpad.invalid/", """<meta name="viewport" content="width=device-width,initial-scale=1"><div id="editor" contenteditable style="position:absolute;left:24px;top:32px;font:24px monospace;width:90%;height:200px">alpha bravo charlie</div><script>window.c=document.getElementById('editor')</script>""", "text/html", "UTF-8", null)
            }
            setContentView(web)
            return
        }
        editor = EditText(this).apply {
            setText("alpha bravo charlie")
            textSize = 24f; typeface = Typeface.MONOSPACE
            setPadding(60, 80, 60, 80)
            showSoftInputOnFocus = false
        }
        setContentView(editor)
        editor.requestFocus()
        editor.setSelection(2)
    }
    override fun onWindowFocusChanged(hasFocus: Boolean) { super.onWindowFocusChanged(hasFocus); focused = hasFocus }
    override fun dispatchGenericMotionEvent(event: MotionEvent): Boolean {
        if (event.isFromSource(android.view.InputDevice.SOURCE_MOUSE) && event.action == MotionEvent.ACTION_HOVER_MOVE) {
            mouseHover = true; mouseX = event.rawX; mouseY = event.rawY
        }
        return super.dispatchGenericMotionEvent(event)
    }
    override fun dispatchTouchEvent(event: MotionEvent): Boolean {
        if (event.isFromSource(android.view.InputDevice.SOURCE_MOUSE) && event.action == MotionEvent.ACTION_DOWN) mouseShift = event.metaState and KeyEvent.META_SHIFT_ON != 0
        return super.dispatchTouchEvent(event)
    }
}
