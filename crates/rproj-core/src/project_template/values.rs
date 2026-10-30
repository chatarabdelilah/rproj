use super::{
    metadata::ValueKind,
    model::{EditorModel, representable},
};
use anyhow::{Context, Result};
use serde_json::{Value, json};

pub const SETTINGS: &[(&str, &str, SettingKind)] = &[
    ("servePort", "Serve port", SettingKind::Port),
    ("serveAddress", "Serve address", SettingKind::Text),
    (
        "servePlaceIds",
        "Allowed place IDs",
        SettingKind::IntegerList,
    ),
    ("placeId", "Place ID", SettingKind::Integer),
    ("gameId", "Game ID", SettingKind::Integer),
    ("serveAllowedHosts", "Allowed hosts", SettingKind::TextList),
    (
        "globIgnorePaths",
        "Ignored path globs",
        SettingKind::TextList,
    ),
    (
        "emitLegacyScripts",
        "Emit legacy scripts",
        SettingKind::Bool,
    ),
];

#[derive(Clone, Copy)]
pub enum SettingKind {
    Port,
    Text,
    Integer,
    IntegerList,
    TextList,
    Bool,
}

pub fn set_attribute(
    model: &mut EditorModel,
    path: &[String],
    name: &str,
    value: Value,
) -> Result<()> {
    let mut attributes = model
        .node(path)
        .and_then(|n| n.get("$properties"))
        .and_then(|p| p.get("Attributes"))
        .map(|value| {
            value.as_object().cloned().context(
                "Attributes has an unfamiliar shape; repair it explicitly in Advanced JSON",
            )
        })
        .transpose()?
        .unwrap_or_default();
    attributes.insert(name.into(), value);
    model.set_property(path, "Attributes", Value::Object(attributes))
}

pub fn attribute_value(kind: &ValueKind, value: Value) -> Value {
    let label = match kind {
        ValueKind::Bool => "Bool",
        ValueKind::String => "String",
        ValueKind::Integer => "Int64",
        ValueKind::Number => "Float64",
        ValueKind::BrickColor => return value,
        ValueKind::Color3 => "Color3",
        ValueKind::Vector2 => "Vector2",
        ValueKind::Vector3 => "Vector3",
        ValueKind::UDim | ValueKind::UDim2 | ValueKind::Rect => return value,
        ValueKind::CFrame => {
            return json!({"CFrame": {
                "position": [value[0], value[1], value[2]],
                "orientation": [[value[3], value[4], value[5]], [value[6], value[7], value[8]], [value[9], value[10], value[11]]]
            }});
        }
        ValueKind::NumberRange => "NumberRange",
        ValueKind::Enum(_) => "String",
    };
    json!({ label: value })
}

pub fn explicit_inner(value: &Value) -> Option<&Value> {
    value.as_object().and_then(|object| {
        if object.len() == 1 {
            object.values().next()
        } else {
            None
        }
    })
}

pub fn attribute_kind(value: &Value) -> Option<ValueKind> {
    let object = value.as_object()?;
    let (kind, _) = object.iter().next()?;
    if object.len() != 1 {
        return None;
    }
    match kind.as_str() {
        "Bool" => Some(ValueKind::Bool),
        "String" => Some(ValueKind::String),
        "Float64" | "Float32" => Some(ValueKind::Number),
        "Int32" | "Int64" => Some(ValueKind::Integer),
        "BrickColor" => Some(ValueKind::BrickColor),
        "Color3" => Some(ValueKind::Color3),
        "Vector2" => Some(ValueKind::Vector2),
        "Vector3" => Some(ValueKind::Vector3),
        "UDim" => Some(ValueKind::UDim),
        "UDim2" => Some(ValueKind::UDim2),
        "CFrame" => Some(ValueKind::CFrame),
        "NumberRange" => Some(ValueKind::NumberRange),
        "Rect" => Some(ValueKind::Rect),
        _ => None,
    }
}

