package dev.olatool.dash;

/** Identifies synthetic callbacks even if TEST_OFF races with queued dispatch. */
final class TestEventGate {
    private int generation;
    private boolean active;
    private final ThreadLocal<Boolean> injected = new ThreadLocal<>();

    synchronized void setActive(boolean enabled) {
        if (active != enabled) generation++;
        active = enabled;
    }

    synchronized int token() { return generation; }
    boolean isInjected() { return Boolean.TRUE.equals(injected.get()); }

    synchronized boolean dispatch(int token, Runnable callback) {
        if (!active || generation != token) return false;
        injected.set(true);
        try { callback.run(); }
        finally { injected.remove(); }
        return true;
    }
}
