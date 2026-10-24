package io.github.dlip.chordpad

import android.app.Activity
import android.app.AlertDialog
import android.content.Intent
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.provider.Settings
import android.text.InputType
import android.view.Gravity
import android.view.InputDevice
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.View
import android.view.WindowInsets
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.RadioButton
import android.widget.RadioGroup
import android.widget.ScrollView
import android.widget.Switch
import android.widget.TextView
import android.widget.Toast
import java.io.IOException
import kotlin.math.roundToInt

class MainActivity : Activity() {
    private val ink = 0xff182b34.toInt()
    private val muted = 0xff586a73.toInt()
    private val blue = 0xff1557b0.toInt()
    private lateinit var state: AppState
    private lateinit var connection: TextView
    private lateinit var status: TextView
    private lateinit var serviceStatus: TextView
    private lateinit var profile: TextView
    private lateinit var action: TextView
    private lateinit var start: Button
    private lateinit var pause: Button
    private lateinit var hideSoftKeyboard: Switch
    private lateinit var controller: ControllerView
    private lateinit var directCard: LinearLayout
    private lateinit var chordCard: LinearLayout
    private val mappingButtons = linkedMapOf<String, Button>()
    private val refresh = Runnable { refresh() }

    override fun onCreate(bundle: Bundle?) {
        super.onCreate(bundle)
        state = AppState.get(this)
        val scroll = ScrollView(this).apply { isFillViewport = true; setBackgroundColor(0xfff6f5f0.toInt()) }
        val page = column().apply { setPadding(dp(24), dp(20), dp(24), dp(32)) }
        scroll.addView(page)
        setContentView(scroll)
        scroll.setOnApplyWindowInsetsListener { _, insets ->
            val bars = insets.getInsets(WindowInsets.Type.systemBars() or WindowInsets.Type.displayCutout())
            scroll.setPadding(bars.left, bars.top, bars.right, bars.bottom)
            insets
        }
        page.addView(text("chordpad.", 36f, ink).apply { setTypeface(null, Typeface.BOLD) })
        page.addView(text("A controller, a keyboard, your layout.", 16f, muted))

        val live = card(page)
        connection = text("", 20f, ink); live.addView(connection)
        val wide = resources.configuration.screenWidthDp >= 840
        val liveBody = if (wide) row() else column(); live.addView(liveBody)
        controller = ControllerView(this, state)
        liveBody.addView(controller, LinearLayout.LayoutParams(if (wide) 0 else -1, -2, if (wide) 1.2f else 0f))
        val controls = column()
        liveBody.addView(controls, LinearLayout.LayoutParams(if (wide) 0 else -1, -2, if (wide) 1f else 0f))
        val buttons = row(); controls.addView(buttons)
        start = button("Start remapping") {
            when {
                state.service == null -> accessibility()
                state.device == null -> toast("Connect a game controller first")
                else -> state.setEnabled(!state.enabled)
            }
        }.apply { id = R.id.start_remapping }
        buttons.addView(start, LinearLayout.LayoutParams(0, dp(52), 1f))
        pause = button("Pause") { state.setPaused(!state.paused) }; buttons.addView(pause)
        status = text("", 16f, blue).apply { setTypeface(null, Typeface.BOLD) }; controls.addView(status)
        action = text("", 14f, muted); controls.addView(action)
        controls.addView(button("Choose controller") { chooseController() })

        val setup = card(page)
        setup.addView(text("Controller input", 20f, ink))
        serviceStatus = text("", 14f, muted); setup.addView(serviceStatus)
        setup.addView(button("Enable controller input…") { accessibility() })
        setup.addView(text("Sends text and keys to a focused text field in this or another app. Games and apps without a text input connection may not accept mapped keys.", 14f, muted))
        hideSoftKeyboard = Switch(this).apply {
            id = R.id.hide_soft_keyboard
            setText(R.string.hide_soft_keyboard)
            isChecked = state.hideSoftKeyboard
            setOnCheckedChangeListener { _, checked -> if (checked != state.hideSoftKeyboard) state.setHideSoftKeyboard(checked) }
        }
        setup.addView(hideSoftKeyboard)
        setup.addView(text(getString(R.string.hide_soft_keyboard_description), 14f, muted))

        val modes = card(page); modes.addView(text("Input mode", 20f, ink))
        val group = RadioGroup(this).apply { orientation = LinearLayout.HORIZONTAL }
        group.addView(RadioButton(this).apply { setText(R.string.mode_chords_label); id = R.id.mode_chords })
        group.addView(RadioButton(this).apply { setText(R.string.mode_direct_label); id = R.id.mode_direct })
        group.check(if (state.mode == "chords") R.id.mode_chords else R.id.mode_direct)
        group.setOnCheckedChangeListener { _, id -> state.setMode(if (id == R.id.mode_chords) "chords" else "direct") }
        modes.addView(group)

        chordCard = card(page); chordCard.addView(text("Typing layout", 20f, ink))
        profile = text("", 14f, muted); chordCard.addView(profile)
        chordCard.addView(text("Cross = A · Circle = B · Square = X · Triangle = Y\nL2: Shift · R2: numbers and symbols\nBundled numbers: clockwise D-pad 1–8; Triangle 9, Circle 0; Square -, Cross =. Add L2 for _ and +.\nBundled layout: hold R2 + L1 for navigation. D-pad arrows; Triangle/Cross Page Up/Down; Square/Circle Home/End. Add L2 to select text.\nHold R2 + R1: function keys using number combos (0 = F10); Cross/Square = F11/F12.\nAdjacent face pairs add media in navigation and volume in functions.\nOptions: Enter · Share: Escape\nSticks: up Alt, down Command, outward Ctrl, inward Ctrl + Alt + Command.\nInward-up: Command + Ctrl + Alt + Shift; inward-down: Alt + Command.", 15f, ink))
        chordCard.addView(button("View chord reference") { startActivity(Intent(this, ReferenceActivity::class.java)) })
        val files = row(); chordCard.addView(files)
        files.addView(button("Import layout…") {
            startActivityForResult(Intent(Intent.ACTION_OPEN_DOCUMENT).setType("*/*").addCategory(Intent.CATEGORY_OPENABLE), 10)
        })
        files.addView(button("Export…") {
            startActivityForResult(Intent(Intent.ACTION_CREATE_DOCUMENT).setType("application/json").putExtra(Intent.EXTRA_TITLE, "chordpad-layout.json"), 11)
        })
        chordCard.addView(button("Restore bundled layout") {
            AlertDialog.Builder(this).setTitle("Restore bundled layout?")
                .setMessage("Replace the imported layout with the included 97-binding Chordpad layout.")
                .setNegativeButton("Cancel", null).setPositiveButton("Restore") { _, _ ->
                    try { state.importLayout(state.defaultLayout()) } catch (error: Exception) { failure(error) }
                }.show()
        })

        directCard = card(page); directCard.addView(text("Direct mappings", 20f, ink))
        directCard.addView(text("Tap an input to change its text, key, modifier, or Android navigation action. Hold a stick direction to repeat. L2 is Shift and R2 is Ctrl by default.", 14f, muted))
        for (control in DirectMapper.CONTROLS) {
            val entry = button("") { editMapping(control) }
            directCard.addView(entry)
            mappingButtons[control] = entry
        }

        val test = card(page); test.addView(text("Try typing", 20f, ink))
        test.addView(text("Start remapping, tap the field, and release all controls. L1 + R1 + L2 + R2 pauses or resumes.", 14f, muted))
        val field = EditText(this).apply {
            id = R.id.test_field; hint = "Type here with the DualSense…"
            minLines = 4; gravity = Gravity.TOP; textSize = 18f
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE or InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
        }
        test.addView(field, LinearLayout.LayoutParams(-1, dp(180)))
        test.addView(button("Focus test field") { field.requestFocus() })
        page.addView(text("Android 14+ · Stored on this device · No network access", 12f, muted))
        refresh()
    }

