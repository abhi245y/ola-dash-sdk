package dev.olatool.dash;

import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.ServiceConnection;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.IBinder;
import android.os.PowerManager;
import android.os.RemoteException;
import android.util.Log;

import java.lang.reflect.Method;

import com.olaelectric.vehicleinterface.AccidentSeverityStatus;
import com.olaelectric.vehicleinterface.ChargingStatus;
import com.olaelectric.vehicleinterface.CommandFromVehicle;
import com.olaelectric.vehicleinterface.DriveModes;
import com.olaelectric.vehicleinterface.FallDetection;
import com.olaelectric.vehicleinterface.*;
import com.olaelectric.vehicleinterface.IAlertInfoCallbacks;
import com.olaelectric.vehicleinterface.IBatteryInfoCallbacks;
import com.olaelectric.vehicleinterface.IChargerInfoCallbacks;
import com.olaelectric.vehicleinterface.ICommanderService;
import com.olaelectric.vehicleinterface.IHandleBarCallbacks;
import com.olaelectric.vehicleinterface.IHardwareVersionInfoCallbacks;
import com.olaelectric.vehicleinterface.IImuInfoCallbacks;
import com.olaelectric.vehicleinterface.IMapInfoCallbacks;
import com.olaelectric.vehicleinterface.IDisplayInfoCallbacks;
import com.olaelectric.vehicleinterface.IMotorInfoCallbacks;
import com.olaelectric.vehicleinterface.IParserService;
import com.olaelectric.vehicleinterface.IProximityCallbacks;
import com.olaelectric.vehicleinterface.IVehicleCommandCallbacks;
import com.olaelectric.vehicleinterface.IVehicleInfoCallbacks;
import com.olaelectric.vehicleinterface.IVehicleModeCallbacks;
import com.olaelectric.vehicleinterface.IVehicleService;
import com.olaelectric.vehicleinterface.IVehicleServiceInfoCallbacks;
import com.olaelectric.vehicleinterface.IWidgetInfoCallbacks;
import com.olaelectric.vehicleinterface.IndicatorFault;
import com.olaelectric.vehicleinterface.RegenStatus;
import com.olaelectric.vehicleinterface.TowTheftDetection;
import com.olaelectric.vehicleinterface.VehicleLockStateBCM;
import com.olaelectric.vehicleinterface.VehicleStates;

import dev.olatool.dash.plugin.PluginHost;

/**
 * Binds the stock vehicle parser and pushes decoded
 * values into the Rust UI. Field ids must match host/src/lib.rs.
 */
final class VehicleBridge {
    static {
        System.loadLibrary("oladash");
    }

    private static final String TAG = "OlaDash";
    private static final String PARSER_PKG = "com.olaelectric.vehicleparser";
    private static final String PARSER_CLASS = "com.olaelectric.vehicleparser.services.VehicleParserService";
    private static final String PARSER_ACTION = "com.olaelectric.vehicleparser.VehicleParserService";
    private static final long RETRY_MS = 3000L;

    static final int SPEED_KMH = 1, BATTERY_PCT = 2, RANGE_KM = 3, MODE = 4, CHARGING = 5,
            ODO_KM = 6, TRIP_KM = 7, LEFT_IND = 8, RIGHT_IND = 9, HIGH_BEAM = 10, HEADLIGHT = 11,
            LOCKED = 12, READY = 13, HYPER_AVAILABLE = 14, ALERT_CODE = 20, SIDE_STAND = 21, FAULT = 22, CLOCK_HHMM = 30,
            TEST_MODE = 31, AUTH_PHASE = 32, PIN_LENGTH = 33, AUTH_ATTEMPTS = 34,
            AUTH_ERROR = 35, AUTH_COOLDOWN = 36, AUTH_AVAILABLE = 37, DATA_STALE = 38, PARKED = 39;
    // Contract v5. Ids match host/src/lib.rs. Unreported values are -1.
    static final int VEHICLE_STATE = 15, SOC_PCT = 40, BAT_CURRENT = 41, BUS_VOLTAGE = 42,
            TEMP_MIN = 43, TEMP_MAX = 44, HEALTH = 45, FRONT_BRAKE = 46, REAR_BRAKE = 47,
            REGEN = 48, MOTOR_TEMP = 49, MCU_TEMP = 50;

    // Contract v6. Trunk latch, from IDisplayInfoCallbacks.isTrunkLock (true = open).
    static final int TRUNK_OPEN = 16;

