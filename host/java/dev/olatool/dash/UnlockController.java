package dev.olatool.dash;

import android.content.BroadcastReceiver;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.ServiceConnection;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.IBinder;
import android.os.Looper;
import com.olaelectric.vehicleinterface.VehicleLockStateBCM;
import com.olaelectric.vehicleinterface.VehicleStates;

/** Main-thread UI state; blocking PMS calls use a dedicated worker, never keep-alive's thread. */
final class UnlockController {
    interface Output { void set(int field, double value); }
    private final Context context;
    private final Output output;
    private final Handler main = new Handler(Looper.getMainLooper());
    private final HandlerThread thread = new HandlerThread("OlaDashPasscode");
    private final UnlockSession session = new UnlockSession();
    private Handler background;
    private ProfileService service;
    private volatile boolean stopped;
    private boolean bound, refreshing;
    private volatile int generation;
    private long cooldownUntil;
    private static final long RETRY_MS = 3000, UNLOCK_TIMEOUT_MS = 10_000;

    UnlockController(Context context, Output output) {
        this.context = context;
        this.output = output;
    }

    void start() {
        thread.start();
        background = new Handler(thread.getLooper());
        context.registerReceiver(screen, new IntentFilter(Intent.ACTION_SCREEN_OFF));
        bind();
        publish();
        main.post(tick);
    }

    void stop() {
        stopped = true;
        generation++;
        session.clear();
        main.removeCallbacksAndMessages(null);
        background.removeCallbacksAndMessages(null);
        context.unregisterReceiver(screen);
        if (bound) context.unbindService(connection);
        thread.quitSafely();
    }

    private void bind() {
        if (stopped || bound) return;
        try {
            bound = context.bindService(new Intent().setComponent(
                    new ComponentName(ProfileService.PACKAGE, ProfileService.SERVICE)),
                    connection, Context.BIND_AUTO_CREATE);
        } catch (SecurityException | IllegalArgumentException e) {
            bound = false;
        }
        if (!bound) {
            session.failed();
            publish();
            main.postDelayed(this::bind, RETRY_MS);
        }
    }

    private final ServiceConnection connection = new ServiceConnection() {
        @Override public void onServiceConnected(ComponentName name, IBinder binder) {
            service = new ProfileService(binder);
            Trace.event("profile service connected");
            refresh();
        }
        @Override public void onServiceDisconnected(ComponentName name) {
            generation++;
            refreshing = false;
            service = null;
            session.failed();
            publish();
            Trace.event("profile service disconnected");
        }
        @Override public void onBindingDied(ComponentName name) { reconnect(); }
        @Override public void onNullBinding(ComponentName name) { reconnect(); }
    };

    private void reconnect() {
        generation++;
        refreshing = false;
        service = null;
        session.failed();
        publish();
        if (bound) context.unbindService(connection);
        bound = false;
        main.postDelayed(this::bind, RETRY_MS);
    }

    void key(int key) {
        main.post(() -> {
            if (stopped) return;
            if (key == 12 && !session.requestInFlight()) {
                if (service == null) bind();
                refresh();
                return;
            }
            String pin = session.key(key);
            publish();
            if (pin == null) return;
            ProfileService target = service;
            if (target == null) {
                session.finishRequest(session.requestId());
                session.failed();
                publish();
                return;
            }
            int requestGeneration = generation;
            int requestId = session.requestId();
            Trace.event("passcode verification requested"); // Never log PIN or individual keys.
            main.removeCallbacks(verificationSlow);
            main.postDelayed(verificationSlow, UNLOCK_TIMEOUT_MS);
            background.post(() -> {
                if (stopped || generation != requestGeneration) {
                    main.post(() -> {
                        if (!stopped) { session.discardRequest(requestId); publish(); }
                    });
                    return;
                }
                try {
                    ProfileService.Result result = target.unlock(pin);
                    main.post(() -> {
                        if (stopped) return;
                        if (generation != requestGeneration) {
                            session.discardRequest(requestId);
                            publish();
                            return;
                        }
                        if (!session.requestInFlight() || session.requestId() != requestId) return;
                        main.removeCallbacks(verificationSlow);
                        session.verified(requestId, result.valid, result.attempts);
                        Trace.event("passcode verification " + (result.valid ? "accepted" : "rejected"));
                        if (session.phase == UnlockSession.UNLOCKING) waitForUnlock();
                        refresh();
                        publish();
                    });
                } catch (Exception e) {
                    main.post(() -> {
                        if (stopped) return;
                        if (generation != requestGeneration) {
                            session.discardRequest(requestId);
                            publish();
                            return;
                        }
                        if (!session.requestInFlight() || session.requestId() != requestId) return;
                        main.removeCallbacks(verificationSlow);
                        // Do not log exception messages: vendor code may include credentials.
                        Trace.event("passcode service call failed");
                        if (session.finishRequest(requestId)) session.failed();
                        publish();
                    });
                }
            });
        });
    }

