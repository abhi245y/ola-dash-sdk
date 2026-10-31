//! Host root window and overlay settings, shared by the Android host and `sim`
//! (sim includes this file with `#[path]`, so keep it free of Android code).
//!
//! The root window (`Host`) embeds the theme's `Dash` and draws `HostOverlay`
//! above it. Contract v6 adds optional `out` properties to `Dash`. Only the ones the
//! theme declares are mirrored on `Host` (the probe compile below finds them), and
//! each mirror's `changed` handler re-applies the settings through `apply()`.
//! Values are checked here (contrast, telltale list) and written to the
//! `OverlayConfig` global, which the overlay reads.

use crate::theme_options::ThemeOption;
use slint::{Brush, Color, Model, ModelRc, VecModel};
use slint_interpreter::Struct;
use slint_interpreter::{CompilationResult, Compiler, ComponentHandle, ComponentInstance, Value};
use std::io;
use std::path::{Path, PathBuf};

/// Optional `Dash` properties of contract v6, with the type each one must have.
const OPTIONAL: [(&str, &str); 5] = [
    ("overlay-placement", "OverlayPlacement"),
    ("telltale-order", "[Telltale]"),
    ("overlay-style", "OverlayStyle"),
    ("alert-position", "AlertPosition"),
    ("lock-style", "LockStyle"),
];

/// Host default telltale order. Themes may reorder; omitted kinds are appended in this order.
const TELLTALES: [&str; 9] = [
    "left-indicator", "right-indicator", "high-beam", "headlight", "ready",
    "locked", "side-stand", "fault", "charging",
];
const PLACEMENTS: [&str; 4] = ["top", "bottom", "left", "right"];
const ALERT_POSITIONS: [&str; 3] = ["center", "upper", "lower"];
/// Host strip colours (`strip-bg` day and night, `strip-fg`; docs/design/host-ui-tokens.md). The
/// strip is dark in both modes. Also the fallback when a theme fails the contrast rule.
const STRIP_BG: [Color; 2] = [Color::from_rgb_u8(0x0f, 0x2a, 0x2a), Color::from_rgb_u8(0x07, 0x14, 0x14)];
const STRIP_FG: Color = Color::from_rgb_u8(0xef, 0xe6, 0xd6);
/// The lit telltale colours (OverlayTokens ind-color, beam-color, lock-color, amber, red). A strip
/// background must show each at 3:1 or more, because the glyphs have no lamp window of their own.
const STATUS: [Color; 5] = [
    Color::from_rgb_u8(0x5c, 0xff, 0x9d), Color::from_rgb_u8(0x3d, 0xa5, 0xff), Color::from_rgb_u8(0xd0, 0xd0, 0xd0),
    Color::from_rgb_u8(0xff, 0xb0, 0x20), Color::from_rgb_u8(0xff, 0x3b, 0x30),
];
/// WCAG 2.x minimum for non-text contrast.
const MIN_GLYPH: f64 = 3.0;
/// WCAG 2.x minimum contrast for text.
const MIN_CONTRAST: f64 = 4.5;
/// The tokens' `ink`: text on the accent fill.
const INK: Color = Color::from_rgb_u8(0x0f, 0x2a, 0x2a);
/// Host lock-screen style for each mode (the look the host draws with no theme style). Also the
/// fallback for any lock token a theme leaves out or fails.
const HOST_LOCK_DAY: Lock = Lock {
    bg: Color::from_rgb_u8(0xf6, 0xf3, 0xec),
    fg: Color::from_rgb_u8(0x0f, 0x2a, 0x2a),
    muted: Color::from_rgb_u8(0x4a, 0x5c, 0x5c),
    accent: Color::from_rgb_u8(0xff, 0xc9, 0x4a),
    on_accent: INK,
    key_bg: Color::from_rgb_u8(0xff, 0xff, 0xff),
    key_border: Color::from_rgb_u8(0xd9, 0xd2, 0xc3),
    error: Color::from_rgb_u8(0xa3, 0x3a, 0x00),
    shape: "rounded",
    digits: "sans",
};
const HOST_LOCK_NIGHT: Lock = Lock {
    bg: Color::from_rgb_u8(0x0b, 0x1c, 0x1c),
    fg: Color::from_rgb_u8(0xef, 0xe6, 0xd6),
    muted: Color::from_rgb_u8(0xa9, 0xb5, 0xae),
    key_bg: Color::from_rgb_u8(0x16, 0x36, 0x36),
    key_border: Color::from_rgb_u8(0x24, 0x50, 0x4e),
    error: Color::from_rgb_u8(0xff, 0xb0, 0x20),
    ..HOST_LOCK_DAY
};