    // ponytail: fixed threshold for the low-battery popup. Calibrate against stock derate points.
    private static final int LOW_BATTERY_PCT = 10;
    // ponytail: blinker lamp pulses on CAN; hold "off" this long so the host reports steady on
    // while the stalk is active. Calibrate against the real blink period.
    private static final long BLINKER_HOLD_MS = 1000L;
    private static final Runnable LEFT_OFF = () -> setBool(LEFT_IND, false);
    private static final Runnable RIGHT_OFF = () -> setBool(RIGHT_IND, false);

    private static native void nativeSet(int id, double value);
    private static native boolean nativeTakeTouched();

    // Trip = odometer metres since the last unlock (onDistanceCovered is always 0 on Gen 1).
    private static long odoMeters = -1;
    private static long tripBaseMeters = -1;

    private static Context app;
    private static HandlerThread thread;
    private static Handler work;
    private static IBinder vehicle; // last connected binder, for resubscribe after a test run
    private static UnlockController unlockController;
    private static final TestEventGate TEST_EVENTS = new TestEventGate();

    private static boolean realVehicleEvent() {
        return !Trace.testMode && !TEST_EVENTS.isInjected();
    }

    // Keep-alive, same as the stock HMI's WakeLockNotifier: every second while the screen is
    // on, ICommanderService.sendTouchEvent(touched in the last second) -> CAN 0x643, so the
    // vehicle's own idle/auto-lock timers still run when nobody uses the screen.
    // Also true for media playing in PARK or charging, as the stock WakeLockNotifier does
    // (built-in media source only). ponytail: calls and pairing popups are not counted yet.
    // Touches inside other apps (Android Auto) are not seen.
    private static final long KEEPALIVE_MS = 1000L;
    private static volatile ICommanderService commander;
    private static volatile boolean unlocked; // real BCM state only, never test-injected
    private static Boolean lastKeepAlive;
    // Real values for TrunkGate (never test-injected). Null/-1 = not reported or lost on disconnect.
    private static volatile Boolean realBcmUnlocked;
    private static volatile Boolean realTrunkOpen;
    private static volatile int realSpeedKmh = -1;
    private static volatile long trunkSentAtMs = Long.MIN_VALUE / 2;
    private static String lastReason;
    private static boolean lastScreenOn;
    private static PowerManager power;
    private static final Runnable KEEPALIVE = new Runnable() {
        @Override
        public void run() {
            boolean screenOn = power.isInteractive();
            if (screenOn != lastScreenOn) {
                Trace.event("screen " + (screenOn ? "on" : "off"));
                lastScreenOn = screenOn;
            }
            ICommanderService c = commander;
            if (screenOn && c != null) {
                boolean touched = nativeTakeTouched();
                // Stock WakeLockNotifier also counts BT music while in PARK or charging. Only the
                // built-in source counts here, never a plugin's publishMedia.
                boolean media = !touched && PluginHost.stockPlaying()
                        && (stateIndex(lastState) == 3 || stateIndex(lastState) == 5);
                boolean v = touched || media;
                String reason = touched ? "touch" : media ? "media" : "none";
                try {
                    c.sendTouchEvent(v);
                    if (lastKeepAlive == null || lastKeepAlive != v || !reason.equals(lastReason)) {
                        Trace.event("keep-alive " + v + " reason=" + reason);
                    }
                    lastKeepAlive = v;
                    lastReason = reason;
                } catch (RemoteException e) {
                    Trace.event("keep-alive failed " + e);
                }
            }
            pushStale();
            Trace.flush();
            work.postDelayed(this, KEEPALIVE_MS);
        }
    };

    // Alert state. Guarded by LOCK; callbacks arrive on binder threads.
    private static final Object LOCK = new Object();
    private static int soc = -1;
    private static int faultBits;          // bit0..2 = TYPE1..3 errors, bit3 = brake worn
    private static boolean sideStandAlert, steeringAlert, fallActive, towActive, accidentActive;

    private VehicleBridge() {
    }

    static void start(Context ctx) {
        app = ctx.getApplicationContext();
        thread = new HandlerThread("OlaDashVehicle");
        thread.start();
        work = new Handler(thread.getLooper());
        power = (PowerManager) app.getSystemService(Context.POWER_SERVICE);
        setBool(HYPER_AVAILABLE, app.getSharedPreferences("dash", 0).getBoolean("hyper", true));
        Trace.start(app);
        unlockController = new UnlockController(app, VehicleBridge::nativeSet);
        unlockController.start();
        work.post(VehicleBridge::connect);
        work.post(KEEPALIVE);
    }

    static void stop() {
        if (work == null) {
            return;
        }
        unlockController.stop();
        work.post(() -> {
            try {
                app.unbindService(conn);
            } catch (IllegalArgumentException ignored) {
                // Not bound.
            }
        });
        work.removeCallbacks(KEEPALIVE);
        work.post(Trace::stop);
        thread.quitSafely();
    }

