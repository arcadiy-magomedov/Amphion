# Amphion execution plan: exact B-Rep kernel

## Goal

Build a Rust, Parasolid-class exact/tolerant B-Rep kernel whose canonical model
preserves analytic geometry. Triangle meshes are derived data for rendering
only.

The first proof milestone is:

- analytic cuboid, cylinder, and cone B-Reps;
- exact/tolerant union, intersection, and difference over the declared domain;
- STEP AP242 subset import/export and semantic round-trips;
- at least 10,000 deterministic randomized cases;
- deep topology validation after every tested operation;
- permanent minimized regression cases for every discovered failure;
- no panics, silent healing, or success-shaped fallback results.

## Repository boundaries

Use one monorepo with independent libraries and one-way dependencies:

```text
apps -> SDK/protocol -> runtime -> document -> sketch + solid-kernel
sketch -> foundation
solid-kernel -> topology -> geometry -> foundation
STEP/tessellation -> topology + geometry
```

The initial physical workspace contains only:

```text
crates/
  foundation/
  geometry/
  topology/
  validation/
  test-support/
qa/
  conformance/
  fuzz/
  corpus/
  differential/
  benchmarks/
```

Create `solid-kernel`, `exchange-step`, `sketch`, `document`, `protocol`,
bindings, SDKs, and applications only when their first implementation task is
ready. Do not create empty placeholder crates.

## Parallel-agent protocol

1. One integration agent owns root manifests, `Cargo.lock`, workspace
   configuration, public re-exports, and frozen contracts.
2. Every implementation task has one owner and exclusive paths. Agents must
   not edit files outside those paths.
3. Parallel work uses isolated Git branches/worktrees named
   `agent/<task-id>`. Agents never share an uncommitted worktree.
4. An agent receives one task ID, its frozen input contracts, owned paths,
   required tests, and acceptance gate.
5. If a contract is insufficient, the agent stops and returns a minimal
   contract-change request. It must not change the contract locally.
6. Each result is one focused commit with the commands run and remaining
   limitations. The integration agent merges only after prerequisite tasks and
   the task gate pass.
7. QA agents own tests and corpus paths, not production algorithms. Algorithm
   agents own local unit tests, but cannot weaken independent QA assertions.
8. A review agent performs a read-only correctness review at every wave gate.
9. No task may suppress a failure, widen a tolerance merely to pass a test, or
   replace analytic geometry with a mesh fallback.

## Universal definition of done

Every task must:

- compile without warnings under the pinned Rust toolchain;
- pass formatting, linting, unit, property, and relevant conformance tests;
- preserve deterministic output for identical inputs;
- reject non-finite and unsupported input with structured diagnostics;
- contain no reachable `unwrap`, `expect`, or panic-based error handling;
- add a minimized regression case for every bug found during implementation;
- leave no generated build artifacts in version control;
- change only its owned paths.

Coverage percentage is informational. Invariant coverage, generated state-space
coverage, mutation resistance, and permanent regression coverage are release
criteria.

## Earliest execution waves

Tasks in the same row may run concurrently after the previous dependencies are
merged.

| Wave | Parallel tasks |
| --- | --- |
| 0 | `monorepo-bootstrap` |
| 1 | `kernel-contracts` |
| 2 | `foundation-numerics`, `analytic-geometry`, `topology-core`, `qa-harness`, `step-scope` |
| 3 | `topology-validation`, `property-generators`, `intersection-contracts`, `step-part21`, `corpus-minimizer` |
| 4 | `primitive-conventions`, `plane-intersections`, `cylinder-intersections`, `cone-intersections`, `step-encode`, `step-decode` |
| 5 | `cuboid-brep`, `cylinder-brep`, `cone-brep`, `intersection-test-battery`, `uv-arrangements` |
| 6 | `primitive-test-battery`, `solid-classification`, `topology-splitting` |
| 7 | `boolean-assembly` |
| 8 | `boolean-public-api` |
| 9 | `step-roundtrip-tests` |
| 10 | `boolean-fuzz-differential` |
| 11 | `test-strength-audit` |
| 12 | `milestone-proof-gate` |

Waves 0 and 1 are intentionally sequential. Parallelizing before contracts are
frozen would produce incompatible math, topology, and error models.