const ROOT: &str = r#"import { Dash } from "{THEME}";
import { HostOverlay, HostState, OverlayConfig } from "overlay.slint";
import { LockConfig } from "unlock.slint";
import { SettingsScreen, PluginInfo, ThemeOptionInfo } from "settings.slint";
import { ThemeInfo, SwitchingScreen, ConfirmThemeScreen, FirstRunScreen } from "theme-switch.slint";
import { Vehicle, Display, Media, Actions, Battery, Ride, Navigation, Plugins, OverlayPlacement, OverlayStyle, Telltale, AlertPosition, LockStyle } from "vehicle.slint";
export { Dash, Vehicle, Display, Media, Actions, Battery, Ride, Navigation, Plugins, HostState, OverlayConfig, LockConfig }
export component Host inherits Window {
    in-out property <string> screen: "dash";
    in-out property <int> settings-pane: 0;
    in-out property <bool> clock24: true;
    in-out property <bool> confirm-theme: false;
    in-out property <bool> first-run: false;
    in property <[ThemeInfo]> themes;
    in property <string> active-theme;
    in property <[PluginInfo]> plugins;
    in property <[ThemeOptionInfo]> theme-options;
    in-out property <bool> theme-options-open: false;
    in-out property <int> confirm-seconds: 10;
    in-out property <string> switching-name;
    callback apply-theme(string);
    callback keep-theme();
    callback revert-theme();
    callback set-setting(string, string);
    callback set-option(string, string);
    callback load-plugins();
    callback load-themes();
    callback set-plugin(string, bool);
    out property <bool> host-screens-allowed: HostState.parked && HostState.alert-code == 0
        && HostState.auth-phase >= 4 && !HostState.test-mode;
    function settle() {
        if !root.host-screens-allowed { root.screen = "dash"; }
        else if root.confirm-theme { root.screen = "confirm-theme"; }
        else if root.first-run && root.screen != "switching" { root.screen = "first-run"; }
        else if root.screen != "dash" && root.screen != "settings" && root.screen != "switching" {
            root.screen = "dash";
        }
    }
    changed host-screens-allowed => { root.settle(); }
    changed confirm-theme => { root.settle(); }
    changed first-run => { root.settle(); }
    changed screen => { root.theme-options-open = false; root.settle(); }
    changed settings-pane => { root.theme-options-open = false; }
    public function switch-theme(id: string) {
        if root.host-screens-allowed && !root.confirm-theme && !root.first-run && root.screen != "switching" {
            root.switching-name = id;
            root.screen = "switching";
            root.apply-theme(id);
        }
    }
    public function keep-current-theme() {
        if root.host-screens-allowed && root.confirm-theme { root.keep-theme(); }
    }
    public function revert-current-theme() {
        if root.host-screens-allowed && root.confirm-theme { root.revert-theme(); }
    }
    Timer {
        interval: 1s;
        running: root.confirm-theme && root.host-screens-allowed && root.screen == "confirm-theme";
        triggered => {
            if root.confirm-seconds <= 1 { root.revert-current-theme(); }
            else { root.confirm-seconds -= 1; }
        }
    }
    public function open-settings() -> bool {
        if root.host-screens-allowed && !root.confirm-theme && !root.first-run && root.screen != "switching" {
            root.theme-options-open = false;
            root.screen = "settings";
            return true;
        }
        return false;
    }
    callback unlock-key(int);
    // Fired when a mirrored theme setting changes; the host re-applies OverlayConfig.
    callback theme-overlay-changed();
    out property <bool> host-night: HostState.night;
    changed host-night => { root.theme-overlay-changed(); }
{MIRRORS}{OPTIONS}    width: 1280px;
    height: 768px;
    background: #000000;
    // Slint scopes this to the whole window, so it is for the theme's text only: every host
    // Text is a HostText with its own font-family.
    default-font-family: dash.default-font-family;
    // The wrapper keeps a theme's own `z` (it merges onto this use-site) below the overlay.
    Rectangle {
        width: parent.width; height: parent.height;
        dash := Dash { width: parent.width; height: parent.height;{OPTION_BINDINGS} }
    }
    if root.screen == "settings" && root.host-screens-allowed : SettingsScreen {
        width: parent.width; height: parent.height;
        selected <=> root.settings-pane;
        clock24: root.clock24;
        themes: root.themes;
        active-theme: root.active-theme;
        theme-options: root.theme-options;
        options-open <=> root.theme-options-open;
        plugins: root.plugins;
        load-plugins => { root.load-plugins(); }
        load-themes => { root.load-themes(); }
        set-plugin(package, on) => { root.set-plugin(package, on); }
        set-setting(key, value) => { root.set-setting(key, value); }
        set-option(id, value) => { root.set-option(id, value); }
        apply-theme(id) => { root.switch-theme(id); }
        closed => { root.screen = "dash"; }
    }
    if root.screen == "switching" && root.host-screens-allowed : SwitchingScreen {
        width: parent.width; height: parent.height;
        name: root.switching-name;
    }
    if root.screen == "confirm-theme" && root.host-screens-allowed : ConfirmThemeScreen {
        width: parent.width; height: parent.height;
        seconds: root.confirm-seconds;
        keep => { root.keep-current-theme(); }
        revert => { root.revert-current-theme(); }
    }
    if root.screen == "first-run" && root.host-screens-allowed : FirstRunScreen {
        width: parent.width; height: parent.height;
        themes: root.themes;
        active-theme: root.active-theme;
        night: HostState.night;
        set-setting(key, value) => { root.set-setting(key, value); }
        done(id) => {
            if root.host-screens-allowed && root.first-run && !root.confirm-theme {
                root.set-setting("setup", "done");
                root.first-run = false;
                if id != root.active-theme { root.switch-theme(id); }
                else { root.screen = "dash"; }
            }
        }
    }
    HostOverlay {
        width: parent.width; height: parent.height;
        unlock-key(key) => { root.unlock-key(key); }
        open-hub => { root.open-settings(); }
    }
}
"#;

/// Root source for a theme. `theme_import` is the theme entry, relative to `root_dir`
/// (the directory the root file is imported from: `files/host` on the device).
/// `declared` lists the optional Dash properties the theme defines (see `declared`).
/// `options` contains validated manifest options kept by the probe.
pub fn root_source(theme_import: &str, declared: &[String], options: &[ThemeOption]) -> String {
    let mut mirrors = String::new();
    for name in declared {
        let Some((_, ty)) = OPTIONAL.iter().find(|(n, _)| n == name) else { continue };
        mirrors.push_str(&format!(
            "    out property <{ty}> theme-{name}: dash.{name};\n    changed theme-{name} => {{ root.theme-overlay-changed(); }}\n"
        ));
    }
    let mut properties = String::new();
    let mut bindings = String::new();
    for option in options {
        let (id, ty) = (&option.id, option.kind.slint_type());
        properties.push_str(&format!("    in-out property <{ty}> opt-{id};\n"));
        bindings.push_str(&format!(" opt-{id}: root.opt-{id};"));
    }
    ROOT.replace("{THEME}", theme_import).replace("{MIRRORS}", &mirrors)
        .replace("{OPTIONS}", &properties).replace("{OPTION_BINDINGS}", &bindings)
}