    static void setClock(int hour, int minute) {
        nativeSet(CLOCK_HHMM, hour * 100 + minute);
    }

    /**
     * Host-gated trunk request: sends the stock HMI's trunk command (Seat_Unlock_Cmd) once,
     * if TrunkGate allows it. Runs on the vehicle thread; every request is logged.
     */
    static void openTrunk() {
        if (work == null) {
            Trace.event("trunk request: refused not started");
            return;
        }
        work.post(() -> {
            long now = System.currentTimeMillis();
            ICommanderService c = commander;
            String why = TrunkGate.refusal(Trace.testMode, c != null, realBcmUnlocked,
                    stateIndex(lastState), realSpeedKmh, realTrunkOpen, now - trunkSentAtMs);
            if (why != null) {
                Trace.event("trunk request: refused " + why);
                return;
            }
            try {
                // One send. The stock HHMIS resends until acked; v1 does not.
                c.sendCommandToScooter(CommandFromHMI.Seat_Unlock_Cmd);
                trunkSentAtMs = now;
                Trace.event("trunk request: sent");
            } catch (RemoteException e) {
                Trace.event("trunk request: refused binder " + e);
            }
        });
    }

    private static void forgetGateState() {
        realBcmUnlocked = null;
        realTrunkOpen = null;
        realSpeedKmh = -1;
        // Also fail closed on state: tripOnState then skips one unlock-based trip reset after a
        // parser restart or a test session, and the keep-alive media count stops until a state arrives.
        lastState = null;
        pushParked();
    }

    private static boolean lastParked;

    /** HostState.parked from real values only. Direct nativeSet: never replayed from REAL. */
    private static synchronized void pushParked() {
        boolean p = TrunkGate.parked(Trace.testMode, realBcmUnlocked,
                stateIndex(lastState), realSpeedKmh);
        if (lastParked && !p && MainActivity.paused) Trace.event("parked -> false while paused");
        lastParked = p;
        nativeSet(PARKED, p ? 1 : 0);
    }

    // ponytail: 5 s until M-rate (session A) calibrates it
    private static final long STALE_MS = 5000L;
    private static volatile long lastCallbackMs = System.currentTimeMillis();
    private static volatile boolean disconnected;
    private static boolean lastStale;

    /** HostState.data-stale: parser gone, or no real callback for STALE_MS while driving. Direct nativeSet: id 38 is not replayed from REAL. */
    private static synchronized void pushStale() {
        boolean s = disconnected || (stateIndex(lastState) == 4
                && System.currentTimeMillis() - lastCallbackMs > STALE_MS);
        if (s != lastStale) {
            lastStale = s;
            Trace.event("data-stale " + s);
            nativeSet(DATA_STALE, s ? 1 : 0);
        }
    }

    static void unlockKey(int key) {
        if (unlockController != null && !Trace.testMode) unlockController.key(key);
    }

    private static void connect() {
        Intent i = new Intent(PARSER_ACTION).setComponent(new ComponentName(PARSER_PKG, PARSER_CLASS));
        boolean ok = app.bindService(i, conn, Context.BIND_AUTO_CREATE);
        Log.i(TAG, "bind vehicle parser ok=" + ok);
        if (!ok) {
            work.postDelayed(VehicleBridge::connect, RETRY_MS);
        }
    }

    private static void rebind(long delayMs) {
        work.postDelayed(() -> {
            try {
                app.unbindService(conn);
            } catch (IllegalArgumentException ignored) {
                // Not bound.
            }
            connect();
        }, delayMs);
    }

