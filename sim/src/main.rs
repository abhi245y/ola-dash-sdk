//! Desktop mock-data runner for ola-dash themes.
//!
//!   sim <theme-dir>                              interactive window
//!   sim <theme-dir> --screenshot out.png --at 30 headless PNG at sim time 30 s
//!   sim --check <theme-dir>                      validate theme.json and compile (no window)
//!   options: --contract <dir>  (default: <theme>/../../contract)
//!            --placement top|bottom|left|right   preview another overlay placement
//!            --alert <code>                      hold a host alert (1-6) instead of the mock cycle
//!            --state ride|park|charge|reverse|trunk|easy-park|locked|locked-error|cooldown
//!                                                 hold one vehicle state (default: the mock ride cycle);
//!                                                 the last three show the lock screen (see docs/themes.md)
//!            --plugins / --nav                    feed mock plugin widgets / an active navigation card
//!
//! The theme is loaded the way the device loads it: inside the host's `Host` root,
//! with `HostOverlay` drawn above it (host/src/theme_root.rs, shared with the host).
//!
//! The fake vehicle is a pure function of time (`sim_at`), so a screenshot at
//! `--at T` is reproducible: the headless path steps a mock clock from 0 to T
//! at 60 Hz so animations settle the way they would on a real device.

use slint::platform::software_renderer::{
    MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType,
};
use slint::platform::{Platform, PlatformError, WindowAdapter};
use slint::{ComponentHandle, Timer, TimerMode};
use slint_interpreter::{ComponentDefinition, ComponentInstance, Compiler, Value};
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::error::Error;
use std::f64::consts::PI;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

// The host's root window and overlay settings, shared with the Android build.
#[path = "../../host/src/theme_root.rs"]
mod theme_root;

/// Keys every theme.json must carry (see docs/themes.md).
const REQUIRED_STR_KEYS: [&str; 7] = ["id", "name", "version", "author", "license", "entry", "description"];

const WIDTH: u32 = 1280;
const HEIGHT: u32 = 768;
const TRACKS: [(&str, &str); 3] = [
    ("Harbor Lights", "Mira Vale"),
    ("Cold Current", "Oslo Static"),
    ("Low Orbit", "The Cartographers"),
];

/// Vehicle state as the host would publish it. Indicators are solid on/off;
/// blinking is the theme's job.
#[derive(Clone, Debug)]
struct Sim {
    speed: f64,
    battery: i64,
    range: i64,
    mode: &'static str,
    charging: bool,
    odo: f64,
    trip: f64,
    left: bool,
    right: bool,
    high_beam: bool,
    headlight: bool,
    locked: bool,
    ready: bool,
    side_stand: bool,
    fault: bool,
    /// Host alert code (0 none, 3 side stand) on the mock cycle.
    alert: i64,
    state: &'static str,
    clock: String,
    trunk: bool,
    hyper_available: bool,
    // Battery global (contract v5).
    soc: f64,
    current_a: f64,
    voltage_v: f64,
    temp_min_c: f64,
    temp_max_c: f64,
    health_pct: i64,
    // Ride global (contract v5).
    front_brake: bool,
    rear_brake: bool,
    regen: &'static str,
    motor_temp_c: f64,
    mcu_temp_c: f64,
}

/// What the fake vehicle is doing. `Ride` is the mock loop; the others hold one state.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Scenario {
    Ride,
    Park,
    Charge,
    Reverse,
    Trunk,
    EasyPark,
    Locked,
    LockedError,
    Cooldown,
}

