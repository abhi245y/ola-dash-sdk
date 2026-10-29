package dev.olatool.dash;

/** Runs on a desktop JVM. No Android, Binder, network, or vehicle access. */
public final class UnlockSessionTest {
    private static int checks;

    private static void check(boolean condition, String reason) {
        checks++;
        if (!condition) throw new AssertionError(reason);
    }

    private static UnlockSession locked() {
        UnlockSession s = new UnlockSession();
        s.profileReady = true;
        s.attempts = 5;
        s.locked();
        return s;
    }

    private static String enter(UnlockSession s) {
        String result = null;
        for (int n : new int[] {0, 1, 2, 3, 4, 5}) result = s.key(n);
        return result;
    }

    public static void main(String[] args) {
        UnlockSession s = new UnlockSession();
        check(s.key(1) == null && s.digitCount() == 0, "No entry before vehicle state");
        s = locked();
        check("012345".equals(enter(s)), "Six digits preserve a leading zero");
        check(s.phase == UnlockSession.VERIFYING && s.digitCount() == 0, "Clear PIN after submission");
        check(s.key(7) == null && s.digitCount() == 0, "No duplicate request while verifying");
        s.verified(s.requestId(), true, 5);
        check(s.phase == UnlockSession.UNLOCKING, "PIN success must not imply PARK or DRIVE");
        s.unlocked();
        check(s.phase == UnlockSession.UNLOCKING, "BCM unlock still waits for PARK");
        s.vehiclePhase(UnlockSession.PARKED);
        check(s.phase == UnlockSession.PARKED, "Real PARK confirms parked screen");
        s.passcodeWindow(true);
        check(s.phase == UnlockSession.PARKED, "Late passcode-window callback cannot relock PARK");
        s.vehiclePhase(UnlockSession.DRIVING);
        s.verified(s.requestId(), true, 5);
        s.unlocked();
        check(s.phase == UnlockSession.DRIVING, "Late auth result/BCM callback cannot downgrade DRIVE");
        s.locked();
        check(s.phase == UnlockSession.LOCKED && s.digitCount() == 0, "Relock clears credentials");

        s = locked();
        enter(s);
        s.verified(s.requestId(), false, 4);
        check(s.phase == UnlockSession.LOCKED && s.error == UnlockSession.WRONG_PIN
                && s.attempts == 4, "Wrong PIN consumes stock-reported attempt");
        enter(s);
        s.verified(s.requestId(), false, 0);
        check(!s.canEnter() && enter(s) == null, "Exhausted attempts block requests");
        s.attempts = 5; // Only the controller's service refresh may supply restored attempts.
        check(s.canEnter(), "Backend refresh restores entry");

        s = locked();
        s.key(1); s.key(2); s.key(10);
        check(s.digitCount() == 1, "Delete removes exactly one digit");
        s.key(11);
        check(s.digitCount() == 0, "Clear removes all digits");
        s.testMode = true;
        check(enter(s) == null && s.digitCount() == 0, "Debug mode cannot submit real unlock");
        s.testMode = false;
        enter(s); s.finishRequest(s.requestId()); s.failed();
        check(!s.canEnter() && s.phase == UnlockSession.LOCKED, "Service failure disables entry");
        check(s.digitCount() == 0, "Service failure clears PIN");

        s = locked();
        enter(s); s.verified(s.requestId(), true, 5); s.unconfirmed();
        check(s.phase == UnlockSession.LOCKED && s.error == UnlockSession.UNCONFIRMED,
                "Missing vehicle confirmation cannot show parked");
        s.disconnected();
        check(s.phase == UnlockSession.CONNECTING && !s.canEnter(), "Lost vehicle state blocks entry");
        s.vehiclePhase(UnlockSession.CHARGING);
        s.unlocked();
        check(s.phase == UnlockSession.CHARGING, "Charging is not PARK or DRIVE");
        s.passcodeWindow(true);
        check(s.phase == UnlockSession.LOCKED, "A real passcode prompt can open while charging");

        s = locked();
        enter(s);
        int first = s.requestId();
        s.bcmLocked();
        check(!s.canEnter() && enter(s) == null, "BCM lock cannot reopen an outstanding request");
        check(s.displayPhase() == UnlockSession.VERIFYING, "Outstanding request stays visibly pending");
        s.vehiclePhase(UnlockSession.LOCKED);
        check(!s.canEnter(), "Idle callback cannot reopen an outstanding request");
        s.verified(first, false, 4);
        enter(s);
        int second = s.requestId();
        check(first != second, "Each transport request has a unique identity");
        s.verified(first, true, 5);
        check(s.requestInFlight() && s.phase == UnlockSession.VERIFYING,
                "A reversed stale response cannot complete the next request");
        s.verified(second, false, 3);
        check(!s.requestInFlight() && s.attempts == 3, "Only matching response releases request fence");
        enter(s);
        int pending = s.requestId();
        s.disconnected(); s.bcmLocked();
        check(!s.canEnter(), "Reconnect lock replay cannot overlap a still-running request");
        s.discardRequest(pending);
        check(s.canEnter(), "Discarded response releases fence without authenticating");

        for (int actual : new int[] {UnlockSession.PARKED, UnlockSession.DRIVING}) {
            s = locked(); s.bcmLocked(); s.vehiclePhase(actual);
            check(s.phase == actual, "Vehicle mode wins over earlier BCM lock");
            s = locked(); s.vehiclePhase(actual); s.bcmLocked();
            check(s.phase == actual, "Vehicle mode wins over delayed BCM lock");
            s.vehiclePhase(UnlockSession.LOCKED);
            check(s.phase == UnlockSession.LOCKED, "Actual idle mode still returns to keypad");
        }

        TestEventGate gate = new TestEventGate();
        gate.setActive(true);
        int oldToken = gate.token();
        int[] callbacks = {0};
        gate.setActive(false);
        check(!gate.dispatch(oldToken, () -> callbacks[0]++), "TEST_OFF discards queued fake DRIVE");
        gate.setActive(true);
        check(!gate.dispatch(oldToken, () -> callbacks[0]++), "New test session cannot revive old events");
        check(gate.dispatch(gate.token(), () -> {
            check(gate.isInjected(), "Injected provenance is attached during callback");
            gate.setActive(false);
            check(gate.isInjected(), "Provenance survives test-state change during callback");
        }), "Current test event dispatches");
        check(callbacks[0] == 0 && !gate.isInjected(), "Stale callbacks never run; provenance is cleared");

        s = locked();
        s.vehiclePhase(UnlockSession.LOCKED);
        s.bcmLocked(); s.passcodeWindow(false);
        s.key(0); s.key(1); s.key(2);
        for (int i = 0; i < 100; i++) {
            s.vehiclePhase(UnlockSession.LOCKED);
            s.bcmLocked(); s.passcodeWindow(false);
        }
        check(s.digitCount() == 3, "Periodic sleeping/locked/window-false snapshots preserve PIN entry");
        s.passcodeWindow(true);
        check(s.digitCount() == 0, "A new passcode-window edge clears previous entry");
        s.key(3); s.key(4);
        for (int i = 0; i < 100; i++) s.passcodeWindow(true);
        check(s.digitCount() == 2, "Repeated window-true snapshots preserve PIN entry");
        s.clear(); enter(s);
        int repeatedRequest = s.requestId();
        for (int i = 0; i < 100; i++) {
            s.vehiclePhase(UnlockSession.LOCKED);
            s.bcmLocked(); s.passcodeWindow(true);
        }
        check(s.requestInFlight() && s.displayPhase() == UnlockSession.VERIFYING,
                "Repeated snapshots do not interrupt a pending PIN check");
        s.verified(repeatedRequest, false, 4);
        for (int i = 0; i < 100; i++) {
            s.vehiclePhase(UnlockSession.LOCKED);
            s.bcmLocked(); s.passcodeWindow(true);
        }
        check(s.error == UnlockSession.WRONG_PIN, "Periodic snapshots preserve wrong-PIN feedback");
        s.key(1); s.disconnected();
        s.vehiclePhase(UnlockSession.LOCKED); s.bcmLocked(); s.passcodeWindow(true);
        check(s.phase == UnlockSession.LOCKED && s.digitCount() == 0,
                "Reconnect resets snapshot history and clears old entry");
        System.out.println("UnlockSession: " + checks + " checks passed");
    }
}
