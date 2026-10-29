//! Ola dash host. Loads a Slint theme at runtime, drives its `Vehicle` global
//! from the vehicle-parser binder (values arrive from Java over JNI), and draws
//! the host-owned safety overlay above the theme.

use std::cell::RefCell;
use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use android_activity::AndroidApp;
use jni::objects::{JClass, JObject, JString, JValue};
use jni::{jni_sig, jni_str, EnvUnowned, JavaVM};
use slint::{ModelRc, VecModel};
use slint_interpreter::{Compiler, ComponentHandle, ComponentInstance, Value};

mod theme_root;

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
const CLOCK_HHMM: usize = 30;
const TEST_MODE: usize = 31;
const AUTH_PHASE: usize = 32;
const PIN_LENGTH: usize = 33;
const AUTH_ATTEMPTS: usize = 34;
const AUTH_ERROR: usize = 35;
const AUTH_COOLDOWN: usize = 36;
const AUTH_AVAILABLE: usize = 37;
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
const SLOTS: usize = 51;

const MODE_NAMES: [&str; 9] =
    ["eco", "normal", "sport", "hyper", "unknown", "park", "reverse", "custom", "easy-park"];
const STATE_NAMES: [&str; 6] = ["unknown", "sleep", "idle", "park", "drive", "charging"];
const REGEN_NAMES: [&str; 3] = ["none", "coast", "brake"];
const DEFAULT_THEME: &str = "default";
const FALLBACK: &str = "fallback"; // bundled copy of themes/default
const LAST_RESORT: &str = "last-resort"; // bundled placeholder, cannot fail to compile
const HEADUNIT_PKG: &str = "com.andrerinas.headunitrevived";
/// Highest contract version this host implements (contract/vehicle.slint `contract-version`).
const HOST_CONTRACT: i64 = 6;

/// Latest value per field, coalesced until the UI thread applies it.
static PENDING: Mutex<[Option<f64>; SLOTS]> = Mutex::new([None; SLOTS]);
static SCHEDULED: AtomicBool = AtomicBool::new(false);
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

/// Sends "media.<x>" or "widget.<id>" to PluginHost.action() through MainActivity.onDashAction.
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
            let hhmm = v as u32;
            let text = format!("{:02}:{:02}", hhmm / 100, hhmm % 100);
            ("Vehicle", "clock-text", Value::String(text.into()))
        }
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
    if let Err(e) = ui.set_global_property(global, prop, val) {
        log::warn!("set {global}.{prop} failed: {e:?}");
    }
}

/// `files/themes/active` holds the theme folder name. Anything odd falls back to "default".
fn theme_name(files: &Path) -> String {
    let name = std::fs::read_to_string(files.join("themes/active"))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let ok = !name.is_empty()
        && name.len() <= 64
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if ok { name } else { DEFAULT_THEME.to_string() }
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

fn build_root(files: &Path, theme: &str) -> Result<ComponentInstance, String> {
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
    let mut probe = Compiler::new();
    probe.set_include_paths(include.clone());
    let probed = spin_on::spin_on(probe.build_from_path(files.join(&rel).join(&entry)));
    let declared = theme_root::declared(&probed)?;
    let src = theme_root::root_source(&format!("../{rel}/{entry}"), &declared);
    let mut compiler = Compiler::new();
    compiler.set_include_paths(include);
    let result = spin_on::spin_on(compiler.build_from_source(src, files.join("host/root.slint")));
    if result.has_errors() {
        let msgs: Vec<String> = result.diagnostics().map(|d| d.to_string()).collect();
        return Err(msgs.join("\n"));
    }
    let def = result.component("Host").ok_or("no Host component")?;
    let ui = def.create().map_err(|e| e.to_string())?;
    theme_root::bind(&ui, log_warn)?;
    Ok(ui)
}

fn log_warn(msg: &str) {
    log::warn!("{msg}");
}

/// Theme first; any compile or create failure falls back to the bundled placeholder.
fn load_ui(files: &Path) -> Result<ComponentInstance, String> {
    let theme = theme_name(files);
    build_root(files, &theme)
        .or_else(|e| {
            log::error!("theme '{theme}' rejected, using fallback:\n{e}");
            build_root(files, FALLBACK)
        })
        .or_else(|e| {
            log::error!("fallback rejected, using last-resort:\n{e}");
            build_root(files, LAST_RESORT)
        })
}

enum Target {
    Package(&'static str),
    Settings,
}

/// Start an activity from the NativeActivity through JNI (no app classes needed).
fn launch(app: &AndroidApp, target: Target) {
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr() as *mut _) };
    let activity_ptr = app.activity_as_ptr() as jni::sys::jobject;
    let res = vm.attach_current_thread(|env| -> jni::errors::Result<bool> {
        // The NativeActivity ref is global, owned by the framework: never delete it.
        let activity = ManuallyDrop::new(unsafe { JObject::from_raw(&*env, activity_ptr) });
        let intent = match target {
            Target::Package(pkg) => {
                let pm = env
                    .call_method(
                        &*activity,
                        jni_str!("getPackageManager"),
                        jni_sig!("()Landroid/content/pm/PackageManager;"),
                        &[],
                    )?
                    .l()?;
                let name = env.new_string(pkg)?;
                env.call_method(
                    &pm,
                    jni_str!("getLaunchIntentForPackage"),
                    jni_sig!("(Ljava/lang/String;)Landroid/content/Intent;"),
                    &[JValue::Object(&name)],
                )?
                .l()?
            }
            Target::Settings => {
                let action = env.new_string("android.settings.SETTINGS")?;
                env.new_object(
                    jni_str!("android/content/Intent"),
                    jni_sig!("(Ljava/lang/String;)V"),
                    &[JValue::Object(&action)],
                )?
            }
        };
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
        launch(&a, Target::Package(HEADUNIT_PKG));
        Value::Void
    })
    .map_err(err)?;
    let a = app.clone();
    ui.set_global_callback("Actions", "open-settings", move |_| {
        launch(&a, Target::Settings);
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

fn run(app: &AndroidApp) -> Result<(), String> {
    let files = app.internal_data_path().ok_or("no internal data path")?;
    let ui = load_ui(&files)?;
    wire(&ui, app)?;
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
    if let Err(e) = run(&app) {
        log::error!("fatal: {e}");
    }
}
