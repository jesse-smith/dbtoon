# Contract: Type Normalization

**Status**: Interface changes — replaces `normalize_odbc_type` with DMV-based approach + fallback.

## Current Contract (ODBC)

```rust
pub fn normalize_odbc_type(data_type: &DataType) -> String
```

Maps ODBC `DataType` enum (with embedded precision/scale/length) to SQL type strings. Produces 23+ distinct type names.

## New Contract (tiberius)

### Primary: DMV-Based Type Description

```rust
async fn describe_result_columns(
    client: &mut Client<Compat<TcpStream>>,
    sql: &str,
) -> Result<Vec<ColumnMeta>, DbtoonError>
```

Executes `sys.dm_exec_describe_first_result_set(@P1, NULL, 0)` as a parameterized query. Returns `ColumnMeta` with `type_name` sourced from the DMV's `system_type_name` column (uppercased).

**Preconditions**:
- `client` is a connected, authenticated tiberius `Client`
- `sql` is the user's query (not yet executed)

**Postconditions**:
- Returns one `ColumnMeta` per result column, ordered by ordinal position
- `type_name` matches the format produced by `normalize_odbc_type` (e.g., `NVARCHAR(255)`, `DECIMAL(18,2)`, `INT`)

**Error conditions**:
- DMV query fails (insufficient permissions, unsupported query) → returns error, caller falls back to `normalize_tiberius_type`

### Fallback: ColumnType-Based Mapping

```rust
fn normalize_tiberius_type(col_type: ColumnType) -> String
```

Best-effort mapping when DMV is unavailable. Omits precision/scale/length for variable types.

**Output mapping** (27 variants):

| ColumnType | Output |
|------------|--------|
| `Null` | `UNKNOWN` |
| `Bit`, `Bitn` | `BIT` |
| `Int1` | `TINYINT` |
| `Int2` | `SMALLINT` |
| `Int4`, `Intn` (4-byte context) | `INT` |
| `Int8` | `BIGINT` |
| `Float4` | `REAL` |
| `Float8`, `Floatn` | `FLOAT` |
| `Money` | `MONEY` |
| `Money4` | `SMALLMONEY` |
| `Datetime`, `Datetimen` | `DATETIME` |
| `Datetime4` | `SMALLDATETIME` |
| `Datetime2` | `DATETIME2` |
| `Daten` | `DATE` |
| `Timen` | `TIME` |
| `DatetimeOffsetn` | `DATETIMEOFFSET` |
| `Decimaln` | `DECIMAL` |
| `Numericn` | `NUMERIC` |
| `BigVarChar` | `VARCHAR` |
| `BigChar` | `CHAR` |
| `NVarchar` | `NVARCHAR` |
| `NChar` | `NCHAR` |
| `BigVarBin` | `VARBINARY` |
| `BigBinary` | `BINARY` |
| `Guid` | `UNIQUEIDENTIFIER` |
| `Xml` | `XML` |
| `Text` | `TEXT` |
| `NText` | `NTEXT` |
| `Image` | `IMAGE` |
| `SSVariant` | `SQL_VARIANT` |
| `Udt` | `UNKNOWN` |

**Note**: `Intn` and `Floatn` are TDS wire types that represent variable-width integers/floats. Without internal TypeInfo, we map them to the most common variant (`INT` / `FLOAT`). The DMV path avoids this ambiguity.

**UDT handling**: SQL Server UDT types (e.g., `geography`, `hierarchyid`) arrive as `Udt` in the `ColumnType` enum and as opaque binary in `ColumnData`. The type name maps to `UNKNOWN` (DMV path will provide the actual type name). The value is rendered as hex with `0x` prefix (same as `Binary`), consistent with the current ODBC behavior for unmapped types.

## Value-to-String Contract

```rust
fn column_data_to_string(data: &ColumnData<'_>) -> CellValue
```

| ColumnData variant | String format | Example |
|-------------------|---------------|---------|
| `U8(Some(v))` | decimal integer | `255` |
| `I16(Some(v))` | decimal integer | `-32768` |
| `I32(Some(v))` | decimal integer | `42` |
| `I64(Some(v))` | decimal integer | `9223372036854775807` |
| `F32(Some(v))` | float with sufficient precision | `3.14` |
| `F64(Some(v))` | float with sufficient precision | `3.141592653589793` |
| `Bit(Some(v))` | `0` or `1` | `1` |
| `String(Some(s))` | as-is | `hello world` |
| `Guid(Some(g))` | hyphenated UUID | `550e8400-e29b-41d4-a716-446655440000` |
| `Binary(Some(b))` | hex string with `0x` prefix | `0x48454C4C4F` |
| `Numeric(Some(n))` | scaled decimal preserving trailing zeros | `123.4500` |
| `DateTime(Some(dt))` | `YYYY-MM-DD HH:MM:SS.mmm` | `2024-01-15 14:30:00.000` |
| `SmallDateTime(Some(dt))` | `YYYY-MM-DD HH:MM:SS` | `2024-01-15 14:30:00` |
| `Date(Some(d))` | `YYYY-MM-DD` | `2024-01-15` |
| `Time(Some(t))` | `HH:MM:SS.nnnnnnn` (scale-dependent) | `14:30:00.1234567` |
| `DateTime2(Some(dt))` | `YYYY-MM-DD HH:MM:SS.nnnnnnn` | `2024-01-15 14:30:00.0000000` |
| `DateTimeOffset(Some(dto))` | `YYYY-MM-DD HH:MM:SS.nnnnnnn +HH:MM` | `2024-01-15 14:30:00.0000000 +05:30` |
| `Xml(Some(x))` | XML string as-is | `<root/>` |
| Any `None` variant | N/A | → `CellValue::Null` |
