package dev.olatool.dash;

/** Runs on a desktop JVM. Gate rules for the trunk request. No Android or vehicle access. */
public final class TrunkGateTest {
    private static int checks;

    private static void check(boolean condition, String reason) {
        checks++;
        if (!condition) throw new AssertionError(reason);
    }

    private static final long READY = 1_000_000L; // time since last send, past the rate limit

    private static String ok(Boolean lock, int state, int speed, Boolean trunk) {
        return TrunkGate.refusal(false, true, lock, state, speed, trunk, READY);
    }

    public static void main(String[] args) {
        int park = TrunkGate.STATE_PARK;
        check(ok(true, park, 0, false) == null, "Unlocked, parked, stopped, closed: allowed");
        check("test mode".equals(TrunkGate.refusal(true, true, true, park, 0, false, READY)),
                "Test mode refuses even with a valid real state");
        check("repeat within 3 s".equals(TrunkGate.refusal(false, true, true, park, 0, false, 2999)),
                "Repeat inside 3 s refused");
        check(TrunkGate.refusal(false, true, true, park, 0, false, 3000) == null,
                "Request at exactly 3 s allowed");
        check("no commander".equals(TrunkGate.refusal(false, false, true, park, 0, false, READY)),
                "No commander binder refuses");
        check("scooter not unlocked".equals(ok(false, park, 0, false)), "Locked BCM refuses");
        check("scooter not unlocked".equals(ok(null, park, 0, false)), "Unknown BCM lock refuses");
        check("not in park".equals(ok(true, 4, 0, false)), "DRIVE refuses");
        check("not in park".equals(ok(true, 0, 0, false)), "Unknown vehicle state refuses");
        check("moving".equals(ok(true, park, 3, false)), "Moving refuses");
        check("speed not reported".equals(ok(true, park, -1, false)), "Unknown speed refuses");
        check("already open".equals(ok(true, park, 0, true)), "Already open refuses");
        check("trunk state not reported".equals(ok(true, park, 0, null)), "Unknown trunk refuses");
        int charging = TrunkGate.STATE_CHARGING;
        check(TrunkGate.parked(false, true, park, 0), "Unlocked, parked, stopped: parked");
        check("scooter not unlocked".equals(TrunkGate.notParked(false, null, park, 0)), "Unknown lock: not parked");
        check(!TrunkGate.parked(false, null, park, 0), "Unknown lock fails closed");
        check(!TrunkGate.parked(false, false, park, 0), "Locked: not parked");
        check(!TrunkGate.parked(false, true, 0, 0), "Unknown state fails closed");
        check(!TrunkGate.parked(false, true, 4, 0), "DRIVE: not parked");
        check(!TrunkGate.parked(false, true, park, 3), "Moving: not parked");
        check(!TrunkGate.parked(false, true, park, -1), "Unknown speed fails closed");
        check(!TrunkGate.parked(true, true, park, 0), "Test mode: not parked");
        check(TrunkGate.parked(false, true, charging, -1), "Charging, unlocked, speed unknown: parked");
        check(TrunkGate.parked(false, true, charging, 5), "Charging does not need speed 0");
        check(!TrunkGate.parked(false, false, charging, 0), "Charging, locked: not parked");
        check(!TrunkGate.parked(false, null, charging, 0), "Charging, unknown lock fails closed");
        check(!TrunkGate.parked(true, true, charging, 0), "Charging in test mode: not parked");
        check(TrunkGate.refusal(false, true, true, charging, 0, false, READY) != null,
                "Charging is parked but still refuses the trunk");
        System.out.println("TrunkGateTest: " + checks + " checks passed");
    }
}
