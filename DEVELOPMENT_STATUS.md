# Amphion development handoff

Updated: 2026-07-20

## Accepted state

The latest accepted implementation commit on `main` is `bad0693` (`geometry:
integrate certified analytic primitives`), merged through PR #1.

Integrated and independently accepted:

- exact foundation numerics: `670516d`;
- validated topology core: `6212395`;
- deterministic QA harness: `ee331be`;
- frozen STEP AP242 subset: `10924d5`;
- certified analytic Geometry: `bad0693`.

The Geometry integration passed all five GitHub Actions jobs in run
`29773500601`, including debug tests on Linux, macOS, and Windows, release-mode
tests, and the complete quality gate. **Wave 2 is closed.**

## Accepted Geometry integration

The accepted integration closes the former Geometry blockers:

- Circle, Cylinder, and Cone evaluation and projection use certified interval
  enclosures of the frozen mathematical ideal frame.
- Cone projection constructs and compares the positive-nappe, negative-nappe,
  and apex candidates by certified squared-distance intervals.
- `EvaluationContext` requires validated budget and derivative-limit groups;
  serde has no missing-field defaults or validation bypass.
- Trigonometric and algebraic certification paths enforce hard pre-allocation
  resource caps, including rational-to-`f64` conversion work.
- Similarity transforms use exact rational classification and reject
  non-representable scales or scaled radii.
- Primitive transforms use exact dyadic affine application and reject
  non-representable transformed coordinates instead of independently rounding
  matrix products.
- Public static domains no longer contain input-dependent panic paths.

`c49b79d` was the initial 225-test proof snapshot. It must not be reused or
integrated alone. Integration reviews subsequently found and corrected:

- Curve2 parameter-space tolerance checks;
- exact representable similarity-scale recovery;
- cone-apex projection documentation;
- preservation of Line2/Line3 affine direction scale for synchronized
  p-curves;
- canonical evaluation of the exact upper seam endpoint on full-period
  circles, cylinders, and cones;
- preservation of Plane affine bases and Circle3/Cylinder/Cone frozen seeds
  across transforms, including bitwise identity no-ops;
- exact affine transform application that preserves Line3/Plane/p-curve
  synchronization under cancellation and retains signed-zero identity bits.

The accepted implementation contains 241 permanent Geometry tests.

The initial source passed independent numeric, curved-frame, and whole-diff
reviews. The corrected integration then passed the complete locked
formatting, check, Clippy, debug/release test, rustdoc, and metadata gates.
A fresh independent read-only review of the complete effective diff returned
no findings before PR #1 was merged.

## Next work

Wave 3 is now unblocked: topology validation, property generators,
intersection contracts, STEP Part 21, and the corpus minimizer. Each task must
start from current `main` in its own exclusive `agent/<task-id>` worktree and
follow the dependency and review gates in `EXECUTION_PLAN.md`.

## Reproducing on another machine

```sh
git clone https://github.com/arcadiy-magomedov/Amphion.git
cd Amphion
git fetch origin
git switch main
git pull --ff-only
```

Start Copilot CLI from the cloned repository and read this file before
continuing. Use `/resume`, then switch the picker to the **Remote** tab, to
locate a synced prior session. The current machine can also expose a live
session with `/remote on` while it remains online.

The local untracked `rust_out` file is intentionally excluded and is not part
of the project state.
