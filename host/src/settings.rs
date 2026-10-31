//! Host preferences, shared with the simulator.

use serde_json::{Map, Value as Json};
use slint_interpreter::{ComponentHandle, ComponentInstance, Value};
use std::cell::RefCell;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub type SharedSettings = Rc<RefCell<Settings>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NightMode { Off = 0, On = 1, Auto = 2 }

const NIGHT_START: u32 = 19 * 60;
const NIGHT_END: u32 = 6 * 60 + 30;

fn valid_minutes(minutes: u32) -> bool {
    minutes < 1440 && minutes % 15 == 0
}

/// Effective style only; unknown or untrusted time keeps Auto in day mode.
pub fn night_now(mode: NightMode, start: u32, end: u32, hhmm: Option<u32>, trusted: bool) -> bool {
    match mode {
        NightMode::Off => false,
        NightMode::On => true,
        NightMode::Auto => {
            let Some(hhmm) = hhmm.filter(|t| t / 100 < 24 && t % 100 < 60) else { return false };
            if !trusted || start == end { return false; }
            let minutes = hhmm / 100 * 60 + hhmm % 100;
            if start > end { minutes >= start || minutes < end }
            else { minutes >= start && minutes < end }
        }
    }
}

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

    pub fn night(&self) -> NightMode {
        match self.values.get("night").and_then(Json::as_str) {
            Some("on") => NightMode::On,
            Some("auto") => NightMode::Auto,
            _ => NightMode::Off,
        }
    }

    fn night_minutes(&self, key: &str, default: u32) -> u32 {
        self.values.get(key).and_then(Json::as_u64)
            .filter(|&n| n < 1440 && valid_minutes(n as u32))
            .map(|n| n as u32).unwrap_or(default)
    }

    pub fn night_start(&self) -> u32 {
        self.night_minutes("night-start", NIGHT_START)
    }

    pub fn night_end(&self) -> u32 {
        self.night_minutes("night-end", NIGHT_END)
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
            ("night", "off" | "on" | "auto") => Json::String(value.into()),
            ("night-start" | "night-end", _) => {
                let minutes = value.parse::<u32>().ok().filter(|&n| valid_minutes(n))
                    .ok_or_else(|| format!("settings: invalid {key}={value}"))?;
                Json::from(minutes)
            }
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

fn apply_night(ui: &ComponentInstance, enabled: bool) -> Result<(), String> {
    for global in ["Display", "HostState"] {
        if matches!(ui.get_global_property(global, "night"), Ok(Value::Bool(current)) if current == enabled) {
            continue;
        }
        ui.set_global_property(global, "night", Value::Bool(enabled))
            .map_err(|e| format!("{global}.night: {e:?}"))?;
    }
    Ok(())
}

pub fn refresh_night(ui: &ComponentInstance) -> Result<(), String> {
    let get = |prop: &str| ui.get_global_property("HostState", prop)
        .map_err(|e| format!("HostState.{prop}: {e:?}"));
    let mode = match get("night-mode")? {
        Value::Number(1.0) => NightMode::On,
        Value::Number(2.0) => NightMode::Auto,
        _ => NightMode::Off,
    };
    let start = match get("night-start")? { Value::Number(n) => n as u32, _ => NIGHT_START };
    let end = match get("night-end")? { Value::Number(n) => n as u32, _ => NIGHT_END };
    let hhmm = match get("clock-hhmm")? { Value::Number(n) if n >= 0.0 => Some(n as u32), _ => None };
    let trusted = matches!(get("clock-trusted")?, Value::Bool(true));
    apply_night(ui, night_now(mode, start, end, hhmm, trusted))
}

fn apply_preferences(ui: &ComponentInstance, settings: &SharedSettings, refresh_clock: fn(&ComponentInstance)) -> Result<(), String> {
    // Release the shared borrow before updating any UI property or callback.
    let (mode, start, end, clock24) = {
        let s = settings.borrow();
        (s.night(), s.night_start(), s.night_end(), s.clock24())
    };
    for (prop, value) in [("night-mode", mode as i32 as f64), ("night-start", start as f64), ("night-end", end as f64)] {
        ui.set_global_property("HostState", prop, Value::Number(value))
            .map_err(|e| format!("HostState.{prop}: {e:?}"))?;
    }
    for (prop, minutes) in [("night-start-text", start), ("night-end-text", end)] {
        let text = clock_text(minutes / 60 * 100 + minutes % 60, clock24);
        ui.set_global_property("HostState", prop, Value::String(text.into()))
            .map_err(|e| format!("HostState.{prop}: {e:?}"))?;
    }
    ui.set_property("clock24", Value::Bool(clock24))
        .map_err(|e| format!("clock24: {e:?}"))?;
    ui.set_global_property("HostState", "clock24", Value::Bool(clock24))
        .map_err(|e| format!("HostState.clock24: {e:?}"))?;
    refresh_clock(ui);
    refresh_night(ui)
}

pub fn bind(ui: &ComponentInstance, settings: SharedSettings, log: fn(&str), refresh_clock: fn(&ComponentInstance)) -> Result<(), String> {
    apply_preferences(ui, &settings, refresh_clock)?;
    let weak = ui.as_weak();
    ui.set_callback("set-setting", move |args| {
        if let [Value::String(key), Value::String(value)] = args {
            let saved = settings.borrow_mut().set(key.as_str(), value.as_str());
            let result = saved.and_then(|_| {
                if key != "setup" {
                    if let Some(ui) = weak.upgrade() { apply_preferences(&ui, &settings, refresh_clock)?; }
                }
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
        assert_eq!(settings.night(), NightMode::Off);
        assert_eq!(settings.night_start(), NIGHT_START);
        assert_eq!(settings.night_end(), NIGHT_END);
        assert!(settings.clock24());
        assert!(!dir.file().exists());
    }

    #[test]
    fn corrupt_file_is_renamed_and_defaults_used() {
        let dir = Dir::new();
        for bytes in ["{broken", "[]", "null"] {
            fs::write(dir.file(), bytes).unwrap();
            let settings = Settings::load(Some(&dir.file()), |_| {});
            assert_eq!(settings.night(), NightMode::Off);
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
        assert_eq!(settings.night(), NightMode::Auto);
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
        assert_eq!(Settings::load(Some(&dir.file()), |_| {}).night(), NightMode::On);
        let tmp = sibling(&dir.file(), ".tmp");
        assert!(!tmp.exists());
        fs::create_dir(tmp).unwrap();
        assert!(settings.set("night", "off").is_err());
        assert_eq!(settings.night(), NightMode::On);
        assert_eq!(Settings::load(Some(&dir.file()), |_| {}).night(), NightMode::On);
    }

    #[test]
    fn invalid_changes_are_rejected() {
        let mut settings = Settings::default();
        for (key, value) in [("night", "true"), ("clock24", "on"), ("future", "x")] {
            assert!(settings.set(key, value).is_err());
        }
        assert!(settings.values.is_empty());
    }

    #[test]
    fn night_schedule_wraps_and_uses_half_open_boundaries() {
        for (hhmm, expected) in [(0, true), (629, true), (630, false), (1859, false), (1900, true), (2359, true)] {
            assert_eq!(night_now(NightMode::Auto, NIGHT_START, NIGHT_END, Some(hhmm), true), expected, "{hhmm}");
        }
        for (hhmm, expected) in [(859, false), (900, true), (1200, true), (1659, true), (1700, false), (2359, false)] {
            assert_eq!(night_now(NightMode::Auto, 540, 1020, Some(hhmm), true), expected, "{hhmm}");
        }
    }

    #[test]
    fn equal_schedule_untrusted_and_unknown_clock_use_day() {
        for hhmm in [None, Some(0), Some(630), Some(1900), Some(2359), Some(2400), Some(1960)] {
            assert!(!night_now(NightMode::Auto, NIGHT_START, NIGHT_END, hhmm, false));
            assert!(!night_now(NightMode::Auto, 390, 390, hhmm, true));
            assert!(!night_now(NightMode::Off, NIGHT_START, NIGHT_END, hhmm, true));
            assert!(night_now(NightMode::On, NIGHT_START, NIGHT_END, hhmm, false));
        }
        for hhmm in [None, Some(2400), Some(1960), Some(u32::MAX)] {
            assert!(!night_now(NightMode::Auto, NIGHT_START, NIGHT_END, hhmm, true));
        }
    }

    #[test]
    fn night_modes_have_distinct_ui_choice_indices() {
        let mut settings = Settings::default();
        for (mode, index) in [("off", 0), ("on", 1), ("auto", 2)] {
            settings.set("night", mode).unwrap();
            assert_eq!(settings.night() as i32, index);
        }
        for bad in [serde_json::json!("AUTO"), Json::Null, Json::Bool(true), Json::from(2)] {
            settings.values.insert("night".into(), bad);
            assert_eq!(settings.night(), NightMode::Off);
        }
    }

    #[test]
    fn schedule_keys_require_json_integers_in_quarter_hours() {
        let dir = Dir::new();
        let mut settings = Settings::load(Some(&dir.file()), |_| {});
        settings.set("night", "auto").unwrap();
        for minutes in [0, 15, 390, 1140, 1425] {
            for key in ["night-start", "night-end"] {
                settings.set(key, &minutes.to_string()).unwrap();
            }
            let saved = Settings::load(Some(&dir.file()), |_| panic!("valid JSON"));
            assert_eq!(saved.night_start(), minutes);
            assert_eq!(saved.night_end(), minutes);
            assert_eq!(saved.values["night-start"], Json::from(minutes));
            assert_eq!(saved.values["night-end"], Json::from(minutes));
            assert_eq!(saved.night(), NightMode::Auto);
        }
        for invalid in ["", "-15", "1", "1439", "1440", "1140.0", "19:00", " 390", "99999999999999999999"] {
            for key in ["night-start", "night-end"] {
                assert!(settings.set(key, invalid).is_err());
            }
            assert_eq!(settings.night_start(), 1425);
            assert_eq!(settings.night_end(), 1425);
        }
        for invalid in [Json::Null, Json::Bool(true), Json::from("1140"), Json::from(-15),
            Json::from(1), Json::from(1439), Json::from(1440), Json::from(1140.0), serde_json::json!([])] {
            fs::write(dir.file(), serde_json::to_vec(&serde_json::json!({"night-start": invalid, "night-end": invalid})).unwrap()).unwrap();
            let saved = Settings::load(Some(&dir.file()), |_| panic!("valid JSON"));
            assert_eq!(saved.night_start(), NIGHT_START);
            assert_eq!(saved.night_end(), NIGHT_END);
        }
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