    private static void subscribe(IBinder vehicleBinder) {
        vehicle = vehicleBinder;
        disconnected = false;
        lastCallbackMs = System.currentTimeMillis();
        pushStale();
        try {
            IVehicleService vs = IVehicleService.Stub.asInterface(vehicleBinder);
            IParserService parser = vs.getParserServiceBinder();
            commander = vs.getCommanderServiceBinder();
            // The parser keys subscriptions by this string. Use our own package name.
            String key = app.getPackageName();
            Log.i(TAG, "subscribe as " + key
                    + " display=" + parser.subscribeDisplayInfo(key, DISPLAY)
                    + " motor=" + parser.subscribeMotorInfo(key, MOTOR)
                    + " modes=" + parser.subscribeVehicleModes(key, MODES)
                    + " alerts=" + parser.subscribeAlertInfo(key, ALERTS)
                    + " commands=" + parser.subscribeVehicleCommand(key, COMMANDS)
                    + " info=" + parser.subscribeVehicleInfo(key, INFO));
            // Log-only subscriptions: every callback lands in the calls log (Trace).
            parser.subscribeBatteryInfo(key, BATTERY);
            parser.subscribeChargerInfo(key, CHARGER);
            parser.subscribeHandleBar(key, HANDLEBAR);
            parser.subscribeHardwareVersionInfo(key, new HardwareVersionInfoCallbacksBase() { });
            parser.subscribeIMUInfo(key, new ImuInfoCallbacksBase() { });
            parser.subscribeMapInfo(key, new MapInfoCallbacksBase() { });
            parser.subscribeProximityInfo(key, new ProximityCallbacksBase() { });
            parser.subscribeVehicleServiceInfo(key, new VehicleServiceInfoCallbacksBase() { });
            parser.subscribeWidgetData(key, new WidgetInfoCallbacksBase() { });
            Trace.event("subscribed");
        } catch (RemoteException e) {
            Log.w(TAG, "subscribe failed, retrying", e);
            rebind(RETRY_MS);
        }
    }

    /**
     * Debug test channel: runs one parser callback as if the vehicle had sent it, so
     * the same bridge logic (alerts, blinker hold, mode mapping) is exercised.
     * target: display|motor|modes|alerts|info|commands|battery|charger|handlebar. Args are parsed by parameter
     * type (int, long, float, boolean, enum constant name); missing args are null.
     */
    static void inject(String target, String method, String[] args) {
        Object cb;
        switch (target == null ? "" : target) {
            case "display": cb = DISPLAY; break;
            case "motor": cb = MOTOR; break;
            case "modes": cb = MODES; break;
            case "alerts": cb = ALERTS; break;
            case "info": cb = INFO; break;
            case "commands": cb = COMMANDS; break;
            case "battery": cb = BATTERY; break;
            case "charger": cb = CHARGER; break;
            case "handlebar": cb = HANDLEBAR; break;
            default: Log.w(TAG, "test: unknown target " + target); return;
        }
        String[] a = args == null ? new String[0] : args;
        for (Method m : cb.getClass().getMethods()) {
            if (!m.getName().equals(method)) {
                continue;
            }
            Class<?>[] types = m.getParameterTypes();
            Object[] vals = new Object[types.length];
            try {
                for (int i = 0; i < types.length; i++) {
                    vals[i] = i < a.length ? parse(types[i], a[i]) : null;
                }
                int token = TEST_EVENTS.token();
                work.post(() -> TEST_EVENTS.dispatch(token, () -> {
                    try {
                        m.invoke(cb, vals);
                    } catch (ReflectiveOperationException e) {
                        Log.w(TAG, "test: " + method + " failed", e);
                    }
                }));
                Log.i(TAG, "test: " + target + "." + method + " " + java.util.Arrays.toString(a));
            } catch (RuntimeException e) {
                Log.w(TAG, "test: bad args for " + method + ": " + e);
            }
            return;
        }
        Log.w(TAG, "test: no method " + target + "." + method);
    }

    @SuppressWarnings({"unchecked", "rawtypes"})
    private static Object parse(Class<?> t, String v) {
        if (t == int.class) return Integer.parseInt(v);
        if (t == long.class) return Long.parseLong(v);
        if (t == float.class) return Float.parseFloat(v);
        if (t == double.class) return Double.parseDouble(v);
        if (t == boolean.class) return Boolean.parseBoolean(v);
        if (t.isEnum()) return Enum.valueOf((Class<Enum>) t, v);
        return null;
    }

    static void setTestMode(boolean on) {
        // The same monitor covers injected dispatch, so TEST_OFF cannot reset the
        // real UI while an old synthetic callback is still writing into it.
        synchronized (TEST_EVENTS) {
            setTestModeLocked(on);
        }
    }

