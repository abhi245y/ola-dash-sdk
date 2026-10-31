//! Owner plugin enablement and the settings model, shared with the simulator.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PluginInfo {
    pub package: String,
    pub label: String,
    pub slots: String,
    pub enabled: bool,
}

/// Package names cannot inject another enablement line or escape their field.
pub fn valid_package(package: &str) -> bool {
    !package.is_empty() && package.len() <= 255
        && !package.starts_with('.') && !package.ends_with('.') && !package.contains("..")
        && package.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'.')
}

/// Reads only the inventory fields. The host never trusts an `enabled` JSON field.
pub fn parse_installed(text: &str) -> Vec<PluginInfo> {
    let Ok(serde_json::Value::Array(entries)) = serde_json::from_str(text) else { return Vec::new() };
    entries.iter().filter_map(|entry| {
        let package = entry["package"].as_str().filter(|s| valid_package(s))?;
        let label = entry["label"].as_str().map(str::trim).filter(|s| !s.is_empty()).unwrap_or(package);
        Some(PluginInfo {
            package: package.into(),
            label: printable(label),
            slots: printable(entry["slots"].as_str().unwrap_or("")),
            enabled: false,
        })
    }).take(32).collect()
}

/// Untrusted text from a plugin APK: no control or bidi-override characters (a newline would
/// spill out of its row), at most 48 characters.
fn printable(text: &str) -> String {
    text.chars().filter(|&c| !c.is_control() && !matches!(c,
        '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
        .take(48).collect()
}

fn line_package(line: &str) -> Option<&str> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') { return None; }
    Some(line.split_once(':').map_or(line, |(package, _)| package.trim()))
}

pub fn list(files: &Path) -> Vec<PluginInfo> {
    let text = fs::read_to_string(files.join("plugins/installed.json")).unwrap_or_default();
    let enabled = fs::read_to_string(files.join("plugins/enabled")).unwrap_or_default();
    let mut plugins = parse_installed(&text);
    for plugin in &mut plugins {
        plugin.enabled = enabled.lines().any(|line| line_package(line) == Some(plugin.package.as_str()));
    }
    plugins
}

/// Preserve comments, whitespace, slot restrictions and unrelated lines byte for byte.
pub fn set_enabled(files: &Path, package: &str, on: bool) -> io::Result<()> {
    if !valid_package(package) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid plugin package"));
    }
    let dir = files.join("plugins");
    let path = dir.join("enabled");
    let previous = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };
    let mut next = String::new();
    let mut found = false;
    for line in previous.split_inclusive('\n') {
        if line_package(line) == Some(package) {
            found = true;
            if !on { continue; }
        }
        next.push_str(line);
    }
    if found == on {
        return Ok(()); // already in that state: no rewrite, no rebind
    }
    if on {
        if !next.is_empty() && !next.ends_with('\n') { next.push('\n'); }
        next.push_str(package);
        next.push('\n');
    }
    fs::create_dir_all(&dir)?;
    let tmp = dir.join("enabled.tmp");
    let mut file = fs::File::create(&tmp)?;
    file.write_all(next.as_bytes())?;
    file.sync_all()?;
    fs::rename(tmp, path)
}

fn slot_words(slots: &str) -> String {
    slots.split(',').map(str::trim).filter(|s| !s.is_empty()).map(|slot| {
        let mut chars = slot.chars();
        let first = chars.next().unwrap();
        first.to_uppercase().chain(chars).collect::<String>()
    }).collect::<Vec<_>>().join(" · ")
}