pub fn attribute_kinds() -> Vec<(&'static str, ValueKind)> {
    vec![
        ("Boolean", ValueKind::Bool),
        ("String", ValueKind::String),
        ("Number", ValueKind::Number),
        ("BrickColor", ValueKind::BrickColor),
        ("Color3", ValueKind::Color3),
        ("Vector2", ValueKind::Vector2),
        ("Vector3", ValueKind::Vector3),
        ("UDim", ValueKind::UDim),
        ("UDim2", ValueKind::UDim2),
        ("CFrame", ValueKind::CFrame),
        ("NumberRange", ValueKind::NumberRange),
        ("Rect", ValueKind::Rect),
    ]
}

pub fn parse_setting(kind: SettingKind, text: &str) -> std::result::Result<Option<Value>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let value = match kind {
        SettingKind::Text => json!(text),
        SettingKind::Port => {
            let port = text
                .parse::<u16>()
                .map_err(|_| "enter a port from 0 to 65535")?;
            json!(port)
        }
        SettingKind::Integer => {
            let number = text
                .parse::<u64>()
                .map_err(|_| "enter a non-negative integer")?;
            json!(number)
        }
        SettingKind::IntegerList => {
            let values: Result<Vec<_>, _> =
                text.split(',').map(|v| v.trim().parse::<u64>()).collect();
            json!(values.map_err(|_| "enter comma-separated non-negative integers")?)
        }
        SettingKind::TextList => json!(
            text.split(',')
                .map(|v| v.trim())
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>()
        ),
        SettingKind::Bool => json!(text.parse::<bool>().map_err(|_| "enter true or false")?),
    };
    Ok(Some(value))
}

pub fn parse_explorer_json(text: &str) -> Result<Value> {
    let value: Value = serde_json::from_str(text).context("invalid JSON")?;
    representable(&value)?;
    super::validate_template_structure(&value)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_attributes_keep_exact_signed_boundaries() {
        for integer in [i64::MIN, i64::MAX, 9_007_199_254_740_993] {
            let original = json!({"Int64": integer});
            let kind = attribute_kind(&original).unwrap();
            let parsed = kind
                .parse(&explicit_inner(&original).unwrap().to_string())
                .unwrap();
            assert_eq!(attribute_value(&kind, parsed), original);
        }
        assert!(ValueKind::Integer.parse("9223372036854775808").is_err());
        assert!(ValueKind::Integer.parse("1.5").is_err());
    }

    #[test]
    fn unfamiliar_attribute_container_is_preserved_for_explicit_repair() {
        let mut value = super::super::builtin_project_template();
        value["tree"]["$properties"] = json!({"Attributes": "future representation"});
        let mut model = EditorModel::new(value.clone()).unwrap();
        assert!(set_attribute(&mut model, &[], "New", json!({"Bool": true})).is_err());
        assert_eq!(model.value(), &value);
    }

    #[test]
    fn settings_keep_unsigned_ids_ports_lists_and_explicit_booleans() {
        assert_eq!(
            parse_setting(SettingKind::Integer, "18446744073709551615").unwrap(),
            Some(json!(u64::MAX))
        );
        assert!(parse_setting(SettingKind::Integer, "-1").is_err());
        assert!(parse_setting(SettingKind::Port, "65536").is_err());
        assert_eq!(
            parse_setting(SettingKind::IntegerList, "42, 18446744073709551615").unwrap(),
            Some(json!([42, u64::MAX]))
        );
        assert_eq!(
            parse_setting(SettingKind::Bool, "false").unwrap(),
            Some(json!(false))
        );
        assert!(parse_setting(SettingKind::Bool, "yes").is_err());
        assert_eq!(parse_setting(SettingKind::Text, " ").unwrap(), None);
    }

    #[test]
    fn invalid_json_never_replaces_model_and_history_is_bounded() {
        let original = super::super::builtin_project_template();
        let mut model = EditorModel::new(original.clone()).unwrap();
        assert!(parse_explorer_json("{").is_err());
        assert!(model.replace_from_json(json!({"name": "bad"})).is_err());
        assert_eq!(model.value(), &original);
        for port in 1..=101 {
            model.set_setting("servePort", Some(json!(port))).unwrap();
        }
        for _ in 0..100 {
            model.undo();
        }
        assert_eq!(model.value()["servePort"], 1);
        model.undo();
        assert_eq!(model.value()["servePort"], 1);
        for _ in 0..100 {
            model.redo();
        }
        assert_eq!(model.value()["servePort"], 101);
    }
}
