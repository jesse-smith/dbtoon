# Research: Agent-Friendly Database Query Tools

Research conducted 2026-02-10. Goal: find or justify building a CLI/MCP tool that gives agents token-efficient query access to both SQL Server and Databricks.

## TL;DR

No single tool covers both SQL Server and Databricks with token-efficient output. The best existing options are dbhub (SQL Server, token-conscious architecture) and RafaelCartenet's mcp-databricks-server (Databricks, progressive disclosure). Neither addresses result-level token efficiency — compact output formats, automatic truncation, or write-to-disk for large results. This is the gap.

---

## Existing Tools

### SQL Server

| Tool | Stars | Language | Token Efficiency | Notes |
|------|-------|----------|-----------------|-------|
| [bytebase/dbhub](https://github.com/bytebase/dbhub) | ~2K | TypeScript | Yes (architectural) | Best option. 2 tools only. Progressive disclosure on schema. |
| [RichardHan/mssql_mcp_server](https://github.com/RichardHan/mssql_mcp_server) | ~307 | Python | No | Most popular MSSQL-specific MCP. PyPI package available. |
| [Aaronontheweb/mssql-mcp](https://github.com/Aaronontheweb/mssql-mcp) | ~139 | C# | No | .NET implementation. |

### Databricks

| Tool | Stars | Language | Token Efficiency | Notes |
|------|-------|----------|-----------------|-------|
| [RafaelCartenet/mcp-databricks-server](https://github.com/RafaelCartenet/mcp-databricks-server) | ~35 | Python | Partial | Progressive disclosure, LLM-optimized markdown. Best community option. |
| [databrickslabs/mcp](https://github.com/databrickslabs/mcp) | ~82 | Python | No | Official Databricks Labs. Unity Catalog server deprecated in favor of Databricks Managed MCP servers. |
| [JordiNeil/mcp-databricks-server](https://github.com/JordiNeil/mcp-databricks-server) | ~46 | Python | No | Simpler. SQL + job management. |

### Multi-Database

| Tool | Stars | Language | Covers Both? | Notes |
|------|-------|----------|-------------|-------|
| [mindsdb/mindsdb](https://github.com/mindsdb/mindsdb) | ~38K | Python | Maybe | Federated query engine. Supports MSSQL + 100 sources. Databricks support unverified. Heavy. |
| [googleapis/genai-toolbox](https://github.com/googleapis/genai-toolbox) | ~13K | Go | No | No MSSQL. Google Cloud focused. |

---

## Deep Dive: dbhub's "Token Efficiency" Claim

dbhub is the only tool that explicitly claims token efficiency. We investigated what that actually means.

### What it IS

**1. Minimal tool surface (biggest factor)**
- 2 MCP tools (`execute_sql`, `search_objects`) = ~1.4k tokens for tool schemas
- Compare: Google MCP Toolbox = 28 tools, ~19k tokens (13.5x more)
- This overhead is paid on every agent turn, so it compounds

**2. Progressive disclosure on `search_objects`**
- `detail_level="names"` → just table names and schemas
- `detail_level="summary"` → adds column/row counts
- `detail_level="full"` → complete DDL, indexes, etc.
- Agent can cheaply scan, then drill into what it needs

### What it is NOT

- No compression or compact encoding of query results
- No automatic truncation — row limits are optional, user-configured via TOML
- No streaming or pagination of large result sets
- Response formatter is straightforward `JSON.stringify` with pretty-printing
- Results from `SELECT` queries come back as standard verbose JSON

### Assessment

Token savings are **front-loaded** (tool definitions) and **metadata-oriented** (progressive schema exploration). For query result payloads — the thing that actually gets large — there's no optimization. An agent pulling 500 rows gets the same bloated JSON any other MCP server would return.

---

## Gap Analysis vs. Our Requirements

| Requirement | dbhub | RafaelCartenet Databricks | Gap? |
|------------|-------|--------------------------|------|
| SQL Server support (Windows auth) | Yes (via tedious driver) | N/A | No |
| Databricks support (token auth) | No | Yes | **Need two tools** |
| Token-efficient result format (CSV, TOON, etc.) | No (JSON) | No (Markdown) | **Yes — nobody does this** |
| Read-only vs read/write separation | Yes (read-only mode) | Read-only by default | Partial — no granular control |
| Non-SELECT read-only (DESCRIBE, query plan) | Unclear | Unclear | **Needs investigation** |
| Default row limits/truncation | Optional config | Not mentioned | Partial |
| Write results to disk | No | No | **Yes — nobody does this** |

### Gaps That Justify Building

1. **No unified tool** covers both SQL Server and Databricks
2. **No tool returns results in token-compact formats** (CSV, TOON, aligned text) — everyone uses JSON or Markdown
3. **No tool writes large results to disk** to keep them out of agent context
4. **Read-only tooling** exists but doesn't clearly handle non-SELECT read-only operations (DESCRIBE, EXPLAIN, sp_help, etc.)

---

## Conclusion

The existing ecosystem solves the "connect to a database from an MCP server" problem but largely ignores the "results are eating my context window" problem. Building a tool that addresses result-level token efficiency, multi-backend support, and context-aware output (disk vs. inline) would fill a real gap.