    override fun onResume() { super.onResume(); state.listen(refresh); refresh() }
    override fun onPause() { state.unlisten(refresh); super.onPause() }

    private fun refresh() {
        connection.text = state.device?.name ?: "Waiting for a controller"
        start.text = if (state.enabled) "Stop remapping" else "Start remapping"
        pause.isEnabled = state.enabled
        pause.text = if (state.paused) "Resume" else "Pause"
        hideSoftKeyboard.isChecked = state.hideSoftKeyboard
        status.text = if (!state.enabled) "Remapping is off" else state.status
        action.text = getString(R.string.last_output, state.lastAction)
        serviceStatus.text = if (state.service == null) "Enable Chordpad in Android accessibility settings to type across apps."
            else "Controller service connected" + if (state.editor) " · Text field connected" else " · Focus a text field to type"
        profile.text = resources.getQuantityString(R.plurals.binding_count, state.layout.bindings.size, state.layout.bindings.size)
        chordCard.visibility = if (state.mode == "chords") View.VISIBLE else View.GONE
        directCard.visibility = if (state.mode == "direct") View.VISIBLE else View.GONE
        for ((control, button) in mappingButtons) {
            val value = getString(R.string.mapping_label, label(control), state.direct.mappings[control]?.label() ?: "Unassigned")
            if (button.text.toString() != value) button.text = value
        }
        controller.invalidate()
    }