/// Keep only options with a public Dash property of the probed type. Number deliberately
/// includes length/float: Slint's root compile checks the actual int binding afterwards.
pub fn kept_options(result: &CompilationResult, options: &[ThemeOption], mut skipped: impl FnMut(&str, &str)) -> Result<Vec<ThemeOption>, String> {
    let dash = result.component("Dash").ok_or("theme does not export component Dash")?;
    let properties: Vec<_> = dash.properties().collect();
    Ok(options.iter().filter(|option| {
        let name = format!("opt-{}", option.id);
        match properties.iter().find(|(n, _)| n == &name) {
            None => { skipped(&option.id, &format!("Dash.{name} missing")); false }
            Some((_, ty)) if *ty != option.kind.probe_type() => {
                skipped(&option.id, &format!("Dash.{name} has type {ty:?}, expected {:?}", option.kind.probe_type()));
                false
            }
            Some(_) => true,
        }
    }).cloned().collect())
}

/// Which optional Dash properties the compiled theme defines. Errors if it did not compile.
pub fn declared(result: &CompilationResult) -> Result<Vec<String>, String> {
    if result.has_errors() {
        let msgs: Vec<String> = result.diagnostics().map(|d| d.to_string()).collect();
        return Err(msgs.join("\n"));
    }
    let dash = result.component("Dash").ok_or("theme does not export component Dash")?;
    let names: Vec<String> = dash.properties().map(|(n, _)| n).collect();
    Ok(OPTIONAL
        .iter()
        .filter(|(n, _)| names.iter().any(|x| x == n))
        .map(|(n, _)| n.to_string())
        .collect())
}

/// Family name of the host font (`host/ui/hosttext.slint`). fontique merges same-name faces and
/// serves the first exact weight, so a theme font with this name could stand in for the host's.
pub const HOST_FONT: &str = "OlaDash Host";

/// Largest theme font file `vet_fonts` reads.
const MAX_FONT: u64 = 16 << 20;

/// `Vehicle` properties the host draws, with their `HostState` copy. The host and the sim set
/// both; a theme can write `Vehicle.*` (they are `in`), so the overlay reads only the copy.
/// Clock text is set separately: Vehicle stays HH:MM; HostState follows the preference.
pub const HOST_COPY: [(&str, &str); 9] = [
    ("left-indicator", "left-indicator"), ("right-indicator", "right-indicator"),
    ("high-beam", "high-beam"), ("headlight", "headlight"), ("ready-to-ride", "ready"),
    ("locked", "locked"), ("charging", "charging"),
    ("battery-pct", "battery-pct"), ("data-stale", "data-stale"),
];

/// Source a theme may not contain: each draws after the host overlay and takes touches over it
/// (`std-widgets.slint` because its widgets embed popups; `Tooltip` lowers to a `PopupWindow`).
const BANNED: [&str; 6] = ["PopupWindow", "ContextMenuArea", "MenuBar", "Tooltip", "DragArea", "std-widgets.slint"];

fn deny(path: &Path, why: &str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, format!("import guard: {}: {why}", path.display()))
}

/// The first font path in theme source that leaves the theme folder (`..` segment or absolute).
/// Font imports are read outside the file loader, so `vet_fonts` cannot see them. Scans every
/// string ending in a font extension, not just `import`, so a comment or stray quote fails closed.
/// Import paths are taken verbatim (no `\u{..}` decoding, i-slint-compiler typeloader.rs:1840), so
/// matching the source text matches what the compiler opens.
fn font_escape(src: &str) -> Option<&str> {
    let low = src.to_ascii_lowercase();
    [".ttf", ".ttc", ".otf"].iter().flat_map(|e| low.match_indices(e)).find_map(|(i, e)| {
        let end = i + e.len();
        let lit = &src[src[..i].rfind('"')? + 1..end];
        (low[end..].starts_with('"') && (lit.starts_with('/') || lit.split(['/', '\\']).any(|s| s == "..")))
            .then_some(lit)
    })
}

/// Reads `path` if it is a plain file under `theme` or one of `allowed` (all canonical),
/// and, inside `theme`, a `.slint` file free of `BANNED` words.
fn vet(path: &Path, allowed: &[PathBuf], theme: &Path) -> io::Result<String> {
    if std::fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(deny(path, "symlinks are not allowed in themes"));
    }
    let real = path.canonicalize()?;
    let in_theme = real.starts_with(theme);
    if !in_theme && !allowed.iter().any(|d| real.starts_with(d)) {
        return Err(deny(path, "a theme may only import its own folder and the contract"));
    }
    if in_theme && real.extension().is_none_or(|e| e != "slint") {
        return Err(deny(path, "theme source imports must use the .slint extension"));
    }
    let src = std::fs::read_to_string(&real)?;
    if let Some(w) = BANNED.iter().find(|w| in_theme && src.contains(**w)) {
        return Err(deny(path, &format!("themes may not use {w}")));
    }
    match font_escape(&src).filter(|_| in_theme) {
        Some(f) => Err(deny(path, &format!("a font import must stay inside the theme folder: {f}"))),
        None => Ok(src),
    }
}