pub fn model(plugins: Vec<PluginInfo>) -> slint_interpreter::Value {
    use slint_interpreter::Value;
    let rows: Vec<Value> = plugins.into_iter().map(|plugin| {
        Value::Struct([
            ("package".into(), Value::String(plugin.package.into())),
            ("label".into(), Value::String(plugin.label.into())),
            ("slots".into(), Value::String(slot_words(&plugin.slots).into())),
            ("enabled".into(), Value::Bool(plugin.enabled)),
        ].into_iter().collect())
    }).collect();
    Value::Model(slint::ModelRc::new(slint::VecModel::from(rows)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn labels_drop_control_characters() {
        let plugins = parse_installed(r#"[{"package":"a.b","label":"Dash\n\n\u202eInfo","slots":"widgets\r"}]"#);
        assert_eq!(plugins[0].label, "DashInfo");
        assert_eq!(plugins[0].slots, "widgets");
    }
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let dir = std::env::temp_dir().join(format!("oladash-plugins-{}-{}",
                std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
        fn write(&self, name: &str, text: &str) {
            fs::create_dir_all(self.0.join("plugins")).unwrap();
            fs::write(self.0.join("plugins").join(name), text).unwrap();
        }
        fn enabled(&self) -> String { fs::read_to_string(self.0.join("plugins/enabled")).unwrap() }
    }
    impl Drop for Temp {
        fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
    }

    #[test]
    fn package_validation() {
        for package in ["a", "dev.example_plugin2", &"a".repeat(255)] {
            assert!(valid_package(package), "{package:?}");
        }
        for package in ["", "bad\npkg", "a..b", ".a", "a.", "a/b", "a:b", "é", " a", &"a".repeat(256)] {
            assert!(!valid_package(package), "{package:?}");
        }
    }

    #[test]
    fn missing_or_invalid_inventory_is_empty() {
        let t = Temp::new();
        assert!(list(&t.0).is_empty());
        for text in ["{", "{}", "null", "[1, null, {}]"] {
            t.write("installed.json", text);
            assert!(list(&t.0).is_empty(), "{text}");
        }
    }

    #[test]
    fn list_validates_caps_and_reads_enablement_lines() {
        let t = Temp::new();
        t.write("installed.json", &serde_json::json!([
            {"package":"dev.plain", "label":" Plain ", "slots":"widgets", "enabled":false},
            {"package":"a..b"}, {"package":"bad\npkg"}, {"label":"missing"},
            {"package":"dev.restricted", "label":"é".repeat(60), "slots":"x".repeat(60)},
            {"package":"dev.off", "label":"  ", "enabled":true},
            {"package":"dev.missing"}
        ]).to_string());
        t.write("enabled", "# dev.off\n\ndev.plain\n dev.restricted : widgets\ndev.offish\n");
        let rows = list(&t.0);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].label, "Plain");
        assert!(rows[0].enabled);
        assert!(rows[1].enabled);
        assert_eq!(rows[1].label.chars().count(), 48);
        assert_eq!(rows[1].slots.chars().count(), 48);
        assert_eq!(rows[2].label, "dev.off");
        assert!(!rows[2].enabled);
        assert_eq!(rows[3].label, "dev.missing");
        assert_eq!(rows[3].slots, "");
        fs::remove_file(t.0.join("plugins/enabled")).unwrap();
        assert!(list(&t.0).iter().all(|row| !row.enabled));
        let entries: Vec<_> = (0..40).map(|i| serde_json::json!({"package":format!("dev.p{i}")})).collect();
        t.write("installed.json", &serde_json::to_string(&entries).unwrap());
        assert_eq!(list(&t.0).len(), 32);
    }

    #[test]
    fn off_removes_all_matching_lines_and_preserves_everything_else() {
        let t = Temp::new();
        t.write("enabled", "# owner\r\n\n dev.test : widgets \r\ndev.other:media\ndev.test\n# dev.test\ndev.test:navigation\ndev.last");
        set_enabled(&t.0, "dev.test", false).unwrap();
        assert_eq!(t.enabled(), "# owner\r\n\ndev.other:media\n# dev.test\ndev.last");
        assert!(!t.0.join("plugins/enabled.tmp").exists());
    }

    #[test]
    fn on_keeps_existing_restricted_lines_unchanged_or_appends() {
        let t = Temp::new();
        let original = "# owner\n\n dev.test : widgets \r\ndev.other\ndev.test:media";
        t.write("enabled", original);
        set_enabled(&t.0, "dev.test", true).unwrap();
        assert_eq!(t.enabled(), original);
        set_enabled(&t.0, "dev.new", true).unwrap();
        assert_eq!(t.enabled(), format!("{original}\ndev.new\n"));
        set_enabled(&t.0, "dev.new", true).unwrap();
        assert_eq!(t.enabled(), format!("{original}\ndev.new\n"));
    }

    #[test]
    fn creates_missing_directory_and_refuses_invalid_packages_without_writing() {
        let t = Temp::new();
        for package in ["bad\npkg", "a..b", ""] {
            for on in [false, true] {
                assert_eq!(set_enabled(&t.0, package, on).unwrap_err().kind(), io::ErrorKind::InvalidInput);
                assert!(!t.0.join("plugins").exists());
            }
        }
        set_enabled(&t.0, "dev.test", true).unwrap();
        assert_eq!(t.enabled(), "dev.test\n");
        assert!(set_enabled(&t.0, "bad\npkg", true).is_err());
        assert_eq!(t.enabled(), "dev.test\n");
    }

    #[test]
    fn failed_write_keeps_enabled_file() {
        let t = Temp::new();
        t.write("enabled", "dev.other:widgets\n");
        fs::create_dir(t.0.join("plugins/enabled.tmp")).unwrap();
        assert!(set_enabled(&t.0, "dev.test", true).is_err());
        assert_eq!(t.enabled(), "dev.other:widgets\n");
    }

    #[test]
    fn slots_display_as_words() {
        assert_eq!(slot_words("widgets,media,navigation"), "Widgets · Media · Navigation");
        assert_eq!(slot_words(" widgets, , media "), "Widgets · Media");
        assert_eq!(slot_words(""), "");
    }
}
