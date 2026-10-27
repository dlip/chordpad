package io.github.dlip.chordpad

import android.os.Binder
import android.os.Handler
import android.os.HandlerThread
import android.os.IBinder
import android.os.Looper
import android.os.Parcel

internal object ModifierProtocol {
    const val DESCRIPTOR = "io.github.dlip.chordpad.Modifiers.v1"
    const val SET = IBinder.FIRST_CALL_TRANSACTION
    const val CONNECT = SET + 1
    const val DISCONNECT = SET + 2
    const val DEVICE = "Chordpad modifiers"
    val SCAN_CODES = intArrayOf(29, 56, 42, 125)

    fun call(remote: IBinder, code: Int, write: (Parcel) -> Unit) {
        val data = Parcel.obtain(); val reply = Parcel.obtain()
        try {
            data.writeInterfaceToken(DESCRIPTOR); write(data)
            check(remote.transact(code, data, reply, 0)) { "Incompatible modifier helper" }
            reply.readException()
        } finally { data.recycle(); reply.recycle() }
    }
}

class GlobalModifiers(private val changed: () -> Unit) {
    private val worker = Handler(HandlerThread("Chordpad modifiers").apply { start() }.looper)
    private val main = Handler(Looper.getMainLooper())
    private val token = Binder()
    private var remote: IBinder? = null
    @Volatile private var desired = 0
    @Volatile var connected = false; private set
    private val heartbeat = Runnable { sync() }
    private var death: IBinder.DeathRecipient? = null

    fun attach(binder: IBinder) {
        worker.post {
            disconnect()
            try {
                val recipient = IBinder.DeathRecipient { worker.post { if (remote === binder) disconnect() } }
                death = recipient
                binder.linkToDeath(recipient, 0)
                ModifierProtocol.call(binder, ModifierProtocol.CONNECT) { it.writeStrongBinder(token) }
                remote = binder
                connected = true; main.post { changed() }
                sync()
            } catch (_: Exception) { death?.let { binder.unlinkToDeath(it, 0) }; disconnect() }
        }
    }

    fun set(mask: Int) {
        desired = mask and 15
        worker.removeCallbacks(heartbeat)
        worker.post(heartbeat)
    }

    private fun sync() {
        worker.removeCallbacks(heartbeat)
        val binder = remote ?: return
        try {
            ModifierProtocol.call(binder, ModifierProtocol.SET) { it.writeInt(desired) }
            worker.postDelayed(heartbeat, 250)
        } catch (_: Exception) { disconnect() }
    }

    private fun disconnect() {
        worker.removeCallbacks(heartbeat)
        remote?.let { binder ->
            try { ModifierProtocol.call(binder, ModifierProtocol.SET) { it.writeInt(0) } } catch (_: Exception) { }
            death?.let { binder.unlinkToDeath(it, 0) }
            try { ModifierProtocol.call(binder, ModifierProtocol.DISCONNECT) { } } catch (_: Exception) { }
        }
        remote = null; death = null
        if (connected) { connected = false; main.post { changed() } }
    }
}
