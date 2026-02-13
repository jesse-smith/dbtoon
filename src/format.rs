use crate::backend::{CellValue, QueryResult};
use crate::error::DbtoonError;

/// Convert a QueryResult to a TOON-formatted string with truncation metadata.
///
/// Output is a root object: `{ "types": [...], "rows": [...], "truncated": bool, "message"?: str }`.
/// The `"truncated"` key is always present. The `"message"` key is present only when truncated.
pub fn to_toon(
    result: &QueryResult,
    truncated: bool,
    message: Option<&str>,
) -> Result<String, DbtoonError> {
    let types: Vec<serde_json::Value> = result
        .columns
        .iter()
        .map(|c| serde_json::Value::String(c.type_name.clone()))
        .collect();

    let rows: Vec<serde_json::Value> = result
        .rows
        .iter()
        .map(|row| {
            let mut map = serde_json::Map::new();
            for (i, col) in result.columns.iter().enumerate() {
                let value = row.get(i).unwrap_or(&CellValue::Null);
                let json_val = match value {
                    CellValue::Text(s) => serde_json::Value::String(s.clone()),
                    CellValue::Null => serde_json::Value::Null,
                };
                map.insert(col.name.clone(), json_val);
            }
            serde_json::Value::Object(map)
        })
        .collect();

    let mut root = serde_json::Map::new();
    root.insert("types".to_string(), serde_json::Value::Array(types));
    root.insert("rows".to_string(), serde_json::Value::Array(rows));
    root.insert("truncated".to_string(), serde_json::Value::Bool(truncated));
    if let Some(msg) = message {
        root.insert("message".to_string(), serde_json::Value::String(msg.to_string()));
    }

    toon_format::encode_default(&serde_json::Value::Object(root))
        .map_err(|e| DbtoonError::Format { message: e.to_string() })
}

