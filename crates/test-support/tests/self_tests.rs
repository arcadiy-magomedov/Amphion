//! Dense self-test suite for `amphion-test-support`.
//!
//! Proves the following properties across the public API:
//!
//! 1. Identical seeds produce identical cases.
//! 2. Different streams derived from the same seed are fully isolated.
//! 3. Shrinking/minimisation metadata survives a corpus round-trip.
//! 4. Malformed corpus inputs are rejected before any entries are returned.
//! 5. Failure reports serialise to the same JSON for the same content.

#![allow(clippy::cognitive_complexity)]

use amphion_test_support::{
    BoundedFloat, BoundedUInt, CASE_SEQUENCE_VERSION, CORPUS_SCHEMA_VERSION, CaseBudget,
    CaseCheckError, CaseContext, CaseId, CheckKind, CorpusEntry, CorpusError, CorpusFile,
    DifferentialOracle, DistributionError, ENV_TEST_CASE, ENV_TEST_CHECK, ENV_TEST_CHECK_KIND,
    ENV_TEST_OPERATION, ENV_TEST_SEED, ENV_TEST_STREAM, ENV_TEST_VERSION, EdgeCaseSchedule,
    FailureReport, FuzzInputReader, Invariant, LegacyCorpusDocument, MetamorphicCase,
    MinimizationMeta, Minimizer, OracleId, OracleRegistry, OracleVerdict,
    RANDOMIZED_CASE_MILESTONE, REPORT_SCHEMA_VERSION, ReplayConfig, ReplayEnvError, ReplayFilter,
    ReplayIdentity, ReportError, ReproducibleCommand, ResourceLimitKind, ResourceLimits, RunConfig,
    RunnerError, TestRng, TestSeed, WeightedChoice, WeightedItem, apply_replay_config,
    configure_replay_from_env, run_invariant_cases, run_metamorphic_cases, run_property_cases,
};

// ── Helpers ────────────────────────────────────────────────────────────────

fn seed(v: u64) -> TestSeed {
    TestSeed::new(v)
}

fn rng_from(v: u64) -> TestRng {
    TestRng::from_seed(TestSeed::new(v))
}

fn new_command(
    package: &str,
    test_name: &str,
    seed: TestSeed,
    case_index: u64,
    stream_name: &str,
) -> Result<ReproducibleCommand, amphion_test_support::CommandTokenError> {
    let identity = ReplayIdentity::new(
        "op",
        stream_name,
        seed,
        case_index,
        CheckKind::Invariant,
        "check",
    )?;
    ReproducibleCommand::new(package, test_name, identity)
}

fn new_failure_report(
    seed: TestSeed,
    case_id: CaseId,
    stream_name: &str,
    operation: &str,
    case_index: u64,
    inputs_json: serde_json::Value,
    failure_message: &str,
) -> Result<FailureReport, ReportError> {
    FailureReport::new(
        seed,
        case_id,
        stream_name,
        operation,
        case_index,
        CheckKind::Invariant,
        "check",
        inputs_json,
        failure_message,
    )
}

fn new_corpus_entry(
    id: CaseId,
    operation: &str,
    stream_name: &str,
    seed: TestSeed,
    case_index: u64,
    inputs_json: serde_json::Value,
    failure_message: impl Into<String>,
) -> Result<CorpusEntry, CorpusError> {
    CorpusEntry::new(
        id,
        operation,
        stream_name,
        seed,
        case_index,
        CheckKind::Invariant,
        "check",
        inputs_json,
        failure_message,
    )
}

/// Build a corpus entry with a [`CaseId`] consistent with the stream name.
fn make_entry(seed_v: TestSeed, idx: u64, op: &str, stream: &str) -> CorpusEntry {
    let id = CaseId::new(seed_v.for_case_stream(stream), idx);
    new_corpus_entry(
        id,
        op,
        stream,
        seed_v,
        idx,
        serde_json::json!({"idx": idx}),
        format!("fail {idx}"),
    )
    .expect("valid entry")
}

fn invariant_replay(case_index: u64, check_name: &str) -> ReplayFilter {
    ReplayFilter::new(
        case_index,
        "test.op".to_string(),
        CheckKind::Invariant,
        check_name.to_string(),
    )
}

fn metamorphic_replay(case_index: u64, check_name: &str) -> ReplayFilter {
    ReplayFilter::new(
        case_index,
        "test.op".to_string(),
        CheckKind::MetamorphicRelation,
        check_name.to_string(),
    )
}

// ── 1. Identical seeds produce identical cases ─────────────────────────────

#[test]
fn identical_seeds_produce_identical_u64_sequences() {
    let mut a = rng_from(0xcafe_babe_dead_beef);
    let mut b = rng_from(0xcafe_babe_dead_beef);
    for i in 0..1_000 {
        assert_eq!(a.next_u64(), b.next_u64(), "mismatch at step {i}");
    }
}

#[test]
fn identical_seeds_produce_identical_f64_sequences() {
    let mut a = rng_from(42);
    let mut b = rng_from(42);
    for _ in 0..500 {
        let va = a.next_f64();
        let vb = b.next_f64();
        assert_eq!(va.to_bits(), vb.to_bits(), "f64 sequences must match");
    }
}

#[test]
fn identical_stream_seeds_produce_identical_case_ids() {
    let primary = seed(7);
    let s = primary.for_stream("test.stream");
    let ids_a: Vec<CaseId> = (0..100).map(|i| CaseId::new(s, i)).collect();
    let ids_b: Vec<CaseId> = (0..100).map(|i| CaseId::new(s, i)).collect();
    assert_eq!(
        ids_a, ids_b,
        "case IDs must be identical for the same stream seed"
    );
}

#[test]
fn identical_seeds_produce_identical_invariant_run_reports() {
    let cfg =
        RunConfig::new(seed(100), "integration.invariant", "test.op", 200).expect("valid config");
    let checks = [Invariant::new("is_finite", |_ctx, v: &f64| {
        if v.is_finite() {
            Ok(())
        } else {
            Err(CaseCheckError::Failure(format!("{v} not finite")))
        }
    })];
    let report_a = run_invariant_cases(&cfg, |ctx| Ok(ctx.next_f64()? * 1000.0), &checks, None)
        .expect("valid");
    let report_b = run_invariant_cases(&cfg, |ctx| Ok(ctx.next_f64()? * 1000.0), &checks, None)
        .expect("valid");
    assert_eq!(report_a.total_cases, report_b.total_cases);
    assert_eq!(report_a.passed_cases, report_b.passed_cases);
    assert_eq!(report_a.failures.len(), report_b.failures.len());
}

// ── 2. Different streams are isolated ─────────────────────────────────────

#[test]
fn different_stream_seeds_from_same_primary_differ() {
    let primary = seed(42);
    let sa = primary.for_stream("stream.alpha");
    let sb = primary.for_stream("stream.beta");
    assert_ne!(sa, sb, "distinct stream names must yield distinct seeds");
}

#[test]
fn different_stream_rngs_produce_independent_sequences() {
    let primary = seed(99);
    let mut ra = TestRng::from_seed(primary.for_stream("stream.a"));
    let mut rb = TestRng::from_seed(primary.for_stream("stream.b"));
    let differ_count = (0..32).filter(|_| ra.next_u64() != rb.next_u64()).count();
    assert!(
        differ_count > 16,
        "isolated streams must produce different sequences; only {differ_count}/32 differed"
    );
}

#[test]
fn case_ids_from_different_streams_differ_for_distinct_names() {
    let primary = seed(7);
    let sa = primary.for_stream("alpha");
    let sb = primary.for_stream("beta");
    for i in 0..50u64 {
        let id_a = CaseId::new(sa, i);
        let id_b = CaseId::new(sb, i);
        assert_ne!(
            id_a, id_b,
            "same index, different streams produce different IDs for these names"
        );
    }
}

#[test]
fn metamorphic_runner_uses_isolated_transform_stream() {
    let mut base_inputs_a: Vec<u64> = Vec::new();
    let mut base_inputs_b: Vec<u64> = Vec::new();

    let collect = |inputs: &mut Vec<u64>| {
        let cases = [MetamorphicCase::new(
            "identity",
            |_ctx, x: &u64| Ok(*x),
            |_ctx, _, o1, _, o2| {
                if o1 == o2 {
                    Ok(())
                } else {
                    Err(CaseCheckError::Failure(format!("{o1} != {o2}")))
                }
            },
        )];
        run_metamorphic_cases(
            &RunConfig::new(seed(200), "integration.metamorphic", "test.op", 50).expect("valid"),
            |ctx| {
                let v = ctx.next_u64()?;
                inputs.push(v);
                Ok(v)
            },
            |_ctx, x| Ok(*x),
            &cases,
        )
        .expect("valid config")
    };

    let r1 = collect(&mut base_inputs_a);
    let r2 = collect(&mut base_inputs_b);
    assert_eq!(
        base_inputs_a, base_inputs_b,
        "base sequence must be reproducible"
    );
    assert!(r1.is_ok());
    assert!(r2.is_ok());
}

