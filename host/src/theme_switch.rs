//! Theme selection files, shared with the simulator's unit tests.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub const DEFAULT_THEME: &str = "default";
pub const MAX_PREVIEW: u64 = 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub struct ThemeInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub preview: Option<PathBuf>,
    pub source: String,
}

fn source(dir: &Path, id: &str) -> &'static str {
    if id == DEFAULT_THEME || dir.join(".bundled").is_file() { "bundled" }
    else if dir.join(".store").is_file() { "store" }
    else { "sideloaded" }
}

/// Preview files are untrusted. The image loader decodes eagerly and sniffs the format, so
/// only a regular file of at most 1 MiB whose PNG header says exactly 640 x 384 reaches it;
/// that bounds the decoded size (a small PNG can claim a huge canvas).
fn preview_ok(path: &Path) -> bool {
    use std::io::Read;
    let Ok(meta) = fs::symlink_metadata(path) else { return false };
    let mut head = [0u8; 24];
    meta.is_file() && meta.len() <= MAX_PREVIEW
        && fs::File::open(path).and_then(|mut f| f.read_exact(&mut head)).is_ok()
        && head[..8] == *b"\x89PNG\r\n\x1a\n" && head[12..16] == *b"IHDR"
        && head[16..20] == 640u32.to_be_bytes() && head[20..24] == 384u32.to_be_bytes()
}

fn preview_path(dir: &Path) -> Option<PathBuf> {
    let path = dir.join("preview.png");
    preview_ok(&path).then_some(path)
}

fn info(dir: &Path, id: &str, host_contract: i64) -> Option<ThemeInfo> {
    let text = fs::read_to_string(dir.join("theme.json")).ok()?;
    let manifest: serde_json::Value = serde_json::from_str(&text).ok()?;
    if manifest["contract"].as_i64()? > host_contract { return None; }
    if !dir.join(manifest["entry"].as_str()?).is_file() { return None; }
    Some(ThemeInfo {
        id: id.into(),
        name: manifest["name"].as_str().filter(|name| !name.is_empty()).unwrap_or(id).into(),
        description: manifest["description"].as_str().unwrap_or("").into(),
        preview: preview_path(dir),
        source: source(dir, id).into(),
    })
}

/// Scanned at startup and when opening the Theme pane: bundled `default` first, then the rest.
/// Missing, unreadable or incompatible entries are quietly skipped.
pub fn installed(files: &Path, host_contract: i64) -> Vec<ThemeInfo> {
    let bad = fs::read_to_string(files.join("themes/bad")).unwrap_or_default();
    let mut themes = scan(&files.join("themes"), host_contract);
    themes.retain(|theme| theme.id != DEFAULT_THEME
        && !bad.split_whitespace().any(|id| id == theme.id));
    if let Some(default) = info(&files.join("fallback"), DEFAULT_THEME, host_contract) {
        themes.insert(0, default);
    }
    themes
}

/// Theme folders in `dir`, sorted by name.
pub fn scan(dir: &Path, host_contract: i64) -> Vec<ThemeInfo> {
    let mut themes = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else { continue };
            if !kind.is_dir() { continue; }
            let Ok(id) = entry.file_name().into_string() else { continue };
            if !valid_id(&id) { continue; }
            if let Some(theme) = info(&entry.path(), &id, host_contract) { themes.push(theme); }
        }
    }
    themes.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
    themes
}

/// The interpreted model is identical for the host and simulator.
pub fn model(themes: Vec<ThemeInfo>) -> slint_interpreter::Value {
    use slint_interpreter::Value;
    let rows: Vec<Value> = themes.into_iter().map(|theme| {
        // Recheck here too: a file may have changed since discovery.
        let preview = theme.preview.as_deref()
            .filter(|path| preview_ok(path))
            .and_then(|path| slint::Image::load_from_path(path).ok())
            .unwrap_or_default();
        Value::Struct([
            ("id".into(), Value::String(theme.id.into())),
            ("name".into(), Value::String(theme.name.into())),
            ("description".into(), Value::String(theme.description.into())),
            ("preview".into(), Value::Image(preview)),
            ("source".into(), Value::String(theme.source.into())),
        ].into_iter().collect())
    }).collect();
    Value::Model(slint::ModelRc::new(slint::VecModel::from(rows)))
}

/// Selection files that live beside the theme folders in `files/themes/`.
const RESERVED_IDS: [&str; 5] = ["active", "previous", "bad", "strikes", "restart"];