impl Scenario {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "ride" => Self::Ride,
            "park" => Self::Park,
            "charge" => Self::Charge,
            "reverse" => Self::Reverse,
            "trunk" => Self::Trunk,
            "easy-park" => Self::EasyPark,
            "locked" => Self::Locked,
            "locked-error" => Self::LockedError,
            "cooldown" => Self::Cooldown,
            _ => return None,
        })
    }

    /// Lock-screen preview: HostState (auth-phase, pin-length, attempts, error, cooldown).
    /// `None` for vehicle states, where the lock screen is hidden.
    fn lock_state(self) -> Option<[i64; 5]> {
        match self {
            // Locked, two digits entered, four attempts left.
            Self::Locked => Some([1, 2, 4, 0, 0]),
            // Wrong passcode: the keypad is cleared, three attempts left.
            Self::LockedError => Some([1, 0, 3, 1, 0]),
            // Attempts used up: the service cooldown counts down.
            Self::Cooldown => Some([1, 0, 0, 0, 272]),
            _ => None,
        }
    }
}

/// 120 s loop: 60 s riding (mode cycles eco, normal, sport, hyper, custom every 12 s,
/// battery drains 100->60), then 60 s stopped and charging back to 100.
fn sim_at(t: f64, sc: Scenario) -> Sim {
    let loop_t = t % 120.0;
    let riding = loop_t < 60.0;
    let (mode, top, hyper_available) = if riding {
        match (loop_t / 12.0) as u32 {
            0 => ("eco", 45.0, true),
            1 => ("normal", 70.0, true),
            2 => ("sport", 95.0, true),
            3 => ("hyper", 120.0, true),
            _ => ("custom", 80.0, false),
        }
    } else {
        ("eco", 0.0, true)
    };
    // Accelerate/brake wave: 0 .. top, 9 s period.
    let speed = top * (0.5 - 0.5 * (2.0 * PI * t / 9.0).cos());
    let battery_f = if riding {
        100.0 - (40.0 / 60.0) * loop_t
    } else {
        60.0 + 40.0 * (loop_t - 60.0) / 60.0
    };
    let battery = battery_f.round() as i64;
    let minutes = (10 * 60 + 24 + (t / 60.0) as i64) % (24 * 60);
    // 120 s cycle: side-stand alert card for 10 s, fault telltale for 6 s before it.
    let phase = t % 120.0;
    let alert = if (100.0..110.0).contains(&phase) { 3 } else { 0 };
    let braking = riding && (t % 18.0) >= 6.0 && (t % 18.0) < 8.0;
    let mut s = Sim {
        side_stand: alert == 3,
        fault: (70.0..76.0).contains(&phase),
        alert,
        speed,
        battery,
        range: (battery_f * 1.05).round() as i64,
        mode,
        charging: !riding,
        odo: 1234.5 + t * 0.01,
        trip: (t % 3600.0) * 0.02,
        left: (t % 40.0) >= 10.0 && (t % 40.0) < 16.0,
        right: (t % 40.0) >= 25.0 && (t % 40.0) < 31.0,
        high_beam: (t % 60.0) >= 40.0 && (t % 60.0) < 45.0,
        headlight: true,
        locked: false,
        ready: true,
        state: if riding { "drive" } else { "charging" },
        clock: format!("{:02}:{:02}", minutes / 60, minutes % 60),
        trunk: false,
        hyper_available,
        soc: battery_f,
        current_a: if riding { -(speed * 0.35) } else { 18.4 },
        voltage_v: 50.0 + 0.1 * battery_f,
        temp_min_c: 25.0,
        temp_max_c: 28.0,
        health_pct: 96,
        front_brake: braking,
        rear_brake: braking,
        regen: if braking { "brake" } else if riding { "coast" } else { "none" },
        motor_temp_c: if riding { 42.0 + speed * 0.05 } else { 31.0 },
        mcu_temp_c: if riding { 38.0 } else { 29.0 },
    };
    // Held states drop the riding-only alerts and telltales.
    let held = |s: &mut Sim| {
        s.alert = 0;
        s.side_stand = false;
        s.fault = false;
        s.left = false;
        s.right = false;
        s.high_beam = false;
        s.headlight = false;
        s.speed = 0.0;
        s.charging = false;
        s.ready = false;
        s.state = "park";
        s.mode = "park";
        s.current_a = 0.0;
        s.regen = "none";
        s.front_brake = false;
        s.rear_brake = false;
        s.soc = 72.0;
        s.battery = 72;
        s.range = 76;
    };
    match sc {
        Scenario::Ride => {}
        Scenario::Park => held(&mut s),
        Scenario::Trunk => {
            held(&mut s);
            s.trunk = true;
        }
        Scenario::Charge => {
            held(&mut s);
            s.charging = true;
            s.state = "charging";
            // Charging: SOC climbs from 64 to 84 over a minute.
            s.soc = 64.0 + (t % 60.0) / 3.0;
            s.battery = s.soc.round() as i64;
            s.range = (s.soc * 1.05).round() as i64;
            s.current_a = 18.4;
            s.voltage_v = 50.0 + 0.1 * s.soc;
        }
        Scenario::Reverse => {
            s.alert = 0;
            s.side_stand = false;
            s.fault = false;
            s.left = false;
            s.right = false;
            s.ready = true;
            s.state = "drive";
            s.mode = "reverse";
            s.speed = 3.0;
            s.current_a = -6.0;
            s.regen = "none";
        }
        Scenario::EasyPark => {
            held(&mut s);
            s.state = "drive";
            s.mode = "easy-park";
            s.ready = true;
        }
        Scenario::Locked | Scenario::LockedError | Scenario::Cooldown => {
            held(&mut s);
            s.state = "sleep";
            s.locked = true;
            s.clock = "18:24".into();
            s.battery = 70;
            s.soc = 70.0;
            s.range = 74;
        }
    }
    s
}

