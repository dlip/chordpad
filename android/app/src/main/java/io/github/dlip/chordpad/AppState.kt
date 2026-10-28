package io.github.dlip.chordpad

import android.content.Context
import android.hardware.input.InputManager
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.InputDevice
import android.view.KeyEvent
import android.view.MotionEvent
import org.json.JSONObject

class AppState(private val context: Context) : InputManager.InputDeviceListener {
    private val preferences = context.getSharedPreferences("chordpad", Context.MODE_PRIVATE)
    private val manager = checkNotNull(context.getSystemService(InputManager::class.java))
    private val handler = Handler(Looper.getMainLooper())
    private val listeners = mutableSetOf<Runnable>()
    private val keys = GamepadKeys()
    @Volatile var service: ControllerService? = null
    var input: ControllerInput? = null; private set
    var device: InputDevice? = null; private set
    var layout: ChordLayout; private set
    var engine: ChordEngine; private set
    val direct = DirectMapper()
    val globalModifiers = GlobalModifiers { changed() }
    private var remappingEnabled = false
    val enabled get() = remappingEnabled
    private var remappingPaused = false
    val paused get() = remappingPaused
    var ready = false; private set
    var editor = false
    private var hideKeyboard = preferences.getBoolean("hide_soft_keyboard", true)
    val hideSoftKeyboard get() = hideKeyboard
    private var mappingMode = preferences.getString("mode", "chords") ?: "chords"
    val mode get() = mappingMode
    var status = "Remapping is off"; private set
    var lastAction = "None"; private set
    var layoutJson: String; private set
    private var gesture = false
    private var resumePending = false
    private var latest = Frame.NEUTRAL
    private val captureUpdate = Runnable { service?.updateCapture() }

    init {
        val saved = preferences.getString("layout", null)
        val restored = try {
            val json = saved ?: defaultLayout()
            json to ChordLayout(json)
        } catch (error: Exception) {
            try {
                val json = defaultLayout()
                json to ChordLayout(json)
            } catch (fatal: Exception) {
                throw IllegalStateException("Bundled layout is invalid", fatal)
            }
        }
        layoutJson = restored.first
        layout = restored.second
        engine = ChordEngine(layout)
        try {
            val savedMappings = JSONObject(preferences.getString("direct", "{}") ?: "{}")
            for (control in DirectMapper.CONTROLS) if (savedMappings.has(control)) {
                if (savedMappings.isNull(control)) direct.mappings.remove(control)
                else {
                    val action = Action.read(savedMappings.getJSONObject(control))
                    KeyboardOutput.validate(action)
                    direct.mappings[control] = action
                }
            }
        } catch (error: Exception) {
            status = "Could not restore direct mappings; using defaults"
            direct.mappings.clear()
            direct.mappings.putAll(DirectMapper.defaults())
        }
        manager.registerInputDeviceListener(this, handler)
        reconnect()
    }

    private fun captureChanged() { handler.removeCallbacks(captureUpdate); handler.post(captureUpdate) }
    private fun freshInput() {
        input = device?.let { ControllerInput(it) }
        latest = input?.frame() ?: Frame.NEUTRAL
    }

    fun defaultLayout(): String = context.assets.open("default-layout.json").use { it.readBytes().toString(Charsets.UTF_8) }

    fun controllers(): List<InputDevice> = InputDevice.getDeviceIds().asIterable().mapNotNull { InputDevice.getDevice(it) }
        .filter { ControllerInput.controller(it) }

    fun choose(device: InputDevice) {
        preferences.edit().putString("controller", device.descriptor).apply()
        this.device = device
        freshInput()
        resumePending = false
        keys.reset()
        reset("Release all controls")
    }

    private fun reconnect() {
        val descriptor = preferences.getString("controller", "") ?: ""
        var found: InputDevice? = null
        for (candidate in controllers()) {
            if (candidate.descriptor == descriptor) { found = candidate; break }
            if (descriptor.isEmpty() && (found == null || candidate.name.contains("DualSense"))) found = candidate
        }
        if (found != null) choose(found)
        else {
            device = null; input = null; keys.reset(); latest = Frame.NEUTRAL
            reset("Waiting for controller")
        }
    }

    override fun onInputDeviceAdded(id: Int) { if (device == null) reconnect() }
    override fun onInputDeviceRemoved(id: Int) { if (device?.id == id) reconnect() }
    override fun onInputDeviceChanged(id: Int) { if (device?.id == id) reconnect() }

    fun listen(listener: Runnable) { listeners.add(listener) }
    fun unlisten(listener: Runnable) { listeners.remove(listener) }
    fun changed() { listeners.toList().forEach { it.run() } }

    fun reset(reason: String) {
        setModifiers(0)
        captureChanged()
        service?.updateSoftKeyboard()
        if (!enabled || paused || device == null) service?.cancelLastAppSwitch()
        ready = false; gesture = true; engine.reset(); direct.reset(); status = reason
        changed()
    }

