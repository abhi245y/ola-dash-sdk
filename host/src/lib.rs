//! Ola dash host. Loads a Slint theme at runtime, drives its `Vehicle` global
//! from the vehicle-parser binder (values arrive from Java over JNI), and draws
//! the host-owned safety overlay above the theme.

use std::cell::RefCell;
use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use android_activity::AndroidApp;
use jni::objects::{JClass, JObject, JString, JValue};
use jni::{jni_sig, jni_str, EnvUnowned, JavaVM};
use slint::{ModelRc, VecModel};
use slint_interpreter::{Compiler, ComponentHandle, ComponentInstance, Value};

mod plugins;
mod settings;
mod theme_options;
mod theme_root;
mod theme_switch;
use theme_switch::DEFAULT_THEME;

// Field ids. Must match VehicleBridge.java.
const SPEED_KMH: usize = 1;
const BATTERY_PCT: usize = 2;
const RANGE_KM: usize = 3;
const MODE: usize = 4;
const CHARGING: usize = 5;
const ODO_KM: usize = 6;
const TRIP_KM: usize = 7;
const LEFT_IND: usize = 8;
const RIGHT_IND: usize = 9;
const HIGH_BEAM: usize = 10;
const HEADLIGHT: usize = 11;
const LOCKED: usize = 12;
const READY: usize = 13;
const HYPER_AVAILABLE: usize = 14;
const ALERT_CODE: usize = 20;
const SIDE_STAND: usize = 21;
const FAULT: usize = 22;
const DATE_YMD: usize = 23;
const CLOCK_HHMM: usize = 30;
const TEST_MODE: usize = 31;
const AUTH_PHASE: usize = 32;
const PIN_LENGTH: usize = 33;
const AUTH_ATTEMPTS: usize = 34;
const AUTH_ERROR: usize = 35;
const AUTH_COOLDOWN: usize = 36;
const AUTH_AVAILABLE: usize = 37;
const DATA_STALE: usize = 38;
const PARKED: usize = 39;
// Contract v6. Trunk latch (true = open). Java side: VehicleBridge.TRUNK_OPEN.
const TRUNK_OPEN: usize = 16;
// Contract v5 (ids 15, 40+). Java side: VehicleBridge.java, same numbers.
const VEHICLE_STATE: usize = 15;
const SOC_PCT: usize = 40;
const BAT_CURRENT: usize = 41;
const BUS_VOLTAGE: usize = 42;
const TEMP_MIN: usize = 43;
const TEMP_MAX: usize = 44;
const HEALTH: usize = 45;
const FRONT_BRAKE: usize = 46;
const REAR_BRAKE: usize = 47;
const REGEN: usize = 48;
const MOTOR_TEMP: usize = 49;
const MCU_TEMP: usize = 50;
// Host-only clock trust; never part of vehicle replay.
const CLOCK_TRUSTED: usize = 51;
const SLOTS: usize = 52;

const MODE_NAMES: [&str; 9] =
    ["eco", "normal", "sport", "hyper", "unknown", "park", "reverse", "custom", "easy-park"];
const STATE_NAMES: [&str; 6] = ["unknown", "sleep", "idle", "park", "drive", "charging"];
const REGEN_NAMES: [&str; 3] = ["none", "coast", "brake"];
const FALLBACK: &str = "fallback"; // bundled copy of themes/default
const LAST_RESORT: &str = "last-resort"; // bundled placeholder, cannot fail to compile
const HEADUNIT_PKG: &str = "com.andrerinas.headunitrevived";
/// Highest contract version this host implements (contract/vehicle.slint `contract-version`).
const HOST_CONTRACT: i64 = 7;

/// Latest value per field, coalesced until the UI thread applies it.
static PENDING: Mutex<[Option<f64>; SLOTS]> = Mutex::new([None; SLOTS]);
static SCHEDULED: AtomicBool = AtomicBool::new(false);
/// Last raw clock value, or u32::MAX until the first clock update arrives.
static LAST_HHMM: AtomicU32 = AtomicU32::new(u32::MAX);
/// Any touch or key input since the keep-alive last asked (stock HMI: mTouchReceived).
static TOUCHED: AtomicBool = AtomicBool::new(false);
/// Latest plugin snapshot from PluginHost.java (JSON), applied on the UI thread.
static PLUGINS: Mutex<Option<String>> = Mutex::new(None);