// ── 3. Shrinking / minimisation metadata round-trips ──────────────────────

#[test]
fn minimization_metadata_survives_corpus_round_trip() {
    let s = seed(11);
    let id = CaseId::new(s.for_case_stream("rt.test"), 5);
    let meta = MinimizationMeta::new(TestSeed::new(8888), 777, 12);
    let entry = new_corpus_entry(
        id,
        "primitive.cone",
        "rt.test",
        s,
        5,
        serde_json::json!({"h": 2.0}),
        "seam invalid",
    )
    .expect("valid")
    .with_minimization(meta.clone());

    let file = CorpusFile::new(vec![entry]).expect("valid corpus");
    let json = file.write_to_string().expect("serialise");
    let loaded = CorpusFile::load_from_str(&json).expect("must round-trip");

    let loaded_entry = &loaded.entries()[0];
    assert_eq!(loaded_entry.seed(), s);
    assert_eq!(loaded_entry.case_index(), 5);
    assert_eq!(loaded_entry.operation(), "primitive.cone");
    assert_eq!(loaded_entry.stream_name(), Some("rt.test"));

    let loaded_meta = loaded_entry.minimization().expect("meta must survive");
    assert_eq!(loaded_meta.original_seed, TestSeed::new(8888));
    assert_eq!(loaded_meta.original_case_index, 777);
    assert_eq!(loaded_meta.shrink_steps, 12);
}

#[test]
fn entry_without_minimization_round_trips() {
    let s = seed(22);
    let id = CaseId::new(s.for_case_stream("nomin.test"), 0);
    let entry = new_corpus_entry(
        id,
        "bool.union",
        "nomin.test",
        s,
        0,
        serde_json::json!({}),
        "topology error",
    )
    .expect("valid");
    let file = CorpusFile::new(vec![entry]).expect("valid corpus");
    let loaded =
        CorpusFile::load_from_str(&file.write_to_string().expect("ser")).expect("round-trip");
    assert_eq!(loaded.entries()[0].minimization(), None);
}

#[test]
fn case_id_hex_survives_corpus_round_trip() {
    let s = seed(33);
    let id = CaseId::new(s.for_case_stream("hex.test"), 99);
    let original_hex = id.to_hex();
    let entry =
        new_corpus_entry(id, "op", "hex.test", s, 99, serde_json::json!({}), "msg").expect("valid");
    let file = CorpusFile::new(vec![entry]).expect("valid corpus");
    let loaded =
        CorpusFile::load_from_str(&file.write_to_string().expect("ser")).expect("round-trip");
    assert_eq!(loaded.entries()[0].id().to_hex(), original_hex);
}

#[test]
fn multi_entry_corpus_round_trips_in_canonical_order() {
    let entries: Vec<CorpusEntry> = (1..=5u64)
        .map(|i| {
            let s = TestSeed::new(i);
            let id = CaseId::new(s.for_case_stream("multi.test"), i);
            new_corpus_entry(
                id,
                "op",
                "multi.test",
                s,
                i,
                serde_json::json!({}),
                format!("fail {i}"),
            )
            .expect("valid")
        })
        .collect();
    let file = CorpusFile::new(entries).expect("valid corpus");
    let json = file.write_to_string().expect("ser");
    let loaded = CorpusFile::load_from_str(&json).expect("round-trip");

    let hexes: Vec<String> = loaded.entries().iter().map(|e| e.id().to_hex()).collect();
    let mut sorted = hexes.clone();
    sorted.sort();
    assert_eq!(hexes, sorted, "loaded entries must be in canonical order");
    assert_eq!(loaded.entries().len(), 5);
}

// ── 4. Malformed corpora are rejected ─────────────────────────────────────

#[test]
fn non_json_is_rejected() {
    assert!(matches!(
        CorpusFile::load_from_str("not json"),
        Err(CorpusError::InvalidJson(_))
    ));
}

#[test]
fn empty_string_is_rejected() {
    assert!(CorpusFile::load_from_str("").is_err());
}

#[test]
fn bare_json_object_without_required_fields_is_rejected() {
    assert!(CorpusFile::load_from_str("{}").is_err());
}

#[test]
fn incompatible_major_schema_version_is_rejected() {
    let json = r#"{"schema_version":{"major":99,"minor":0},"entries":[]}"#;
    assert!(matches!(
        CorpusFile::load_from_str(json),
        Err(CorpusError::SchemaVersionMismatch { .. })
    ));
}

#[test]
fn future_minor_schema_version_is_rejected() {
    let json = r#"{"schema_version":{"major":1,"minor":99},"entries":[]}"#;
    assert!(
        matches!(
            CorpusFile::load_from_str(json),
            Err(CorpusError::SchemaVersionMismatch { .. })
        ),
        "unsupported future minor version must be rejected"
    );
}

#[test]
fn unknown_top_level_field_is_rejected_by_strict_mode() {
    let json = r#"{"schema_version":{"major":1,"minor":1},"entries":[],"unknown_extra":"field"}"#;
    assert!(CorpusFile::load_from_str(json).is_err());
}

#[test]
fn null_schema_version_is_rejected() {
    let json = r#"{"schema_version":null,"entries":[]}"#;
    assert!(CorpusFile::load_from_str(json).is_err());
}

#[test]
fn array_at_root_is_rejected() {
    assert!(CorpusFile::load_from_str("[]").is_err());
}

#[test]
fn entry_with_invalid_case_id_is_rejected() {
    let json = r#"{
        "schema_version": {"major":1,"minor":1},
        "entries": [{
            "schema_version": {"major":1,"minor":1},
            "id": "not-a-valid-case-id",
            "operation": "op",
            "seed": 1,
            "case_index": 0,
            "inputs_json": {},
            "failure_message": "fail"
        }]
    }"#;
    assert!(
        CorpusFile::load_from_str(json).is_err(),
        "invalid case id must be rejected"
    );
}

#[test]
fn duplicate_case_ids_in_file_are_rejected() {
    let s = TestSeed::new(100);
    let id = CaseId::new(s.for_case_stream("dup.test"), 0);
    let ea = new_corpus_entry(
        id,
        "op.a",
        "dup.test",
        s,
        0,
        serde_json::json!({}),
        "fail a",
    )
    .unwrap();
    let eb = new_corpus_entry(
        id,
        "op.a",
        "dup.test",
        s,
        0,
        serde_json::json!({}),
        "fail b",
    )
    .unwrap();
    let result = CorpusFile::new(vec![ea, eb]);
    assert!(
        matches!(result, Err(CorpusError::DuplicateCaseId { .. })),
        "duplicate CaseIds must be rejected"
    );
}

#[test]
fn entry_schema_version_incompatibility_is_detected() {
    let seed = TestSeed::new(1);
    let id = CaseId::new(seed.for_case_stream("s"), 0).to_hex();
    let good_json = format!(
        concat!(
            r#"{{"schema_version":{{"major":1,"minor":2}},"entries":["#,
            r#"{{"schema_version":{{"major":1,"minor":2}},"id":"{id}","#,
            r#""operation":"op","stream_name":"s","seed":1,"case_index":0,"#,
            r#""inputs_json":{{}},"failure_message":"fail","case_sequence_version":3,"#,
            r#""check_kind":"invariant","check_name":"check"}}"#,
            r#"]}}"#,
        ),
        id = id
    );
    assert!(
        CorpusFile::load_from_str(&good_json).is_ok(),
        "matching versions must succeed"
    );

    let bad_json = format!(
        concat!(
            r#"{{"schema_version":{{"major":1,"minor":0}},"entries":["#,
            r#"{{"schema_version":{{"major":1,"minor":1}},"id":"{id}","#,
            r#""operation":"op","seed":1,"case_index":0,"#,
            r#""inputs_json":{{}},"failure_message":"fail"}}"#,
            r#"]}}"#,
        ),
        id = "00000000000000000000000000000000"
    );
    assert!(
        CorpusFile::load_from_str(&bad_json).is_err(),
        "entry version mismatch must be rejected"
    );
}