/// Media state the theme can change through callbacks.
#[derive(Default)]
struct MediaState {
    playing: Cell<bool>,
    /// Added to the time-based track index; moved by next/previous.
    track_offset: Cell<usize>,
}

impl MediaState {
    fn new() -> Rc<Self> {
        let m = Self::default();
        m.playing.set(true);
        Rc::new(m)
    }
    fn track(&self, t: f64) -> (&'static str, &'static str) {
        let idx = (t / 30.0) as usize + self.track_offset.get();
        TRACKS[idx % TRACKS.len()]
    }
}

/// Pushes sim state into the Vehicle and Media globals of an instance.
struct Feeder {
    media: Rc<MediaState>,
    warned: RefCell<HashSet<String>>,
    /// `--plugins`: mock plugin widgets. `--nav`: an active navigation card.
    plugins: bool,
    nav: bool,
    /// `--alert <code>`: holds this host alert instead of the mock cycle.
    alert_override: Option<i64>,
    scenario: Scenario,
}

impl Feeder {
    fn new(args: &Args) -> Self {
        // SIM_PLUGINS=1 is the older spelling of --plugins --nav.
        let env = std::env::var_os("SIM_PLUGINS").is_some();
        Self {
            media: MediaState::new(),
            warned: Default::default(),
            plugins: args.plugins || env,
            nav: args.nav || env,
            alert_override: args.alert,
            scenario: args.scenario,
        }
    }

    fn push_plugins(&self, inst: &ComponentInstance) {
        let card = |pairs: [(&str, &str); 4]| {
            Value::Struct(
                pairs.iter().map(|(k, v)| (k.to_string(), Value::String((*v).into()))).collect(),
            )
        };
        let widgets = vec![
            card([("id", "ram"), ("title", "Free RAM"), ("value", "1793"), ("unit", "MB")]),
            card([("id", "temp"), ("title", "CPU"), ("value", "41"), ("unit", "C")]),
            card([("id", "uptime"), ("title", "Uptime"), ("value", "3h 12m"), ("unit", "")]),
        ];
        if self.plugins {
            self.set(inst, "Plugins", "widgets", Value::Model(slint::ModelRc::new(slint::VecModel::from(widgets))));
        }
        if self.nav {
            self.set(inst, "Navigation", "active", Value::Bool(true));
            self.set(inst, "Navigation", "instruction", Value::String("Turn left onto Harbour Road".into()));
            self.set(inst, "Navigation", "distance", Value::String("320 m".into()));
            self.set(inst, "Navigation", "eta", Value::String("14:32".into()));
        }
    }