    private static void setTestModeLocked(boolean on) {
        TEST_EVENTS.setActive(on);
        Trace.testMode = on;
        // Real inputs freeze while test mode is on; forget them so exit waits for the parser replay.
        if (on) forgetGateState(); else pushParked();
        unlockController.testMode(on);
        setBool(TEST_MODE, on);
        if (on) {
            return;
        }
        // Drop every injected value, then let the parser replay what it really has.
        for (int id : new int[] {SPEED_KMH, CHARGING, ODO_KM, TRIP_KM,
                LEFT_IND, RIGHT_IND, HIGH_BEAM, HEADLIGHT, LOCKED, READY, SIDE_STAND,
                BAT_CURRENT, FRONT_BRAKE, REAR_BRAKE, REGEN, VEHICLE_STATE, TRUNK_OPEN}) {
            nativeSet(id, 0);
        }
        for (int id : new int[] {BATTERY_PCT, RANGE_KM, SOC_PCT, BUS_VOLTAGE, TEMP_MIN, TEMP_MAX,
                HEALTH, MOTOR_TEMP, MCU_TEMP}) {
            nativeSet(id, -1);
        }
        mcuOff = false;
        nativeSet(MODE, 4);
        for (int id = 0; id < REAL.length; id++) {
            // Vehicle data only: 30-39 are clock, test and unlock-UI slots.
            boolean vehicleData = id < CLOCK_HHMM || id >= SOC_PCT;
            if (vehicleData && !Double.isNaN(REAL[id])) {
                nativeSet(id, REAL[id]);
            }
        }
        setBool(HYPER_AVAILABLE, app.getSharedPreferences("dash", 0).getBoolean("hyper", true));
        synchronized (LOCK) {
            soc = -1;
            faultBits = 0;
            sideStandAlert = steeringAlert = fallActive = towActive = accidentActive = false;
            pushAlerts();
        }
        if (vehicle != null) {
            // Ask the parser to replay its cached real values over the injected ones.
            work.post(() -> subscribe(vehicle));
        }
    }

    private static final ServiceConnection conn = new ServiceConnection() {
        @Override
        public void onServiceConnected(ComponentName name, IBinder service) {
            Log.i(TAG, "connected " + name);
            work.post(() -> subscribe(service));
        }

        @Override
        public void onServiceDisconnected(ComponentName name) {
            Log.w(TAG, "vehicle parser disconnected");
            vehicle = null;
            commander = null;
            unlocked = false;
            disconnected = true;
            forgetGateState();
            pushStale();
            unlockController.parserDisconnected();
            rebind(RETRY_MS);
        }

        @Override
        public void onBindingDied(ComponentName name) {
            Log.w(TAG, "vehicle parser binding died");
            vehicle = null;
            commander = null;
            unlocked = false;
            disconnected = true;
            forgetGateState();
            pushStale();
            unlockController.parserDisconnected();
            rebind(RETRY_MS);
        }
    };

    // Last value the real vehicle sent per slot (NaN = never). Restored when test mode ends,
    // because the parser does not replay every field (SOC, range) on re-subscribe.
    private static final double[] REAL = new double[64];
    static {
        java.util.Arrays.fill(REAL, Double.NaN);
    }

    private static void setNum(int id, double v) {
        if (realVehicleEvent()) {
            REAL[id] = v;
            lastCallbackMs = System.currentTimeMillis();
        }
        nativeSet(id, v);
    }

    private static void setBool(int id, boolean v) {
        setNum(id, v ? 1 : 0);
    }

    /** VehicleState index for the Vehicle.state enum (contract v5). */
    private static int stateIndex(VehicleStates s) {
        if (s == null) {
            return 0;
        }
        switch (s) {
            case SLEEP: return 1;
            case VEHICLE_IDLE: return 2;
            case PARK: return 3;
            case DRIVE: return 4;
            case BC_CHARGE_ACTIVE:
            case SC_CHARGE_ACTIVE:
            case FC_CHARGE_ACTIVE: return 5;
            default: return 0;
        }
    }

    /** Ride.regen index: none / coast / brake. Forced regen counts as its non-forced kind. */
    private static int regenIndex(RegenStatus r) {
        if (r == null) {
            return 0;
        }
        switch (r) {
            case Coast_Regen:
            case Coast_Forced_Regen: return 1;
            case Coast_Brake_Regen:
            case Coast_Forced_Brake_Regen: return 2;
            default: return 0;
        }
    }

    private static int modeIndex(DriveModes m) {
        if (m == null) {
            return 4;
        }
        switch (m) {
            case ECO: return 0;
            case LIMPHOME: return 0; // Gen 1 reports Eco as LIMPHOME (rider-confirmed)
            case NORMAL: return 1;
            case SPORT: return 2;
            case HYPER: return 3;
            case PARK: return 5;
            case REVERSE: return 6;
            // Easy Park (slow forward + reverse). Plain reverse is REVERSE. Ride log 2026-10-30
            // saw only PARK_ASSIST; REVERSE mapping not yet seen on the bus.
            case PARK_ASSIST: return 8;
            case CUSTOM: return 7;
            default: return 4;
        }
    }

    /** Recomputes the top-priority alert and the fault telltale. Call with LOCK held. */
    private static void pushAlerts() {
        int code;
        if (accidentActive || fallActive) {
            code = 1;
        } else if (towActive) {
            code = 2;
        } else if (sideStandAlert) {
            code = 3;
        } else if (steeringAlert) {
            code = 4;
        } else if (faultBits != 0) {
            code = 5;
        } else if (soc >= 0 && soc <= LOW_BATTERY_PCT) {
            code = 6;
        } else {
            code = 0;
        }
        nativeSet(ALERT_CODE, code);
        nativeSet(FAULT, faultBits != 0 ? 1 : 0);
    }

