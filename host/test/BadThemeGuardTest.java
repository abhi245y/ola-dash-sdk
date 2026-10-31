package dev.olatool.dash;

import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;

/** Runs on a desktop JVM. Bad-theme guard decisions and its file handling in a temp dir. No Android. */
public final class BadThemeGuardTest {
    private static int checks;

    private static void check(boolean condition, String reason) {
        checks++;
        if (!condition) throw new AssertionError(reason);
    }

    private static String read(File f) throws Exception {
        return f.exists() ? new String(Files.readAllBytes(f.toPath()), StandardCharsets.UTF_8) : null;
    }

    public static void main(String[] args) throws Exception {
        BadThemeGuard.Result r = BadThemeGuard.next("neon", null);
        check(r.active.equals("neon") && "neon 1".equals(r.strikes) && r.bad == null, "First start: strike 1");
        r = BadThemeGuard.next("neon\n", "neon 1\n");
        check("neon 2".equals(r.strikes) && r.bad == null && r.active.equals("neon"), "Second start: strike 2, still neon");
        r = BadThemeGuard.next("neon", "neon 2");
        check(r.active.equals("default") && "neon".equals(r.bad) && r.strikes == null, "Third start: default, neon bad");
        r = BadThemeGuard.next("neon", "other 2");
        check("neon 1".equals(r.strikes) && r.bad == null, "Strikes of another theme do not count");
        check("neon 1".equals(BadThemeGuard.next("neon", "neon x").strikes), "Garbage count restarts at 1");
        check("neon 1".equals(BadThemeGuard.next("neon", "neon 1 2").strikes), "Malformed line restarts at 1");
        r = BadThemeGuard.next("default", "default 5");
        check(r.active.equals("default") && "default 6".equals(r.strikes) && r.bad == null, "Default is never struck out");
        check(BadThemeGuard.next(null, null).active.equals("default"), "No active file: default");
        check(BadThemeGuard.next("../x", null).active.equals("default"), "Invalid name: default");
        check("default 1".equals(BadThemeGuard.next("  ", null).strikes), "Blank name: default strikes");
        check("default 1".equals(BadThemeGuard.next("default", "default 99999999999").strikes), "Overflowing count restarts");

        File files = Files.createTempDirectory("guard").toFile();
        File themes = new File(files, "themes");
        themes.mkdirs();
        Files.write(new File(themes, "active").toPath(), "neon".getBytes(StandardCharsets.UTF_8));
        BadThemeGuard.onStart(files);
        check("neon 1".equals(read(new File(themes, "strikes"))), "onStart writes strike 1");
        check(BadThemeGuard.onStart(files) == null, "Second onStart in the same process is ignored");
        BadThemeGuard.cleanExit();
        check(read(new File(themes, "strikes")) == null, "cleanExit deletes strikes");
        System.out.println("BadThemeGuardTest: " + checks + " checks passed");
    }
}
