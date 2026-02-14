use dbtoon::backend::sqlserver::{column_data_to_string, normalize_tiberius_type, parse_server_address};
use dbtoon::backend::CellValue;
use std::borrow::Cow;
use tiberius::numeric::Numeric;
use tiberius::time::{Date, DateTime, DateTime2, DateTimeOffset, SmallDateTime, Time};
use tiberius::xml::XmlData;
use tiberius::{ColumnData, ColumnType, Uuid};

// ---------------------------------------------------------------------------
// T003: parse_server_address unit tests
// Covers all 9 format variants per contracts/server-address-parsing.md
// ---------------------------------------------------------------------------

#[test]
fn test_parse_plain_hostname() {
    let (host, port, instance) = parse_server_address("myserver").unwrap();
    assert_eq!(host, "myserver");
    assert_eq!(port, None);
    assert_eq!(instance, None);
}

#[test]
fn test_parse_hostname_with_port() {
    let (host, port, instance) = parse_server_address("myserver,1433").unwrap();
    assert_eq!(host, "myserver");
    assert_eq!(port, Some(1433));
    assert_eq!(instance, None);
}

#[test]
fn test_parse_hostname_with_instance() {
    let (host, port, instance) = parse_server_address("myserver\\INSTANCE").unwrap();
    assert_eq!(host, "myserver");
    assert_eq!(port, None);
    assert_eq!(instance.as_deref(), Some("INSTANCE"));
}

#[test]
fn test_parse_hostname_with_instance_and_port() {
    let (host, port, instance) = parse_server_address("myserver\\INSTANCE,1434").unwrap();
    assert_eq!(host, "myserver");
    assert_eq!(port, Some(1434));
    assert_eq!(instance.as_deref(), Some("INSTANCE"));
}

#[test]
fn test_parse_tcp_prefix_stripping() {
    let (host, port, instance) = parse_server_address("tcp:myserver").unwrap();
    assert_eq!(host, "myserver");
    assert_eq!(port, None);
    assert_eq!(instance, None);
}

#[test]
fn test_parse_tcp_prefix_with_port() {
    let (host, port, instance) = parse_server_address("tcp:myserver,1433").unwrap();
    assert_eq!(host, "myserver");
    assert_eq!(port, Some(1433));
    assert_eq!(instance, None);
}

#[test]
fn test_parse_ipv4_address() {
    let (host, port, instance) = parse_server_address("192.168.1.1").unwrap();
    assert_eq!(host, "192.168.1.1");
    assert_eq!(port, None);
    assert_eq!(instance, None);
}

#[test]
fn test_parse_ipv4_with_port() {
    let (host, port, instance) = parse_server_address("192.168.1.1,1433").unwrap();
    assert_eq!(host, "192.168.1.1");
    assert_eq!(port, Some(1433));
    assert_eq!(instance, None);
}

#[test]
fn test_parse_ipv4_with_instance() {
    let (host, port, instance) = parse_server_address("192.168.1.1\\INSTANCE").unwrap();
    assert_eq!(host, "192.168.1.1");
    assert_eq!(port, None);
    assert_eq!(instance.as_deref(), Some("INSTANCE"));
}

#[test]
fn test_parse_invalid_port_errors() {
    let result = parse_server_address("myserver,abc");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("config"), "Expected Config error, got: {}", err);
}

#[test]
fn test_parse_port_out_of_range_errors() {
    let result = parse_server_address("myserver,99999");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("config"), "Expected Config error, got: {}", err);
}

// ---------------------------------------------------------------------------
// T004: normalize_tiberius_type unit tests
// Covers all 27+ ColumnType variants per contracts/type-normalization.md
// ---------------------------------------------------------------------------

#[test]
fn test_normalize_null() {
    assert_eq!(normalize_tiberius_type(ColumnType::Null), "UNKNOWN");
}

#[test]
fn test_normalize_bit_variants() {
    assert_eq!(normalize_tiberius_type(ColumnType::Bit), "BIT");
    assert_eq!(normalize_tiberius_type(ColumnType::Bitn), "BIT");
}

