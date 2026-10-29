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
use slint_interpreter::{CompilationResult, ComponentHandle, ComponentInstance, Value};

/// Optional `Dash` properties of contract v6, with the type each one must have.
const OPTIONAL: [(&str, &str); 4] = [
    ("overlay-placement", "OverlayPlacement"),
    ("telltale-order", "[Telltale]"),
    ("overlay-style", "OverlayStyle"),
    ("alert-position", "AlertPosition"),
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

const ROOT: &str = r#"import { Dash } from "{THEME}";
import { HostOverlay, HostState, OverlayConfig } from "overlay.slint";
import { Vehicle, Media, Actions, Battery, Ride, Navigation, Plugins, OverlayPlacement, OverlayStyle, Telltale, AlertPosition } from "vehicle.slint";
export { Dash, Vehicle, Media, Actions, Battery, Ride, Navigation, Plugins, HostState, OverlayConfig }
export component Host inherits Window {
    callback unlock-key(int);
    // Fired when a mirrored theme setting changes; the host re-applies OverlayConfig.
    callback theme-overlay-changed();
{MIRRORS}    width: 1280px;
    height: 768px;
    background: #000000;
    default-font-family: dash.default-font-family;
    dash := Dash { width: parent.width; height: parent.height; }
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
    fn contrast_black_white_is_21() {
        assert!((contrast(Color::from_rgb_u8(0, 0, 0), Color::from_rgb_u8(255, 255, 255)) - 21.0).abs() < 0.01);
        assert!(contrast(DEFAULT_BG, DEFAULT_FG) >= MIN_CONTRAST);
    }
}