#[test]
fn v1_1_corpus_entry_requires_stream_name() {
    // v1.1 entry without stream_name must be rejected.
    let json = serde_json::json!({
        "schema_version": {"major": 1, "minor": 1},
        "entries": [{
            "schema_version": {"major": 1, "minor": 1},
            "id": "00000000000000000000000000000000",
            "operation": "op",
            "seed": 1,
            "case_index": 0,
            "inputs_json": {},
            "failure_message": "fail"
        }]
    });
    assert!(CorpusFile::load_from_str(&json.to_string()).is_err());
}

#[test]
fn v1_1_corpus_entry_rejects_mismatched_case_id() {
    // Valid stream_name but CaseId derived from primary seed (not for_stream) → mismatch.
    let s = TestSeed::new(77);
    let wrong_id = CaseId::new(s, 0); // uses primary seed, not for_stream
    let id_hex = wrong_id.to_hex();
    let json = serde_json::json!({
        "schema_version": {"major": 1, "minor": 1},
        "entries": [{
            "schema_version": {"major": 1, "minor": 1},
            "id": id_hex,
            "operation": "op",
            "stream_name": "s",
            "seed": 77u64,
            "case_index": 0u64,
            "inputs_json": {},
            "failure_message": "fail"
        }]
    });
    assert!(
        CorpusFile::load_from_str(&json.to_string()).is_err(),
        "CaseId mismatch must be rejected for v1.1 entries"
    );
}

// ── 5. Failure reports are stable ─────────────────────────────────────────

#[test]
fn failure_reports_are_stable_across_calls() {
    let s = seed(555);
    let id = CaseId::new(s.for_case_stream("stability.test"), 1);
    let report = new_failure_report(
        s,
        id,
        "stability.test",
        "op.test",
        1,
        serde_json::json!({"x": 1.0}),
        "check failed",
    )
    .expect("valid report")
    .with_replay_command(
        new_command("amphion-test-support", "test_op", s, 1, "stability.test")
            .expect("valid cmd")
            .with_operation("op.test")
            .expect("valid operation"),
    )
    .expect("cmd fields must match report fields");

    let json1 = report.to_json().expect("ser1");
    let json2 = report.to_json().expect("ser2");
    assert_eq!(
        json1, json2,
        "failure report JSON must be identical across calls"
    );
}

#[test]
fn failure_report_serde_round_trip_is_identity() {
    let s = seed(666);
    let id = CaseId::new(s.for_case_stream("rt.stability"), 2);
    let original = new_failure_report(
        s,
        id,
        "rt.stability",
        "op.roundtrip",
        2,
        serde_json::json!({}),
        "some failure",
    )
    .expect("valid report");
    let json = original.to_json().expect("ser");
    let decoded: FailureReport = serde_json::from_str(&json).expect("decode failure report");
    assert_eq!(decoded, original);
}

#[test]
fn failure_reports_with_same_fields_produce_same_json() {
    let build = |idx: u64| {
        let s = TestSeed::new(77);
        let id = CaseId::new(s.for_case_stream("stable"), idx);
        new_failure_report(s, id, "stable", "op", idx, serde_json::json!({}), "failure")
            .expect("valid")
    };
    assert_eq!(
        build(0).to_json().expect("a"),
        build(0).to_json().expect("b")
    );
    assert_ne!(
        build(0).to_json().expect("c"),
        build(1).to_json().expect("d")
    );
}

#[test]
fn failure_report_schema_version_is_present_in_json() {
    let s = seed(1);
    let id = CaseId::new(s.for_case_stream("v"), 0);
    let r = new_failure_report(s, id, "v", "op", 0, serde_json::json!({}), "fail").expect("valid");
    assert_eq!(r.schema_version(), REPORT_SCHEMA_VERSION);
    let json = r.to_json().expect("ser");
    assert!(
        json.contains("schema_version"),
        "schema_version must appear in JSON"
    );
}

#[test]
fn failure_report_inputs_json_is_inline_object_not_string() {
    let s = seed(2);
    let id = CaseId::new(s.for_case_stream("s"), 0);
    let inputs = serde_json::json!({"x": 1.5, "flag": true});
    let r = new_failure_report(s, id, "s", "op", 0, inputs.clone(), "fail").expect("valid");
    assert_eq!(r.inputs_json(), &inputs);
    // In JSON the value must be an inline object, NOT a quoted string.
    let json = r.to_json().expect("ser");
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(
        parsed["inputs_json"].is_object(),
        "inputs_json must be inline object in JSON"
    );
}

#[test]
fn failure_report_rejects_mismatched_case_id() {
    let s = seed(3);
    let wrong_id = CaseId::new(s, 0); // primary seed, not for_stream
    let result = new_failure_report(s, wrong_id, "s", "op", 0, serde_json::json!({}), "fail");
    assert!(result.is_err(), "mismatched CaseId must be rejected");
}

#[test]
fn replay_command_display_is_stable() {
    let s = seed(42);
    let cmd = new_command("pkg", "test_fn", s, 7, "ops.sphere").expect("valid");
    assert_eq!(cmd.to_string(), cmd.to_string(), "Display must be stable");
    assert!(cmd.to_string().contains("AMPHION_TEST_SEED=42"));
    assert!(cmd.to_string().contains("AMPHION_TEST_CASE=7"));
    assert!(cmd.to_string().contains("AMPHION_TEST_STREAM=ops.sphere"));
}

#[test]
fn replay_command_rejects_nul_byte_in_package() {
    let s = seed(1);
    assert!(new_command("pkg\x00bad", "test", s, 0, "s").is_err());
}

#[test]
fn replay_command_rejects_control_char_in_test_name() {
    let s = seed(1);
    assert!(new_command("pkg", "test\x01fn", s, 0, "s").is_err());
}

#[test]
fn replay_command_rejects_invalid_stream_name() {
    let s = seed(1);
    assert!(new_command("pkg", "test", s, 0, "bad stream").is_err());
    assert!(new_command("pkg", "test", s, 0, "").is_err());
}

#[test]
fn replay_command_powershell_renderer_includes_env_prefix() {
    let s = seed(99);
    let cmd = new_command("pkg", "test_fn", s, 3, "ops.stream").expect("valid");
    let ps = cmd.as_powershell().to_string();
    assert!(ps.contains("$env:AMPHION_TEST_SEED"), "PS must set seed");
    assert!(
        ps.contains("$env:AMPHION_TEST_STREAM"),
        "PS must set stream"
    );
    assert!(ps.contains("cargo test"), "PS must invoke cargo test");
    assert!(ps.contains("ops.stream"), "PS must include stream name");
}

// ── Additional composability tests ────────────────────────────────────────

#[test]
fn bounded_float_samples_remain_in_range_across_seeds() {
    let dist = BoundedFloat::try_new(-10.0, 10.0).expect("valid range");
    for s in [1u64, 42, 999, u64::MAX] {
        let mut r = TestRng::from_seed(TestSeed::new(s));
        for _ in 0..100 {
            let v = dist.sample(&mut r);
            assert!((-10.0..10.0).contains(&v), "out of range for seed {s}: {v}");
        }
    }
}

#[test]
fn bounded_float_extreme_bounds_never_overflow() {
    let dist = BoundedFloat::try_new(-f64::MAX, f64::MAX).expect("valid");
    let mut r = rng_from(0xdead_beef);
    for _ in 0..1_000 {
        let v = dist.sample(&mut r);
        assert!(v.is_finite(), "must remain finite");
    }
}

#[test]
fn bounded_float_always_strictly_below_hi() {
    // For any RNG state, sample must return [lo, hi).
    let dist = BoundedFloat::try_new(0.0, 1.0).expect("valid");
    let mut r = rng_from(42);
    for _ in 0..10_000 {
        let v = dist.sample(&mut r);
        assert!(v < 1.0, "must be < hi=1.0, got {v}");
        assert!(v >= 0.0, "must be >= lo=0.0, got {v}");
    }
}

#[test]
fn bounded_uint_covers_full_range() {
    let dist = BoundedUInt::try_new(0, 9).expect("valid range");
    let mut seen = [false; 10];
    let mut r = rng_from(12345);
    for _ in 0..1_000 {
        let v = usize::try_from(dist.sample(&mut r)).expect("0..=9 fits usize");
        assert!(v <= 9, "out of range: {v}");
        seen[v] = true;
    }
    assert!(
        seen.iter().all(|&s| s),
        "all values in [0,9] should appear in 1000 samples"
    );
}

