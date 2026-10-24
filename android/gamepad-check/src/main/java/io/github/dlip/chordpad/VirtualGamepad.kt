package io.github.dlip.chordpad

import android.app.UiAutomation
import android.os.ParcelFileDescriptor
import android.view.KeyEvent
import java.io.OutputStream
import kotlin.math.roundToInt

class VirtualGamepad(automation: UiAutomation) : AutoCloseable {
    private val output: ParcelFileDescriptor
    private val commands: OutputStream

    init {
        val pipes = automation.executeShellCommandRw("uinput -")
        output = pipes[0]
        commands = ParcelFileDescriptor.AutoCloseOutputStream(pipes[1])
        send("""
                {"id":1,"command":"register","name":"chordpad test gamepad","vid":1356,"pid":3302,"bus":"usb",
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
                """.trimIndent())
    }

    private fun send(command: String) {
        commands.write((command + "\n").toByteArray(Charsets.UTF_8))
        commands.flush()
    }
    private fun inject(events: String) {
        send("{\"id\":1,\"command\":\"updateTimeBase\"}")
        send("{\"id\":1,\"command\":\"inject\",\"events\":[$events,0,0,0]}")
    }
    fun key(code: Int, action: Int) {
        val scan = when (code) {
            KeyEvent.KEYCODE_BUTTON_A -> 304; KeyEvent.KEYCODE_BUTTON_B -> 305
            KeyEvent.KEYCODE_BUTTON_X -> 308; KeyEvent.KEYCODE_BUTTON_Y -> 307
            KeyEvent.KEYCODE_BUTTON_L1 -> 310; KeyEvent.KEYCODE_BUTTON_R1 -> 311
            KeyEvent.KEYCODE_BUTTON_L2 -> 312; KeyEvent.KEYCODE_BUTTON_R2 -> 313
            else -> throw IllegalArgumentException("Unsupported test button")
        }
        inject("1,$scan,${if (action == KeyEvent.ACTION_DOWN) 1 else 0}")
    }
    fun motion(x: Float, y: Float, l2: Float, r2: Float, hat: Float) {
        val axes = listOf(0 to ((x + 1) * 127.5f).roundToInt(), 1 to ((y + 1) * 127.5f).roundToInt(),
            3 to ((x + 1) * 127.5f).roundToInt(), 4 to ((y + 1) * 127.5f).roundToInt(),
            2 to (l2 * 255).roundToInt(), 5 to (r2 * 255).roundToInt(), 16 to hat.roundToInt())
        inject(axes.joinToString(",") { (axis, value) -> "3,$axis,$value" })
    }
    override fun close() { try { commands.close() } finally { output.close() } }

    companion object { const val NAME = "chordpad test gamepad" }
}