    fn set(&self, inst: &ComponentInstance, global: &str, prop: &str, v: Value) {
        if let Err(e) = inst.set_global_property(global, prop, v) {
            let key = format!("{global}.{prop}");
            if self.warned.borrow_mut().insert(key.clone()) {
                eprintln!("sim: cannot set {key}: {e}");
            }
        }
    }

    fn push(&self, inst: &ComponentInstance, t: f64) {
        let s = sim_at(t, self.scenario);
        let (title, artist) = self.media.track(t);
        self.set(inst, "Vehicle", "speed-kmh", Value::Number(s.speed));
        self.set(inst, "Vehicle", "battery-pct", Value::Number(s.battery as f64));
        self.set(inst, "Vehicle", "range-km", Value::Number(s.range as f64));
        self.set(inst, 
            "Vehicle",
            "mode",
            Value::EnumerationValue("DriveMode".into(), s.mode.into()),
        );
        self.set(inst, "Vehicle", "charging", Value::Bool(s.charging));
        self.set(inst, "Vehicle", "odometer-km", Value::Number(s.odo));
        self.set(inst, "Vehicle", "trip-km", Value::Number(s.trip));
        self.set(inst, "Vehicle", "left-indicator", Value::Bool(s.left));
        self.set(inst, "Vehicle", "right-indicator", Value::Bool(s.right));
        self.set(inst, "Vehicle", "high-beam", Value::Bool(s.high_beam));
        self.set(inst, "Vehicle", "headlight", Value::Bool(s.headlight));
        self.set(inst, "Vehicle", "locked", Value::Bool(s.locked));
        self.set(inst, "Vehicle", "ready-to-ride", Value::Bool(s.ready));
        self.set(inst, "Vehicle", "clock-text", Value::String(s.clock.into()));
        self.set(inst, "Vehicle", "trunk-open", Value::Bool(s.trunk));
        self.set(inst, "Vehicle", "hyper-available", Value::Bool(s.hyper_available));
        self.set(inst, "Battery", "soc-pct", Value::Number(s.soc));
        self.set(inst, "Battery", "current-a", Value::Number(s.current_a));
        self.set(inst, "Battery", "voltage-v", Value::Number(s.voltage_v));
        self.set(inst, "Battery", "temp-min-c", Value::Number(s.temp_min_c));
        self.set(inst, "Battery", "temp-max-c", Value::Number(s.temp_max_c));
        self.set(inst, "Battery", "health-pct", Value::Number(s.health_pct as f64));
        self.set(inst, "Ride", "front-brake", Value::Bool(s.front_brake));
        self.set(inst, "Ride", "rear-brake", Value::Bool(s.rear_brake));
        self.set(inst, "Ride", "regen", Value::EnumerationValue("Regen".into(), s.regen.into()));
        self.set(inst, "Ride", "motor-temp-c", Value::Number(s.motor_temp_c));
        self.set(inst, "Ride", "mcu-temp-c", Value::Number(s.mcu_temp_c));
        self.set(
            inst,
            "Vehicle",
            "state",
            Value::EnumerationValue("VehicleState".into(), s.state.into()),
        );
        // HostState: the unlock screen is hidden in DRIVE (auth-phase 5).
        let auth = self.scenario.lock_state().unwrap_or([5, 0, -1, 0, 0]);
        for (prop, v) in ["auth-phase", "pin-length", "auth-attempts", "auth-error", "auth-cooldown"].iter().zip(auth) {
            self.set(inst, "HostState", prop, Value::Number(v as f64));
        }
        // Keys are live while passcode entry is open, and disabled during the cooldown.
        self.set(inst, "HostState", "auth-available", Value::Bool(matches!(self.scenario, Scenario::Locked | Scenario::LockedError)));
        let alert = self.alert_override.unwrap_or(s.alert);
        self.set(inst, "HostState", "alert-code", Value::Number(alert as f64));
        self.set(inst, "HostState", "side-stand", Value::Bool(s.side_stand || alert == 3));
        self.set(inst, "HostState", "fault", Value::Bool(s.fault || alert == 5));
        self.set(inst, "Media", "title", Value::String(title.into()));
        self.set(inst, "Media", "artist", Value::String(artist.into()));
        self.set(inst, "Media", "playing", Value::Bool(self.media.playing.get()));
        if self.plugins || self.nav {
            self.push_plugins(inst);
        }
    }

