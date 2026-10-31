//! Theme manifest options, shared by the Android host and sim. No Android code.

use serde_json::Value as Json;
use slint::{Brush, Color};
use slint_interpreter::{ComponentInstance, Value, ValueType};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Color,
    Bool,
    Int,
    Choice(Vec<String>),
}

impl Kind {
    pub fn slint_type(&self) -> &'static str {
        match self {
            Self::Color => "color",
            Self::Bool => "bool",
            Self::Int => "int",
            Self::Choice(_) => "string",
        }
    }

    pub fn probe_type(&self) -> ValueType {
        match self {
            Self::Color => ValueType::Brush,
            Self::Bool => ValueType::Bool,
            Self::Int => ValueType::Number,
            Self::Choice(_) => ValueType::String,
        }
    }

    fn parse_default(&self, value: &Json) -> Result<OptionValue, String> {
        match self {
            Self::Color => value.as_str().and_then(color).map(OptionValue::Color)
                .ok_or_else(|| "expected #rrggbb string".into()),
            Self::Bool => value.as_bool().map(OptionValue::Bool)
                .ok_or_else(|| "expected JSON bool".into()),
            Self::Int => match value {
                Json::Number(n) => n.as_i64().and_then(|v| i32::try_from(v).ok()).map(OptionValue::Int)
                    .ok_or_else(|| "expected 32-bit integer".into()),
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
            Self::Int => return text.parse::<i32>().map(OptionValue::Int)
                .map_err(|_| "expected 32-bit integer".into()),
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
        Some("int") => Kind::Int,
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

/// Only root opt-* properties are written; no option can name a host global or screen.
pub fn set_defaults(ui: &ComponentInstance, options: &[ThemeOption]) -> Result<(), String> {
    for option in options {
        ui.set_property(&format!("opt-{}", option.id), option.default.value())
            .map_err(|e| format!("theme option '{}': {e:?}", option.id))?;
    }
    Ok(())
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
            {"id":"n", "label":"Count", "type":"int", "default":-42},
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
            (Kind::Int, "-123", "1.0"), (Kind::Choice(vec!["a".into(), "b".into()]), "b", "c")] {
            assert!(kind.parse_override(good).is_ok());
            assert!(kind.parse_override(bad).is_err());
        }
        assert!(Kind::Int.parse_override("2147483647").is_ok());
        assert!(Kind::Int.parse_override("2147483648").is_err());
        assert!(Kind::Int.parse_override("18446744073709551616").is_err());
    }
}
