use crate::backend::{CellValue, QueryResult};
use crate::error::DbtoonError;

/// Convert a QueryResult to a TOON-formatted string.
pub fn to_toon(result: &QueryResult) -> Result<String, DbtoonError> {
    // Handle zero-row results with column names — toon_format can't infer columns
    // from an empty array, so we produce the TOON header manually.
    if result.rows.is_empty() && !result.columns.is_empty() {
        let col_names = result
            .columns
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>()
            .join(",");
        return Ok(format!("[0]{{{}}}:\n", col_names));
    }

    let array: Vec<serde_json::Value> = result
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

    let json_array = serde_json::Value::Array(array);

    toon_format::encode_default(&json_array)
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
