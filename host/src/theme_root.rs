//! Host root window and overlay settings, shared by the Android host and `sim`
//! (sim includes this file with `#[path]`, so keep it free of Android code).
//!
//! The root window (`Host`) embeds the theme's `Dash` and draws `HostOverlay`
//! above it. Contract v6 adds optional `out` properties to `Dash`. Only the ones the
//! theme declares are mirrored on `Host` (the probe compile below finds them), and
//! each mirror's `changed` handler re-applies the settings through `apply()`.
//! Values are checked here (contrast, telltale list) and written to the
//! `OverlayConfig` global, which the overlay reads.

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
/// Host default strip colours (also the fallback when a theme fails the contrast rule).
const DEFAULT_BG: Color = Color::from_rgb_u8(0x0a, 0x0a, 0x0a);
const DEFAULT_FG: Color = Color::from_rgb_u8(0xf4, 0xf6, 0xfa);
/// WCAG 2.x minimum contrast for text.
const MIN_CONTRAST: f64 = 4.5;
/// WCAG 2.x minimum for large or non-text contrast: the passcode accent (dots, LOCKED).
const MIN_ACCENT: f64 = 3.0;
/// Host lock-screen style (the look the host draws with no theme style). Also the fallback
/// for any lock token a theme leaves out or fails.
const HOST_LOCK: Lock = Lock {
    bg: Color::from_rgb_u8(0x07, 0x0b, 0x10),
    fg: Color::from_rgb_u8(0xf4, 0xf6, 0xfa),
    muted: Color::from_rgb_u8(0xae, 0xbb, 0xd0),
    accent: Color::from_rgb_u8(0x3f, 0xb8, 0xff),
    key_bg: Color::from_rgb_u8(0x17, 0x21, 0x2e),
    key_border: Color::from_rgb_u8(0x2c, 0x3a, 0x4e),
    error: Color::from_rgb_u8(0xff, 0xb0, 0x20),
    shape: "rounded",
    digits: "sans",
};

const ROOT: &str = r#"import { Dash } from "{THEME}";
import { HostOverlay, HostState, OverlayConfig } from "overlay.slint";
import { LockConfig } from "unlock.slint";
import { Vehicle, Display, Media, Actions, Battery, Ride, Navigation, Plugins, OverlayPlacement, OverlayStyle, Telltale, AlertPosition, LockStyle } from "vehicle.slint";
export { Dash, Vehicle, Display, Media, Actions, Battery, Ride, Navigation, Plugins, HostState, OverlayConfig, LockConfig }
export component Host inherits Window {
    callback unlock-key(int);
    // Fired when a mirrored theme setting changes; the host re-applies OverlayConfig.
    callback theme-overlay-changed();
{MIRRORS}    width: 1280px;
    height: 768px;
    background: #000000;
    // Slint scopes this to the whole window, so it is for the theme's text only: every host
    // Text is a HostText with its own font-family.
    default-font-family: dash.default-font-family;
    // The wrapper keeps a theme's own `z` (it merges onto this use-site) below the overlay.
    Rectangle {
        width: parent.width; height: parent.height;
        dash := Dash { width: parent.width; height: parent.height; }
    }
    HostOverlay {
        width: parent.width; height: parent.height;
        unlock-key(key) => { root.unlock-key(key); }
    }
}
"#;

