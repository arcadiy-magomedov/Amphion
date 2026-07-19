# Amphion Copilot instructions

Before changing code, read `CONTRACTS.md`, `EXECUTION_PLAN.md`, and
`DEVELOPMENT_STATUS.md`. Read `PRODUCT_PLAN.md`, `STEP_SCOPE.md`, or
`CAPABILITY_INVENTORY.md` when the task touches those surfaces.

- Analytic/tolerant B-Rep is canonical. Meshes are derived artifacts and may
  never be used as a silent geometry fallback.
- Preserve deterministic behavior, structured failures, finite-input
  validation, immutable topology, semantic identity, and explicit tolerances.
- Treat every numerical certificate as a proof obligation. Do not substitute
  fixed epsilon guesses, host-math fallbacks, clamping, or widened tolerances
  for a missing proof.
- Add a minimized permanent regression for every discovered bug. Prefer
  invariant, property, metamorphic, differential, serialization, and
  adversarial boundary tests over coverage-only assertions.
- Run the smallest relevant tests while iterating, then all locked formatting,
  check, test, release-test, Clippy, and rustdoc gates before integration.
- Follow the exclusive-path worktree protocol in `EXECUTION_PLAN.md`. Do not
  integrate an agent branch until its prerequisites, full gates, and an
  independent read-only correctness review pass.
- For non-trivial algorithms, search primary literature in any relevant
  language, record assumptions and guarantees, compare alternatives, and
  implement clean-room from public specifications.
- Never weaken frozen contracts or tests merely to make an implementation
  pass. Raise a contract-change request when the contract is insufficient.

