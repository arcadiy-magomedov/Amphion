# STEP AP242 Subset Scope — Amphion v0

*Input for tasks `step-part21`, `step-encode`, `step-decode`, and `step-roundtrip-tests`.*
*Status: **FROZEN** — all normative mapping blockers resolved (B-01..B-47); implementation may proceed.*

Upstream contracts: [CONTRACTS.md](CONTRACTS.md) · [EXECUTION_PLAN.md](EXECUTION_PLAN.md)

---

## 0. Unresolved Blockers

Normative mapping blockers are listed below. Items B-01..B-09 are resolved.
Items B-10..B-14 are resolved in this revision; see each §-reference.
B-05 remains documentation-only and does not block implementation.

| ID | Status | Resolved by |
|---|---|---|
| B-01 | ✅ Resolved | SMRL CONICAL\_SURFACE formula (§4.4, §13.3) |
| B-02 | ✅ Resolved | Four fixtures with header/APD evidence; CATIA downgraded to OID-only; policy corrected (§8.3) |
| B-03 | ✅ Resolved | SMRL PARAMETRIC\_REPRESENTATION\_CONTEXT WHERE rule (§7.4) |
| B-04 | ✅ Resolved | Part 21 §12.2.5.3 'shall' ascending order (§4.6) |
| B-05 | 📝 Doc-only | ISO 10303-43 catalogue page; SMRL used as normative source |
| B-06 | ✅ Resolved | Exact apex loop frozen: 3 items, PCURVE occurrence mapping, pcurve formulas (§6.6) |
| B-07 | ✅ Resolved | Full 8-row analytic identity matrix with invariants and certification method (§6.3) |
| B-08 | ✅ Resolved | Ryu crate `1.0.23` pinned; exact golden values; CI test requirement (§3.4) |
| B-09 | ✅ Resolved | Non-circular writer algorithm: Phase A–F with structural keys before ID assignment (§10.2) |
| B-10 | ✅ Resolved | Unit pipeline rewritten: staged conversion before validation; no double-scaling (§9.1) |
| B-11 | ✅ Resolved | M2–M6 full 3D coefficient equalities; M7 single-curve; M8 coverage statement (§6.3) |
| B-12 | ✅ Resolved | N-52 redefined as representable file fault (associated\_geometry order [Ptau,P0]) |
| B-13 | ✅ Resolved | Structural key prefixed with type discriminator; SELECT discriminator added (§10.2) |
| B-14 | ✅ Resolved | CATIA APD malformed ($ for required attr); downgraded to OID evidence; N-49 is synthetic (§8.3) |
| B-15 | ✅ Resolved | Per-attribute dimension table; component-wise pcurve scaling; CONVERSION_BASED_UNIT excluded from staging (§9.1) |
| B-16 | ✅ Resolved | Cone R>0 normalization moved to explicit pipeline step 9 before trim/trig/M-matrix (§9.1) |
| B-17 | ✅ Resolved | Placement frame ê(u),ê_θ(u) defined via AXIS2_PLACEMENT_3D axes; raw (cos,sin,0) eliminated (§6.3) |
| B-18 | ✅ Resolved | Curved+curved non-seam pairing rejected with STEP_UNSUPPORTED_ENTITY; §4.5 corrected |
| B-19 | ✅ Resolved | §3 sharing rule defers to typed structural keys; 13→15 fix; full Phase A–F in §10.2; evidence count corrected |
| B-20 | ✅ Resolved | §3.2 impl-level code 4;2/4;3 → STEP\_UNSUPPORTED\_IMPL\_LEVEL\_CLASS; FILE\_SCHEMA import defers to §8.3 whitelist; timestamp gains Z suffix; APD sample corrected to \_mim\_lf/2020; 2014 compat removed from v0 (§3.2, §3.4, §8.3) |
| B-21 | ✅ Resolved | Effective vector = mag×normalize(dir) not mag×raw; per-PCURVE-occurrence staging; raw τ dimensionless; step 9 does NOT shift trim τ; step 12 inverse-maps τ correctly; uncertainty value is in file units (§9.1, §7.2) |
| B-22 | ✅ Resolved | Writer R=0 cone explicit in §5.2; seam/cap pnt expressed in placement-frame vectors (ê(u)); identity-placement note added; p-curve order uses occurrence key not entity IDs (§5.2, §6.3, §6.4, §6.6) |
| B-23 | ✅ Resolved | DEFINITIONAL\_REPRESENTATION exactly ONE 2D curve; same\_sense=.F. import algorithm; v0 CIRCLE = closed only; STEP\_UNSUPPORTED\_ARC/STEP\_AMBIGUOUS\_PCURVE added (§4.3, §4.5, §6.5) |
| B-24 | ✅ Resolved | Phase B: full internable-class list; no 0xFF; TLV encoding; SET duplicate via identity; Phase D: executable root-node table; N-32/N-34/N-60 repaired; N-61..N-64 added (§10.2, §11.2) |
| B-25 | ✅ Resolved | §6.4 cylinder seam pnt includes v\_start; raw pcurve q(τ)=(uᵢ,v\_start+τ); trim τ∈[0,v\_end−v\_start]; M3 raw-wire b=1/staged b=length\_factor; §6.6 remaining +Z/+X literals replaced with Ẑ/X̂ (§6.3, §6.4, §6.6) |
| B-26 | ✅ Resolved | Complete canonical frustum mapping added (§6.7): α,v\_min,v\_max, seam/near/far circles, 4-item lateral loop with closure proof, far cap T/near cap F, M4/M6 verification, P-09/P-12 wire values |
| B-27 | ✅ Resolved | Step 9: transactional overflow checks for shift, each C'\_component, each v+shift; commit only after all certified (§9.1); Phase A occurrence-key construction with doc-root ordinal and topology path; TLV tags for $, *, absent, present (§10.2) |
| B-28 | ✅ Resolved | N-34 replaced with exact computation: R=0.05 m, α=π/4, wrong pcurve pnt.v=0.01 m, M3 residual=0.01√2≈0.01414 m; correct pnt.v=0 shown |
| B-29 | ✅ Resolved | §6.6 remaining world triples replaced with placement-frame: V\_base=C+h·(tan α·X̂+Ẑ); DIRECTION sin α·X̂+cos α·Ẑ; P(t)=C+t·(tan α·X̂+Ẑ); identity-placement example labelled |
| B-30 | ✅ Resolved | §6.7 seam always SEAM\_CURVE; E\_s.T→Ptau/u=TAU/right/near→far; E\_s.F→P0/u=0/left/far→near; loop comments, Note, and unwrapped proof corrected; contradictory .T.=u=0 claim removed |
| B-31 | ✅ Resolved | §6.7.6 M-table: M3×2+M7 for seam; M6 for lateral cone pcurves; M4 for cap plane pcurves; M5 cylinder-only removed; both pcurves of each SURFACE\_CURVE must pass their row |
| B-32 | ✅ Resolved | §6.7.2/§6.7.7 raw τ dimensionless, never staged; b stages to length\_factor m/τ; VECTOR.magnitude is length measure in output unit; raw\_tau\_upper\_numeric renamed; P-12 coefficient check 100×1e-3=0.1 m added |
| B-33 | ✅ Resolved | AXIS2\_PLACEMENT\_3D frame: Ẑ=normalize(axis); Ŷ=normalize(Ẑ×ref\_direction); X̂=Ŷ×Ẑ; reject zero axis, parallel axis/ref, cert failure; §4.4/§4.5/§6.3 updated; skew-nonparallel positive fixture P-17 added; parallel negative retained N-46 |
| B-34 | ✅ Resolved | §4.5 curved+planar pcurve types corrected: curved lateral = 2D LINE q=(φ±t,v\_const); cap PLANE = 2D CIRCLE; planar+planar = both 2D LINE; any unlisted pairing unsupported; §6.3 ordering text consistent |
| B-35 | ✅ Resolved | §6.7.3 stale M4/M6 sentence replaced: M6 checks lateral CONICAL\_SURFACE LINE pcurve; M4 checks cap PLANE CIRCLE pcurve; agrees with §6.7.6 |
| B-36 | ✅ Resolved | Step 8: CONICAL\_SURFACE radius R ≥ 0 explicit; R < 0 → STEP\_INVALID\_PARAMETER; R=0 accepted; R > 0 → step 9; negative fixture N-65 added |
| B-37 | ✅ Resolved | Region SemanticId: Region added to introductory list; ADVANCED\_BREP\_SHAPE\_REPRESENTATION added to derivation table; import reconstruction rule; roundtrip clause already covers Region |
| B-38 | ✅ Resolved | SemanticId extension 45 chars (13+32), not 46; exact length fixture P-17b/N-66..N-68 added |
| B-39 | ✅ Resolved | §6.6 base-circle lateral cone pcurve pnt corrected to (0,h); q(t)=(t,h) now consistent (was pnt=(0,0)→q=(t,0)) |
| B-40 | ✅ Resolved | Step 9 immutability: allocates new occurrence-local IR nodes (cloned AXIS2\_PLACEMENT\_3D, CONICAL\_SURFACE, PCURVE helpers) instead of mutating shared entities; shared placement/DEFINITIONAL\_REPRESENTATION unchanged; fixture P-18 (shared AXIS2\_PLACEMENT\_3D cone+plane), N-69 (overflow leaves IR clean) |
| B-41 | ✅ Resolved | N-64 repaired: raw q=(φ−t,h) + same\_sense=.F. → consistent reversal → canonical F\_y negative, canonical ε positive → M5 ③ residual 2R; §4.3 simultaneous reversal stated explicitly; §6.3/§6.5 independent-of-ORIENTED\_EDGE note added |
| B-42 | ✅ Resolved | §7.3 + step 6: PLANE\_ANGLE\_UNIT required when any CONICAL\_SURFACE present; missing/ambiguous → STEP\_INVALID\_UNIT; omitted in cone-free files → deterministic warning only; N-70 (missing angle unit + cone) and P-19a/P-19b (missing angle unit + cuboid/cylinder warning) added |
| B-43 | ✅ Resolved | N-69: R=1.1e308 m, α=π/6; exact real shift≈1.905e308 is finite as a real but has no finite f64 enclosure; certified interval lower\_bound > f64::MAX before any f64 materialization → STEP\_UNIT\_OVERFLOW; shared PLANE byte-identical; N-69 now includes valid LENGTH/ANGLE units so B-42 cannot preempt |
| B-44 | ✅ Resolved | Step 6 cardinality-exact: collect and count PLANE\_ANGLE\_UNIT members; unsupported kind → STEP\_INVALID\_UNIT always; cone-present: resolved\_count≠1 → STEP\_INVALID\_UNIT; cone-free: count=0 → warning+factor 1.0, count=1 → use it, count>1 → STEP\_INVALID\_UNIT; deterministic selection forbidden |
| B-45 | ✅ Resolved | N-71 added: cone + two PLANE\_ANGLE\_UNIT members (radian + degree) → STEP\_INVALID\_UNIT; proves >1 cardinality rule; acceptance gate updated |
| B-46 | ✅ Resolved | P-19 split: P-19a (cuboid, no angle unit → warning) + P-19b (cylinder with seam/cap pcurve, no angle unit → warning; pcurve u canonical radians unchanged) |
| B-47 | ✅ Resolved | B-43 row and N-69: exact real shift=1.1e308×√3≈1.905e308 is finite as a real; no finite f64 enclosure → certified interval lower\_bound > f64::MAX before any f64 materialization; N-69 includes valid LENGTH\_UNIT + PLANE\_ANGLE\_UNIT so B-42 cannot preempt |

---

## 1. Purpose

This document defines the precise STEP AP242 subset Amphion targets for its first proof milestone: lossless
analytic B-Rep round-trips for **cuboid**, **cylinder**, and **cone** solids. It is the normative
reference for the `step-part21`, `step-encode`, `step-decode`, and `step-roundtrip-tests` tasks. Nothing
not listed here may be silently accepted or emitted. See Section 0 for items that still block
implementation.

---

## 2. Application Protocol, Edition, and Conformance Intent

### 2.1 Standard References

| Reference | Identifier | Note |
|---|---|---|
| Application protocol | ISO 10303-242 | "Managed model-based 3D engineering" |
| Edition 1 | ISO 10303-242:2014 | Paywalled; FILE_SCHEMA string confirmed via public CAx-IF data |
| Edition 2 | ISO 10303-242:2020 | Paywalled; primary target edition |
| Physical file format | ISO 10303-21 Ed3 (2016) | Part 21; defines implementation levels 1, 2, 6 |
| Geometry and topology resource | ISO 10303-42 | Part 42; analytic surface and B-Rep entity definitions |
| Product description resource | ISO 10303-41 | Part 41; product, context, units, uncertainty |
| Representation resource | ISO 10303-43 | Part 43; representation framework |

The FILE_SCHEMA header string for both editions is:

```
FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'));
```

> **Uncertainty — edition disambiguation**: Both AP242 Ed1 and Ed2 use the identical FILE_SCHEMA string.
> The edition cannot be determined from the file header alone. Normative conformance class numbers and
> any edition-distinguishing metadata are in paywalled ISO 10303-242 Annex F. Amphion does not assert an
> edition number inside the written file.

### 2.2 Amphion's Conformance Claim

Amphion does **not** assert a numbered AP242 conformance class. Amphion claims:

> *An implementor subset of ISO 10303-242 AP242 Ed2 MIM Long Form sufficient for single-body manifold
> analytic B-Rep round-trips (Plane, Cylinder, Cone surfaces) using ADVANCED\_BREP topology, with
> deterministic encoding and strict import validation.*

This means:

- Files Amphion writes are structurally valid AP242 MIM LF files and should be accepted by conforming AP242
  importers that support ADVANCED\_BREP\_SHAPE\_REPRESENTATION.
- Amphion's importer accepts **only** the entities listed in Section 4 and rejects all others with structured
  diagnostics.
- Amphion does not claim support for assemblies, PMI/GD&T, colors, layers, NURBS, swept solids,
  tessellation, or any capability absent from Section 4.

---

## 3. Part 21 Syntax and Encoding Subset

### 3.1 File Structure

A valid Amphion Part 21 file has this exact outer structure. No other sections are used:

```
ISO-10303-21;
HEADER;
  FILE_DESCRIPTION(...);
  FILE_NAME(...);
  FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'));
ENDSEC;
DATA;
  ... entity instances ...
ENDSEC;
END-ISO-10303-21;
```

Amphion does not write ANCHOR, REFERENCE, or SIGNATURE sections and will reject files containing them.

### 3.2 Header Records

**FILE\_DESCRIPTION**

```
FILE_DESCRIPTION(('<description_string>'), '2;1');
```

- Implementation level `'2;1'`: Ed1-compatible exchange structure, strictness class 1 (Part 21 §8.2.2). No raw UTF-8; non-ASCII encoded only via `\X2\`/`\X4\` escape sequences.
ISO 10303-21 Ed3 (2016) defines three backwards-compatible conformance classes via the implementation
level string:

| Level | Edition | Additional features |
|---|---|---|
| `'2;1'` | Ed1 (1994) | Basic ASCII + `\X2\`/`\X4\` escape sequences; no anchor/reference sections |
| `'3;1'` | Ed2 (2002) | Adds HEADER-level extensions |
| `'4;1'` | Ed3 (2016) | Raw UTF-8 encoding in strings; adds ANCHOR sections (Part 21 §8.2.2) |
| `'4;2'` | Ed3 (2016) | Adds REFERENCE section (external entity references); **not supported in v0** |
| `'4;3'` | Ed3 (2016) | Further Ed3 extension sections; **not supported in v0** |

- **Amphion writer emits `'2;1'`**: maximally interoperable; all name/description strings are empty or
  printable 7-bit ASCII (U+0020–U+007E). This sidesteps escape-encoding entirely.
- **Non-ASCII under `'2;1'` is an error**: raw UTF-8 multi-byte sequences in string tokens are not
  valid under level `'2;1'`; Amphion import rejects them with `STEP_ENCODING_ERROR`.
  Escape sequences `\X2\..\X0\` (BMP) and `\X4\..\X0\` (supplementary) are the correct form
  (Part 21 §6.4.3) and must be decoded on import.
- **Import accepts `'2;1'`, `'3;1'`, `'4;1'`**. Levels `'4;2'` and `'4;3'` are syntactically valid
  under Ed3 but use REFERENCE/SIGNATURE sections outside v0 scope; import produces
  `STEP_UNSUPPORTED_IMPL_LEVEL_CLASS` (recognised valid class, unsupported in v0). There is no level
  `'6;1'` or similar.
- Import rejects unknown, malformed, or unrecognised level strings (e.g., `'1;1'`) with
  `STEP_UNSUPPORTED_IMPL_LEVEL`.

**FILE\_NAME**

```
FILE_NAME('<name>', '<timestamp>', ('<author>'), ('<org>'),
           'Amphion/<semver>', '', '');