#[test]
fn test_normalize_integer_types() {
    assert_eq!(normalize_tiberius_type(ColumnType::Int1), "TINYINT");
    assert_eq!(normalize_tiberius_type(ColumnType::Int2), "SMALLINT");
    assert_eq!(normalize_tiberius_type(ColumnType::Int4), "INT");
    assert_eq!(normalize_tiberius_type(ColumnType::Intn), "INT");
    assert_eq!(normalize_tiberius_type(ColumnType::Int8), "BIGINT");
}

#[test]
fn test_normalize_float_types() {
    assert_eq!(normalize_tiberius_type(ColumnType::Float4), "REAL");
    assert_eq!(normalize_tiberius_type(ColumnType::Float8), "FLOAT");
    assert_eq!(normalize_tiberius_type(ColumnType::Floatn), "FLOAT");
}

#[test]
fn test_normalize_money_types() {
    assert_eq!(normalize_tiberius_type(ColumnType::Money), "MONEY");
    assert_eq!(normalize_tiberius_type(ColumnType::Money4), "SMALLMONEY");
}

#[test]
fn test_normalize_datetime_types() {
    assert_eq!(normalize_tiberius_type(ColumnType::Datetime), "DATETIME");
    assert_eq!(normalize_tiberius_type(ColumnType::Datetimen), "DATETIME");
    assert_eq!(normalize_tiberius_type(ColumnType::Datetime4), "SMALLDATETIME");
    assert_eq!(normalize_tiberius_type(ColumnType::Datetime2), "DATETIME2");
    assert_eq!(normalize_tiberius_type(ColumnType::Daten), "DATE");
    assert_eq!(normalize_tiberius_type(ColumnType::Timen), "TIME");
    assert_eq!(normalize_tiberius_type(ColumnType::DatetimeOffsetn), "DATETIMEOFFSET");
}

#[test]
fn test_normalize_decimal_types() {
    assert_eq!(normalize_tiberius_type(ColumnType::Decimaln), "DECIMAL");
    assert_eq!(normalize_tiberius_type(ColumnType::Numericn), "NUMERIC");
}

#[test]
fn test_normalize_string_types() {
    assert_eq!(normalize_tiberius_type(ColumnType::BigVarChar), "VARCHAR");
    assert_eq!(normalize_tiberius_type(ColumnType::BigChar), "CHAR");
    assert_eq!(normalize_tiberius_type(ColumnType::NVarchar), "NVARCHAR");
    assert_eq!(normalize_tiberius_type(ColumnType::NChar), "NCHAR");
}

#[test]
fn test_normalize_binary_types() {
    assert_eq!(normalize_tiberius_type(ColumnType::BigVarBin), "VARBINARY");
    assert_eq!(normalize_tiberius_type(ColumnType::BigBinary), "BINARY");
}

#[test]
fn test_normalize_special_types() {
    assert_eq!(normalize_tiberius_type(ColumnType::Guid), "UNIQUEIDENTIFIER");
    assert_eq!(normalize_tiberius_type(ColumnType::Xml), "XML");
    assert_eq!(normalize_tiberius_type(ColumnType::Text), "TEXT");
    assert_eq!(normalize_tiberius_type(ColumnType::NText), "NTEXT");
    assert_eq!(normalize_tiberius_type(ColumnType::Image), "IMAGE");
    assert_eq!(normalize_tiberius_type(ColumnType::SSVariant), "SQL_VARIANT");
    assert_eq!(normalize_tiberius_type(ColumnType::Udt), "UNKNOWN");
}

// ---------------------------------------------------------------------------
// T005: column_data_to_string unit tests
// Covers all 17+ ColumnData variants per contracts/type-normalization.md
// ---------------------------------------------------------------------------

// --- Integer types ---

#[test]
fn test_u8_value() {
    let data = ColumnData::U8(Some(255));
    assert_eq!(column_data_to_string(&data), CellValue::Text("255".to_string()));
}

