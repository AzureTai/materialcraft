use materialcraft_core::Value;
use serde_json::{Value as Json, json};

pub(super) fn typed(local: &Value, output: &Json) -> Json {
    let (kind, values): (&str, Vec<Json>) = match local {
        Value::Float(v) => ("float", vec![json!(v)]),
        Value::Float2(v) => ("float2", v.iter().map(|v| json!(v)).collect()),
        Value::Float3(v) => ("float3", v.iter().map(|v| json!(v)).collect()),
        Value::Float4(v) => ("float4", v.iter().map(|v| json!(v)).collect()),
        Value::Int(v) => ("int", vec![json!(v)]),
        Value::Int2(v) => ("int2", v.iter().map(|v| json!(v)).collect()),
        Value::Int3(v) => ("int3", v.iter().map(|v| json!(v)).collect()),
        Value::Int4(v) => ("int4", v.iter().map(|v| json!(v)).collect()),
        Value::Bool(v) => ("bool", vec![json!(v)]),
        Value::Text(v) => ("string", vec![json!(v)]),
        _ => return json!({"status": "unsupported_local_value"}),
    };
    let reference = output.get("value").map(|value| value.as_array().cloned().unwrap_or_else(|| vec![value.clone()])).unwrap_or_default();
    let type_match = output["type"].as_str() == Some(kind);
    let mut mismatches = values.len().abs_diff(reference.len());
    let mut max_error = 0.0_f64;
    let mut unavailable_numeric_components = 0;
    for (a, b) in values.iter().zip(&reference) {
        if kind.starts_with("float") {
            match (a.as_f64(), b.as_f64()) {
                (Some(a), Some(b)) => {
                    let (a, b) = (a as f32, b as f32);

                    // JSON value reports do not preserve the sign of zero
                    if a != b {
                        mismatches += 1;
                    }
                    max_error = max_error.max((f64::from(a) - f64::from(b)).abs());
                }
                _ => {
                    unavailable_numeric_components += 1;
                }
            }
        } else {
            if a != b {
                mismatches += 1;
            }
            if let (Some(a), Some(b)) = (a.as_i64(), b.as_i64()) {
                max_error = max_error.max((a as f64 - b as f64).abs());
            }
        }
    }
    let numeric = kind.starts_with("float") || kind.starts_with("int");
    let max_error = if numeric && unavailable_numeric_components == 0 && values.len() == reference.len() { json!(max_error) } else { Json::Null };

    let status = if unavailable_numeric_components != 0 {
        "unmeasurable_value"
    } else if mismatches == 0 && type_match {
        "exact_value"
    } else {
        "value_mismatch"
    };

    json!({"status": status, "type_match": type_match, "samples": values.len(),
        "mismatches": if unavailable_numeric_components == 0 { json!(mismatches) } else { Json::Null },
        "known_mismatches": mismatches, "max_error": max_error,
        "unavailable_numeric_components": unavailable_numeric_components})
}