/// Refuses a font file in `dir` (any depth) whose bytes name the host font, in either name-table
/// encoding (ASCII, UTF-16BE) and any case: fontique matches family names without case.
/// Fonts are read outside the file loader, so this walks the folder instead (`font_escape` keeps
/// imports inside it); symlinks and fonts over `MAX_FONT` fail closed.
fn vet_fonts(dir: &Path) -> io::Result<()> {
    let utf16: Vec<u8> = HOST_FONT.encode_utf16().flat_map(u16::to_be_bytes).collect();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let kind = std::fs::symlink_metadata(&path)?.file_type();
        if kind.is_symlink() {
            return Err(deny(&path, "symlinks are not allowed in themes"));
        } else if kind.is_dir() {
            vet_fonts(&path)?;
        } else if matches!(path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref(), Some("ttf" | "otf" | "ttc")) {
            if std::fs::metadata(&path)?.len() > MAX_FONT {
                return Err(deny(&path, &format!("font files may not be larger than {} MiB", MAX_FONT >> 20)));
            }
            let bytes = std::fs::read(&path)?;
            if [HOST_FONT.as_bytes(), &utf16].iter().any(|n| bytes.windows(n.len()).any(|w| w.eq_ignore_ascii_case(n))) {
                return Err(deny(&path, &format!("a theme font may not use the host font name \"{HOST_FONT}\"")));
            }
        }
    }
    Ok(())
}

/// Gate for a compile of a theme: vets `entry` (`build_from_path` reads it without the loader)
/// and every import the loader serves. `allowed` is the contract dir, plus the host dir for the
/// root compile. The probe is the gate (`gate`, which also scans the theme's fonts): on the root
/// compile the host dir is reachable from theme files too, so there this only closes paths
/// outside theme, contract and host.
/// Always `Some(..)`: `None` would fall through to the disk.
pub fn guard(c: &mut Compiler, allowed: &[&Path], theme: &Path, entry: &Path, gate: bool) -> Result<(), String> {
    let canon = |p: &Path| p.canonicalize().map_err(|e| format!("{}: {e}", p.display()));
    if std::fs::symlink_metadata(theme).map_err(|e| format!("{}: {e}", theme.display()))?.file_type().is_symlink() {
        return Err(format!("import guard: {}: the theme folder is a symlink", theme.display()));
    }
    let allowed = allowed.iter().map(|p| canon(p)).collect::<Result<Vec<_>, _>>()?;
    let theme = canon(theme)?;
    vet(entry, &allowed, &theme).map_err(|e| e.to_string())?;
    if gate {
        vet_fonts(&theme).map_err(|e| e.to_string())?;
    }
    c.set_file_loader(move |p| Box::pin(std::future::ready(Some(vet(p, &allowed, &theme)))));
    Ok(())
}

/// Connects host screen actions and the theme-change callback, and applies settings once.
/// `log` receives each warning (for example a rejected colour pair).
pub fn bind(ui: &ComponentInstance, log: fn(&str)) -> Result<(), String> {
    apply(ui, log);
    for name in ["open-settings", "open-themes"] {
        let weak = ui.as_weak();
        let ignored = std::cell::Cell::new(false);
        ui.set_global_callback("Actions", name, move |_| {
            if let Some(ui) = weak.upgrade() {
                if matches!(ui.invoke("open-settings", &[]), Ok(Value::Bool(true))) {
                    ignored.set(false);
                    if name == "open-themes" { let _ = ui.set_property("settings-pane", Value::Number(1.0)); }
                } else if !ignored.replace(true) {
                    log(&format!("action: {name} ignored (requires parked, no alert, unlocked and not test mode)"));
                }
            }
            Value::Void
        }).map_err(|e| format!("Actions.{name}: {e:?}"))?;
    }
    let weak = ui.as_weak();
    ui.set_callback("theme-overlay-changed", move |_| {
        if let Some(ui) = weak.upgrade() {
            apply(&ui, log);
        }
        Value::Void
    })
    .map_err(|e| format!("theme-overlay-changed: {e:?}"))
}

/// Reads the mirrored theme settings (absent ones use the defaults), checks them,
/// and writes `OverlayConfig`. The host defaults follow `HostState.night`.
pub fn apply(ui: &ComponentInstance, log: fn(&str)) {
    let get = |name: &str| ui.get_property(name).ok();
    let night = matches!(ui.get_global_property("HostState", "night"), Ok(Value::Bool(true)));
    let mut warn = |m: String| log(&m);

    let placement = get("theme-overlay-placement")
        .and_then(|v| enum_name(&v))
        .filter(|n| PLACEMENTS.contains(&n.as_str()))
        .unwrap_or_else(|| "top".into());
    let alert = get("theme-alert-position")
        .and_then(|v| enum_name(&v))
        .filter(|n| ALERT_POSITIONS.contains(&n.as_str()))
        .unwrap_or_else(|| "center".into());
    let order = telltale_order(get("theme-telltale-order"), &mut warn);
    let (mut bg, mut fg, mut labels) = (STRIP_BG[night as usize], STRIP_FG, false);
    if let Some(Value::Struct(s)) = get("theme-overlay-style") {
        let brush = |f: &str| match s.get_field(f) {
            Some(Value::Brush(Brush::SolidColor(c))) => Some(*c),
            _ => None,
        };
        if let (Some(b), Some(f)) = (brush("background"), brush("foreground")) {
            match strip_rejection(b, f) {
                None => (bg, fg) = (b, f),
                Some(why) => warn(format!("overlay-style rejected: {why}; using host colours")),
            }
        }
        if let Some(Value::Bool(b)) = s.get_field("show-labels") {
            labels = *b;
        }
    }

    let set = |prop: &str, v: Value| {
        if let Err(e) = ui.set_global_property("OverlayConfig", prop, v) {
            log(&format!("set OverlayConfig.{prop} failed: {e:?}"));
        }
    };
    set("placement", Value::EnumerationValue("OverlayPlacement".into(), placement));
    set("alert-position", Value::EnumerationValue("AlertPosition".into(), alert));
    set("background", Value::Brush(Brush::SolidColor(bg)));
    set("foreground", Value::Brush(Brush::SolidColor(fg)));
    set("show-labels", Value::Bool(labels));
    let model: Vec<Value> = order
        .into_iter()
        .map(|n| Value::EnumerationValue("Telltale".into(), n.into()))
        .collect();
    set("telltale-order", Value::Model(ModelRc::new(VecModel::from(model))));
    apply_lock(ui, get("theme-lock-style").as_ref(), night, log);
}

