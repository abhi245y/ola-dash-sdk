package dev.olatool.dash;

import java.io.File;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.LinkOption;
import java.util.Arrays;

/** Removes bundled themes dropped from the APK. Plain file handling, testable on a desktop JVM. */
final class BundledThemes {
    private BundledThemes() {
    }

    static void prune(File themesDir, String[] bundled) throws IOException {
        if (!themesDir.isDirectory()) return;
        if (bundled == null) throw new IOException("Cannot list bundled themes");
        File[] themes = themesDir.listFiles();
        if (themes == null) throw new IOException("Cannot list " + themesDir);
        File active = new File(themesDir, "active");
        String activeId = active.isFile()
                ? new String(Files.readAllBytes(active.toPath()), StandardCharsets.UTF_8).trim() : null;
        // Finish interrupted deletions before renaming any more folders to .del.
        for (File theme : themes) {
            String name = theme.getName();
            if (name.endsWith(".del") && Files.isDirectory(theme.toPath(), LinkOption.NOFOLLOW_LINKS)) {
                if (name.substring(0, name.length() - 4).equals(activeId)) Files.deleteIfExists(active.toPath());
                deleteRecursively(theme);
            }
        }
        for (File theme : themes) {
            if (Files.isDirectory(theme.toPath(), LinkOption.NOFOLLOW_LINKS)
                    && new File(theme, ".bundled").isFile()
                    && !Arrays.asList(bundled).contains(theme.getName())) {
                File deleted = new File(themesDir, theme.getName() + ".del");
                // A dot makes the folder invisible to the host's valid_id theme scan.
                Files.move(theme.toPath(), deleted.toPath());
                if (theme.getName().equals(activeId)) Files.deleteIfExists(active.toPath());
                deleteRecursively(deleted);
            }
        }
    }

    private static void deleteRecursively(File file) throws IOException {
        if (Files.isDirectory(file.toPath(), LinkOption.NOFOLLOW_LINKS)) {
            File[] children = file.listFiles();
            if (children == null) throw new IOException("Cannot list " + file);
            for (File child : children) deleteRecursively(child);
        }
        Files.delete(file.toPath());
    }
}
