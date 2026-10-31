//! Theme manifest options, shared by the Android host and sim. No Android code.

use serde_json::Value as Json;
use crate::settings::SharedSettings;
use slint::{Brush, Color, Model, ModelRc, VecModel};
use slint_interpreter::{ComponentHandle, ComponentInstance, Value, ValueType};
use std::collections::HashSet;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Color,
    Bool,
    Int { min: i32, max: i32 },
    Choice(Vec<String>),
}

impl Kind {
    pub fn slint_type(&self) -> &'static str {
        match self {
            Self::Color => "color",
            Self::Bool => "bool",
            Self::Int { .. } => "int",
            Self::Choice(_) => "string",
        }
    }

    pub fn probe_type(&self) -> ValueType {
        match self {
            Self::Color => ValueType::Brush,
            Self::Bool => ValueType::Bool,
            Self::Int { .. } => ValueType::Number,
            Self::Choice(_) => ValueType::String,
        }
    }

    fn parse_default(&self, value: &Json) -> Result<OptionValue, String> {
        match self {
            Self::Color => value.as_str().and_then(color).map(OptionValue::Color)
                .ok_or_else(|| "expected #rrggbb string".into()),
            Self::Bool => value.as_bool().map(OptionValue::Bool)
                .ok_or_else(|| "expected JSON bool".into()),
            Self::Int { min, max } => match value {
                Json::Number(n) => {
                    let n = n.as_i64().and_then(|v| i32::try_from(v).ok())
                        .ok_or("expected 32-bit integer")?;
                    if n < *min || n > *max { return Err(format!("expected integer in {min}..={max}")); }
                    Ok(OptionValue::Int(n))
                }
                _ => Err("expected JSON integer".into()),
            },
            Self::Choice(values) => value.as_str().filter(|s| values.iter().any(|v| v.as_str() == *s))
                .map(|s| OptionValue::Choice(s.into()))
                .ok_or_else(|| "expected one of values".into()),
        }
    }

    pub fn parse_override(&self, text: &str) -> Result<OptionValue, String> {
        let value = match self {
            Self::Color | Self::Choice(_) => Json::String(text.into()),
            Self::Bool => match text {
                "true" => Json::Bool(true),
                "false" => Json::Bool(false),
                _ => return Err("expected true or false".into()),
            },
            Self::Int { .. } => Json::from(text.parse::<i32>().map_err(|_| "expected 32-bit integer")?),
        };
        self.parse_default(&value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum OptionValue {
    Color([u8; 3]),
    Bool(bool),
    Int(i32),
    Choice(String),
}

impl OptionValue {
    pub fn json(&self) -> Json {
        match self {
            Self::Color([r, g, b]) => Json::String(format!("#{r:02x}{g:02x}{b:02x}")),
            Self::Bool(b) => Json::Bool(*b),
            Self::Int(n) => Json::from(*n),
            Self::Choice(s) => Json::String(s.clone()),
        }
    }

    pub fn value(&self) -> Value {
        match self {
            Self::Color([r, g, b]) => Value::Brush(Brush::SolidColor(Color::from_rgb_u8(*r, *g, *b))),
            Self::Bool(b) => Value::Bool(*b),
            Self::Int(n) => Value::Number(*n as f64),
            Self::Choice(s) => Value::String(s.clone().into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThemeOption {
    pub id: String,
    pub label: String,
    pub kind: Kind,
    pub default: OptionValue,
}

impl ThemeOption {
    /// Missing values are quiet; invalid saved values log once and use the manifest default.
    pub fn saved_value(&self, saved: Option<&Json>, mut invalid: impl FnMut(&str)) -> OptionValue {
        if let Some(saved) = saved {
            match self.kind.parse_default(saved) {
                Ok(value) => return value,
                Err(e) => invalid(&format!("theme option '{}': invalid saved value ({e}); using default", self.id)),
            }
        }
        self.default.clone()
    }
}

fn color(s: &str) -> Option<[u8; 3]> {
    if s.len() != 7 || !s.starts_with('#') || !s.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    Some([u8::from_str_radix(&s[1..3], 16).ok()?, u8::from_str_radix(&s[3..5], 16).ok()?, u8::from_str_radix(&s[5..7], 16).ok()?])
}

fn short_string(value: Option<&Json>, max: usize) -> Option<&str> {
    value?.as_str().filter(|s| !s.is_empty() && s.chars().count() <= max)
}

fn parse_option(value: &Json) -> Result<ThemeOption, String> {
    let obj = value.as_object().ok_or("must be an object")?;
    let id = obj.get("id").and_then(Json::as_str)
        .filter(|id| (1..=32).contains(&id.len())
            && id.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-'))
        .ok_or("id must match [a-z0-9-]{1,32}")?;
    let label = short_string(obj.get("label"), 32).ok_or("label must be a non-empty string up to 32 chars")?;
    let kind = match obj.get("type").and_then(Json::as_str) {
        Some("color") => Kind::Color,
        Some("bool") => Kind::Bool,
        Some("int") => {
            let bound = |name: &str, fallback: i32| -> Result<i32, String> {
                match obj.get(name) {
                    None => Ok(fallback),
                    Some(v) => v.as_i64().and_then(|n| i32::try_from(n).ok())
                        .ok_or_else(|| format!("{name} must be a 32-bit integer")),
                }
            };
            let (min, max) = (bound("min", 0)?, bound("max", 100)?);
            if min >= max { return Err("int min must be less than max".into()); }
            Kind::Int { min, max }
        }
        Some("choice") => {
            let values = obj.get("values").and_then(Json::as_array)
                .filter(|v| (2..=8).contains(&v.len()))
                .ok_or("choice values must be an array of 2-8 strings")?;
            let mut seen = HashSet::new();
            let mut out = Vec::new();
            for value in values {
                let s = short_string(Some(value), 24).ok_or("choice values must be non-empty strings up to 24 chars")?;
                if !seen.insert(s) { return Err("choice values must be unique".into()); }
                out.push(s.into());
            }
            Kind::Choice(out)
        }
        _ => return Err("type must be color, bool, int or choice".into()),
    };
    if !matches!(&kind, Kind::Choice(_)) && obj.contains_key("values") {
        return Err("values is only allowed for choice options".into());
    }
    let default = kind.parse_default(obj.get("default").ok_or("default missing")?)
        .map_err(|e| format!("default: {e}"))?;
    Ok(ThemeOption { id: id.into(), label: label.into(), kind, default })
}

/// An absent array is valid. Any invalid entry rejects the whole options array.
pub fn parse(manifest: &Json) -> Result<Vec<ThemeOption>, Vec<String>> {
    let Some(value) = manifest.get("options") else { return Ok(Vec::new()) };
    let Some(values) = value.as_array() else { return Err(vec!["theme.json: options must be an array".into()]) };
    if values.len() > 16 { return Err(vec!["theme.json: options must have at most 16 entries".into()]); }
    let mut out = Vec::new();
    let mut errors = Vec::new();
    let mut ids = HashSet::new();
    for (i, value) in values.iter().enumerate() {
        match parse_option(value) {
            Ok(option) if !ids.insert(option.id.clone()) => errors.push(format!("theme.json: options[{i}]: duplicate id '{}'", option.id)),
            Ok(option) => out.push(option),
            Err(e) => errors.push(format!("theme.json: options[{i}]: {e}")),
        }
    }
    if errors.is_empty() { Ok(out) } else { Err(errors) }
}

fn strings(values: &[String]) -> Value {
    Value::Model(ModelRc::new(VecModel::from(values.iter().map(|s| Value::String(s.clone().into())).collect::<Vec<_>>())))
}

fn info(option: &ThemeOption, current: &OptionValue) -> Value {
    let (kind, values, min, max) = match &option.kind {
        Kind::Color => ("color", &[][..], 0, 0),
        Kind::Bool => ("bool", &[][..], 0, 0),
        Kind::Int { min, max } => ("int", &[][..], *min, *max),
        Kind::Choice(values) => ("choice", values.as_slice(), 0, 0),
    };
    let mut swatches = Vec::new();
    if let OptionValue::Color(default) = &option.default {
        let mut colors = vec![*default];
        for preset in ["#ffc94a", "#4cc9f0", "#5cff9d", "#ff6b6b", "#f4a261", "#b388ff", "#ffffff"] {
            let rgb = color(preset).unwrap();
            if rgb != *default { colors.push(rgb); }
        }
        swatches = colors.into_iter().map(|rgb| {
            let value = OptionValue::Color(rgb);
            Value::Struct([("color".into(), value.value()), ("text".into(), Value::String(value.json().as_str().unwrap().into()))].into_iter().collect())
        }).collect();
    }
    Value::Struct([
        ("id", Value::String(option.id.clone().into())),
        ("label", Value::String(option.label.clone().into())),
        ("kind", Value::String(kind.into())),
        ("values", strings(values)),
        ("min", Value::Number(min as f64)), ("max", Value::Number(max as f64)),
        ("color", if matches!(current, OptionValue::Color(_)) { current.value() } else { OptionValue::Color([0, 0, 0]).value() }),
        ("flag", Value::Bool(matches!(current, OptionValue::Bool(true)))),
        ("number", Value::Number(if let OptionValue::Int(n) = current { *n as f64 } else { 0.0 })),
        ("text", Value::String(if let OptionValue::Choice(s) = current { s.clone().into() } else { "".into() })),
        ("selected", Value::Number(if let OptionValue::Choice(s) = current { values.iter().position(|v| v == s).unwrap_or(0) as f64 } else { 0.0 })),
        ("swatches", Value::Model(ModelRc::new(VecModel::from(swatches)))),
    ].into_iter().map(|(k, v)| (k.into(), v)).collect())
}

/// Shared host/sim path: validated saved values, run-only overrides, and gated live edits.
/// Only generated root opt-* properties reach Dash; no input can name a host global.
pub fn bind(ui: &ComponentInstance, options: &[ThemeOption], theme: &str, settings: SharedSettings,
    overrides: &[(String, String)], log: fn(&str)) -> Result<(), String> {
    let mut current: Vec<_> = options.iter().map(|o| o.saved_value(settings.borrow().option(theme, &o.id), log)).collect();
    let mut skipped = HashSet::new();
    for (id, text) in overrides {
        let Some(i) = options.iter().position(|o| &o.id == id) else {
            if skipped.insert(id) { log(&format!("theme option '{id}' skipped: not declared by the theme")); }
            continue;
        };
        current[i] = options[i].kind.parse_override(text).map_err(|e| format!("--opt {id}: {e}"))?;
    }
    for (option, value) in options.iter().zip(&current) {
        ui.set_property(&format!("opt-{}", option.id), value.value())
            .map_err(|e| format!("theme option '{}': {e:?}", option.id))?;
    }
    let rows = Rc::new(VecModel::from(options.iter().zip(&current).map(|(o, v)| info(o, v)).collect::<Vec<_>>()));
    ui.set_property("theme-options", Value::Model(ModelRc::from(rows.clone())))
        .map_err(|e| format!("theme-options: {e:?}"))?;
    let weak = ui.as_weak();
    let options = options.to_vec();
    let theme = theme.to_string();
    ui.set_callback("set-option", move |args| {
        let Some(ui) = weak.upgrade() else { return Value::Void };
        let change = || -> Result<(), String> {
            if !matches!(ui.get_property("host-screens-allowed"), Ok(Value::Bool(true))) {
                return Err("requires parked, no alert, unlocked and not test mode".into());
            }
            let [Value::String(id), Value::String(text)] = args else { return Err("expected id and value text".into()) };
            let i = options.iter().position(|o| o.id == id.as_str()).ok_or_else(|| format!("unknown option '{id}'"))?;
            let option = &options[i];
            let value = option.kind.parse_override(text.as_str()).map_err(|e| format!("'{id}': {e}"))?;
            let name = format!("opt-{id}");
            let previous = ui.get_property(&name).map_err(|e| format!("{name}: {e:?}"))?;
            ui.set_property(&name, value.value()).map_err(|e| format!("{name}: {e:?}"))?;
            let saved = settings.borrow_mut().set_option(&theme, id.as_str(), value.json());
            if let Err(e) = saved {
                ui.set_property(&name, previous).map_err(|rollback| format!("{e}; restoring {name}: {rollback:?}"))?;
                return Err(e);
            }
            rows.set_row_data(i, info(option, &value));
            Ok(())
        };
        if let Err(e) = change() { log(&format!("set-option refused: {e}")); }
        Value::Void
    }).map_err(|e| format!("set-option: {e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn option(ty: &str, default: Json) -> Json {
        json!({"id": "accent", "label": "Accent", "type": ty, "default": default})
    }

    #[test]
    fn parses_all_types_and_absent_options() {
        assert!(parse(&json!({})).unwrap().is_empty());
        assert!(parse(&json!({"options": []})).unwrap().is_empty());
        let options = parse(&json!({"options": [
            option("color", json!("#aA22ff")),
            {"id":"on", "label":"On", "type":"bool", "default":true},
            {"id":"n", "label":"Count", "type":"int", "default":-42, "min":-100},
            {"id":"style", "label":"Style", "type":"choice", "default":"thin", "values":["thin", "wide"]}
        ]})).unwrap();
        assert_eq!(options[0].default, OptionValue::Color([170, 34, 255]));
        assert_eq!(options[1].default, OptionValue::Bool(true));
        assert_eq!(options[2].default, OptionValue::Int(-42));
        assert_eq!(options[3].default, OptionValue::Choice("thin".into()));
        for o in options { assert_eq!(o.default.value().value_type(), o.kind.probe_type()); }
    }

    #[test]
    fn rejects_bad_arrays_and_duplicate_ids() {
        for value in [Json::Null, json!({}), json!(false), json!(vec![option("bool", json!(true)); 17])] {
            assert!(parse(&json!({"options": value})).is_err());
        }
        let same = option("bool", json!(false));
        assert!(parse(&json!({"options": [same.clone(), same]})).unwrap_err()[0].contains("duplicate"));
        assert_eq!(parse(&json!({"options": [null, false]})).unwrap_err().len(), 2);
        let entries: Vec<_> = (0..16).map(|i| {
            let mut o = option("int", json!(i)); o["id"] = json!(format!("n-{i}")); o
        }).collect();
        assert_eq!(parse(&json!({"options": entries})).unwrap().len(), 16);
    }

    #[test]
    fn enforces_ids_labels_and_defaults() {
        for id in ["", "A", "a_b", "a.b", "é", "a;}", &"a".repeat(33)] {
            let mut o = option("bool", json!(true)); o["id"] = json!(id);
            assert!(parse(&json!({"options": [o]})).is_err(), "{id}");
        }
        for label in [json!(""), json!("a".repeat(33)), json!(32)] {
            let mut o = option("bool", json!(true)); o["label"] = label;
            assert!(parse(&json!({"options": [o]})).is_err());
        }
        let mut o = option("int", json!(0)); o["id"] = json!("a".repeat(32)); o["label"] = json!("é".repeat(32));
        assert!(parse(&json!({"options": [o]})).is_ok());
        for (ty, bad) in [("color", json!("#fff")), ("color", json!("#12345678")), ("color", json!("#gg0000")),
            ("bool", json!("true")), ("int", json!(1.5)), ("int", json!(1.0)), ("int", json!("1")), ("float", json!(1))] {
            assert!(parse(&json!({"options": [option(ty, bad)]})).is_err(), "{ty}");
        }
        let mut o = option("bool", json!(false)); o.as_object_mut().unwrap().remove("default");
        assert!(parse(&json!({"options": [o]})).is_err());
    }

    #[test]
    fn enforces_choice_values() {
        for values in [json!(["a"]), json!(["a", "a"]), json!(["a", ""]), json!(["a", 2]),
            json!(["a", "x".repeat(25)]), json!(["a", "b", "c", "d", "e", "f", "g", "h", "i"]), Json::Null] {
            let mut o = option("choice", json!("a")); o["values"] = values;
            assert!(parse(&json!({"options": [o]})).is_err());
        }
        let mut o = option("choice", json!("absent")); o["values"] = json!(["a", "b"]);
        assert!(parse(&json!({"options": [o]})).is_err());
        let mut o = option("bool", json!(true)); o["values"] = json!(["a", "b"]);
        assert!(parse(&json!({"options": [o]})).is_err());
        let mut o = option("choice", json!("a"));
        o["values"] = json!(["a", "b", "c", "d", "e", "f", "g", "é".repeat(24)]);
        assert!(parse(&json!({"options": [o]})).is_ok());
    }

    #[test]
    fn parses_typed_overrides() {
        for (kind, good, bad) in [(Kind::Color, "#ff0000", "red"), (Kind::Bool, "false", "False"),
            (Kind::Int { min: -200, max: 200 }, "-123", "1.0"), (Kind::Choice(vec!["a".into(), "b".into()]), "b", "c")] {
            assert!(kind.parse_override(good).is_ok());
            assert!(kind.parse_override(bad).is_err());
        }
        assert!(Kind::Int { min: i32::MIN, max: i32::MAX }.parse_override("2147483647").is_ok());
        assert!(Kind::Int { min: i32::MIN, max: i32::MAX }.parse_override("2147483648").is_err());
        assert!(Kind::Int { min: i32::MIN, max: i32::MAX }.parse_override("18446744073709551616").is_err());
    }

    #[test]
    fn int_bounds_default_and_validate_manifest_and_overrides() {
        let parsed = parse(&json!({"options": [option("int", json!(50))]})).unwrap();
        assert_eq!(parsed[0].kind, Kind::Int { min: 0, max: 100 });
        for text in ["0", "100"] { assert!(parsed[0].kind.parse_override(text).is_ok()); }
        for text in ["-1", "101"] { assert!(parsed[0].kind.parse_override(text).is_err()); }
        for default in [-1, 101] {
            assert!(parse(&json!({"options": [option("int", json!(default))]})).is_err());
        }
        let bounded = |min: Json, max: Json, default: Json| {
            json!({"options": [{"id":"n", "label":"Count", "type":"int", "min":min, "max":max, "default":default}]})
        };
        for manifest in [bounded(json!(2), json!(2), json!(2)), bounded(json!(3), json!(2), json!(2)),
            bounded(json!(0), json!(2), json!(3)), bounded(json!(0), json!(2), json!(-1))] {
            assert!(parse(&manifest).is_err());
        }
        for bad in [json!(-2147483649_i64), json!(2147483648_i64), json!(1.0), json!("1"), Json::Null, json!(true)] {
            assert!(parse(&bounded(bad.clone(), json!(100), json!(50))).is_err());
            assert!(parse(&bounded(json!(0), bad, json!(0))).is_err());
        }
        let extremes = parse(&bounded(json!(i32::MIN), json!(i32::MAX), json!(i32::MIN))).unwrap();
        assert!(extremes[0].kind.parse_override("2147483647").is_ok());
        assert!(extremes[0].kind.parse_override("-2147483648").is_ok());
        let mut only_min = option("int", json!(-1)); only_min["min"] = json!(-2);
        let mut only_max = option("int", json!(3)); only_max["max"] = json!(4);
        assert_eq!(parse(&json!({"options": [only_min]})).unwrap()[0].kind, Kind::Int { min: -2, max: 100 });
        assert_eq!(parse(&json!({"options": [only_max]})).unwrap()[0].kind, Kind::Int { min: 0, max: 4 });
    }

    #[test]
    fn saved_values_are_typed_and_invalid_values_fall_back_once() {
        let options = parse(&json!({"options": [
            option("color", json!("#123456")),
            {"id":"enabled", "label":"Enabled", "type":"bool", "default":true},
            {"id":"count", "label":"Count", "type":"int", "min":-2, "max":8, "default":3},
            {"id":"style", "label":"Style", "type":"choice", "values":["thin", "wide"], "default":"wide"}
        ]})).unwrap();
        let bad = [json!(42), json!("true"), json!(9), json!("unknown")];
        let good = [json!("#aAbBcC"), json!(false), json!(-2), json!("thin")];
        for ((option, bad), good) in options.iter().zip(bad).zip(good) {
            let mut lines = Vec::new();
            assert_eq!(option.saved_value(Some(&bad), |m| lines.push(m.to_string())), option.default);
            assert_eq!(lines.len(), 1);
            assert!(lines[0].contains(&option.id));
            assert_eq!(option.saved_value(None, |_| panic!("missing value is quiet")), option.default);
            assert_eq!(option.saved_value(Some(&good), |_| panic!("valid value is quiet")), option.kind.parse_default(&good).unwrap());
        }
        for bad in [json!("3"), json!(3.0), json!(i64::MAX), json!(-3), Json::Null] {
            assert_eq!(options[2].saved_value(Some(&bad), |_| {}), options[2].default);
        }
    }

    #[test]
    fn swatches_keep_default_first_and_skip_duplicate_presets() {
        for (default, count) in [("#123456", 8), ("#FFC94A", 7)] {
            let option = parse(&json!({"options": [option("color", json!(default))]})).unwrap().remove(0);
            let Value::Struct(row) = info(&option, &option.default) else { panic!("row must be a struct") };
            let Some(Value::Model(swatches)) = row.get_field("swatches") else { panic!("missing swatches") };
            assert_eq!(swatches.row_count(), count);
            let Value::Struct(first) = swatches.row_data(0).unwrap() else { panic!("swatch must be a struct") };
            assert_eq!(first.get_field("color"), Some(&option.default.value()));
        }
    }
}
