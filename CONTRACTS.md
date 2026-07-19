# Amphion kernel contracts

This document freezes the public assumptions required for parallel work on the
first kernel milestone. Changes require an integration decision and coordinated
updates to dependent crates and tests.

## Exact/tolerant geometry

"Exact B-Rep" means that canonical edges and faces retain analytic or NURBS
geometry. Tessellation is derived output and never becomes the source of truth.

Numerical algorithms are tolerance-based. Exactness does not mean symbolic
arithmetic for every operation. Algorithms must either return geometry with an
explicit error certificate inside the caller's `ToleranceContext`, or return a
structured failure. They must not silently widen tolerance or substitute mesh
geometry.

## Model space

- Coordinates are finite `f64` values.
- Canonical length unit: metre.
- Canonical angle unit: radian.
- Coordinate systems are right-handed.
- Positive rotations follow the right-hand rule.
- Points are column vectors.
- `Transform3` stores the first three rows of a 4 by 4 affine matrix in
  row-major order.
- Transform composition applies the right operand first.
- User and exchange units are metadata converted at boundaries.
- NaN and infinity are rejected at every public deserialization and operation
  boundary.

## Tolerances

There is no global epsilon and `ToleranceContext` has no default.

Each comparison-based operation receives:

- positive absolute length tolerance in metres;
- non-negative relative length tolerance;
- positive angular tolerance in radians;
- positive parameter-space tolerance.

Effective length comparison uses `max(absolute, relative * characteristic
scale)`. Individual vertices and edges may carry larger certified tolerances,
but an operation cannot enlarge them merely to force success.

## Geometry

Canonical geometry is stored independently from topology and referenced through
generation-checked typed IDs.

Curve and surface evaluators:

- are immutable, `Send + Sync`, and free of hidden global state;
- declare finite, infinite, and periodic parameter domains explicitly;
- reject non-finite and out-of-domain input;
- provide derivatives through second order;
- return all certified inverse mappings inside the declared domain;
- attach an upper distance bound to inverse mappings;
- use stable geometry-family tags;
- represent non-elementary intersection curves procedurally or as NURBS with
  certified approximation bounds.

Every edge use on a face carries a parameter-space curve synchronized with the
edge's three-dimensional curve. An edge stores a finite increasing trimming
interval on its canonical 3D curve. The p-curve evaluator used by each coedge
must expose the same directed parameter interval, so evaluating either curve at
the same parameter identifies the same model-space point within certified
tolerance. Closed edges may reference one vertex twice while their parameter
interval spans one full period.

## Topology

The entity hierarchy is:

```text
Body
  Region
    outer Shell
    zero or more cavity Shells
      Face
        outer Loop
        zero or more inner Loops
          ordered Coedges
            Edge
              start and end Vertex
```

Rules:

- local handles contain deterministic arena slot and generation;
- stale handles are errors;
- semantic IDs are caller-supplied and deterministic, never random;
- snapshots are immutable and `Send + Sync`;
- public callers cannot mutate entity fields directly;
- lists and maps serialize in deterministic order;
- a face orientation is relative to its support-surface normal;
- an oriented face normal points away from region material;
- outer shell normals point outside the region;
- cavity shell normals point into the cavity;
- an outer loop is counter-clockwise when viewed along the oriented face
  normal;
- an inner loop is clockwise under the same view;
- coedge orientation determines traversal relative to the canonical edge
  curve;
- periodic faces represent seams explicitly through distinct coedges that may
  use the same edge;
- non-manifold incidence is rejected unless a future API explicitly requests a
  non-manifold model.

## Identity and provenance

Local IDs address one immutable snapshot. `SemanticId` identifies model meaning
across deterministic recomputation.

Every topology entity records:

- its semantic ID;
- the creating operation when applicable;
- sorted, deduplicated source semantic IDs;
- a stable semantic role.

Kernel algorithms propagate provenance but do not invent random identity.
Feature and import layers supply operation identities and deterministic ID
derivation.

## Serialization

- Every persisted root carries a `SchemaVersion`.
- Serialized forms use explicit field names and typed IDs.
- Collection order is deterministic.
- Deserialization validates finite numbers, IDs, domains, and tolerances before
  creating canonical values.
- Byte identity is required for the same schema, input, and serialization
  format.
- Semantic equivalence, not byte identity, is required across different schema
  versions and STEP round-trips.

## Errors and diagnostics

Recoverable invalid input and numerical failure return `Result` or a validation
report. Public operations do not panic.

Diagnostics contain:

- a stable uppercase machine code;
- severity;
- a human-readable message;
- a deterministic structured path;
- related semantic IDs where available.

Invalid output is never returned as success. Automatic healing is a separate,
explicit operation and cannot run silently inside construction, boolean, or
import APIs.

## Concurrency and cancellation

- Public canonical values and evaluator traits are `Send + Sync`.
- Operations do not depend on thread-local or mutable global geometry state.
- Deterministic output must not depend on worker scheduling.
- Long-running public operations will accept cancellation and resource limits
  at the kernel facade; low-level algorithms must remain interruptible at
  bounded checkpoints.

## Panic and unsafe-code policy

- Workspace crates forbid unsafe Rust.
- No input-dependent path may use `panic`, `unwrap`, `expect`, unchecked
  indexing, or assertion as error handling.
- Assertions are allowed in tests.
- A process abort is reserved for an impossible internal invariant after a
  validated construction boundary, and must still be replaced with a
  structured failure before the affected API is public.
