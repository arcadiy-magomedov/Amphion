# Amphion development handoff

Updated: 2026-07-20

## Accepted state

`main` and `origin/main` are at `86e5908` (`Document cross-device development
handoff`). The latest accepted implementation commit is `10924d5` (`Freeze
STEP AP242 subset`).

Integrated and independently accepted:

- exact foundation numerics: `670516d`;
- validated topology core: `6212395`;
- deterministic QA harness: `ee331be`;
- frozen STEP AP242 subset: `10924d5`.

The STEP commit passed all five GitHub Actions jobs in run `29698775502`.

## Active work

Analytic Geometry remains the only blocker for closing Wave 2. Its proof,
isolated integration, and local gates are complete.

- Reviewed source branch: `agent/analytic-geometry`
- Initial reviewed source commit: `c49b79d` (**superseded**)
- Integration branch: `agent/integrate-analytic-geometry`
- Integration base: `86e5908`
- Accepted integration artifact: the commit containing this handoff
- Status: **local gates and final read-only review passed; GitHub integration pending**

The integration candidate closes the former blockers:

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

The resulting integration candidate contains 241 permanent Geometry tests.

The initial source passed independent numeric, curved-frame, and whole-diff
reviews. The corrected integration candidate then passed the complete locked
formatting, check, Clippy, debug/release test, rustdoc, and metadata gates.
A fresh independent read-only review of the complete effective integration
diff returned no findings.

Before Wave 2 can close:

1. push the integration commit and require every GitHub Actions job to pass;
2. merge the accepted integration and update this handoff to the resulting
   `main` commit.

## Reproducing on another machine

```sh
git clone https://github.com/arcadiy-magomedov/Amphion.git
cd Amphion
git fetch origin
git switch agent/integrate-analytic-geometry
```

Start Copilot CLI from the cloned repository and read this file before
continuing. Use `/resume`, then switch the picker to the **Remote** tab, to
locate a synced prior session. The current machine can also expose a live
session with `/remote on` while it remains online.

The local untracked `rust_out` file is intentionally excluded and is not part
of the project state.