/// Called once a second by the Java keep-alive tick; reading clears the flag.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_olatool_dash_VehicleBridge_nativeTakeTouched(
    _env: *mut c_void,
    _class: *mut c_void,
) -> u8 {
    TOUCHED.swap(false, Ordering::AcqRel) as u8
}

thread_local! {
    static UI: RefCell<Option<ComponentInstance>> = const { RefCell::new(None) };
}

/// Called by VehicleBridge.java from binder and main threads.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_olatool_dash_VehicleBridge_nativeSet(
    _env: *mut c_void,
    _class: *mut c_void,
    id: i32,
    value: f64,
) {
    let Ok(slot) = usize::try_from(id) else { return };
    if slot >= SLOTS {
        return;
    }
    PENDING.lock().unwrap()[slot] = Some(value);
    schedule();
}

/// Called by PluginHost.java with a full snapshot of widgets, media and navigation.
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_olatool_dash_plugin_PluginHost_nativeSetPlugins<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    json: JString<'local>,
) {
    let text = env
        .with_env(|env| json.try_to_string(env))
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
    *PLUGINS.lock().unwrap() = Some(text);
    schedule();
}

/// Queues one UI-thread flush. Values written before it runs are coalesced into it.
fn schedule() {
    if !SCHEDULED.swap(true, Ordering::AcqRel) && slint::invoke_from_event_loop(flush).is_err() {
        SCHEDULED.store(false, Ordering::Release);
    }
}

/// UI thread: apply coalesced values. Keeps pending values until the UI exists.
fn flush() {
    SCHEDULED.store(false, Ordering::Release);
    UI.with(|slot| {
        let slot = slot.borrow();
        let Some(ui) = slot.as_ref() else { return };
        let vals = std::mem::replace(&mut *PENDING.lock().unwrap(), [None; SLOTS]);
        for (id, v) in vals.iter().enumerate() {
            if let Some(v) = *v {
                apply(ui, id, v);
            }
        }
        if let Some(json) = PLUGINS.lock().unwrap().take() {
            apply_plugins(ui, &json);
        }
    });
}

/// Applies a PluginHost snapshot:
/// {"widgets":[{id,title,value,unit}], "media":{title,artist,playing}, "nav":{active,instruction,distance,eta}}
fn apply_plugins(ui: &ComponentInstance, json: &str) {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        log::warn!("plugin snapshot is not JSON");
        return;
    };
    let widgets: Vec<Value> = v["widgets"]
        .as_array()
        .map(|a| a.iter().map(|w| strct(w, &["id", "title", "value", "unit"])).collect())
        .unwrap_or_default();
    let set = |global: &str, prop: &str, val: Value| {
        if let Err(e) = ui.set_global_property(global, prop, val) {
            log::warn!("set {global}.{prop} failed: {e:?}");
        }
    };
    set("Plugins", "widgets", Value::Model(ModelRc::new(VecModel::from(widgets))));
    let m = &v["media"];
    set("Media", "title", text(m, "title"));
    set("Media", "artist", text(m, "artist"));
    set("Media", "playing", Value::Bool(m["playing"].as_bool().unwrap_or(false)));
    let n = &v["nav"];
    set("Navigation", "active", Value::Bool(n["active"].as_bool().unwrap_or(false)));
    set("Navigation", "instruction", text(n, "instruction"));
    set("Navigation", "distance", text(n, "distance"));
    set("Navigation", "eta", text(n, "eta"));
}

fn text(obj: &serde_json::Value, key: &str) -> Value {
    Value::String(obj[key].as_str().unwrap_or("").into())
}

fn strct(obj: &serde_json::Value, keys: &[&str]) -> Value {
    Value::Struct(keys.iter().map(|k| (k.to_string(), text(obj, k))).collect())
}