## Task DAG

### Workspace and contracts

| Task | Depends on | Exclusive ownership | Deliverable |
| --- | --- | --- | --- |
| `monorepo-bootstrap` | None | Repository root and workspace configuration | Git/Cargo workspace, initial directories, pinned toolchain, standard gates, deterministic settings, and forbidden-dependency check |
| `kernel-contracts` | `monorepo-bootstrap` | Public contract files and crate façades | Coordinate/orientation conventions, units, `ToleranceContext`, typed IDs, errors, evaluator traits, topology relationships, serialization, provenance, panic and thread-safety policy |

### Foundation

| Task | Depends on | Exclusive ownership | Deliverable |
| --- | --- | --- | --- |
| `foundation-numerics` | `kernel-contracts` | `crates/foundation/**` | Points, vectors, transforms, bounds, units, scale-aware tolerances, finite guards, interval/error bounds, and adaptive robust predicates |
| `analytic-geometry` | `kernel-contracts` | `crates/geometry/**` | Line, Circle, Plane, Cylinder, and Cone evaluation, derivatives, transforms, domains, inverse parameter mapping, and error reporting |
| `topology-core` | `kernel-contracts` | `crates/topology/**` | Body, Region, Shell, Face, Loop, Coedge, Edge, Vertex, orientation, adjacency, seams, singularities, builders, snapshots, and provenance |
| `topology-validation` | Foundation, geometry, and topology | `crates/validation/**` | Cheap and deep validation with deterministic diagnostic paths plus intentional-corruption tests for every invariant |

### Test infrastructure

| Task | Depends on | Exclusive ownership | Deliverable |
| --- | --- | --- | --- |
| `qa-harness` | `kernel-contracts` | `crates/test-support/**`, QA harness files | Deterministic seed/replay format, structured fuzz inputs, corpus conventions, resource limits, and machine-readable failures |
| `property-generators` | QA harness and foundation crates | `crates/test-support/src/generators/**` | Shrinkable valid and invalid topology, analytic geometry, transform, tolerance, tangent, coincident, and near-coincident generators |
| `corpus-minimizer` | `qa-harness` | `tools/corpus-minimizer/**`, corpus tooling | Minimization of command sequences, parameters, topology graphs, and STEP entity sets into permanent fixtures |

### Analytic primitives

| Task | Depends on | Exclusive ownership | Deliverable |
| --- | --- | --- | --- |
| `primitive-conventions` | Geometry, topology, validation | Solid-kernel primitive façade and contracts | Canonical orientation, seam, cap, cone-apex, stable-order, provenance, volume, and area conventions |
| `cuboid-brep` | `primitive-conventions` | `primitives/cuboid.rs` and local tests | Arbitrarily placed analytic planar cuboid |
| `cylinder-brep` | `primitive-conventions` | `primitives/cylinder.rs` and local tests | Analytic periodic cylinder with caps and explicit seam topology |
| `cone-brep` | `primitive-conventions` | `primitives/cone.rs` and local tests | Analytic cone/frustum with cap and apex singularity handling |
| `primitive-test-battery` | All primitives, generators, validator | `qa/conformance/primitives/**` | At least 10,000 randomized constructions with analytic, topology, transform, serialization, determinism, and rejection oracles |

### Surface intersections

| Task | Depends on | Exclusive ownership | Deliverable |
| --- | --- | --- | --- |
| `intersection-contracts` | Foundation, geometry, topology | Intersection façade and shared result types | Points, overlap regions, analytic/procedural curves, synchronized 3D curves and p-curves, intervals, certificates, classifications, and deterministic ordering |
| `plane-intersections` | `intersection-contracts` | `intersections/plane.rs` and local tests | Plane-plane, plane-cylinder, and plane-cone intersections |
| `cylinder-intersections` | `intersection-contracts` | `intersections/cylinder.rs` and local tests | General cylinder-cylinder intersections |
| `cone-intersections` | `intersection-contracts` | `intersections/cone.rs` and local tests | General cylinder-cone and cone-cone intersections |
| `intersection-test-battery` | All intersection implementations and generators | `qa/conformance/intersections/**`, `qa/fuzz/intersections/**` | Surface residual, p-curve agreement, symmetry, transform, determinism, tangency, coincidence, certificate, shrinking, and panic tests |