    /** Steady-on indicator: a false is applied only after the hold window with no true. */
    private static void indicator(int id, boolean on) {
        Runnable off = id == LEFT_IND ? LEFT_OFF : RIGHT_OFF;
        work.removeCallbacks(off);
        if (on) {
            setBool(id, true);
        } else {
            work.postDelayed(off, BLINKER_HOLD_MS);
        }
    }

    private static void setFault(int bit, boolean on) {
        synchronized (LOCK) {
            faultBits = on ? (faultBits | (1 << bit)) : (faultBits & ~(1 << bit));
            pushAlerts();
        }
    }

    private static final IDisplayInfoCallbacks DISPLAY = new DisplayInfoCallbacksBase() {
        @Override
        public void isHighBeamOn(boolean on) {
            Trace.call("IDisplayInfoCallbacks.isHighBeamOn", on);
            setBool(HIGH_BEAM, on);
        }

        @Override
        public void isLowBeamOn(boolean on) {
            Trace.call("IDisplayInfoCallbacks.isLowBeamOn", on);
            setBool(HEADLIGHT, on);
        }

        @Override
        public void isLeftIndicatorOn(boolean on, IndicatorFault fault) {
            Trace.call("IDisplayInfoCallbacks.isLeftIndicatorOn", on, fault);
            indicator(LEFT_IND, on);
        }

        @Override
        public void isRightIndicatorOn(boolean on, IndicatorFault fault) {
            Trace.call("IDisplayInfoCallbacks.isRightIndicatorOn", on, fault);
            indicator(RIGHT_IND, on);
        }

        @Override
        public void isSideStandOn(boolean on) {
            Trace.call("IDisplayInfoCallbacks.isSideStandOn", on);
            setBool(SIDE_STAND, on);
        }

        @Override
        public void isTrunkLock(boolean locked) {
            Trace.call("IDisplayInfoCallbacks.isTrunkLock", locked);
            if (realVehicleEvent()) realTrunkOpen = !locked;
            setBool(TRUNK_OPEN, !locked);
        }

        @Override
        public void onBatterySocChanged(int pct) {
            Trace.call("IDisplayInfoCallbacks.onBatterySocChanged", pct);
            setNum(BATTERY_PCT, pct);
            synchronized (LOCK) {
                soc = pct;
                pushAlerts();
            }
        }

        @Override
        public void onBrakeWornOutAlert(boolean on) {
            Trace.call("IDisplayInfoCallbacks.onBrakeWornOutAlert", on);
            setFault(3, on);
        }

        @Override
        public void onChargingStatusChanged(ChargingStatus status) {
            Trace.call("IDisplayInfoCallbacks.onChargingStatusChanged", status);
            setBool(CHARGING, status == ChargingStatus.ON_IN_PROGRESS);
        }

        @Override
        public void onOdoValueChangedInMeters(long meters) {
            Trace.call("IDisplayInfoCallbacks.onOdoValueChangedInMeters", meters);
            setNum(ODO_KM, meters / 1000.0);
            odoMeters = meters;
            if (tripBaseMeters < 0) {
                tripBaseMeters = app.getSharedPreferences("dash", 0).getLong("tripBase", meters);
            }
            setNum(TRIP_KM, Math.max(0, meters - tripBaseMeters) / 1000.0);
        }

        @Override
        public void onMcuSupplyTurnedOn(boolean on) {
            Trace.call("IDisplayInfoCallbacks.onMcuSupplyTurnedOn", on);
            // MCU temperatures read 0.0 while the MCU is off: publish -1 instead.
            mcuOff = !on;
            if (mcuOff) {
                setNum(MOTOR_TEMP, -1);
                setNum(MCU_TEMP, -1);
            }
        }

        @Override
        public void onRangeChanged(int km) {
            Trace.call("IDisplayInfoCallbacks.onRangeChanged", km);
            setNum(RANGE_KM, km);
        }
    };

