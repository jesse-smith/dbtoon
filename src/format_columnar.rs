use std::sync::Arc;

use arrow::array::{
    ArrayRef, BinaryArray, BooleanArray, Date32Array, Float32Array, Float64Array, Int16Array,
    Int32Array, Int64Array, StringArray, Time64MicrosecondArray, TimestampMicrosecondArray,
    UInt8Array,
};
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use arrow::record_batch::RecordBatch;

use crate::backend::{CellValue, QueryResult};
use crate::error::DbtoonError;

/// Map a SQL type string to an Arrow DataType.
/// Unknown types map to Utf8 (string fallback).
pub fn sql_type_to_arrow(type_name: &str) -> DataType {
    let normalized = type_name.trim().to_uppercase();

    // Extract base type (before parenthesis) for parameterized types
    let base = match normalized.find('(') {
        Some(pos) => &normalized[..pos],
        None => &normalized,
    };

    match base {
        // Integer types
        "INT" | "INTEGER" => DataType::Int32,
        "SMALLINT" => DataType::Int16,
        "BIGINT" => DataType::Int64,
        "TINYINT" => DataType::UInt8,

        // Boolean
        "BIT" | "BOOLEAN" => DataType::Boolean,

        // Floating point
        "REAL" => DataType::Float32,
        "FLOAT" => DataType::Float64,

        // String types
        "VARCHAR" | "NVARCHAR" | "CHAR" | "NCHAR" | "STRING" => DataType::Utf8,

        // Decimal/Numeric — parse precision and scale from parenthesized params
        "DECIMAL" | "NUMERIC" => parse_decimal_params(&normalized),

        // Date/Time types
        "DATE" => DataType::Date32,
        "DATETIME2" | "TIMESTAMP" => DataType::Timestamp(TimeUnit::Microsecond, None),
        "TIME" => DataType::Time64(TimeUnit::Microsecond),

        // Binary types
        "BINARY" | "VARBINARY" => DataType::Binary,

        // Unknown → string fallback (FR-009)
        _ => DataType::Utf8,
    }
}

/// Parse DECIMAL(p,s) or NUMERIC(p,s) parameters.
/// Falls back to Decimal128(38, 10) if parsing fails.
fn parse_decimal_params(normalized: &str) -> DataType {
    if let (Some(start), Some(end)) = (normalized.find('('), normalized.find(')')) {
        let inner = &normalized[start + 1..end];
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 2
            && let (Ok(p), Ok(s)) = (
                parts[0].trim().parse::<u8>(),
                parts[1].trim().parse::<i8>(),
            )
        {
            return DataType::Decimal128(p, s);
        }
    }
    DataType::Decimal128(38, 10)
}

/// Build an Arrow RecordBatch from a QueryResult.
/// Each column is converted to a typed Arrow array based on its type_name.
/// If value parsing fails for a column, that column falls back to Utf8.
pub fn build_record_batch(
    result: &QueryResult,
) -> Result<(Arc<Schema>, RecordBatch), DbtoonError> {
    let num_rows = result.rows.len();
    let num_cols = result.columns.len();

    // Build schema and arrays
    let mut fields = Vec::with_capacity(num_cols);
    let mut arrays: Vec<ArrayRef> = Vec::with_capacity(num_cols);

    for (col_idx, col_meta) in result.columns.iter().enumerate() {
        let target_type = sql_type_to_arrow(&col_meta.type_name);

        // Collect column values
        let col_values: Vec<Option<&str>> = (0..num_rows)
            .map(|row_idx| match &result.rows[row_idx][col_idx] {
                CellValue::Text(s) => Some(s.as_str()),
                CellValue::Null => None,
            })
            .collect();

        // Try to build a typed array; on failure, fall back to StringArray
        let (final_type, array) =
            build_typed_array(&target_type, &col_values).unwrap_or_else(|| {
                let fallback: ArrayRef =
                    Arc::new(StringArray::from(col_values.to_vec()));
                (DataType::Utf8, fallback)
            });

        fields.push(Field::new(&col_meta.name, final_type, true));
        arrays.push(array);
    }

    let schema = Arc::new(Schema::new(fields));
    let batch = RecordBatch::try_new(schema.clone(), arrays).map_err(|e| DbtoonError::Format {
        message: format!("failed to build record batch: {e}"),
    })?;

    Ok((schema, batch))
}

