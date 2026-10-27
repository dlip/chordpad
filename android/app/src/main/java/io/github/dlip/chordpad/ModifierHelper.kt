package io.github.dlip.chordpad

import android.content.Context
import android.content.AttributionSource
import android.os.Binder
import android.os.Bundle
import android.os.IBinder
import android.os.Looper
import android.os.Parcel
import android.os.Process
import android.os.SystemClock
import android.view.InputDevice
import java.io.RandomAccessFile
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.Timer
import kotlin.concurrent.timerTask

/** Started by ADB with app_process; this class never runs inside the app process. */
object ModifierHelper {
    @JvmStatic fun main(args: Array<String>) {
        try {
            RandomAccessFile("/data/local/tmp/chordpad-modifiers.lock", "rw").use { file ->
                val lock = file.channel.tryLock()
                checkNotNull(lock) { "Modifier helper is already running" }
                lock.use { run() }
            }
        } catch (error: Throwable) { error.printStackTrace(); kotlin.system.exitProcess(1) }
    }

    private fun run() {
        check(Process.myUid() == 2000 || Process.myUid() == 0) { "Start this helper through ADB or root" }
        Looper.prepareMainLooper()
        val activityThread = Class.forName("android.app.ActivityThread")
        val thread = activityThread.getMethod("systemMain").invoke(null)
        val system = activityThread.getMethod("getSystemContext").invoke(thread) as Context
        val context = system.createPackageContext("com.android.shell", 0)
        val uid = context.packageManager.getApplicationInfo("io.github.dlip.chordpad", 0).uid
        val stopped = CountDownLatch(1)
        val connected = CountDownLatch(1)
        val process = ProcessBuilder("/system/bin/uinput", "-").redirectErrorStream(true).start()
        val output = process.outputStream.bufferedWriter()
        val lock = Any()
        var mask = 0
        var lastUpdate = SystemClock.uptimeMillis()
        var client: IBinder? = null
        val death = IBinder.DeathRecipient { stopped.countDown() }
        fun send(json: String) { output.write(json); output.newLine(); output.flush() }
        fun set(next: Int) = synchronized(lock) {
            val events = mutableListOf<Int>()
            for (i in ModifierProtocol.SCAN_CODES.indices) if ((next xor mask) and (1 shl i) != 0) {
                events.addAll(listOf(1, ModifierProtocol.SCAN_CODES[i], if (next and (1 shl i) != 0) 1 else 0))
            }
            if (events.isNotEmpty()) {
                send("""{"id":1,"command":"updateTimeBase"}""")
                events.addAll(listOf(0, 0, 0))
                send("""{"id":1,"command":"inject","events":[${events.joinToString(",")}]}""")
                mask = next
            }
            lastUpdate = SystemClock.uptimeMillis()
        }
        val timer = Timer("Modifier watchdog", true)
        try {
            send("""{"id":1,"command":"register","name":"${ModifierProtocol.DEVICE}","vid":1,"pid":1,"bus":"usb","configuration":[{"type":"UI_SET_EVBIT","data":["EV_KEY"]},{"type":"UI_SET_KEYBIT","data":[29,30,42,56,125]}]}""")
            val deadline = SystemClock.uptimeMillis() + 5000
            while (InputDevice.getDeviceIds().none { InputDevice.getDevice(it)?.name == ModifierProtocol.DEVICE }) {
                check(process.isAlive && SystemClock.uptimeMillis() < deadline) { "Virtual keyboard did not register" }
                SystemClock.sleep(20)
            }
            val binder = object : Binder() {
                override fun onTransact(code: Int, data: Parcel, reply: Parcel?, flags: Int): Boolean {
                    check(Binder.getCallingUid() == uid) { "Wrong client UID" }
                    data.enforceInterface(ModifierProtocol.DESCRIPTOR)
                    when (code) {
                        ModifierProtocol.CONNECT -> synchronized(lock) {
                            check(client == null) { "Helper already connected" }
                            val token = requireNotNull(data.readStrongBinder())
                            token.linkToDeath(death, 0); client = token; connected.countDown()
                        }
                        ModifierProtocol.SET -> {
                            check(client != null) { "Connect first" }
                            val next = data.readInt(); require(next in 0..15)
                            set(next)
                        }
                        ModifierProtocol.DISCONNECT -> stopped.countDown()
                        else -> return false
                    }
                    reply?.writeNoException()
                    return true
                }
            }
            timer.schedule(timerTask {
                try {
                    synchronized(lock) { if (SystemClock.uptimeMillis() - lastUpdate > 1500) set(0) }
                    if (!process.isAlive) stopped.countDown()
                } catch (_: Exception) { stopped.countDown() }
            }, 500, 500)
            val activityManager = Class.forName("android.app.ActivityManager").getMethod("getService").invoke(null)
            val managerClass = Class.forName("android.app.IActivityManager")
            val authority = "io.github.dlip.chordpad.modifiers"
            val holder = managerClass.getMethod("getContentProviderExternal", String::class.java,
                Int::class.javaPrimitiveType, IBinder::class.java, String::class.java)
                .invoke(activityManager, authority, 0, null, "chordpad")
            checkNotNull(holder) { "Chordpad provider unavailable" }
            try {
                val provider = holder.javaClass.getField("provider").get(holder)
                val source = AttributionSource.Builder(Process.myUid()).setPackageName("com.android.shell").build()
                Class.forName("android.content.IContentProvider").getMethod("call", AttributionSource::class.java,
                    String::class.java, String::class.java, String::class.java, Bundle::class.java)
                    .invoke(provider, source, authority, "attach", null, Bundle().apply { putBinder("helper", binder) })
            } finally {
                managerClass.getMethod("removeContentProviderExternal", String::class.java, IBinder::class.java)
                    .invoke(activityManager, authority, null)
            }
            check(connected.await(5, TimeUnit.SECONDS)) { "App did not connect" }
            println("Chordpad global modifiers ready")
            stopped.await()
        } finally {
            timer.cancel()
            client?.unlinkToDeath(death, 0)
            try { set(0) } finally { output.close(); process.destroy() }
        }
        kotlin.system.exitProcess(0)
    }
}
