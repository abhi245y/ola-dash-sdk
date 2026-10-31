//! Host preferences, shared with the simulator.

use serde_json::{Map, Value as Json};
use slint_interpreter::{ComponentHandle, ComponentInstance, Value};
use std::cell::RefCell;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub type SharedSettings = Rc<RefCell<Settings>>;

#[derive(Default)]
pub struct Settings {
    path: Option<PathBuf>,
    values: Map<String, Json>,
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    name.into()
}

fn write(path: &Path, values: &Map<String, Json>) -> io::Result<()> {
    let tmp = sibling(path, ".tmp");
    let mut file = fs::File::create(&tmp)?;
    file.write_all(&serde_json::to_vec(values)?)?;
    file.sync_all()?;
    fs::rename(tmp, path)
}

impl Settings {
    pub fn load(path: Option<&Path>, log: fn(&str)) -> Self {
        let mut settings = Self { path: path.map(Path::to_path_buf), ..Self::default() };
        let Some(path) = path else { return settings };
        match fs::read(path) {
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(values) => settings.values = values,
                Err(e) => {
                    let renamed = fs::rename(path, sibling(path, ".bad"));
                    log(&format!("settings: invalid JSON ({e}); using defaults; .bad rename: {renamed:?}"));
                }
            },
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => log(&format!("settings: {e}; using defaults")),
        }
        settings
    }

    pub fn night(&self) -> bool {
        self.values.get("night").and_then(Json::as_str) == Some("on")
    }

    pub fn clock24(&self) -> bool {
        self.values.get("clock24").and_then(Json::as_bool).unwrap_or(true)
    }

    pub fn setup_done(&self) -> bool {
        self.values.get("setup").and_then(Json::as_str) == Some("done")
    }

    pub fn option(&self, theme: &str, id: &str) -> Option<&Json> {
        self.values.get("theme-options")?.as_object()?.get(theme)?.as_object()?.get(id)
    }

    pub fn set_option(&mut self, theme: &str, id: &str, json: Json) -> Result<(), String> {
        let mut values = self.values.clone();
        let options = values.entry("theme-options").or_insert_with(|| Json::Object(Map::new()));
        if !options.is_object() { *options = Json::Object(Map::new()); }
        let theme = options.as_object_mut().unwrap().entry(theme).or_insert_with(|| Json::Object(Map::new()));
        if !theme.is_object() { *theme = Json::Object(Map::new()); }
        theme.as_object_mut().unwrap().insert(id.into(), json);
        self.save(values)
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        let value = match (key, value) {
            ("night", "off" | "on") => Json::String(value.into()),
            ("clock24", "true" | "false") => Json::Bool(value == "true"),
            ("setup", "done") => Json::String(value.into()),
            _ => return Err(format!("settings: invalid {key}={value}")),
        };
        let mut values = self.values.clone();
        values.insert(key.into(), value);
        self.save(values)
    }

    fn save(&mut self, values: Map<String, Json>) -> Result<(), String> {
        if let Some(path) = &self.path {
            write(path, &values).map_err(|e| format!("settings: write failed: {e}"))?;
        }
        self.values = values;
        Ok(())
    }
}

fn apply(ui: &ComponentInstance, key: &str, enabled: bool, refresh_clock: fn(&ComponentInstance)) -> Result<(), String> {
    if key == "setup" {
        return Ok(());
    } else if key == "clock24" {
        ui.set_property("clock24", Value::Bool(enabled))
            .map_err(|e| format!("clock24: {e:?}"))?;
        ui.set_global_property("HostState", "clock24", Value::Bool(enabled))
            .map_err(|e| format!("HostState.clock24: {e:?}"))?;
        refresh_clock(ui);
    } else {
        for global in ["Display", "HostState"] {
            ui.set_global_property(global, "night", Value::Bool(enabled))
                .map_err(|e| format!("{global}.night: {e:?}"))?;
        }
    }
    Ok(())
}

