# Amphion fuzz targets — conventions and directory layout

This document describes the conventions for fuzz targets and their corpora in
the Amphion kernel.

## Directory layout

```
qa/fuzz/
  README.md           This file
  <target-name>/
    corpus/           Coverage-interesting seeds managed by the fuzzer runner
    interesting/      Manually curated inputs worth preserving
```

Do not commit the `artifacts/` directory produced by `cargo-fuzz` (crashes and
timeouts); commit only seeds that survive minimisation into `qa/corpus/`.

## Adding a fuzz target

1. Create a new binary under `fuzz/fuzz_targets/<target>.rs` in the relevant
   crate (following `cargo-fuzz` conventions).
2. Parse inputs with `amphion_test_support::FuzzInputReader` for structured
   access; do not use `arbitrary` or other external crates unless approved.
3. At the end of the target, call the operation and assert it does not panic.
   The harness already catches panics as findings.
4. Seed the initial corpus by generating cases with
   `amphion_test_support::TestRng` and writing them to
   `qa/fuzz/<target>/corpus/`.

## Structured input convention

```rust
// fuzz_targets/my_target.rs
#![no_main]
use amphion_test_support::FuzzInputReader;

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let mut r = FuzzInputReader::new(data);
    // Extract structured fields deterministically.
    let width  = r.read_f64_le();
    let height = r.read_f64_le();
    let depth  = r.read_f64_le();
    // Pass inputs to the kernel. Reject non-finite at the boundary.
    // Do NOT unwrap kernel results inside a fuzz target.
    let _ = kernel::cuboid(width, height, depth);
});
```

`FuzzInputReader::read_f64_le` may return NaN, Inf, or any other IEEE 754
value. The kernel must reject these at its public boundary without panicking.
A panic is a finding; a structured `Err` is expected and should not be
re-panicked.

## Corpus to regression pipeline

When a crash or invariant violation is found:

1. Minimise the input with `cargo-fuzz tmin`.
2. Reproduce the minimal case with a deterministic `TestSeed` and record the
   seed + case index.
3. Construct a `CorpusEntry` with `amphion_test_support::CorpusEntry::new` and
   commit it to `qa/corpus/<operation>/<case-id>.json`.

See `qa/corpus/SCHEMA.md` for the entry format.

## Milestone target

The first kernel proof milestone requires at least **10,000 randomised cases**
across primitives, intersections, and booleans (`RANDOMIZED_CASE_MILESTONE =
10_000` in `amphion_test_support`). CI counts the total cases executed across
all `run_invariant_cases` / `run_property_cases` / `run_metamorphic_cases`
invocations and fails the gate if the count is below 10,000.

## Replay

Every discovered failure embeds a [`ReproducibleCommand`] that can be pasted
directly into a terminal. The command sets all seven identity fields required
for deterministic replay:

**POSIX (bash/zsh):**
```
AMPHION_TEST_VERSION=3 AMPHION_TEST_SEED=<seed> AMPHION_TEST_CASE=<idx> \
  AMPHION_TEST_STREAM=<stream> AMPHION_TEST_OPERATION=<op> \
  AMPHION_TEST_CHECK_KIND=<kind> AMPHION_TEST_CHECK=<check> \
  cargo test --package=<pkg> -- --exact <test_name>
```

**PowerShell:**
```
$env:AMPHION_TEST_VERSION='3'; $env:AMPHION_TEST_SEED='<seed>'; $env:AMPHION_TEST_CASE='<idx>';
$env:AMPHION_TEST_STREAM='<stream>'; $env:AMPHION_TEST_OPERATION='<op>';
$env:AMPHION_TEST_CHECK_KIND='<kind>'; $env:AMPHION_TEST_CHECK='<check>';
cargo test --package=<pkg> -- --exact <test_name>
```

All seven fields are required simultaneously; providing a partial set is an error.
The test harness validates each field before generating any input:

| Env var | Constant | Description |
|---------|----------|-------------|
| `AMPHION_TEST_VERSION` | `ENV_TEST_VERSION` | Case-sequence version (must equal `CASE_SEQUENCE_VERSION`) |
| `AMPHION_TEST_SEED` | `ENV_TEST_SEED` | Primary seed (u64 decimal) |
| `AMPHION_TEST_CASE` | `ENV_TEST_CASE` | Zero-based case index (u64 decimal) |
| `AMPHION_TEST_STREAM` | `ENV_TEST_STREAM` | RNG stream name (stable token) |
| `AMPHION_TEST_OPERATION` | `ENV_TEST_OPERATION` | Operation label (stable token) |
| `AMPHION_TEST_CHECK_KIND` | `ENV_TEST_CHECK_KIND` | Check kind: `invariant`, `property`, or `metamorphic_relation` |
| `AMPHION_TEST_CHECK` | `ENV_TEST_CHECK` | Check/relation name (stable token) |
