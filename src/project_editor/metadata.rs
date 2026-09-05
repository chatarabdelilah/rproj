use std::collections::BTreeMap;

use rbx_reflection::{
    ClassTag, DataType, PropertyKind, PropertySerialization, PropertyTag, ReflectionDatabase,
};
use rbx_types::VariantType;
use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueKind {
    Bool,
    String,
    Integer,
    Number,
    Enum(Vec<String>),
    BrickColor,
    Color3,
    Vector2,
    Vector3,
    UDim,
    UDim2,
    CFrame,
    NumberRange,
    Rect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassInfo {
    pub name: String,
    pub service: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertyInfo {
    pub name: String,
    pub kind: ValueKind,
}

pub struct Metadata {
    database: &'static ReflectionDatabase<'static>,
}

impl Metadata {
    pub fn bundled() -> Self {
        Self {
            database: rbx_reflection_database::get_bundled(),
        }
    }

    pub fn classes(&self, at_root: bool) -> Vec<ClassInfo> {
        let mut classes: Vec<_> = self
            .database
            .classes
            .values()
            .filter(|class| {
                !class.tags.contains(&ClassTag::Deprecated)
                    && !class.tags.contains(&ClassTag::NotBrowsable)
                    && if at_root {
                        class.tags.contains(&ClassTag::Service)
                            || !class.tags.contains(&ClassTag::NotCreatable)
                    } else {
                        !class.tags.contains(&ClassTag::NotCreatable)
                            && !class.tags.contains(&ClassTag::Service)
                    }
            })
            .map(|class| ClassInfo {
                name: class.name.to_string(),
                service: class.tags.contains(&ClassTag::Service),
            })
            .collect();
        classes.sort_by(|a, b| a.name.cmp(&b.name));
        classes
    }

    pub fn properties(&self, class_name: &str) -> Vec<PropertyInfo> {
        let Some(class) = self.database.classes.get(class_name) else {
            return Vec::new();
        };
        let mut properties = BTreeMap::new();
        for class in self.database.superclasses_iter(class) {
            for property in class.properties.values() {
                if property.name == "Name"
                    || property.tags.contains(&PropertyTag::Deprecated)
                    || property.tags.contains(&PropertyTag::Hidden)
                    || property.tags.contains(&PropertyTag::NotBrowsable)
                    || property.tags.contains(&PropertyTag::ReadOnly)
                {
                    continue;
                }
                let serializes = matches!(
                    property.kind,
                    PropertyKind::Canonical {
                        serialization: PropertySerialization::Serializes
                    }
                );
                if serializes && let Some(kind) = self.value_kind(&property.data_type) {
                    properties.entry(property.name.to_string()).or_insert(kind);
                }
            }
        }
        properties
            .into_iter()
            .map(|(name, kind)| PropertyInfo { name, kind })
            .collect()
    }

    pub fn property(&self, class_name: &str, name: &str) -> Option<PropertyInfo> {
        self.properties(class_name)
            .into_iter()
            .find(|property| property.name == name)
    }

    fn value_kind(&self, data_type: &DataType<'_>) -> Option<ValueKind> {
        match data_type {
            DataType::Enum(name) => {
                let mut items: Vec<_> = self
                    .database
                    .enums
                    .get(name)?
                    .items
                    .keys()
                    .map(ToString::to_string)
                    .collect();
                items.sort();
                Some(ValueKind::Enum(items))
            }
            DataType::Value(kind) => match kind {
                VariantType::Bool => Some(ValueKind::Bool),
                VariantType::String | VariantType::ContentId | VariantType::Content => {
                    Some(ValueKind::String)
                }
                VariantType::Float32 | VariantType::Float64 => Some(ValueKind::Number),
                VariantType::Int32 | VariantType::Int64 => Some(ValueKind::Integer),
                VariantType::BrickColor => Some(ValueKind::BrickColor),
                VariantType::Color3 => Some(ValueKind::Color3),
                VariantType::Vector2 => Some(ValueKind::Vector2),
                VariantType::Vector3 => Some(ValueKind::Vector3),
                VariantType::UDim => Some(ValueKind::UDim),
                VariantType::UDim2 => Some(ValueKind::UDim2),
                VariantType::CFrame => Some(ValueKind::CFrame),
                VariantType::NumberRange => Some(ValueKind::NumberRange),
                VariantType::Rect => Some(ValueKind::Rect),
                _ => None,
            },
            _ => None,
        }
    }
}

impl ValueKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Bool => "boolean",
            Self::String => "text",
            Self::Integer => "integer",
            Self::Number => "number",
            Self::Enum(_) => "enum",
            Self::BrickColor => "BrickColor number",
            Self::Color3 => "R, G, B (0-1)",
            Self::Vector2 => "X, Y",
            Self::Vector3 => "X, Y, Z",
            Self::UDim => "scale, offset",
            Self::UDim2 => "x scale, x offset, y scale, y offset",
            Self::CFrame => "12 CFrame components",
            Self::NumberRange => "min, max",
            Self::Rect => "min X, min Y, max X, max Y",
        }
    }

    pub fn default_value(&self) -> Value {
        match self {
            Self::Bool => json!(false),
            Self::String => json!(""),
            Self::Integer => json!(0),
            Self::Number => json!(0),
            Self::Enum(items) => json!(items.first().cloned().unwrap_or_default()),
            Self::BrickColor => json!({ "BrickColor": 194 }),
            Self::Color3 => json!([0.0, 0.0, 0.0]),
            Self::Vector2 | Self::NumberRange => json!([0.0, 0.0]),
            Self::UDim => json!({"UDim": [0.0, 0]}),
            Self::Vector3 => json!([0.0, 0.0, 0.0]),
            Self::UDim2 => json!({"UDim2": [[0.0, 0], [0.0, 0]]}),
            Self::Rect => json!({"Rect": [[0.0, 0.0], [0.0, 0.0]]}),
            Self::CFrame => json!([0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]),
        }
    }

    pub fn parse(&self, input: &str) -> Result<Value, String> {
        match self {
            Self::Bool => input
                .parse::<bool>()
                .map(Value::Bool)
                .map_err(|_| "enter true or false".into()),
            Self::String => Ok(Value::String(input.to_string())),
            Self::Integer => input
                .parse::<i64>()
                .map(|number| json!(number))
                .map_err(|_| "enter a whole number".into()),
            Self::Number => input
                .parse::<f64>()
                .ok()
                .filter(|number| number.is_finite())
                .map(|number| json!(number))
                .ok_or_else(|| "enter a finite number".into()),
            Self::Enum(items) => items
                .iter()
                .find(|item| item.eq_ignore_ascii_case(input))
                .cloned()
                .map(Value::String)
                .ok_or_else(|| "choose an enum item from the list".into()),
            Self::BrickColor => input
                .parse::<u16>()
                .map(|number| json!({ "BrickColor": number }))
                .map_err(|_| "enter a BrickColor number from 0 to 65535".into()),
            Self::Color3 => parse_components(input, 3),
            Self::Vector2 | Self::NumberRange => parse_components(input, 2),
            Self::UDim => {
                let values = parse_components(input, 2)?;
                Ok(json!({"UDim": [values[0], offset(&values[1])?]}))
            }
            Self::Vector3 => parse_components(input, 3),
            Self::UDim2 => {
                let values = parse_components(input, 4)?;
                Ok(
                    json!({"UDim2": [[values[0], offset(&values[1])?], [values[2], offset(&values[3])?]]}),
                )
            }
            Self::Rect => {
                let values = parse_components(input, 4)?;
                Ok(json!({"Rect": [[values[0], values[1]], [values[2], values[3]]]}))
            }
            Self::CFrame => parse_components(input, 12),
        }
    }

    pub fn accepts(&self, value: &Value) -> bool {
        let explicit = |names: &[&str]| {
            value.as_object().and_then(|object| {
                if object.len() == 1 {
                    names.iter().find_map(|name| object.get(*name))
                } else {
                    None
                }
            })
        };
        match self {
            Self::Bool => value.is_boolean() || explicit(&["Bool"]).is_some_and(Value::is_boolean),
            Self::String => {
                value.is_string()
                    || explicit(&["String", "Content", "ContentId"]).is_some_and(Value::is_string)
            }
            Self::Integer => {
                value.as_i64().is_some()
                    || value.as_u64().is_some()
                    || explicit(&["Int32", "Int64"])
                        .is_some_and(|value| value.as_i64().is_some() || value.as_u64().is_some())
            }
            Self::Number => {
                value.is_number() || explicit(&["Float32", "Float64"]).is_some_and(Value::is_number)
            }
            Self::Enum(items) => value
                .as_str()
                .is_some_and(|value| items.iter().any(|item| item == value)),
            Self::BrickColor => {
                value.is_number() || explicit(&["BrickColor"]).is_some_and(Value::is_number)
            }
            Self::Color3 => accepts_array(value, 3, &["Color3", "Color3uint8"]),
            Self::Vector2 => accepts_array(value, 2, &["Vector2"]),
            Self::Vector3 => accepts_array(value, 3, &["Vector3"]),
            Self::UDim => explicit(&["UDim"]).is_some_and(valid_udim),
            Self::UDim2 => explicit(&["UDim2"]).is_some_and(|value| {
                value
                    .as_array()
                    .is_some_and(|values| values.len() == 2 && values.iter().all(valid_udim))
            }),
            Self::CFrame => {
                accepts_array(value, 12, &[])
                    || explicit(&["CFrame"]).is_some_and(|value| {
                        value.as_object().is_some_and(|fields| {
                            fields.len() == 2
                                && fields
                                    .get("position")
                                    .is_some_and(|position| accepts_array(position, 3, &[]))
                                && fields
                                    .get("orientation")
                                    .and_then(Value::as_array)
                                    .is_some_and(|rows| {
                                        rows.len() == 3
                                            && rows.iter().all(|row| accepts_array(row, 3, &[]))
                                    })
                        })
                    })
            }
            Self::NumberRange => accepts_array(value, 2, &["NumberRange"]),
            Self::Rect => explicit(&["Rect"]).is_some_and(|value| {
                value.as_array().is_some_and(|values| {
                    values.len() == 2 && values.iter().all(|value| accepts_array(value, 2, &[]))
                })
            }),
        }
    }
}

