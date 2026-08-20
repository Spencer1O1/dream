use serde_json::{json, Map, Value};

pub fn object_params(fields: &[(&str, Value)], required: &[&str]) -> Value {
    let mut properties = Map::new();
    for (name, schema) in fields {
        properties.insert((*name).to_string(), schema.clone());
    }
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}

pub fn enum_arg(description: &str, values: &[&str]) -> Value {
    json!({
        "type": "string",
        "description": description,
        "enum": values
    })
}

pub fn string_arg(description: &str) -> Value {
    json!({
        "type": "string",
        "description": description
    })
}

pub fn object_array_arg(description: &str, fields: &[(&str, Value)], required: &[&str]) -> Value {
    json!({
        "type": "array",
        "description": description,
        "items": object_params(fields, required)
    })
}

pub fn arg_str<'a>(args: &'a Value, name: &str) -> &'a str {
    args[name].as_str().unwrap_or("")
}