    /// Logs Actions and wires Media callbacks to the local media state.
    fn wire_callbacks(&self, inst: &ComponentInstance) {
        let m = self.media.clone();
        let _ = inst.set_global_callback("Media", "play-pause", move |_| {
            m.playing.set(!m.playing.get());
            println!("media: play-pause -> playing={}", m.playing.get());
            Value::Void
        });
        let m = self.media.clone();
        let _ = inst.set_global_callback("Media", "next", move |_| {
            m.track_offset.set(m.track_offset.get() + 1);
            println!("media: next");
            Value::Void
        });
        let m = self.media.clone();
        let _ = inst.set_global_callback("Media", "previous", move |_| {
            m.track_offset.set(m.track_offset.get() + TRACKS.len() - 1);
            println!("media: previous");
            Value::Void
        });
        let _ = inst.set_global_callback("Actions", "open-android-auto", |_| {
            println!("action: open-android-auto (sim: no-op)");
            Value::Void
        });
        let _ = inst.set_global_callback("Actions", "open-settings", |_| {
            println!("action: open-settings (sim: no-op)");
            Value::Void
        });
        let _ = inst.set_global_callback("Actions", "open-trunk", |_| {
            println!("action: open-trunk (sim: no-op)");
            Value::Void
        });
        let _ = inst.set_global_callback("Plugins", "widget-clicked", |args| {
            println!("plugins: widget-clicked {:?}", args.first());
            Value::Void
        });
    }
}

/// Headless platform: window is a software-rendered buffer, time is a mock clock.
struct Headless {
    window: Rc<MinimalSoftwareWindow>,
    clock: Rc<Cell<Duration>>,
}

impl Platform for Headless {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Ok(self.window.clone())
    }
    fn duration_since_start(&self) -> Duration {
        self.clock.get()
    }
}

struct Args {
    theme: PathBuf,
    contract: PathBuf,
    screenshot: Option<PathBuf>,
    at: f64,
    check: bool,
    placement: Option<String>,
    alert: Option<i64>,
    scenario: Scenario,
    plugins: bool,
    nav: bool,
    pixel: Option<(usize, usize)>,
}