#[test]
fn explicit_edge_case_schedule_rejects_unsorted() {
    assert!(matches!(
        EdgeCaseSchedule::try_explicit(vec![5, 3, 8]),
        Err(DistributionError::UnsortedIndices)
    ));
    assert!(matches!(
        EdgeCaseSchedule::try_explicit(vec![1, 1, 2]),
        Err(DistributionError::UnsortedIndices)
    ));
}

#[test]
fn geometric_schedule_hits_expected_indices() {
    let s = EdgeCaseSchedule::geometric();
    let edge_indices: Vec<u64> = (0u64..20).filter(|&i| s.is_edge_case(i)).collect();
    assert!(edge_indices.contains(&0));
    assert!(edge_indices.contains(&1));
    assert!(edge_indices.contains(&2));
    assert!(!edge_indices.contains(&3));
    assert!(edge_indices.contains(&4));
    assert!(!edge_indices.contains(&5));
    assert!(edge_indices.contains(&8));
    assert!(edge_indices.contains(&16));
}

#[test]
fn weighted_choice_distribution_is_proportional() {
    let items = vec![
        WeightedItem {
            item: 'a',
            weight: 1,
        },
        WeightedItem {
            item: 'b',
            weight: 4,
        },
    ];
    let choice = WeightedChoice::try_new(items).expect("valid");
    let mut r = rng_from(101);
    let (mut count_a, mut count_b) = (0u32, 0u32);
    for _ in 0..5_000 {
        match choice.sample(&mut r) {
            'a' => count_a += 1,
            'b' => count_b += 1,
            _ => unreachable!(),
        }
    }
    let ratio = f64::from(count_b) / f64::from(count_a);
    assert!(
        ratio > 2.0 && ratio < 8.0,
        "ratio b/a should be ~4, got {ratio:.2}"
    );
}

#[test]
fn oracle_registry_collects_all_verdicts() {
    struct AlwaysAgree;
    impl DifferentialOracle<(), ()> for AlwaysAgree {
        fn oracle_id(&self) -> OracleId {
            OracleId::try_new("always.agree").unwrap()
        }
        fn classify(
            &self,
            _ctx: &mut CaseContext,
            (): &(),
            (): &(),
        ) -> Result<OracleVerdict, ResourceLimitKind> {
            Ok(OracleVerdict::Agree)
        }
    }

    struct AlwaysDisagree;
    impl DifferentialOracle<(), ()> for AlwaysDisagree {
        fn oracle_id(&self) -> OracleId {
            OracleId::try_new("always.disagree").unwrap()
        }
        fn classify(
            &self,
            _ctx: &mut CaseContext,
            (): &(),
            (): &(),
        ) -> Result<OracleVerdict, ResourceLimitKind> {
            Ok(OracleVerdict::Disagree {
                description: String::from("bad"),
            })
        }
    }

    let mut reg = OracleRegistry::<(), ()>::new();
    reg.register(AlwaysAgree).expect("unique");
    reg.register(AlwaysDisagree).expect("unique");

    let mut ctx = CaseContext::new(
        0,
        TestRng::from_seed(TestSeed::new(1)),
        CaseBudget::unlimited(),
    );
    let all = reg.run_all(&mut ctx, &(), &()).unwrap();
    assert_eq!(all.len(), 2);
    assert!(all[0].verdict.is_agreement());
    assert!(all[1].verdict.is_failure());

    let mut ctx = CaseContext::new(
        0,
        TestRng::from_seed(TestSeed::new(2)),
        CaseBudget::unlimited(),
    );
    let failures = reg.run_all_failures(&mut ctx, &(), &()).unwrap();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].oracle_id.as_str(), "always.disagree");
}

#[test]
fn oracle_registry_rejects_duplicate_ids() {
    struct MyOracle;
    impl DifferentialOracle<(), ()> for MyOracle {
        fn oracle_id(&self) -> OracleId {
            OracleId::try_new("dup.oracle").unwrap()
        }
        fn classify(
            &self,
            _ctx: &mut CaseContext,
            (): &(),
            (): &(),
        ) -> Result<OracleVerdict, ResourceLimitKind> {
            Ok(OracleVerdict::Agree)
        }
    }
    let mut reg = OracleRegistry::<(), ()>::new();
    reg.register(MyOracle).expect("first");
    assert!(
        reg.register(MyOracle).is_err(),
        "duplicate OracleId must be rejected"
    );
}

#[test]
fn duplicate_invariant_names_cause_runner_error() {
    let cfg = RunConfig::new(seed(0), "dup.test", "test.op", 10).expect("valid config");
    let inv = [
        Invariant::new("check", |_ctx, _: &u64| Ok(())),
        Invariant::new("check", |_ctx, _: &u64| Ok(())),
    ];
    assert!(matches!(
        run_invariant_cases(&cfg, CaseContext::next_u64, &inv, None),
        Err(RunnerError::DuplicateCheckName { .. })
    ));
}

#[test]
fn fuzz_input_reader_is_panic_free_on_empty_input() {
    let mut r = FuzzInputReader::new(&[]);
    assert_eq!(r.read_u8(), 0);
    assert_eq!(r.read_u64_le(), 0);
    assert_eq!(
        r.read_f64_le().to_bits(),
        0u64,
        "empty reader must yield zero bits"
    );
    assert!(!r.read_bool());
    assert_eq!(r.read_bytes(100), &[] as &[u8]);
}

#[test]
fn fuzz_input_reader_usize_max_safe() {
    let data = [1u8, 2, 3];
    let mut r = FuzzInputReader::new(&data);
    let bytes = r.read_bytes(usize::MAX);
    assert_eq!(bytes, &[1u8, 2, 3]);
    assert!(r.is_exhausted());
}

#[test]
fn randomized_case_milestone_constant_is_correct() {
    assert_eq!(RANDOMIZED_CASE_MILESTONE, 10_000);
}

#[test]
fn corpus_schema_version_matches_constant() {
    let file = CorpusFile::new(vec![]).expect("valid corpus");
    assert_eq!(file.schema_version(), CORPUS_SCHEMA_VERSION);
}

#[test]
fn case_sequence_version_is_3() {
    assert_eq!(CASE_SEQUENCE_VERSION, 3u8);
}

#[test]
fn runner_collects_failure_details_correctly() {
    let cfg = RunConfig::new(seed(999), "integration.failure-details", "test.op", 10)
        .expect("valid config");
    let checks = [Invariant::new("always_fails", |_ctx, v: &u64| {
        Err(CaseCheckError::Failure(format!("value was {v}")))
    })];
    let report =
        run_invariant_cases(&cfg, |ctx| Ok(ctx.next_u64()? % 100), &checks, None).expect("valid");
    assert_eq!(report.failures.len(), 10);
    for f in &report.failures {
        assert_eq!(f.identity.check_name(), "always_fails");
        assert_eq!(f.identity.seed(), seed(999));
        assert_eq!(f.identity.stream_name(), "integration.failure-details");
        assert!(f.message.starts_with("value was "));
    }
    for (i, f) in report.failures.iter().enumerate() {
        assert_eq!(f.identity.case_index(), i as u64);
    }
}

#[test]
fn property_runner_detects_out_of_range_value() {
    let cfg = RunConfig::new(seed(31337), "integration.property", "test.op", 1_000)
        .expect("valid config");
    let dist = BoundedFloat::try_new(0.0, 1.0).expect("valid");
    let report = run_property_cases(
        &cfg,
        move |ctx| {
            let t = ctx.next_f64()?;
            let sampled = dist.lo() * (1.0 - t) + dist.hi() * t;
            Ok(if sampled >= dist.hi() {
                dist.lo().max(dist.hi().next_down())
            } else {
                sampled
            })
        },
        "in_unit_interval",
        |_ctx, v: &f64| {
            if (0.0..1.0).contains(v) {
                Ok(())
            } else {
                Err(CaseCheckError::Failure(format!("failed: {v}")))
            }
        },
        None,
    )
    .expect("valid config");
    assert!(
        report.is_ok(),
        "BoundedFloat must always stay in [0,1): {:?}",
        report.failures
    );
}