/// Lock-screen date from `yyyymmdd`, as "Fri 31 Oct". Empty for an impossible date.
pub fn date_text(ymd: u32) -> String {
    let (y, m, d) = (ymd / 10000, ymd / 100 % 100, ymd % 100);
    if y == 0 || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return String::new();
    }
    // Sakamoto's weekday, 0 = Sunday.
    let y = if m < 3 { y - 1 } else { y };
    let dow = (y + y / 4 - y / 100 + y / 400 + [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4][m as usize - 1] + d) % 7;
    let day = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][dow as usize];
    let month = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"][m as usize - 1];
    format!("{day} {d} {month}")
}

/// Why a theme's strip colours are refused, or `None`: foreground on background reaches 4.5:1 and
/// every lit telltale colour reaches 3:1 on the background.
fn strip_rejection(bg: Color, fg: Color) -> Option<String> {
    let ratio = contrast(bg, fg);
    if ratio < MIN_CONTRAST {
        return Some(format!("contrast {ratio:.2} < {MIN_CONTRAST} between background and foreground"));
    }
    let glyph = STATUS.iter().map(|c| contrast(bg, *c)).fold(f64::MAX, f64::min);
    (glyph < MIN_GLYPH).then(|| format!("a telltale colour has contrast {glyph:.2} < {MIN_GLYPH} on the background"))
}

/// Lock-screen style: colours and two shape choices from the theme's `lock-style`, checked
/// and written to `LockConfig`. The host alone draws and handles the keypad.
fn apply_lock(ui: &ComponentInstance, v: Option<&Value>, night: bool, log: fn(&str)) {
    let raw = v.and_then(raw_lock);
    let host = if night { &HOST_LOCK_NIGHT } else { &HOST_LOCK_DAY };
    let lock = resolve_lock(raw.as_ref(), host, &mut |m| log(&m));
    let brush = |c: Color| Value::Brush(Brush::SolidColor(c));
    let style: Struct = [
        ("background", brush(lock.bg)),
        ("foreground", brush(lock.fg)),
        ("muted", brush(lock.muted)),
        ("accent", brush(lock.accent)),
        ("key-background", brush(lock.key_bg)),
        ("key-border", brush(lock.key_border)),
        ("key-shape", Value::EnumerationValue("LockKeyShape".into(), lock.shape.into())),
        ("digits", Value::EnumerationValue("LockDigits".into(), lock.digits.into())),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();
    if let Err(e) = ui.set_global_property("LockConfig", "style", Value::Struct(style)) {
        log(&format!("set LockConfig.style failed: {e:?}"));
    }
    for (prop, c) in [("error", lock.error), ("on-accent", lock.on_accent)] {
        if let Err(e) = ui.set_global_property("LockConfig", prop, brush(c)) {
            log(&format!("set LockConfig.{prop} failed: {e:?}"));
        }
    }
}

/// A theme's `lock-style` as read from the Dash, before checks. A channel left at alpha 0
/// (a struct literal that omitted it) reads as "not set" and fails the checks.
#[derive(Clone, Debug)]
pub struct RawLock {
    pub bg: Color,
    pub fg: Color,
    pub muted: Color,
    pub accent: Color,
    pub key_bg: Color,
    pub key_border: Color,
    pub shape: String,
    pub digits: String,
}

fn raw_lock(v: &Value) -> Option<RawLock> {
    let Value::Struct(s) = v else { return None };
    let color = |f: &str| match s.get_field(f) {
        Some(Value::Brush(Brush::SolidColor(c))) => *c,
        _ => Color::from_argb_u8(0, 0, 0, 0),
    };
    let name = |f: &str| s.get_field(f).and_then(enum_name).unwrap_or_default();
    Some(RawLock {
        bg: color("background"),
        fg: color("foreground"),
        muted: color("muted"),
        accent: color("accent"),
        key_bg: color("key-background"),
        key_border: color("key-border"),
        shape: name("key-shape"),
        digits: name("digits"),
    })
}

/// The lock style the host draws. Every field is opaque or deliberately transparent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lock {
    pub bg: Color,
    pub fg: Color,
    pub muted: Color,
    pub accent: Color,
    pub on_accent: Color,
    pub key_bg: Color,
    pub key_border: Color,
    pub error: Color,
    pub shape: &'static str,
    pub digits: &'static str,
}