#[test]
fn test_i16_value() {
    let data = ColumnData::I16(Some(-32768));
    assert_eq!(column_data_to_string(&data), CellValue::Text("-32768".to_string()));
}

#[test]
fn test_i32_value() {
    let data = ColumnData::I32(Some(42));
    assert_eq!(column_data_to_string(&data), CellValue::Text("42".to_string()));
}

#[test]
fn test_i64_value() {
    let data = ColumnData::I64(Some(9_223_372_036_854_775_807));
    assert_eq!(
        column_data_to_string(&data),
        CellValue::Text("9223372036854775807".to_string())
    );
}

// --- Float types ---

#[test]
fn test_f32_value() {
    let data = ColumnData::F32(Some(1.25));
    let result = column_data_to_string(&data);
    match result {
        CellValue::Text(s) => {
            let parsed: f32 = s.parse().expect("should be a valid float string");
            assert!((parsed - 1.25).abs() < 0.001, "Got: {}", s);
        }
        CellValue::Null => panic!("Expected Text, got Null"),
    }
}

#[test]
fn test_f64_value() {
    let data = ColumnData::F64(Some(9.876543210123456));
    let result = column_data_to_string(&data);
    match result {
        CellValue::Text(s) => {
            let parsed: f64 = s.parse().expect("should be a valid float string");
            assert!((parsed - 9.876543210123456).abs() < 1e-10, "Got: {}", s);
        }
        CellValue::Null => panic!("Expected Text, got Null"),
    }
}

// --- Bit type ---

#[test]
fn test_bit_true_as_1() {
    let data = ColumnData::Bit(Some(true));
    assert_eq!(column_data_to_string(&data), CellValue::Text("1".to_string()));
}

#[test]
fn test_bit_false_as_0() {
    let data = ColumnData::Bit(Some(false));
    assert_eq!(column_data_to_string(&data), CellValue::Text("0".to_string()));
}

// --- String type ---

#[test]
fn test_string_value() {
    let data = ColumnData::String(Some(Cow::Borrowed("hello world")));
    assert_eq!(
        column_data_to_string(&data),
        CellValue::Text("hello world".to_string())
    );
}

// --- Guid type ---

#[test]
fn test_guid_value() {
    let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let data = ColumnData::Guid(Some(uuid));
    assert_eq!(
        column_data_to_string(&data),
        CellValue::Text("550e8400-e29b-41d4-a716-446655440000".to_string())
    );
}

// --- Binary type ---

#[test]
fn test_binary_value() {
    let data = ColumnData::Binary(Some(Cow::Owned(vec![0x48, 0x45, 0x4C, 0x4C, 0x4F])));
    assert_eq!(
        column_data_to_string(&data),
        CellValue::Text("0x48454C4C4F".to_string())
    );
}

#[test]
fn test_binary_empty() {
    let data = ColumnData::Binary(Some(Cow::Owned(vec![])));
    assert_eq!(
        column_data_to_string(&data),
        CellValue::Text("0x".to_string())
    );
}

// --- Numeric type ---

#[test]
fn test_numeric_with_trailing_zeros() {
    // 1234500 / 10^4 = 123.4500
    let data = ColumnData::Numeric(Some(Numeric::new_with_scale(1234500, 4)));
    assert_eq!(
        column_data_to_string(&data),
        CellValue::Text("123.4500".to_string())
    );
}

#[test]
fn test_numeric_integer_with_scale() {
    // 100 / 10^2 = 1.00
    let data = ColumnData::Numeric(Some(Numeric::new_with_scale(100, 2)));
    assert_eq!(
        column_data_to_string(&data),
        CellValue::Text("1.00".to_string())
    );
}

#[test]
fn test_numeric_zero_scale() {
    // 42 / 10^0 = 42
    let data = ColumnData::Numeric(Some(Numeric::new_with_scale(42, 0)));
    assert_eq!(
        column_data_to_string(&data),
        CellValue::Text("42".to_string())
    );
}

