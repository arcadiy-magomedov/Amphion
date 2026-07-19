# Amphion

Amphion is an experimental, source-available exact/tolerant B-Rep modeling
kernel written in Rust.

The project is building a portable geometry core for native, web, and mobile
CAD applications. Analytic geometry and B-Rep topology are canonical; triangle
meshes are derived data used only for rendering and interchange formats that
require them.

> **Status:** architecture bootstrap. No modeling API is usable yet.

## Design goals

- Robust analytic curves, surfaces, topology, intersections, and booleans.
- Explicit, scale-aware tolerances instead of hidden global epsilon values.
- Deterministic operations with structured, actionable failures.
- Headless libraries with no UI, renderer, or platform dependencies.
- Native Rust, C ABI, and WebAssembly integration.
- Stable semantic identity and provenance across parametric recomputation.
- A test corpus that permanently captures every discovered failure.

In this project, "exact B-Rep" means that curves and surfaces remain analytic or
NURBS-based instead of becoming triangle meshes. Like industrial CAD kernels,
numerical calculations remain tolerance-based; exact B-Rep does not imply
symbolic or arbitrary-precision arithmetic for every operation.

## Architecture

The monorepo uses independent libraries with one-way dependencies:

```text
apps -> SDK/protocol -> runtime -> document -> sketch + solid-kernel
sketch -> foundation
solid-kernel -> topology -> geometry -> foundation
STEP/tessellation -> topology + geometry
```

The initial workspace contains:

| Crate | Responsibility |
| --- | --- |
| `amphion-foundation` | Math, units, tolerances, error bounds, and robust predicates |
| `amphion-geometry` | Analytic curves and surfaces |
| `amphion-topology` | B-Rep entities, adjacency, orientation, and provenance |
| `amphion-validation` | Cheap and deep geometry/topology validation |
| `amphion-test-support` | Shared property generators, replay, and test utilities |

Sketching, solid operations, document/history, STEP, bindings, SDKs, renderers,
and UI clients will be added as separate libraries or applications when their
implementation work begins.

## First proof milestone

The first milestone requires:

- analytic cuboid, cylinder, and cone B-Reps;
- union, intersection, and difference over an explicitly declared domain;
- STEP AP242 subset import/export with semantic round-trips;
- at least 10,000 deterministic randomized cases;
- deep topology validation and permanent minimized regression cases;
- no panics, silent healing, or mesh fallbacks.

The frozen kernel conventions are in [CONTRACTS.md](CONTRACTS.md). The kernel
dependency graph, agent ownership rules, and acceptance gates are in
[EXECUTION_PLAN.md](EXECUTION_PLAN.md). Human-visible milestones and the
Fusion-familiar browser product contract are in [PRODUCT_PLAN.md](PRODUCT_PLAN.md).

## Quality strategy

Correctness is evaluated with unit, invariant, property-based, metamorphic,
fuzz, differential, round-trip, mutation, and performance tests. Coverage
percentage is secondary to state-space and invariant coverage.

Every failure discovered by tests or users must be minimized into a permanent,
replayable regression fixture.

## Development

Rust 1.96.0 is pinned by `rust-toolchain.toml`.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
```

Pull requests must pass the same gates on Linux, macOS, and Windows through
GitHub Actions.

## Contributions

Bug reports and reproducible geometry cases are welcome. Code contributions are
not accepted until a Contributor License Agreement is published. This is
necessary to preserve Amphion's dual-licensing model.

## License

Source code in this repository is licensed under the
[PolyForm Noncommercial License 1.0.0](LICENSE). It may be used, studied,
modified, and redistributed only for purposes permitted by that license.

Any commercial use requires a separate written commercial license from the
copyright holder. See [COMMERCIAL.md](COMMERCIAL.md). This project is
source-available, not OSI-defined open source.