/// Attempt to build a typed Arrow array from string values.
/// Returns None if any non-null value fails to parse (triggering column-level fallback).
fn build_typed_array(
    target_type: &DataType,
    values: &[Option<&str>],
) -> Option<(DataType, ArrayRef)> {
    let array: ArrayRef = match target_type {
        DataType::Int16 => {
            let parsed: Option<Vec<Option<i16>>> = values
                .iter()
                .map(|v| match v {
                    None => Some(None),
                    Some(s) => s.parse::<i16>().ok().map(Some),
                })
                .collect();
            Arc::new(Int16Array::from(parsed?))
        }
        DataType::Int32 => {
            let parsed: Option<Vec<Option<i32>>> = values
                .iter()
                .map(|v| match v {
                    None => Some(None),
                    Some(s) => s.parse::<i32>().ok().map(Some),
                })
                .collect();
            Arc::new(Int32Array::from(parsed?))
        }
        DataType::Int64 => {
            let parsed: Option<Vec<Option<i64>>> = values
                .iter()
                .map(|v| match v {
                    None => Some(None),
                    Some(s) => s.parse::<i64>().ok().map(Some),
                })
                .collect();
            Arc::new(Int64Array::from(parsed?))
        }
        DataType::UInt8 => {
            let parsed: Option<Vec<Option<u8>>> = values
                .iter()
                .map(|v| match v {
                    None => Some(None),
                    Some(s) => s.parse::<u8>().ok().map(Some),
                })
                .collect();
            Arc::new(UInt8Array::from(parsed?))
        }
        DataType::Boolean => {
            let parsed: Option<Vec<Option<bool>>> = values
                .iter()
                .map(|v| match v {
                    None => Some(None),
                    Some(s) => parse_bool(s).map(Some),
                })
                .collect();
            Arc::new(BooleanArray::from(parsed?))
        }
        DataType::Float32 => {
            let parsed: Option<Vec<Option<f32>>> = values
                .iter()
                .map(|v| match v {
                    None => Some(None),
                    Some(s) => s.parse::<f32>().ok().map(Some),
                })
                .collect();
            Arc::new(Float32Array::from(parsed?))
        }
        DataType::Float64 => {
            let parsed: Option<Vec<Option<f64>>> = values
                .iter()
                .map(|v| match v {
                    None => Some(None),
                    Some(s) => s.parse::<f64>().ok().map(Some),
                })
                .collect();
            Arc::new(Float64Array::from(parsed?))
        }
        DataType::Utf8 => {
            Arc::new(StringArray::from(values.to_vec()))
        }
        DataType::Date32 => {
            let parsed: Option<Vec<Option<i32>>> = values
                .iter()
                .map(|v| match v {
                    None => Some(None),
                    Some(s) => parse_date32(s).map(Some),
                })
                .collect();
            Arc::new(Date32Array::from(parsed?))
        }
        DataType::Timestamp(TimeUnit::Microsecond, None) => {
            let parsed: Option<Vec<Option<i64>>> = values
                .iter()
                .map(|v| match v {
                    None => Some(None),
                    Some(s) => parse_timestamp_micros(s).map(Some),
                })
                .collect();
            Arc::new(TimestampMicrosecondArray::from(parsed?))
        }
        DataType::Time64(TimeUnit::Microsecond) => {
            let parsed: Option<Vec<Option<i64>>> = values
                .iter()
                .map(|v| match v {
                    None => Some(None),
                    Some(s) => parse_time_micros(s).map(Some),
                })
                .collect();
            Arc::new(Time64MicrosecondArray::from(parsed?))
        }
        DataType::Binary => {
            let parsed: Option<Vec<Option<Vec<u8>>>> = values
                .iter()
                .map(|v| match v {
                    None => Some(None),
                    Some(s) => hex_decode(s).map(Some),
                })
                .collect();
            let parsed = parsed?;
            let refs: Vec<Option<&[u8]>> = parsed.iter().map(|v| v.as_deref()).collect();
            Arc::new(BinaryArray::from(refs))
        }
        DataType::Decimal128(p, s) => {
            let scale = *s;
            let parsed: Option<Vec<Option<i128>>> = values
                .iter()
                .map(|v| match v {
                    None => Some(None),
                    Some(sv) => parse_decimal_i128(sv, scale).map(Some),
                })
                .collect();
            let arr = arrow::array::Decimal128Array::from(parsed?)
                .with_precision_and_scale(*p, scale)
                .map_err(|_| ())
                .ok()?;
            Arc::new(arr)
        }
        // Any other type → treat as string (shouldn't normally reach here since
        // sql_type_to_arrow already maps unknowns to Utf8)
        _ => {
            Arc::new(StringArray::from(values.to_vec()))
        }
    };

    Some((target_type.clone(), array))
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.trim().to_lowercase().as_str() {
        "1" | "true" => Some(true),
        "0" | "false" => Some(false),
        _ => None,
    }
}