pub fn bind(ui: &ComponentInstance, settings: SharedSettings, log: fn(&str), refresh_clock: fn(&ComponentInstance)) -> Result<(), String> {
    let preferences = { let s = settings.borrow(); [("night", s.night()), ("clock24", s.clock24())] };
    for (key, enabled) in preferences {
        apply(ui, key, enabled, refresh_clock)?;
    }
    let weak = ui.as_weak();
    ui.set_callback("set-setting", move |args| {
        if let [Value::String(key), Value::String(value)] = args {
            let saved = {
                let mut settings = settings.borrow_mut();
                settings.set(key.as_str(), value.as_str()).map(|_| {
                    if key == "clock24" { settings.clock24() } else { settings.night() }
                })
            };
            let result = saved.and_then(|enabled| {
                if let Some(ui) = weak.upgrade() { apply(&ui, key.as_str(), enabled, refresh_clock)?; }
                Ok(())
            });
            if let Err(e) = result { log(&e); }
        }
        Value::Void
    }).map_err(|e| format!("set-setting: {e:?}"))
}

pub fn clock_text(hhmm: u32, clock24: bool) -> String {
    let (hour, minute) = (hhmm / 100, hhmm % 100);
    if clock24 {
        format!("{hour:02}:{minute:02}")
    } else {
        format!("{}:{minute:02} {}", (hour + 11) % 12 + 1, if hour < 12 { "AM" } else { "PM" })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Dir(PathBuf);
    impl Dir {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!("oladash-settings-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn file(&self) -> PathBuf { self.0.join("settings.json") }
    }
    impl Drop for Dir {
        fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
    }

    #[test]
    fn missing_file_uses_defaults_without_writing() {
        let dir = Dir::new();
        let settings = Settings::load(Some(&dir.file()), |_| panic!("missing file must be quiet"));
        assert!(!settings.night());
        assert!(settings.clock24());
        assert!(!dir.file().exists());
    }

    #[test]
    fn corrupt_file_is_renamed_and_defaults_used() {
        let dir = Dir::new();
        for bytes in ["{broken", "[]", "null"] {
            fs::write(dir.file(), bytes).unwrap();
            let settings = Settings::load(Some(&dir.file()), |_| {});
            assert!(!settings.night());
            assert!(settings.clock24());
            assert!(!dir.file().exists());
            assert_eq!(fs::read_to_string(sibling(&dir.file(), ".bad")).unwrap(), bytes);
        }
    }

    #[test]
    fn unknown_keys_and_auto_survive_rewrite() {
        let dir = Dir::new();
        fs::write(dir.file(), r#"{"night":"auto","future":{"values":[1,true,"x"]}}"#).unwrap();
        let mut settings = Settings::load(Some(&dir.file()), |_| panic!("valid JSON"));
        assert!(!settings.night());
        settings.set("clock24", "false").unwrap();
        let saved = Settings::load(Some(&dir.file()), |_| panic!("valid JSON"));
        assert_eq!(saved.values["future"], settings.values["future"]);
        assert_eq!(saved.values["night"], "auto");
        assert!(!saved.clock24());
    }

    #[test]
    fn atomic_write_replaces_file_and_keeps_original_on_failure() {
        let dir = Dir::new();
        fs::write(dir.file(), r#"{"night":"off"}"#).unwrap();
        let mut settings = Settings::load(Some(&dir.file()), |_| {});
        settings.set("night", "on").unwrap();
        assert!(Settings::load(Some(&dir.file()), |_| {}).night());
        let tmp = sibling(&dir.file(), ".tmp");
        assert!(!tmp.exists());
        fs::create_dir(tmp).unwrap();
        assert!(settings.set("night", "off").is_err());
        assert!(settings.night());
        assert!(Settings::load(Some(&dir.file()), |_| {}).night());
    }

    #[test]
    fn invalid_changes_are_rejected() {
        let mut settings = Settings::default();
        for (key, value) in [("night", "auto"), ("night", "true"), ("clock24", "on"), ("future", "x")] {
            assert!(settings.set(key, value).is_err());
        }
        assert!(settings.values.is_empty());
    }

    #[test]
    fn theme_options_round_trip_with_other_preferences_and_unknown_keys() {
        let dir = Dir::new();
        fs::write(dir.file(), r##"{"night":"auto","future":{"nested":[1,true]},"theme-options":{"other":{"accent":"#ffffff"},"theme":{"future-option":[4]}}}"##).unwrap();
        let settings = Rc::new(RefCell::new(Settings::load(Some(&dir.file()), |_| {})));
        let option_writer = settings.clone();
        for (id, json) in [("accent", serde_json::json!("#123456")), ("enabled", serde_json::json!(false)),
            ("count", serde_json::json!(-2)), ("style", serde_json::json!("thin"))] {
            option_writer.borrow_mut().set_option("theme", id, json).unwrap();
        }
        settings.borrow_mut().set("clock24", "false").unwrap();
        option_writer.borrow_mut().set_option("theme", "count", Json::from(8)).unwrap();
        let saved = Settings::load(Some(&dir.file()), |_| panic!("valid JSON"));
        assert_eq!(saved.option("theme", "count"), Some(&Json::from(8)));
        assert_eq!(saved.option("theme", "enabled"), Some(&Json::Bool(false)));
        assert_eq!(saved.option("theme", "accent"), Some(&Json::from("#123456")));
        assert_eq!(saved.option("theme", "style"), Some(&Json::from("thin")));
        assert_eq!(saved.option("other", "accent"), Some(&Json::from("#ffffff")));
        assert_eq!(saved.option("theme", "future-option"), Some(&serde_json::json!([4])));
        assert!(saved.option("missing", "count").is_none());
        assert_eq!(saved.values["future"], serde_json::json!({"nested":[1,true]}));
        assert_eq!(saved.values["night"], "auto");
        assert!(!saved.clock24());
    }

    #[test]
    fn set_option_creates_missing_file_and_keeps_state_on_failed_write() {
        let dir = Dir::new();
        let mut settings = Settings::load(Some(&dir.file()), |_| panic!("missing file is quiet"));
        settings.set_option("theme", "count", Json::from(3)).unwrap();
        assert_eq!(Settings::load(Some(&dir.file()), |_| {}).option("theme", "count"), Some(&Json::from(3)));
        assert!(!sibling(&dir.file(), ".tmp").exists());
        fs::create_dir(sibling(&dir.file(), ".tmp")).unwrap();
        assert!(settings.set_option("theme", "count", Json::from(4)).is_err());
        assert_eq!(settings.option("theme", "count"), Some(&Json::from(3)));
        assert_eq!(Settings::load(Some(&dir.file()), |_| {}).option("theme", "count"), Some(&Json::from(3)));
    }

    #[test]
    fn malformed_option_containers_use_defaults_and_can_be_replaced() {
        let dir = Dir::new();
        for json in [r#"{"theme-options":false,"future":7}"#, r#"{"theme-options":{"theme":null,"other":{"count":2}},"future":7}"#] {
            fs::write(dir.file(), json).unwrap();
            let mut settings = Settings::load(Some(&dir.file()), |_| {});
            assert!(settings.option("theme", "count").is_none());
            settings.set_option("theme", "count", Json::from(3)).unwrap();
            assert_eq!(settings.option("theme", "count"), Some(&Json::from(3)));
            assert_eq!(settings.values["future"], Json::from(7));
        }
    }

    #[test]
    fn setup_accepts_only_done_and_survives_reload() {
        let dir = Dir::new();
        let mut settings = Settings::load(Some(&dir.file()), |_| {});
        assert!(!settings.setup_done());
        for value in ["", "on", "true", "Done", "pending"] {
            assert!(settings.set("setup", value).is_err());
            assert!(!settings.setup_done());
        }
        settings.set("setup", "done").unwrap();
        assert!(settings.setup_done());
        assert!(Settings::load(Some(&dir.file()), |_| {}).setup_done());
        assert!(settings.set("setup", "false").is_err());
        assert!(settings.setup_done());
        for value in [r#""pending""#, "true", "null"] {
            fs::write(dir.file(), format!(r#"{{"setup":{value}}}"#)).unwrap();
            assert!(!Settings::load(Some(&dir.file()), |_| {}).setup_done());
        }
    }

    #[test]
    fn clock_formats_midnight_noon_and_evening() {
        for (hhmm, day, half) in [(0, "00:00", "12:00 AM"), (924, "09:24", "9:24 AM"), (1200, "12:00", "12:00 PM"), (1824, "18:24", "6:24 PM")] {
            assert_eq!(clock_text(hhmm, true), day);
            assert_eq!(clock_text(hhmm, false), half);
        }
    }
}