fn parse_args() -> Result<Args, String> {
    let mut theme = None;
    let mut contract = None;
    let mut screenshot = None;
    let mut at = 5.0;
    let mut check = false;
    let mut placement = None;
    let mut alert = None;
    let mut scenario = Scenario::Ride;
    let mut plugins = false;
    let mut nav = false;
    let mut pixel = None;
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--check" => check = true,
            "--contract" => contract = it.next().map(PathBuf::from),
            "--placement" => placement = it.next(),
            "--alert" => {
                alert = Some(
                    it.next()
                        .and_then(|v| v.parse().ok())
                        .filter(|c| (1..=6).contains(c))
                        .ok_or("--alert needs a code 1-6")?,
                )
            }
            "--state" => {
                scenario = it
                    .next()
                    .and_then(|v| Scenario::parse(&v))
                    .ok_or("--state needs ride, park, charge, reverse, trunk, easy-park, locked, locked-error or cooldown")?
            }
            "--plugins" => plugins = true,
            "--nav" => nav = true,
            "--pixel" => {
                pixel = Some(it
                    .next()
                    .and_then(|v| v.split_once(',').and_then(|(x, y)| Some((x.parse().ok()?, y.parse().ok()?))))
                    .filter(|&(x, y)| x < WIDTH as usize && y < HEIGHT as usize)
                    .ok_or("--pixel needs X,Y inside the 1280 x 768 screen")?)
            }
            "--screenshot" => screenshot = it.next().map(PathBuf::from),
            "--at" => {
                at = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .ok_or("--at needs seconds")?
            }
            "-h" | "--help" => {
                println!("usage: sim <theme-dir> [--screenshot out.png --at SECS] [--contract DIR]");
                println!("       [--placement top|bottom|left|right] [--alert 1-6]");
                println!("       [--state ride|park|charge|reverse|trunk|easy-park|locked|locked-error|cooldown] [--plugins] [--nav]");
                println!("       [--pixel X,Y]  (with --screenshot: also print the colour at that point)");
                println!("       sim --check <theme-dir> [--contract DIR]");
                std::process::exit(0);
            }
            _ if theme.is_none() => theme = Some(PathBuf::from(a)),
            _ => return Err(format!("unexpected argument: {a}")),
        }
    }
    let theme = theme.ok_or("usage: sim <theme-dir> [...]")?;
    // Default: the repo layout themes/<name>/ next to contract/.
    let contract = contract.unwrap_or_else(|| theme.join("../../contract"));
    // The compiler wants absolute include paths.
    let contract = std::fs::canonicalize(&contract)
        .map_err(|e| format!("contract dir {}: {e}", contract.display()))?;
    let theme = std::fs::canonicalize(&theme).map_err(|e| format!("theme {}: {e}", theme.display()))?;
    if let Some(p) = &placement {
        if !["top", "bottom", "left", "right"].contains(&p.as_str()) {
            return Err(format!("--placement needs top, bottom, left or right, got {p}"));
        }
    }
    Ok(Args { theme, contract, screenshot, at, check, placement, alert, scenario, plugins, nav, pixel })
}

/// Path of `to` relative to `from` (both absolute, same drive), with '/' separators.
fn relative(from: &Path, to: &Path) -> String {
    let f: Vec<_> = from.components().collect();
    let t: Vec<_> = to.components().collect();
    let common = f.iter().zip(&t).take_while(|(a, b)| a == b).count();
    let mut parts = vec!["..".to_string(); f.len() - common];
    parts.extend(t[common..].iter().map(|c| c.as_os_str().to_string_lossy().into_owned()));
    parts.join("/")
}

/// Compiles the theme inside the host's `Host` root, as the device does.
fn load(main: &Path, args: &Args) -> Result<ComponentDefinition, Box<dyn Error>> {
    let include = vec![args.contract.clone()];
    let host_ui = args.contract.parent().unwrap_or(Path::new(".")).join("host/ui");
    let mut probe = Compiler::default();
    probe.set_include_paths(include.clone());
    theme_root::guard(&mut probe, &[&args.contract], &args.theme, main)?;
    let probed = pollster::block_on(probe.build_from_path(main));
    for d in probed.diagnostics() {
        eprintln!("{d}");
    }
    let declared = theme_root::declared(&probed).map_err(|e| format!("{}: {e}", main.display()))?;
    let src = theme_root::root_source(&relative(&host_ui, main), &declared);
    let mut compiler = Compiler::default();
    compiler.set_include_paths(include);
    theme_root::guard(&mut compiler, &[&args.contract, &host_ui], &args.theme, main)?;
    let result = pollster::block_on(compiler.build_from_source(src, host_ui.join("root.slint")));
    for d in result.diagnostics() {
        eprintln!("{d}");
    }
    if result.has_errors() {
        return Err(format!("{} failed to compile", main.display()).into());
    }
    result
        .component("Host")
        .ok_or_else(|| "host root component missing".into())
}

