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

    private TrunkGate() {
    }

    /**
     * Returns null when the request may be sent, otherwise the refusal reason.
     *
     * @param bcmUnlocked real BCM lock state; null = not reported yet
     * @param state       real contract VehicleState index (0 = unknown)
     * @param speedKmh    real speed in km/h; -1 = not reported yet
     * @param trunkOpen   real trunk latch state; null = not reported yet
     * @param sinceLastSentMs time since the last request that was sent
     */
    static String refusal(boolean testMode, boolean commanderReady, Boolean bcmUnlocked,
            int state, int speedKmh, Boolean trunkOpen, long sinceLastSentMs) {
        if (testMode) return "test mode";
        if (sinceLastSentMs < RATE_LIMIT_MS) return "repeat within 3 s";
        if (!commanderReady) return "no commander";
        if (!Boolean.TRUE.equals(bcmUnlocked)) return "scooter not unlocked";
        if (state != STATE_PARK) return "not in park";
        if (speedKmh != 0) return speedKmh < 0 ? "speed not reported" : "moving";
        if (trunkOpen == null) return "trunk state not reported";
        if (trunkOpen) return "already open";
        return null;
    }
}