    private fun accessibility() { startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS)) }

    private fun chooseController() {
        val devices = state.controllers()
        if (devices.isEmpty()) { toast("No game controllers connected"); return }
        AlertDialog.Builder(this).setTitle("Choose controller").setItems(devices.map { it.name }.toTypedArray()) { _, index ->
            state.choose(devices[index])
        }.show()
    }

    private fun editMapping(control: String) {
        val options = arrayOf("Text…", "Key…", "Hold modifier…", "Android navigation…", "Unassigned")
        AlertDialog.Builder(this).setTitle(label(control)).setItems(options) { _, index ->
            when (index) {
                0 -> {
                    val input = EditText(this).apply { setSingleLine(true); hint = "Text to type" }
                    state.direct.mappings[control]?.takeIf { it.type == "character" }?.let { input.setText(it.value) }
                    AlertDialog.Builder(this).setTitle("Text for ${label(control)}").setView(input).setNegativeButton("Cancel", null)
                        .setPositiveButton("Save") { _, _ -> save(control, Action.text(input.text.toString())) }.show()
                }
                1 -> {
                    val keys = arrayOf("enter", "escape", "space", "backspace", "delete", "tab", "up", "down", "left", "right", "home", "end", "page_up", "page_down",
                        "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z",
                        "f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8", "f9", "f10", "f11", "f12", "print_screen", "media_play_pause", "media_next", "media_previous", "media_stop", "volume_up", "volume_down", "mute")
                    AlertDialog.Builder(this).setTitle("Output key").setItems(keys) { _, which -> save(control, Action.key(keys[which])) }.show()
                }
                2 -> {
                    val modifiers = arrayOf("ctrl", "alt", "shift", "meta")
                    AlertDialog.Builder(this).setTitle("Held modifier").setItems(modifiers) { _, which -> save(control, Action("modifier", modifiers[which])) }.show()
                }
                3 -> {
                    val actions = arrayOf("back", "home", "recents")
                    AlertDialog.Builder(this).setTitle("Android navigation").setItems(actions) { _, which -> save(control, Action("system", actions[which])) }.show()
                }
                else -> save(control, null)
            }
        }.show()
    }

    private fun save(control: String, action: Action?) {
        try { state.map(control, action) } catch (error: Exception) { failure(error) }
    }

    override fun onActivityResult(request: Int, result: Int, data: Intent?) {
        super.onActivityResult(request, result, data)
        val uri = data?.data ?: return
        if (result != RESULT_OK) return
        try {
            when (request) {
                10 -> contentResolver.openInputStream(uri).use { stream ->
                    if (stream == null) throw IOException("Cannot open layout")
                    val bytes = stream.readNBytes(1024 * 1024 + 1)
                    if (bytes.size > 1024 * 1024) throw IOException("Layout exceeds 1 MiB")
                    state.importLayout(bytes.toString(Charsets.UTF_8)); toast("Layout imported")
                }
                11 -> contentResolver.openOutputStream(uri, "wt").use { stream ->
                    if (stream == null) throw IOException("Cannot write layout")
                    stream.write(state.layoutJson.toByteArray(Charsets.UTF_8)); toast("Layout exported")
                }
            }
        } catch (error: Exception) { failure(error) }
    }

    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        if (event.deviceId == state.device?.id && (KeyEvent.isGamepadButton(event.keyCode) ||
            event.keyCode in KeyEvent.KEYCODE_DPAD_UP..KeyEvent.KEYCODE_DPAD_RIGHT)) {
            if (state.service == null && state.key(event)) return true
        }
        return super.dispatchKeyEvent(event)
    }

    override fun dispatchGenericMotionEvent(event: MotionEvent): Boolean {
        if (event.deviceId == state.device?.id && event.isFromSource(InputDevice.SOURCE_JOYSTICK) && state.motion(event)) return true
        return super.dispatchGenericMotionEvent(event)
    }

    private fun column() = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
    private fun row() = LinearLayout(this).apply { gravity = Gravity.CENTER_VERTICAL }
    private fun card(page: LinearLayout): LinearLayout {
        val card = column().apply {
            setPadding(dp(20), dp(16), dp(20), dp(16))
            background = GradientDrawable().apply { setColor(0xfffefefd.toInt()); cornerRadius = dp(16).toFloat(); setStroke(dp(1), 0xffd7dfdf.toInt()) }
        }
        page.addView(card, LinearLayout.LayoutParams(-1, -2).apply { topMargin = dp(18) })
        return card
    }
    private fun text(value: String, size: Float, color: Int) = TextView(this).apply {
        text = value; textSize = size; setTextColor(color); setPadding(0, dp(4), 0, dp(8))
    }
    private fun button(label: String, listener: View.OnClickListener) = Button(this).apply {
        text = label; setTextColor(blue); isAllCaps = false; minHeight = dp(48); setOnClickListener(listener)
    }
    private fun dp(value: Int) = (value * resources.displayMetrics.density).roundToInt()
    private fun toast(text: String) { Toast.makeText(this, text, Toast.LENGTH_LONG).show() }
    private fun failure(error: Exception) {
        AlertDialog.Builder(this).setTitle("Could not apply mapping").setMessage(error.message).setPositiveButton("OK", null).show()
    }

    companion object {
        fun direction(bits: Int) = (if (bits and 4 != 0) "↑" else if (bits and 8 != 0) "↓" else "") +
            (if (bits and 1 != 0) "←" else if (bits and 2 != 0) "→" else "")
        fun label(control: String) = when (control) {
            "A" -> "Cross ×"; "B" -> "Circle ○"; "X" -> "Square □"; "Y" -> "Triangle △"
            "Start" -> "Options"; "Select" -> "Share"; else -> control
        }
    }
}
