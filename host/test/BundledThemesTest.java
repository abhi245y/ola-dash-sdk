package dev.olatool.dash;

import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.LinkOption;

/** Runs on a desktop JVM. Bundled-theme pruning in a temp dir. No Android. */
public final class BundledThemesTest {
    private static int checks;

    private static void check(boolean condition, String reason) {
        checks++;
        if (!condition) throw new AssertionError(reason);
    }

    private static void write(File file, String text) throws Exception {
        Files.write(file.toPath(), text.getBytes(StandardCharsets.UTF_8));
    }

    private static String read(File file) throws Exception {
        return new String(Files.readAllBytes(file.toPath()), StandardCharsets.UTF_8);
    }

    private static File theme(File themes, String id, boolean bundled) throws Exception {
        File dir = new File(themes, id);
        Files.createDirectories(dir.toPath());
        write(new File(dir, "theme.json"), "{}");
        if (bundled) write(new File(dir, ".bundled"), "");
        return dir;
    }

    private static void clean(File file) throws Exception {
        if (Files.isDirectory(file.toPath(), LinkOption.NOFOLLOW_LINKS)) {
            File[] children = file.listFiles();
            if (children == null) throw new AssertionError("Cannot list " + file);
            for (File child : children) clean(child);
        }
        Files.delete(file.toPath());
    }

    public static void main(String[] args) throws Exception {
        File temp = Files.createTempDirectory("bundled-themes").toFile();
        try {
            File themes = new File(temp, "themes");
            BundledThemes.prune(themes, new String[] {"current"});
            check(!themes.exists(), "Pruning a missing themes directory creates nothing");
            Files.createDirectory(themes.toPath());
            File stale = theme(themes, "old", true);
            File kept = theme(themes, "current", true);
            File sideloaded = theme(themes, "custom", false);
            File leftover = theme(themes, "x.del", false);
            File outside = new File(temp, "outside");
            Files.createDirectory(outside.toPath());
            File survivor = new File(outside, "keep.txt");
            write(survivor, "outside contents");
            File nested = new File(stale, "nested");
            Files.createDirectory(nested.toPath());
            write(new File(nested, "asset"), "bundled asset");
            Files.createSymbolicLink(new File(nested, "outside").toPath(), outside.toPath());
            Files.createSymbolicLink(new File(stale, "dangling").toPath(), new File(temp, "missing").toPath());
            File linked = new File(themes, "linked");
            write(new File(outside, ".bundled"), "");
            Files.createSymbolicLink(linked.toPath(), outside.toPath());
            File active = new File(themes, "active");
            write(active, "  old\n");
            String[] stateFiles = {"previous", "strikes", "bad", "restart", "plain", "plain.del"};
            for (String name : stateFiles) write(new File(themes, name), "keep " + name);

            BundledThemes.prune(themes, new String[] {"current"});
            check(!stale.exists(), "Dropped marked theme is removed");
            check(!new File(themes, "old.del").exists(), "Renamed theme is fully deleted");
            check(new File(kept, ".bundled").isFile(), "Current bundled theme is kept");
            check("{}".equals(read(new File(kept, "theme.json"))), "Current theme contents survive");
            check(new File(sideloaded, "theme.json").isFile(), "Unmarked theme is kept");
            check(!active.exists(), "Active naming the pruned id is deleted after trimming");
            check(!leftover.exists(), "Leftover .del folder is removed even without a marker");
            check(outside.isDirectory(), "Pruning does not follow a directory symlink");
            check("outside contents".equals(read(survivor)), "Outside directory contents survive");
            check(new File(outside, ".bundled").isFile(), "Outside marker survives");
            check(Files.isSymbolicLink(linked.toPath()), "Top-level symlink is kept");
            for (String name : stateFiles) {
                check(("keep " + name).equals(read(new File(themes, name))), "Plain file is kept: " + name);
            }

            stale = theme(themes, "another", true);
            write(active, "  custom\n");
            BundledThemes.prune(themes, new String[] {"current"});
            check(!stale.exists(), "Another dropped bundled theme is removed");
            check("  custom\n".equals(read(active)), "Active naming another id is kept unchanged");

            leftover = theme(themes, "interrupted.del", false);
            write(active, "interrupted\n");
            BundledThemes.prune(themes, new String[] {"current"});
            check(!leftover.exists(), "Interrupted deletion is finished");
            check(!active.exists(), "Interrupted deletion also resets its active id");
            BundledThemes.prune(themes, new String[] {"current"});
            check(kept.isDirectory() && sideloaded.isDirectory(), "Repeated pruning keeps installed themes");
        } finally {
            clean(temp);
        }
        System.out.println("BundledThemesTest: " + checks + " checks passed");
    }
}