    fun setEnabled(enabled: Boolean) {
        remappingEnabled = enabled
        remappingPaused = false
        resumePending = enabled && keys.hasHeldPresses()
        if (enabled && !resumePending && device != null) freshInput()
        reset(if (enabled) "Release all controls" else "Remapping is off")
        if (enabled) { process(); handler.removeCallbacks(repeater); handler.postDelayed(repeater, if (mode == "chords" && layout.alphaBanks != null) 20L else 80L) }
        else handler.removeCallbacks(repeater)
        changed()
    }

    fun setPaused(paused: Boolean) {
        remappingPaused = paused
        resumePending = !paused && keys.hasHeldPresses()
        if (!paused && !resumePending && device != null) freshInput()
        if (paused) keys.suppressHeldGesture()
        reset(if (paused) "Paused; controller input goes to apps" else "Release all controls")
        process()
    }

    fun setHideSoftKeyboard(hide: Boolean) {
        hideKeyboard = hide
        preferences.edit().putBoolean("hide_soft_keyboard", hide).apply()
        service?.updateSoftKeyboard()
        changed()
    }

    fun setMode(mode: String) {
        mappingMode = mode
        preferences.edit().putString("mode", mode).apply()
        reset("Release all controls")
        process()
    }

    fun importLayout(json: String) {
        val next = ChordLayout(json)
        next.bindings.forEach { KeyboardOutput.validate(it.action) }
        layout = next; layoutJson = json; engine = ChordEngine(next)
        preferences.edit().putString("layout", json).apply()
        reset("Layout loaded; release all controls")
        process()
    }

    fun map(control: String, action: Action?) {
        if (action == null) direct.mappings.remove(control)
        else { KeyboardOutput.validate(action); direct.mappings[control] = action }
        val saved = JSONObject()
        for (name in DirectMapper.CONTROLS) saved.put(name, direct.mappings[name]?.json() ?: JSONObject.NULL)
        preferences.edit().putString("direct", saved.toString()).apply()
        reset("Mapping saved; release all controls")
        process()
    }

    fun focusChanged() {
        reset(if (editor) "Text field connected; release all controls" else "Focus a text field to type")
        process()
    }

    fun capturingMotion() = enabled && !paused && !resumePending || keys.hasConsumedPresses()

    fun key(event: KeyEvent): Boolean {
        val selected = device ?: return false
        if (event.deviceId != selected.id) return false
        val controller = input ?: return false
        val captured = capturingMotion()
        val active = enabled && !paused && !resumePending
        val handled = controller.key(event)
        val result = keys.update(event.keyCode, event.action, event.repeatCount, active && handled)
        if (resumePending && !keys.hasHeldPresses()) { resumePending = false; freshInput() }
        if (handled) {
            latest = input?.frame() ?: Frame.NEUTRAL
            if (enabled && paused && result.layerGesture) setPaused(false) else process()
            changed()
        }
        // Acknowledge the key release before Android rebuilds its input filter.
        if (captured != capturingMotion()) captureChanged()
        return result.consumed
    }

    fun motion(event: MotionEvent): Boolean {
        if (event.deviceId != device?.id || !event.isFromSource(InputDevice.SOURCE_JOYSTICK)) return false
        val controller = input ?: return false
        val active = enabled && !paused && !resumePending
        for (i in 0 until event.historySize) {
            controller.motion(event, i); latest = controller.frame(); process()
        }
        controller.motion(event, -1); latest = controller.frame(); process(); changed()
        return active
    }

    fun frame() = latest

    private fun process() {
        if (!enabled || device == null || resumePending) return
        if (gesture && latest.neutral()) gesture = false
        if (paused) return
        if (!gesture && latest.buttons and 0x0f == 0x0f) { setPaused(true); return }
        if (!ready) {
            if (!latest.neutral()) return
            ready = true; engine.update(latest); status = "Ready"
            return
        }
        val actions = if (mode == "chords") engine.update(latest) else direct.update(latest, SystemClock.uptimeMillis())
        if (mode == "chords") { status = engine.status; ready = engine.ready() }
        setModifiers(if (ready) {
            if (mode == "direct") direct.heldModifiers else engine.heldModifiers(latest)
        } else 0)
        emit(actions)
    }

    private fun setModifiers(mask: Int) {
        globalModifiers.set(mask)
        service?.setModifiers(mask)
    }

    private fun emit(actions: List<Action>) {
        for (action in actions) {
            try {
                if (service?.send(action) != true) {
                    status = "Focus a text field to type"; direct.reset(); setModifiers(0); break
                }
                lastAction = action.label()
            } catch (error: RuntimeException) { reset("Output stopped: ${error.message}"); break }
        }
    }

    private val repeater = object : Runnable {
        override fun run() {
            if (!enabled) return
            if (!paused && ready && mode == "direct") emit(direct.repeat(SystemClock.uptimeMillis()))
            if (!paused && ready && mode == "chords") {
                val actions = engine.repeat()
                setModifiers(engine.heldModifiers(latest))
                if (actions.isNotEmpty()) { status = engine.status; emit(actions); changed() }
            }
            handler.postDelayed(this, if (mode == "chords" && layout.alphaBanks != null) 20L else 80L)
        }
    }

    companion object {
        fun get(context: Context) = (context.applicationContext as ChordpadApplication).state()
    }
}
