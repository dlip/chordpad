package io.github.dlip.chordpad

class LastAppSwitch(
    private val active: () -> Boolean,
    private val recents: () -> Boolean,
    private val schedule: (Runnable) -> Unit
) {
    private var pending: Runnable? = null

    fun start(): Boolean {
        if (pending != null) return true
        if (!active() || !recents()) return false
        val task = object : Runnable {
            override fun run() {
                if (pending !== this) return
                pending = null
                if (active()) recents()
            }
        }
        pending = task
        schedule(task)
        return true
    }

    fun cancel() { pending = null }
}
