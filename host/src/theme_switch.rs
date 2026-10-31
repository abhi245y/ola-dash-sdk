//! Theme selection files, shared with the simulator's unit tests.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

pub const DEFAULT_THEME: &str = "default";

pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64
        && id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}

pub fn active(files: &Path) -> String {
    fs::read_to_string(files.join("themes/active"))
        .ok().map(|s| s.trim().to_string()).filter(|s| valid_id(s))
        .unwrap_or_else(|| DEFAULT_THEME.into())
}

pub fn validate(files: &Path, id: &str) -> io::Result<()> {
    if !valid_id(id) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid theme id"));
    }
    match fs::read_to_string(files.join("themes/bad")) {
        Ok(bad) if bad.split_whitespace().any(|name| name == id) => {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "theme is marked bad"));
        }
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
        _ => {}
    }
    let dir = if id == DEFAULT_THEME { files.join("fallback") } else { files.join("themes").join(id) };
    if !dir.join("theme.json").is_file() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "unknown theme"));
    }
    if active(files) == id {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, "theme is already active"));
    }
    Ok(())
}

fn write(dir: &Path, name: &str, value: &str) -> io::Result<()> {
    let tmp = dir.join(format!("{name}.tmp"));
    let mut file = fs::File::create(&tmp)?;
    file.write_all(value.as_bytes())?;
    file.sync_all()?;
    fs::rename(tmp, dir.join(name))
}

pub fn begin(files: &Path, id: &str) -> io::Result<()> {
    validate(files, id)?;
    let themes = files.join("themes");
    fs::create_dir_all(&themes)?;
    write(&themes, "previous", &active(files))?;
    write(&themes, "active", id)
}

pub fn keep(files: &Path) -> io::Result<()> {
    match fs::remove_file(files.join("themes/previous")) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}

/// Restores `previous` as active and returns its id. A corrupt `previous` is dropped
/// (the current theme stays) so the confirm card does not retry forever.
pub fn revert(files: &Path) -> io::Result<String> {
    let themes = files.join("themes");
    let previous = fs::read_to_string(themes.join("previous"))?.trim().to_string();
    if !valid_id(&previous) {
        keep(files)?;
        return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid previous theme id"));
    }
    write(&themes, "active", &previous)?;
    keep(files)?;
    Ok(previous)
}

/// Theme name for the switching card the next start shows until lock state is known.
pub fn mark_restart(files: &Path, name: &str) -> io::Result<()> {
    write(&files.join("themes"), "restart", name)
}

/// Reads and removes the restart marker; empty when this start is not a theme restart.
pub fn take_restart(files: &Path) -> String {
    let path = files.join("themes/restart");
    let name = fs::read_to_string(&path).unwrap_or_default();
    let _ = fs::remove_file(path);
    name
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let dir = std::env::temp_dir().join(format!("oladash-switch-{}-{}",
                std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
            for theme in ["fallback", "themes/new"] {
                fs::create_dir_all(dir.join(theme)).unwrap();
                fs::write(dir.join(theme).join("theme.json"), "{}").unwrap();
            }
            Self(dir)
        }
        fn read(&self, name: &str) -> String {
            fs::read_to_string(self.0.join("themes").join(name)).unwrap()
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
    }

    #[test]
    fn ids_match_the_loader_rule() {
        for id in ["default", "Twin_Dial-2", &"x".repeat(64)] {
            assert!(valid_id(id), "{id}");
        }
        for id in ["", ".", "..", "../new", "/new", "a/b", "a\\b", "new\n", " new", "é", &"x".repeat(65)] {
            assert!(!valid_id(id), "{id}");
        }
    }

    #[test]
    fn switch_and_keep() {
        let t = Temp::new();
        fs::write(t.0.join("themes/active"), "old\n").unwrap();
        begin(&t.0, "new").unwrap();
        assert_eq!(t.read("previous"), "old");
        assert_eq!(t.read("active"), "new");
        keep(&t.0).unwrap();
        keep(&t.0).unwrap();
        assert!(!t.0.join("themes/previous").exists());
        assert_eq!(t.read("active"), "new");
    }

    #[test]
    fn switch_and_revert() {
        let t = Temp::new();
        begin(&t.0, "new").unwrap();
        assert_eq!(t.read("previous"), DEFAULT_THEME);
        assert_eq!(revert(&t.0).unwrap(), DEFAULT_THEME);
        assert_eq!(t.read("active"), DEFAULT_THEME);
        assert!(!t.0.join("themes/previous").exists());
        begin(&t.0, "new").unwrap();
        begin(&t.0, DEFAULT_THEME).unwrap();
        assert_eq!(t.read("active"), DEFAULT_THEME);
    }

    #[test]
    fn revert_restores_custom_theme() {
        let t = Temp::new();
        fs::write(t.0.join("themes/active"), "Twin_Dial-2\n").unwrap();
        begin(&t.0, "new").unwrap();
        revert(&t.0).unwrap();
        assert_eq!(t.read("active"), "Twin_Dial-2");
        assert!(!t.0.join("themes/previous").exists());
    }

    #[test]
    fn failed_switch_keeps_old_active_and_backup() {
        let t = Temp::new();
        fs::write(t.0.join("themes/active"), "old").unwrap();
        fs::create_dir(t.0.join("themes/active.tmp")).unwrap();
        assert!(begin(&t.0, "new").is_err());
        assert_eq!(t.read("active"), "old");
        assert_eq!(t.read("previous"), "old");
    }

    #[test]
    fn invalid_active_uses_default() {
        let t = Temp::new();
        fs::write(t.0.join("themes/active"), "../escape").unwrap();
        begin(&t.0, "new").unwrap();
        assert_eq!(t.read("previous"), DEFAULT_THEME);
    }

    #[test]
    fn refused_targets_leave_files_untouched() {
        let t = Temp::new();
        fs::write(t.0.join("themes/active"), "default").unwrap();
        fs::write(t.0.join("themes/bad"), "other\nnew\n").unwrap();
        for id in ["../new", "unknown", "new", "default"] {
            assert!(begin(&t.0, id).is_err());
            assert_eq!(t.read("active"), DEFAULT_THEME);
            assert!(!t.0.join("themes/previous").exists());
        }
    }

    #[test]
    fn failed_revert_keeps_previous() {
        let t = Temp::new();
        begin(&t.0, "new").unwrap();
        fs::create_dir(t.0.join("themes/active.tmp")).unwrap();
        assert!(revert(&t.0).is_err());
        assert_eq!(t.read("active"), "new");
        assert_eq!(t.read("previous"), DEFAULT_THEME);
    }

    #[test]
    fn corrupt_previous_is_dropped_and_active_kept() {
        let t = Temp::new();
        fs::write(t.0.join("themes/active"), "new").unwrap();
        fs::write(t.0.join("themes/previous"), "../escape").unwrap();
        assert!(revert(&t.0).is_err());
        assert_eq!(t.read("active"), "new");
        assert!(!t.0.join("themes/previous").exists());
    }

    #[test]
    fn restart_marker_is_read_once() {
        let t = Temp::new();
        assert_eq!(take_restart(&t.0), "");
        mark_restart(&t.0, "Twin Dial").unwrap();
        assert_eq!(take_restart(&t.0), "Twin Dial");
        assert_eq!(take_restart(&t.0), "");
    }
}