/// Checks a theme's lock style and fills gaps with the `host` style of the current mode. Rules (each failure is
/// logged and falls back to the host colour, or to the foreground when the host colour
/// does not read on the theme's background either):
/// - background and foreground are opaque and reach 4.5:1 together, or both are host colours;
/// - muted text (it carries attempt and cooldown copy) reaches 4.5:1 on the background;
/// - the accent (Delete's fill) is opaque and carries text: the best of the host ink, the
///   background and the foreground reaches 4.5:1 on it;
/// - key background (transparent means the screen background) reaches 4.5:1 with the foreground;
/// - key border is decorative and not checked;
/// - unknown shape or digit names become rounded and sans.
pub fn resolve_lock(raw: Option<&RawLock>, host: &Lock, warn: &mut dyn FnMut(String)) -> Lock {
    let Some(r) = raw else { return *host };
    let (bg, fg) = if opaque(r.bg) && opaque(r.fg) && contrast(r.bg, r.fg) >= MIN_CONTRAST {
        (r.bg, r.fg)
    } else {
        warn(format!(
            "lock-style rejected: background/foreground must be opaque with contrast >= {MIN_CONTRAST}; using host colours"
        ));
        (host.bg, host.fg)
    };
    let muted = pick("muted", r.muted, bg, MIN_CONTRAST, host.muted, fg, warn);
    // The host ink when it reads on the accent, else the better of the two screen colours.
    let on_accent = |a: Color| {
        let best = if contrast(bg, a) > contrast(fg, a) { bg } else { fg };
        if contrast(INK, a) >= MIN_CONTRAST { INK } else { best }
    };
    let accent = if opaque(r.accent) && contrast(on_accent(r.accent), r.accent) >= MIN_CONTRAST {
        r.accent
    } else {
        warn(format!("lock-style accent rejected (needs text at {MIN_CONTRAST}:1 on it); using host colour"));
        host.accent
    };
    let key_bg = if r.key_bg.alpha() == 0 {
        bg
    } else {
        pick("key-background", r.key_bg, fg, MIN_CONTRAST, host.key_bg, bg, warn)
    };
    let shape = match r.shape.as_str() {
        "rounded" | "square" | "grid" => static_name(&r.shape),
        other => {
            warn(format!("lock-style: unknown key-shape '{other}', using rounded"));
            "rounded"
        }
    };
    let digits = match r.digits.as_str() {
        "sans" | "segment" => static_name(&r.digits),
        other => {
            warn(format!("lock-style: unknown digits '{other}', using sans"));
            "sans"
        }
    };
    // Error copy stays readable on any background: this mode's host colour when it passes, else the other mode's, else the foreground.
    let error = [host.error, HOST_LOCK_DAY.error, HOST_LOCK_NIGHT.error]
        .into_iter()
        .find(|c| contrast(*c, bg) >= MIN_CONTRAST)
        .unwrap_or(fg);
    Lock { bg, fg, muted, accent, on_accent: on_accent(accent), key_bg, key_border: r.key_border, error, shape, digits }
}

fn opaque(c: Color) -> bool {
    c.alpha() == 255
}

/// `theme` when it is opaque and reaches `min` against `on`. Otherwise the host colour when
/// that passes, otherwise `fallback`.
fn pick(
    name: &str,
    theme: Color,
    on: Color,
    min: f64,
    host: Color,
    fallback: Color,
    warn: &mut dyn FnMut(String),
) -> Color {
    let ok = |c: Color| opaque(c) && contrast(c, on) >= min;
    if ok(theme) {
        return theme;
    }
    warn(format!("lock-style {name} rejected (needs {min}:1 on the screen); using host colour"));
    if ok(host) { host } else { fallback }
}

/// The `&'static str` for a validated name.
fn static_name(name: &str) -> &'static str {
    ["rounded", "square", "grid", "sans", "segment"]
        .into_iter()
        .find(|n| *n == name)
        .unwrap_or("rounded")
}

fn enum_name(v: &Value) -> Option<String> {
    match v {
        Value::EnumerationValue(_, name) => Some(name.clone()),
        _ => None,
    }
}

/// Theme order with unknown and duplicate entries dropped, then every missing kind
/// appended in host order. Always returns all nine kinds once.
fn telltale_order(v: Option<Value>, warn: &mut dyn FnMut(String)) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    if let Some(Value::Model(model)) = v {
        for item in model.iter() {
            let name = enum_name(&item).unwrap_or_default();
            match TELLTALES.iter().find(|t| **t == name) {
                Some(t) if !out.contains(t) => out.push(t),
                Some(_) => warn(format!("telltale-order: duplicate '{name}' ignored")),
                None => warn(format!("telltale-order: unknown '{name}' ignored")),
            }
        }
    }
    for t in TELLTALES {
        if !out.contains(&t) {
            out.push(t);
        }
    }
    out
}

