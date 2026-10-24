package io.github.dlip.chordpad;

import java.util.function.BooleanSupplier;
import java.util.function.Consumer;

final class LastAppSwitch {
    private final BooleanSupplier active, recents;
    private final Consumer<Runnable> schedule;
    private Runnable pending;

    LastAppSwitch(BooleanSupplier active, BooleanSupplier recents, Consumer<Runnable> schedule) {
        this.active = active; this.recents = recents; this.schedule = schedule;
    }
    boolean start() {
        if (pending != null) return true;
        if (!active.getAsBoolean() || !recents.getAsBoolean()) return false;
        pending = new Runnable() {
            @Override public void run() {
                if (pending != this) return;
                pending = null;
                if (active.getAsBoolean()) recents.getAsBoolean();
            }
        };
        schedule.accept(pending); return true;
    }
    void cancel() { pending = null; }
}
