package dev.olatool.dash.plugin.sysinfo;

import android.app.Service;
import android.content.Intent;
import android.os.Handler;
import android.os.IBinder;
import android.os.Looper;
import android.os.RemoteException;
import android.os.SystemClock;

import dev.olatool.dash.plugin.IDashHost;
import dev.olatool.dash.plugin.IDashPlugin;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileReader;
import java.io.IOException;
import java.util.Locale;

/**
 * Example plugin: publishes three widgets every 5 s (free RAM, hottest CPU zone, uptime).
 * Tapping the RAM or temperature widget toggles its unit. Display-only, no vehicle access.
 */
public final class SysinfoService extends Service {
    private static final long PERIOD_MS = 5000L;

    private final Handler main = new Handler(Looper.getMainLooper());
    private IDashHost host;
    private boolean gigabytes;
    private boolean fahrenheit;

    private final Runnable tick = new Runnable() {
        @Override
        public void run() {
            publish();
            main.postDelayed(this, PERIOD_MS);
        }
    };

    private final IDashPlugin.Stub stub = new IDashPlugin.Stub() {
        @Override
        public void attach(IDashHost h, int hostApiVersion) {
            main.post(() -> {
                host = h;
                main.removeCallbacks(tick);
                tick.run();
            });
        }

        @Override
        public void detach() {
            main.post(() -> {
                main.removeCallbacks(tick);
                host = null;
            });
        }

        @Override
        public void onAction(String action) {
            main.post(() -> {
                if ("widget.ram".equals(action)) {
                    gigabytes = !gigabytes;
                } else if ("widget.temp".equals(action)) {
                    fahrenheit = !fahrenheit;
                } else {
                    return;
                }
                publish();
            });
        }
    };

    @Override
    public IBinder onBind(Intent intent) {
        return stub;
    }

    private void publish() {
        if (host == null) {
            return;
        }
        try {
            long freeKb = memAvailableKb();
            host.publishWidget("ram", "Free RAM",
                    gigabytes ? String.format(Locale.US, "%.1f", freeKb / 1048576.0)
                            : String.valueOf(freeKb / 1024),
                    gigabytes ? "GB" : "MB");
            double c = hottestCpuCelsius();
            host.publishWidget("temp", "CPU",
                    Double.isNaN(c) ? "--"
                            : String.format(Locale.US, "%.0f", fahrenheit ? c * 9 / 5 + 32 : c),
                    fahrenheit ? "F" : "C");
            long up = SystemClock.elapsedRealtime() / 60000L;
            host.publishWidget("uptime", "Uptime", (up / 60) + "h " + (up % 60) + "m", "");
        } catch (RemoteException ignored) {
            // The host went away; detach() stops us.
        }
    }

    /** MemAvailable from /proc/meminfo, in kB; 0 if unreadable. */
    private static long memAvailableKb() {
        try (BufferedReader r = new BufferedReader(new FileReader("/proc/meminfo"))) {
            String line;
            while ((line = r.readLine()) != null) {
                if (line.startsWith("MemAvailable:")) {
                    return Long.parseLong(line.replaceAll("[^0-9]", ""));
                }
            }
        } catch (IOException | NumberFormatException ignored) {
            // Show 0 rather than crash.
        }
        return 0;
    }

    /** Highest thermal zone whose type names a CPU, in degrees C; NaN if none is readable. */
    private static double hottestCpuCelsius() {
        double hottest = Double.NaN;
        for (int i = 0; i < 64; i++) {
            File dir = new File("/sys/class/thermal/thermal_zone" + i);
            if (!dir.isDirectory()) {
                continue;
            }
            try {
                String type = read(new File(dir, "type"));
                if (type.contains("cpu")) {
                    double c = Long.parseLong(read(new File(dir, "temp")).trim()) / 1000.0;
                    if (Double.isNaN(hottest) || c > hottest) {
                        hottest = c;
                    }
                }
            } catch (IOException | NumberFormatException ignored) {
                // Skip zones we cannot read.
            }
        }
        return hottest;
    }

    private static String read(File f) throws IOException {
        try (BufferedReader r = new BufferedReader(new FileReader(f))) {
            String line = r.readLine();
            return line == null ? "" : line;
        }
    }
}