/// A theme folder name: 1-64 ASCII letters, digits, `_` or `-`, not a selection file name.
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64
        && id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        && !RESERVED_IDS.contains(&id)
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
        for id in ["", ".", "..", "../new", "/new", "a/b", "a\\b", "new\n", " new", "é", &"x".repeat(65), "active", "previous", "bad", "strikes", "restart"] {
            assert!(!valid_id(id), "{id}");
        }
    }

    #[test]
    fn installed_defaults_first_sorts_and_filters() {
        let t = Temp::new();
        fs::write(t.0.join("fallback/theme.json"), r#"{"entry":"main.slint","contract":7,"name":"Z Default"}"#).unwrap();
        fs::write(t.0.join("fallback/main.slint"), "").unwrap();
        for (id, manifest) in [
            ("z-id", Some(r#"{"entry":"main.slint","contract":7,"name":"Alpha","description":"First"}"#)),
            ("a-id", Some(r#"{"entry":"main.slint","contract":6,"name":"Beta"}"#)),
            ("unnamed", Some(r#"{"entry":"main.slint","contract":7}"#)),
            ("blank", Some(r#"{"entry":"main.slint","contract":7,"name":""}"#)),
            ("crashy", Some(r#"{"entry":"main.slint","contract":7,"name":"Crashy"}"#)),
            ("invalid.id", Some(r#"{"entry":"main.slint","contract":7}"#)),
            ("future", Some(r#"{"entry":"main.slint","contract":8}"#)),
            ("missing", None),
            ("broken", Some("{")),
            ("no-contract", Some("{}")),
            ("no-entry", Some(r#"{"contract":7,"name":"No entry"}"#)),
            ("default", Some(r#"{"entry":"main.slint","contract":7,"name":"Duplicate"}"#)),
        ] {
            let dir = t.0.join("themes").join(id);
            fs::create_dir(&dir).unwrap();
            if let Some(manifest) = manifest { fs::write(dir.join("theme.json"), manifest).unwrap(); }
            fs::write(dir.join("main.slint"), "").unwrap();
        }
        fs::write(t.0.join("themes/bad"), "crashy\nother\n").unwrap();
        let themes = installed(&t.0, 7);
        assert_eq!(themes.iter().map(|theme| theme.id.as_str()).collect::<Vec<_>>(),
            ["default", "z-id", "a-id", "blank", "unnamed"]);
        assert_eq!(themes[0].name, "Z Default");
        assert_eq!(themes[1].description, "First");
        assert_eq!(themes[3].name, "blank");
        assert_eq!(themes[3].description, "");
    }

    #[test]
    fn installed_tolerates_missing_directories_and_manifests() {
        let t = Temp::new();
        fs::remove_dir_all(t.0.join("themes")).unwrap();
        fs::write(t.0.join("fallback/theme.json"), r#"{"entry":"main.slint","contract":7}"#).unwrap();
        fs::write(t.0.join("fallback/main.slint"), "").unwrap();
        assert_eq!(installed(&t.0, 7)[0].id, DEFAULT_THEME);
        fs::remove_file(t.0.join("fallback/theme.json")).unwrap();
        assert!(installed(&t.0, 7).is_empty());
    }

    #[test]
    fn installed_classifies_sources_and_fallback_is_always_bundled() {
        let t = Temp::new();
        for name in ["fallback", "themes/bundled", "themes/store", "themes/local"] {
            let dir = t.0.join(name);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("theme.json"), r#"{"entry":"main.slint","contract":7}"#).unwrap();
            fs::write(dir.join("main.slint"), "").unwrap();
        }
        fs::write(t.0.join("themes/bundled/.bundled"), "").unwrap();
        fs::write(t.0.join("themes/bundled/.store"), "").unwrap();
        fs::write(t.0.join("themes/store/.store"), "").unwrap();
        // Even a stale store marker cannot relabel the host's fallback copy.
        fs::write(t.0.join("fallback/.store"), "").unwrap();
        let themes = installed(&t.0, 7);
        let pairs: Vec<_> = themes.iter().map(|t| (t.id.as_str(), t.source.as_str())).collect();
        assert_eq!(pairs, [("default", "bundled"), ("bundled", "bundled"),
            ("local", "sideloaded"), ("store", "store")]);
    }

    #[test]
    fn preview_path_checks_size_and_png_header() {
        let t = Temp::new();
        let dir = t.0.join("fallback");
        let path = dir.join("preview.png");
        assert_eq!(preview_path(&dir), None);
        fs::create_dir(&path).unwrap();
        assert_eq!(preview_path(&dir), None);
        fs::remove_dir(&path).unwrap();
        let header = |w: u32, h: u32| {
            let mut b = b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR".to_vec();
            b.extend(w.to_be_bytes());
            b.extend(h.to_be_bytes());
            b
        };
        fs::write(&path, header(640, 384)).unwrap();
        assert_eq!(preview_path(&dir), Some(path.clone()));
        fs::write(&path, header(10000, 10000)).unwrap(); // tiny file, huge canvas
        assert_eq!(preview_path(&dir), None);
        fs::write(&path, b"GIF89a not a png at all").unwrap();
        assert_eq!(preview_path(&dir), None);
        fs::write(&path, header(640, 384)).unwrap();
        fs::File::options().write(true).open(&path).unwrap().set_len(MAX_PREVIEW + 1).unwrap();
        assert_eq!(preview_path(&dir), None);
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
