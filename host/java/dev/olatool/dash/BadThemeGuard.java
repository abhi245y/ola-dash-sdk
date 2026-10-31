package dev.olatool.dash;

import java.io.File;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.StandardOpenOption;

/**
 * Crash-loop guard for themes (E0.S6 T4). Called from MainActivity.onCreate before Slint starts:
 * every start strikes the active theme (files/themes/strikes = "&lt;name&gt; &lt;n&gt;"), a start that
 * survives 10 s clears the strike, and a theme that already has {@link #MAX_STRIKES} strikes is
 * replaced by "default" and appended to files/themes/bad (one name per line).
 * {@link #next} is pure so it runs on a desktop JVM (host/test/BadThemeGuardTest.java).
 */
final class BadThemeGuard {
    static final int MAX_STRIKES = 2;
    static final String DEFAULT = "default";
    private static File themes;

    private BadThemeGuard() {
    }

    /** Outcome of one start. {@code strikes} is the new strikes line (null = clear); {@code bad} is the theme struck out, or null. */
    static final class Result {
        final String active;
        final String strikes;
        final String bad;

        Result(String active, String strikes, String bad) {
            this.active = active;
            this.strikes = strikes;
            this.bad = bad;
        }
    }

    /**
     * @param active      content of themes/active; null, blank or an invalid name = "default" (as the Rust loader)
     * @param strikesLine content of themes/strikes; null or garbage = no strikes
     */
    static Result next(String active, String strikesLine) {
        String name = active == null ? "" : active.trim();
        if (name.isEmpty() || name.length() > 64 || !name.matches("[A-Za-z0-9_-]+")) name = DEFAULT;
        int n = 0;
        String[] p = strikesLine == null ? new String[0] : strikesLine.trim().split("\\s+");
        if (p.length == 2 && p[0].equals(name)) {
            try {
                n = Math.max(Integer.parseInt(p[1]), 0);
            } catch (NumberFormatException e) {
                n = 0;
            }
        }
        // ponytail: default is never swapped out, it only keeps counting so the log shows it failing.
        if (n >= MAX_STRIKES && !name.equals(DEFAULT)) return new Result(DEFAULT, null, name);
        return new Result(name, name + " " + (n + 1), null);
    }

    /**
     * Strike the active theme; returns what happened for logging, or null if files/ is unusable.
     * Runs once per process (a recreate() must not count as a start).
     */
    static synchronized String onStart(File files) {
        if (themes != null) return null;
        themes = new File(files, "themes");
        themes.mkdirs();
        Result r = next(read("active"), read("strikes"));
        try {
            if (r.bad != null) {
                write("active", DEFAULT, false);
                write("bad", r.bad + "\n", true);
                new File(themes, "strikes").delete();
                return "theme " + r.bad + " struck out, active reset to default";
            }
            write("strikes", r.strikes, false);
        } catch (IOException e) {
            return "guard write failed: " + e;
        }
        return "strike " + r.strikes;
    }

    /** The theme survived its first seconds, or the app is exiting on purpose (E0.S6 T1 calls this before exit(0)). */
    static synchronized void cleanExit() {
        if (themes != null) new File(themes, "strikes").delete();
    }

    private static String read(String file) {
        try {
            return new String(Files.readAllBytes(new File(themes, file).toPath()), StandardCharsets.UTF_8);
        } catch (IOException e) {
            return null;
        }
    }

    private static void write(String file, String text, boolean append) throws IOException {
        Files.write(new File(themes, file).toPath(), text.getBytes(StandardCharsets.UTF_8),
                StandardOpenOption.CREATE, append ? StandardOpenOption.APPEND : StandardOpenOption.TRUNCATE_EXISTING);
    }
}