    private static final IMotorInfoCallbacks MOTOR = new MotorInfoCallbacksBase() {
        @Override
        public void onSpeedChanged(int kmh) {
            Trace.call("IMotorInfoCallbacks.onSpeedChanged", kmh);
            if (realVehicleEvent()) realSpeedKmh = kmh;
            setNum(SPEED_KMH, kmh);
            pushParked();
        }

        @Override
        public void onVehicleDischargeModeChanged(DriveModes mode) {
            Trace.call("IMotorInfoCallbacks.onVehicleDischargeModeChanged", mode);
            // ponytail: discharge mode from the MCU (0x113). Adaptive/boost/park map to "unknown".
            setNum(MODE, modeIndex(mode));
        }

        @Override
        public void onMotorTemperatureChanged(float c) {
            Trace.call("IMotorInfoCallbacks.onMotorTemperatureChanged", c);
            setNum(MOTOR_TEMP, mcuOff ? -1 : c);
        }

        @Override
        public void onMcuTemperatureChanged(float c) {
            Trace.call("IMotorInfoCallbacks.onMcuTemperatureChanged", c);
            setNum(MCU_TEMP, mcuOff ? -1 : c);
        }

        @Override
        public void onRegenNotificatioReceived(RegenStatus status) {
            Trace.call("IMotorInfoCallbacks.onRegenNotificatioReceived", status);
            setNum(REGEN, regenIndex(status));
        }

        @Override
        public void onDistanceCovered(int distance) {
            Trace.call("IMotorInfoCallbacks.onDistanceCovered", distance);
            // Always 0 on Gen 1 (ride log 2026-10-30); trip comes from the odometer instead.
        }
    };

    private static final IVehicleModeCallbacks MODES = new VehicleModeCallbacksBase() {
        @Override
        public void onVehicleLockStateBCMChanged(VehicleLockStateBCM state) {
            Trace.call("IVehicleModeCallbacks.onVehicleLockStateBCMChanged", state);
            if (realVehicleEvent()) unlockController.lockState(state);
            if (state == VehicleLockStateBCM.Vehicle_Locked) {
                setBool(LOCKED, true);
            } else if (state == VehicleLockStateBCM.Vehicle_Unlocked) {
                setBool(LOCKED, false);
            }
            if (realVehicleEvent() && (state == VehicleLockStateBCM.Vehicle_Locked
                    || state == VehicleLockStateBCM.Vehicle_Unlocked)) {
                unlocked = state == VehicleLockStateBCM.Vehicle_Unlocked;
                realBcmUnlocked = unlocked;
            }
            pushParked();
        }

        @Override
        public void onVehicleStateChanged(VehicleStates state) {
            Trace.call("IVehicleModeCallbacks.onVehicleStateChanged", state);
            if (realVehicleEvent()) unlockController.vehicleState(state);
            // Ready to ride = DRIVE (ride log 2026-10-30: power button in PARK -> DRIVE).
            setBool(READY, state == VehicleStates.DRIVE);
            setNum(VEHICLE_STATE, stateIndex(state));
            if (realVehicleEvent()) {
                tripOnState(state);
            }
            // PARK/DRIVE only happen unlocked; covers an app start after the unlock event.
            if (realVehicleEvent() && (state == VehicleStates.PARK || state == VehicleStates.DRIVE)) {
                unlocked = true;
            } else if (realVehicleEvent() && (state == VehicleStates.SLEEP
                    || state == VehicleStates.VEHICLE_IDLE || state == VehicleStates.VACATION)) {
                unlocked = false;
            }
            pushParked();
        }
    };

    /** Hyper access is only reported when the mode button reaches it, so remember it. */
    private static void hyperAvailable(boolean on) {
        setBool(HYPER_AVAILABLE, on);
        if (realVehicleEvent()) {
            app.getSharedPreferences("dash", 0).edit().putBoolean("hyper", on).apply();
        }
    }

    private static volatile VehicleStates lastState;
    /** True while the parser reports the MCU supply off; MCU temperatures are then -1. */
    private static volatile boolean mcuOff;

    /** Unlock (SLEEP/VEHICLE_IDLE -> PARK) starts a new trip at the current odometer. */
    private static void tripOnState(VehicleStates state) {
        boolean unlock = state == VehicleStates.PARK
                && (lastState == VehicleStates.SLEEP || lastState == VehicleStates.VEHICLE_IDLE);
        lastState = state;
        if (unlock && odoMeters >= 0) {
            tripBaseMeters = odoMeters;
            app.getSharedPreferences("dash", 0).edit().putLong("tripBase", odoMeters).apply();
            setNum(TRIP_KM, 0);
            Trace.event("trip reset at " + odoMeters + " m");
        }
    }