/// Asks MainActivity.onOpenTrunk to send the host-gated trunk request (VehicleBridge.openTrunk).
fn request_trunk(app: &AndroidApp) {
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut _) };
    let res = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
        let activity = ManuallyDrop::new(unsafe {
            JObject::from_raw(&*env, app.activity_as_ptr() as jni::sys::jobject)
        });
        env.call_method(&*activity, jni_str!("onOpenTrunk"), jni_sig!("()V"), &[])?;
        Ok(())
    });
    if res.is_err() {
        log::warn!("trunk bridge unavailable");
    }
}

/// Sends media, widget or host refresh actions through MainActivity.onDashAction.
fn dash_action(app: &AndroidApp, action: &str) {
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut _) };
    let res = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
        let activity = ManuallyDrop::new(unsafe {
            JObject::from_raw(&*env, app.activity_as_ptr() as jni::sys::jobject)
        });
        let s = env.new_string(action)?;
        env.call_method(&*activity, jni_str!("onDashAction"), jni_sig!("(Ljava/lang/String;)V"),
            &[JValue::Object(&s)])?;
        Ok(())
    });
    if res.is_err() {
        log::warn!("action bridge unavailable");
    }
}

fn apply(ui: &ComponentInstance, id: usize, v: f64) {
    let b = Value::Bool(v != 0.0);
    let n = Value::Number(v);
    let (global, prop, val) = match id {
        SPEED_KMH => ("Vehicle", "speed-kmh", n),
        BATTERY_PCT => ("Vehicle", "battery-pct", n),
        RANGE_KM => ("Vehicle", "range-km", n),
        MODE => {
            let name = MODE_NAMES.get(v as usize).copied().unwrap_or("unknown");
            ("Vehicle", "mode", Value::EnumerationValue("DriveMode".into(), name.into()))
        }
        CHARGING => ("Vehicle", "charging", b),
        TRUNK_OPEN => ("Vehicle", "trunk-open", b),
        ODO_KM => ("Vehicle", "odometer-km", n),
        TRIP_KM => ("Vehicle", "trip-km", n),
        LEFT_IND => ("Vehicle", "left-indicator", b),
        RIGHT_IND => ("Vehicle", "right-indicator", b),
        HIGH_BEAM => ("Vehicle", "high-beam", b),
        HEADLIGHT => ("Vehicle", "headlight", b),
        LOCKED => ("Vehicle", "locked", b),
        READY => ("Vehicle", "ready-to-ride", b),
        HYPER_AVAILABLE => ("Vehicle", "hyper-available", b),
        CLOCK_HHMM => {
            let hhmm = if v.is_finite() && v >= 0.0 && v.fract() == 0.0
                && v < 2400.0 && v as u32 % 100 < 60 { v as u32 } else { u32::MAX };
            LAST_HHMM.store(hhmm, Ordering::Relaxed);
            refresh_clock(ui);
            return;
        }
        CLOCK_TRUSTED => {
            if let Err(e) = ui.set_global_property("HostState", "clock-trusted", b) {
                log::warn!("set HostState.clock-trusted failed: {e:?}");
            }
            if let Err(e) = settings::refresh_night(ui) { log_warn(&e); }
            return;
        }
        DATE_YMD => ("HostState", "date-text", Value::String(theme_root::date_text(v as u32).into())),
        ALERT_CODE => ("HostState", "alert-code", n),
        SIDE_STAND => ("HostState", "side-stand", b),
        FAULT => ("HostState", "fault", b),
        TEST_MODE => ("HostState", "test-mode", b),
        AUTH_PHASE => ("HostState", "auth-phase", n),
        PIN_LENGTH => ("HostState", "pin-length", n),
        AUTH_ATTEMPTS => ("HostState", "auth-attempts", n),
        AUTH_ERROR => ("HostState", "auth-error", n),
        AUTH_COOLDOWN => ("HostState", "auth-cooldown", n),
        AUTH_AVAILABLE => ("HostState", "auth-available", b),
        DATA_STALE => ("Vehicle", "data-stale", b), // HOST_COPY mirrors it into HostState
        PARKED => ("HostState", "parked", b),
        VEHICLE_STATE => {
            let name = STATE_NAMES.get(v as usize).copied().unwrap_or("unknown");
            ("Vehicle", "state", Value::EnumerationValue("VehicleState".into(), name.into()))
        }
        SOC_PCT => ("Battery", "soc-pct", n),
        BAT_CURRENT => ("Battery", "current-a", n),
        BUS_VOLTAGE => ("Battery", "voltage-v", n),
        TEMP_MIN => ("Battery", "temp-min-c", n),
        TEMP_MAX => ("Battery", "temp-max-c", n),
        HEALTH => ("Battery", "health-pct", n),
        FRONT_BRAKE => ("Ride", "front-brake", b),
        REAR_BRAKE => ("Ride", "rear-brake", b),
        REGEN => {
            let name = REGEN_NAMES.get(v as usize).copied().unwrap_or("none");
            ("Ride", "regen", Value::EnumerationValue("Regen".into(), name.into()))
        }
        MOTOR_TEMP => ("Ride", "motor-temp-c", n),
        MCU_TEMP => ("Ride", "mcu-temp-c", n),
        _ => return,
    };
    let copy = theme_root::HOST_COPY.iter().find(|(p, _)| global == "Vehicle" && *p == prop);
    if let Some((_, host)) = copy {
        if let Err(e) = ui.set_global_property("HostState", host, val.clone()) {
            log::warn!("set HostState.{host} failed: {e:?}");
        }
    }
    if let Err(e) = ui.set_global_property(global, prop, val) {
        log::warn!("set {global}.{prop} failed: {e:?}");
    }
}