/// Binds the host overlay and applies the sim-only placement override, if any.
fn bind_overlay(inst: &ComponentInstance, args: &Args) -> Result<(), Box<dyn Error>> {
    theme_root::bind(inst, sim_warn)?;
    if let Some(p) = &args.placement {
        inst.set_global_property("OverlayConfig", "placement", Value::EnumerationValue("OverlayPlacement".into(), p.clone()))?;
    }
    Ok(())
}

fn sim_warn(msg: &str) {
    eprintln!("sim: {msg}");
}

/// Contract version the SDK ships: `contract-version: N;` in contract/vehicle.slint.
fn current_contract(contract: &Path) -> Result<i64, String> {
    let path = contract.join("vehicle.slint");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let key = "contract-version:";
    let at = text
        .find(key)
        .ok_or_else(|| format!("{}: no `contract-version: N` line", path.display()))?;
    let digits: String = text[at + key.len()..]
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits
        .parse()
        .map_err(|_| format!("{}: contract-version is not an integer", path.display()))
}

/// Validates theme.json. Pushes every problem into `errors`; returns the entry file when it exists.
fn check_manifest(theme: &Path, current: i64, errors: &mut Vec<String>) -> Option<PathBuf> {
    let path = theme.join("theme.json");
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            errors.push(format!("{}: {e}", path.display()));
            return None;
        }
    };
    let json: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            errors.push(format!("{}: invalid JSON: {e}", path.display()));
            return None;
        }
    };
    let Some(obj) = json.as_object() else {
        errors.push(format!("{}: top level must be an object", path.display()));
        return None;
    };
    for key in REQUIRED_STR_KEYS {
        match obj.get(key).and_then(|v| v.as_str()) {
            Some(s) if !s.trim().is_empty() => {}
            Some(_) => errors.push(format!("theme.json: \"{key}\" is empty")),
            None => errors.push(format!("theme.json: missing string key \"{key}\"")),
        }
    }
    match obj.get("contract").and_then(|v| v.as_i64()) {
        None => errors.push("theme.json: \"contract\" must be an integer".into()),
        Some(n) if n < 1 => errors.push(format!("theme.json: \"contract\" must be >= 1, got {n}")),
        Some(n) if n > current => errors.push(format!(
            "theme.json: needs contract {n}, this SDK ships contract {current}; lower the value or update the SDK"
        )),
        Some(_) => {}
    }
    if let Some(id) = obj.get("id").and_then(|v| v.as_str()) {
        let ok = id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        let dir = theme.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !ok {
            errors.push(format!("theme.json: \"id\" must use only a-z, 0-9, '-': {id:?}"));
        } else if id != dir {
            errors.push(format!("theme.json: \"id\" {id:?} must match the folder name {dir:?}"));
        }
    }
    let entry = obj.get("entry").and_then(|v| v.as_str())?;
    let entry_path = theme.join(entry);
    if entry_path.is_file() {
        Some(entry_path)
    } else {
        errors.push(format!("theme.json: entry {entry:?} not found in {}", theme.display()));
        None
    }
}

/// `sim --check <theme>`: manifest + compile. Prints each problem; exit status 1 on any.
fn run_check(args: &Args) -> bool {
    let mut errors = Vec::new();
    let current = match current_contract(&args.contract) {
        Ok(v) => v,
        Err(e) => {
            errors.push(e);
            0
        }
    };
    let entry = check_manifest(&args.theme, current, &mut errors);
    if let (true, Some(entry)) = (errors.is_empty(), entry) {
        match load(&entry, args) {
            Ok(_) => println!("ok: {} (sim contract {current})", args.theme.display()),
            Err(e) => errors.push(e.to_string()),
        }
    }
    for e in &errors {
        eprintln!("error: {}: {e}", args.theme.display());
    }
    errors.is_empty()
}

