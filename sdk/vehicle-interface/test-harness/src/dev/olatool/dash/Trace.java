package dev.olatool.dash;

import android.content.Context;
import android.util.Log;

import java.io.File;
import java.io.FileWriter;
import java.io.IOException;
import java.io.PrintWriter;
import java.text.SimpleDateFormat;
import java.util.Arrays;
import java.util.Date;
import java.util.Locale;

/**
 * Harness stand-in for the host's Trace: the same call() signature, so the generated
 * callback bases compile unchanged. Writes to logcat (tag IfaceTest) and to a file in
 * the app's external files dir.
 */
public final class Trace {
    private static final String TAG = "IfaceTest";
    private static PrintWriter file;

    private Trace() {
    }

    public static synchronized void init(Context ctx) {
        File dir = ctx.getExternalFilesDir(null);
        String stamp = new SimpleDateFormat("yyyyMMdd-HHmmss", Locale.US).format(new Date());
        try {
            file = new PrintWriter(new FileWriter(new File(dir, "ifacetest-" + stamp + ".log")), true);
        } catch (IOException e) {
            Log.w(TAG, "no log file", e);
        }
    }

    public static synchronized void call(String name, Object... args) {
        String line = name + " " + Arrays.deepToString(args);
        Log.i(TAG, line);
        if (file != null) {
            file.println(line);
        }
    }
}