fn refresh_clock(ui: &ComponentInstance) {
    let hhmm = LAST_HHMM.load(Ordering::Relaxed);
    let clock24 = !matches!(ui.get_property("clock24"), Ok(Value::Bool(false)));
    let number = if hhmm == u32::MAX { -1.0 } else { hhmm as f64 };
    if let Err(e) = ui.set_global_property("HostState", "clock-hhmm", Value::Number(number)) {
        log::warn!("set HostState.clock-hhmm failed: {e:?}");
    }
    for (global, format24) in [("HostState", clock24), ("Vehicle", true)] {
        let text = if hhmm == u32::MAX { String::new() } else { settings::clock_text(hhmm, format24) };
        if let Err(e) = ui.set_global_property(global, "clock-text", Value::String(text.into())) {
            log::warn!("set {global}.clock-text failed: {e:?}");
        }
    }
    if let Err(e) = settings::refresh_night(ui) { log_warn(&e); }
}

/// Reads `theme.json` in the theme folder (relative to `files/`) and returns its entry file.
/// Rejects a theme that needs a newer contract than this host, or a missing or escaping entry.
fn theme_entry(files: &Path, rel: &str) -> Result<String, String> {
    let dir = files.join(rel);
    let text = std::fs::read_to_string(dir.join("theme.json"))
        .map_err(|e| format!("{rel}/theme.json: {e}"))?;
    let m: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("{rel}/theme.json: {e}"))?;
    let contract = m["contract"].as_i64().ok_or("theme.json: contract missing")?;
    if contract > HOST_CONTRACT {
        return Err(format!("{rel}: needs contract {contract}, host has {HOST_CONTRACT}"));
    }
    let entry = m["entry"].as_str().ok_or("theme.json: entry missing")?;
    let escapes = entry.is_empty() || entry.starts_with('/') || entry.split('/').any(|s| s == "..");
    if escapes || !dir.join(entry).is_file() {
        return Err(format!("{rel}: entry '{entry}' missing or outside the theme folder"));
    }
    Ok(entry.to_string())
}

/// `timing <stage> <N> ms` lines, parsed by ola-dash tools/measure.sh.
fn timing(stage: &str, since: Instant) {
    log::info!("timing {stage} {} ms", since.elapsed().as_millis());
}