fn parse_date32(s: &str) -> Option<i32> {
    // Parse YYYY-MM-DD to days since Unix epoch
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y: i32 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    let d: u32 = parts[2].parse().ok()?;

    // Days from civil date to Unix epoch using a simplified algorithm
    days_from_civil(y, m, d)
}

fn days_from_civil(y: i32, m: u32, d: u32) -> Option<i32> {
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400) as u32;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe as i32 - 719468;
    Some(days)
}

fn parse_timestamp_micros(s: &str) -> Option<i64> {
    // Parse ISO 8601: YYYY-MM-DDThh:mm:ss[.ffffff] or YYYY-MM-DD hh:mm:ss[.ffffff]
    let s = s.trim();
    let (date_part, time_part) = if let Some(pos) = s.find('T') {
        (&s[..pos], &s[pos + 1..])
    } else if let Some(pos) = s.find(' ') {
        (&s[..pos], &s[pos + 1..])
    } else {
        return None;
    };

    let days = parse_date32(date_part)?;
    let micros = parse_time_micros(time_part)?;

    Some(days as i64 * 86_400_000_000 + micros)
}

fn parse_time_micros(s: &str) -> Option<i64> {
    // Parse HH:MM:SS[.ffffff]
    let parts: Vec<&str> = s.splitn(2, '.').collect();
    let hms: Vec<&str> = parts[0].split(':').collect();
    if hms.len() != 3 {
        return None;
    }
    let h: i64 = hms[0].parse().ok()?;
    let m: i64 = hms[1].parse().ok()?;
    let sec: i64 = hms[2].parse().ok()?;

    let mut micros = (h * 3600 + m * 60 + sec) * 1_000_000;

    if parts.len() == 2 {
        let frac = parts[1];
        // Pad or truncate to 6 digits
        let padded = format!("{:0<6}", &frac[..frac.len().min(6)]);
        let frac_micros: i64 = padded.parse().ok()?;
        micros += frac_micros;
    }

    Some(micros)
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")).unwrap_or(s);
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

fn parse_decimal_i128(s: &str, scale: i8) -> Option<i128> {
    // Parse a decimal string like "123.45" into a scaled i128
    let s = s.trim();
    let negative = s.starts_with('-');
    let s = s.strip_prefix('-').unwrap_or(s).strip_prefix('+').unwrap_or(s);

    let (integer_part, frac_part) = match s.find('.') {
        Some(pos) => (&s[..pos], &s[pos + 1..]),
        None => (s, ""),
    };

    let scale = scale as usize;

    // Pad or truncate fractional part to match scale
    let frac_str = if frac_part.len() >= scale {
        &frac_part[..scale]
    } else {
        // Need to pad — build a new string
        &format!("{:0<width$}", frac_part, width = scale)
    };

    let combined = if scale > 0 {
        format!("{integer_part}{frac_str}")
    } else {
        integer_part.to_string()
    };

    let mut val: i128 = combined.parse().ok()?;
    if negative {
        val = -val;
    }
    Some(val)
}