### Boolean pipeline

| Task | Depends on | Exclusive ownership | Deliverable |
| --- | --- | --- | --- |
| `uv-arrangements` | All intersections | `solid-kernel/src/arrangement/**` | Robust p-curve arrangements on closed and periodic faces with seams, singularities, loops, and provenance |
| `solid-classification` | Primitives and foundation | `solid-kernel/src/classification/**` | Explicit IN/OUT/ON classification with adaptive fallback and boundary ambiguity diagnostics |
| `topology-splitting` | UV arrangements and validation | `solid-kernel/src/splitting/**` | Deterministic edge/face splitting with synchronized curves, shared coincident topology, and provenance |
| `boolean-assembly` | Splitting and classification | `solid-kernel/src/boolean/assembly.rs` | Region selection, orientation, stitching, manifold shell construction, and explicit non-manifold failure |
| `boolean-public-api` | Assembly and validation | Kernel façade and public boolean API | Transactional union/intersection/difference, structured diagnostics, validation hooks, cancellation, limits, and no internal-handle leakage |

### STEP AP242 subset

| Task | Depends on | Exclusive ownership | Deliverable |
| --- | --- | --- | --- |
| `step-scope` | `kernel-contracts` | STEP contract files | Frozen AP242/Part 21 subset and semantic equivalence rules |
| `step-part21` | `step-scope` | `exchange-step/src/part21/**` | Bounded streaming lexer/parser/writer with source locations, deterministic output, malformed-input tests, and fuzzing |
| `step-encode` | Part 21, geometry, topology, validation | `exchange-step/src/encode/**` | Valid analytic B-Rep to deterministic AP242 entities |
| `step-decode` | Part 21, geometry, topology, validation | `exchange-step/src/decode/**` | AP242 entity graph to validated analytic B-Rep with STEP provenance |
| `step-roundtrip-tests` | Encode, decode, primitives, booleans, generators | `qa/conformance/step/**`, `qa/fuzz/step/**` | Semantic B-Rep/STEP round-trips, units/orientation checks, malformed-input fuzzing, normalized determinism, and corpus replay |

### Final QA

| Task | Depends on | Exclusive ownership | Deliverable |
| --- | --- | --- | --- |
| `boolean-fuzz-differential` | Boolean API, intersection/primitive tests, STEP tests, minimizer | `qa/fuzz/booleans/**`, `qa/differential/**` | Random disjoint/overlap/containment/tangent/coincident/near-coincident pairs; algebraic laws, validity, volume, transform, STEP, replay, and independent-kernel comparisons |
| `test-strength-audit` | Full QA paths | Mutation-test configuration and audit outputs | Evidence that critical guards are killed by tests and every historical failure has a permanent minimized fixture |
| `milestone-proof-gate` | Every preceding gate | Integration-owned | Complete formatting, lint, architecture, unit, property, conformance, regression, fuzz-smoke, mutation, STEP, and determinism proof run |

## Required test families

| Family | Purpose |
| --- | --- |
| Unit | Local formulas, contracts, and known geometric cases |
| Invariant | Topology, geometry/topology agreement, finite values, orientation, and manifoldness |
| Property-based | Broad generated dimensions, placements, scales, and configurations with shrinking |
| Metamorphic | Operand symmetry where applicable, transform invariance, complement laws, and serialization stability |
| Fuzz | Parser safety, topology transitions, intersections, and full operation sequences |
| Differential | Compare semantic results with independent kernels without adding runtime dependencies |
| Round-trip | Geometry, topology, units, and orientation through STEP and internal serialization |
| Mutation | Prove tests detect removed checks, inverted classifications, and tolerance mistakes |
| Performance | Detect pathological growth and establish reproducible baselines without weakening correctness |

## Wave gate

The integration agent closes a wave only when:

1. All tasks in the wave satisfy their individual definition of done.
2. The merged workspace passes formatting, linting, architecture, and all
   currently applicable tests.
3. A read-only review reports no high-confidence correctness defects.
4. New failures are minimized and committed to `qa/corpus`.
5. Contracts for the next wave are compileable and frozen.

The first executable task is `monorepo-bootstrap`. No other task may start in a
shared implementation branch before `kernel-contracts` is merged.