fn offset(value: &Value) -> Result<i32, String> {
    value
        .as_f64()
        .filter(|value| {
            value.is_finite()
                && value.fract() == 0.0
                && *value >= i32::MIN as f64
                && *value <= i32::MAX as f64
        })
        .map(|value| value as i32)
        .ok_or_else(|| "offset must be a whole number in the signed 32-bit range".into())
}

fn valid_udim(value: &Value) -> bool {
    value.as_array().is_some_and(|values| {
        values.len() == 2
            && values[0].is_number()
            && values[1]
                .as_i64()
                .is_some_and(|value| i32::try_from(value).is_ok())
    })
}

fn accepts_array(value: &Value, length: usize, explicit_names: &[&str]) -> bool {
    let value = value
        .as_object()
        .and_then(|object| {
            (object.len() == 1)
                .then(|| explicit_names.iter().find_map(|name| object.get(*name)))
                .flatten()
        })
        .unwrap_or(value);
    value
        .as_array()
        .is_some_and(|values| values.len() == length && values.iter().all(Value::is_number))
}

fn parse_components(input: &str, expected: usize) -> Result<Value, String> {
    let values: Result<Vec<_>, _> = input
        .split(',')
        .map(|value| value.trim().parse::<f64>())
        .collect();
    let values = values.map_err(|_| format!("enter {expected} comma-separated numbers"))?;
    if values.len() != expected {
        return Err(format!("enter exactly {expected} comma-separated numbers"));
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Err("all components must be finite numbers".into());
    }
    Ok(json!(values))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_inputs_reject_non_finite_values_and_fractional_offsets() {
        for input in ["NaN", "inf", "-inf", "1e999"] {
            assert!(ValueKind::Number.parse(input).is_err());
            assert!(ValueKind::Vector2.parse(&format!("0, {input}")).is_err());
        }
        for input in ["1, 2.5", "1, 2147483648", "1, -2147483649"] {
            assert!(ValueKind::UDim.parse(input).is_err());
        }
        assert_eq!(
            ValueKind::UDim.parse("0.5, -10").unwrap(),
            json!({"UDim": [0.5, -10]})
        );
    }

    #[test]
    fn compound_defaults_and_parsed_values_use_valid_explicit_shapes() {
        for (kind, input) in [
            (ValueKind::UDim, "0.5, 10"),
            (ValueKind::UDim2, "0.5, 10, 1, -20"),
            (ValueKind::Rect, "1, 2, 3, 4"),
        ] {
            assert!(kind.accepts(&kind.default_value()));
            assert!(kind.accepts(&kind.parse(input).unwrap()));
        }
        assert!(!ValueKind::UDim2.accepts(&json!({"UDim2": [1, 2, 3, 4]})));
        assert!(!ValueKind::Rect.accepts(&json!({"Rect": [1, 2, 3, 4]})));
        assert!(!ValueKind::CFrame.accepts(&json!({"CFrame": vec![0; 12]})));
        assert!(ValueKind::CFrame.accepts(&json!({"CFrame": {
            "position": [1, 2, 3], "orientation": [[1, 0, 0], [0, 1, 0], [0, 0, 1]]
        }})));
    }

    #[test]
    fn bundled_catalog_filters_uncreatable_children_but_keeps_services_at_root() {
        let metadata = Metadata::bundled();
        assert!(
            metadata
                .classes(false)
                .iter()
                .any(|class| class.name == "Part")
        );
        assert!(!metadata.classes(false).iter().any(|class| class.service));
        assert!(
            metadata
                .classes(true)
                .iter()
                .any(|class| class.name == "Workspace" && class.service)
        );
    }

    #[test]
    fn common_property_types_are_exposed() {
        let metadata = Metadata::bundled();
        assert_eq!(
            metadata.property("Part", "Anchored").unwrap().kind,
            ValueKind::Bool
        );
        assert!(matches!(
            metadata.property("Part", "Material").unwrap().kind,
            ValueKind::Enum(_)
        ));
        assert_eq!(
            ValueKind::Vector3.parse("1, 2, 3").unwrap(),
            json!([1.0, 2.0, 3.0])
        );
        assert_eq!(ValueKind::Integer.parse("42").unwrap(), json!(42));
        assert!(ValueKind::Bool.accepts(&json!(true)));
        assert!(!ValueKind::Bool.accepts(&json!("true")));
    }

    #[test]
    fn instance_name_is_owned_by_the_tree_key() {
        let metadata = Metadata::bundled();
        assert!(metadata.property("Part", "Name").is_none());
    }
}
