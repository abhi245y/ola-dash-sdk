package dev.olatool.dash;

/** Authentication/UI state only. Contains no vehicle transport or Android APIs. */
final class UnlockSession {
    static final int CONNECTING = 0, LOCKED = 1, VERIFYING = 2, UNLOCKING = 3,
            PARKED = 4, DRIVING = 5, CHARGING = 6;
    static final int NONE = 0, WRONG_PIN = 1, SERVICE_ERROR = 2, UNCONFIRMED = 3;

    int phase = CONNECTING;
    int error = NONE;
    int attempts = -1;
    int cooldownSeconds;
    boolean profileReady;
    boolean testMode;
    private final StringBuilder digits = new StringBuilder(6);
    private boolean requestInFlight;
    private int requestId;
    private int observedVehiclePhase = CONNECTING;
    private Boolean observedBcmLocked;
    private Boolean observedPasscodeWindow;

    int digitCount() { return digits.length(); }
    int requestId() { return requestId; }
    boolean requestInFlight() { return requestInFlight; }
    int displayPhase() {
        return requestInFlight && phase == LOCKED ? VERIFYING : phase;
    }

    boolean canEnter() {
        return phase == LOCKED && profileReady && attempts > 0 && !testMode && !requestInFlight;
    }

    /** Returns one six-digit request, preserving leading zeroes; never auto-retries. */
    String key(int key) {
        if (key == 11) {
            clear();
            return null;
        }
        if (!canEnter()) return null;
        error = NONE;
        if (key == 10) {
            if (digits.length() > 0) {
                digits.setCharAt(digits.length() - 1, '\0');
                digits.deleteCharAt(digits.length() - 1);
            }
        } else if (key >= 0 && key <= 9 && digits.length() < 6) {
            digits.append((char) ('0' + key));
            if (digits.length() == 6) {
                String pin = digits.toString();
                clear();
                phase = VERIFYING;
                requestInFlight = true;
                requestId++;
                return pin;
            }
        }
        return null;
    }

    void clear() {
        for (int i = 0; i < digits.length(); i++) digits.setCharAt(i, '\0');
        digits.setLength(0);
    }

    void locked() {
        clear();
        phase = LOCKED;
        error = NONE;
    }

    /** Mode reports take precedence over a delayed BCM lock report/replay. */
    boolean bcmLocked() {
        if (Boolean.TRUE.equals(observedBcmLocked)) return false;
        observedBcmLocked = true;
        if (observedVehiclePhase != PARKED && observedVehiclePhase != DRIVING
                && observedVehiclePhase != CHARGING) locked();
        return true;
    }

    boolean passcodeWindow(boolean active) {
        if (observedPasscodeWindow != null && observedPasscodeWindow == active) return false;
        observedPasscodeWindow = active;
        clear();
        if (active && (phase == CONNECTING || phase == LOCKED || phase == CHARGING)) locked();
        return true;
    }

    boolean unlocked() {
        if (Boolean.FALSE.equals(observedBcmLocked)) return false;
        observedBcmLocked = false;
        clear();
        if (phase != PARKED && phase != DRIVING && phase != CHARGING) phase = UNLOCKING;
        return true;
    }

    boolean vehiclePhase(int value) {
        if (observedVehiclePhase == value) return false;
        clear();
        observedVehiclePhase = value;
        phase = value;
        error = NONE;
        return true;
    }

    boolean finishRequest(int id) {
        if (!requestInFlight || id != requestId) return false;
        requestInFlight = false;
        return true;
    }

    void discardRequest(int id) {
        if (finishRequest(id) && phase == VERIFYING) locked();
    }

    void verified(int id, boolean valid, int remaining) {
        if (!finishRequest(id)) return;
        attempts = remaining;
        if (phase != VERIFYING && phase != LOCKED) return; // Preserve actual PARK/DRIVE feedback.
        phase = valid ? UNLOCKING : LOCKED;
        error = valid ? NONE : WRONG_PIN;
    }

    void failed() {
        clear();
        if (phase == VERIFYING) phase = LOCKED;
        error = SERVICE_ERROR;
        profileReady = false;
    }

    void unconfirmed() {
        if (phase == UNLOCKING) {
            phase = LOCKED;
            error = UNCONFIRMED;
        }
    }

    void disconnected() {
        clear();
        phase = CONNECTING;
        error = NONE;
        observedVehiclePhase = CONNECTING;
        observedBcmLocked = null;
        observedPasscodeWindow = null;
    }
}