fn build_root(files: &Path, theme: &str, active_theme: &str, settings: settings::SharedSettings) -> Result<ComponentInstance, String> {
    // Folder relative to files/, and the entry file inside it.
    let (rel, entry) = match theme {
        // The default theme ships inside the APK as the fallback copy.
        DEFAULT_THEME | FALLBACK => ("fallback".to_string(), theme_entry(files, "fallback")?),
        // Hard-coded placeholder: no manifest, cannot fail to compile.
        LAST_RESORT => ("last-resort".to_string(), "main.slint".to_string()),
        t => (format!("themes/{t}"), theme_entry(files, &format!("themes/{t}"))?),
    };
    let include = vec![files.join("contract")];
    // Probe compile of the theme alone: which optional Dash properties it defines.
    let t = Instant::now();
    let mut probe = Compiler::new();
    probe.set_include_paths(include.clone());
    let entry_path = files.join(&rel).join(&entry);
    let (contract, theme_dir, host) = (files.join("contract"), files.join(&rel), files.join("host"));
    theme_root::guard(&mut probe, &[&contract], &theme_dir, &entry_path, true)?;
    let probed = spin_on::spin_on(probe.build_from_path(&entry_path));
    let declared = theme_root::declared(&probed)?;
    // Invalid options disable only options, never an otherwise valid theme.
    let options = if theme == LAST_RESORT {
        Vec::new()
    } else {
        let text = std::fs::read_to_string(theme_dir.join("theme.json")).map_err(|e| e.to_string())?;
        let manifest = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        theme_options::parse(&manifest).unwrap_or_else(|errors| {
            log::warn!("theme options ignored: {}", errors.join("; "));
            Vec::new()
        })
    };
    let options = theme_root::kept_options(&probed, &options, |id, why| {
        log::warn!("theme option '{id}' skipped: {why}");
    })?;
    timing("probe", t);
    let t = Instant::now();
    let src = theme_root::root_source(&format!("../{rel}/{entry}"), &declared, &options);
    let mut compiler = Compiler::new();
    compiler.set_include_paths(include);
    theme_root::guard(&mut compiler, &[&contract, &host], &theme_dir, &entry_path, false)?;
    let result = spin_on::spin_on(compiler.build_from_source(src, files.join("host/root.slint")));
    if result.has_errors() {
        let msgs: Vec<String> = result.diagnostics().map(|d| d.to_string()).collect();
        return Err(msgs.join("\n"));
    }
    timing("root", t);
    let def = result.component("Host").ok_or("no Host component")?;
    let t = Instant::now();
    let ui = def.create().map_err(|e| e.to_string())?;
    timing("create", t);
    theme_options::bind(&ui, &options, active_theme, settings, &[], log_warn)?;
    theme_root::bind(&ui, log_warn)?;
    Ok(ui)
}

fn log_warn(msg: &str) {
    log::warn!("{msg}");
}

/// Theme first; any compile or create failure falls back to the bundled placeholder.
fn load_ui(files: &Path, settings: settings::SharedSettings) -> Result<ComponentInstance, String> {
    let theme = theme_switch::active(files);
    build_root(files, &theme, &theme, settings.clone())
        .or_else(|e| {
            log::error!("theme '{theme}' rejected, using fallback:\n{e}");
            // Options and saved values belong to the theme actually rendered.
            build_root(files, FALLBACK, DEFAULT_THEME, settings.clone())
        })
        .or_else(|e| {
            log::error!("fallback rejected, using last-resort:\n{e}");
            build_root(files, LAST_RESORT, DEFAULT_THEME, settings)
        })
}

/// Start an activity from the NativeActivity through JNI (no app classes needed).
fn launch(app: &AndroidApp, pkg: &str) {
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut _) };
    let activity_ptr = app.activity_as_ptr() as jni::sys::jobject;
    let res = vm.attach_current_thread(|env| -> jni::errors::Result<bool> {
        // The NativeActivity ref is global, owned by the framework: never delete it.
        let activity = ManuallyDrop::new(unsafe { JObject::from_raw(&*env, activity_ptr) });
        let pm = env
            .call_method(
                &*activity,
                jni_str!("getPackageManager"),
                jni_sig!("()Landroid/content/pm/PackageManager;"),
                &[],
            )?
            .l()?;
        let name = env.new_string(pkg)?;
        let intent = env.call_method(
            &pm,
            jni_str!("getLaunchIntentForPackage"),
            jni_sig!("(Ljava/lang/String;)Landroid/content/Intent;"),
            &[JValue::Object(&name)],
        )?
        .l()?;
        if intent.is_null() {
            log::warn!("launch: target not installed");
            return Ok(false);
        }
        env.call_method(
            &*activity,
            jni_str!("startActivity"),
            jni_sig!("(Landroid/content/Intent;)V"),
            &[JValue::Object(&intent)],
        )?;
        Ok(true)
    });
    if let Err(e) = res {
        log::error!("launch failed: {e}");
    }
}