    private static final IAlertInfoCallbacks ALERTS = new AlertInfoCallbacksBase() {
        @Override
        public void onSideStandPostionAlert(boolean on) {
            Trace.call("IAlertInfoCallbacks.onSideStandPostionAlert", on);
            synchronized (LOCK) {
                sideStandAlert = on;
                pushAlerts();
            }
        }

        @Override
        public void onSteeringLockUnLockAlert(boolean on) {
            Trace.call("IAlertInfoCallbacks.onSteeringLockUnLockAlert", on);
            synchronized (LOCK) {
                steeringAlert = on;
                pushAlerts();
            }
        }

        @Override
        public void onFallDetectionAlert(FallDetection state) {
            Trace.call("IAlertInfoCallbacks.onFallDetectionAlert", state);
            synchronized (LOCK) {
                fallActive = state == FallDetection.ACTIVE;
                pushAlerts();
            }
        }

        @Override
        public void onTowTheftDetectionAlert(TowTheftDetection state) {
            Trace.call("IAlertInfoCallbacks.onTowTheftDetectionAlert", state);
            synchronized (LOCK) {
                towActive = state == TowTheftDetection.ACTIVE;
                pushAlerts();
            }
        }
    };

    private static final IVehicleInfoCallbacks INFO = new VehicleInfoCallbacksBase() {
        @Override
        public void isFrontBrakeApplied(boolean on) {
            Trace.call("IVehicleInfoCallbacks.isFrontBrakeApplied", on);
            setBool(FRONT_BRAKE, on);
        }

        @Override
        public void isRearBrakeApplied(boolean on) {
            Trace.call("IVehicleInfoCallbacks.isRearBrakeApplied", on);
            setBool(REAR_BRAKE, on);
        }

        @Override
        public void onAccidentSeverityStatus(AccidentSeverityStatus status) {
            Trace.call("IVehicleInfoCallbacks.onAccidentSeverityStatus", status);
            synchronized (LOCK) {
                accidentActive = status != null && status != AccidentSeverityStatus.NO_ACCIDENT;
                pushAlerts();
            }
        }
    };

    private static final IBatteryInfoCallbacks BATTERY = new BatteryInfoCallbacksBase() {
        @Override
        public void onOverallSocChanged(float pct) {
            Trace.call("IBatteryInfoCallbacks.onOverallSocChanged", pct);
            setNum(SOC_PCT, pct);
        }

        @Override
        public void onBatteryCurrent(float amps) {
            Trace.call("IBatteryInfoCallbacks.onBatteryCurrent", amps);
            setNum(BAT_CURRENT, amps);
        }

        @Override
        public void onBusVoltage(float volts) {
            Trace.call("IBatteryInfoCallbacks.onBusVoltage", volts);
            setNum(BUS_VOLTAGE, volts);
        }

        @Override
        public void onMinBatteryTemperatureChanged(float c) {
            Trace.call("IBatteryInfoCallbacks.onMinBatteryTemperatureChanged", c);
            setNum(TEMP_MIN, c);
        }

        @Override
        public void onMaxBatteryTemperatureChanged(float c) {
            Trace.call("IBatteryInfoCallbacks.onMaxBatteryTemperatureChanged", c);
            setNum(TEMP_MAX, c);
        }

        @Override
        public void onPackSoH(int pct) {
            Trace.call("IBatteryInfoCallbacks.onPackSoH", pct);
            setNum(HEALTH, pct);
        }
    };

    // Log-only: every callback lands in the calls log; no theme value yet.
    private static final IChargerInfoCallbacks CHARGER = new ChargerInfoCallbacksBase() { };
    private static final IHandleBarCallbacks HANDLEBAR = new HandleBarCallbacksBase() { };

    private static final IVehicleCommandCallbacks COMMANDS = new VehicleCommandCallbacksBase() {
        @Override
        public void onPasscodeWindowActivated(boolean active) {
            Trace.call("IVehicleCommandCallbacks.onPasscodeWindowActivated", active);
            if (realVehicleEvent()) unlockController.passcodeWindow(active);
        }

        @Override
        public void onCommandFromVehicle(CommandFromVehicle command) {
            Trace.call("IVehicleCommandCallbacks.onCommandFromVehicle", command);
            if (command == null) {
                return;
            }
            switch (command) {
                case TYPE1_ERROR_SET: setFault(0, true); break;
                case TYPE1_ERROR_CLEAR: setFault(0, false); break;
                case TYPE2_ERROR_SET: setFault(1, true); break;
                case TYPE2_ERROR_CLEAR: setFault(1, false); break;
                case TYPE3_ERROR_SET: setFault(2, true); break;
                case TYPE3_ERROR_CLEAR: setFault(2, false); break;
                case PRNTCNTRL_HYPER_ACS_REVOKED: hyperAvailable(false); break;
                case PRNTCNTRL_ACS_RESET: hyperAvailable(true); break;
                default: break;
            }
        }
    };
}
