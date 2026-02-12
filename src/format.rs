use crate::backend::{CellValue, QueryResult};
use crate::error::DbtoonError;

/// Convert a QueryResult to a TOON-formatted string.
///
/// Output is a root object: `{ "types": [...], "rows": [...] }`.
pub fn to_toon(result: &QueryResult) -> Result<String, DbtoonError> {
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

    toon_format::encode_default(&serde_json::Value::Object(root))
        .map_err(|e| DbtoonError::Format { message: e.to_string() })
}

/// Convert key-value pairs to TOON format (for truncation metadata and file output summary).
pub fn to_toon_kv(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{}: {}", k, v))
        .collect::<Vec<_>>()
        .join("\n")
}