fn wire(ui: &ComponentInstance, app: &AndroidApp) -> Result<(), String> {
    let err = |e: slint_interpreter::SetCallbackError| format!("{e:?}");
    let keypad_app = app.clone();
    ui.set_callback("unlock-key", move |args| {
        if let Some(Value::Number(key)) = args.first() {
            let vm = unsafe { JavaVM::from_raw(keypad_app.vm_as_ptr() as *mut _) };
            let result = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
                let activity = ManuallyDrop::new(unsafe {
                    JObject::from_raw(&*env, keypad_app.activity_as_ptr() as jni::sys::jobject)
                });
                env.call_method(&*activity, jni_str!("onUnlockKey"), jni_sig!("(I)V"),
                    &[JValue::Int(*key as i32)])?;
                Ok(())
            });
            if result.is_err() { log::warn!("keypad bridge unavailable"); }
        }
        Value::Void
    }).map_err(|e| format!("keypad callback: {e:?}"))?;
    let a = app.clone();
    ui.set_global_callback("Actions", "open-android-auto", move |_| {
        launch(&a, HEADUNIT_PKG);
        Value::Void
    })
    .map_err(err)?;
    let a = app.clone();
    ui.set_global_callback("Actions", "open-trunk", move |_| {
        request_trunk(&a);
        Value::Void
    })
    .map_err(err)?;
    ui.set_global_callback("Actions", "open-apps", |_| {
        log::info!("action: open-apps (not built yet)");
        Value::Void
    })
    .map_err(err)?;
    // Media and widget taps go to the plugin host (display-only; it routes to the owner).
    for name in ["play-pause", "next", "previous"] {
        let a = app.clone();
        ui.set_global_callback("Media", name, move |_| {
            dash_action(&a, &format!("media.{name}"));
            Value::Void
        })
        .map_err(err)?;
    }
    let a = app.clone();
    ui.set_global_callback("Plugins", "widget-clicked", move |args| {
        if let Some(Value::String(id)) = args.first() {
            dash_action(&a, &format!("widget.{id}"));
        }
        Value::Void
    })
    .map_err(err)?;
    Ok(())
}

/// Leaves the restart marker, clears the bad-theme strike and exits; Android relaunches
/// the home app. A failed bridge call still exits: the selection files are already written.
fn clean_restart(app: &AndroidApp, files: &Path, name: &str) -> ! {
    if let Err(e) = theme_switch::mark_restart(files, name) {
        log::warn!("theme restart marker: {e}");
    }
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut _) };
    if let Err(e) = vm.attach_current_thread(|env| -> jni::errors::Result<()> {
        let activity = ManuallyDrop::new(unsafe {
            JObject::from_raw(&*env, app.activity_as_ptr() as jni::sys::jobject)
        });
        env.call_method(&*activity, jni_str!("onThemeExit"), jni_sig!("()V"), &[])?;
        Ok(())
    }) {
        log::warn!("theme clean-exit bridge: {e}");
    }
    std::process::exit(0)
}

/// Theme folder relative to `files/`; "default" is the bundled fallback copy.
fn theme_dir(id: &str) -> String {
    if id == DEFAULT_THEME { FALLBACK.into() } else { format!("themes/{id}") }
}

/// Display name from theme.json, or the id.
fn theme_label(files: &Path, id: &str) -> String {
    std::fs::read_to_string(files.join(theme_dir(id)).join("theme.json")).ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|m| m["name"].as_str().filter(|s| !s.is_empty()).map(String::from))
        .unwrap_or_else(|| id.to_string())
}