// --- DateTime type (YYYY-MM-DD HH:MM:SS.mmm) ---

#[test]
fn test_datetime_epoch() {
    // days=0, seconds_fragments=0 → 1900-01-01 00:00:00.000
    let data = ColumnData::DateTime(Some(DateTime::new(0, 0)));
    assert_eq!(
        column_data_to_string(&data),
        CellValue::Text("1900-01-01 00:00:00.000".to_string())
    );
}

#[test]
fn test_datetime_with_time() {
    // days=45304 → 2024-01-15
    // seconds_fragments = (14*3600 + 30*60) * 300 = 52200 * 300 = 15660000 → 14:30:00.000
    let data = ColumnData::DateTime(Some(DateTime::new(45304, 15_660_000)));
    assert_eq!(
        column_data_to_string(&data),
        CellValue::Text("2024-01-15 14:30:00.000".to_string())
    );
}

// --- SmallDateTime type (YYYY-MM-DD HH:MM:SS) ---

#[test]
fn test_smalldatetime_epoch() {
    // days=0, minutes=0 → 1900-01-01 00:00:00
    let data = ColumnData::SmallDateTime(Some(SmallDateTime::new(0, 0)));
    assert_eq!(
        column_data_to_string(&data),
        CellValue::Text("1900-01-01 00:00:00".to_string())
    );
}

#[test]
fn test_smalldatetime_with_time() {
    // days=45304 → 2024-01-15
    // minutes = 14*60 + 30 = 870 → 14:30:00
    let data = ColumnData::SmallDateTime(Some(SmallDateTime::new(45304, 870)));
    assert_eq!(
        column_data_to_string(&data),
        CellValue::Text("2024-01-15 14:30:00".to_string())
    );
}

// --- Date type (YYYY-MM-DD) ---

#[test]
fn test_date_value() {
    // days since 0001-01-01
    // 738899 → 2024-01-15
    let data = ColumnData::Date(Some(Date::new(738_899)));
    assert_eq!(
        column_data_to_string(&data),
        CellValue::Text("2024-01-15".to_string())
    );
}

// --- Time type (HH:MM:SS.nnnnnnn) ---

#[test]
fn test_time_value() {
    // increments in 10^(-scale) second units from midnight
    // For 14:30:00.1234567 at scale 7 (100-nanosecond units):
    // (14*3600 + 30*60) * 10_000_000 + 1_234_567 = 522_001_234_567
    let data = ColumnData::Time(Some(Time::new(522_001_234_567, 7)));
    assert_eq!(
        column_data_to_string(&data),
        CellValue::Text("14:30:00.1234567".to_string())
    );
}

#[test]
fn test_time_zero() {
    // midnight at scale 7
    let data = ColumnData::Time(Some(Time::new(0, 7)));
    assert_eq!(
        column_data_to_string(&data),
        CellValue::Text("00:00:00.0000000".to_string())
    );
}

// --- DateTime2 type (YYYY-MM-DD HH:MM:SS.nnnnnnn) ---

#[test]
fn test_datetime2_value() {
    // 2024-01-15 14:30:00.0000000
    let date = Date::new(738_899);
    let time = Time::new(522_000_000_000, 7);
    let data = ColumnData::DateTime2(Some(DateTime2::new(date, time)));
    assert_eq!(
        column_data_to_string(&data),
        CellValue::Text("2024-01-15 14:30:00.0000000".to_string())
    );
}

// --- DateTimeOffset type (YYYY-MM-DD HH:MM:SS.nnnnnnn +HH:MM) ---

#[test]
fn test_datetimeoffset_value() {
    // 2024-01-15 14:30:00.0000000 +05:30
    let date = Date::new(738_899);
    let time = Time::new(522_000_000_000, 7);
    let dt2 = DateTime2::new(date, time);
    let data = ColumnData::DateTimeOffset(Some(DateTimeOffset::new(dt2, 330)));
    assert_eq!(
        column_data_to_string(&data),
        CellValue::Text("2024-01-15 14:30:00.0000000 +05:30".to_string())
    );
}