    private void refresh() {
        if (stopped || service == null || refreshing || session.requestInFlight()) return;
        refreshing = true;
        int requestGeneration = generation;
        ProfileService target = service;
        background.post(() -> {
            try {
                int attempts = target.remainingAttempts();
                long firstTry = attempts <= 0 ? target.firstTryTimestamp() : 0;
                main.post(() -> {
                    if (stopped || generation != requestGeneration) return;
                    refreshing = false;
                    session.profileReady = true;
                    if (session.error == UnlockSession.SERVICE_ERROR) session.error = UnlockSession.NONE;
                    session.attempts = attempts;
                    // Stock UI uses a five-minute interval from PMS's first-try timestamp.
                    cooldownUntil = attempts <= 0 && firstTry > 0 ? firstTry + 300_000 : 0;
                    publish();
                });
            } catch (Exception e) {
                main.post(() -> {
                    if (stopped || generation != requestGeneration) return;
                    refreshing = false;
                    session.failed();
                    publish();
                });
            }
        });
    }

    private void waitForUnlock() {
        main.removeCallbacks(unlockTimeout);
        main.postDelayed(unlockTimeout, UNLOCK_TIMEOUT_MS);
    }

    private final Runnable unlockTimeout = () -> {
        if (session.phase != UnlockSession.UNLOCKING) return;
        session.unconfirmed();
        Trace.event("unlock confirmation timed out");
        publish();
    };

    // A slow Binder call cannot safely be retried: it may still unlock the vehicle.
    private final Runnable verificationSlow = () -> {
        if (session.requestInFlight()) {
            session.error = UnlockSession.SERVICE_ERROR;
            publish();
        }
    };

    void lockState(VehicleLockStateBCM state) {
        main.post(() -> {
            if (stopped || session.testMode || state == null) return;
            if (state == VehicleLockStateBCM.Vehicle_Locked) {
                if (!session.bcmLocked()) return;
                main.removeCallbacks(unlockTimeout);
                refresh();
            } else if (state == VehicleLockStateBCM.Vehicle_Unlocked) {
                if (!session.unlocked()) return;
                if (session.phase == UnlockSession.UNLOCKING) waitForUnlock();
            } else if (state == VehicleLockStateBCM.Vehicle_Unlock_Fail) {
                session.unconfirmed();
            }
            publish();
        });
    }

    void vehicleState(VehicleStates state) {
        main.post(() -> {
            if (stopped || session.testMode || state == null) return;
            switch (state) {
                case PARK: session.vehiclePhase(UnlockSession.PARKED); break;
                case DRIVE: session.vehiclePhase(UnlockSession.DRIVING); break;
                case BC_CHARGE_ACTIVE:
                case SC_CHARGE_ACTIVE:
                case FC_CHARGE_ACTIVE: session.vehiclePhase(UnlockSession.CHARGING); break;
                case SLEEP:
                case VEHICLE_IDLE:
                case VACATION: session.vehiclePhase(UnlockSession.LOCKED); break;
                default: return;
            }
            if (session.phase != UnlockSession.UNLOCKING) main.removeCallbacks(unlockTimeout);
            publish();
        });
    }

    void passcodeWindow(boolean active) {
        main.post(() -> {
            if (stopped || session.testMode) return;
            if (!session.passcodeWindow(active)) return;
            if (active) refresh();
            publish();
        });
    }

    void parserDisconnected() {
        main.post(() -> {
            if (stopped) return;
            generation++;
            refreshing = false;
            session.disconnected();
            main.removeCallbacks(unlockTimeout);
            main.removeCallbacks(verificationSlow);
            publish();
        });
    }

    void testMode(boolean enabled) {
        main.post(() -> {
            if (stopped) return;
            if (enabled) {
                generation++;
                refreshing = false;
                if (session.phase == UnlockSession.VERIFYING) session.locked();
                main.removeCallbacks(verificationSlow);
            }
            session.testMode = enabled;
            session.clear();
            publish();
        });
    }

    private final BroadcastReceiver screen = new BroadcastReceiver() {
        @Override public void onReceive(Context c, Intent i) {
            session.clear();
            publish();
        }
    };

    private final Runnable tick = new Runnable() {
        @Override public void run() {
            if (stopped) return;
            session.cooldownSeconds = (int) Math.max(0, Math.min(300,
                    (cooldownUntil - System.currentTimeMillis() + 999) / 1000));
            if ((session.phase == UnlockSession.LOCKED || session.phase == UnlockSession.CONNECTING) && !refreshing
                    && (!session.profileReady || session.attempts <= 0)) refresh();
            publish();
            main.postDelayed(this, 1000);
        }
    };

    private void publish() {
        output.set(VehicleBridge.AUTH_PHASE, session.displayPhase());
        output.set(VehicleBridge.PIN_LENGTH, session.digitCount());
        output.set(VehicleBridge.AUTH_ATTEMPTS, session.attempts);
        output.set(VehicleBridge.AUTH_ERROR, session.error);
        output.set(VehicleBridge.AUTH_COOLDOWN, session.cooldownSeconds);
        output.set(VehicleBridge.AUTH_AVAILABLE, session.canEnter() ? 1 : 0);
    }
}
