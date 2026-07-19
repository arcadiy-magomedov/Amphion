# Amphion QA directory

This directory contains permanent test artefacts that are committed to version
control. Nothing here is generated at build time; everything here is either
authored manually or committed by an automated QA agent after minimisation.

## Layout

```
qa/
  conformance/    Deterministic conformance test fixtures and golden outputs
  corpus/         Permanent minimised regression cases (see corpus/SCHEMA.md)
  differential/   Differential-oracle result logs and classification records
  fuzz/           Fuzz corpus seeds and coverage-interesting inputs
  benchmarks/     Stable reproducible performance baselines
```

## Ownership rules

- `qa/**` is owned exclusively by QA agents. Algorithm-implementation agents
  must not modify these paths.
- Every entry in `qa/corpus/` represents a confirmed kernel failure. Removing
  or modifying entries requires an explicit integration decision.
- `qa/fuzz/*/corpus/` holds coverage-interesting seeds. These are managed by
  the fuzzer runner; do not edit manually.

## Reproducibility contract

Every file in this directory that encodes a test case must contain the
information required to replay it without any external context. For corpus
entries this means: seed, case index, operation label, JSON-encoded inputs,
and failure message. See `qa/corpus/SCHEMA.md` for the exact JSON schema.

## Milestone tracking

The first kernel proof milestone requires at least **10,000 deterministic
randomised cases** covering primitives, intersections, and booleans. The
`amphion_test_support::RANDOMIZED_CASE_MILESTONE` constant (= 10,000) is the
canonical threshold used in CI checks.