#[test]
fn test_datetimeoffset_negative_offset() {
    // 2024-01-15 14:30:00.0000000 -05:00
    let date = Date::new(738_899);
    let time = Time::new(522_000_000_000, 7);
    let dt2 = DateTime2::new(date, time);
    let data = ColumnData::DateTimeOffset(Some(DateTimeOffset::new(dt2, -300)));
    assert_eq!(
        column_data_to_string(&data),
        CellValue::Text("2024-01-15 14:30:00.0000000 -05:00".to_string())
    );
}

// --- Xml type ---

#[test]
fn test_xml_value() {
    let xml = XmlData::new("<root/>");
    let data = ColumnData::Xml(Some(Cow::Owned(xml)));
    assert_eq!(
        column_data_to_string(&data),
        CellValue::Text("<root/>".to_string())
    );
}

// --- Null variants ---

#[test]
fn test_null_u8() {
    let data = ColumnData::U8(None);
    assert_eq!(column_data_to_string(&data), CellValue::Null);
}

#[test]
fn test_null_i16() {
    let data = ColumnData::I16(None);
    assert_eq!(column_data_to_string(&data), CellValue::Null);
}

#[test]
fn test_null_i32() {
    let data = ColumnData::I32(None);
    assert_eq!(column_data_to_string(&data), CellValue::Null);
}

#[test]
fn test_null_i64() {
    let data = ColumnData::I64(None);
    assert_eq!(column_data_to_string(&data), CellValue::Null);
}

#[test]
fn test_null_f32() {
    let data = ColumnData::F32(None);
    assert_eq!(column_data_to_string(&data), CellValue::Null);
}

#[test]
fn test_null_f64() {
    let data = ColumnData::F64(None);
    assert_eq!(column_data_to_string(&data), CellValue::Null);
}

#[test]
fn test_null_bit() {
    let data = ColumnData::Bit(None);
    assert_eq!(column_data_to_string(&data), CellValue::Null);
}

#[test]
fn test_null_string() {
    let data: ColumnData<'_> = ColumnData::String(None);
    assert_eq!(column_data_to_string(&data), CellValue::Null);
}

#[test]
fn test_null_guid() {
    let data = ColumnData::Guid(None);
    assert_eq!(column_data_to_string(&data), CellValue::Null);
}

#[test]
fn test_null_binary() {
    let data: ColumnData<'_> = ColumnData::Binary(None);
    assert_eq!(column_data_to_string(&data), CellValue::Null);
}

#[test]
fn test_null_numeric() {
    let data = ColumnData::Numeric(None);
    assert_eq!(column_data_to_string(&data), CellValue::Null);
}

#[test]
fn test_null_datetime() {
    let data = ColumnData::DateTime(None);
    assert_eq!(column_data_to_string(&data), CellValue::Null);
}

#[test]
fn test_null_smalldatetime() {
    let data = ColumnData::SmallDateTime(None);
    assert_eq!(column_data_to_string(&data), CellValue::Null);
}

#[test]
fn test_null_date() {
    let data = ColumnData::Date(None);
    assert_eq!(column_data_to_string(&data), CellValue::Null);
}

#[test]
fn test_null_time() {
    let data = ColumnData::Time(None);
    assert_eq!(column_data_to_string(&data), CellValue::Null);
}

#[test]
fn test_null_datetime2() {
    let data = ColumnData::DateTime2(None);
    assert_eq!(column_data_to_string(&data), CellValue::Null);
}

#[test]
fn test_null_datetimeoffset() {
    let data = ColumnData::DateTimeOffset(None);
    assert_eq!(column_data_to_string(&data), CellValue::Null);
}

#[test]
fn test_null_xml() {
    let data: ColumnData<'_> = ColumnData::Xml(None);
    assert_eq!(column_data_to_string(&data), CellValue::Null);
}