fn wire_theme(ui: &ComponentInstance, app: &AndroidApp, files: &Path) -> Result<(), String> {
    let weak = ui.as_weak();
    let a = app.clone();
    let dir = files.to_path_buf();
    ui.set_callback("apply-theme", move |args| {
        let Some(ui) = weak.upgrade() else { return Value::Void };
        let Some(Value::String(id)) = args.first() else { return Value::Void };
        if !matches!(ui.get_property("host-screens-allowed"), Ok(Value::Bool(true))) {
            return Value::Void;
        }
        let checked = theme_switch::validate(&dir, id).map_err(|e| e.to_string())
            .and_then(|_| theme_entry(&dir, &theme_dir(id)));
        if let Err(e) = checked {
            log::warn!("theme switch '{id}' refused: {e}");
            let _ = ui.set_property("screen", Value::String("settings".into()));
            return Value::Void;
        }
        let name = theme_label(&dir, id);
        let _ = ui.set_property("switching-name", Value::String(name.clone().into()));
        // One event-loop turn so the switching card is drawn before the exit.
        let (weak, a, dir, id) = (ui.as_weak(), a.clone(), dir.clone(), id.to_string());
        slint::Timer::single_shot(std::time::Duration::from_millis(50), move || {
            let Some(ui) = weak.upgrade() else { return };
            if !matches!(ui.get_property("host-screens-allowed"), Ok(Value::Bool(true)))
                || !matches!(ui.get_property("screen"), Ok(Value::String(s)) if s == "switching") {
                log::info!("theme switch cancelled (host screen closed)");
                return;
            }
            match theme_switch::begin(&dir, &id) {
                Ok(()) => clean_restart(&a, &dir, &name),
                Err(e) => {
                    log::warn!("theme switch '{id}' failed: {e}");
                    let _ = ui.set_property("screen", Value::String("settings".into()));
                }
            }
        });
        Value::Void
    }).map_err(|e| format!("apply-theme: {e:?}"))?;
    for revert in [false, true] {
        let weak = ui.as_weak();
        let a = app.clone();
        let dir = files.to_path_buf();
        let name = if revert { "revert-theme" } else { "keep-theme" };
        ui.set_callback(name, move |_| {
            let Some(ui) = weak.upgrade() else { return Value::Void };
            if !matches!(ui.get_property("host-screens-allowed"), Ok(Value::Bool(true)))
                || !matches!(ui.get_property("confirm-theme"), Ok(Value::Bool(true))) {
                return Value::Void;
            }
            let result = if revert {
                theme_switch::revert(&dir).map(|id| { clean_restart(&a, &dir, &theme_label(&dir, &id)); })
            } else {
                theme_switch::keep(&dir)
            };
            if let Err(e) = result { log::warn!("{name} failed: {e}"); }
            let pending = dir.join("themes/previous").exists();
            let _ = ui.set_property("confirm-theme", Value::Bool(pending));
            Value::Void
        }).map_err(|e| format!("{name}: {e:?}"))?;
    }
    Ok(())
}

fn wire_plugins(ui: &ComponentInstance, app: &AndroidApp, files: &Path) -> Result<(), String> {
    let weak = ui.as_weak();
    let dir = files.to_path_buf();
    ui.set_callback("load-plugins", move |_| {
        if let Some(ui) = weak.upgrade() {
            if matches!(ui.get_property("host-screens-allowed"), Ok(Value::Bool(true))) {
                if let Err(e) = ui.set_property("plugins", plugins::model(plugins::list(&dir))) {
                    log::warn!("load-plugins: {e:?}");
                }
            }
        }
        Value::Void
    }).map_err(|e| format!("load-plugins: {e:?}"))?;
    let weak = ui.as_weak();
    let dir = files.to_path_buf();
    let a = app.clone();
    ui.set_callback("set-plugin", move |args| {
        let Some(ui) = weak.upgrade() else { return Value::Void };
        if !matches!(ui.get_property("host-screens-allowed"), Ok(Value::Bool(true))) {
            return Value::Void;
        }
        if let [Value::String(package), Value::Bool(on)] = args {
            let saved = plugins::set_enabled(&dir, package.as_str(), *on);
            if let Err(e) = &saved { log::warn!("set-plugin '{package}' failed: {e}"); }
            if let Err(e) = ui.set_property("plugins", plugins::model(plugins::list(&dir))) {
                log::warn!("reload plugins: {e:?}");
            }
            if saved.is_ok() { dash_action(&a, "plugins.refresh"); }
        }
        Value::Void
    }).map_err(|e| format!("set-plugin: {e:?}"))
}