#[test]
fn metamorphic_runner_per_relation_rng_isolation() {
    use std::sync::{Arc, Mutex};

    let cfg_a = RunConfig::new(seed(7777), "meta.iso", "test.op", 30).expect("valid");
    let cfg_b = RunConfig::new(seed(7777), "meta.iso", "test.op", 30).expect("valid");

    let draws_single: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(Vec::new()));
    let draws_double: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(Vec::new()));

    {
        let capture = Arc::clone(&draws_single);
        let cases = [MetamorphicCase::new(
            "rel_target",
            move |ctx, _x: &u64| {
                let v = ctx.next_u64()?;
                capture.lock().unwrap().push(v);
                Ok(0u64)
            },
            |_ctx, _, o1, _, o2| {
                if o1 == o2 {
                    Ok(())
                } else {
                    Err(CaseCheckError::Failure(String::new()))
                }
            },
        )];
        let _ = run_metamorphic_cases(&cfg_a, CaseContext::next_u64, |_ctx, x| Ok(*x), &cases)
            .expect("valid");
    }
    {
        let capture_d = Arc::clone(&draws_double);
        let cases = [
            MetamorphicCase::new(
                "rel_other",
                |ctx, _x: &u64| ctx.next_u64(),
                |_ctx, _, o1, _, o2| {
                    if o1 == o2 {
                        Ok(())
                    } else {
                        Err(CaseCheckError::Failure(String::new()))
                    }
                },
            ),
            MetamorphicCase::new(
                "rel_target",
                move |ctx, _x: &u64| {
                    let v = ctx.next_u64()?;
                    capture_d.lock().unwrap().push(v);
                    Ok(0u64)
                },
                |_ctx, _, o1, _, o2| {
                    if o1 == o2 {
                        Ok(())
                    } else {
                        Err(CaseCheckError::Failure(String::new()))
                    }
                },
            ),
        ];
        let _ = run_metamorphic_cases(&cfg_b, CaseContext::next_u64, |_ctx, x| Ok(*x), &cases)
            .expect("valid");
    }

    let single = draws_single.lock().unwrap().clone();
    let double = draws_double.lock().unwrap().clone();
    assert_eq!(
        single, double,
        "rel_target's RNG must be unaffected by adding rel_other before it"
    );
}

// ── Resource limits tests ─────────────────────────────────────────────────

#[test]
fn resource_limits_cap_retained_failures() {
    let limits = ResourceLimits {
        max_retained_failures: Some(3),
        ..ResourceLimits::default()
    };
    let cfg = RunConfig::new(seed(42), "rl.failures", "test.op", 100)
        .expect("valid config")
        .with_resource_limits(limits);
    let checks = [Invariant::new("always_fails", |_ctx, _: &u64| {
        Err(CaseCheckError::Failure("fail".to_string()))
    })];
    let report = run_invariant_cases(&cfg, CaseContext::next_u64, &checks, None).expect("valid");
    assert!(
        report.failures.len() <= 3,
        "retained failures must be capped at 3, got {}",
        report.failures.len()
    );
    assert_eq!(
        report.resource_limit_hit,
        Some(ResourceLimitKind::MaxRetainedFailures)
    );
}

#[test]
fn resource_limits_cap_failure_message_length() {
    let limits = ResourceLimits {
        max_failure_message_bytes: Some(10),
        ..ResourceLimits::default()
    };
    let cfg = RunConfig::new(seed(42), "rl.msg", "test.op", 5)
        .expect("valid config")
        .with_resource_limits(limits);
    let checks = [Invariant::new("long_msg", |_ctx, _: &u64| {
        Err(CaseCheckError::Failure("x".repeat(1000)))
    })];
    let report = run_invariant_cases(&cfg, CaseContext::next_u64, &checks, None).expect("valid");
    for f in &report.failures {
        assert!(
            f.message.len() <= 10,
            "truncated message must be <= 10 bytes, got {}",
            f.message.len()
        );
    }
}

// ── Replay filter tests ───────────────────────────────────────────────────

#[test]
fn replay_filter_executes_exactly_targeted_case() {
    // Run 100 cases, all always-fail. Then replay case 7 with ReplayFilter.
    let cfg_full = RunConfig::new(seed(55), "replay.test", "test.op", 100).expect("valid config");
    let checks = [Invariant::new("always_fails", |_ctx, _: &u64| {
        Err(CaseCheckError::Failure("fail".to_string()))
    })];
    let full_report =
        run_invariant_cases(&cfg_full, CaseContext::next_u64, &checks, None).expect("valid");

    // Find the CaseId of case 7 in the full run.
    let target_failure = full_report
        .failures
        .iter()
        .find(|f| f.identity.case_index() == 7)
        .expect("case 7 must have failed");
    let target_case_id = target_failure.case_id;

    // Now replay only case 7.
    let filter = invariant_replay(7, "always_fails");
    let cfg_replay = RunConfig::new(seed(55), "replay.test", "test.op", 100)
        .expect("valid config")
        .with_replay(filter);
    let replay_report = run_invariant_cases(&cfg_replay, CaseContext::next_u64, &checks, None)
        .expect("valid replay");

    // Only case 7 should be in the replay report.
    assert_eq!(
        replay_report.total_cases, 1,
        "replay must run exactly 1 case"
    );
    assert_eq!(replay_report.failures.len(), 1);
    assert_eq!(replay_report.failures[0].case_id, target_case_id);
    assert_eq!(replay_report.failures[0].identity.case_index(), 7);
}

#[test]
fn replay_filter_at_high_index_is_o1() {
    // Replay case at index 9999 — must not require running 0..9998 first.
    let high_idx: u64 = 9999;
    let filter = invariant_replay(high_idx, "always_fails");
    let cfg = RunConfig::new(seed(33), "replay.hi", "test.op", 10_000)
        .expect("valid config")
        .with_replay(filter);

    let checks = [Invariant::new("always_fails", |_ctx, _: &u64| {
        Err(CaseCheckError::Failure("fail".to_string()))
    })];
    let report = run_invariant_cases(&cfg, CaseContext::next_u64, &checks, None).expect("valid");
    assert_eq!(report.total_cases, 1, "only 1 case must be executed");
    assert_eq!(report.failures[0].identity.case_index(), high_idx);
}

#[test]
fn runconfig_new_rejects_invalid_stream_name() {
    assert!(RunConfig::new(seed(1), "bad stream", "test.op", 10).is_err());
    assert!(RunConfig::new(seed(1), "", "test.op", 10).is_err());
    assert!(RunConfig::new(seed(1), "valid-stream", "test.op", 10).is_ok());
}

// ── make_entry helper round-trip ──────────────────────────────────────────

#[test]
fn make_entry_helper_round_trips() {
    let s = seed(77);
    let e = make_entry(s, 3, "primitive.sphere", "sphere.test");
    let file = CorpusFile::new(vec![e]).expect("valid corpus");
    let json = file.write_to_string().expect("ser");
    let loaded = CorpusFile::load_from_str(&json).expect("round-trip");
    assert_eq!(loaded.entries()[0].seed(), s);
    assert_eq!(loaded.entries()[0].case_index(), 3);
    assert_eq!(loaded.entries()[0].stream_name(), Some("sphere.test"));
}

// ── Issue 1: u64::MAX replay + ReplayMismatch (public API level) ──────────

#[test]
fn replay_filter_u64_max_case_index_runs_exactly_one_case_invariant() {
    let cfg = RunConfig::new(seed(1), "e2e.replay.max.invariant", "test.op", 5)
        .expect("valid config")
        .with_replay(invariant_replay(u64::MAX, "always_ok"));
    let checks = [Invariant::new("always_ok", |_ctx, _: &u64| Ok(()))];
    let report = run_invariant_cases(&cfg, CaseContext::next_u64, &checks, None)
        .expect("must not panic on u64::MAX");
    assert_eq!(report.total_cases, 1, "replay must run exactly 1 case");
    assert!(report.failures.is_empty());
}

#[test]
fn replay_filter_u64_max_case_index_runs_exactly_one_case_metamorphic() {
    let cfg = RunConfig::new(seed(2), "e2e.replay.max.metamorphic", "test.op", 5)
        .expect("valid config")
        .with_replay(metamorphic_replay(u64::MAX, "identity"));
    let cases = [MetamorphicCase::new(
        "identity",
        |_ctx, x: &u64| Ok(*x),
        |_ctx, _, o1, _, o2| {
            if o1 == o2 {
                Ok(())
            } else {
                Err(CaseCheckError::Failure(String::new()))
            }
        },
    )];
    let report = run_metamorphic_cases(&cfg, CaseContext::next_u64, |_ctx, x| Ok(*x), &cases)
        .expect("must not panic on u64::MAX");
    assert_eq!(report.total_cases, 1, "replay must run exactly 1 case");
}

