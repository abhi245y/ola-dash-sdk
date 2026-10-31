package dev.olatool.dash;

import android.app.NativeActivity;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.pm.ApplicationInfo;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.provider.Settings;
import android.util.Log;
import android.view.WindowManager;

import dev.olatool.dash.plugin.PluginHost;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.Calendar;

/** Full-screen NativeActivity that hosts the Slint UI. Java only binds the vehicle and ticks the clock. */
public final class MainActivity extends NativeActivity {
    private static final String TAG = "OlaDash";

    // Debug-build test channel. Shell/root only (DUMP permission); auto-off after TEST_MAX_MS.
    private static final String ACTION_TEST = "dev.olatool.dash.TEST";
    private static final String ACTION_TEST_OFF = "dev.olatool.dash.TEST_OFF";
    private static final long TEST_MAX_MS = 10 * 60_000L;

    private final Handler handler = new Handler(Looper.getMainLooper());
    private final Runnable testOff = () -> VehicleBridge.setTestMode(false);
    private BroadcastReceiver testReceiver;
    static volatile boolean paused;

    /** Called from Rust for Media.* and Plugins.widget-clicked. Moves the work to the main looper. */
    public void onDashAction(String action) {
        handler.post(() -> PluginHost.action(action));
    }

    /** Called only by the host-owned Slint keypad; no passcode text crosses JNI. */
    public void onUnlockKey(int key) {
        VehicleBridge.unlockKey(key);
    }

    /** Theme asked for the trunk. The host gate decides (see TrunkGate). */
    public void onOpenTrunk() {
        VehicleBridge.openTrunk();
    }

    /** Called synchronously by Rust before a theme-switch process exit. */
    public void onThemeExit() {
        BadThemeGuard.cleanExit();
    }
    private final Runnable clock = new Runnable() {
        @Override
        public void run() {
            Calendar c = Calendar.getInstance();
            VehicleBridge.setClockTrusted(Settings.Global.getInt(getContentResolver(),
                    Settings.Global.AUTO_TIME, 0) == 1 && c.get(Calendar.YEAR) >= 2025);
            VehicleBridge.setClock(c.get(Calendar.HOUR_OF_DAY), c.get(Calendar.MINUTE));
            VehicleBridge.setDate(c.get(Calendar.YEAR), c.get(Calendar.MONTH) + 1, c.get(Calendar.DAY_OF_MONTH));
            handler.postDelayed(this, 60_000L - (System.currentTimeMillis() % 60_000L));
        }
    };

    @Override
    protected void onCreate(Bundle state) {
        // Copy assets first: NativeActivity starts the Rust main thread inside super.onCreate.
        try {
            copyAssetsIfChanged();
        } catch (IOException e) {
            Log.e(TAG, "asset copy failed", e);
        }
        // Bad-theme guard: strike the active theme before Slint can crash on it (cleared below).
        Log.i(TAG, "theme guard: " + BadThemeGuard.strike(getFilesDir()));
        super.onCreate(state);
        handler.postDelayed(BadThemeGuard::cleanExit, 10_000L); // survived 10 s
        // Unverified until M-sleep (session A): does the locked panel still sleep with this flag?
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        VehicleBridge.start(this);
        PluginHost.start(this);
        clock.run();
        if ((getApplicationInfo().flags & ApplicationInfo.FLAG_DEBUGGABLE) != 0) {
            registerTestChannel();
        }
    }

    private void registerTestChannel() {
        testReceiver = new BroadcastReceiver() {
            @Override
            public void onReceive(Context c, Intent i) {
                handler.removeCallbacks(testOff);
                if (ACTION_TEST_OFF.equals(i.getAction())) {
                    testOff.run();
                    return;
                }
                VehicleBridge.setTestMode(true);
                handler.postDelayed(testOff, TEST_MAX_MS);
                VehicleBridge.inject(i.getStringExtra("cb"), i.getStringExtra("m"),
                        i.getStringArrayExtra("a"));
            }
        };
        IntentFilter f = new IntentFilter(ACTION_TEST);
        f.addAction(ACTION_TEST_OFF);
        registerReceiver(testReceiver, f, "android.permission.DUMP", null);
        Log.i(TAG, "test channel enabled (debug build)");
    }

    @Override
    protected void onPause() {
        paused = true;
        BadThemeGuard.cleanExit(); // Android pauses before any background kill, and Slint is up by now
        super.onPause();
    }

    @Override
    protected void onResume() {
        super.onResume();
        paused = false;
    }

    @Override
    protected void onDestroy() {
        handler.removeCallbacks(clock);
        handler.removeCallbacks(testOff);
        if (testReceiver != null) {
            unregisterReceiver(testReceiver);
        }
        PluginHost.stop();
        VehicleBridge.stop();
        super.onDestroy();
    }

    /** Skips the copy when files/.assets-stamp matches the APK's update time; any doubt copies. */
    private void copyAssetsIfChanged() throws IOException {
        String update = null;
        try {
            update = Long.toString(getPackageManager().getPackageInfo(getPackageName(), 0).lastUpdateTime);
        } catch (Exception e) {
            Log.w(TAG, "package info unavailable, copying assets", e);
        }
        File stamp = new File(getFilesDir(), ".assets-stamp");
        try {
            if (update != null && update.equals(new String(Files.readAllBytes(stamp.toPath()), StandardCharsets.UTF_8))) {
                return;
            }
        } catch (IOException e) {
            // Missing or unreadable stamp: copy.
        }
        copyAssets("");
        if (update != null) Files.write(stamp.toPath(), update.getBytes(StandardCharsets.UTF_8));
    }

    /** Copies bundled assets into files/, overwriting bundled theme ids on APK updates. */
    private void copyAssets(String dir) throws IOException {
        String[] names = getAssets().list(dir);
        if (names == null) {
            return;
        }
        for (String name : names) {
            String path = dir.isEmpty() ? name : dir + "/" + name;
            String[] children = getAssets().list(path);
            if (children != null && children.length > 0) {
                new File(getFilesDir(), path).mkdirs();
                copyAssets(path);
                continue;
            }
            try (InputStream in = getAssets().open(path);
                 OutputStream out = new FileOutputStream(new File(getFilesDir(), path))) {
                byte[] buf = new byte[8192];
                int n;
                while ((n = in.read(buf)) > 0) {
                    out.write(buf, 0, n);
                }
            }
        }
    }
}
