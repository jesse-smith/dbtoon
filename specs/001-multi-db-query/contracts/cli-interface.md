# CLI Interface Contract

**Binary name**: `dbtoon`

---

## Global Flags

| Flag | Short | Env Var | Default | Description |
|------|-------|---------|---------|-------------|
| `--config <PATH>` | `-c` | `DBTOON_CONFIG` | Platform default | Path to config file |
| `--verbose` | `-v` | `DBTOON_VERBOSE=true` | `false` | Emit diagnostics to stderr |
| `--show-secrets` | — | `DBTOON_SHOW_SECRETS=true` | `false` | Disable credential masking |

---

## Subcommands

### `exec-read`

Execute a read-only query. Validates the query cannot modify state before execution.

```
dbtoon exec-read [OPTIONS] <SQL>
dbtoon exec-read [OPTIONS] --file <SQL_FILE>
```

| Argument/Flag | Short | Env Var | Default | Description |
|---------------|-------|---------|---------|-------------|
| `<SQL>` | — | — | — | SQL query text (positional, conflicts with --file) |
| `--file <PATH>` | `-f` | — | — | Read SQL from file (conflicts with positional SQL) |
| `--backend <TYPE>` | `-b` | `DBTOON_BACKEND` | — | Backend type: `sqlserver` or `databricks` |
| `--server <HOST>` | `-s` | `DBTOON_SERVER` | — | SQL Server hostname |
| `--database <DB>` | `-d` | `DBTOON_DATABASE` | — | Database name |
| `--username <USER>` | `-u` | `DBTOON_USERNAME` | — | SQL Auth username |
| `--password <PASS>` | `-p` | `DBTOON_PASSWORD` | — | SQL Auth password |
| `--windows-auth` | `-w` | `DBTOON_WINDOWS_AUTH=true` | `false` | Use Windows Integrated Auth |
| `--host <URL>` | — | `DBTOON_DATABRICKS_HOST` | — | Databricks workspace host |
| `--token <TOKEN>` | — | `DBTOON_DATABRICKS_TOKEN` | — | Databricks bearer token |
| `--warehouse <ID>` | — | `DBTOON_WAREHOUSE_ID` | — | Databricks SQL warehouse ID |
| `--catalog <NAME>` | — | `DBTOON_CATALOG` | — | Databricks catalog |
| `--schema <NAME>` | — | `DBTOON_SCHEMA` | — | Databricks schema |
| `--limit <N>` | `-l` | `DBTOON_ROW_LIMIT` | `500` | Max rows to return |
| `--no-limit` | — | — | `false` | Disable row limit |
| `--timeout <SECS>` | `-t` | `DBTOON_TIMEOUT` | `60` | Query timeout in seconds |
| `--output <PATH>` | `-o` | — | — | Write results to file instead of stdout |
| `--profile <NAME>` | `-P` | `DBTOON_PROFILE` | — | Config file profile name |

**Exit codes**: `0` = success, `1` = error (validation failure, connection error, query error, timeout).

**Stdout**: TOON-formatted query results (or summary if `--output` is used).

**Stderr**: Error messages. Diagnostic info if `--verbose`.

---

### `exec-write`

Execute a query without read-only validation. Requires explicit write opt-in.

```
dbtoon exec-write [OPTIONS] <SQL>
dbtoon exec-write [OPTIONS] --file <SQL_FILE>
```

Accepts the same flags as `exec-read`. Additionally requires:

| Env Var | Required | Description |
|---------|----------|-------------|
| `DBTOON_ALLOW_WRITE` | Yes (must be `true`) | Env/config flag gating write access |

Both `DBTOON_ALLOW_WRITE=true` AND the `exec-write` subcommand are required. The subcommand alone is insufficient (FR-004).

**Exit codes**: Same as `exec-read`.

---

### `list-warehouses`

Discover available Databricks SQL warehouses.

```
dbtoon list-warehouses [OPTIONS]
```

| Flag | Short | Env Var | Default | Description |
|------|-------|---------|---------|-------------|
| `--host <URL>` | — | `DBTOON_DATABRICKS_HOST` | — | Databricks workspace host |
| `--token <TOKEN>` | — | `DBTOON_DATABRICKS_TOKEN` | — | Databricks bearer token |
| `--profile <NAME>` | `-P` | `DBTOON_PROFILE` | — | Config file profile name |

**Stdout**: TOON-formatted list of warehouses (id, name, state, cluster_size, warehouse_type).

---

## Output Contracts

### Query Result (stdout, TOON)

Successful `exec-read` or `exec-write` with results:

```
[N]{col1,col2,...}:
  val1,val2,...
  val1,val2,...
```

Where `N` is the number of returned rows.

### Truncated Result (stdout, TOON)

When row limit is applied and more rows exist:

```
[500]{col1,col2,...}:
  val1,val2,...
  ...
truncated: true
message: Showing 500 rows. Use --no-limit to return all rows.
```

### File Output Summary (stdout, TOON)

When `--output` is specified:

```
rows_written: 500
file: /path/to/output.toon
truncated: false
```

### Zero-Row Result (stdout, TOON)

```
[0]{col1,col2,...}:
```

### Warehouse List (stdout, TOON)

```
[N]{id,name,state,cluster_size,type}:
  abc123,My Warehouse,RUNNING,Small,PRO
  def456,Dev Warehouse,STOPPED,Medium,CLASSIC
```

### Error Output (stderr)

```
error: <category>: <message>
```

Categories: `validation`, `connection`, `query`, `timeout`, `config`, `auth`.

### Verbose Diagnostics (stderr)

```
[dbtoon] connecting to sqlserver at localhost...
[dbtoon] connection established (45ms)
[dbtoon] validating query (read-only mode)...
[dbtoon] validation passed (2 statements, 1ms)
[dbtoon] executing query...
[dbtoon] query complete (234ms, 500 rows)
[dbtoon] formatting TOON output...
```