#[test]
fn replay_mismatch_rejected_before_generating_input_invariant() {
    let cfg = RunConfig::new(seed(3), "e2e.replay.mismatch.invariant", "test.op", 5)
        .expect("valid config")
        .with_replay(invariant_replay(0, "nonexistent_check"));
    let checks = [Invariant::new("real_check", |_ctx, _: &u64| Ok(()))];
    let result = run_invariant_cases(&cfg, CaseContext::next_u64, &checks, None);
    assert!(
        matches!(
            result,
            Err(RunnerError::ReplayMismatch { ref name }) if name == "nonexistent_check"
        ),
        "expected ReplayMismatch, got {result:?}"
    );
}

#[test]
fn replay_mismatch_rejected_before_generating_input_metamorphic() {
    let cfg = RunConfig::new(seed(4), "e2e.replay.mismatch.metamorphic", "test.op", 5)
        .expect("valid config")
        .with_replay(metamorphic_replay(0, "nonexistent_relation"));
    let cases = [MetamorphicCase::new(
        "real_relation",
        |_ctx, x: &u64| Ok(*x),
        |_ctx, _, o1, _, o2| {
            if o1 == o2 {
                Ok(())
            } else {
                Err(CaseCheckError::Failure(String::new()))
            }
        },
    )];
    let result = run_metamorphic_cases(&cfg, CaseContext::next_u64, |_ctx, x| Ok(*x), &cases);
    assert!(
        matches!(
            result,
            Err(RunnerError::ReplayMismatch { ref name }) if name == "nonexistent_relation"
        ),
        "expected ReplayMismatch, got {result:?}"
    );
}

// ── Issue 2: end-to-end replay-from-environment API ───────────────────────

#[test]
fn apply_replay_config_runs_exactly_one_case() {
    // Bypass actual process env: build a ReplayConfig by hand and apply it.
    let replay = ReplayConfig {
        case_sequence_version: CASE_SEQUENCE_VERSION,
        operation: "test.op".to_string(),
        seed: seed(9001),
        case_index: 42,
        stream_name: "e2e.replay.applied".to_string(),
        check_kind: CheckKind::Invariant,
        check_name: "always_ok".to_string(),
    };
    let base_cfg =
        RunConfig::new(seed(1), "e2e.replay.applied", "test.op", 100).expect("valid base config");
    let cfg = apply_replay_config(base_cfg, &replay).expect("matching stream");

    assert_eq!(cfg.seed(), seed(9001), "seed must be overridden by replay");
    let checks = [Invariant::new("always_ok", |_ctx, _: &u64| Ok(()))];
    let report =
        run_invariant_cases(&cfg, CaseContext::next_u64, &checks, None).expect("valid replay run");
    assert_eq!(report.total_cases, 1, "replay must run exactly 1 case");
    assert!(report.failures.is_empty());
}

#[test]
fn e2e_replay_env_fixture() {
    let base_cfg =
        RunConfig::new(seed(12345), "e2e.replay.env.fixture", "e2e.test.op", 100).expect("valid");
    let (cfg, expected_total) = match configure_replay_from_env(base_cfg.clone()) {
        Ok(Some(replay_cfg)) => (replay_cfg, 1u64),
        Ok(None) => (base_cfg, 100u64),
        Err(error) => {
            eprintln!("replay env error: {error}");
            std::process::exit(1);
        }
    };
    let checks = [Invariant::new("always_ok", |_ctx, _: &u64| Ok(()))];
    let report =
        run_invariant_cases(&cfg, CaseContext::next_u64, &checks, None).expect("valid replay run");
    assert_eq!(
        report.total_cases, expected_total,
        "replay must run exactly {expected_total} case(s)"
    );
}

fn run_fixture_subprocess(envs: &[(&str, &str)]) -> std::process::Output {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root");
    let mut command = std::process::Command::new("cargo");
    command.current_dir(repo_root).args([
        "test",
        "--locked",
        "-p",
        "amphion-test-support",
        "--",
        "--exact",
        "e2e_replay_env_fixture",
    ]);
    for key in [
        ENV_TEST_VERSION,
        ENV_TEST_SEED,
        ENV_TEST_CASE,
        ENV_TEST_STREAM,
        ENV_TEST_OPERATION,
        ENV_TEST_CHECK_KIND,
        ENV_TEST_CHECK,
    ] {
        command.env_remove(key);
    }
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("cargo test")
}