/// WCAG 2.x contrast ratio between two opaque colours.
fn contrast(a: Color, b: Color) -> f64 {
    fn lum(c: Color) -> f64 {
        let ch = |v: u8| {
            let c = v as f64 / 255.0;
            if c <= 0.03928 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * ch(c.red()) + 0.7152 * ch(c.green()) + 0.0722 * ch(c.blue())
    }
    let (x, y) = (lum(a), lum(b));
    let (hi, lo) = if x > y { (x, y) } else { (y, x) };
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_without_options_keeps_the_original_source() {
        let original = ROOT.replace("{THEME}", "../theme/main.slint").replace("{MIRRORS}", "")
            .replace("{OPTIONS}", "").replace("{OPTION_BINDINGS}", "");
        let src = root_source("../theme/main.slint", &[], &[]);
        assert_eq!(src, original);
        assert!(src.contains("dash := Dash { width: parent.width; height: parent.height; }"));
        assert!(!src.contains("opt-"));
    }

    #[test]
    fn root_binds_all_option_types_only_into_dash() {
        let options = crate::theme_options::parse(&serde_json::json!({"options": [
            {"id":"accent", "type":"color", "default":"#123456", "label":"Accent"},
            {"id":"enabled", "type":"bool", "default":true, "label":"Enabled"},
            {"id":"count", "type":"int", "default":3, "label":"Count"},
            {"id":"style", "type":"choice", "default":"thin", "label":"Style", "values":["thin", "wide"]}
        ]})).unwrap();
        let src = root_source("../theme/main.slint", &["overlay-placement".into()], &options);
        assert!(src.contains("out property <OverlayPlacement> theme-overlay-placement: dash.overlay-placement;"));
        let dash = src.split("dash := Dash {").nth(1).unwrap().split('}').next().unwrap();
        for (id, ty) in [("accent", "color"), ("enabled", "bool"), ("count", "int"), ("style", "string")] {
            assert!(src.contains(&format!("    in-out property <{ty}> opt-{id};\n")));
            let binding = format!("opt-{id}: root.opt-{id};");
            assert!(dash.contains(&binding));
            assert_eq!(src.matches(&binding).count(), 1);
        }
        assert!(!src.contains("#123456"), "defaults are applied in Rust, never interpolated into Slint");
    }

    #[test]
    fn root_draws_settings_before_host_overlay() {
        for screen in ["SettingsScreen {", "SwitchingScreen {", "ConfirmThemeScreen {", "FirstRunScreen {"] {
            assert!(ROOT.find(screen).unwrap() < ROOT.find("HostOverlay {").unwrap(), "{screen}");
        }
        assert!(ROOT.contains("changed host-screens-allowed => { root.settle(); }")
            && ROOT.contains("HostState.parked && HostState.alert-code == 0\n        && HostState.auth-phase >= 4 && !HostState.test-mode"));
    }

    #[test]
    fn order_keeps_theme_order_and_appends_the_rest() {
        let mut logs = Vec::new();
        let theme = Value::Model(ModelRc::new(VecModel::from(vec![
            Value::EnumerationValue("Telltale".into(), "fault".into()),
            Value::EnumerationValue("Telltale".into(), "fault".into()),
            Value::EnumerationValue("Telltale".into(), "bogus".into()),
        ])));
        let out = telltale_order(Some(theme), &mut |m| logs.push(m));
        assert_eq!(out.len(), 9);
        assert_eq!(out[0], "fault");
        assert_eq!(out[1], "left-indicator");
        assert_eq!(logs.len(), 2);
    }

    #[test]
    fn lock_rejects_low_contrast_and_fills_host_values() {
        let mut logs = Vec::new();
        let bad = RawLock {
            bg: Color::from_rgb_u8(0x20, 0x20, 0x20),
            fg: Color::from_rgb_u8(0x30, 0x30, 0x30),
            muted: Color::from_rgb_u8(0x30, 0x30, 0x30),
            accent: Color::from_rgb_u8(0x77, 0x77, 0x77),
            key_bg: Color::from_argb_u8(0, 0, 0, 0),
            key_border: Color::from_rgb_u8(0x30, 0x30, 0x30),
            shape: "grid".into(),
            digits: "segment".into(),
        };
        for host in [&HOST_LOCK_DAY, &HOST_LOCK_NIGHT] {
            let lock = resolve_lock(Some(&bad), host, &mut |m| logs.push(m));
            // Background and foreground fail as a pair: both host colours, shape and digits kept.
            assert_eq!((lock.bg, lock.fg), (host.bg, host.fg));
            assert_eq!((lock.shape, lock.digits), ("grid", "segment"));
            assert!(lock.key_bg == host.bg || lock.key_bg == lock.bg);
            assert_eq!((lock.accent, lock.on_accent), (host.accent, host.on_accent));
        }
        assert!(logs.iter().any(|m| m.contains("background/foreground")));
        assert!(logs.iter().any(|m| m.contains("accent rejected")));
    }

    #[test]
    fn lock_host_defaults_pass_their_own_rules() {
        for host in [HOST_LOCK_DAY, HOST_LOCK_NIGHT] {
            assert!(contrast(host.fg, host.bg) >= MIN_CONTRAST);
            assert!(contrast(host.muted, host.bg) >= MIN_CONTRAST);
            assert!(contrast(host.error, host.bg) >= MIN_CONTRAST);
            assert!(contrast(host.fg, host.key_bg) >= MIN_CONTRAST);
            assert!(contrast(host.muted, host.key_bg) >= MIN_CONTRAST);
            assert!(contrast(host.on_accent, host.accent) >= MIN_CONTRAST);
            let mut logs = Vec::new();
            assert_eq!(resolve_lock(None, &host, &mut |m| logs.push(m)), host);
            assert!(logs.is_empty());
            // The same values as a theme's lock-style pass unchanged.
            let same = RawLock {
                bg: host.bg, fg: host.fg, muted: host.muted, accent: host.accent,
                key_bg: host.key_bg, key_border: host.key_border, shape: "rounded".into(), digits: "sans".into(),
            };
            assert_eq!(resolve_lock(Some(&same), &host, &mut |m| logs.push(m)), host);
            assert!(logs.is_empty(), "{logs:?}");
        }
    }

    #[test]
    fn strip_defaults_show_every_telltale_in_both_modes() {
        for bg in STRIP_BG {
            assert_eq!(strip_rejection(bg, STRIP_FG), None);
            for c in STATUS {
                assert!(contrast(bg, c) >= MIN_GLYPH);
            }
        }
        // The status colours here are the ones the overlay draws.
        let slint = include_str!("../ui/overlay.slint");
        for c in STATUS {
            let hex = format!("#{:02x}{:02x}{:02x};", c.red(), c.green(), c.blue());
            assert!(slint.contains(&hex), "{hex}");
        }
    }

    #[test]
    fn strip_rejects_a_background_that_hides_a_lit_telltale() {
        // Red on red passes 4.5:1 with black text but the fault light would vanish.
        let (red, black) = (Color::from_rgb_u8(0xff, 0x3b, 0x30), Color::from_rgb_u8(0, 0, 0));
        assert!(contrast(red, black) >= MIN_CONTRAST);
        assert!(strip_rejection(red, black).unwrap().contains("telltale"));
        assert!(strip_rejection(black, black).unwrap().contains("background and foreground"));
    }

    #[test]
    fn date_text_names_the_weekday() {
        assert_eq!(date_text(20251031), "Fri 31 Oct");
        assert_eq!(date_text(20261031), "Sat 31 Oct");
        assert_eq!(date_text(20280229), "Tue 29 Feb");
        assert_eq!(date_text(20000101), "Sat 1 Jan");
        assert_eq!(date_text(20251301), "");
        assert_eq!(date_text(0), "");
    }

    #[test]
    fn contrast_black_white_is_21() {
        assert!((contrast(Color::from_rgb_u8(0, 0, 0), Color::from_rgb_u8(255, 255, 255)) - 21.0).abs() < 0.01);
    }

    /// `<tmp>/<name>/{contract/c.slint, theme/ok.slint, outside.slint}`; returns canonical (root, contract, theme).
    fn tree(name: &str) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!("ola-guard-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("contract")).unwrap();
        std::fs::create_dir_all(root.join("theme")).unwrap();
        for f in ["contract/c.slint", "theme/ok.slint", "outside.slint"] {
            std::fs::write(root.join(f), "export component A {}").unwrap();
        }
        let root = root.canonicalize().unwrap();
        (root.clone(), root.join("contract"), root.join("theme"))
    }

    #[test]
    fn guard_serves_theme_and_contract_files() {
        let (root, c, t) = tree("ok");
        assert!(vet(&t.join("ok.slint"), &[c.clone()], &t).is_ok());
        assert!(vet(&c.join("c.slint"), &[c.clone()], &t).is_ok());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn guard_refuses_theme_source_with_non_slint_extensions() {
        let (root, c, t) = tree("extension");
        let src = "export component X {}";
        std::fs::write(t.join("lib.txt"), src).unwrap();
        std::fs::write(t.join("lib.slint"), src).unwrap();
        let error = vet(&t.join("lib.txt"), &[c.clone()], &t).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        assert!(error.to_string().contains("import guard"));
        assert_eq!(vet(&t.join("lib.slint"), &[c], &t).unwrap(), src);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn guard_refuses_escapes() {
        let (root, c, t) = tree("escape");
        let denied = |p: &Path| vet(p, &[c.clone()], &t).unwrap_err().kind() == io::ErrorKind::PermissionDenied;
        assert!(denied(&t.join("../outside.slint")));
        assert!(denied(&root.join("outside.slint")));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn guard_refuses_symlinks() {
        let (root, c, t) = tree("link");
        std::os::unix::fs::symlink(t.join("ok.slint"), t.join("link.slint")).unwrap();
        assert!(vet(&t.join("link.slint"), &[c.clone()], &t).unwrap_err().to_string().contains("symlink"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn guard_refuses_keywords_in_theme_files() {
        let (root, c, t) = tree("words");
        for w in BANNED {
            std::fs::write(t.join("bad.slint"), format!("// {w}")).unwrap();
            assert!(vet(&t.join("bad.slint"), &[c.clone()], &t).unwrap_err().to_string().contains(w), "{w}");
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn font_scan_refuses_the_host_name_in_any_encoding_or_case() {
        let (root, _, t) = tree("fonts");
        std::fs::create_dir_all(t.join("fonts/deep")).unwrap();
        let utf16: Vec<u8> = "oladash HOST".encode_utf16().flat_map(u16::to_be_bytes).collect();
        for (name, bytes) in [("a.ttf", b"..OlaDash Host..".to_vec()), ("deep/b.OTF", utf16)] {
            std::fs::write(t.join("fonts").join(name), bytes).unwrap();
            assert!(vet_fonts(&t).unwrap_err().to_string().contains("import guard"), "{name}");
            std::fs::remove_file(t.join("fonts").join(name)).unwrap();
        }
        std::fs::write(t.join("fonts/c.ttf"), b"Inter OlaDash").unwrap();
        assert!(vet_fonts(&t).is_ok());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn host_text_and_font_files_carry_the_host_family() {
        assert!(include_str!("../ui/hosttext.slint").contains(&format!("font-family: \"{HOST_FONT}\"")));
        let utf16: Vec<u8> = HOST_FONT.encode_utf16().flat_map(u16::to_be_bytes).collect();
        // sim includes this file with `#[path]`, so CARGO_MANIFEST_DIR is sim/ there and host/ here:
        // `../host` is the host crate from both.
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../host/ui/fonts");
        for f in ["Medium", "Bold", "ExtraBold"] {
            let bytes = std::fs::read(dir.join(format!("OlaDashHost-{f}.ttf"))).unwrap();
            assert!(bytes.windows(utf16.len()).any(|w| w == utf16), "{f}");
        }
        assert!(vet_fonts(&dir).is_err());
    }

    #[test]
    fn host_screens_use_host_text_only() {
        // Every host screen, so a new one is covered without touching this test.
        let ui = Path::new(env!("CARGO_MANIFEST_DIR")).join("../host/ui");
        let mut seen = 0;
        for f in std::fs::read_dir(ui).unwrap().map(|f| f.unwrap().path()) {
            if !f.extension().is_some_and(|e| e == "slint") || f.ends_with("hosttext.slint") {
                continue;
            }
            seen += 1;
            let src = std::fs::read_to_string(&f).unwrap();
            assert!(!src.contains("font-family:"), "{}: host text gets its family from HostText", f.display());
            for (i, _) in src.match_indices("Text") {
                let after = src[i + 4..].trim_start();
                let alone = !src[..i].ends_with(|c: char| c.is_alphanumeric());
                assert!(!(alone && after.starts_with('{')), "{}: bare Text near: {}", f.display(), &src[i.saturating_sub(40)..i + 6]);
            }
        }
        assert!(seen >= 5);
    }

    #[test]
    fn guard_refuses_font_imports_that_leave_the_theme() {
        let (root, c, t) = tree("fontpath");
        for bad in ["../other/x.ttf", "/data/x.otf", "fonts/../../x.TTC", "a\\..\\x.ttf"] {
            std::fs::write(t.join("bad.slint"), format!("// \"\nimport \"{bad}\";")).unwrap();
            assert!(vet(&t.join("bad.slint"), &[c.clone()], &t).unwrap_err().to_string().contains("font import"), "{bad}");
        }
        std::fs::write(t.join("ok.slint"), "import \"fonts/a.ttf\"; import \"fonts/v1..2/b.otf\";").unwrap();
        assert!(vet(&t.join("ok.slint"), &[c.clone()], &t).is_ok());
        std::fs::remove_dir_all(root).unwrap();
    }
}
