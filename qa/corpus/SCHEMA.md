# Amphion regression corpus — schema reference

Version **1.2** (current). Legacy **v1.0** and **v1.1** files load via
`LegacyCorpusDocument`, not `CorpusFile`.
Implementation: `amphion_test_support::corpus` in `crates/test-support/src/corpus.rs`.

## Overview

A corpus file is a JSON document containing permanently committed, minimised
failing test cases. Entries are self-contained and deterministic.

## File schema

```json
{
  "schema_version": { "major": 1, "minor": 2 },
  "entries": [ <CorpusEntry>, ... ]
}
```

`entries` may be empty. Canonical ordering is lexicographic by `CaseId` hex.

## `CorpusEntry` (v1.2)

```json
{
  "schema_version": { "major": 1, "minor": 2 },
  "id": "<32-hex-char case ID>",
  "operation": "<stable.token>",
  "stream_name": "<stable.token>",
  "seed": <u64>,
  "case_index": <u64>,
  "inputs_json": <json-value>,
  "failure_message": "<human-readable description>",
  "case_sequence_version": <u8>,
  "check_kind": "invariant|property|metamorphic_relation",
  "check_name": "<stable.token>",
  "minimization": <MinimizationMeta | null>
}
```

### Required fields in v1.2

- `stream_name`
- `case_sequence_version`
- `check_kind`
- `check_name`

`operation`, `stream_name`, and `check_name` must be stable tokens.

## Legacy compatibility

- `CorpusFile::load_from_str` accepts **only v1.2**.
- `LegacyCorpusDocument::load_from_str` accepts **v1.0** and **v1.1** and
  preserves the exact original JSON bytes.

- **v1.0** legacy documents use:
  - `inputs_json` is stored as a JSON string
  - `stream_name`, `case_sequence_version`, `check_kind`, and `check_name` are absent
- **v1.1** legacy documents use:
  - inline `inputs_json`
  - required `stream_name`
  - required `case_sequence_version`
  - absent `check_kind` / `check_name`
- Migrate legacy documents with `LegacyCorpusDocument::migrate_to_current(...)`.

## Migration matrix

| From | To v1.2 requires caller to supply |
|------|-----------------------------------|
| v1.0 | `stream_name`, `check_kind`, `check_name` |
| v1.1 | `check_kind`, `check_name` |
| v1.2 | nothing |

## Case ID derivation

The ID derivation formula is unchanged.

A `CaseId` is a 32-character lowercase hexadecimal string:
- first 16 hex chars: 64-bit stream seed, little-endian
- last 16 hex chars: 64-bit case index, little-endian

Replay uses:

```text
CaseId::new(seed.for_case_stream(stream_name), case_index)
```

## Versioning policy

- Major version mismatch is rejected.
- Minor versions above `1.2` are rejected.
- v1.0/v1.1 are rejected by `CorpusFile::load_from_str` with `LegacyVersion`.
- Use `LegacyCorpusDocument` for exact-byte round-trip or migration.

## Strict parsing

`CorpusFile` and `CorpusEntry` use strict field validation. Unknown fields are
rejected.
