# Research: Truncation Metadata

**Feature**: 005-truncation-metadata | **Date**: 2026-02-12

## R1: TOON Root Object Extension

**Decision**: Add `truncated` (boolean) and `message` (string, conditional) keys directly to the existing `serde_json::Map` in `format::to_toon()` before passing to `toon_format::encode_default()`.

**Rationale**: The `toon-format` crate's `encode_default()` accepts any `serde::Serialize` value, including JSON objects with arbitrary keys. The current `to_toon()` already builds a `serde_json::Map` with `"types"` and `"rows"` keys — adding `"truncated"` and `"message"` is a two-line change to the map construction. The TOON encoder handles boolean and string serialization natively.

**Alternatives considered**:
- Wrapping the existing TOON output in an outer envelope object — rejected because it would change the shape of all TOON output and break existing consumers.
- Appending truncation metadata as a separate TOON stanza after the main object — rejected because the spec explicitly requires a single self-describing object (FR-001).

## R2: Arrow Schema Metadata for Parquet and Arrow IPC

**Decision**: Add key-value pairs to the Arrow `Schema.metadata` field (a `HashMap<String, String>`) before passing the schema to the file writers.

**Rationale**: Both Parquet (`ArrowWriter::try_new`) and Arrow IPC (`FileWriter::try_new`) accept an Arrow `Schema` reference and write its metadata to the file. The Parquet format stores key-value metadata in the file footer; Arrow IPC stores it in the schema message. The current `build_record_batch()` in `format_columnar.rs` returns `(Arc<Schema>, RecordBatch)` — the schema can be cloned with additional metadata via `Schema::with_metadata()` before being passed to the writer, without modifying the batch.

The batch's internal schema reference does not need to match the writer's schema metadata (metadata is a writer concern, not a data concern). Verified by inspecting the `ArrowWriter` and `FileWriter` APIs — they use their own schema copy for file-level metadata, not the batch's.

**Key**: `Schema::with_metadata(metadata: HashMap<String, String>)` returns a new `Schema` with the provided metadata replacing the existing metadata. Since the default schema has an empty metadata map, this is effectively an "add" operation.

**Alternatives considered**:
- Adding metadata via Parquet `WriterProperties` — rejected because Arrow IPC doesn't have writer properties, and using different mechanisms for the two formats would duplicate the metadata key knowledge.
- Modifying `build_record_batch()` to accept truncation info — rejected because it violates SoC (record batch building is a data concern, not a metadata concern).

## R3: Valid TOON Print Summary

**Decision**: Replace `format::to_toon_kv()` with `toon_format::encode_default()` in `output::print_summary()`.

**Rationale**: The current `to_toon_kv()` manually formats `key: value\nkey: value` strings. While this resembles TOON surface syntax, it does not handle quoting rules (e.g., file paths with colons, special characters), does not encode types correctly (all values are strings, including numeric `rows_written`), and has no length validation. Using `toon_format::encode_default()` on a `serde_json::Value::Object` guarantees valid TOON output that any TOON parser can decode.

The print summary object will include:
- `"rows_written"`: number (from `serde_json::Value::Number`)
- `"file"`: string (file path)
- `"truncated"`: boolean
- `"message"`: string (only when truncated)

**Alternatives considered**:
- Keeping `to_toon_kv()` and fixing its quoting — rejected because reimplementing TOON quoting is reinventing `toon-format`.
- Using a struct with `#[derive(Serialize)]` — rejected as over-engineering for a 4-field object constructed in one place.

## R4: Stderr Truncation Warning

**Decision**: Add `output::print_truncation_warning()` that writes `"warning: {message}"` to stderr via `eprintln!`.

**Rationale**: FR-012 requires a human-readable truncation warning on stderr for interactive visibility. The warning uses the same message text as the in-band `"message"` key, prefixed with `"warning: "` to match the existing `"error: "` prefix pattern from `output::print_error()`.

**Alternatives considered**:
- Using a structured format on stderr — rejected because stderr is for human consumption, not machine parsing.
- Only warning when stdout is a TTY — rejected because the spec says "regardless of output format or destination."

## R5: Removal of Old Truncation Output

**Decision**: Remove `output::print_truncation_message()` and `format::to_toon_kv()`.

**Rationale**: `print_truncation_message()` appended non-TOON text to stdout after the TOON object, violating SC-004. FR-011 explicitly requires its removal. `to_toon_kv()` is only used by `print_truncation_message()` and `print_summary()` — both callers are being replaced. No other callers exist.

**Impact**: Zero — both functions are internal (not `pub` in `lib.rs`) and only called from `main.rs` and `output.rs`.

## R6: Truncation Message Text

**Decision**: Use `result.rows.len()` (actual rows returned) for the row count in the message, matching the existing pattern: `"Showing {N} rows. Use --no-limit to return all rows."`

**Rationale**: The message describes what the user sees (`result.rows.len()`), not the configured limit (which may differ if fewer rows existed than the limit). In practice these are equal when truncated, but using `rows.len()` is semantically correct and avoids needing to pass the limit value through the output pipeline.

**Edge case**: If `truncated` is true but no limit is configured (shouldn't happen per spec), the message uses `result.rows.len()` and remains accurate.