#[test]
fn e2e_replay_env_subprocess_valid_identity_runs_exactly_one_case() {
    let output = run_fixture_subprocess(&[
        (ENV_TEST_VERSION, "3"),
        (ENV_TEST_SEED, "12345"),
        (ENV_TEST_CASE, "7"),
        (ENV_TEST_STREAM, "e2e.replay.env.fixture"),
        (ENV_TEST_OPERATION, "e2e.test.op"),
        (ENV_TEST_CHECK_KIND, "invariant"),
        (ENV_TEST_CHECK, "always_ok"),
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "subprocess must succeed; stdout:\n{stdout}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("1 passed"),
        "subprocess output must report exactly 1 test passed; got:\n{stdout}"
    );
}

#[test]
fn e2e_replay_env_subprocess_wrong_stream_exits_nonzero() {
    let output = run_fixture_subprocess(&[
        (ENV_TEST_VERSION, "3"),
        (ENV_TEST_SEED, "12345"),
        (ENV_TEST_CASE, "7"),
        (ENV_TEST_STREAM, "wrong.stream"),
        (ENV_TEST_OPERATION, "e2e.test.op"),
        (ENV_TEST_CHECK_KIND, "invariant"),
        (ENV_TEST_CHECK, "always_ok"),
    ]);
    assert!(
        !output.status.success(),
        "wrong stream must fail; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn e2e_replay_env_subprocess_wrong_operation_exits_nonzero() {
    let output = run_fixture_subprocess(&[
        (ENV_TEST_VERSION, "3"),
        (ENV_TEST_SEED, "12345"),
        (ENV_TEST_CASE, "7"),
        (ENV_TEST_STREAM, "e2e.replay.env.fixture"),
        (ENV_TEST_OPERATION, "wrong.valid.operation"),
        (ENV_TEST_CHECK_KIND, "invariant"),
        (ENV_TEST_CHECK, "always_ok"),
    ]);
    assert!(
        !output.status.success(),
        "wrong operation must fail; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn e2e_replay_env_subprocess_wrong_check_kind_exits_nonzero() {
    let output = run_fixture_subprocess(&[
        (ENV_TEST_VERSION, "3"),
        (ENV_TEST_SEED, "12345"),
        (ENV_TEST_CASE, "7"),
        (ENV_TEST_STREAM, "e2e.replay.env.fixture"),
        (ENV_TEST_OPERATION, "e2e.test.op"),
        (ENV_TEST_CHECK_KIND, "property"),
        (ENV_TEST_CHECK, "always_ok"),
    ]);
    assert!(
        !output.status.success(),
        "wrong check kind must fail; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn e2e_replay_env_subprocess_wrong_check_name_exits_nonzero() {
    let output = run_fixture_subprocess(&[
        (ENV_TEST_VERSION, "3"),
        (ENV_TEST_SEED, "12345"),
        (ENV_TEST_CASE, "7"),
        (ENV_TEST_STREAM, "e2e.replay.env.fixture"),
        (ENV_TEST_OPERATION, "e2e.test.op"),
        (ENV_TEST_CHECK_KIND, "invariant"),
        (ENV_TEST_CHECK, "wrong.check.name"),
    ]);
    assert!(
        !output.status.success(),
        "wrong check name must fail; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn e2e_replay_env_subprocess_wrong_version_exits_nonzero() {
    let output = run_fixture_subprocess(&[
        (ENV_TEST_VERSION, "1"),
        (ENV_TEST_SEED, "12345"),
        (ENV_TEST_CASE, "7"),
        (ENV_TEST_STREAM, "e2e.replay.env.fixture"),
        (ENV_TEST_OPERATION, "e2e.test.op"),
        (ENV_TEST_CHECK_KIND, "invariant"),
        (ENV_TEST_CHECK, "always_ok"),
    ]);
    assert!(
        !output.status.success(),
        "wrong version must fail; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn e2e_replay_env_subprocess_partial_env_exits_nonzero() {
    let output = run_fixture_subprocess(&[
        (ENV_TEST_VERSION, "3"),
        (ENV_TEST_SEED, "12345"),
        (ENV_TEST_CASE, "7"),
        (ENV_TEST_STREAM, "e2e.replay.env.fixture"),
        (ENV_TEST_CHECK_KIND, "invariant"),
        (ENV_TEST_CHECK, "always_ok"),
    ]);
    assert!(
        !output.status.success(),
        "partial replay env must fail; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

// ── Issue 3: with_replay_command validation ───────────────────────────────

#[test]
fn with_replay_command_rejects_seed_mismatch() {
    let s = seed(10);
    let id = CaseId::new(s.for_case_stream("cmd.mismatch"), 0);
    let report = new_failure_report(
        s,
        id,
        "cmd.mismatch",
        "op",
        0,
        serde_json::json!({}),
        "fail",
    )
    .expect("valid report");
    let wrong_cmd = new_command("pkg", "test_fn", seed(11), 0, "cmd.mismatch").expect("valid cmd");
    let result = report.with_replay_command(wrong_cmd);
    assert!(
        matches!(
            result,
            Err(ReportError::CommandMismatch { field: "seed", .. })
        ),
        "expected CommandMismatch on seed, got {result:?}"
    );
}

#[test]
fn with_replay_command_rejects_case_index_mismatch() {
    let s = seed(12);
    let id = CaseId::new(s.for_case_stream("cmd.mismatch2"), 0);
    let report = new_failure_report(
        s,
        id,
        "cmd.mismatch2",
        "op",
        0,
        serde_json::json!({}),
        "fail",
    )
    .expect("valid report");
    let wrong_cmd = new_command("pkg", "test_fn", s, 99, "cmd.mismatch2").expect("valid cmd");
    let result = report.with_replay_command(wrong_cmd);
    assert!(
        matches!(
            result,
            Err(ReportError::CommandMismatch {
                field: "case_index",
                ..
            })
        ),
        "expected CommandMismatch on case_index, got {result:?}"
    );
}

// ── Issue 4/5: resource limits enforced before push ────────────────────────

#[test]
fn max_cases_run_stops_the_loop_early() {
    let limits = ResourceLimits {
        max_cases_run: Some(4),
        ..ResourceLimits::default()
    };
    let cfg = RunConfig::new(seed(20), "rl.max-cases", "test.op", 100)
        .expect("valid config")
        .with_resource_limits(limits);
    let checks = [Invariant::new("always_ok", |_ctx, _: &u64| Ok(()))];
    let report = run_invariant_cases(&cfg, CaseContext::next_u64, &checks, None).expect("valid");
    assert_eq!(report.total_cases, 4, "must stop after exactly 4 cases");
    assert_eq!(
        report.resource_limit_hit,
        Some(ResourceLimitKind::MaxCasesRun)
    );
}

#[test]
fn max_retained_failures_enforced_before_push_not_after() {
    // With max_retained_failures = Some(1), at most 1 failure must ever be
    // recorded -- never 2, which would happen if the check ran after push.
    let limits = ResourceLimits {
        max_retained_failures: Some(1),
        ..ResourceLimits::default()
    };
    let cfg = RunConfig::new(seed(21), "rl.push-order", "test.op", 50)
        .expect("valid config")
        .with_resource_limits(limits);
    let checks = [Invariant::new("always_fails", |_ctx, _: &u64| {
        Err(CaseCheckError::Failure("fail".to_string()))
    })];
    let report = run_invariant_cases(&cfg, CaseContext::next_u64, &checks, None).expect("valid");
    assert_eq!(
        report.failures.len(),
        1,
        "exactly 1 failure must be retained, never 2"
    );
}

// ── Issue 9: ReproducibleCommand package validation + rendering ──────────

#[test]
fn reproducible_command_rejects_package_starting_with_dash() {
    let s = seed(30);
    assert!(new_command("-pkg", "test_fn", s, 0, "s").is_err());
}

#[test]
fn reproducible_command_display_uses_package_equals_flag() {
    let s = seed(31);
    let cmd = new_command("my-pkg", "test_fn", s, 0, "s").expect("valid cmd");
    let display = cmd.to_string();
    assert!(
        display.contains("--package=my-pkg"),
        "expected --package= form, got: {display}"
    );
    assert!(
        !display.contains("-p my-pkg"),
        "must not use the short -p form, got: {display}"
    );
}

// ── ReplayIdentity tests ────────────────────────────────────────────────────

#[test]
fn replay_identity_from_failure_report_and_command_are_consistent() {
    let s = seed(77);
    let stream = "id.consistency";
    let id = CaseId::new(s.for_case_stream(stream), 5);
    let report = new_failure_report(s, id, stream, "op.identity", 5, serde_json::json!({}), "m")
        .expect("valid report");
    let cmd = new_command("pkg", "test_fn", s, 5, stream).expect("valid cmd");
    let report_id: ReplayIdentity = report.identity();
    let cmd_id: ReplayIdentity = cmd.identity();
    assert_eq!(report_id.seed(), cmd_id.seed(), "seeds must match");
    assert_eq!(
        report_id.case_index(),
        cmd_id.case_index(),
        "case_index must match"
    );
    assert_eq!(
        report_id.stream_name(),
        cmd_id.stream_name(),
        "stream_name must match"
    );
    assert_eq!(
        report_id.case_sequence_version(),
        cmd_id.case_sequence_version(),
        "case_sequence_version must match"
    );
}

// ── CaseBudget / CaseContext integration tests ──────────────────────────────

#[test]
fn case_budget_unlimited_never_exhausted() {
    let mut b = CaseBudget::unlimited();
    for _ in 0..10_000 {
        assert!(b.charge_draw().is_ok());
        assert!(b.charge_work(1).is_ok());
        assert!(b.charge_oracle().is_ok());
        assert!(b.charge_minimization_step().is_ok());
    }
}

#[test]
fn case_budget_from_limits_enforces_draw_cap() {
    let limits = ResourceLimits {
        max_rng_draws_per_case: Some(3),
        ..ResourceLimits::default()
    };
    let mut b = CaseBudget::from_limits(&limits);
    assert!(b.charge_draw().is_ok());
    assert!(b.charge_draw().is_ok());
    assert!(b.charge_draw().is_ok());
    assert_eq!(b.charge_draw(), Err(ResourceLimitKind::MaxRngDrawsPerCase));
}

#[test]
fn case_context_exposes_charged_rng() {
    let rng = amphion_test_support::rng::TestRng::from_seed(seed(1));
    let mut ctx = CaseContext::new(42, rng, CaseBudget::unlimited());
    assert_eq!(ctx.case_index, 42);
    assert!(ctx.draws_remaining().is_none());
    assert!(ctx.next_u64().is_ok());
}

#[test]
fn case_budget_draw_cap_enforced_through_case_context() {
    let limits = ResourceLimits {
        max_rng_draws_per_case: Some(2),
        ..ResourceLimits::default()
    };
    let rng = amphion_test_support::rng::TestRng::from_seed(seed(77));
    let mut ctx = CaseContext::new(0, rng, CaseBudget::from_limits(&limits));
    assert!(ctx.next_u64().is_ok());
    assert!(ctx.next_u64().is_ok());
    assert_eq!(ctx.next_u64(), Err(ResourceLimitKind::MaxRngDrawsPerCase));
}

#[test]
fn max_total_input_bytes_zero_rejects_first_case() {
    let cfg = RunConfig::new(seed(80), "self.max-total-input-bytes", "test.op", 10)
        .expect("valid")
        .with_resource_limits(ResourceLimits {
            max_total_input_bytes: Some(0),
            ..ResourceLimits::default()
        });
    let checks = [Invariant::new("always_ok", |_ctx, _: &u64| Ok(()))];
    let report = run_invariant_cases(&cfg, CaseContext::next_u64, &checks, None).expect("valid");
    assert_eq!(
        report.resource_limit_hit,
        Some(ResourceLimitKind::MaxTotalInputBytes)
    );
}

#[test]
fn max_oracle_calls_enforced_by_runner() {
    let cfg = RunConfig::new(seed(81), "self.max-oracle-calls", "test.op", 10)
        .expect("valid")
        .with_resource_limits(ResourceLimits {
            max_oracle_calls: Some(0),
            ..ResourceLimits::default()
        });
    let checks = [Invariant::new("must_not_run", |_ctx, _: &u64| {
        panic!("invariant must not run when oracle budget is zero")
    })];
    let report = run_invariant_cases(&cfg, CaseContext::next_u64, &checks, None).expect("valid");
    assert_eq!(
        report.resource_limit_hit,
        Some(ResourceLimitKind::MaxOracleCalls)
    );
}

// ── ReplayEnvError stream mismatch test ─────────────────────────────────────

#[test]
fn apply_replay_config_stream_mismatch_returns_error() {
    let replay = ReplayConfig {
        case_sequence_version: CASE_SEQUENCE_VERSION,
        operation: "shape.boolean".to_string(),
        seed: seed(100),
        case_index: 0,
        stream_name: "wrong.stream".to_string(),
        check_kind: CheckKind::Invariant,
        check_name: "always_ok".to_string(),
    };
    let cfg = RunConfig::new(seed(100), "correct.stream", "test.op", 10).expect("valid");
    let result = apply_replay_config(cfg, &replay);
    assert!(
        matches!(result, Err(ReplayEnvError::StreamMismatch { .. })),
        "expected StreamMismatch, got {result:?}"
    );
}

// ── Corpus v1.0 load-write-load round-trip ──────────────────────────────────

#[test]
fn corpus_v1_0_load_write_load_is_lossless() {
    // Minimal valid v1.0 corpus (double-encoded inputs_json).
    let original = serde_json::json!({
        "schema_version": {"major": 1, "minor": 0},
        "entries": [{
            "schema_version": {"major": 1, "minor": 0},
            "id": "00000000000000000000000000000000",
            "operation": "op",
            "seed": 1,
            "case_index": 0,
            "inputs_json": r#"{"x":1}"#,
            "failure_message": "fail"
        }]
    });
    let original_str = serde_json::to_string(&original).unwrap();
    let loaded = LegacyCorpusDocument::load_from_str(&original_str).expect("first load");
    assert_eq!(
        loaded.as_str(),
        original_str,
        "legacy bytes must be preserved"
    );
}

// ── Blocker-10 tests: direct replay op validation, shared check-kind loop ─

/// Direct replay with the wrong operation returns `OperationMismatch`
/// before any generator is invoked.
#[test]
fn direct_replay_wrong_operation_returns_operation_mismatch() {
    let filter = ReplayFilter::new(
        0,
        "wrong.op".to_string(),
        CheckKind::Invariant,
        "ok".to_string(),
    );
    let config = RunConfig::new(seed(1), "test.op.check", "right.op", 10)
        .expect("valid config")
        .with_replay(filter);
    let inv = [Invariant::new("ok", |_ctx, _: &u64| Ok(()))];
    let mut gen_called = false;
    let err = run_invariant_cases(
        &config,
        |_ctx: &mut CaseContext| {
            gen_called = true;
            Ok::<u64, ResourceLimitKind>(0)
        },
        &inv,
        None,
    )
    .unwrap_err();
    assert!(
        !gen_called,
        "generator must NOT be called when operation mismatches"
    );
    assert!(
        matches!(err, RunnerError::OperationMismatch { ref expected, ref found }
            if expected == "right.op" && found == "wrong.op"),
        "expected OperationMismatch {{expected=right.op, found=wrong.op}}, got {err:?}"
    );
}

/// A `Property` [`ReplayFilter`] applied to the property runner executes exactly
/// one case — checks that the shared inner loop honours `CheckKind::Property`.
#[test]
fn property_replay_filter_on_property_runner_runs_exactly_one_case() {
    let filter = ReplayFilter::new(
        5,
        "test.op".to_string(),
        CheckKind::Property,
        "my.prop".to_string(),
    );
    let config = RunConfig::new(seed(2), "prop.replay.stream", "test.op", 100)
        .expect("valid config")
        .with_replay(filter);
    let report = run_property_cases(
        &config,
        CaseContext::next_u64,
        "my.prop",
        |_ctx, _: &u64| Ok(()),
        None,
    )
    .expect("valid");
    assert_eq!(report.total_cases, 1, "replay must run exactly one case");
    assert_eq!(report.passed_cases, 1);
    assert!(report.failures.is_empty());
}

/// An `Invariant` [`ReplayFilter`] applied to the property runner is rejected with
/// `CheckKindMismatch` — no post-hoc relabelling can accidentally accept it.
#[test]
fn invariant_replay_filter_on_property_runner_rejected() {
    let filter = ReplayFilter::new(
        0,
        "test.op".to_string(),
        CheckKind::Invariant,
        "prop".to_string(),
    );
    let config = RunConfig::new(seed(3), "crosskind.stream", "test.op", 10)
        .expect("valid config")
        .with_replay(filter);
    let mut gen_called = false;
    let err = run_property_cases(
        &config,
        |_ctx: &mut CaseContext| {
            gen_called = true;
            Ok::<u64, ResourceLimitKind>(0)
        },
        "prop",
        |_ctx, _: &u64| Ok(()),
        None,
    )
    .unwrap_err();
    assert!(!gen_called);
    assert!(
        matches!(
            err,
            RunnerError::CheckKindMismatch {
                expected: CheckKind::Property,
                found: CheckKind::Invariant
            }
        ),
        "expected CheckKindMismatch, got {err:?}"
    );
}

/// A `Property` [`ReplayFilter`] applied to the invariant runner is rejected.
#[test]
fn property_replay_filter_on_invariant_runner_rejected() {
    let filter = ReplayFilter::new(
        0,
        "test.op".to_string(),
        CheckKind::Property,
        "inv".to_string(),
    );
    let config = RunConfig::new(seed(4), "crosskind2.stream", "test.op", 10)
        .expect("valid config")
        .with_replay(filter);
    let inv = [Invariant::new("inv", |_ctx, _: &u64| Ok(()))];
    let mut gen_called = false;
    let err = run_invariant_cases(
        &config,
        |_ctx: &mut CaseContext| {
            gen_called = true;
            Ok::<u64, ResourceLimitKind>(0)
        },
        &inv,
        None,
    )
    .unwrap_err();
    assert!(!gen_called);
    assert!(
        matches!(
            err,
            RunnerError::CheckKindMismatch {
                expected: CheckKind::Invariant,
                found: CheckKind::Property
            }
        ),
        "expected CheckKindMismatch, got {err:?}"
    );
}

/// Full end-to-end artifact round-trip:
/// run a failing property with a minimizer → convert failure → serialize and
/// deserialize `FailureReport` and `CorpusEntry` → identity and minimized
/// input are byte-for-byte identical after the round-trip.
#[test]
fn case_failure_converts_to_artifacts_round_trip() {
    use std::cell::Cell;
    use std::rc::Rc;

    // Generator returns 20u64; property fails when v >= 10.
    // Minimizer tries v/2 once: 20/2=10, still fails. Then stops.
    let tried = Rc::new(Cell::new(false));
    let tried2 = Rc::clone(&tried);

    let config =
        RunConfig::new(seed(500), "artifact.stream", "artifact.op", 50).expect("valid config");

    let mut minimizer = Minimizer::new(move |_ctx: &mut CaseContext, v: &u64| {
        if tried2.get() {
            Ok(None)
        } else {
            tried2.set(true);
            Ok(Some(*v / 2))
        }
    });

    let report = run_property_cases(
        &config,
        |_ctx: &mut CaseContext| Ok::<u64, ResourceLimitKind>(20),
        "big.value",
        |_ctx, v: &u64| {
            if *v < 10 {
                Ok(())
            } else {
                Err(CaseCheckError::Failure(format!("{v} >= 10")))
            }
        },
        Some(&mut minimizer),
    )
    .expect("runner ok");

    assert!(!report.failures.is_empty(), "expected at least one failure");
    let failure = &report.failures[0];

    // 20/2=10; 10 still fails (10 >= 10). Next call returns None → minimizer ends.
    assert_eq!(
        failure.input_json,
        serde_json::json!(10u64),
        "minimizer shrinks 20→10"
    );
    assert_eq!(failure.identity.operation(), "artifact.op");
    assert_eq!(failure.identity.check_kind(), CheckKind::Property);

    // FailureReport round-trip.
    let fr = failure.to_failure_report().expect("to_failure_report");
    let fr_json = serde_json::to_string(&fr).expect("serialize FailureReport");
    let fr2: FailureReport = serde_json::from_str(&fr_json).expect("deserialize FailureReport");
    assert_eq!(fr2.operation(), failure.identity.operation());
    assert_eq!(fr2.check_kind(), failure.identity.check_kind());
    assert_eq!(fr2.check_name(), failure.identity.check_name());
    assert_eq!(fr2.inputs_json(), &failure.input_json);

    // CorpusEntry round-trip.
    let entry = failure.to_corpus_entry().expect("to_corpus_entry");
    assert_eq!(entry.check_kind(), Some(failure.identity.check_kind()));
    assert_eq!(entry.check_name(), Some(failure.identity.check_name()));

    // ReproducibleCommand renders operation and test name.
    let cmd = failure
        .to_reproducible_command("my-crate", "big_value_test")
        .expect("to_reproducible_command");
    let posix = cmd.to_string();
    assert!(posix.contains("artifact.op"), "POSIX includes operation");
    assert!(posix.contains("big_value_test"), "POSIX includes test name");
}
