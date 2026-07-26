//! Place-level defaults baked into every scaffolded `default.project.json`.
//!
//! Data-driven: adding a service, a child instance, or a property to every
//! new project is a matter of adding an entry to `PLACE_TEMPLATE` below -
//! `steps::rojo` renders whatever is here and needs no changes.

use serde_json::{json, Value};

/// A property value in the terms a Roblox developer thinks in. `Color` is
/// written as 0-255 components (what Studio's colour picker shows) and
/// converted to the 0-1 floats Rojo's project format expects on the way
/// out, so the catalog stays readable and copy-pasteable from Studio.
#[derive(Clone, Copy)]
pub enum PropValue {
    Number(f64),
    Color(u8, u8, u8),
}

impl PropValue {
    pub fn to_json(self) -> Value {
        match self {
            PropValue::Number(n) => json!(n),
            PropValue::Color(r, g, b) => {
                json!([f64::from(r) / 255.0, f64::from(g) / 255.0, f64::from(b) / 255.0])
            }
        }
    }

    /// How the value is shown in `rproj info`, in the same 0-255 terms it
    /// was authored in rather than the converted floats.
    pub fn display(self) -> String {
        match self {
            PropValue::Number(n) => format!("{n}"),
            PropValue::Color(r, g, b) => format!("{r}, {g}, {b}"),
        }
    }
}

pub struct PropertySpec {
    pub name: &'static str,
    pub value: PropValue,
}

pub struct InstanceSpec {
    /// Instance name in the DataModel. For a service this is the service
    /// name itself (`Lighting`); for a child it's whatever it should be
    /// called under its parent.
    pub name: &'static str,
    pub class_name: &'static str,
    /// `None` for a service sitting directly under the DataModel, otherwise
    /// the `name` of the entry in this same table that owns it.
    pub parent: Option<&'static str>,
    pub properties: &'static [PropertySpec],
}

/// The starting look every scaffolded project gets, so a new place opens
/// with the intended lighting instead of Studio's defaults.
pub const PLACE_TEMPLATE: &[InstanceSpec] = &[
    InstanceSpec {
        name: "Lighting",
        class_name: "Lighting",
        parent: None,
        properties: &[
            PropertySpec { name: "Ambient", value: PropValue::Color(200, 160, 225) },
            PropertySpec { name: "Brightness", value: PropValue::Number(2.5) },
            PropertySpec { name: "ColorShift_Bottom", value: PropValue::Color(0, 0, 0) },
            PropertySpec { name: "ColorShift_Top", value: PropValue::Color(214, 189, 135) },
            PropertySpec { name: "EnvironmentDiffuseScale", value: PropValue::Number(0.5) },
            PropertySpec { name: "EnvironmentSpecularScale", value: PropValue::Number(1.0) },
            PropertySpec { name: "OutdoorAmbient", value: PropValue::Color(124, 100, 149) },
            PropertySpec { name: "FogColor", value: PropValue::Color(200, 170, 249) },
            PropertySpec { name: "FogEnd", value: PropValue::Number(2500.0) },
            PropertySpec { name: "FogStart", value: PropValue::Number(0.0) },
        ],
    },
    InstanceSpec {
        name: "ColorCorrection",
        class_name: "ColorCorrectionEffect",
        parent: Some("Lighting"),
        properties: &[
            PropertySpec { name: "Brightness", value: PropValue::Number(0.05) },
            PropertySpec { name: "Contrast", value: PropValue::Number(0.1) },
            PropertySpec { name: "Saturation", value: PropValue::Number(0.15) },
            PropertySpec { name: "TintColor", value: PropValue::Color(255, 255, 255) },
        ],
    },
];

/// Renders the template into the `$className`/`$properties` objects Rojo's
/// project format expects, nesting children under their declared parent.
pub fn render() -> serde_json::Map<String, Value> {
    let mut roots = serde_json::Map::new();

    for spec in PLACE_TEMPLATE.iter().filter(|s| s.parent.is_none()) {
        let mut node = render_one(spec);
        for child in PLACE_TEMPLATE.iter().filter(|s| s.parent == Some(spec.name)) {
            node.insert(child.name.to_string(), Value::Object(render_one(child)));
        }
        roots.insert(spec.name.to_string(), Value::Object(node));
    }

    roots
}

fn render_one(spec: &InstanceSpec) -> serde_json::Map<String, Value> {
    let mut node = serde_json::Map::new();
    node.insert("$className".to_string(), json!(spec.class_name));
    if !spec.properties.is_empty() {
        let mut props = serde_json::Map::new();
        for prop in spec.properties {
            props.insert(prop.name.to_string(), prop.value.to_json());
        }
        node.insert("$properties".to_string(), Value::Object(props));
    }
    node
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rojo expects Color3 components as 0-1 floats, but the catalog is
    /// authored in the 0-255 values Studio's colour picker shows.
    #[test]
    fn colors_convert_from_studio_0_255_to_rojo_0_1() {
        let rendered = render();
        let ambient = &rendered["Lighting"]["$properties"]["Ambient"];
        let parts: Vec<f64> = ambient.as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();

        assert!((parts[0] - 200.0 / 255.0).abs() < 1e-9, "got {parts:?}");
        assert!((parts[1] - 160.0 / 255.0).abs() < 1e-9, "got {parts:?}");
        assert!((parts[2] - 225.0 / 255.0).abs() < 1e-9, "got {parts:?}");
        assert!(parts.iter().all(|p| (0.0..=1.0).contains(p)), "out of range: {parts:?}");
    }

    /// Entries declaring a parent must be nested under it, not emitted as
    /// a second top-level service.
    #[test]
    fn children_nest_under_their_declared_parent() {
        let rendered = render();
        assert!(!rendered.contains_key("ColorCorrection"), "child leaked to top level");

        let effect = &rendered["Lighting"]["ColorCorrection"];
        assert_eq!(effect["$className"], "ColorCorrectionEffect");
        assert_eq!(effect["$properties"]["Contrast"], 0.1);
    }

    /// Every declared parent must exist in the table, or the child is
    /// silently dropped by `render`.
    #[test]
    fn every_declared_parent_exists() {
        for spec in PLACE_TEMPLATE {
            if let Some(parent) = spec.parent {
                assert!(
                    PLACE_TEMPLATE.iter().any(|s| s.name == parent && s.parent.is_none()),
                    "{} declares unknown parent {parent}",
                    spec.name
                );
            }
        }
    }
}
