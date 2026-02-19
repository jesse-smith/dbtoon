# Feature: Rewrite Query Validation as Deny-List with Safe EXEC Allowlist

## Description

Replace the current AST-based allowlist query validator with a deny-list approach. The current implementation (`src/validation.rs`) enumerates known-safe statement types and rejects everything else, causing false positives on legitimate read-only patterns (transaction wrappers, SET statements, variable declarations, etc.). A deny-list on known write/DDL/DCL statement types is equally comprehensive — every SQL Server and Databricks write path goes through a finite, well-specified set of keywords — and eliminates the ongoing maintenance burden.

Additionally, allow a curated set of known-safe SQL Server system procedures through EXEC, since these are useful for schema exploration (especially by LLM agents).

## Requirements

1. **Deny known-dangerous statements instead of allowing known-safe ones.** Every SQL write path goes through a finite set of statement types (DML, DDL, DCL, ops, EXEC). Block those; allow everything else.

2. **Allow safe system procedures through EXEC.** A hardcoded, compile-time allowlist of read-only SQL Server system stored procedures should be permitted. Research a canonical list of these procedures. The initial list should cover schema/metadata exploration (`sp_help`, `sp_columns`, `sp_tables`, etc.) and session info (`sp_who`). `sp_executesql` is explicitly excluded — it accepts arbitrary SQL strings and cannot be verified.

3. **Parse failures remain denied.** If sqlparser cannot parse the SQL, reject it.

4. **No changes to the public API.** `validate()`, `ValidationResult`, and `BackendDialect` retain their current signatures.

5. **No configuration changes.** The allowlist is compile-time only for now. User-configurable procedure allowlists and a `--allow-exec` flag are deferred to a future feature.
