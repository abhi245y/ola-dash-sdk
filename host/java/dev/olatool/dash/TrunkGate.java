package dev.olatool.dash;

/**
 * Gate for the one vehicle request the host can send: open the trunk/seat latch.
 * Pure logic on plain values so it runs on a desktop JVM (host/test/TrunkGateTest.java).
 * Callers pass real vehicle values only; test-injected values never reach here.
 */
final class TrunkGate {
    static final long RATE_LIMIT_MS = 3000L;
    /** Contract VehicleState.park index (see VehicleBridge.stateIndex). */
    static final int STATE_PARK = 3;
    /** Contract VehicleState.charging index. */
    static final int STATE_CHARGING = 5;

    private TrunkGate() {
    }

    /**
     * The park rule: null when the scooter is unlocked, in park and known to be stopped,
     * otherwise the reason.
     *
     * @param bcmUnlocked real BCM lock state; null = not reported yet
     * @param state       real contract VehicleState index (0 = unknown)
     * @param speedKmh    real speed in km/h; -1 = not reported yet
     */
    static String notParked(boolean testMode, Boolean bcmUnlocked, int state, int speedKmh) {
        if (testMode) return "test mode";
        if (!Boolean.TRUE.equals(bcmUnlocked)) return "scooter not unlocked";
        if (state != STATE_PARK) return "not in park";
        if (speedKmh != 0) return speedKmh < 0 ? "speed not reported" : "moving";
        return null;
    }

    /** Gate for host screens and app launches: parked, or unlocked and charging (speed not required). */
    static boolean parked(boolean testMode, Boolean bcmUnlocked, int state, int speedKmh) {
        return notParked(testMode, bcmUnlocked, state, speedKmh) == null
                || (!testMode && state == STATE_CHARGING && Boolean.TRUE.equals(bcmUnlocked));
    }

    /**
     * Returns null when the request may be sent, otherwise the refusal reason.
     * Parameters as in {@link #notParked}, plus:
     *
     * @param trunkOpen   real trunk latch state; null = not reported yet
     * @param sinceLastSentMs time since the last request that was sent
     */
    static String refusal(boolean testMode, boolean commanderReady, Boolean bcmUnlocked,
            int state, int speedKmh, Boolean trunkOpen, long sinceLastSentMs) {
        String why = notParked(testMode, bcmUnlocked, state, speedKmh);
        if (why != null) return why;
        if (sinceLastSentMs < RATE_LIMIT_MS) return "repeat within 3 s";
        if (!commanderReady) return "no commander";
        if (trunkOpen == null) return "trunk state not reported";
        if (trunkOpen) return "already open";
        return null;
    }
}
