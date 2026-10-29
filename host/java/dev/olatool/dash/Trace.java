package dev.olatool.dash;

import android.content.Context;
import android.util.Log;

import java.io.BufferedWriter;
import java.io.File;
import java.io.FileWriter;
import java.io.IOException;
import java.text.SimpleDateFormat;
import java.util.Arrays;
import java.util.Date;
import java.util.Locale;

/**
 * Session logs for field testing, in the app's external files dir (adb pull, no root):
 *   logs/calls-<start>.log  every vehicle-parser callback with its arguments, plus host events
 *   logs/can-<start>.log    raw vcan0 traffic from the vendor candump (read-only)
 */
public final class Trace {
    private static final String TAG = "OlaDash";
    // ponytail: whole-folder cap, oldest session files deleted first. Raise if a ride needs more.
    private static final long MAX_LOG_BYTES = 1L << 30;

    private static BufferedWriter calls;
    private static Process candump;
    static volatile boolean testMode;

    private Trace() {
    }

    static synchronized void start(Context ctx) {
        File dir = new File(ctx.getExternalFilesDir(null), "logs");
        if (!dir.mkdirs() && !dir.isDirectory()) {
            Log.w(TAG, "trace: no log dir " + dir);
            return;
        }
        prune(dir);
        String stamp = new SimpleDateFormat("yyyyMMdd-HHmmss", Locale.US).format(new Date());
        try {
            calls = new BufferedWriter(new FileWriter(new File(dir, "calls-" + stamp + ".log")));
        } catch (IOException e) {
            Log.w(TAG, "trace: calls log failed", e);
        }
        try {
            candump = new ProcessBuilder("/vendor/bin/candump", "-L", "vcan0")
                    .redirectOutput(new File(dir, "can-" + stamp + ".log"))
                    .redirectErrorStream(true)
                    .start();
        } catch (IOException e) {
            Log.w(TAG, "trace: candump failed", e);
        }
        Log.i(TAG, "trace: logging to " + dir);
        event("session start");
    }

    static synchronized void stop() {
        event("session stop");
        if (candump != null) {
            candump.destroy();
            candump = null;
        }
        if (calls != null) {
            try {
                calls.close();
            } catch (IOException ignored) {
                // Closing anyway.
            }
            calls = null;
        }
    }

    /** One vehicle callback. Called from binder threads. */
    public static void call(String name, Object... args) {
        write((testMode ? "TEST " : "") + name + " " + Arrays.deepToString(args));
    }

    /** A host-side event (keep-alive change, screen state, ...). */
    public static void event(String what) {
        write("# " + what);
    }

    /** Called once a second by the keep-alive tick so a crash loses at most ~1 s. */
    static synchronized void flush() {
        if (calls != null) {
            try {
                calls.flush();
            } catch (IOException ignored) {
                // Disk full or gone; keep running.
            }
        }
    }

    private static synchronized void write(String line) {
        if (calls == null) {
            return;
        }
        try {
            calls.write(System.currentTimeMillis() + " " + line + "\n");
        } catch (IOException ignored) {
            // Disk full or gone; keep running.
        }
    }

    private static void prune(File dir) {
        File[] files = dir.listFiles();
        if (files == null) {
            return;
        }
        Arrays.sort(files, (a, b) -> Long.compare(a.lastModified(), b.lastModified()));
        long total = 0;
        for (File f : files) {
            total += f.length();
        }
        for (File f : files) {
            if (total <= MAX_LOG_BYTES) {
                break;
            }
            total -= f.length();
            f.delete();
        }
    }
}