/// Root source for a theme. `theme_import` is the theme entry, relative to `root_dir`
/// (the directory the root file is imported from: `files/host` on the device).
/// `declared` lists the optional Dash properties the theme defines (see `declared`).
pub fn root_source(theme_import: &str, declared: &[String]) -> String {
    let mut mirrors = String::new();
    for name in declared {
        let Some((_, ty)) = OPTIONAL.iter().find(|(n, _)| n == name) else { continue };
        mirrors.push_str(&format!(
            "    out property <{ty}> theme-{name}: dash.{name};\n    changed theme-{name} => {{ root.theme-overlay-changed(); }}\n"
        ));
    }
    ROOT.replace("{THEME}", theme_import).replace("{MIRRORS}", &mirrors)
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
pub const HOST_COPY: [(&str, &str); 10] = [
    ("left-indicator", "left-indicator"), ("right-indicator", "right-indicator"),
    ("high-beam", "high-beam"), ("headlight", "headlight"), ("ready-to-ride", "ready"),
    ("locked", "locked"), ("charging", "charging"), ("clock-text", "clock-text"),
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
/// and, inside `theme`, free of `BANNED` words.
fn vet(path: &Path, allowed: &[PathBuf], theme: &Path) -> io::Result<String> {
    if std::fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(deny(path, "symlinks are not allowed in themes"));
    }
    let real = path.canonicalize()?;
    let in_theme = real.starts_with(theme);
    if !in_theme && !allowed.iter().any(|d| real.starts_with(d)) {
        return Err(deny(path, "a theme may only import its own folder and the contract"));
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

/// Connects the theme-change callback and applies the settings once.
/// `log` receives each warning (for example a rejected colour pair).
pub fn bind(ui: &ComponentInstance, log: fn(&str)) -> Result<(), String> {
    apply(ui, log);
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
/// and writes `OverlayConfig`.
pub fn apply(ui: &ComponentInstance, log: fn(&str)) {
    let get = |name: &str| ui.get_property(name).ok();
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
    let (mut bg, mut fg, mut radius, mut labels) = (DEFAULT_BG, DEFAULT_FG, 8.0_f32, false);
    if let Some(Value::Struct(s)) = get("theme-overlay-style") {
        let brush = |f: &str| match s.get_field(f) {
            Some(Value::Brush(Brush::SolidColor(c))) => Some(*c),
            _ => None,
        };
        if let (Some(b), Some(f)) = (brush("background"), brush("foreground")) {
            let ratio = contrast(b, f);
            if ratio >= MIN_CONTRAST {
                bg = b;
                fg = f;
            } else {
                warn(format!(
                    "overlay-style rejected: contrast {ratio:.2} < {MIN_CONTRAST} between background and foreground; using host colours"
                ));
            }
        }
        if let Some(Value::Number(r)) = s.get_field("corner-radius") {
            radius = (*r as f32).max(0.0);
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
    set("corner-radius", Value::Number(radius as f64));
    set("show-labels", Value::Bool(labels));
    let model: Vec<Value> = order
        .into_iter()
        .map(|n| Value::EnumerationValue("Telltale".into(), n.into()))
        .collect();
    set("telltale-order", Value::Model(ModelRc::new(VecModel::from(model))));
    apply_lock(ui, get("theme-lock-style").as_ref(), log);
}

/// Lock-screen style: colours and two shape choices from the theme's `lock-style`, checked
/// and written to `LockConfig`. The host alone draws and handles the keypad.
fn apply_lock(ui: &ComponentInstance, v: Option<&Value>, log: fn(&str)) {
    let raw = v.and_then(raw_lock);
    let lock = resolve_lock(raw.as_ref(), &mut |m| log(&m));
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
    if let Err(e) = ui.set_global_property("LockConfig", "error", brush(lock.error)) {
        log(&format!("set LockConfig.error failed: {e:?}"));
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
    pub key_bg: Color,
    pub key_border: Color,
    pub error: Color,
    pub shape: &'static str,
    pub digits: &'static str,
}

/// Checks a theme's lock style and fills gaps with host values. Rules (each failure is
/// logged and falls back to the host colour, or to the foreground when the host colour
/// does not read on the theme's background either):
/// - background and foreground are opaque and reach 4.5:1 together, or both are host colours;
/// - muted text (it carries attempt and cooldown copy) reaches 4.5:1 on the background;
/// - the accent (dots, LOCKED) reaches 3:1 on the background;
/// - key background (transparent means the screen background) reaches 4.5:1 with the foreground;
/// - key border is decorative and not checked;
/// - unknown shape or digit names become rounded and sans.
pub fn resolve_lock(raw: Option<&RawLock>, warn: &mut dyn FnMut(String)) -> Lock {
    let Some(r) = raw else { return HOST_LOCK };
    let (bg, fg) = if opaque(r.bg) && opaque(r.fg) && contrast(r.bg, r.fg) >= MIN_CONTRAST {
        (r.bg, r.fg)
    } else {
        warn(format!(
            "lock-style rejected: background/foreground must be opaque with contrast >= {MIN_CONTRAST}; using host colours"
        ));
        (HOST_LOCK.bg, HOST_LOCK.fg)
    };
    let muted = pick("muted", r.muted, bg, MIN_CONTRAST, HOST_LOCK.muted, fg, warn);
    let accent = pick("accent", r.accent, bg, MIN_ACCENT, HOST_LOCK.accent, fg, warn);
    let key_bg = if r.key_bg.alpha() == 0 {
        bg
    } else {
        pick("key-background", r.key_bg, fg, MIN_CONTRAST, HOST_LOCK.key_bg, bg, warn)
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
    // Error copy stays readable on any background: amber when it passes, else the foreground.
    let error = if contrast(HOST_LOCK.error, bg) >= MIN_CONTRAST { HOST_LOCK.error } else { fg };
    Lock { bg, fg, muted, accent, key_bg, key_border: r.key_border, error, shape, digits }
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
            accent: Color::from_rgb_u8(0x30, 0x30, 0x30),
            key_bg: Color::from_argb_u8(0, 0, 0, 0),
            key_border: Color::from_rgb_u8(0x30, 0x30, 0x30),
            shape: "grid".into(),
            digits: "segment".into(),
        };
        let lock = resolve_lock(Some(&bad), &mut |m| logs.push(m));
        // Background and foreground fail as a pair: both host colours, shape and digits kept.
        assert_eq!((lock.bg, lock.fg), (HOST_LOCK.bg, HOST_LOCK.fg));
        assert_eq!((lock.shape, lock.digits), ("grid", "segment"));
        assert!(lock.key_bg == HOST_LOCK.bg || lock.key_bg == lock.bg);
        assert!(logs.iter().any(|m| m.contains("background/foreground")));
    }

    #[test]
    fn lock_host_defaults_pass_their_own_rules() {
        assert!(contrast(HOST_LOCK.fg, HOST_LOCK.bg) >= MIN_CONTRAST);
        assert!(contrast(HOST_LOCK.muted, HOST_LOCK.bg) >= MIN_CONTRAST);
        assert!(contrast(HOST_LOCK.accent, HOST_LOCK.bg) >= MIN_ACCENT);
        assert!(contrast(HOST_LOCK.fg, HOST_LOCK.key_bg) >= MIN_CONTRAST);
        let mut logs = Vec::new();
        assert_eq!(resolve_lock(None, &mut |m| logs.push(m)), HOST_LOCK);
        assert!(logs.is_empty());
    }

    #[test]
    fn contrast_black_white_is_21() {
        assert!((contrast(Color::from_rgb_u8(0, 0, 0), Color::from_rgb_u8(255, 255, 255)) - 21.0).abs() < 0.01);
        assert!(contrast(DEFAULT_BG, DEFAULT_FG) >= MIN_CONTRAST);
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
        for f in ["Regular", "SemiBold", "Bold"] {
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
        assert!(seen >= 3);
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
