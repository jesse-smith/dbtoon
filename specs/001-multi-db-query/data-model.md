# Data Model: Multi-Database Query CLI

**Feature Branch**: `001-multi-db-query` | **Date**: 2026-02-10

This document describes the internal data model for the dbtoon CLI. These are not persisted entities — they are in-memory types that flow through the execution pipeline.

---

## Entities

### 1. BackendConfig

Represents a configured database connection target. Polymorphic over backend type.

```
BackendConfig
├── SqlServer
│   ├── driver: String           -- ODBC driver name (e.g., "ODBC Driver 18 for SQL Server")
│   ├── server: String           -- hostname or hostname\instance
│   ├── database: Option<String> -- target database
│   ├── auth: SqlServerAuth      -- authentication method (see below)
│   └── encrypt: Option<bool>    -- TLS encryption preference
│
└── Databricks
    ├── host: String             -- workspace URL (e.g., "adb-123.azuredatabricks.net")
    ├── token: SecretString      -- bearer token (masked by default)
    ├── warehouse_id: String     -- SQL warehouse identifier
    ├── catalog: Option<String>  -- Unity Catalog catalog
    └── schema: Option<String>   -- default schema
```

### 2. SqlServerAuth

Authentication method for SQL Server connections.

```
SqlServerAuth
├── WindowsIntegrated            -- Trusted_Connection=yes
└── SqlLogin
    ├── username: String
    └── password: SecretString   -- masked by default
```

### 3. AppConfig

Top-level application configuration, assembled from config file + env vars + CLI flags.

```
AppConfig
├── backend: BackendConfig       -- resolved connection target
├── allow_write: bool            -- write access opt-in (default: false)
├── default_row_limit: usize     -- default: 500
├── query_timeout_secs: u64      -- default: 60
├── verbose: bool                -- diagnostic output to stderr
├── show_secrets: bool           -- disable credential masking
└── output_file: Option<PathBuf> -- file output path (None = stdout)
```

**Precedence**: CLI flags > environment variables > config file > defaults.

### 4. Query

A SQL statement submitted for execution, with its execution mode.

```
Query
├── text: String                 -- raw SQL text
└── mode: ExecutionMode          -- read or write
```

### 5. ExecutionMode

```
ExecutionMode
├── Read                         -- validates query before execution
└── Write                        -- executes without validation (requires allow_write)
```

### 6. ValidationResult

Outcome of read-only query analysis.

```
ValidationResult
├── Safe                         -- all statements passed validation
└── Denied
    └── reasons: Vec<DenialReason>
```

### 7. DenialReason

Explains why a specific statement was denied in read-only mode.

```
DenialReason
├── statement_index: usize       -- 0-based position in multi-statement batch
├── kind: DenialKind             -- category of denial
└── detail: String               -- human-readable explanation
```

### 8. DenialKind

```
DenialKind
├── WriteStatement               -- INSERT, UPDATE, DELETE, DDL, etc.
├── SelectInto                   -- SELECT INTO detected
├── CteWrappedWrite              -- WITH ... INSERT/UPDATE/DELETE/MERGE
├── StoredProcedure              -- EXEC/EXECUTE
├── ParseFailure                 -- could not parse SQL (fail-safe)
└── Unrecognized                 -- unknown statement type (catch-all deny)
```

### 9. QueryResult

The output of executing a query, before TOON serialization.

```
QueryResult
├── columns: Vec<ColumnMeta>     -- column metadata
├── rows: Vec<Vec<CellValue>>    -- row data
├── total_rows: Option<usize>    -- total available (if known)
└── truncated: bool              -- whether row limit was applied
```

### 10. ColumnMeta

Metadata for a single result column.

```
ColumnMeta
├── name: String                 -- column name/alias
└── type_name: String            -- source type name (e.g., "INT", "VARCHAR(255)", "BIGINT")
```

### 11. CellValue

A single cell value from a query result. All values are represented as their string form for TOON serialization.

```
CellValue
├── Text(String)                 -- string representation of the value
└── Null                         -- SQL NULL
```

**Design note**: We represent all values as strings rather than preserving native types. This is intentional: TOON format is text-based, the Databricks API returns all values as strings, and for SQL Server the columnar buffer values are converted to strings during fetch. A typed intermediate representation adds complexity without benefit.

### 12. WarehouseInfo

Databricks SQL warehouse metadata (for discovery subcommand).

```
WarehouseInfo
├── id: String                   -- warehouse identifier
├── name: String                 -- display name
├── state: String                -- RUNNING, STOPPED, STARTING, etc.
├── cluster_size: String         -- e.g., "Small", "Medium"
└── warehouse_type: Option<String> -- CLASSIC, PRO, etc.
```

---

## Entity Relationships

```
AppConfig ──contains──> BackendConfig ──contains──> SqlServerAuth (if SQL Server)

User invokes CLI
  │
  ├── exec_read  → Query(mode=Read)  → ValidationResult → (if Safe) → QueryResult → TOON output
  ├── exec_write → Query(mode=Write) → (if allow_write)  → QueryResult → TOON output
  └── list-warehouses → Vec<WarehouseInfo> → TOON output
```

---

## State Transitions

### Query Execution Pipeline

```
Input(SQL text + mode)
  │
  ├─[Read mode]──> Parse ──> Validate ──> Execute ──> Format ──> Output
  │                  │           │
  │                  │ fail      │ denied
  │                  v           v
  │                Reject     Reject
  │
  └─[Write mode]─> Check allow_write ──> Execute ──> Format ──> Output
                        │
                        │ denied
                        v
                      Reject
```

### Databricks Async Execution States

```
PENDING → RUNNING → SUCCEEDED → (results fetched) → CLOSED
                  → FAILED
         → CANCELED (by timeout or user)
```

---

## Validation Rules

| Field | Rule |
|-------|------|
| `BackendConfig.SqlServer.server` | Non-empty string, required |
| `BackendConfig.Databricks.host` | Non-empty string, valid URL host, required |
| `BackendConfig.Databricks.token` | Non-empty, required |
| `BackendConfig.Databricks.warehouse_id` | Non-empty, required |
| `Query.text` | Non-empty string |
| `AppConfig.default_row_limit` | > 0 (or 0 for unlimited) |
| `AppConfig.query_timeout_secs` | > 0 |
| `AppConfig.allow_write` | Must be true for `exec_write` to execute |
| `AppConfig.output_file` | Parent directory must exist |
