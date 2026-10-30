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
        System.out.println("TrunkGateTest: " + checks + " checks passed");
    }
}
