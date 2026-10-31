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
        File active = new File(themes, "active"), strikes = new File(themes, "strikes"), bad = new File(themes, "bad");
        Files.write(active.toPath(), "neon".getBytes(StandardCharsets.UTF_8));
        BadThemeGuard.strike(files);
        check("neon 1".equals(read(strikes)), "strike writes strike 1");
        check(BadThemeGuard.strike(files) == null, "Second strike in the same process is ignored");
        BadThemeGuard.cleanExit();
        check(read(strikes) == null, "cleanExit deletes strikes");

        for (int i = 0; i < 2; i++) { // second round: bad already lists neon, no duplicate
            BadThemeGuard.reset();
            Files.write(active.toPath(), "neon".getBytes(StandardCharsets.UTF_8));
            Files.write(strikes.toPath(), "neon 2".getBytes(StandardCharsets.UTF_8));
            BadThemeGuard.strike(files);
            check("default".equals(read(active)), "Struck out: active is default");
            check("neon\n".equals(read(bad)), "Struck out: neon listed once in bad");
            check(read(strikes) == null, "Struck out: strikes deleted");
        }
        System.out.println("BadThemeGuardTest: " + checks + " checks passed");
    }
}