fn run(app: &AndroidApp, start: Instant) -> Result<(), String> {
    let files = app.internal_data_path().ok_or("no internal data path")?;
    let settings = Rc::new(RefCell::new(settings::Settings::load(Some(&files.join("settings.json")), log_warn)));
    let ui = load_ui(&files, settings.clone())?;
    ui.set_property("themes", theme_switch::model(theme_switch::installed(&files, HOST_CONTRACT)))
        .map_err(|e| format!("themes: {e:?}"))?;
    ui.set_property("active-theme", Value::String(theme_switch::active(&files).into()))
        .map_err(|e| format!("active-theme: {e:?}"))?;
    ui.set_property("first-run", Value::Bool(!settings.borrow().setup_done()))
        .map_err(|e| format!("first-run: {e:?}"))?;
    settings::bind(&ui, settings, log_warn, refresh_clock)?;
    ui.set_global_property("HostState", "version-text", Value::String(env!("CARGO_PKG_VERSION").into()))
        .map_err(|e| format!("HostState.version-text: {e:?}"))?;
    wire(&ui, app)?;
    wire_theme(&ui, app, &files)?;
    wire_plugins(&ui, app, &files)?;
    ui.set_property("confirm-theme", Value::Bool(files.join("themes/previous").exists()))
        .map_err(|e| format!("confirm-theme: {e:?}"))?;
    let restart = theme_switch::take_restart(&files);
    ui.set_global_property("HostState", "restart-name", Value::String(restart.into()))
        .map_err(|e| format!("HostState.restart-name: {e:?}"))?;
    // First frame is measured from android_main entry (the load start), not process start.
    let mut first = Some(start);
    if let Err(e) = ui.window().set_rendering_notifier(move |state, _| {
        if let (slint::RenderingState::AfterRendering, Some(t)) = (state, first) {
            first = None;
            timing("first-frame", t);
        }
    }) {
        log::warn!("first-frame timing unavailable: {e:?}");
    }
    // Theme coordinates are panel pixels (1280x768). The Android backend applies the
    // 236 dpi density (~1.47x); force 1.0 once the window exists and after resumes.
    let weak = ui.as_weak();
    let unscale = slint::Timer::default();
    unscale.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(500), move || {
        if let Some(ui) = weak.upgrade() {
            if ui.window().scale_factor() != 1.0 {
                ui.window().dispatch_event(slint::platform::WindowEvent::ScaleFactorChanged {
                    scale_factor: 1.0,
                });
            }
        }
    });
    UI.with(|slot| *slot.borrow_mut() = Some(ui));
    flush();
    UI.with(|slot| match slot.borrow().as_ref() {
        Some(ui) => ui.run().map_err(|e| e.to_string()),
        None => Ok(()),
    })
}

#[unsafe(no_mangle)]
fn android_main(app: android_activity::AndroidApp) {
    let start = Instant::now();
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("OlaDash"),
    );
    let listener = |e: &android_activity::PollEvent<'_>| {
        if matches!(e, android_activity::PollEvent::Main(android_activity::MainEvent::InputAvailable)) {
            TOUCHED.store(true, Ordering::Release);
        }
    };
    if let Err(e) = slint::android::init_with_event_listener(app.clone(), listener) {
        log::error!("slint android init failed: {e}");
        return;
    }
    if let Err(e) = run(&app, start) {
        log::error!("fatal: {e}");
        // files/ may be damaged: recopy the bundled assets on the next start.
        if let Some(files) = app.internal_data_path() {
            let _ = std::fs::remove_file(files.join(".assets-stamp"));
        }
    }
}