```

- Timestamp: ISO 8601 UTC with `Z` suffix, second precision (`YYYY-MM-DDThh:mm:ssZ`), caller-supplied; writer never reads system time. Import accepts broader ISO 8601 forms (with or without seconds, with or without `Z`).
- Author and org may be empty strings.
- Preprocessor field carries `Amphion/<semver>` for traceability.

**FILE\_SCHEMA**

```
FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'));
```

Case-sensitive. Import accepts only strings matching the §8.3 whitelist; any other value produces `STEP_UNSUPPORTED_SCHEMA`.

### 3.3 Data Section Token Rules

| Construct | Rule |
|---|---|
| Entity instance | `#<uint> = <ENTITY_NAME>(<attr>, ...);` |
| Instance ID | **Writer**: unsigned decimal, no leading zeros, positive (e.g. `#1`, `#42`). **Import**: accepts valid unsigned decimal with or without leading zeros (e.g. `#007` is legal Part 21 and must round-trip); no gaps required; duplicate IDs rejected. |
| String | Single-quoted; empty is `''`; apostrophe doubled `''`; backslash as `\\`; control chars/NUL rejected on import and not emitted by writer; non-ASCII uses `\X2\<hex>\X0\` (BMP) or `\X4\<hex>\X0\` (supplementary) escapes under level `'2;1'` (Part 21 §6.4.3). |
| Real | **Import**: accepts any valid Part 21 REAL token: bare decimal (`1.0`, `1.`), integer-decimal (`1.0E+7`, `1.0E7`, `1.0E-7`), no-digit-after-point (`1.`). **Writer**: emits one canonical form per §3.4 rule 3. |
| Enumeration | `.LITERAL.` with leading and trailing dot |
| Boolean | `.T.` or `.F.` |
| Omitted optional | `$` |
| Complex entity | Parenthesized list of named entity clauses — used for representation context (Section 7) |
| Comments | `/* ... */` accepted on import; Amphion writer does not emit comments |
| Line endings | Writer emits LF only; importer normalises CRLF and accepts LF or CRLF |

### 3.4 Deterministic Writer Rules

The writer must produce **byte-identical** output for identical kernel state, schema version, output unit, and all writer-supplied metadata (timestamp, author name, organisation, preprocessor version, description strings).

1. **Entity ID assignment (B-09 resolved)**: per the non-circular algorithm in §10.2 (Phase A–F). Structural keys computed bottom-up before IDs are assigned; internable geometry nodes deduplicated by key; IDs assigned in deterministic traversal of the full node set.
2. **Attribute order**: exactly as declared in the EXPRESS schema for each entity type.
3. **Real number format (B-08 resolved)** — pinned to Rust crate `ryu = "=1.0.23"`,
   source commit `22a692e0b27d9ca74231a475eb690a9446ed44af` (pure Rust, no\_std, WASM-compatible).

   Algorithm: (a) reject non-finite (NaN/±Inf → `STEP_INVALID_NUMERIC`); (b) if x=±0.0, emit `0.0`;
   (c) otherwise call `ryu::Buffer::format_finite(x)`; (d) replace the sole lowercase `e` with `E`;
   (e) if the mantissa portion (before `E` or at end) contains no `.`, insert `.0` immediately before
   `E` or at the end. No locale, no whitespace, no `+`, no leading exponent zero, no newline.

   **Output grammar**: `-?[0-9]+\.[0-9]+(E-?[0-9]+)?`

   **Golden values** (Ryu 1.0.23 + transforms; use as CI regression set):

   | Input | Output |
   |---|---|
   | `0.0` | `0.0` |
   | `-0.0` | `0.0` |
   | `1.0` | `1.0` |
   | `nextdown(1.0)` = `0x3FEFFFFFFFFFFFFF` | `0.9999999999999998` |
   | `nextup(1.0)` = `0x3FF0000000000001` | `1.0000000000000002` |
   | `5e-324` (min subnormal) | `5.0E-324` |
   | `2.2250738585072014e-308` (MIN_POSITIVE) | `2.2250738585072014E-308` |
   | `1.7976931348623157e308` (MAX) | `1.7976931348623157E308` |
   | `1e-5` | `0.00001` |
   | `1e-6` | `1.0E-6` |
   | `1e15` | `1000000000000000.0` |
   | `1e16` | `1.0E16` |

   The Ryu crate's internal fixed/scientific threshold governs all other values; do not define an
   independent threshold. CI must include a bit-exact round-trip test for every mantissa/exponent
   boundary in the golden table plus random 10 000 finite doubles.
4. **Collections**: serialised in the deterministic order returned by Amphion's internal iterators
   (CONTRACTS.md: "lists and maps serialize in deterministic order").
5. **Timestamp**: caller-supplied UTC `YYYY-MM-DDThh:mm:ssZ` exactly (ISO 8601, `Z` designator, second precision); the writer never reads wall time or derives timestamps internally. Import accepts broader ISO 8601 forms.
6. **Line layout**: one entity instance per line; no line-length wrapping of a single instance.
7. **Geometry sharing** (B-19 — defers to typed structural key algorithm, §10.2 Phase B–C): only
   entities in the explicit **internable class list** (§10.2 Phase C) are candidates for sharing.
   Two internable entities of the same class are shared (collapsed to a single instance) iff their
   structural keys — which embed a type-name discriminator as their first field — are identical.
   Entities of **different classes** are never shared even if all attribute value bytes are equal.
   Non-internable entities (topology, product, context) are never shared regardless of structural
   equality. No within-ULP merging.

---

## 4. Supported Entity Allowlist

Every entity Amphion reads or writes is listed here. Any entity in the DATA section not on this list causes
`STEP_UNSUPPORTED_ENTITY` and fails the import transaction (Section 9.2).

### 4.1 Product and Application Context (mandatory, one of each per file)

| STEP entity | Role |
|---|---|
| `APPLICATION_CONTEXT` | Names the application domain; value `'mechanical design'` |
| `APPLICATION_PROTOCOL_DEFINITION` | Declares AP242; links to APPLICATION\_CONTEXT |
| `PRODUCT` | The product item; references a set of PRODUCT\_CONTEXT |
| `PRODUCT_CONTEXT` | Connects PRODUCT to APPLICATION\_CONTEXT; `discipline_type = 'mechanical'` |
| `PRODUCT_DEFINITION_FORMATION` | A versioned configuration of the product |
| `PRODUCT_DEFINITION_CONTEXT` | Declares the life-cycle stage (e.g. `'design'`); referenced by PRODUCT\_DEFINITION |
| `PRODUCT_DEFINITION` | One realisation of the formation; references PRODUCT\_DEFINITION\_CONTEXT |
| `PRODUCT_DEFINITION_SHAPE` | Declares that this definition carries a shape |
| `SHAPE_DEFINITION_REPRESENTATION` | Binds PRODUCT\_DEFINITION\_SHAPE to ADVANCED\_BREP\_SHAPE\_REPRESENTATION |

Optional (accepted on import, not emitted by default):

| STEP entity | Role |
|---|---|
| `PRODUCT_RELATED_PRODUCT_CATEGORY` | Category membership (e.g. `'part'`); ignored on import |

### 4.2 Shape Representation

| STEP entity | Role |
|---|---|
| `ADVANCED_BREP_SHAPE_REPRESENTATION` | Top-level B-Rep representation; items list contains the solid; context is the combined representation context |

### 4.3 Solid and Topology

| STEP entity | Role |
|---|---|
| `MANIFOLD_SOLID_BREP` | The closed solid; outer attribute is a single CLOSED\_SHELL |
| `CLOSED_SHELL` | Set of oriented faces forming a watertight boundary |
| `ADVANCED_FACE` | One trimmed oriented use of a canonical surface; `same_sense` (from `FACE_SURFACE`) encodes orientation |
| `FACE_OUTER_BOUND` | The single outer trimming loop for each face; `orientation = .T.` |
| `EDGE_LOOP` | An ordered list of ORIENTED\_EDGEs forming one closed boundary |
| `ORIENTED_EDGE` | One directed use of an EDGE\_CURVE in a loop; `orientation` (BOOLEAN) encodes traversal direction — **not** `same_sense` |
| `EDGE_CURVE` | A bounded model-space curve shared between two adjacent faces; `same_sense` (BOOLEAN): `.T.` means increasing curve parameter agrees with edge\_start→edge\_end direction. **Amphion writer always constructs the edge\_geometry so that increasing parameter runs from edge\_start to edge\_end, therefore always emits `.T.`.** Import must accept `.F.` and normalise as follows:
  - Compute raw trim τ interval [τ_a, τ_b] by inverse-mapping edge\_start and edge\_end onto the edge curve
    using the forward (`.T.`) parameterisation.
  - Verify that the curve direction at τ_a agrees with edge\_start→edge\_end when `same_sense=.T.`, or
    disagrees when `.F.` (i.e., the curve runs edge\_end→edge\_start for increasing τ with `.F.`).
  - Construct the canonical internal edge with parameter increasing from start to end: for `.F.`, invert τ
    (use [−τ_b, −τ_a] or equivalently flip the direction). The paired pcurve parameterisation must be
    consistently inverted in the same step.
  - For a closed CIRCLE with `same_sense=.F.`: **simultaneously reverse both** (a) the canonical 3D
    CIRCLE frame (negate F\_y, i.e. select ε\_o = −1 in M5/M6) **and** (b) the canonical pcurve
    orientation (negate the raw pcurve ε\_o). This simultaneous reversal is **independent of
    ORIENTED\_EDGE.orientation**; it is applied once per EDGE\_CURVE occurrence before any coedge
    processing. A raw pcurve `q=(φ+t,h)` with `same_sense=.F.` yields canonical pcurve `q=(φ−t,h)`
    and canonical F\_y negated — the pair is consistent and M5/M6 ③ passes. An internally
    inconsistent raw file where the raw pcurve is already `q=(φ−t,h)` and `same_sense=.F.` yields
    canonical pcurve `q=(φ+t,h)` (ε=+1) but canonical F\_y negated (ε_3d=−1); M5 ③:
    ‖F\_y − ε\_o·R·ê\_θ‖ = ‖−R·ê\_θ − (+1)·R·ê\_θ‖ = 2R >> ε\_l → `STEP_PCURVE_3D_SYNC_FAILURE`. |
| `VERTEX_POINT` | A topological vertex; geometry attribute is a CARTESIAN\_POINT |

v0 restrictions:

- `FACE_BOUND` (inner holes / pockets) is **not in v0 scope**.
- Cavity shells inside MANIFOLD\_SOLID\_BREP (voids) are **not in v0 scope**.
- `OPEN_SHELL` and related non-manifold entities are **not in v0 scope**.
- Exactly **one** MANIFOLD\_SOLID\_BREP per file.

### 4.4 Three-Dimensional Analytic Geometry

| STEP entity | Parameters | Notes |
|---|---|---|
| `CARTESIAN_POINT` | `(name, (x, y, z))` | All values finite `f64`; NaN / infinity rejected |
| `DIRECTION` | `(name, (dx, dy, dz))` | Syntactically not required unit; Amphion normalises on export, validates non-zero on import |
| `VECTOR` | `(name, orientation:DIRECTION, magnitude:REAL)` | magnitude > 0 strictly (schema WHERE rule); used as LINE direction argument; magnitude encodes parameter scale; zero magnitude rejected with `STEP_INVALID_PARAMETER` |
| `AXIS2_PLACEMENT_3D` | `(name, location:CARTESIAN_POINT, axis:DIRECTION, ref_direction:DIRECTION)` | Frame derivation: **Ẑ = normalize(axis)**; **Ŷ = normalize(Ẑ × ref\_direction)**; **X̂ = Ŷ × Ẑ** (right-handed; X̂ is the normalized projection of raw ref\_direction into the plane ⊥ to Ẑ). Reject if axis is zero, or if Ẑ × ref\_direction is zero (parallel), or if any step yields a nonfinite result → `STEP_INVALID_FRAME`. Writer emits already-orthonormal Ẑ and X̂. |
| `PLANE` | `(name, position:AXIS2_PLACEMENT_3D)` | Infinite plane through placement origin in its XY plane |
| `CYLINDRICAL_SURFACE` | `(name, position:AXIS2_PLACEMENT_3D, radius:REAL)` | Axis along Z of placement; circle in XY; radius > 0 (`positive_length_measure` WHERE rule in schema) |
| `CONICAL_SURFACE` | `(name, position:AXIS2_PLACEMENT_3D, radius:REAL, semi_angle:REAL)` | Normative parameterisation: σ(u,v) = C + (R+v·tan α)·(cos u·x̂+sin u·ŷ) + v·ẑ; C=placement origin, R=radius≥0, α=semi\_angle∈(0,π/2) (Amphion restriction); apex at C−(R/tan α)·ẑ. **Amphion writer always emits R=0** with placement origin at apex, so STEP v = Amphion v directly. See §6.4. |
| `LINE` | `(name, pnt:CARTESIAN_POINT, dir:VECTOR)` | Infinite line; parameterisation: P(t) = pnt + t × dir |
| `CIRCLE` | `(name, position:AXIS2_PLACEMENT_3D, radius:REAL)` | Full circle in XY plane of position; parameterisation: P(t) = position\_origin + radius × (cos t · X + sin t · Y); radius > 0 (`positive_length_measure` WHERE rule) |

**CONICAL\_SURFACE parameterisation** (resolved from ISO 10303-42:2021 public SMRL,
https://ap238.org/SMRL\_v8\_final/data/resource\_docs/geometric\_and\_topological\_representation/sys/4\_schema.htm):

> σ(u,v) = C + (R + v·tan α)·(cos u · x̂ + sin u · ŷ) + v · ẑ

where C = placement origin, R = `radius` ≥ 0, α = `semi_angle`, x̂/ŷ/ẑ = placement X/Y/Z axes.
u is periodic with period 2π; v is unbounded along ẑ. Apex is at C − (R/tan α)·ẑ, i.e.,
at STEP v = −R/tan α.

**Amphion canonical writer mapping**: emit with R=0, placement origin = Amphion apex. Then:
σ(u,v) = C + v·tan α·(cos u·x̂ + sin u·ŷ) + v·ẑ, apex at v=0 = C.
STEP v equals Amphion axial-height parameter directly.

**Import reparameterisation** for arbitrary-R STEP files: translate to apex form by
v\_amphion = v\_step + R/tan α. Always performed before any geometry comparison.

Amphion's restriction α ∈ (0, π/2) is an Amphion subset restriction; the schema allows
any angle satisfying the WHERE rule `radius >= 0`. Import rejects α ≤ 0 or α ≥ π/2 with
`STEP_INVALID_PARAMETER`.

### 4.5 Parametric (P-curve) and Surface-Curve Geometry

CONTRACTS.md §Geometry: "Every edge use on a face carries a parameter-space curve synchronized with the
edge's three-dimensional curve." Amphion therefore writes a PCURVE for **every coedge regardless of
surface type**, including planar faces. Import requires a PCURVE for every coedge; a missing p-curve is
`STEP_MISSING_PCURVE` in all cases. Amphion never reconstructs missing p-curves (§9.3).

| STEP entity | Role |
|---|---|
| `SURFACE_CURVE` | Bundles a 3D curve with associated p-curves/surfaces. Attributes: `curve_3d` (the 3D curve), `associated_geometry` (`LIST [1:2] OF pcurve_or_surface`, where `pcurve_or_surface` is SELECT(PCURVE, SURFACE)), `master_representation`. Amphion writes `.CURVE_3D.` |
| `SEAM_CURVE` | Subtype of SURFACE\_CURVE. Schema-enforced (verified from STEP Tools public schema): exactly 2 entries in `associated_geometry`; both must be PCURVE; both must reference the same surface. No additional attributes beyond SURFACE\_CURVE. |
| `PCURVE` | Attributes: `basis_surface` (reference to the face's support surface) and `reference_to_curve` (DEFINITIONAL\_REPRESENTATION) |
| `DEFINITIONAL_REPRESENTATION` | Subtype of REPRESENTATION; attributes: `name`, `items` (SET of 2D curve), `context_of_items` — **required, not `$`**; references a 2D parametric context (see Section 7.4). v0 requires **exactly one** 2D curve in `items`; zero items → `STEP_MISSING_PCURVE`; more than one item → `STEP_AMBIGUOUS_PCURVE`. |
| `PARAMETRIC_REPRESENTATION_CONTEXT` | Subtype of REPRESENTATION\_CONTEXT; no additional attributes; indicates the context is a parametric (parameter-space) domain; used in the 2D context for DEFINITIONAL\_REPRESENTATION |

Four p-curve cases — all are strict; missing PCURVE is always `STEP_MISSING_PCURVE`:

| Case | Surface types | `associated_geometry` type | PCURVE count | Notes |
|---|---|---|---|---|
| Planar + planar | Both PLANE (e.g., all cuboid edges) | `SURFACE_CURVE` | 2 PCURVEs, one per face | Both are 2D LINEs; 2D LINE start point is the edge endpoint projected to (u,v) of each plane |
| Curved + planar | One Cylinder/Cone cap circle bounding PLANE | `SURFACE_CURVE` | 2 PCURVEs | **Curved lateral side**: 2D LINE q=(φ±t, v\_const) (M5 cylinder / M6 cone); **planar cap side**: 2D CIRCLE in plane UV (M4). Any other pairing not listed in M1–M8 is unsupported in v0 → `STEP_UNSUPPORTED_ENTITY`. |
| Curved + curved (non-seam) | Both Cylinder/Cone | **Not supported in v0** | — | Import: `STEP_UNSUPPORTED_ENTITY`; seam adjacency on the SAME surface → SEAM\_CURVE (see Seam row) |
| Seam | Single periodic face using same EDGE\_CURVE twice | `SEAM_CURVE` | Exactly 2 PCURVEs (schema-enforced) | Both reference the same periodic surface, at u=0 and u=2π respectively |

2D geometry inside DEFINITIONAL\_REPRESENTATION uses the same entity names as 3D geometry but with
2-tuple coordinates. All entities listed below are allowlisted:

| 2D entity | When used |
|---|---|
| `CARTESIAN_POINT` | 2-tuple `(u, v)` start point of a p-curve |
| `DIRECTION` | 2-tuple `(du, dv)` direction of a p-curve |
| `VECTOR` | 2D vector: 2-tuple DIRECTION + magnitude; used in 2D LINE |
| `LINE` | Straight line in (u,v) space: seam edges, cap edges on CYLINDRICAL/CONICAL faces |
| `AXIS2_PLACEMENT_2D` | 2D placement `(name, location:CARTESIAN_POINT_2D, ref_direction:DIRECTION_2D)`; required as `position` of a 2D CIRCLE |
| `CIRCLE` | Circular arc in (u,v) space; used when a circular 3D edge maps to a circle in a PLANE face's parameter space |

Import must accept both 2-tuple and 3-tuple CARTESIAN\_POINT; the enclosing DEFINITIONAL\_REPRESENTATION
context determines expected dimensionality.

> **SEAM\_CURVE vs SURFACE\_CURVE**: The SEAM\_CURVE WHERE rules (SIZEOF = 2, both entries PCURVE,
> same surface) are confirmed from the STEP Tools public schema browser, which reflects the merged
> AP schema used in AP242. This is not a remaining uncertainty; use SEAM\_CURVE for all periodic seams.

### 4.6 Units and Representation Context

| STEP entity | Role |
|---|---|
| `REPRESENTATION_CONTEXT` | Abstract supertype; instantiated as part of the combined complex context entity |
| `GEOMETRIC_REPRESENTATION_CONTEXT` | Declares `coordinate_space_dimension = 3` |
| `GLOBAL_UNIT_ASSIGNED_CONTEXT` | Assigns the set of named units (length, angle, solid angle) |
| `GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT` | Assigns the global model accuracy measure |
| `NAMED_UNIT` | Abstract supertype; instantiated as part of unit complex entities |
| `SI_UNIT` | An SI base unit; prefix (optional) and name; used for METRE, RADIAN, STERADIAN |
| `CONVERSION_BASED_UNIT` | A unit defined by conversion from a named unit; used for MILLIMETRE |
| `MEASURE_WITH_UNIT` | Abstract supertype of `LENGTH_MEASURE_WITH_UNIT`; carries `value_component` and `unit_component`; appears as a complex entity part |
| `LENGTH_MEASURE_WITH_UNIT` | Concrete length measure; subtype of MEASURE\_WITH\_UNIT; used as conversion factor |
| `LENGTH_UNIT` | Abstract mixin entity; appears alongside NAMED\_UNIT and SI\_UNIT in unit complex entities |
| `PLANE_ANGLE_UNIT` | Abstract mixin entity; appears in angle-unit complex entities |
| `SOLID_ANGLE_UNIT` | Abstract mixin entity; appears in solid-angle-unit complex entities |
| `DIMENSIONAL_EXPONENTS` | The seven SI dimensional exponents `(L, M, T, I, Θ, N, J)` |
| `UNCERTAINTY_MEASURE_WITH_UNIT` | A named scalar uncertainty with unit; attributes: `(value_component, unit_component, name, description)` |

The representation context is always a single **complex entity instance**:

```
#ctx = (
  GEOMETRIC_REPRESENTATION_CONTEXT(3)
  GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#uncert))
  GLOBAL_UNIT_ASSIGNED_CONTEXT((#len_unit, #angle_unit, #sangle_unit))
  REPRESENTATION_CONTEXT('3D Dimensioning Context', '3D')
);
```

> **Complex entity component ordering** (resolved): ISO 10303-21:2016 §12.2.5.3 states that component
> entity names within a complex entity **shall** appear in ascending alphabetical (lexicographic) order.
> This is normative, not optional. Amphion writes complex entities in ascending-name order.

### 4.7 Mechanical Self-Check: All Entity Tokens in Examples Must Be Allowlisted

Every entity name token appearing in any code example in this document must appear in the allowlist in
Sections 4.1–4.6. The following table enumerates all entity tokens from code examples and confirms
their presence. Any future code example that introduces a new token must also extend the allowlist.

| Entity token in examples | Allowlist section |
|---|---|
| `APPLICATION_CONTEXT` | 4.1 |
| `APPLICATION_PROTOCOL_DEFINITION` | 4.1 |
| `PRODUCT` | 4.1 |
| `PRODUCT_CONTEXT` | 4.1 |
| `PRODUCT_DEFINITION_FORMATION` | 4.1 |
| `PRODUCT_DEFINITION_CONTEXT` | 4.1 |
| `PRODUCT_DEFINITION` | 4.1 |
| `PRODUCT_DEFINITION_SHAPE` | 4.1 |
| `SHAPE_DEFINITION_REPRESENTATION` | 4.1 |
| `ADVANCED_BREP_SHAPE_REPRESENTATION` | 4.2 |
| `MANIFOLD_SOLID_BREP` | 4.3 |
| `CLOSED_SHELL` | 4.3 |
| `ADVANCED_FACE` | 4.3 |
| `FACE_OUTER_BOUND` | 4.3 |
| `EDGE_LOOP` | 4.3 |
| `ORIENTED_EDGE` | 4.3 |
| `EDGE_CURVE` | 4.3 |
| `VERTEX_POINT` | 4.3 |
| `CARTESIAN_POINT` | 4.4 |
| `DIRECTION` | 4.4 |
| `VECTOR` | 4.4 |
| `AXIS2_PLACEMENT_3D` | 4.4 |
| `PLANE` | 4.4 |
| `CYLINDRICAL_SURFACE` | 4.4 |
| `CONICAL_SURFACE` | 4.4 |
| `LINE` | 4.4 and 4.5 |
| `CIRCLE` | 4.4 and 4.5 |
| `SURFACE_CURVE` | 4.5 |
| `SEAM_CURVE` | 4.5 |
| `PCURVE` | 4.5 |
| `DEFINITIONAL_REPRESENTATION` | 4.5 |
| `PARAMETRIC_REPRESENTATION_CONTEXT` | 4.5 |
| `AXIS2_PLACEMENT_2D` | 4.5 |
| `REPRESENTATION_CONTEXT` | 4.6 |
| `GEOMETRIC_REPRESENTATION_CONTEXT` | 4.6 |
| `GLOBAL_UNIT_ASSIGNED_CONTEXT` | 4.6 |
| `GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT` | 4.6 |
| `NAMED_UNIT` | 4.6 |
| `SI_UNIT` | 4.6 |
| `CONVERSION_BASED_UNIT` | 4.6 |
| `MEASURE_WITH_UNIT` | 4.6 |
| `LENGTH_MEASURE_WITH_UNIT` | 4.6 |
| `LENGTH_UNIT` | 4.6 |
| `PLANE_ANGLE_UNIT` | 4.6 |
| `SOLID_ANGLE_UNIT` | 4.6 |
| `DIMENSIONAL_EXPONENTS` | 4.6 |
| `UNCERTAINTY_MEASURE_WITH_UNIT` | 4.6 |

`LENGTH_MEASURE(...)` in attribute position is a typed parameter notation (SELECT value), not an entity
instantiation; no allowlist entry is required. Similarly `LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(...), #n)`
in non-complex form is a concrete entity instantiation and is already in 4.6.

---

## 5. Entity Mapping: Amphion ↔ STEP

### 5.1 Topology

| Amphion entity | STEP entity | Notes |
|---|---|---|
| `Body` | `MANIFOLD_SOLID_BREP` + product structure | One Body per file in v0 |
| `Region` | Implicit; MANIFOLD\_SOLID\_BREP encodes one material region | v0: one Region per Body. Region SemanticId stored in `ADVANCED_BREP_SHAPE_REPRESENTATION.name`; import reconstructs via same regex. |
| outer `Shell` | `CLOSED_SHELL` inside MANIFOLD\_SOLID\_BREP | One CLOSED\_SHELL per solid |
| cavity `Shell` | *Deferred (Section 12)* | |
| `Face` | `ADVANCED_FACE` | `ADVANCED_FACE.same_sense` (inherited from `FACE_SURFACE`) encodes face orientation |
| outer `Loop` | `FACE_OUTER_BOUND` → `EDGE_LOOP` | One per Face in v0 |
| inner `Loop` | *Deferred (Section 12)* | |
| `Coedge` | `ORIENTED_EDGE` | `ORIENTED_EDGE.orientation` (BOOLEAN) encodes loop traversal direction |
| `Edge` | `EDGE_CURVE` | `edge_start`/`edge_end` = VERTEX\_POINT; `edge_geometry` = 3D curve or SURFACE\_CURVE |
| `Vertex` | `VERTEX_POINT` → `CARTESIAN_POINT` | |

### 5.2 Geometry

| Amphion geometry | STEP entity | Mapping notes |
|---|---|---|
| `SurfaceKind::Plane` | `PLANE` | Z-axis of placement = surface normal (when `Face.orientation = Same`) |
| `SurfaceKind::Cylinder` | `CYLINDRICAL_SURFACE` | Z-axis = cylinder axis; radius as declared |
| `SurfaceKind::Cone` | `CONICAL_SURFACE` | Z-axis = cone axis; **writer always emits `radius=0` with placement origin at apex**; `semi_angle` in radians. Arbitrary-R import reparameterized to apex form in step 9. |
| `CurveKind::Line` (3D) | `LINE` | `pnt` = a point on the line; `dir` VECTOR |
| `CurveKind::Circle` (3D) | `CIRCLE` | `position` Z-axis = circle plane normal; radius as declared |
| `CurveKind::Line` (2D p-curve) | `LINE` inside DEFINITIONAL\_REPRESENTATION | 2D; parameters in surface (u,v) space |
| `CurveKind::Circle` (2D p-curve) | `CIRCLE` inside DEFINITIONAL\_REPRESENTATION | 2D; used where parameterisation maps the edge to an arc |
| coordinate frame | `AXIS2_PLACEMENT_3D` | Ẑ=normalize(axis); Ŷ=normalize(Ẑ×ref\_direction); X̂=Ŷ×Ẑ (right-handed). Reject zero axis, parallel axis/ref, or cert failure → `STEP_INVALID_FRAME`. (CONTRACTS.md §Model space) |
| `Point3` | `CARTESIAN_POINT` | 3-tuple `(x, y, z)` in declared unit |
| unit direction | `DIRECTION` | Amphion normalises before writing; import checks non-zero |

---

## 6. Orientation, P-curves, Trimming, Seam, and Closed-Edge Rules

### 6.1 Face and Shell Orientation

- `ADVANCED_FACE.same_sense = .T.` — face normal agrees with the underlying surface's outward normal.
- `ADVANCED_FACE.same_sense = .F.` — face normal is reversed relative to the surface normal.
  (`same_sense` is defined on `FACE_SURFACE`, inherited by `ADVANCED_FACE`; attribute name confirmed from
  STEP Tools public schema, https://www.steptools.com/stds/stp_aim/html/t_face_surface.html)
- Mapping: Amphion `Orientation::Same` → `.T.`; `Orientation::Opposite` → `.F.`.
- CONTRACTS.md rule: "an oriented face normal points away from region material". For the outer shell of a
  manifold solid, every face normal points outward. The writer enforces this invariant; the reader validates
  it.

### 6.2 Edge Orientation

- `ORIENTED_EDGE.orientation = .T.` — the loop traverses the edge from `EDGE_CURVE.edge_start` to
  `EDGE_CURVE.edge_end`.
- `ORIENTED_EDGE.orientation = .F.` — the loop traverses in the opposite direction.
  (`orientation` is the correct EXPRESS attribute name; `ORIENTED_EDGE` has no `same_sense` attribute;
  confirmed from STEP Tools public schema, https://www.steptools.com/stds/stp_aim/html/t_oriented_edge.html)
- Mapping: Amphion `Coedge.orientation::Same` → `.T.`; `Coedge.orientation::Opposite` → `.F.`.
- `EDGE_CURVE.same_sense = .T.` always in Amphion-written files. The canonical edge\_geometry is constructed with increasing parameter from edge\_start to edge\_end; `same_sense` is therefore `.T.` by construction, not by conditional proof.

### 6.3 P-curves and SURFACE\_CURVE

For every edge whose `EDGE_CURVE.edge_geometry` is a `SURFACE_CURVE` or `SEAM_CURVE`:

1. `SURFACE_CURVE.curve_3d` is the 3D curve (LINE or CIRCLE).
2. `SURFACE_CURVE.associated_geometry` is `LIST [1:2] OF pcurve_or_surface` where `pcurve_or_surface`
   is SELECT(PCURVE, SURFACE). See Section 4.7 self-check table; `SURFACE_CURVE` is in 4.5.
3. `SURFACE_CURVE.master_representation = .CURVE_3D.` — the 3D curve is authoritative in Amphion.
4. Each `PCURVE.basis_surface` references the face's support surface.
5. Each `PCURVE.reference_to_curve` is a `DEFINITIONAL_REPRESENTATION`; its `context_of_items` is a
   2D parametric context (see Section 7.4).

**P-curve cardinality by case** (see Section 4.5 table); all cases are strict:

- *Planar + planar*: 2 PCURVE entries, both 2D LINEs. One per adjacent face. Ordered by the
  **pre-ID canonical topology occurrence key** `(region_index, shell_index, face_index)` of the
  adjacent face using Amphion's deterministic iterator order — the face with the lower tuple index
  first. This key is assigned in Phase A before any entity IDs are known; no ID comparison is used.
  Import associates each PCURVE entry to the adjacent face via `PCURVE.basis_surface`; list order
  need not be checked by the importer.
- *Curved + planar*: 2 PCURVE entries. Ordered: curved-surface PCURVE first, planar-surface PCURVE second.
- *Curved + curved (non-seam)*: **not admitted in v0** → `STEP_UNSUPPORTED_ENTITY`. Only seam adjacency (same periodic surface, SEAM\_CURVE) is admitted.
- *Seam*: exactly 2 PCURVE entries (SEAM\_CURVE schema-enforced); PCURVE at u=0 first.

**Import transactional validation — analytic identity matrix (B-07 resolved)**:

> **Unsoundness of sampling**: unit cylinder, C(t)=(1,0,t), q(t)=(4πt,t). S(q(0))=S(q(0.5))=S(q(1))
> all equal C(t) at samples, but S(q(0.25))=(-1,0,0.25)≠C(0.25). Sampling **never** certifies the
> whole interval. Only whole-interval algebraic invariants are accepted below.

Define: **effective 3D vector** A = VECTOR.magnitude × normalize(DIRECTION); trim interval [a,b].
All checks use caller `ToleranceContext`: ε_l = `absolute_length`, ε_a = `angular`, ε_p = `parameter_space`.
For a CIRCLE or SEAM full-circle: b−a must equal TAU within ε_p; no modular reduction of slope/phase.

**Family matrix — full 3D coefficient equalities (all rows must pass for their pairing):**

Notation (B-17 — use certified AXIS2_PLACEMENT_3D frame, not world coordinates):

**Surface frame**: for any CYLINDRICAL or CONICAL surface with AXIS2_PLACEMENT_3D defining
origin C and raw `axis`/`ref_direction` attributes, the certified frame is:
- **Ẑ = normalize(axis)**
- **Ŷ = normalize(Ẑ × ref\_direction)** — this removes any component of ref\_direction parallel to Ẑ then normalizes
- **X̂ = Ŷ × Ẑ** — unit vector in the ref\_direction half-plane; right-handed triple (X̂, Ŷ, Ẑ)

Raw ref\_direction need not be orthogonal to axis (skew allowed; the formula handles it). Reject if Ẑ × ref\_direction is zero (parallel case) → `STEP_INVALID_FRAME`.

- **ê(u)** = cos(u)·X̂ + sin(u)·Ŷ  (azimuthal radial unit vector at angle u)
- **ê_θ(u)** = −sin(u)·X̂ + cos(u)·Ŷ  (azimuthal tangential unit vector)

These reduce to (cos u, sin u, 0) and (−sin u, cos u, 0) only for identity placement (X̂=(1,0,0), Ŷ=(0,1,0)). Coefficient checks for P-07/P-11 (tilted cylinders/cones) **must** use the actual surface axes, not world axes. A check using raw (cos,sin,0) would incorrectly accept/reject files with non-identity placement.

3D CIRCLE frame vectors: F_x = R_c·X̂_c, F_y = R_c·Ŷ_c (radius × AXIS2_PLACEMENT_3D ref/binormal axes).
2D CIRCLE frame vectors: G_x = r·Û_q, G_y = r·V̂_q where V̂_q = (−Û_q.y, Û_q.x) (2D perpendicular).
Cone: apex C, semi-angle α. All checks use certified interval arithmetic.

| # | 3D curve | Surface | PCURVE | Coefficient equalities (whole-interval; each residual ≤ stated tolerance) |
|---|---|---|---|---|
| M1 | LINE P(t)=P₀+t·A | PLANE (O, X̂,Ŷ) | LINE q(t)=(u₀+bᵤt, v₀+bᵥt) | **①** \|P₀ − (O+u₀X̂+v₀Ŷ)\| ≤ ε_l; **②** \|A − (bᵤX̂+bᵥŶ)\| ≤ ε_l |
| M2 | Seam LINE on CYLINDER (O,R,Ẑ) at azimuth uᵢ | CYLINDRICAL\_SURFACE | LINE q=(uᵢ, v₀+b·t) | **①** \|P₀ − (O+R·e(uᵢ)+v₀·Ẑ)\| ≤ ε_l (full point: radial AND axial offset); **②** \|A − b·Ẑ\| ≤ ε_l (u-slope=0, v-slope=b); **③** uᵢ ∈ {0,TAU}±ε_p; **④** same v₀/b/trim for seam pair |
| M3 | Seam LINE on CONE (C,α,R=0) at azimuth uᵢ | CONICAL\_SURFACE | LINE q=(uᵢ, v₀+b·t) | **①** \|P₀ − (C+v₀·tan α·ê(uᵢ)+v₀·Ẑ)\| ≤ ε_l; **②** \|A − b·(tan α·ê(uᵢ)+Ẑ)\| ≤ ε_l; **③** uᵢ ∈ {0,TAU}±ε_p; **④** same v₀/b/trim. Canonical apex writer: v₀=0, **raw wire b=1**, staged b=length\_factor (m/τ); 3D A scales identically. |
| M4 | CIRCLE: C₃(t)=Oc+cos(t)·F_x+sin(t)·F_y | PLANE (Op, X̂_p,Ŷ_p) | CIRCLE 2D: q(t)=qc+cos(t)·G_x+sin(t)·G_y | **①** \|Oc − (Op+qc.u·X̂_p+qc.v·Ŷ_p)\| ≤ ε_l; **②** \|F_x − (G_x.u·X̂_p+G_x.v·Ŷ_p)\| ≤ ε_l; **③** \|F_y − (G_y.u·X̂_p+G_y.v·Ŷ_p)\| ≤ ε_l. (Centre+radius+normal alone permits reversed parameterisation; ② and ③ together certify frame and orientation.) |
| M5 | CIRCLE on CYLINDER (O,R_cyl,Ẑ): C₃(t)=Oc+cos(t)·F_x+sin(t)·F_y | CYLINDRICAL\_SURFACE | LINE q=(φ+ε_o·t, h), ε_o∈{±1} | **①** \|Oc − (O+h·Ẑ)\| ≤ ε_l (full centre, not just height); **②** \|F_x − R_cyl·e(φ)\| ≤ ε_l; **③** \|F_y − ε_o·R_cyl·e_θ(φ)\| ≤ ε_l; **④** v-slope = 0, \|ε_o\|=1±ε_p; **⑤** b−a = TAU±ε_p |
| M6 | CIRCLE on CONE (C,α,R=0): same as M5 | CONICAL\_SURFACE | LINE q=(φ+ε_o·t, h), ε_o∈{±1} | Same as M5 with R_cyl replaced by h·tan α: **①** \|Oc − (C+h·Ẑ)\| ≤ ε_l; **②** \|F_x − h·tan α·e(φ)\| ≤ ε_l; **③** \|F_y − ε_o·h·tan α·e_θ(φ)\| ≤ ε_l |
| M7 | ONE shared 3D LINE (the single EDGE\_CURVE) | CYLINDER or CONE | Two PCURVEs [P0, Ptau] | Apply M2 (cylinder) or M3 (cone) to the **same** 3D curve paired with each PCURVE independently; then require: same surface entity, same v₀/b/trim for both; u₀ = 0±ε_p; uτ = TAU±ε_p. (There is one 3D curve, not two.) |
| M8 | LINE (planar adjacency) | Two PLANEs | Two 2D LINEs | Apply M1 independently for each (PLANE, PCURVE) pair. Non-seam adjacency between two curved surfaces → `STEP_UNSUPPORTED_ENTITY` in v0 (no coefficient row exists). |

**Counterexamples that now fail with full coefficient equalities**:
- q(t)=(4πt,t) on unit cylinder: M2 ② fails — u-slope = 4π ≠ 0.
- C(t)=(1,0,t) with q=(0,1+t) on unit cylinder (R=1,O=origin): M2 ① fails — P₀=(1,0,0) ≠ O+R·e(0)+1·Ẑ = (1,0,1).
- CIRCLE/PLANE with reversed 2D frame (G_y negated): M4 ③ fails — ‖F_y − (G_y.u·X̂_p+G_y.v·Ŷ_p)‖ > ε_l.
- CIRCLE/CYLINDER with off-axis centre (Oc=(0.5,0,h)): M5 ① fails — ‖Oc − (O+h·Ẑ)‖ = 0.5 > ε_l.

**Curved+curved non-seam adjacency** (B-18): non-seam adjacency between two distinct curved surfaces
(any CYLINDRICAL/CONICAL pair that is NOT a SEAM_CURVE on the same surface) is **not admitted in v0**.
No coefficient row exists; import produces `STEP_UNSUPPORTED_ENTITY`. Seam adjacency on the same
periodic surface is covered by M7 (SEAM_CURVE with ONE shared 3D curve, TWO distinct PCURVEs).

**Phase/slope rule** (M2/M3): u-slope must be exactly 0 (within ε_p); any nonzero u-slope is immediate failure. (M5/M6): |ε_o| must equal 1 (within ε_p). No modular reduction.

**Certification method**: all residuals must be computed using **correctly-enclosed interval arithmetic**
(each operation produces an interval guaranteed to contain the true real-valued result). For trigonometric
functions, use correctly-rounded interval transcendental evaluations. A residual is certified only if the
interval upper bound ≤ stated tolerance. An ambiguous or non-converging interval fails. Sampling is extra
test coverage only (Section 11.3) and never substitutes for certification.

**Backend requirement** (mandatory for `step-decode` acceptance): a certified interval/transcendental
library targeting both native and WASM must be provided. The import crate must not call `f64::sin`
directly for certification; only the interval backend may be used.

Failure codes: `STEP_PCURVE_3D_SYNC_FAILURE` for M1–M6 and M8 (planar adjacency); `STEP_INVALID_SEAM_PCURVES` for M7; `STEP_UNSUPPORTED_ENTITY` for non-seam curved+curved adjacency.

### 6.4 Seam Edges on Periodic Surfaces

CYLINDRICAL\_SURFACE and CONICAL\_SURFACE are periodic in u (azimuth) with period 2π.
The seam makes the parameter domain simply-connected:

- The seam edge is a `SEAM_CURVE` (not plain SURFACE\_CURVE).
- Two `ORIENTED_EDGE` instances in the lateral face's EDGE\_LOOP reference the **same** EDGE\_CURVE with
  opposite `orientation` values.
- `SEAM_CURVE.associated_geometry` carries exactly two PCURVE entries (schema-enforced):
  - Entry 1 (at u = 0): 2D LINE with start point `CARTESIAN_POINT('', (0.0, v_start))`,
    direction `VECTOR(DIRECTION('', (0.0, 1.0)), 1.0)`.
  - Entry 2 (at u = 2π): 2D LINE with start point `CARTESIAN_POINT('', (6.28318…, v_start))`,
    direction `VECTOR(DIRECTION('', (0.0, 1.0)), 1.0)`.
  **The two LINEs have different 2D CARTESIAN\_POINT start values and must be distinct entity instances.**
  They may share the same 2D DIRECTION instance (same bitwise value). They may **not** share the same
  LINE instance because the LINE's `pnt` attribute differs.
- **Seam 3D curve — cylinder**: The seam LINE lies **on the cylinder surface** at the canonical seam
  azimuth (u=0), **not** on the cylinder axis. Using the surface AXIS2\_PLACEMENT\_3D (origin O,
  axis Ẑ, ref X̂):
  - `pnt` = O + R·X̂ + v_start·Ẑ (point on cylinder surface at azimuth 0 and height v_start)
  - `dir` = VECTOR with DIRECTION Ẑ and **raw wire magnitude 1.0**; after staging b=length\_factor.
  - Raw pcurve q(τ) = (0, v_start + τ); raw trim τ ∈ [0, v_end − v_start] (in output numeric units).
  - P(τ) = O + R·X̂ + v_start·Ẑ + τ·Ẑ = O + R·X̂ + (v_start + τ)·Ẑ = σ(0, v_start + τ) ✓.
  Identity-placement example (X̂=(1,0,0), Ẑ=(0,0,1), O=world origin): `pnt=(R, 0.0, v_start)`, `dir=VECTOR(DIRECTION('', (0.0, 0.0, 1.0)), 1.0)`, raw trim τ∈[0, v_end−v_start].
  For P-07 (tilted 30° from Z: Ẑ=(0,sin30°,cos30°), X̂=(1,0,0)): `pnt = O+(R,0.0,0.0)`, `dir` = Ẑ direction.
- **Seam 3D curve — cone** (R=0 canonical writer form; see §4.4 and §5.2):
  The seam LINE is the surface generatrix (surface parameter line at u=0), **not** the axis.
  With placement origin at apex C and semi\_angle α:
  Using the surface AXIS2\_PLACEMENT\_3D (apex C, axis Ẑ, ref X̂ with Ŷ=normalize(Ẑ×X̂)):
  - `pnt` = C (= apex = placement origin for R=0 canonical writer)
  - `dir` = VECTOR with DIRECTION (sin α·X̂ + cos α·Ẑ) and magnitude sec α
  so that P(τ) = C + τ·sec α·(sin α·X̂+cos α·Ẑ) = C + τ·(tan α·X̂+Ẑ) = σ(0,τ).
  For identity placement (X̂=(1,0,0), Ẑ=(0,0,1)):
  `pnt = CARTESIAN_POINT('', (0.0, 0.0, 0.0))`,
  `dir = VECTOR(DIRECTION('', (sin α, 0.0, cos α)), sec α)`.
  For P-11 (tilted 20°: Ẑ=(0,sin20°,cos20°), X̂=(1,0,0)): `pnt=C`, `dir`=(sin α·(1,0,0)+cos α·(0,sin20°,cos20°))×sec α.
  Canonical direction: **away from apex** (increasing τ is positive v).
  `EDGE_CURVE.edge_start = V_apex`, `EDGE_CURVE.edge_end = V_base`, `EDGE_CURVE.same_sense = .T.`.
  Reversing an ORIENTED\_EDGE reverses only that coedge's traversal direction; it does **not** modify
  the shared EDGE\_CURVE, its seam LINE, or the shared SEAM\_CURVE PCURVEs. The 3D LINE and both
  p-curve LINEs always point away from apex regardless of any ORIENTED\_EDGE orientation value.

- **Seam p-curve entity sharing**: The two SEAM\_CURVE PCURVEs at u=0 and u=2π require **distinct**
  LINE instances (different `pnt`). Their DIRECTION and VECTOR helper entities **may** be shared if
  bitwise-identical (same α, same orientation). Sharing DIRECTION and VECTOR across the two p-curve
  LINEs is correct and canonical; sharing the LINE itself is forbidden because `pnt` differs.

**PCURVE occurrence mapping for seam coedges**: When the same SEAM\_CURVE EDGE\_CURVE appears twice
in an EDGE\_LOOP with opposite orientations, the two coedges traverse different parameter boundaries.
The mapping between SEAM\_CURVE.associated\_geometry = [P0, Ptau] and the two ORIENTED\_EDGE
occurrences is as follows (Amphion canonical convention):

| ORIENTED\_EDGE.orientation | Parameter boundary | PCURVE used |
|---|---|---|
| `.T.` (forward: apex→base or bottom→top) | u = TAU (right edge of unwrapped rectangle) | Ptau (index 1 in [P0, Ptau]) |
| `.F.` (reversed: base→apex or top→bottom) | u = 0 (left edge of unwrapped rectangle) | P0 (index 0 in [P0, Ptau]) |

This mapping ensures the coedge traversal direction agrees with the positive boundary of the
surface's unwrapped parameter rectangle (CCW when viewed from the outward normal S_u × S_v).
Shared EDGE\_CURVE and SEAM\_CURVE entities are **never** modified per coedge use; only the
ORIENTED\_EDGE orientation attribute changes.

**Writer invariant**: the seam is placed at u=0 azimuth. The .T. ORIENTED\_EDGE (Ptau side) always
appears as the first seam item in the canonical EDGE\_LOOP (see §6.6).

### 6.5 Closed Edges (Full Circles)

For circular edges that close on themselves (base circle of a cylinder or cone; both cap circles of a
cylinder):

- `EDGE_CURVE.edge_start` and `EDGE_CURVE.edge_end` reference the **same** VERTEX\_POINT instance.
- The CIRCLE parameter interval spans exactly one full period \[0, 2π\] (closed, finite). Start and
  end vertices are the same `VERTEX_POINT` instance and evaluate to the same 3D point.
- The vertex is the point on the circle at parameter t = 0, which coincides with the seam edge's endpoint
  on the same circle.

**Import rule**: when `edge_start = edge_end`, the edge is classified as **closed** (full circle).
The circle parameter interval is reconstructed from the start vertex: set t\_start = 0 (or the
inverse-mapped angle of the vertex position), t\_end = t\_start + 2π. The vertex 3D position must
evaluate within `ToleranceContext.absolute_length` of the CIRCLE at t\_start and at t\_end.

**v0 restriction — closed full circles only**: v0 does not support non-closed CIRCLE arcs
(`edge_start ≠ edge_end` on a CIRCLE, or a trim interval other than a full 2π period).
A non-closed CIRCLE arc produces `STEP_UNSUPPORTED_ARC`. This restriction applies to both CIRCLE
in EDGE\_CURVE.edge\_geometry and CIRCLE in SURFACE\_CURVE.curve\_3d.

`edge_start = edge_end` on a non-periodic LINE is `STEP_INVALID_CLOSED_EDGE`.

**Note — simultaneous reversal independent of ORIENTED_EDGE**: `EDGE_CURVE.same_sense=.F.` reverses
BOTH the canonical 3D CIRCLE frame (F_y sign) and the canonical pcurve orientation (ε_o sign) in one
operation when the edge is first processed (step 12 / M5/M6). ORIENTED_EDGE.orientation reverses only
the traversal direction of a coedge; it does not affect the M5/M6 frame-consistency check.
These two reversals are orthogonal and must not be conflated.

### 6.6 Cone Apex Singularity

When the cone solid tapers to a point (no top cap) — **(B-06 resolved)**: full specification follows.

Surface parameterisation: S(u,v) = C + v·tan α · ê(u) + v·Ẑ, v ∈ [0, h].
Apex V\_apex at v=0 (position C), base seam point V\_base at S(0, h) = C + h·(tan α·X̂ + Ẑ).

**Seam edge E\_s**:
- `EDGE_CURVE.edge_start = V_apex`, `EDGE_CURVE.edge_end = V_base`, `EDGE_CURVE.same_sense = .T.`
- 3D curve: LINE, `pnt` = C (apex),
  `dir` = VECTOR(DIRECTION(sin α·X̂ + cos α·Ẑ), magnitude = sec α in the output length unit)
  so P(τ) = C + τ·(tan α·X̂ + Ẑ) = S(0, τ), τ ∈ [0, h]
  Identity-placement example (X̂=(1,0,0), Ẑ=(0,0,1)): `VECTOR(DIRECTION('', (sin α, 0.0, cos α)), sec α)`
- SEAM\_CURVE.associated\_geometry = [P0, Ptau]:
  - P0: 2D LINE pnt=(0, 0), dir=VECTOR(DIRECTION(0,1),1) → q(t)=(0, t)
  - Ptau: 2D LINE pnt=(TAU, 0), dir=VECTOR(DIRECTION(0,1),1) → q(t)=(TAU, t)
  (DIRECTION and VECTOR helper entities shared between P0 and Ptau are bitwise-identical; LINE instances are distinct)

**Base circle E\_b**:
- `EDGE_CURVE.edge_start = V_base = edge_end` (closed), `EDGE_CURVE.same_sense = .T.`
- 3D curve: CIRCLE, centre C+h·Ẑ, radius h·tan α, placement axis Ẑ, ref\_direction X̂
  so C\_b(t) = C + h·Ẑ + h·tan α · ê(t), t ∈ [0, TAU]
- SURFACE\_CURVE.associated\_geometry = [q\_b\_lat, q\_b\_cap]:
  - q\_b\_lat: LINE pcurve on lateral cone surface: `pnt=(0,h)`, `dir=VECTOR(DIRECTION(1,0),1)` → q(t) = (0,h) + t·(1,0) = (t, h). M6 ① at t=0: q=(0,h) ✓.
  - q\_b\_cap: CIRCLE pcurve on base PLANE (in 2D), centre (0,0), radius h·tan α, 2D placement axis Ẑ→(0,1), ref X̂→(1,0) → q(t)=(h·tan α·cos t, h·tan α·sin t)

**Lateral FACE\_OUTER\_BOUND** (ADVANCED\_FACE same\_sense=.T.):
EDGE\_LOOP with exactly 3 ORIENTED\_EDGE items in this order:

```
EDGE_LOOP('', (
  ORIENTED_EDGE('', *, *, #E_s, .T.),   -- item 1: apex→base, uses Ptau (right boundary u=TAU)
  ORIENTED_EDGE('', *, *, #E_b, .F.),   -- item 2: base circle TAU→0
  ORIENTED_EDGE('', *, *, #E_s, .F.)    -- item 3: base→apex, uses P0 (left boundary u=0)
));
```

**Loop closure proof**:
- Item 1 start: E\_s.edge\_start = V\_apex; item 1 end: E\_s.edge\_end = V\_base ✓
- Item 2 start: E\_b.edge\_start = V\_base (.F. reversal starts at edge\_end=V\_base); item 2 end: E\_b.edge\_start = V\_base ✓
- Item 3 start: E\_s.edge\_end = V\_base (.F. reversal starts at edge\_end); item 3 end: E\_s.edge\_start = V\_apex ✓
- Loop closes: item 3 end = item 1 start = V\_apex ✓

**Unwrapped parameter boundary** (positive CCW when viewed from outward normal S_u×S_v):
item 1 follows the right side (u=TAU, v: 0→h); item 2 follows the top (u: TAU→0, v=h);
item 3 follows the left side (u=0, v: h→0). The v=0 side is collapsed to the apex point.

**Base cap face** (ADVANCED\_FACE same\_sense=.T., outward normal = Ẑ away from apex):
- PLANE placement: location = C + h·Ẑ, axis = Ẑ (surface axis direction), ref\_direction = X̂ (surface ref)
- FACE\_OUTER\_BOUND: `EDGE_LOOP('', (ORIENTED_EDGE('', *, *, #E_b, .T.)))`
  (orientation=.T. → circle traverses 0→TAU, CCW viewed from Ẑ ✓)

**Note on E\_b pcurve sense**: q\_b\_lat(t)=(t, h) always points in +u direction. When E\_b is used
with orientation=.F. in item 2 of the lateral loop, the traversal direction is reversed (TAU→0) but
the PCURVE entity itself is not modified. Shared EDGE\_CURVE/SEAM\_CURVE/SURFACE\_CURVE/PCURVE
entities are never reversed per coedge; only the ORIENTED\_EDGE.orientation attribute changes.

No degenerate zero-length edge is introduced. V\_apex participates in exactly two coedges (items 1
and 3); it has no independent circular edge. P-10 and P-11 are unblocked.

---

---

## 6.7 Cone Frustum Canonical Mapping (P-09 / P-12)

A **frustum** is a truncated cone with a small-radius cap (r_small) and a large-radius cap (r_large),
height H along the cone axis (small end toward apex). The canonical writer always outputs R=0 with
the apex placed at C; the frustum becomes a bounded region of the cone's lateral surface.

### 6.7.1 Parameter Derivation

```
alpha   = atan2(r_large − r_small, H)   -- semi-angle; result in (0, π/2)
tan_a   = tan(alpha) = (r_large − r_small) / H
v_min   = r_small / tan_a               -- axial distance from apex to small cap
v_max   = r_large / tan_a = v_min + H   -- axial distance from apex to large cap
```

Both v_min > 0 and v_max > v_min. Apex cone is the limiting case v_min → 0 (3-item loop, §6.6).
Writer placement: apex C, axis Ẑ points from small cap toward large cap (small-to-large orientation).
R=0 always in writer output; non-zero R on import is normalized in step 9 (§9.1).

### 6.7.2 Vertices and Seam Edge

Two vertices: **V_near** at S(0, v_min) and **V_far** at S(0, v_max).

Using placement-frame axes (X̂, Ŷ, Ẑ of AXIS2_PLACEMENT_3D):
- V_near (world): C + v_min·(tan α·X̂ + Ẑ) = C + v_min·tan α·X̂ + v_min·Ẑ
- V_far  (world): C + v_max·(tan α·X̂ + Ẑ) = C + v_max·tan α·X̂ + v_max·Ẑ

**Seam EDGE_CURVE E_s** (generator at u=0, bounded v_min..v_max):
- `EDGE_CURVE.edge_start = V_near`, `EDGE_CURVE.edge_end = V_far`, `same_sense = .T.`
- 3D LINE: `pnt` = V_near (world) = C + v_min·(tan α·X̂ + Ẑ);
  `dir` = VECTOR(DIRECTION(sin α·X̂ + cos α·Ẑ), sec α) — same as apex form (§6.6)
- `VECTOR.magnitude` is a **STEP length measure** in the selected output unit (not dimensionless).
  Wire numeric value = sec α (e.g., ≈1.04403065 mm for a mm file; ≈1.04403065 m for a m file).
  After staging: effective magnitude = sec α · length_factor m/τ (i.e., the staged |A| coefficient).
- **Raw trim τ ∈ [0, Δv_wire]** where Δv_wire = v_max − v_min expressed in the output numeric unit
  (= 100 for mm file, 0.1 for m file). **τ is dimensionless and is never staged**; its numeric
  interval is the same before and after staging. The staged 3D effective direction coefficient is
  A_staged = length_factor · (tan α·X̂ + Ẑ) m/τ; the staged pcurve b = length_factor m/τ.

**SEAM_CURVE** associated_geometry = [P0_lat, Ptau_lat] (exactly 2 PCURVEs on the same surface; §6.4/§6.3 WHERE rules apply):
- P0_lat (at u=0): 2D LINE `pnt = (0, v_min)`, `dir = VECTOR(DIRECTION(0,1), 1)` → q(τ) = (0, v_min + τ)
- Ptau_lat (at u=TAU): 2D LINE `pnt = (TAU, v_min)`, `dir = VECTOR(DIRECTION(0,1), 1)` → q(τ) = (TAU, v_min + τ)

M3 checks (applied independently to P0_lat and Ptau_lat; M7 checks seam pair): v₀=v_min, raw b=1, staged b=length_factor.
For P0_lat (uᵢ=0): ① |P₀ − S(0,v_min)| ≤ ε_l; ② |A − b·(tan α·ê(0)+Ẑ)| ≤ ε_l; ③ u=0±ε_p ✓.
For Ptau_lat (uᵢ=TAU): same checks with ê(TAU)=ê(0)=X̂; ③ u=TAU±ε_p ✓.

### 6.7.3 Near and Far Cap Circles

**E_near** — closed CIRCLE on small cap (v = v_min):
- `edge_start = edge_end = V_near`, `same_sense = .T.`
- 3D CIRCLE: centre C + v_min·Ẑ, radius v_min·tan α, placement axis Ẑ, ref X̂
  → C_near(t) = C + v_min·Ẑ + v_min·tan α · ê(t), t ∈ [0, TAU]
- SURFACE_CURVE associated_geometry = [q_near_lat, q_near_cap]:
  - q_near_lat: LINE on lateral cone: `pnt = (0, v_min)`, `dir = VECTOR(DIRECTION(1,0), 1)` → q(t) = (t, v_min)
  - q_near_cap: CIRCLE on near cap PLANE: centre (0,0), radius v_min·tan α, 2D axis Ẑ→(0,1), ref X̂→(1,0)
    → q(t) = (v_min·tan α · cos t, v_min·tan α · sin t)

**E_far** — closed CIRCLE on large cap (v = v_max):
- `edge_start = edge_end = V_far`, `same_sense = .T.`
- 3D CIRCLE: centre C + v_max·Ẑ, radius v_max·tan α, placement axis Ẑ, ref X̂
  → C_far(t) = C + v_max·Ẑ + v_max·tan α · ê(t), t ∈ [0, TAU]
- SURFACE_CURVE associated_geometry = [q_far_lat, q_far_cap]:
  - q_far_lat: LINE on lateral cone: `pnt = (0, v_max)`, `dir = VECTOR(DIRECTION(1,0), 1)` → q(t) = (t, v_max)
  - q_far_cap: CIRCLE on far cap PLANE: centre (0,0), radius v_max·tan α, 2D axis Ẑ→(0,1), ref X̂→(1,0)
    → q(t) = (v_max·tan α · cos t, v_max·tan α · sin t)

**M6** checks each of E_near/E_far against its lateral CONICAL_SURFACE LINE pcurve q=(t,v_const): ① Oc=C+v_const·Ẑ ✓; ② radius=v_const·tan α ✓; ③ frame ✓.
**M4** checks each of E_near/E_far against its cap PLANE CIRCLE pcurve: ①–③ centre+frame ✓.

### 6.7.4 Lateral Face (4-Item Loop)

```
EDGE_LOOP('', (
  ORIENTED_EDGE('', *, *, #E_s,    .T.),   -- item 1: V_near→V_far, uses Ptau_lat (right boundary u=TAU)
  ORIENTED_EDGE('', *, *, #E_far,  .F.),   -- item 2: V_far→V_far (circle TAU→0, top boundary)
  ORIENTED_EDGE('', *, *, #E_s,    .F.),   -- item 3: V_far→V_near, uses P0_lat (left boundary u=0)
  ORIENTED_EDGE('', *, *, #E_near, .T.)    -- item 4: V_near→V_near (circle 0→TAU, bottom boundary)
));
```

**Loop closure proof** (frustum with v_min > 0):
- Item 1 (.T.): start = E_s.edge_start = V_near; end = E_s.edge_end = V_far ✓
- Item 2 (.F.): reverse of E_far → start = E_far.edge_end = V_far; end = E_far.edge_start = V_far ✓
- Item 3 (.F.): reverse of E_s → start = E_s.edge_end = V_far; end = E_s.edge_start = V_near ✓
- Item 4 (.T.): start = E_near.edge_start = V_near; end = E_near.edge_end = V_near ✓
- Loop closes: item 4 end = item 1 start = V_near ✓

**PCURVE mapping** (consistent with §6.6 and §6.4): orientation=.T. on E_s means the 3D LINE is
traversed in its natural direction (V_near→V_far); the SEAM_CURVE resolution picks the PCURVE whose
u-parameter matches the coedge's boundary side. By the convention established in §6.6:
- `E_s .T.` → Ptau_lat (u=TAU), right boundary, traversal near→far (item 1).
- `E_s .F.` → P0_lat (u=0), left boundary, traversal far→near (item 3).
Both PCURVEs cover the full seam; one shared 3D LINE, two distinct PCURVEs.

**Unwrapped parameter boundary** (positive CCW viewed from outward normal S_u×S_v):
- Item 1 (.T.): right (u=TAU, v: v_min→v_max)
- Item 2 (.F.): top (u: TAU→0, v=v_max)  [.F. reverses 0→TAU to TAU→0]
- Item 3 (.F.): left (u=0, v: v_max→v_min)  [.F. reverses near→far to far→near]
- Item 4 (.T.): bottom (u: 0→TAU, v=v_min)

Standard counter-clockwise traversal of the rectangle [0,TAU]×[v_min,v_max] in (u,v) space ✓.

### 6.7.5 Cap Faces

**Far cap** (large circle, outward normal = Ẑ away from apex):
- PLANE placement: location = C + v_max·Ẑ, axis = Ẑ, ref_direction = X̂
- `ADVANCED_FACE.same_sense = .T.` (surface normal Ẑ is outward)
- FACE_OUTER_BOUND: `EDGE_LOOP('', (ORIENTED_EDGE('', *, *, #E_far, .T.)))`
  → E_far traverses 0→TAU; CCW viewed from Ẑ (outward) ✓

**Near cap** (small circle, outward normal = −Ẑ toward apex):
- PLANE placement: location = C + v_min·Ẑ, axis = Ẑ, ref_direction = X̂
  (same direction Ẑ as far cap plane; surface normal Ẑ points inward for this face)
- `ADVANCED_FACE.same_sense = .F.` (surface normal −Ẑ is outward, i.e. toward apex)
- FACE_OUTER_BOUND orientation = .T. (FACE_OUTER_BOUND.orientation = .T. marks this as the outer bound)
- `EDGE_LOOP('', (ORIENTED_EDGE('', *, *, #E_near, .F.)))`
  → E_near traverses TAU→0; CCW viewed from −Ẑ (outward toward apex) ✓

Proof of near-cap closure: E_near is closed (edge_start = edge_end = V_near); the loop is trivially closed ✓.

### 6.7.6 M-Matrix Applicability

Each SURFACE_CURVE/SEAM_CURVE has two PCURVEs; both must pass their respective row.

| Check | Edge + pcurve type | Applies |
|---|---|---|
| M3 × 2 | E_s: P0_lat (uᵢ=0) and Ptau_lat (uᵢ=TAU) on CONICAL_SURFACE | Apply M3 independently to each seam pcurve |
| M7 | E_s seam pair (P0_lat + Ptau_lat) | Same surface, same v₀/b/trim; u₀=0±ε_p, uτ=TAU±ε_p |
| M6 | E_near/E_far: lateral CONICAL_SURFACE LINE pcurves q=(t, v_const) | CIRCLE on CONE: checks ①–③ (§6.3 M6) |
| M4 | E_near/E_far: cap PLANE CIRCLE pcurves q=(v·tan α·cos t, v·tan α·sin t) | CIRCLE on PLANE: checks ①–③ (§6.3 M4) |

M5 (CIRCLE on CYLINDER) is **not applicable** to a pure frustum (no CYLINDRICAL_SURFACE).

### 6.7.7 Exact Wire Values for P-09 and P-12

**P-09 — metre file**, r_small=0.02 m, r_large=0.05 m, H=0.10 m:
```
tan_a = (0.05 − 0.02) / 0.10 = 0.3000000000 exactly
alpha = atan(0.3) ≈ 0.29145679...
v_min = 0.02 / 0.3  = 1/15 m ≈ 0.06666666... m
v_max = 0.05 / 0.3  = 1/6  m ≈ 0.16666666... m
raw_tau_upper_numeric = v_max − v_min = 0.1  -- τ ∈ [0, 0.1], dimensionless
sec_a = sqrt(1 + 0.09) = sqrt(1.09) ≈ 1.04403065...
V_near world (identity placement): (v_min*tan_a, 0, v_min) = (0.02, 0, 1/15) m
V_far  world (identity placement): (v_max*tan_a, 0, v_max) = (0.05, 0, 1/6)  m
```
VECTOR.magnitude wire value = sec_a ≈ 1.04403065 m (a length measure in the output unit, metres here).
Staged b = length_factor = 1.0 m/τ; staged |A| = sec_a × 1.0 = sec_a m/τ.
Coefficient check: τ_max × |A_staged| = 0.1 × sec_a ≈ 0.10440 m (arc length of seam).
UNCERTAINTY = 1.0E-7 m (metric file).

**P-12 — mm file**, same geometry in millimetres (wire numeric unit = mm):
```
v_min = 200/3 mm ≈ 66.66666... mm; v_max = 500/3 mm ≈ 166.66666... mm
raw_tau_upper_numeric = 100  -- τ ∈ [0, 100], dimensionless; wire numeric = 100 mm, NOT staged to 0.1
length_factor = 1.0E-3
VECTOR.magnitude wire value = sec_a ≈ 1.04403065 mm (length measure in mm, the output unit)
staged b = 1.0E-3 m/τ; staged |A| = sec_a × 1.0E-3 m/τ
coefficient check: τ_max × |A_staged| = 100 × 1.0E-3 × sec_a ≈ 0.10440 m (same arc length as P-09 ✓)
internal s at far vertex: s = (100 − 0) × sec_a × 1.0E-3 ≈ 0.10440 m
UNCERTAINTY = 1.0E-4 mm (mm file per §7)
```

---

## 7. Units, Tolerance, and Representation Context

### 7.1 Internal and Exchange Units

Amphion's canonical model-space unit is the **metre** (CONTRACTS.md §Model space). Exchange files may
use a different unit; conversion occurs only at import/export boundaries.

**Writer default**: **millimetre** (most prevalent in industry STEP files).

| Quantity | Exchange unit | STEP encoding |
|---|---|---|
| Length (mm) | millimetre | CONVERSION\_BASED\_UNIT referencing SI\_UNIT($, .METRE.) with factor 1.0E-3 |
| Length (m) | metre | SI\_UNIT($, .METRE.) |
| Plane angle | radian | SI\_UNIT($, .RADIAN.) |
| Solid angle | steradian | SI\_UNIT($, .STERADIAN.) |

Millimetre unit encoding (complex entity):

```
#dim_mm = DIMENSIONAL_EXPONENTS(1.,0.,0.,0.,0.,0.,0.);
#si_m   = SI_UNIT($, .METRE.);
#mm_val = LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.0E-3), #si_m);
#len_unit = (
  CONVERSION_BASED_UNIT('MILLIMETRE', #mm_val)
  LENGTH_UNIT()
  NAMED_UNIT(#dim_mm)  /* explicit DIMENSIONAL_EXPONENTS valid for CONVERSION_BASED_UNIT */
);
```

Metre unit encoding (complex entity):

```
/* SI_UNIT overrides NAMED_UNIT.dimensions (it is DERIVE in EXPRESS);
   Part 21 requires '*' for derived attributes in complex entities */
#len_unit = (
  LENGTH_UNIT()
  NAMED_UNIT(*)
  SI_UNIT($, .METRE.)
);
```

Radian and steradian units:

```
#ang_unit = (
  NAMED_UNIT(*)
  PLANE_ANGLE_UNIT()
  SI_UNIT($, .RADIAN.)
);
#sang_unit = (
  NAMED_UNIT(*)
  SI_UNIT($, .STERADIAN.)
  SOLID_ANGLE_UNIT()
);
```

Import accepts any linear unit expressible as a finite positive conversion factor to metres. Import rejects
angle units other than radian or degree (degree → radian conversion applied internally).

### 7.2 Global Uncertainty

One UNCERTAINTY\_MEASURE\_WITH\_UNIT per file, inside the combined context entity:

**Metre file** (value in metres, against the metre unit):
```
#uncert = UNCERTAINTY_MEASURE_WITH_UNIT(
  LENGTH_MEASURE(1.0E-7),
  #len_unit,
  'distance_accuracy_value',
  'Maximum model space distance'
);
```

**Millimetre file** (value in millimetres, against the millimetre unit):
```
#uncert = UNCERTAINTY_MEASURE_WITH_UNIT(
  LENGTH_MEASURE(1.0E-4),
  #len_unit,
  'distance_accuracy_value',
  'Maximum model space distance'
);
```

Default: 1.0E-7 m (100 nm). In a metre file, emit `1.0E-7` against the metre `#len_unit`.
In a millimetre file, emit `1.0E-4` against the millimetre `#len_unit` (1.0E-4 mm = 1.0E-7 m).
The numeric value must be expressed in the **declared file unit**, not in metres.
Round-trip assertion: a body written in mm then read back must yield the same SI uncertainty value as
a body written in metres (both 1.0E-7 m after unit conversion).

**Import contract**: the `UNCERTAINTY_MEASURE_WITH_UNIT` value is extracted and stored as provenance
metadata on the imported Body. It is **not** used as Amphion's operational `ToleranceContext`; the caller
supplies tolerances independently.

### 7.3 PLANE\_ANGLE\_UNIT and SOLID\_ANGLE\_UNIT

**Strict policy for files containing CONICAL\_SURFACE (B-42)**: if any `CONICAL_SURFACE` entity is
present in the data section, the file **must** supply exactly one resolvable `PLANE_ANGLE_UNIT` in
`GLOBAL_UNIT_ASSIGNED_CONTEXT`. Supported angle units:

| Unit | Recognised form | angle\_factor |
|---|---|---|
| Radian | `(NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.))` | 1.0 |
| Degree | `CONVERSION_BASED_UNIT('DEGREE',...)` resolving to π/180 rad | π/180 |

**Cone-present failure cases** (→ `STEP_INVALID_UNIT`, no Body):
- resolved\_count = 0: no `PLANE_ANGLE_UNIT` found in context.
- resolved\_count > 1: multiple angle-unit members (even numerically equivalent; deterministic selection
  is explicitly forbidden — the file must contain exactly one).
- Angle unit present but not radian or degree: unsupported kind → `STEP_INVALID_UNIT` regardless of count.

**Cone-free files** (PLANE/CYLINDER only, no CONICAL\_SURFACE):
- resolved\_count = 0: deterministic non-fatal **warning**; `angle_factor` defaults to 1.0 (radians).
  Implicit pcurve u-coordinates on CYLINDER remain canonical radians per §6.3 dimension table.
- resolved\_count = 1: use it (no warning).
- resolved\_count > 1 or unsupported kind: `STEP_INVALID_UNIT` (ambiguity is never silently resolved).

**SOLID\_ANGLE\_UNIT**: absence is always a warning (not an error) in v0; steradian unit optional.

### 7.4 P-curve Context (DEFINITIONAL\_REPRESENTATION `context_of_items`)

`DEFINITIONAL_REPRESENTATION.context_of_items` is a required `REPRESENTATION_CONTEXT` (inherited from
REPRESENTATION supertype in Part 43; cannot be `$`). For 2D parametric p-curve geometry, Amphion writes
a single shared complex entity:

```
#p2d_ctx = (
  GEOMETRIC_REPRESENTATION_CONTEXT(2)
  PARAMETRIC_REPRESENTATION_CONTEXT()
  REPRESENTATION_CONTEXT('2D ACME', 'PARAMETER_SPACE')
);
```

All DEFINITIONAL\_REPRESENTATION instances in the file share the same `#p2d_ctx` instance.

Import requires `context_of_items` to contain `PARAMETRIC_REPRESENTATION_CONTEXT` (by WHERE rule
confirmed from ISO 10303-42:2021 public SMRL). The complex entity must have coordinate space dimension 2
(from `GEOMETRIC_REPRESENTATION_CONTEXT(2)`). Import rejects a `DEFINITIONAL_REPRESENTATION` whose
`context_of_items` is `$` or lacks `PARAMETRIC_REPRESENTATION_CONTEXT` with `STEP_MISSING_REQUIRED_ENTITY`.

---

## 8. IDs, Names, Schema Version, and Product Structure Rules

### 8.1 Entity Instance IDs

- Assigned deterministically by the writer (Section 3.4 rule 1).
- Import does not require contiguous or sorted IDs.
- Duplicate IDs: `STEP_DUPLICATE_INSTANCE_ID` error.
- Unresolved forward reference: `STEP_UNRESOLVED_REFERENCE` error.
- Self-referential instance (an entity referencing its own ID): `STEP_CIRCULAR_REFERENCE` error.

### 8.2 Name Attributes

Amphion `SemanticId` and provenance exist on topology entities (`Body`, `Region`, `Shell`, `Face`, `Loop`,
`Coedge`, `Edge`, `Vertex`) via their `Provenance` field. Geometry evaluator objects (`SurfaceId`,
`Curve3Id`, `Curve2Id`) carry local IDs but no `SemanticId` in the current contracts.

STEP entity name attributes are therefore derived as follows:

| Entity | Name source |
|---|---|
| `MANIFOLD_SOLID_BREP` | `Body.provenance.semantic_id` |
| `ADVANCED_BREP_SHAPE_REPRESENTATION` | `Region.provenance.semantic_id` (v0 one implicit Region per Body) |
| `CLOSED_SHELL` | Outer `Shell.provenance.semantic_id` |
| `ADVANCED_FACE` | `Face.provenance.semantic_id` |
| `VERTEX_POINT` | `Vertex.provenance.semantic_id` |
| `EDGE_CURVE` | `Edge.provenance.semantic_id` |
| `PLANE`, `CYLINDRICAL_SURFACE`, `CONICAL_SURFACE` | Empty string `''` (derived from associated Face on round-trip; no independent semantic ID) |
| `LINE`, `CIRCLE` (3D) | Empty string `''` (derived from associated Edge on round-trip) |
| `CARTESIAN_POINT`, `DIRECTION`, `VECTOR`, `AXIS2_PLACEMENT_3D`, `AXIS2_PLACEMENT_2D` | Empty string `''` |
| `PCURVE`, `DEFINITIONAL_REPRESENTATION`, `SURFACE_CURVE`, `SEAM_CURVE` | Empty string `''` |

**Amphion SemanticId extension** — versioned ASCII scheme:

Format: `amphion.id/1/<32-lowercase-hex-digits>` (prefix + version + 128-bit identity, **45 chars total**: `len("amphion.id/1/")=13` + 32 hex digits = 45).
Example: `amphion.id/1/4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d`

Parse rule: the name attribute is treated as a SemanticId **only** if it exactly matches the regex
`^amphion\.id/1/[0-9a-f]{32}$`. Any other name string is display-only metadata; it is **never**
accidentally promoted to a SemanticId.

**Entity → STEP name attribute mapping** (writer):

| Amphion entity | STEP name attribute | ID source |
|---|---|---|
| `Body` | `MANIFOLD_SOLID_BREP.name` | `Body.provenance.semantic_id` formatted as extension |
| `Region` | `ADVANCED_BREP_SHAPE_REPRESENTATION.name` | `Region.provenance.semantic_id` formatted as extension (no alternate source) |
| outer `Shell` | `CLOSED_SHELL.name` | `Shell.provenance.semantic_id` |
| `Face` | `ADVANCED_FACE.name` | `Face.provenance.semantic_id` |
| `Loop` | `EDGE_LOOP.name` | `Loop.provenance.semantic_id` |
| `Coedge` | `ORIENTED_EDGE.name` | `Coedge.provenance.semantic_id` |
| `Edge` | `EDGE_CURVE.name` | `Edge.provenance.semantic_id` |
| `Vertex` | `VERTEX_POINT.name` | `Vertex.provenance.semantic_id` |
| Geometry, placement helpers | empty string `''` | No SemanticId on geometry evaluators |

**Import: Region SemanticId reconstruction** — on import, `ADVANCED_BREP_SHAPE_REPRESENTATION.name`
is parsed with the regex `^amphion\.id/1/[0-9a-f]{32}$`. If it matches, the 45-char string becomes
the Region SemanticId. If absent or non-matching, a synthetic ID is derived as for other entities.
No alternate Region source is used.

**Synthetic ID derivation** (for foreign files without the extension):
sha256(`<entity-type>:<structural-path>`) where structural-path is the deterministic
index-path of the entity within the B-Rep hierarchy (e.g. `Shell:0/Face:2/Loop:0/Coedge:1`).
Truncate to 16 bytes (32 hex chars).

**Collision**: two entities with the same parsed `amphion.id/1/<hex>` ID produce `STEP_DUPLICATE_SEMANTIC_ID`.
**Escaping**: `amphion.id/1/<hex>` contains only printable ASCII; no escaping is needed under level `'2;1'`.
**Version**: the `/1/` segment is a format version; unknown versions are treated as display-only metadata.

Name strings must not contain embedded NUL characters or ASCII control codes. v0 writer restriction:
all names are empty strings or printable 7-bit ASCII (Section 3.2).

### 8.3 APPLICATION\_PROTOCOL\_DEFINITION

```
#apd = APPLICATION_PROTOCOL_DEFINITION(
  'international standard',
  'ap242_managed_model_based_3d_engineering_mim_lf',
  2020,
  #app_ctx
);
```

**(B-02 resolved)** Two immutable fixtures have valid APD tuples (Onshape year-2020 + NIST year-2011); CATIA is OID-only evidence (B-14); OCCT is header-only. Summary:

| Fixture | APD name | year | FILE\_SCHEMA OID | Source |
|---|---|---|---|---|
| CATIA-generated assembly (fpb\_assy\_v3.step, commit `bcc76aeb`) | **OID evidence only** — APD entity uses `$` for required `application` attribute; schema-invalid APD; `_mim_lf` name and year=2014 observed in attribute text but not from a conformant APD entity | **lexical** | `{1 0 10303 442 1 1 4}` | https://raw.githubusercontent.com/AmedeoPelliccia/Robbbo-T_OLD/bcc76aeb5f612a7ead0567ebd49054c2e8166e48/C-AMEDEO-FRAMEWORK/CA-DEOPTIMISE/CAD-DESIGN/H2-BWB-Q100-CONF0000/AAA-ARCHITECTURES_AIRFRAMES_AERODYNAMICS/CE-CAD-Q100-AAA-ATA-53-FUSELAGE/CC/CE-CC-CAD-Q100-AAA-ATA-53-10-STRUCTURE-1/CI/CE-CC-CI-CAD-Q100-AAA-ATA-53-10-01-COMPONENT-1/3DModels/fpb_assy_v3.step |
| Onshape/ST-Developer export (cam\_wedge\_15.step, commit `51ca0ec`) | `ap242_managed_model_based_3d_engineering` (no `_mim_lf`), valid APD | **2020** | `{1 0 10303 442 3 1 4}` | https://raw.githubusercontent.com/Vector-Wangel/XLeRobot/51ca0ec31bdb48713b94bacdba828bf8d889296b/hardware/misc/cam_wedge_15.step |
| NIST/GUID-data (box-guid.stp, commit `e0dbd5e`) | `ap242_managed_model_based_3d_engineering_mim_lf`, valid APD, no FILE\_SCHEMA OID | **2011** | bare | https://raw.githubusercontent.com/allisonfeeney/guid-data/e0dbd5ecd0e972105ba4a4d99858d93ae674ba48/box_model/box-guid.stp |
| OCCT PMI test (bug32745\_pmi1.stp, existing citation) | no APD entity | — | `{1 0 10303 442 2 1 4}` | header-only reference |

**Conclusion**: APD `year` is NOT an AP edition discriminator in observed practice (2011/2014/2020 all
appear in files (Onshape: 2020; NIST: 2011; CATIA lexical: 2014). The `_mim_lf` suffix appears in
NIST and CATIA-class files; Onshape omits it. The CATIA APD entity is schema-invalid (B-14: downgraded
to OID-only evidence). Year=2014 is observed in CATIA attribute text only, not from a valid APD entity.
Public EXPRESS source (`SwiftSDAIap242` rule) constrains the MIM-LF schema name but does not normatively
mandate APD year.

**Amphion writer policy** (frozen as explicit interoperability profile, not normative mandate):
- APD name: `'ap242_managed_model_based_3d_engineering_mim_lf'`
- APD year: `2020` (reflects most recently observed conformant practice)
- No alternative named profiles exist in v0 canonical output; year 2014 is import evidence only

**Import policy**: accept APD name with or without `_mim_lf` suffix (both observed in valid files); accept year 2011, 2014, 2020;
do not use year as edition discriminator; store as provenance metadata only.

**FILE\_SCHEMA writer**: `FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'))` (bare form).

**FILE\_SCHEMA import whitelist** (verified OIDs only; never arbitrary suffix):
- Bare: `'AP242_MANAGED_MODEL_BASED_3D_ENGINEERING_MIM_LF'`
- Bare (no `_mim_lf`): `'AP242_MANAGED_MODEL_BASED_3D_ENGINEERING'`
- OID `{1 0 10303 442 1 1 4}`: CATIA-class exporters
- OID `{1 0 10303 442 2 1 4}`: OCCT-class exporters
- OID `{1 0 10303 442 3 1 4}`: Onshape/ST-Developer exporters

Any other FILE\_SCHEMA produces `STEP_UNSUPPORTED_SCHEMA`.

### 8.4 Minimal Product Structure

Every Amphion-written file contains exactly the entities in Section 4.1, linked as follows:

```
APPLICATION_CONTEXT ('mechanical design')
  ↑ referenced by APPLICATION_PROTOCOL_DEFINITION
  ↑ referenced by PRODUCT_CONTEXT (via PRODUCT)
  ↑ referenced by PRODUCT_DEFINITION_CONTEXT

PRODUCT (id, name, '', {PRODUCT_CONTEXT})
  ↑ referenced by PRODUCT_DEFINITION_FORMATION

PRODUCT_DEFINITION_FORMATION ('', '', PRODUCT)
  ↑ referenced by PRODUCT_DEFINITION

PRODUCT_DEFINITION_CONTEXT ('part definition', APPLICATION_CONTEXT, 'design')
  ↑ referenced by PRODUCT_DEFINITION

PRODUCT_DEFINITION ('design', '', PRODUCT_DEFINITION_FORMATION, PRODUCT_DEFINITION_CONTEXT)
  ↑ referenced by PRODUCT_DEFINITION_SHAPE

PRODUCT_DEFINITION_SHAPE ('', '', PRODUCT_DEFINITION)
  ↓
SHAPE_DEFINITION_REPRESENTATION (PRODUCT_DEFINITION_SHAPE, ADVANCED_BREP_SHAPE_REPRESENTATION)
```

---

## 9. Import Validation, Unsupported Entities, and Failure Semantics

### 9.1 Validation Pipeline

Steps execute in order; each must succeed before the next begins.

1. **Lexing** — validate byte stream against declared level (level `'2;1'`: 7-bit ASCII + escape sequences
   only; raw multi-byte UTF-8 is `STEP_ENCODING_ERROR`); tokenise; record source byte offsets for
   diagnostics.
2. **Header parse** — validate FILE\_DESCRIPTION implementation level; extract FILE\_SCHEMA; reject
   unsupported schemas and unsupported levels immediately.
3. **Entity graph construction** — parse all DATA instances; reject duplicate IDs; resolve all `#ref`
   references (forward and backward); reject unresolved references.
4. **Type check** — verify each entity's attribute types against the EXPRESS schema for allowlisted
   entities.
5. **Allowlist check** — every entity instance must appear in Section 4. Unknown entities fail the
   transaction (Section 9.2).
6. **Unit extraction** — resolve each unit in GLOBAL\_UNIT\_ASSIGNED\_CONTEXT recursively:
   - `SI_UNIT(prefix, METRE)`: length\_factor = SI prefix multiplier (e.g., MILLI→1e-3, none→1.0).
   - `CONVERSION_BASED_UNIT(name, MEASURE_WITH_UNIT(factor, base_unit))`: length\_factor = factor × resolve(base\_unit) recursively. **Never multiply the unit-definition factor by the global length\_factor it defines** (avoid double-scaling; 1 mm = 0.001 × resolve(METRE) = 0.001 m, not 0.001 × 0.001 m).
   - Detect: cycle in unit graph → `STEP_INVALID_UNIT`; factor ≤ 0 or non-finite → `STEP_INVALID_UNIT`.
   - `PLANE_ANGLE_UNIT(RADIAN)`: angle\_factor = 1.0; `PLANE_ANGLE_UNIT(DEGREE)`: angle\_factor = π/180.
   - **Angle-unit cardinality check (B-42/B-44)** — execute after all unit resolutions above:
     1. Collect the list of all `PLANE_ANGLE_UNIT` members from `GLOBAL_UNIT_ASSIGNED_CONTEXT`.
     2. For each member: attempt resolution (radian → factor 1.0; degree → factor π/180; other →
        record as unsupported). Track resolved\_count (supported) and unsupported\_count separately.
     3. If unsupported\_count > 0 → `STEP_INVALID_UNIT` (always, regardless of cone presence).
     4. Let cone\_present = (any `CONICAL_SURFACE` in entity graph):
        - cone\_present and resolved\_count ≠ 1 → `STEP_INVALID_UNIT` (0 = missing; >1 = ambiguous;
          deterministic selection of one from many is **forbidden**).
        - not cone\_present and resolved\_count = 0 → emit deterministic structured warning; angle\_factor = 1.0.
        - not cone\_present and resolved\_count = 1 → use it; no warning.
        - not cone\_present and resolved\_count > 1 → `STEP_INVALID_UNIT` (ambiguity is never silent).
        - cone\_present and resolved\_count = 1 → angle\_factor = resolved value; proceed.
   - Store length\_factor and angle\_factor for the staging step below.

7. **Attribute staging** (B-10/B-15 — must precede domain validation): for every allowlisted entity,
   convert REAL-bearing attributes using the dimension table below. `CONVERSION_BASED_UNIT` and
   `MEASURE_WITH_UNIT` contain unit-definition data only and must **not** be passed through this step.

   **Per-attribute dimension table** (B-15):

   | Entity | Attribute | Dimension | Factor |
   |---|---|---|---|
   | `CARTESIAN_POINT` (3D model-space) | each coordinate | length | ×length\_factor |
   | `DIRECTION` (any) | each direction\_ratio | dimensionless | ×1 |
   | `VECTOR` | magnitude | length | ×length\_factor |
   | `AXIS2_PLACEMENT_3D` | location.coordinates | length | via CARTESIAN\_POINT |
   | `LINE` (3D) | pnt.coordinates | length | via CARTESIAN\_POINT |
   | `LINE` (3D) | dir effective magnitude | length | via VECTOR.magnitude |
   | `CIRCLE` (3D) | radius | length | ×length\_factor |
   | `CYLINDRICAL_SURFACE` | radius | length | ×length\_factor |
   | `CONICAL_SURFACE` | radius | length | ×length\_factor |
   | `CONICAL_SURFACE` | semi\_angle | angular | ×angle\_factor |
   | `UNCERTAINTY_MEASURE_WITH_UNIT` | value\_component | length | ×length\_factor |
   | 2D `CARTESIAN_POINT` on PLANE | each coordinate (u,v) | length (both) | ×length\_factor each |
   | 2D `CARTESIAN_POINT` on CYLINDER/CONE | first coord (u) | angular | ×1 (radians; NO length scale) |
   | 2D `CARTESIAN_POINT` on CYLINDER/CONE | second coord (v) | length | ×length\_factor |
   | 2D `VECTOR` on PLANE | magnitude | length | ×length\_factor |
   | 2D `VECTOR` on CYLINDER/CONE | — | **component-wise, normalise first** | normalize raw `(du,dv)` ratios: d̂=(du,dv)/‖(du,dv)‖ (‖(du,dv)‖=0 → `STEP_INVALID_DIRECTION`); compute typed effective components eff_u=mag×d̂.u×1 (angular), eff_v=mag×d̂.v×length\_factor (length); rebuild canonical typed vector from (eff_u, eff_v) |
   | 2D `CIRCLE` (on PLANE pcurve) | radius | length | ×length\_factor |
   | 2D `AXIS2_PLACEMENT_2D` on PLANE | location.coordinates | length | ×length\_factor each |
   | 2D `AXIS2_PLACEMENT_2D` on CYLINDER/CONE | location.u | angular | ×1 |
   | 2D `AXIS2_PLACEMENT_2D` on CYLINDER/CONE | location.v | length | ×length\_factor |

   Per-value checks: overflow (result > f64::MAX) → `STEP_UNIT_OVERFLOW`; nonzero→zero underflow
   → `STEP_UNIT_UNDERFLOW`. Apply to each component independently.

   **Raw curve parameter τ** (B-21): the raw STEP LINE/CIRCLE parameter τ is dimensionless.
   A 3D LINE P(τ) = pnt + τ×A (A in metres after staging) has metres/τ units for A; the pcurve
   q(τ) = pnt2d + τ×A2d has rad/τ for the u-component and metres/τ for the v-component (cylinder/cone).
   The identity matrix compares using the same raw τ. After validated identity, the Amphion internal
   length parameter s (in metres) is: s = τ × |A| for a LINE.

   **Per-PCURVE-occurrence staging** (B-21): 2D geometry entities inside DEFINITIONAL\_REPRESENTATION
   are staged per PCURVE occurrence, NOT by mutating the shared raw entity. If the same raw 2D helper
   is referenced by two PCURVEs whose basis surfaces have different parameter-space dimensions (e.g., one
   PLANE and one CYLINDRICAL\_SURFACE), create independent occurrence-typed internal values for each.
   Sharing at the raw IR level is transparent to staging; the resulting typed values are per-occurrence.
   Writer interning (Phase C) operates only on typed output values after conversion.

   **CONVERSION\_BASED\_UNIT exclusion**: `CONVERSION_BASED_UNIT.conversion_factor` and the inner
   `MEASURE_WITH_UNIT.value_component` are unit-graph data consumed in step 6; they must not be
   treated as model-space attributes and must not receive the global scale factor.

8. **Numeric validation** — all values now in metres and radians:
   - Radius (CIRCLE, CYLINDRICAL\_SURFACE): strictly > 0; `STEP_INVALID_PARAMETER` otherwise.
   - **CONICAL\_SURFACE radius R**: R < 0 → `STEP_INVALID_PARAMETER` (step 9 requires R ≥ 0);
     R = 0 → accepted as apex form; R > 0 → accepted here, normalized to apex in step 9.
   - `semi_angle` ∈ (0, π/2) radians; `STEP_INVALID_PARAMETER` for ≤ 0 or ≥ π/2.
   - VECTOR magnitude > 0; direction vector non-zero.
   - Uncertainty value > 0.

9. **Cone R>0 normalisation** (B-16/B-27/B-40 — must precede trim recovery and M3/M6 checks): for every
   CONICAL\_SURFACE with radius R > 0 (values already in metres from step 7; semi\_angle α validated in
   step 8 so tan α > 0 and finite):

   **Immutability constraint (B-40)**: the parsed raw entity graph is **never mutated**. A
   CONICAL\_SURFACE may share its AXIS2\_PLACEMENT\_3D (and its CARTESIAN\_POINT/DIRECTION helpers)
   with a PLANE, another CYLINDRICAL\_SURFACE, or any other entity. Mutating the shared placement
   would silently move those other surfaces. Step 9 therefore allocates **new occurrence-local
   canonical IR nodes** and rebinds references on the cone occurrence only; all raw/shared entities
   remain byte-identical.

   **Transactional semantics**: compute ALL transformed values using certified interval arithmetic
   before allocating or rebinding any new node; if any arithmetic check fails, raise the error and
   leave the entire IR (raw and canonical) unchanged.

   a. **shift = R / tan(α)** — if result is nonfinite → `STEP_UNIT_OVERFLOW`.
   b. **C' components**: for each component cᵢ of C and shift·Ẑᵢ compute cᵢ − shift·Ẑᵢ;
      check finite → `STEP_UNIT_OVERFLOW`; check no catastrophic cancellation (both inputs nonzero
      but result zero) → `STEP_UNIT_UNDERFLOW`.
   c. **For every occurrence-local PCURVE on this cone** — compute new values, do NOT allocate yet:
      - Seam LINE pcurve pnt.v: new\_v = pnt.v + shift; check finite/underflow.
      - Cap circle pcurve v-height h: new\_h = h + shift; same checks.
      - 2D CIRCLE pcurve on cap PLANE: unchanged (plane's own frame origin).
   d. If all certified: **allocate new canonical nodes atomically**:
      - New CARTESIAN\_POINT at C'; new AXIS2\_PLACEMENT\_3D reusing original DIRECTION helpers
        but pointing to new CARTESIAN\_POINT (Ẑ/X̂ unchanged).
      - New CONICAL\_SURFACE with R=0, pointing to new placement (semi\_angle unchanged).
      - For each PCURVE: new occurrence-local 2D CARTESIAN\_POINT (new pnt.v or new h), new 2D LINE
        rebound to new point, new DEFINITIONAL\_REPRESENTATION wrapping new LINE (original
        DIRECTION/VECTOR helpers shared if bitwise-identical — see Phase B interning).
      - Rebind the cone occurrence and its PCURVE occurrences to new nodes; raw graph untouched.
      - New canonical nodes participate in Phase A occurrence-path assignment and are internable via
        Phase B/C (writer re-interns them; structurally identical clones collapse to one instance).

   After normalisation, all cone v-parameters ≥ 0 (apex at v=0); validate v\_start ≥ 0.
   M3 and M6 checks then operate exclusively on R=0 apex form.
   **Trim parameter τ values are NOT modified in step 9** — recovered from 3D vertices in step 12.

   **P-RP-01 update**: the positive R≠0 reparameterisation fixture (R=0.03 m, α=π/6, h=0.1 m) must
   be accepted after step 9 normalises to R=0 with apex at C−(0.03/tan(π/6))·Ẑ = C−0.03√3·Ẑ and
   pcurve v values shifted accordingly. The original raw AXIS2\_PLACEMENT\_3D entity at C is
   unchanged throughout.

10. **Topology graph assembly** — construct the Body/Region/Shell/Face/Loop/Coedge/Edge/Vertex hierarchy
    from the entity graph; validate structural constraints (closed shell, consistent adjacency, loop
    closure).
11. **Geometry attachment** — verify each ADVANCED\_FACE references an allowlisted surface type; each
    EDGE\_CURVE an allowlisted 3D curve; each PCURVE an allowlisted 2D curve.
12. **Trim interval recovery** — for each EDGE\_CURVE, derive the parameter interval [t\_start, t\_end]
    by inverse-mapping the vertex 3D positions (already in metres) onto the edge curve:
    - **3D LINE**: τ = dot(P − pnt, d̂) / m, where d̂ = normalize(DIRECTION), m = staged VECTOR.magnitude (metres).
      After normalization (step 9), the seam pcurve v at that endpoint is v = pnt_pcurve.v + slope × τ.
      Do **not** assert τ = v directly; the pcurve v offset (pnt.v after step-9 shift) is independent.
    - **3D CIRCLE (closed)**: t\_start = 0 (or inverse-mapped angle of start vertex position); t\_end = t\_start + 2π.
      `edge_start = edge_end` is required for CIRCLE; non-closed arc → `STEP_UNSUPPORTED_ARC`.
    - Reparameterize from raw τ to Amphion internal length parameter s = τ × |A| (metres) for LINE edges
      only after trim recovery, for edges where Amphion uses a length-based internal parameter.
13. **P-curve presence check** — every coedge must carry a PCURVE; absence is `STEP_MISSING_PCURVE`.
14. **P-curve / 3D-curve synchronisation** — perform analytic identity checks as defined in §6.3.
    Verify SEAM\_CURVE associated\_geometry = [P0, Ptau] order (u=0 first, u=TAU second); verify
    distinct u values differing by 2π±ε_p; same-u pair → `STEP_INVALID_SEAM_PCURVES`;
    reversed order (Ptau first) → `STEP_INVALID_SEAM_PCURVES`. All checks transactional.
15. **Canonical geometry/topology construction** — values already in metres (step 7), cone already
    normalised (step 9); build canonical Amphion IR. Detect any remaining numeric anomaly as
    `STEP_INVALID_PARAMETER`.

### 9.2 Unsupported Entity Behaviour

Amphion's importer operates in **strict mode**:

- Any entity not in Section 4 produces `STEP_UNSUPPORTED_ENTITY` and **aborts the transaction**.
- Import is fully transactional: the caller receives either a complete valid Body or a ValidationReport;
  no partial body is ever returned.
- There is **no silent healing**, no mesh fallback, no geometry substitution, and no coedge reordering
  to repair orientation errors.
- Future advisory mode (not in v0) may demote unknown non-topological entities to warnings.

### 9.3 No Silent Healing

Per CONTRACTS.md §Errors and diagnostics: "Automatic healing is a separate, explicit operation and
cannot run silently inside construction, boolean, or import APIs."

The importer must not:

- Reorder coedges to satisfy loop closure.
- Snap vertices to close gaps.
- Reconstruct missing p-curves from 3D geometry.
- Reconstruct missing product structure.
- Substitute an approximate surface for an unrecognised entity.

### 9.4 Transactional Failure Semantics

On any validation failure:

- A `ValidationReport` is returned containing all diagnostics collected up to the point of failure.
- No topology entity (`Body`, `Shell`, `Face`, etc.) is visible to the caller.
- All intermediate allocations are released before return.
- Identical input always produces identical diagnostics (determinism required).

Each diagnostic carries:

| Field | Content |
|---|---|
| `code` | Stable uppercase machine code, e.g. `STEP_UNSUPPORTED_ENTITY` |
| `severity` | `Error` or `Warning` |
| `message` | Human-readable description |
| `path` | Entity instance ID + attribute path, e.g. `#42 → edge_geometry → curve_3d` |
| `semantic_ids` | Related Amphion SemanticIds where available |

Diagnostic codes reserved for this scope (non-exhaustive):

`STEP_UNSUPPORTED_SCHEMA` · `STEP_UNSUPPORTED_IMPL_LEVEL` · `STEP_UNSUPPORTED_ENTITY` ·
`STEP_ENCODING_ERROR` · `STEP_PARSE_UNEXPECTED_EOF` · `STEP_PARSE_SYNTAX_ERROR` ·
`STEP_DUPLICATE_INSTANCE_ID` · `STEP_UNRESOLVED_REFERENCE` · `STEP_CIRCULAR_REFERENCE` ·
`STEP_INVALID_NUMERIC` · `STEP_INVALID_PARAMETER` · `STEP_INVALID_DIRECTION` ·
`STEP_MISSING_PCURVE` · `STEP_PCURVE_3D_SYNC_FAILURE` · `STEP_INVALID_SEAM_PCURVES` ·
`STEP_MISSING_REQUIRED_ENTITY` · `STEP_NON_MANIFOLD_TOPOLOGY` · `STEP_LOOP_NOT_CLOSED` ·
`STEP_UNIT_OVERFLOW` · `STEP_UNIT_UNDERFLOW` · `STEP_COMPLEX_ORDER_ERROR` · `STEP_INVALID_CLOSED_EDGE` ·
`STEP_DUPLICATE_SEMANTIC_ID` · `STEP_UNSUPPORTED_IMPL_LEVEL_CLASS` · `STEP_INVALID_FRAME` ·
`STEP_UNSUPPORTED_ARC` · `STEP_AMBIGUOUS_PCURVE`

---

## 10. Round-Trip Equivalence Contract

### 10.1 Definition

A STEP round-trip is the sequence: **Amphion Body → Part 21 file → Amphion Body**.

Two bodies B₁ and B₂ are **round-trip equivalent** when all of the following hold:

**Hierarchy and incidence checks** (per-quantity tolerances where applicable):

1. Equal count of regions, shells, faces, loops, coedges, edges, and vertices.
2. Topology incidence preserved: for each face, same set of adjacent faces (via shared edges).
3. Each edge appears in exactly the same two coedge positions with opposite orientations.
4. Seam edges: each periodic face has exactly two coedges referencing the same EDGE\_CURVE with
   opposite `orientation` values.

**Geometry checks** (tolerances per quantity type):

5. Equal surface type tag per face (`SurfaceKind::{Plane, Cylinder, Cone}`).
6. Surface parameters: length quantities (radii, positions) within `ToleranceContext.absolute_length`;
   angular quantities (semi\_angle, placement axes) within `ToleranceContext.angular`.
7. Equal curve type tag per edge (`CurveKind::{Line, Circle}`).
8. Edge curve parameters: length quantities within `ToleranceContext.absolute_length`; angles within
   `ToleranceContext.angular`.
9. Vertex positions within `ToleranceContext.absolute_length`.
10. Face orientation (`ADVANCED_FACE.same_sense`) and coedge orientation (`ORIENTED_EDGE.orientation`)
    are bitwise identical (boolean; no tolerance).
11. P-curve types and parameters: UV coordinates within `ToleranceContext.parameter_space`; do **not**
    compare UV parameters using absolute length tolerance.
12. Trim intervals: each edge's [t\_start, t\_end] within `ToleranceContext.parameter_space`.
13. Unit conversion is transparent: a body written in millimetres and read back in metres must be
    geometrically identical to the original body.
14. SemanticId of every topology entity (Body, Region, Shell, Face, Loop, Coedge, Edge, Vertex)
    preserved byte-identically across the round-trip when the Amphion SemanticId extension is used.

Round-trip equivalence does **not** require:

- Identical entity instance IDs (file-local).
- Identical string names for geometric helper entities (CARTESIAN\_POINT, DIRECTION, etc.).
- Identical file-level timestamps or author fields.

### 10.2 Canonicalization and Determinism

The encoder is **canonical**: identical Body snapshot + schema version + output unit + writer metadata
(timestamp, author, organisation) → byte-identical Part 21 output.

Canonicalization rules:

- Faces are written in `Shell.faces()` order.
- Edges within an EDGE\_LOOP follow coedge traversal order.
- Within SEAM\_CURVE.associated\_geometry: PCURVE at u=0 always precedes PCURVE at u=2π.
- **Geometry sharing**: two geometry entities may share a single instance if and only if, after
  normalising negative zero to positive zero (−0.0 → 0.0) in all real attributes, every attribute
  value is bitwise identical. No within-ULP merging. This rule is transitive and deterministic.
- **Real-number format**: shortest decimal string that uniquely round-trips to the same IEEE 754
  double-precision value. Always includes a decimal point. Does not include a leading `+` on the
  exponent. Uses uppercase `E`. Exponent has no leading zeros, no leading `+` (e.g. `E3` not `E03` and not `E+3`). Negative zero
  is normalised to positive zero before formatting. (See Section 3.4 rule 3.)
- Complex entity component names are sorted in ascending lexicographic order per Part 21 §12.2.5.3
  (resolved; see §4.6).
- **Entity ID assignment and traversal order (B-09 algorithm, Phase D and Phase E)**: see §10.2 for
  the authoritative 6-phase non-circular algorithm. Summary: fixed document-root sequence (Phase D)
  followed by BFS traversal of B-Rep/geometry nodes (Phase E, after interning/key assignment in Phase C).
  BFS is used only as the Phase-E traversal order after all structural keys are computed. Product/context/
  unit entities precede B-Rep entities in a fixed canonical order; within BFS, children are visited in
  the EXPRESS attribute-declaration order for each entity type.
- **Geometry sharing key (B-09 resolved, B-13 type-discriminator added)**: Keys are computed
  bottom-up from leaf nodes using only structural content; no IDs are referenced at any point.
  Key encoding:
  - **Type prefix** (B-13): every key begins with `4-byte-LE-length + entity-type-name-bytes`
    (e.g., `15||CARTESIAN_POINT`; CARTESIAN\_POINT is 15 characters). For complex entities, use the
    canonical sorted-name join (e.g., `9||DIRECTION` vs `15||CARTESIAN_POINT` — these can never
    collide even for identical REAL triples). This prefix must be the outermost wrapper; child keys
    already carry their own.
  - **SELECT discriminator**: for any attribute whose EXPRESS type is a SELECT type, prefix the
    attribute value's encoding with `4-byte-LE-length + selected-alternative-type-name`.
  - REAL attribute: 8-byte LE IEEE 754 double (after −0→+0 normalisation)
  - STRING attribute: 4-byte LE length + UTF-8 bytes
  - ENUM/BOOLEAN/LOGICAL attribute: ASCII bytes of canonical string representation
  - REF to internable node: that node's structural key (recurse; terminates because internable
    geometry is acyclic — references only scalars or other internable geometry)
  - LIST aggregate: 4-byte count + ordered child keys
  - SET aggregate: 4-byte count + child keys sorted lexicographically + rejection of duplicates

  **Internable classes** (structural key fully determines value; B-24):
  Simple scalars: `CARTESIAN_POINT`, `DIRECTION`, `VECTOR`, `AXIS2_PLACEMENT_2D`, `AXIS2_PLACEMENT_3D`, `DIMENSIONAL_EXPONENTS`.
  Unit-system nodes: `SI_UNIT`, `LENGTH_MEASURE_WITH_UNIT` (conversion-factor measure node),
    `CONVERSION_BASED_UNIT`, `UNCERTAINTY_MEASURE_WITH_UNIT`.
  Unit complexes (the four concrete instances): metre complex `(LENGTH_UNIT NAMED_UNIT(*) SI_UNIT($,.METRE.))`,
    millimetre complex `(CONVERSION_BASED_UNIT NAMED_UNIT LENGTH_UNIT)`,
    radian complex `(NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.))`,
    steradian complex `(NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT())`.
  Geometry: `LINE` (geometry), `CIRCLE` (geometry), `PLANE`, `CYLINDRICAL_SURFACE`, `CONICAL_SURFACE`.
  NOT internable: `PCURVE`, `DEFINITIONAL_REPRESENTATION`, `SURFACE_CURVE`, `SEAM_CURVE` (non-canonical
    topology ties make them identity-bearing), and all topology/product/context entities.

  Two internable nodes with identical structural keys are collapsed to one canonical instance;
  all references updated before ID assignment.

  **Phase A–F algorithm** (B-19 — full inline):

  **Phase A — Build non-interned IR and assign canonical occurrence paths**:
  Allocate one IR node per entity instance (no IDs yet). Store all attribute values as typed fields.
  Normalize −0.0 → +0.0 in every REAL. Reject nonfinite REALs as `STEP_INVALID_NUMERIC`.
  All pointer fields reference IR nodes by pointer, not by ID.

  **Canonical occurrence path assignment** (before keys/IDs):
  Every writer IR node receives a stable path independent of pointer values.
  - **Document-root nodes**: path = 1-byte `role_ordinal` from the Phase D table (roles 1–19).
  - **Non-root internable nodes**: these will be collapsed by key in Phase C; their occurrence path
    is only needed as a tie-break for non-internable SET members. Assign the lexicographically least
    BFS discovery path as the canonical occurrence path (see below).
  - **Non-root non-internable nodes** (PCURVE, topology, product, etc.): path encodes the structural
    position reachable from the nearest root-role node:
    `(root_role_ordinal, attr_ordinal₀, idx₀, attr_ordinal₁, idx₁, ...)` where each step is
    (EXPRESS-declared attribute ordinal, LIST index or kernel-SET occurrence index). Path is encoded
    as a byte-sequence (1-byte ordinal per level; 4-byte-LE index per level) so no two distinct
    structural positions produce the same sequence.
    **Shared node rule**: if the same node is reachable via multiple paths, assign the lexicographically
    least path.
  - **Kernel SET occurrence index**: the SET's iteration order is already determined by the frozen
    deterministic topology iterator (bottom-up kernel order); use that index directly without re-sorting,
    avoiding a use-before-definition cycle with Phase B.
  - No pointers, IDs, or memory addresses appear in any occurrence path.

  **Phase B — Bottom-up structural keys**:
  Compute keys recursively from leaf nodes to roots. No keys reference entity IDs.
  Key encoding for each node:
  1. `4-byte-LE length` of the entity/complex-type name, then those ASCII bytes.
  2. For complex entities: sorted name join (ascending lexicographic, each with its own length prefix).
  3. For each attribute (in EXPRESS declaration order):
     - REAL: 8-byte LE IEEE 754 double (already −0-normalized).
     - STRING: `4-byte-LE length` + UTF-8 bytes.
     - ENUM/BOOLEAN/LOGICAL: ASCII bytes of canonical uppercase token.
     - SELECT value: `4-byte-LE length` of alternative-type name + alternative-type bytes + value encoding.
     - REF to internable node: that node's structural key (recurse; terminates because internable geometry is acyclic).
     - REF to internable node reachable from another internable node: recurse (the internable closure is
       acyclic and references only scalars or other internable nodes — no `0xFF` placeholder is ever needed).
     - REF to non-internable node appearing in a non-internable node's attribute: use the canonical
       occurrence-path string (Phase A) encoded as a length-prefixed UTF-8 byte sequence. If a class
       contains any such reference, it is non-internable.
     - LIST aggregate: `4-byte count` + ordered child key bytes.
     - SET aggregate: `4-byte count` + child keys sorted lexicographically + reject duplicates → `STEP_DUPLICATE_SET_MEMBER`.
  **TLV kind tags** (B-27): every attribute slot is prefixed by a 1-byte kind tag before the length+value:
  - `0x00` — **omitted optional** (`$`); no further bytes for this slot. E.g. `SI_UNIT($,.METRE.)`:
    the first attribute (name) emits tag 0x00 with no payload.
  - `0x01` — **derived** (`*`); no further bytes. E.g. `NAMED_UNIT(*)`: the dimensions attribute
    emits tag 0x01. This makes the NAMED_UNIT(*) and SI_UNIT($,.METRE.) keys unambiguously constructible.
  - `0x02` — **null/absent** (explicitly null in schema, distinct from omitted); no further bytes.
  - `0x03` — **present value**; followed by `4-byte-LE-length` then the encoded value bytes.

  **TLV encoding** (B-24): every key segment is length-delimited — `4-byte-LE-length` prefix followed by
  the value bytes. Variable-length segments (strings, child keys, aggregates) are never concatenated
  without a length delimiter. This prevents ambiguous key collisions.

  **SET duplicate** (B-24): a SET duplicate means two members have identical entity identity **after**
  Phase C interning (same pointer). Two non-internable members with structurally equal attributes but
  distinct entity identity are NOT duplicates and remain distinct in the SET.
  SET sort key: internable members sort by their structural key bytes; non-internable members sort by
  their canonical occurrence path assigned in Phase A.

  Keys computed for **all** nodes, but interning collapses only nodes in the explicit internable list.

  **Phase C — Intern internable classes**:
  For every node in the internable class list (see above), look up its key in a hash map.
  Hash is used only as a bucket; full-key comparison on collision.
  First occurrence wins; all later nodes with identical full keys have their pointers replaced by the
  first occurrence and are removed from the IR. After Phase C, every internable class member is unique
  by structural key. Non-internable nodes are untouched.

  **Phase D — Fixed synthetic document-root sequence** (B-24):
  The canonical root table lists every document-level node in fixed emission order, independent of IR
  pointer values. Phase E then BFS-traverses from MANIFOLD\_SOLID\_BREP.

  | # | Role | Entity | Condition |
  |---|---|---|---|
  | 1 | Application context | `APPLICATION_CONTEXT` (`'mechanical design'`) | always |
  | 2 | AP definition | `APPLICATION_PROTOCOL_DEFINITION` (`_mim_lf`, 2020) | always |
  | 3 | Length dimensions | `DIMENSIONAL_EXPONENTS(1.,0.,0.,0.,0.,0.,0.)` | mm file only |
  | 4 | SI metre base | `SI_UNIT($, .METRE.)` | mm file only |
  | 5 | mm conversion measure | `LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.0E-3), #si_m)` | mm file only |
  | 6 | Length unit | metre complex or mm complex (`#len_unit`) | always (one of two forms) |
  | 7 | Angle unit | radian complex (`#ang_unit`) | always |
  | 8 | Solid-angle unit | steradian complex (`#sang_unit`) | always |
  | 9 | Uncertainty | `UNCERTAINTY_MEASURE_WITH_UNIT` (`#uncert`) | always |
  | 10 | Combined context | complex `(GEOMETRIC_REPRESENTATION_CONTEXT GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT GLOBAL_UNIT_ASSIGNED_CONTEXT REPRESENTATION_CONTEXT)` (`#ctx`) | always |
  | 11 | 2D param context | complex `(GEOMETRIC_REPRESENTATION_CONTEXT(2) PARAMETRIC_REPRESENTATION_CONTEXT REPRESENTATION_CONTEXT)` (`#p2d_ctx`) | always |
  | 12 | Product context | `PRODUCT_CONTEXT` (`#prd_ctx`) | always |
  | 13 | Product def context | `PRODUCT_DEFINITION_CONTEXT` (`#prd_def_ctx`) | always |
  | 14 | Product | `PRODUCT` (`#prd`) | always |
  | 15 | Product formation | `PRODUCT_DEFINITION_FORMATION` (`#pdf`) | always |
  | 16 | Product definition | `PRODUCT_DEFINITION` (`#pd`) | always |
  | 17 | Product def shape | `PRODUCT_DEFINITION_SHAPE` (`#pds`) | always |
  | 18 | BRep representation | `ADVANCED_BREP_SHAPE_REPRESENTATION` (`#absr`) | always |
  | 19 | Shape–def link | `SHAPE_DEFINITION_REPRESENTATION` (`#sdr`) | always |
  | 20+ | B-Rep/geometry | `MANIFOLD_SOLID_BREP` + BFS-order topology/geometry | Phase E |

  Rows 3–5 are present only in millimetre output; in metre output the SI metre complex (row 6) requires
  no prior `DIMENSIONAL_EXPONENTS` or conversion-measure rows. The complex context components in rows
  10–11 are complex entity parts, not separate root entries; they are emitted as one entity instance each.
  Optional roles (e.g., PRODUCT\_RELATED\_PRODUCT\_CATEGORY) are omitted in canonical output.
  No phrase `unit/dimensional entities` — each role is explicit in this table.

  **Phase E — Deterministic traversal and ID assignment**:
  Two-pass BFS over the root sequence and all reachable nodes:
  - **Pass 1** (ID assignment): visit each node in BFS order (EXPRESS attribute declaration order
    for children of each node; LIST order preserved; SET child order = key-sorted order from Phase B).
    First visit assigns the next sequential ID `#1, #2, ...`. Already-visited nodes are not
    re-enqueued. Nodes not reachable from the root sequence receive no ID (orphan — see Phase F).
  - **Pass 2** (serialization): emit `#ID = ENTITY(attr, ...);` for each ID in numeric order.
    Forward references (to a node whose ID was assigned after the current node's in Pass 1) are
    emitted as ID references without special treatment — forward refs are legal in Part 21.
  IDs are assigned only after Phase C (interning); no ID is ever embedded in a structural key.

  **Phase F — Reachability audit**:
  After Phase E, walk the full IR:
  - Internable node with no ID (unreachable): silently drop (no error — it was deduped to another node).
  - Non-internable node with no ID (orphan): emit `STEP_ORPHAN_ENTITY`.
  - Any attribute referencing a node that received no ID: emit `STEP_DANGLING_REF`.
  - Topology pointer cycle detected during BFS: emit `STEP_TOPOLOGY_CYCLE`.
  - Duplicate key in a SET aggregate (caught in Phase B): emit `STEP_DUPLICATE_SET_MEMBER`.

  **Complexity**: O(total key bytes + Σ kᵢ log kᵢ) time for Phase B sort; O(key bytes + node count)
  space. Schema/APD/unit/header/name/SemanticId/provenance strings are included as UTF-8 bytes in
  Phase B keys; they contribute to byte-identity.

  **Future SCC requirement**: if Amphion ever admits topology or non-geometry entities referencing
  each other cyclically, Phase B must be extended to handle strongly-connected components (SCCs) via
  Tarjan's algorithm before key finalization. v0 geometry is a DAG; SCCs are not currently possible.

- **Real-number format (B-08 resolved — Ryu 1.0.23)**: see §3.4. Summary: `ryu = "=1.0.23"`;
  `format_finite` → `e`→`E` → insert `.0` if no `.` in mantissa. Grammar: `-?[0-9]+\.[0-9]+(E-?[0-9]+)?`.

---

## 11. Fixture Matrix, Negative Cases, Differential Tests, and Acceptance Gates

### 11.1 Positive Fixtures

All fixtures must produce passing differential tests (Section 11.3).

| ID | Shape | Placement | File unit | Special |
|---|---|---|---|---|
| P-01 | Cuboid 1×1×1 m | Origin, identity axes | metre | Baseline planar |
| P-02 | Cuboid 0.05×0.10×0.20 m | Origin | metre | Non-unit dimensions |
| P-03 | Cuboid 1×1×1 m | 45° rotation about Z | metre | Rotated |
| P-04 | Cuboid 0.05×0.10×0.20 m | Arbitrary translation + rotation | metre | General placement |
| P-05 | Cuboid 50×100×200 mm | Origin | millimetre | Unit conversion (mm) |
| P-06 | Cylinder r=0.05 m, h=0.10 m | Origin, Z-axis | metre | Standard cylinder |
| P-07 | Cylinder r=0.03 m, h=0.08 m | Tilted 30° from Z | metre | Non-Z axis |
| P-08 | Cylinder r=50 mm, h=100 mm | Origin | millimetre | Unit conversion |
| P-09 | Cone frustum r₁=0.05, r₂=0.02, h=0.10 m | Origin | metre | Frustum, two caps |
| P-10 | Cone to apex r=0.05, h=0.10 m | Origin | metre | Apex singularity |
| P-11 | Cone to apex r=0.03, h=0.07 m | Tilted 20° from Z | metre | Apex + placement |
| P-12 | Cone frustum 50/20 mm, h=100 mm | Origin | millimetre | Unit conversion |
| P-13 | Cuboid 1×10⁻⁴×10⁻⁴ m | Origin | metre | Near-uncertainty scale |
| P-14 | Cuboid 500×500×500 m | Origin | metre | Large scale |
| P-15 | Cylinder r = 5×uncertainty, h=0.01 m | Origin | metre | Tolerance boundary |

### 11.2 Negative Cases

Each must return the specified diagnostic code and no Body.

| ID | Fault | Expected code |
|---|---|---|
| N-01 | Truncated file; no `END-ISO-10303-21;` | `STEP_PARSE_UNEXPECTED_EOF` |
| N-02 | `FILE_SCHEMA(('CONFIG_CONTROL_DESIGN'))` | `STEP_UNSUPPORTED_SCHEMA` |
| N-03 | Implementation level `'1;1'` | `STEP_UNSUPPORTED_IMPL_LEVEL` |
| N-04 | `B_SPLINE_SURFACE_WITH_KNOTS` in DATA | `STEP_UNSUPPORTED_ENTITY` |
| N-05 | `NEXT_ASSEMBLY_USAGE_OCCURRENCE` in DATA | `STEP_UNSUPPORTED_ENTITY` |
| N-06 | `TESSELLATED_SHAPE_REPRESENTATION` | `STEP_UNSUPPORTED_ENTITY` |
| N-07 | `FACE_BOUND` (inner loop) in DATA | `STEP_UNSUPPORTED_ENTITY` |
| N-08 | `OPEN_SHELL` | `STEP_UNSUPPORTED_ENTITY` |
| N-09 | NaN in CARTESIAN\_POINT coordinate | `STEP_INVALID_NUMERIC` |
| N-10 | Infinity in CARTESIAN\_POINT coordinate | `STEP_INVALID_NUMERIC` |
| N-11 | Negative radius in CYLINDRICAL\_SURFACE | `STEP_INVALID_PARAMETER` |
| N-12 | `semi_angle = 0` in CONICAL\_SURFACE | `STEP_INVALID_PARAMETER` |
| N-13 | `semi_angle ≥ π/2` in CONICAL\_SURFACE | `STEP_INVALID_PARAMETER` |
| N-14 | DIRECTION with zero magnitude | `STEP_INVALID_DIRECTION` |
| N-15 | Duplicate instance ID `#5 = ... ; #5 = ...` | `STEP_DUPLICATE_INSTANCE_ID` |
| N-16 | Forward reference to non-existent `#999` | `STEP_UNRESOLVED_REFERENCE` |
| N-17 | Edge on CYLINDRICAL\_SURFACE with no PCURVE | `STEP_MISSING_PCURVE` |
| N-18 | Empty DATA section | `STEP_MISSING_REQUIRED_ENTITY` |
| N-19 | T-edge (3 faces sharing one edge) | `STEP_NON_MANIFOLD_TOPOLOGY` |
| N-20 | EDGE\_LOOP not closed (end ≠ start vertex) | `STEP_LOOP_NOT_CLOSED` |
| N-21 | SEAM\_CURVE p-curve at u=0 and u=2π are identical 2D LINE instances (same entity ID) | `STEP_INVALID_SEAM_PCURVES` |
| N-22 | SURFACE\_CURVE where 3D LINE pnt + dir cannot evaluate within tolerance of p-curve-mapped surface | `STEP_PCURVE_3D_SYNC_FAILURE` |
| N-23 | DEFINITIONAL\_REPRESENTATION with `$` for `context_of_items` | `STEP_MISSING_REQUIRED_ENTITY` (`$` is valid omitted-optional Part 21 syntax; error is schema/required-entity validation, not parse syntax) |
| N-24 | Implementation level `'4;2'` (uses REFERENCE section; not supported in v0) | `STEP_UNSUPPORTED_IMPL_LEVEL_CLASS` |
| N-25 | Implementation level `'4;3'` (further Ed3 extension sections; not supported in v0) | `STEP_UNSUPPORTED_IMPL_LEVEL_CLASS` |
| N-26 | Malformed `\X2\` escape (odd number of hex digits) | `STEP_ENCODING_ERROR` |
| N-27 | Raw UTF-8 two-byte sequence in string token under level `'2;1'` | `STEP_ENCODING_ERROR` |
| N-28 | Malformed `\X4\` escape (non-eight-hex-digit group) | `STEP_ENCODING_ERROR` |
| N-29 | Complex entity components in non-ascending order (e.g., `SI_UNIT(...)` before `NAMED_UNIT(*)`) | `STEP_COMPLEX_ORDER_ERROR` |
| N-30 | SI\_UNIT complex with explicit `NAMED_UNIT(#dim_id)` instead of `NAMED_UNIT(*)` | `STEP_INVALID_PARAMETER` (`NAMED_UNIT(#dim)` is syntactically parseable in a complex entity but violates the derived-attribute rule; it is a schema/type error, not a parse error) |
| N-31 | Cuboid edge (planar+planar adjacency) with no PCURVE in SURFACE\_CURVE | `STEP_MISSING_PCURVE` |
| N-32 | Cone seam SEAM\_CURVE has only one PCURVE entry in `associated_geometry` (schema requires exactly 2) | `STEP_MISSING_PCURVE` (B-24: must be SEAM\_CURVE, not SURFACE\_CURVE, for the seam; WHERE clause violated) |
| N-33 | Cylinder cap CIRCLE p-curve parameterisation-scale mismatch (LINE magnitude ≠ 1.0, p-curve not synchronised) | `STEP_PCURVE_3D_SYNC_FAILURE` |
| N-34 | Cone R=0.05 m, α=π/4, C=origin, identity placement. 3D seam: P(τ)=(0.05,0,0)+τ·(1,0,1) (correct: starts at R surface). Pcurve deliberately wrong: q(τ)=(0, 0.01+τ) (pnt.v=0.01 m instead of 0). Step-9 shift=0.05/tan(π/4)=0.05 m; new apex C'=(0,0,−0.05); pcurve q'(τ)=(0, 0.01+0.05+τ)=(0, 0.06+τ). At τ=0: surface S'(0, 0.06)=C'+0.06·(tan(π/4)·X̂+Ẑ)=(0.06,0,0.01); 3D P₀=(0.05,0,0). M3 ① residual=‖(0.05,0,0)−(0.06,0,0.01)‖=‖(−0.01,0,−0.01)‖=0.01√2≈0.01414 m >> ε_l → fails. Correct pcurve q(τ)=(0,τ) shifts to q'=(0,0.05+τ); S'(0,0.05)=(0.05,0,0)=P₀ ✓ | `STEP_PCURVE_3D_SYNC_FAILURE` (B-28: exact residual 0.01414 m proves inconsistency; correct input would pass) |
| N-35 | Cone seam PCURVE both at u=0 (seam sides not at u=0 and u=2π) | `STEP_INVALID_SEAM_PCURVES` |
| N-36 | SEAM\_CURVE where p-curve u=0 direction is `(0,-1)` but u=2π direction is `(0,+1)` (sense mismatch) | `STEP_INVALID_SEAM_PCURVES` |
| N-37 | CYLINDRICAL\_SURFACE radius = 1.0E+307 in a KILO.METRE file (factor = 1000); converted value = 1.0E+310 m, exceeds f64 max (~1.8E+308) → overflow | `STEP_UNIT_OVERFLOW` |
| N-38 | Two ADVANCED\_FACE entities with same `amphion.id/1/<hex>` name | `STEP_DUPLICATE_SEMANTIC_ID` |
| N-39 | ORIENTED\_EDGE instance with only 3 positional arguments instead of the required 5: `ORIENTED_EDGE('', edge_ref, .T.)` (missing two `*` for derived endpoint slots) | `STEP_PARSE_SYNTAX_ERROR` (wrong arity; Part 21 is positional so missing `*` is detectable) |
| N-40 | `NEXT_ASSEMBLY_USAGE_OCCURRENCE` in DATA | `STEP_UNSUPPORTED_ENTITY` |
| N-41 | Minimal synthetic AP242 Ed2 fixture with formal OID-suffixed FILE\_SCHEMA string and a valid simple cuboid body | *(positive header acceptance test)* — must import successfully (moved out of negative table; see §13 for citation policy: external files containing rejected entities must be used as header-only parser tests, not as full-import fixtures) |
| N-42 | `edge_start = edge_end` on a LINE EDGE\_CURVE (non-closed edge) | `STEP_INVALID_CLOSED_EDGE` |
| N-43 | CIRCLE radius = 0.0 in EDGE\_CURVE geometry | `STEP_INVALID_PARAMETER` |
| N-44 | CYLINDRICAL\_SURFACE radius = 0.0 | `STEP_INVALID_PARAMETER` |
| N-45 | VECTOR magnitude = 0.0 in LINE direction | `STEP_INVALID_PARAMETER` |
| N-46 | AXIS2\_PLACEMENT\_3D with axis parallel to ref\_direction (frame undefined) | `STEP_INVALID_FRAME` |
| N-47 | BREP\_WITH\_VOIDS (void solid with ORIENTED\_CLOSED\_SHELL) | `STEP_UNSUPPORTED_ENTITY` |
| N-48 | CONICAL\_SURFACE with R>0, valid matching seam p-curves for that R value (positive reparameterisation) | *(see P-RP-01)* |
| P-RP-01 | Cone with R=0.03 m, semi\_angle=π/6, h=0.1 m, STEP file emitted with R=0.03 (not canonical); p-curves correct for that R | Positive: must import and produce correct apex-form canonical body after reparameterisation |
| N-49 | Synthetic header: FILE\_SCHEMA with OID `{1 0 10303 442 1 1 4}` (verified from CATIA file) + valid `APPLICATION_PROTOCOL_DEFINITION('','ap242_managed_model_based_3d_engineering_mim_lf',2014,#app_ctx)` with proper #app\_ctx reference — positive | Positive: accepted; OID confirmed from commit `bcc76aeb` |
| N-50 | Onshape/ST-Developer header: APD without `_mim_lf`, year=2020, FILE\_SCHEMA OID `{1 0 10303 442 3 1 4}` — positive; must be accepted | Positive: accepted; commit `51ca0ec3` |
| N-51 | NIST bare-schema header: year=2011, no FILE\_SCHEMA OID — positive; must be accepted | Positive: accepted; commit `e0dbd5ec` |
| N-52 | Cone SEAM\_CURVE.associated\_geometry = [Ptau, P0] (u=TAU first, u=0 second — reversed from required [P0, Ptau] order) | `STEP_INVALID_SEAM_PCURVES` |
| N-53 | Cylinder SURFACE\_CURVE: 3D LINE C(t)=(1,0,t), pcurve LINE q(t)=(4πt,t); slope 4π in u detected by M2 interval check | `STEP_PCURVE_3D_SYNC_FAILURE` |
| N-54 | Cylinder CIRCLE with LINE pcurve q(t)=(φ+2t,h); slope 2 detected by M5 interval check (ε_o≠±1) | `STEP_PCURVE_3D_SYNC_FAILURE` |
| N-55 | Cylinder seam: 3D LINE pnt=(1,0,0), dir=(0,0,1), pcurve q=(0, 1+t) — P₀=(1,0,0) ≠ O+R·e(0)+1·Ẑ = (1,0,1) on unit cylinder; M2 ① fails (axial-offset mismatch) | `STEP_PCURVE_3D_SYNC_FAILURE` |
| N-56 | CIRCLE/PLANE: 2D CIRCLE with ref\_direction negated so that G\_y points opposite to expected F\_y; M4 ③ coefficient fails | `STEP_PCURVE_3D_SYNC_FAILURE` |
| N-57 | CIRCLE on unit cylinder at height h=1: 3D centre Oc=(0.5,0,1) (off axis); M5 ① ‖Oc−(O+hẐ)‖=0.5 > ε_l | `STEP_PCURVE_3D_SYNC_FAILURE` |
| N-58 | semi\_angle = 200° with PLANE\_ANGLE\_UNIT = DEGREE; converted 200×π/180 ≈ 3.49 rad > π/2 | `STEP_INVALID_PARAMETER` |
| N-59 | Two CYLINDRICAL\_SURFACE faces sharing a non-seam SURFACE\_CURVE edge (two PCURVEs on distinct cylinder entities) | `STEP_UNSUPPORTED_ENTITY` (B-18: non-seam curved+curved not admitted in v0) |
| N-60 | Tilted cylinder (P-07: Ẑ_surf=(0,0.5,√3/2), X̂_surf=(1,0,0), Ŷ_surf=(0,√3/2,−0.5)); cap CIRCLE placed with world-frame Z=(0,0,1) and ref=(1,0,0) (X̂_c=(1,0,0), Ŷ_c=(0,1,0)); correct pcurve LINE q=(φ+t,h). M5 ②: F_x=R·X̂_c=R·(1,0,0) = R·ê(0) — passes. M5 ③: F_y=R·Ŷ_c=R·(0,1,0) but ε_o·R·ê_θ(0)=ε_o·R·Ŷ_surf=R·(0,√3/2,−0.5); ‖F_y−ε_o·R·ê_θ(0)‖=R·sin30°=0.015m>>ε_l | `STEP_PCURVE_3D_SYNC_FAILURE` (B-24/B-17: M5 ③ fails with correct placement-frame ê_θ) |

| N-61 | DEFINITIONAL\_REPRESENTATION.items = empty SET (zero 2D curves for a PCURVE) | `STEP_MISSING_PCURVE` (B-23: exactly one 2D curve required) |
| N-62 | DEFINITIONAL\_REPRESENTATION.items contains two 2D LINEs (ambiguous pcurve) | `STEP_AMBIGUOUS_PCURVE` (B-23: exactly one 2D curve required) |
| N-63 | CIRCLE EDGE\_CURVE with edge\_start ≠ edge\_end (arc, not closed circle) | `STEP_UNSUPPORTED_ARC` (B-23: v0 supports closed full circles only) |
| N-64 | Cylinder cap: `EDGE_CURVE.same_sense = .F.`; raw 3D CIRCLE has **positive** frame (F\_y\_raw = +R·ê\_θ(φ), ε\_raw\_3d=+1); raw pcurve **already** `q=(φ−t,h)` (ε\_raw\_pcurve=−1). Simultaneous reversal: canonical F\_y = −R·ê\_θ (ε\_3d\_canonical=−1); canonical pcurve ε = −(−1)=+1 → `q=(φ+t,h)`. M5 ③: ‖F\_y\_canonical − ε\_canonical·R·ê\_θ‖ = ‖−R·ê\_θ − (+1)·R·ê\_θ‖ = 2R >> ε\_l | `STEP_PCURVE_3D_SYNC_FAILURE` (B-41: internally inconsistent: raw pcurve pre-negated while same\_sense already negates it; correct input is raw `q=(φ+t,h)` which reverses consistently) |
| N-65 | CONICAL\_SURFACE radius = −0.01 m | `STEP_INVALID_PARAMETER` (B-36: step 8 rejects R < 0 before step 9) |
| N-66 | SemanticId with 31 hex digits (44 chars total) | Display-only; not parsed as SemanticId (regex requires exactly 32 hex) |
| N-67 | SemanticId with 33 hex digits (46 chars total) | Display-only; not parsed as SemanticId (regex requires exactly 32 hex) |
| N-68 | SemanticId with uppercase hex: `amphion.id/1/4A5B6C7D8E9F0A1B2C3D4E5F6A7B8C9D` | Display-only; regex `[0-9a-f]` requires lowercase |
| P-18 | Cone (R=0.05 m, α=π/4) and PLANE sharing the same `AXIS2_PLACEMENT_3D` instance at origin. After step 9: cone canonical placement cloned to C'=(0,0,−0.05); PLANE's basis\_surface still references original placement at origin. Both surfaces geometry-correct after normalization. | `STEP_OK` (B-40: shared placement not mutated; cone-clone leaves other surface unchanged) |
| N-69 | Cone R=1.1e308 m (finite, passes staging), α=π/6; file includes valid LENGTH\_UNIT (metre) and PLANE\_ANGLE\_UNIT (radian) so B-42 does not preempt. Shares AXIS2\_PLACEMENT\_3D with a PLANE. Exact real shift = 1.1e308×√3 ≈ 1.905e308 is a finite real number but has no finite f64 enclosure; certified interval lower\_bound > f64::MAX ≈ 1.798e308 before any f64 materialization → `STEP_UNIT_OVERFLOW`. No new IR nodes allocated; PLANE placement byte-identical to raw input. | `STEP_UNIT_OVERFLOW` (B-40/B-43/B-47: certified shift; no partial rebind; not preempted by B-42) |
| N-70 | CONICAL\_SURFACE with semi\_angle=π/4 present in data section; GLOBAL\_UNIT\_ASSIGNED\_CONTEXT contains LENGTH\_UNIT and SOLID\_ANGLE\_UNIT but **no** PLANE\_ANGLE\_UNIT | `STEP_INVALID_UNIT` (B-42: cone present but angle unit missing) |
| P-19a | Cuboid-only file (no CONICAL\_SURFACE); GLOBAL\_UNIT\_ASSIGNED\_CONTEXT omits PLANE\_ANGLE\_UNIT | Deterministic non-fatal warning; body decoded; angle\_factor=1.0 (B-42/B-46) |
| P-19b | Cylinder with seam + cap pcurves (no CONICAL\_SURFACE); GLOBAL\_UNIT\_ASSIGNED\_CONTEXT omits PLANE\_ANGLE\_UNIT | Same warning; body decoded; pcurve u-coordinates (seam u=0/TAU, cap φ parameter) remain canonical radians — unmodified by angle\_factor absence (B-42/B-46) |
| N-71 | CONICAL\_SURFACE with valid semi\_angle; GLOBAL\_UNIT\_ASSIGNED\_CONTEXT has both radian and degree PLANE\_ANGLE\_UNIT members (resolved\_count=2); valid LENGTH\_UNIT and SOLID\_ANGLE\_UNIT also present | `STEP_INVALID_UNIT` (B-44/B-45: cone-present, resolved\_count>1, deterministic selection forbidden) |

### 11.1b Additional Positive Fixtures

| ID | Shape | Special | Notes |
|---|---|---|---|
| P-16 | Cone semi\_angle = 45°, file unit = DEGREE (angle\_factor = π/180) | Degree conversion | Verifies unit staging: 45×π/180 = π/4 radians, valid ∈ (0,π/2) |
| P-17 | Cylinder AXIS2\_PLACEMENT\_3D: axis=(0,0,1), ref\_direction=(1,1,0) (skew, not orthogonal to axis, not parallel) | Frame skew | Ẑ=(0,0,1); Ŷ=normalize((0,0,1)×(1,1,0))=normalize((−1,1,0))=(−1/√2,1/√2,0); X̂=Ŷ×Ẑ=(1/√2,1/√2,0). Accepted; ê(0)=X̂=(1/√2,1/√2,0). M-matrix uses these axes. |
| P-17b | SemanticId exactly 45 chars: `amphion.id/1/` (13) + `4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d` (32) | Length | Accepted as SemanticId; Body.provenance.semantic_id reconstructed. |

### 11.3 Differential Tests

For every positive fixture, after the round-trip decode, compare the decoded Body against an
independently constructed canonical Amphion Body:

1. **Hierarchy check**: region, shell, face, loop, coedge, edge, and vertex counts must match the
   canonical body.
2. **Topology incidence check**: adjacency graph, seam-edge use pairs, and loop membership must be
   isomorphic to the canonical body.
3. **Surface type check**: each face's `SurfaceKind` tag must match.
4. **Surface parameter check**: length parameters within `ToleranceContext.absolute_length`; angle
   parameters within `ToleranceContext.angular`; use family-specific analytic check, not sampling.
5. **Edge curve type and parameter check**: using the same per-quantity tolerances.
6. **Vertex position residual**: each decoded vertex within `ToleranceContext.absolute_length` of canonical.
7. **P-curve check**: for each coedge, verify type and UV parameters within `ToleranceContext.parameter_space`.
8. **Trim interval check**: each edge's trim interval within `ToleranceContext.parameter_space`.
9. **Orientation check**: `ADVANCED_FACE.same_sense` and `ORIENTED_EDGE.orientation` must be bitwise
   identical to the canonical body's values.
10. **SemanticId preservation**: each topology entity's SemanticId must be byte-identical to the original.
11. **Additional sampling** (extra coverage only, not substitution for analytic checks): evaluate each
    surface/curve at 100 deterministic (u,v)/(t) sample points seeded from fixture-ID; verify the
    decoded value matches the canonical via evaluator; failure indicates a non-analytic divergence.

All differential samples are generated from a deterministic seeded RNG (`entity_semantic_id ++ fixture_id`).

### 11.4 Milestone Acceptance Gates

`step-roundtrip-tests` closes wave 9 when all of the following pass:

1. All P-01 through P-15 positive fixtures pass the Section 11.3 differential check.
2. All N-01 through N-68 negative cases produce the stated diagnostic code and return no Body
   (N-41, N-48, N-49, N-50, N-51 are positive acceptance tests; P-16/P-17/P-17b/P-19a/P-19b are
   additional positive fixtures; N-66..N-68 produce display-only parse behavior, not error codes). P-RP-01
   must decode to a geometrically correct canonical body.
   Interval arithmetic backend passes certification for all M1–M8 matrix rows in §6.3.
   N-59, N-60 (placement frame checks), N-61/N-62 (exactly-one-curve), N-63 (arc), N-64 (same_sense),
   N-65 (cone R<0), N-66..N-68 (SemanticId length/case), N-69 (certified overflow, no partial rebind),
   N-70 (missing angle unit + cone → error), N-71 (two angle units + cone → STEP\_INVALID\_UNIT) pass.
   P-18 (shared placement cone+plane clone), P-19a (cuboid no angle unit → warning),
   P-19b (cylinder no angle unit → warning; pcurve u canonical radians) must succeed.
   Unit staging (step 7) is validated by P-16 (degree conversion) and N-58 (domain error after
   conversion), plus a CI mutation test that verifying CONVERSION\_BASED\_UNIT factor is not
   applied twice.
3. The Part 21 fuzz target (`qa/fuzz/step/`) completes ≥ 60 CPU-seconds from the CI seed corpus with
   no panic, no `unwrap` unwind, and no silent misparse.
4. The encoder produces byte-identical output on two consecutive runs for each positive fixture.
5. No `unwrap`, `expect`, or `panic!` is reachable from any positive or negative fixture under
   `cargo test --all-features`.
6. `cargo clippy --workspace --all-features -- -D warnings` and `cargo fmt --all --check` pass on the
   `exchange-step` crate.

---

## 12. Deferred Capabilities (Out of v0 Scope)

Encountering any of these during import is `STEP_UNSUPPORTED_ENTITY` in strict mode.

| Capability | Representative entities |
|---|---|
| Assembly structure | `NEXT_ASSEMBLY_USAGE_OCCURRENCE` (note correct spelling), `PRODUCT_DEFINITION_RELATIONSHIP`, `PRODUCT_DEFINITION_FORMATION_WITH_SPECIFIED_SOURCE` (common vendor wrapper; intentionally rejected in strict mode), component placement |
| PMI / GD&T (semantic) | `GEOMETRIC_TOLERANCE`, `DIMENSIONAL_CHARACTERISTIC_REPRESENTATION` |
| PMI / GD&T (presentation) | `DRAUGHTING_MODEL`, `PRESENTATION_LAYER_ASSIGNMENT` |
| Colors and rendering | `COLOUR_RGB`, `SURFACE_STYLE_USAGE`, `STYLED_ITEM` |
| NURBS surfaces | `B_SPLINE_SURFACE`, `B_SPLINE_SURFACE_WITH_KNOTS`, `RATIONAL_B_SPLINE_SURFACE` |
| NURBS curves | `B_SPLINE_CURVE`, `B_SPLINE_CURVE_WITH_KNOTS` |
| Swept and revolved solids | `SURFACE_OF_REVOLUTION`, `EXTRUDED_AREA_SOLID` |
| Tessellation | `TESSELLATED_SHAPE_REPRESENTATION`, `COORDINATES_LIST` |
| Sphere | `SPHERICAL_SURFACE` |
| Torus | `TOROIDAL_SURFACE` |
| Composite curves | `COMPOSITE_CURVE`, `COMPOSITE_CURVE_SEGMENT` |
| Non-manifold B-Rep | `OPEN_SHELL_BASED_SURFACE_MODEL`, `SHELL_BASED_SURFACE_MODEL` |
| Face inner loops | `FACE_BOUND` |
| Void shells (cavity) | `BREP_WITH_VOIDS` (subtype of MANIFOLD\_SOLID\_BREP with `voids: SET OF ORIENTED_CLOSED_SHELL`), `ORIENTED_CLOSED_SHELL`. v0 rejects `BREP_WITH_VOIDS` and `ORIENTED_CLOSED_SHELL` with `STEP_UNSUPPORTED_ENTITY`. |
| Multiple bodies per file | More than one MANIFOLD\_SOLID\_BREP |
| Wireframe / curves-only | `GEOMETRICALLY_BOUNDED_WIREFRAME_SHAPE_REPRESENTATION` |
| Kinematic modules | AP242 Kinematics entities |
| Composite materials | AP242 Composite materials entities |
| Configuration management | `CONFIGURATION_DESIGN` entity set |
| Non-radian angle units (export) | Degrees on write (import accepts degrees with conversion) |

---

## 13. Citations and Authoritative References

### 13.1 Primary Normative Standards (paywalled)

ISO 10303-41 and -42 catalogue numbers are now verified (URLs below). ISO 10303-43 individual catalogue
page is not confirmed; the public SMRL source at ap238.org is used as the normative reference for
Part 42/43 geometry schema details in this document (documentation-only gap; not an encoding blocker).

| Standard | Full title | Verified catalogue URL | Status |
|---|---|---|---|
| ISO 10303-242:2014 | Part 242: Application protocol: Managed model-based 3D engineering (Ed1) | https://www.iso.org/standard/57628.html | Verified |
| ISO 10303-242:2020 | Same title (Ed2) | https://www.iso.org/standard/72021.html | Verified |
| ISO 10303-21:2016 | Part 21: Clear text encoding (Ed3) | https://www.iso.org/standard/67335.html | Verified |
| ISO 10303-42:2021 | Part 42: Geometric and topological representation | https://www.iso.org/standard/84672.html | Verified |
| ISO 10303-41:2020 | Part 41: Fundamentals of product description and support | https://www.iso.org/standard/78578.html | Verified |
| ISO 10303-43 (current edition) | Part 43: Representation structures | Public SMRL source used; ISO catalogue page not individually confirmed (documentation-only citation gap; not an encoding blocker) | See §13.2 |

### 13.2 Public Implementor Resources (free, no paywall)

| Resource | URL | Used for |
|---|---|---|
| AP242 project site | https://www.ap242.org/ | Conformance class overview (high-level only) |
| MBx-IF (CAx Implementor Forum) | https://www.mbx-if.org/ | Test round-trip files, recommended practices |
| MBx-IF recommended practices | https://www.mbx-if.org/cax/cax_recommPract.php | AP242 B-Rep implementor guidance |
| prostep ivip — MBx-IF overview | https://www.prostep.org/en/projects/mbx-interoperability-forum-mbx-if-1 | Forum context |
| STEP Tools — public EXPRESS schema browser | https://www.steptools.com/stds/stp_aim/html/schema.html | Entity definitions, WHERE rules (**authoritative public source for Section 4 allowlist**) |
| STEP Tools — SURFACE\_CURVE entity | https://www.steptools.com/stds/stp_aim/html/t_surface_curve.html | `LIST [1:2]` cardinality confirmed |
| STEP Tools — SEAM\_CURVE entity | https://www.steptools.com/stds/stp_aim/html/t_seam_curve.html | WHERE rules (SIZEOF=2, both PCURVE, same surface) confirmed |
| STEP Tools — CONICAL\_SURFACE entity | https://www.steptools.com/stds/stp_aim/html/t_conical_surface.html | `radius >= 0` WHERE rule confirmed |
| STEP Tools — FACE\_SURFACE entity | https://www.steptools.com/stds/stp_aim/html/t_face_surface.html | `same_sense` attribute name confirmed |
| STEP Tools — ORIENTED\_EDGE entity | https://www.steptools.com/stds/stp_aim/html/t_oriented_edge.html | `orientation` attribute name confirmed |
| ISO 10303-42:2021 public SMRL (ap238.org) | https://ap238.org/SMRL_v8_final/data/resource_docs/geometric_and_topological_representation/sys/4_schema.htm | Normative CONICAL\_SURFACE parameterisation formula; Part 43 representation schema reference |
| OCCT test STEP file (header-only reference, NOT an APD confirmation) | https://raw.githubusercontent.com/caimingzhi/reOCCT8_0_0rc3/7cba8120b41df9f7625598cdb4d16062be862e59/data/step/bug32745_pmi1.stp | PMI test file; contains AP242 FILE\_SCHEMA OID but **no APPLICATION\_PROTOCOL\_DEFINITION** entity; does not confirm `application_protocol_year=2014`; may be used as a header-parser test only |
| **CATIA OID fixture** — fpb\_assy\_v3.step (commit `bcc76aeb`) | https://raw.githubusercontent.com/AmedeoPelliccia/Robbbo-T_OLD/bcc76aeb5f612a7ead0567ebd49054c2e8166e48/C-AMEDEO-FRAMEWORK/CA-DEOPTIMISE/CAD-DESIGN/H2-BWB-Q100-CONF0000/AAA-ARCHITECTURES_AIRFRAMES_AERODYNAMICS/CE-CAD-Q100-AAA-ATA-53-FUSELAGE/CC/CE-CC-CAD-Q100-AAA-ATA-53-10-STRUCTURE-1/CI/CE-CC-CI-CAD-Q100-AAA-ATA-53-10-01-COMPONENT-1/3DModels/fpb_assy_v3.step | **OID evidence only** — FILE_SCHEMA confirms `{1 0 10303 442 1 1 4}`; APD entity has schema-invalid `$` for required `application`; not a conformant APD fixture (B-14) |
| **Onshape/ST-Developer AP242 Ed2 fixture** — cam\_wedge\_15.step (commit `51ca0ec`) | https://raw.githubusercontent.com/Vector-Wangel/XLeRobot/51ca0ec31bdb48713b94bacdba828bf8d889296b/hardware/misc/cam_wedge_15.step | Verified immutable: APD no `_mim_lf`, year 2020, OID `{1 0 10303 442 3 1 4}`; resolves B-02 |
| **NIST/GUID-data AP242 fixture** — box-guid.stp (commit `e0dbd5e`) | https://raw.githubusercontent.com/allisonfeeney/guid-data/e0dbd5ecd0e972105ba4a4d99858d93ae674ba48/box_model/box-guid.stp | Verified immutable: APD name `ap242_managed_model_based_3d_engineering_mim_lf`, bare FILE\_SCHEMA (no OID), year 2011; valid APD; resolves B-02 import year/name evidence |
| **Ryu crate v1.0.23** | https://crates.io/crates/ryu/1.0.23 (source: https://github.com/dtolnay/ryu/commit/22a692e0b27d9ca74231a475eb690a9446ed44af) | Pure Rust/no\_std/WASM canonical REAL formatter; resolves B-08 |
| Wikipedia — ISO 10303-21 | https://en.wikipedia.org/wiki/ISO_10303-21 | Part 21 syntax overview |
| Wikipedia — ISO 10303 | https://en.wikipedia.org/wiki/ISO_10303 | STEP overview |

### 13.3 Standards Uncertainties and Resolution Status

Items preceded by ✅ are **resolved** from public sources. Items preceded by ⚠ remain open (see also
Section 0 Blockers).

✅ **SEAM\_CURVE schema rules**: SEAM\_CURVE requires exactly 2 PCURVE entries in `associated_geometry`,
both referencing the same surface. Confirmed from STEP Tools public schema browser (WHERE clauses
visible at https://www.steptools.com/stds/stp_aim/html/t_seam_curve.html).

✅ **SURFACE\_CURVE `associated_geometry` cardinality**: `LIST [1:2]` — 1 or 2 entries permitted.
Confirmed from STEP Tools (https://www.steptools.com/stds/stp_aim/html/t_surface_curve.html).

✅ **PARAMETRIC\_REPRESENTATION\_CONTEXT**: Subtype of REPRESENTATION\_CONTEXT, no additional attributes.
Confirmed from publicly described Part 42 entity hierarchy.

✅ **Two seam PCURVEs must be distinct instances**: The u=0 and u=2π p-curves have different 2D start
points and therefore different LINE instances; they may share only the DIRECTION sub-entity.

✅ **Seam 3D LINE is not the axis**: For a cylinder seam at azimuth θ=0, the LINE passes through
(r, 0, 0), parallel to the axis, offset by radius r. For a cone seam at θ=0, the LINE is a generatrix
from the base point (R, 0, 0) to the apex. Neither is the axis.

✅ **Implementation levels**: Level `'2;1'` (Ed1), `'3;1'` (Ed2), `'4;1'`/`'4;2'`/`'4;3'` (Ed3).
No level `'6;1'` exists. Raw UTF-8 in strings is only valid at `'4;x'` levels; under `'2;1'` it is an error.
Confirmed from STEP Tools Part 21 Ed3 public draft (https://steptools.com/stds/step/IS_final_p21e3.html).

✅ **(B-01 resolved) CONICAL\_SURFACE parameterisation**: σ(u,v) = C + (R+v·tan α)·(cos u·x̂+sin u·ŷ) + v·ẑ;
apex at v = −R/tan α. Confirmed from ISO 10303-42:2021 public SMRL (ap238.org). Canonical writer emits
R=0 with placement at apex. See §4.4 and §6.4.

✅ **(B-02 resolved) `application_protocol_year`**: Two immutable fixtures have valid APD tuples (Onshape
year 2020, NIST year 2011). CATIA fixture downgraded to OID-only evidence (APD schema-invalid, B-14).
Writer policy: `_mim_lf`, year 2020. Import accepts 2011/2014/2020; OID whitelist `{...442 1/2/3 1 4}`.
Year is not an edition discriminator. See §8.3 and §13.2.

✅ **(B-03 resolved) DEFINITIONAL\_REPRESENTATION context**: `context_of_items` must include
`PARAMETRIC_REPRESENTATION_CONTEXT` (WHERE rule in Part 42). Confirmed from ISO 10303-42:2021 SMRL.
Dimension 2 from `GEOMETRIC_REPRESENTATION_CONTEXT(2)`. See §7.4.

✅ **(B-04 resolved) Complex entity component ordering**: ISO 10303-21:2016 §12.2.5.3 requires ascending
lexicographic order ('shall'). Confirmed from STEP Tools Part 21 Ed3 public draft. See §4.6.

✅ **(B-05 largely resolved)** ISO 10303-41:2020 (https://www.iso.org/standard/78578.html) and
ISO 10303-42:2021 (https://www.iso.org/standard/84672.html) confirmed. ISO 10303-43 individual catalogue
page not confirmed; public SMRL at ap238.org used as normative source; documentation-only gap.

⚠ **Conformance class numbers**: AP242 Ed2 Annex F defines class numbers; not confirmed from free
sources. Amphion does not assert a specific class number.

⚠ **DIMENSIONAL\_EXPONENTS for dimensionless units**: Whether radian/steradian units carry all-zero
exponents is stated in ISO 10303-41 (paywalled). All-zero is consistently observed in CAx-IF files.

✅ **(B-06 resolved) Cone apex loop**: Full EDGE\_LOOP specification frozen (§6.6): 3 items [E\_s.T.→Ptau,
E\_b.F., E\_s.F.→P0]; loop closure proved; PCURVE occurrence mapping explicit (§6.4). P-10/P-11 unblocked.

✅ **(B-07 resolved) Pcurve analytic identity proof**: 8-row analytic identity matrix (M1–M8) with
whole-interval algebraic invariants specified (§6.3). Slope invariant catches q(t)=(4πt,t). Certified
interval/transcendental backend is a mandatory `step-decode` dependency.

✅ **(B-08 resolved) Canonical real formatter**: Ryu crate `=1.0.23` pinned (source commit 22a692e0);
exact algorithm + grammar + 12 golden values documented (§3.4); CI bit-roundtrip gate required.

✅ **(B-09 resolved) Entity ID sharing-key circularity**: non-circular 6-phase writer algorithm
specified (§10.2). Structural keys computed bottom-up from content only (no IDs); interning by key;
two-pass ID assignment; completeness audit. No circularity possible.

✅ **(B-10 resolved) Unit pipeline**: §9.1 steps 6–7 rewritten. Unit factors resolved recursively
without double-scaling. Explicit staged conversion step 7 converts all physical-unit REALs to
metres/radians before domain validation. Degree cone positive (P-16) and domain-error negative (N-58)
added.

✅ **(B-11 resolved) M2–M6 coefficient identities**: full 3D coefficient equalities in §6.3 matrix;
M2 requires full point P₀=O+R·e(uᵢ)+v₀·Ẑ (axial offset checked); M4 requires derivative frame
equalities ② and ③ (not centre+normal alone); M5/M6 require ‖Oc−(O+h·Ẑ)‖ (full centre, not
height-only); M7 states single shared 3D curve. Counterexample-driven negatives N-55..N-57.

✅ **(B-12 resolved) N-52 representable**: redefined as SEAM\_CURVE.associated\_geometry = [Ptau,P0]
(order reversed in file); this is a file-observable protocol violation.

✅ **(B-13 resolved) Interning key type discriminator**: structural keys prefixed with entity-type-name
bytes; SELECT attributes preceded by alternative-type discriminator. CARTESIAN\_POINT and DIRECTION
with identical coordinates can never collide.

✅ **(B-14 resolved) CATIA APD malformed**: CATIA fixture downgraded to OID-only evidence; N-49
changed to synthetic valid APD test with verified OID `{1 0 10303 442 1 1 4}`.

✅ **(B-15 resolved) Per-attribute dimension table**: §9.1 step 7 now classifies every REAL-bearing
attribute; CYLINDER/CONE pcurve u is angular (×1, never scaled), v is length (×length\_factor);
mixed-basis 2D VECTOR scaled component-wise; CONVERSION\_BASED\_UNIT excluded from staging.

✅ **(B-16 resolved) Cone normalization order**: cone R>0 normalization elevated to step 9 (after
numeric validation, before trim/trig/M-matrix). shift=R/tan(α); apex correction; all pcurve
v-offsets updated; certified tan() with overflow check. M3/M6 now exclusively operate on R=0 form.

✅ **(B-17 resolved) Placement frame**: ê(u)=cos(u)X̂+sin(u)Ŷ, ê_θ(u)=−sin(u)X̂+cos(u)Ŷ defined
using AXIS2\_PLACEMENT\_3D local axes; raw (cos,sin,0) world-frame assumption eliminated from all M2/
M3/M5/M6 equations; tilted-placement negatives N-60 and positive P-07 require correct local frame.

✅ **(B-18 resolved) Curved+curved non-seam adjacency**: §4.5 cardinality table corrected to "not
supported in v0"; §6.3 M8 updated; curved+curved non-seam → STEP\_UNSUPPORTED\_ENTITY; N-59 added.

✅ **(B-19 resolved) §3/§10.2 algorithm and counts**: §3 sharing rule defers to typed structural keys
and explicit internable class list; 13→15 CARTESIAN\_POINT type-prefix length corrected; full Phase
A–F algorithm inline in §10.2 replacing self-referential pointer; evidence count corrected to two
valid APD fixtures (Onshape year-2020, NIST year-2011).

✅ **(B-20 resolved) Header/schema/timestamp/APD**: §3.2 impl-level 4;2/4;3 → `STEP_UNSUPPORTED_IMPL_LEVEL_CLASS`; FILE\_SCHEMA import defers to §8.3 whitelist; timestamp gains Z suffix in §3.2 and §3.4; APD sample corrected to `_mim_lf`/2020; 2014 compat profile removed from v0 output.

✅ **(B-21 resolved) Staging/τ/occurrence**: effective vector = mag×normalize(direction\_ratios) with normalize-first step; per-PCURVE-occurrence staging defined; raw τ is dimensionless; step 9 does NOT shift trim τ endpoints; step 12 inverse-maps τ from 3D vertex; uncertainty value emitted in file units (1.0E-7 m → `1.0E-7` in metre file, `1.0E-4` in mm file).

✅ **(B-22 resolved) Writer/world-frame prose**: §5.2 cone row says `radius=0, placement at apex`; §6.4 cylinder seam expressed in placement-frame ê(u), identity-placement example labelled; §6.6 cone seam/base-circle/cap in placement-frame with P-07/P-11 guidance; PCURVE order uses pre-ID occurrence key `(region,shell,face)`.

✅ **(B-23 resolved) Pcurve/trim/sense**: DEFINITIONAL\_REPRESENTATION exactly one 2D curve (N-61/N-62); `same_sense=.F.` import algorithm detailed (pcurve consistently reversed); v0 closed full circles only — arcs produce `STEP_UNSUPPORTED_ARC` (N-63); N-64 same\_sense reversal mismatch.

✅ **(B-24 resolved) Algorithm/fixtures**: Phase B internable-class list fully enumerated; `0xFF` eliminated; TLV encoding stated; SET duplicate defined via post-interning identity; Phase D executable root-table with all 19 named roles; N-32 fixed to SEAM\_CURVE; N-34 replaced with concrete R=0.05 inconsistency; N-60 fixed to 3D CIRCLE world-frame M5 ③ failure.

✅ **(B-25 resolved) §6.4 cylinder seam pnt includes v\_start**: formula `pnt = O + R·X̂ + v_start·Ẑ`; raw pcurve q(τ)=(uᵢ,v\_start+τ); trim τ∈[0,v\_end−v\_start]; §6.6 +Z/+X literals replaced with Ẑ/X̂ (labelled identity examples retained); M3 row documents raw wire b=1 and staged b=length\_factor.

✅ **(B-26 resolved) §6.7 cone frustum canonical mapping**: α=atan((r_l−r_s)/H); v\_min=r\_s/tan α; v\_max=v\_min+H; seam at u=0 from V\_near to V\_far, pcurves at u=0/TAU with v=v\_min+τ; two closed-circle caps; 4-item lateral EDGE\_LOOP with closure proof; far cap same\_sense=.T., near cap same\_sense=.F.; M3/M4/M6 applicability; P-09/P-12 exact wire values including tan\_a=0.3, v\_min=1/15 m, v\_max=1/6 m, trim=100 mm.

✅ **(B-27 resolved) Step 9 per-operation transactional arithmetic**: shift, each C' component, each pcurve v+shift all certified before IR mutation; `STEP_UNIT_OVERFLOW`/`STEP_UNIT_UNDERFLOW` on nonfinite or catastrophic-cancellation results; commit atomic after all checks pass (§9.1 step 9).

✅ **(B-28 resolved) Phase A occurrence paths and Phase B TLV kind tags**: Phase A assigns document-root ordinal for root nodes, structural path `(role, attr, idx, ...)` for non-root nodes, lexicographically least for shared nodes, kernel-SET order for SET indices (§10.2); Phase B TLV tags 0x00 ($), 0x01 (*), 0x02 (absent), 0x03+len+bytes (present) — SI\_UNIT($,…) and NAMED\_UNIT(*) keys constructible without ambiguity; N-34 replaced with R=0.05/α=π/4 exact residual 0.01414 m fixture.

✅ **(B-29 resolved) §6.6 all general world triples replaced with placement-frame**: V\_base=C+h·(tan α·X̂+Ž); DIRECTION sin α·X̂+cos α·Ž; P(τ)=C+τ·(tan α·X̂+Ž); VECTOR magnitude=sec α in the output length unit; identity-placement literal (sin α,0,cos α) retained in labelled example only.

✅ **(B-30 resolved) §6.7 seam always SEAM\_CURVE; orientation mapping corrected**: E\_s.T→Ptau\_lat/u=TAU/right boundary/near→far; E\_s.F→P0\_lat/u=0/left boundary/far→near; loop code-block comments, PCURVE mapping note, and unwrapped boundary (item 1=right/TAU, item 3=left/u=0) corrected; contradictory item-1-u=0 claim removed.

✅ **(B-31 resolved) §6.7.6 M-table rewritten**: M3×2 (each seam pcurve independently) + M7 (seam pairing); M6 for lateral CONICAL\_SURFACE LINE pcurves q=(t,v\_const); M4 for cap PLANE CIRCLE pcurves; M5 (CIRCLE on CYLINDER) explicitly excluded; both pcurves of each SURFACE\_CURVE must pass their row.

✅ **(B-32 resolved) §6.7 raw/staged τ and VECTOR.magnitude**: τ dimensionless and never staged; numeric interval [0,Δv\_wire] unchanged after staging; VECTOR.magnitude is a STEP length measure in the output unit; b stages to length\_factor m/τ; raw\_tau\_upper\_numeric renamed; P-12 coefficient check: 100×1e-3×sec α≈0.1044 m = P-09 axial delta ✓.

✅ **(B-33 resolved) AXIS2\_PLACEMENT\_3D frame derivation**: Ž=normalize(axis); Ŷ=normalize(Ž×ref\_direction); X̂=Ŷ×Ž (right-handed; X̂ is the normalized projection of raw ref\_direction ⊥ to Ž). Reject zero axis, parallel axis/ref, or cert failure → `STEP_INVALID_FRAME`. Updated in §4.4, §4.5, §6.3. P-17 (skew ref\_direction accepted); N-46 (parallel rejected) retained.

✅ **(B-34 resolved) §4.5 curved+planar pcurve types**: curved lateral = 2D LINE q=(φ±t,v\_const) (M5/M6); planar cap = 2D CIRCLE (M4); planar+planar = both 2D LINEs; any unlisted pairing unsupported → `STEP_UNSUPPORTED_ENTITY`.

✅ **(B-35 resolved) §6.7.3 M4/M6 sentence corrected**: M6 checks lateral CONICAL\_SURFACE LINE pcurve (q=(t,v\_const)) — centre C+v_const·Ž, radius v_const·tan α; M4 checks cap PLANE CIRCLE pcurve. Consistent with §6.7.6.

✅ **(B-36 resolved) Step 8 cone radius validation**: R < 0 → `STEP_INVALID_PARAMETER` before step 9; R=0 accepted (apex); R>0 → step 9. Fixture N-65 added.

✅ **(B-37 resolved) Region SemanticId**: Region added to introductory provenance list; `ADVANCED_BREP_SHAPE_REPRESENTATION` row added to derivation table; import reconstruction rule (same regex, fallback synthetic ID); writer table Region source clarified: `Region.provenance.semantic_id` only, no alternate source.

✅ **(B-38 resolved) SemanticId extension 45 chars**: `len("amphion.id/1/")=13`+32=45 (not 46). P-17b (exact 45-char accepted); N-66 (31 hex/display-only); N-67 (33 hex/display-only); N-68 (uppercase hex/display-only) added.

✅ **(B-39 resolved) §6.6 base-circle lateral cone pcurve pnt corrected**: `pnt=(0,h)` gives q(t)=(0,h)+t·(1,0)=(t,h) ✓; previous `pnt=(0,0)` gave q=(t,0) contradicting M6 and the defined base height h.

✅ **(B-40 resolved) Step 9 immutability**: parsed raw entity graph never mutated. Allocates occurrence-local canonical IR: new CARTESIAN\_POINT at C', new AXIS2\_PLACEMENT\_3D (reuses original DIRECTION helpers), new CONICAL\_SURFACE R=0, new 2D points/LINEs/DEFINITIONAL\_REPRESENTATIONs for each pcurve; all raw shared entities unchanged. Transactional: any arithmetic failure leaves IR in original state (no partial rebind). New canonical nodes participate in Phase A/B/C/D interning normally. P-18 (shared placement cone+plane) and N-69 (overflow, no partial rebind) added.

✅ **(B-41 resolved) N-64 repaired and §4.3/§6.5 simultaneous reversal explicit**: EDGE\_CURVE.same\_sense=.F. simultaneously reverses BOTH canonical 3D CIRCLE F\_y sign (ε\_3d) AND canonical pcurve ε; independent of ORIENTED\_EDGE.orientation. N-64 now: raw q=(φ−t,h) with same\_sense=.F. → canonical F\_y\_canonical=−R·ê\_θ, canonical ε=+1 → M5 ③ residual 2R >> ε\_l. §6.5 note added.

✅ **(B-42 resolved) PLANE\_ANGLE\_UNIT required when cone present**: step 6 checks for exactly one resolvable PLANE\_ANGLE\_UNIT (radian or degree) when any CONICAL\_SURFACE is in the data section; missing/ambiguous/unsupported → `STEP_INVALID_UNIT`, no Body. Cone-free files (PLANE/CYLINDER only) may omit angle unit with a deterministic non-fatal warning; angle\_factor defaults to 1.0. §7.3 rewritten; N-70 (missing angle unit + cone → error) and P-19a/P-19b (missing angle unit + cuboid/cylinder → warning) added.

✅ **(B-43 resolved) N-69 corrected with certified overflow values**: R=1.1e308 m (finite), α=π/6; shift = 1.1e308/tan(π/6) = 1.1e308×√3 ≈ 1.905e308 > f64::MAX ≈ 1.798e308; interval arithmetic lower bound exceeds f64::MAX → `STEP_UNIT_OVERFLOW`; shared PLANE placement byte-identical to raw input, no allocations.

✅ **(B-44 resolved) Step 6 cardinality-exact algorithm**: collect all PLANE\_ANGLE\_UNIT members from context; count supported (radian/degree) and unsupported separately. Cone-present: resolved\_count≠1 → STEP\_INVALID\_UNIT; resolved\_count=1 → proceed. Cone-free: count=0 → warning+1.0; count=1 → use; count>1 or unsupported → STEP\_INVALID\_UNIT. Deterministic selection from multiple units is forbidden.

✅ **(B-45 resolved) N-71 added**: cone + two PLANE\_ANGLE\_UNIT members (radian + degree, resolved\_count=2) → STEP\_INVALID\_UNIT; proves the >1 cardinality rule that N-70 alone cannot cover. Acceptance gate updated.

✅ **(B-46 resolved) P-19 split into P-19a/P-19b**: P-19a = cuboid no angle unit → warning; P-19b = cylinder (seam + cap pcurve) no angle unit → warning; pcurve u-coordinates (u=0/TAU, cap φ) remain canonical radians unaffected by angle\_factor absence.

✅ **(B-47 resolved) B-43 row and N-69 corrected**: exact real shift≈1.905e308 is a finite real but has no finite f64 enclosure; certified interval lower\_bound > f64::MAX before any f64 materialization (not simply hardware overflow to inf). N-69 now includes valid LENGTH\_UNIT + PLANE\_ANGLE\_UNIT so B-42 cannot preempt the intended overflow diagnostic.

---

*End of document.*