fn write_png(path: &Path, buf: &[PremultipliedRgbaColor]) -> Result<(), Box<dyn Error>> {
    let mut data = Vec::with_capacity(buf.len() * 4);
    for p in buf {
        // Theme backgrounds are opaque; un-premultiply anyway for translucent pixels.
        let a = p.alpha as u32;
        let un = |c: u8| -> u8 {
            if a == 0 { 0 } else { ((c as u32 * 255 + a / 2) / a).min(255) as u8 }
        };
        data.extend_from_slice(&[un(p.red), un(p.green), un(p.blue), p.alpha]);
    }
    let file = std::io::BufWriter::new(std::fs::File::create(path)?);
    let mut enc = png::Encoder::new(file, WIDTH, HEIGHT);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(&data)?;
    Ok(())
}

fn run_screenshot(args: &Args, out: &Path) -> Result<(), Box<dyn Error>> {
    let window = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    let clock = Rc::new(Cell::new(Duration::ZERO));
    slint::platform::set_platform(Box::new(Headless {
        window: window.clone(),
        clock: clock.clone(),
    }))?;
    let def = load(&args.theme.join("main.slint"), args)?;
    let inst = def.create()?;
    bind_overlay(&inst, args)?;
    let feeder = Feeder::new(args);
    feeder.wire_callbacks(&inst);
    inst.show()?;
    window.set_size(slint::PhysicalSize::new(WIDTH, HEIGHT));

    // Step the mock clock at 60 Hz so easings and blink phases match a live run.
    // Slint only starts an animation when a property is evaluated, and that
    // happens on render, so the last 1.5 s before `--at` are rendered per step
    // (the longest easing in the default theme is 900 ms).
    let mut buf = vec![PremultipliedRgbaColor::default(); (WIDTH * HEIGHT) as usize];
    let steps = (args.at * 60.0).round() as u64;
    let warm_from = args.at - 1.5;
    for i in 0..=steps {
        let t = i as f64 / 60.0;
        clock.set(Duration::from_secs_f64(t));
        feeder.push(&inst, t);
        slint::platform::update_timers_and_animations();
        if t >= warm_from {
            window.request_redraw();
            window.draw_if_needed(|r| {
                r.render(&mut buf, WIDTH as usize);
            });
        }
    }
    window.request_redraw();
    window.draw_if_needed(|r| {
        r.render(&mut buf, WIDTH as usize);
    });
    write_png(out, &buf)?;
    println!("wrote {} (sim t={:.2}s)", out.display(), args.at);
    if let Some((x, y)) = args.pixel {
        let p = buf[y * WIDTH as usize + x];
        println!("#{:02x}{:02x}{:02x}", p.red, p.green, p.blue);
    }
    Ok(())
}

fn run_live(args: &Args) -> Result<(), Box<dyn Error>> {
    let def = load(&args.theme.join("main.slint"), args)?;
    let inst = def.create()?;
    bind_overlay(&inst, args)?;
    let feeder = Rc::new(Feeder::new(args));
    feeder.wire_callbacks(&inst);
    let start = Instant::now();
    feeder.push(&inst, 0.0);
    let f = feeder.clone();
    let weak = inst.as_weak();
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_millis(16), move || {
        if let Some(i) = weak.upgrade() {
            f.push(&i, start.elapsed().as_secs_f64());
        }
    });
    inst.run()?;
    drop(timer);
    Ok(())
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("sim: {e}");
            std::process::exit(2);
        }
    };
    if args.check {
        std::process::exit(if run_check(&args) { 0 } else { 1 });
    }
    let result = match &args.screenshot {
        Some(out) => run_screenshot(&args, out),
        None => run_live(&args),
    };
    if let Err(e) = result {
        eprintln!("sim: {e}");
        std::process::exit(1);
    }
}
