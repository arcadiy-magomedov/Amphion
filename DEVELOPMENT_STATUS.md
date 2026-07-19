# Amphion development handoff

Updated: 2026-07-19

## Accepted state

`main` and `origin/main` are at `10924d5` (`Freeze STEP AP242 subset`).

Integrated and independently accepted:

- exact foundation numerics: `670516d`;
- validated topology core: `6212395`;
- deterministic QA harness: `ee331be`;
- frozen STEP AP242 subset: `10924d5`.

The STEP commit passed all five GitHub Actions jobs in run `29698775502`.

## Active work

Analytic Geometry is the only blocker for closing Wave 2.

- Remote branch: `agent/analytic-geometry`
- Current WIP commit: `df10382`
- Status: **not accepted and not ready to merge**

The branch contains useful exact-rational, trig, transform, typed tolerance,
serialization, and regression work, but its curved-frame proof remains under
adversarial review. In particular, do not accept scalar frame-deviation
patches as a substitute for evaluating and projecting against certified
interval enclosures of the frozen mathematical ideal frame. Cone projection
must compare all admissible nappe/apex candidates by certified distance, not
select only by `sign(h)`.

Additional API cleanup still required at `df10382`:

- remove serde defaults that permit missing derivative-limit/budget fields;
- prevent `EvaluationContext::with_budget` from bypassing budget validation;
- keep mandatory derivative-limit groups constructor-only and slot-typed.

Do not merge Geometry until:

1. the complete branch passes an independent proof-level review;
2. all required adversarial frame, projection, cone, periodic, serde, and
   subnormal-root regressions pass;
3. locked debug/release tests, formatting, check, Clippy, and warnings-denied
   rustdoc all pass;
4. `CONTRACTS.md` is updated to the accepted public API;
5. the branch is rebased or squash-integrated onto current `main`;
6. all GitHub Actions jobs pass on the integrated commit.

## Reproducing on another machine

```sh
git clone https://github.com/arcadiy-magomedov/Amphion.git
cd Amphion
git fetch origin
git switch agent/analytic-geometry
```

Start Copilot CLI from the cloned repository and read this file before
continuing. Use `/resume`, then switch the picker to the **Remote** tab, to
locate a synced prior session. The current machine can also expose a live
session with `/remote on` while it remains online.

The local untracked `rust_out` file is intentionally excluded and is not part
of the project state.
