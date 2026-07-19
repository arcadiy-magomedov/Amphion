//! Integration test battery for `amphion-topology` T1.1–T1.6.
//!
//! Covers:
//! - valid minimal bodies (tetrahedron, single-face open shell)
//! - invalid/stale references
//! - ownership boundaries
//! - closed edges (same start/end vertex)
//! - orientation reversal
//! - loop traversal
//! - manifold and non-manifold incidences
//! - Euler characteristic counts
//! - deterministic traversal/serialization
//! - provenance preservation

use amphion_foundation::{LengthTolerance, OperationId, Point3, SemanticId, Severity};
use amphion_geometry::{Curve2Id, Curve3Id, ParameterInterval, SurfaceId};
use amphion_topology::{
    BodyId, BodyParams, CoedgeId, CoedgeParams, EdgeId, EdgeParams, FaceId, FaceParams, LoopId,
    LoopKind, LoopParams, Orientation, ProvenanceRole, RegionId, RegionParams, ShellId, ShellKind,
    ShellParams, TopologyBuilder, TopologyError, TopologyKind, TopologyLineageId,
    TopologySnapshotId, TopologyStore, VertexId, VertexParams, body_total_euler_ve_f,
    coedge_end_vertex_id, coedge_start_vertex_id, edge_adjacent_face_ids, face_adjacent_face_ids,
    face_loop_ids, is_closed_manifold_shell, loop_coedge_ids, loop_next_coedge_id,
    shell_euler_metrics,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn pos(x: f64, y: f64, z: f64) -> Point3 {
    Point3::try_new(x, y, z).expect("finite")
}

/// A deterministic non-zero test lineage (AA…AA).
fn test_lineage() -> TopologyLineageId {
    TopologyLineageId::new(SemanticId::from_bytes([0xAA; 16]))
}

/// A deterministic test snapshot (CC…CC).
fn test_snapshot() -> TopologySnapshotId {
    TopologySnapshotId::new(SemanticId::from_bytes([0xCC; 16]))
}

/// A second deterministic test snapshot (DD…DD), distinct from `test_snapshot`.
fn test_snapshot_b() -> TopologySnapshotId {
    TopologySnapshotId::new(SemanticId::from_bytes([0xDD; 16]))
}

/// Canonical builder for tests that don't care about the specific lineage.
fn test_builder() -> TopologyBuilder {
    TopologyBuilder::with_lineage_and_snapshot(test_lineage(), test_snapshot())
}

fn tol() -> LengthTolerance {
    LengthTolerance::try_new(1.0e-9).expect("positive")
}

fn sem(n: u8) -> SemanticId {
    SemanticId::from_bytes([n; 16])
}

fn prov(n: u8) -> amphion_topology::Provenance {
    let role = ProvenanceRole::try_new("test.entity").expect("valid");
    amphion_topology::Provenance::new(sem(n), None, vec![], role)
}

fn prov_with_op(n: u8, op: u8) -> amphion_topology::Provenance {
    let role = ProvenanceRole::try_new("test.entity").expect("valid");
    let op_id = OperationId::from_bytes([op; 16]);
    amphion_topology::Provenance::new(sem(n), Some(op_id), vec![], role)
}

fn fake_curve3() -> Curve3Id {
    Curve3Id::new(0, 0)
}

fn fake_curve2() -> Curve2Id {
    Curve2Id::new(0, 0)
}

fn fake_surface() -> SurfaceId {
    SurfaceId::new(0, 0)
}

fn interval(s: f64, e: f64) -> ParameterInterval {
    ParameterInterval::try_new(s, e).expect("valid interval")
}

/// Builds a triangle face and returns its vertices and edges.
fn build_triangle_face_with_edges(
    b: &mut TopologyBuilder,
) -> (FaceId, VertexId, VertexId, VertexId, EdgeId, EdgeId, EdgeId) {
    let v0 = b
        .add_vertex(VertexParams {
            position: pos(0.0, 0.0, 0.0),
            tolerance: tol(),
            provenance: prov(1),
        })
        .expect("v0");
    let v1 = b
        .add_vertex(VertexParams {
            position: pos(1.0, 0.0, 0.0),
            tolerance: tol(),
            provenance: prov(2),
        })
        .expect("v1");
    let v2 = b
        .add_vertex(VertexParams {
            position: pos(0.0, 1.0, 0.0),
            tolerance: tol(),
            provenance: prov(3),
        })
        .expect("v2");

    let e0 = b
        .add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: v0,
            end_vertex: v1,
            tolerance: tol(),
            provenance: prov(4),
        })
        .expect("e0");
    let e1 = b
        .add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: v1,
            end_vertex: v2,
            tolerance: tol(),
            provenance: prov(5),
        })
        .expect("e1");
    let e2 = b
        .add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: v2,
            end_vertex: v0,
            tolerance: tol(),
            provenance: prov(6),
        })
        .expect("e2");

    let f0 = b
        .add_face(FaceParams {
            surface: fake_surface(),
            orientation: Orientation::Forward,
            provenance: prov(7),
        })
        .expect("f0");
    let l0 = b
        .add_loop(LoopParams {
            face: f0,
            kind: LoopKind::Outer,
            provenance: prov(8),
        })
        .expect("l0");
    b.add_coedge(CoedgeParams {
        edge: e0,
        loop_id: l0,
        orientation: Orientation::Forward,
        pcurve: fake_curve2(),
        provenance: prov(9),
    })
    .expect("c0");
    b.add_coedge(CoedgeParams {
        edge: e1,
        loop_id: l0,
        orientation: Orientation::Forward,
        pcurve: fake_curve2(),
        provenance: prov(10),
    })
    .expect("c1");
    b.add_coedge(CoedgeParams {
        edge: e2,
        loop_id: l0,
        orientation: Orientation::Forward,
        pcurve: fake_curve2(),
        provenance: prov(11),
    })
    .expect("c2");

    (f0, v0, v1, v2, e0, e1, e2)
}

fn build_triangle_face(b: &mut TopologyBuilder) -> (FaceId, VertexId, VertexId, VertexId) {
    let (face_id, v0, v1, v2, _, _, _) = build_triangle_face_with_edges(b);
    (face_id, v0, v1, v2)
}

/// Adds a triangular face to the builder and returns the face ID.
///
/// `edges` is `[(edge_id, orientation); 3]`. `prov_base` is a u8 used for
/// the face and loop provenance values; coedges share `prov_base + 2`.
fn add_tri_face(
    b: &mut TopologyBuilder,
    edges: [(EdgeId, Orientation); 3],
    prov_base: u8,
) -> FaceId {
    let f = b
        .add_face(FaceParams {
            surface: fake_surface(),
            orientation: Orientation::Forward,
            provenance: prov(prov_base),
        })
        .expect("face");
    let l = b
        .add_loop(LoopParams {
            face: f,
            kind: LoopKind::Outer,
            provenance: prov(prov_base + 1),
        })
        .expect("loop");
    for (edge, orient) in edges {
        b.add_coedge(CoedgeParams {
            edge,
            loop_id: l,
            orientation: orient,
            pcurve: fake_curve2(),
            provenance: prov(prov_base + 2),
        })
        .expect("coedge");
    }
    f
}

fn add_face_with_loop(
    b: &mut TopologyBuilder,
    edges: &[(EdgeId, Orientation)],
    kind: LoopKind,
    prov_base: u8,
) -> FaceId {
    let face_id = mk_face(b, prov_base);
    let loop_id = mk_loop(b, face_id, kind, prov_base + 1);
    for &(edge_id, orientation) in edges {
        mk_coedge(b, edge_id, loop_id, orientation, prov_base + 2);
    }
    face_id
}

fn flip_o(orientation: Orientation) -> Orientation {
    match orientation {
        Orientation::Forward => Orientation::Reversed,
        Orientation::Reversed => Orientation::Forward,
    }
}

fn close_face_as_solid(
    b: &mut TopologyBuilder,
    orig_face: FaceId,
    edges: &[(EdgeId, Orientation)],
    prov_base: u8,
) -> BodyId {
    let mirror_face = mk_face(b, prov_base);
    let mirror_loop = mk_loop(b, mirror_face, LoopKind::Outer, prov_base + 1);
    for &(edge_id, orientation) in edges.iter().rev() {
        mk_coedge(b, edge_id, mirror_loop, flip_o(orientation), prov_base + 2);
    }
    let s = b
        .add_shell(ShellParams {
            kind: ShellKind::Closed,
            faces: vec![orig_face, mirror_face],
            provenance: prov(prov_base + 3),
        })
        .expect("shell");
    let r = b
        .add_region(RegionParams {
            outer_shell: s,
            inner_shells: vec![],
            provenance: prov(prov_base + 4),
        })
        .expect("region");
    b.add_body(BodyParams {
        regions: vec![r],
        provenance: prov(prov_base + 5),
    })
    .expect("body")
}

fn add_single_face_body(
    b: &mut TopologyBuilder,
    face: FaceId,
    edges: &[(EdgeId, Orientation)],
) -> BodyId {
    close_face_as_solid(b, face, edges, 30)
}

/// Builds a complete closed triangle-pair store using `build_triangle_face`.
///
/// Returns `(store, f0, v0, v1, v2)` using the standard triangle geometry.
fn build_open_triangle_store() -> (TopologyStore, FaceId, VertexId, VertexId, VertexId) {
    let mut b = test_builder();
    let (f0, v0, v1, v2, e0, e1, e2) = build_triangle_face_with_edges(&mut b);
    close_face_as_solid(
        &mut b,
        f0,
        &[
            (e0, Orientation::Forward),
            (e1, Orientation::Forward),
            (e2, Orientation::Forward),
        ],
        30,
    );
    let store = b.build().expect("closed bipyramid store");
    (store, f0, v0, v1, v2)
}

fn build_reversed_triangle_store() -> (TopologyStore, CoedgeId, VertexId, VertexId) {
    let mut b = test_builder();
    let v0 = mk_v(&mut b, 0.0, 0.0, 1);
    let v1 = mk_v(&mut b, 1.0, 0.0, 2);
    let v2 = mk_v(&mut b, 0.0, 1.0, 3);
    let e01 = mk_e(&mut b, v0, v1, 4);
    let e12 = mk_e(&mut b, v1, v2, 5);
    let e02 = mk_e(&mut b, v0, v2, 6);
    let f0 = mk_face(&mut b, 7);
    let l0 = mk_loop(&mut b, f0, LoopKind::Outer, 8);
    mk_coedge(&mut b, e01, l0, Orientation::Forward, 9);
    mk_coedge(&mut b, e12, l0, Orientation::Forward, 10);
    let c_rev = b
        .add_coedge(CoedgeParams {
            edge: e02,
            loop_id: l0,
            orientation: Orientation::Reversed,
            pcurve: fake_curve2(),
            provenance: prov(11),
        })
        .expect("reversed coedge");
    close_face_as_solid(
        &mut b,
        f0,
        &[
            (e01, Orientation::Forward),
            (e12, Orientation::Forward),
            (e02, Orientation::Reversed),
        ],
        30,
    );
    (b.build().expect("store"), c_rev, v0, v2)
}

fn same_direction_edge_pair_error() -> TopologyError {
    let mut b = test_builder();
    let v0 = mk_v(&mut b, 0.0, 0.0, 0);
    let v1 = mk_v(&mut b, 1.0, 0.0, 1);
    let v2 = mk_v(&mut b, 0.5, 1.0, 2);
    let v3 = mk_v(&mut b, 0.5, -1.0, 6);
    let e01 = mk_e(&mut b, v0, v1, 3);
    let e12 = mk_e(&mut b, v1, v2, 4);
    let e20 = mk_e(&mut b, v2, v0, 5);
    let e13 = mk_e(&mut b, v1, v3, 7);
    let e30 = mk_e(&mut b, v3, v0, 8);
    let f0 = add_tri_face(
        &mut b,
        [
            (e01, Orientation::Forward),
            (e12, Orientation::Forward),
            (e20, Orientation::Forward),
        ],
        9,
    );
    let f1 = add_tri_face(
        &mut b,
        [
            (e01, Orientation::Forward),
            (e13, Orientation::Forward),
            (e30, Orientation::Forward),
        ],
        12,
    );
    let s0 = mk_shell(&mut b, ShellKind::Closed, vec![f0, f1], 16);
    let r0 = mk_region(&mut b, s0, vec![], 17);
    mk_body(&mut b, vec![r0], 18);
    b.build().expect_err("same-direction pair must be rejected")
}

// ── Low-ceremony helpers (vertex / edge / face / loop / coedge / shell …) ──

fn mk_v(b: &mut TopologyBuilder, x: f64, y: f64, p: u8) -> VertexId {
    b.add_vertex(VertexParams {
        position: pos(x, y, 0.0),
        tolerance: tol(),
        provenance: prov(p),
    })
    .expect("vertex")
}

fn mk_e(b: &mut TopologyBuilder, sv: VertexId, ev: VertexId, p: u8) -> EdgeId {
    b.add_edge(EdgeParams {
        curve: fake_curve3(),
        parameter_interval: interval(0.0, 1.0),
        start_vertex: sv,
        end_vertex: ev,
        tolerance: tol(),
        provenance: prov(p),
    })
    .expect("edge")
}

fn mk_face(b: &mut TopologyBuilder, p: u8) -> FaceId {
    b.add_face(FaceParams {
        surface: fake_surface(),
        orientation: Orientation::Forward,
        provenance: prov(p),
    })
    .expect("face")
}

fn mk_loop(b: &mut TopologyBuilder, face: FaceId, kind: LoopKind, p: u8) -> LoopId {
    b.add_loop(LoopParams {
        face,
        kind,
        provenance: prov(p),
    })
    .expect("loop")
}

fn mk_coedge(b: &mut TopologyBuilder, edge: EdgeId, loop_id: LoopId, o: Orientation, p: u8) {
    b.add_coedge(CoedgeParams {
        edge,
        loop_id,
        orientation: o,
        pcurve: fake_curve2(),
        provenance: prov(p),
    })
    .expect("coedge");
}

fn mk_shell(b: &mut TopologyBuilder, kind: ShellKind, faces: Vec<FaceId>, p: u8) -> ShellId {
    b.add_shell(ShellParams {
        kind,
        faces,
        provenance: prov(p),
    })
    .expect("shell")
}

fn mk_region(b: &mut TopologyBuilder, outer: ShellId, inner: Vec<ShellId>, p: u8) -> RegionId {
    b.add_region(RegionParams {
        outer_shell: outer,
        inner_shells: inner,
        provenance: prov(p),
    })
    .expect("region")
}

fn mk_body(b: &mut TopologyBuilder, regions: Vec<RegionId>, p: u8) {
    b.add_body(BodyParams {
        regions,
        provenance: prov(p),
    })
    .expect("body");
}

fn sorted_semantic_ids(mut ids: Vec<SemanticId>) -> Vec<SemanticId> {
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn find_nested_error(
    err: &TopologyError,
    predicate: fn(&TopologyError) -> bool,
) -> Option<&TopologyError> {
    if predicate(err) {
        Some(err)
    } else if let TopologyError::Multiple(errors) = err {
        errors
            .iter()
            .find_map(|error| find_nested_error(error, predicate))
    } else {
        None
    }
}

// ── T1.1 Arena and handle tests ───────────────────────────────────────────────

#[test]
fn vertex_roundtrip_through_store() {
    // v1 from build_triangle_face is at pos(1.0, 0.0, 0.0) — verify round-trip.
    let (store, _f0, _v0, v1, _v2) = build_open_triangle_store();
    let v = store.vertex(v1).expect("lookup vertex");
    assert!((v.position().x() - 1.0_f64).abs() < f64::EPSILON);
    assert!((v.position().y() - 0.0_f64).abs() < f64::EPSILON);
    assert!((v.position().z() - 0.0_f64).abs() < f64::EPSILON);
}

#[test]
fn deterministic_vertex_slot_order() {
    let mut b = test_builder();
    let v0 = b
        .add_vertex(VertexParams {
            position: pos(0.0, 0.0, 0.0),
            tolerance: tol(),
            provenance: prov(1),
        })
        .expect("v0");
    let v1 = b
        .add_vertex(VertexParams {
            position: pos(1.0, 0.0, 0.0),
            tolerance: tol(),
            provenance: prov(2),
        })
        .expect("v1");
    assert_eq!(v0.handle().slot(), 0);
    assert_eq!(v1.handle().slot(), 1);
    assert_eq!(v0.handle().generation(), 0);
}

#[test]
fn deterministic_iteration_order() {
    let mut b = test_builder();
    let v0 = b
        .add_vertex(VertexParams {
            position: pos(0.0, 0.0, 0.0),
            tolerance: tol(),
            provenance: prov(0),
        })
        .expect("v0");
    let v1 = b
        .add_vertex(VertexParams {
            position: pos(1.0, 0.0, 0.0),
            tolerance: tol(),
            provenance: prov(1),
        })
        .expect("v1");
    let v2 = b
        .add_vertex(VertexParams {
            position: pos(2.0, 0.0, 0.0),
            tolerance: tol(),
            provenance: prov(2),
        })
        .expect("v2");
    let e01 = b
        .add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: v0,
            end_vertex: v1,
            tolerance: tol(),
            provenance: prov(10),
        })
        .expect("e01");
    let e12 = b
        .add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: v1,
            end_vertex: v2,
            tolerance: tol(),
            provenance: prov(11),
        })
        .expect("e12");
    let e20 = b
        .add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: v2,
            end_vertex: v0,
            tolerance: tol(),
            provenance: prov(12),
        })
        .expect("e20");
    let f0 = add_tri_face(
        &mut b,
        [
            (e01, Orientation::Forward),
            (e12, Orientation::Forward),
            (e20, Orientation::Forward),
        ],
        13,
    );
    add_single_face_body(
        &mut b,
        f0,
        &[
            (e01, Orientation::Forward),
            (e12, Orientation::Forward),
            (e20, Orientation::Forward),
        ],
    );
    let store = b.build().expect("store");
    let xs: Vec<f64> = store.vertices().map(|v| v.position().x()).collect();
    assert_eq!(xs, vec![0.0, 1.0, 2.0]);
    // Iterate twice: same order.
    let xs2: Vec<f64> = store.vertices().map(|v| v.position().x()).collect();
    assert_eq!(xs, xs2);
}

// ── T1.2 Builder and validated construction ───────────────────────────────────

#[test]
fn build_minimal_open_triangle() {
    let mut b = test_builder();
    let (f0, _, _, _, e0, e1, e2) = build_triangle_face_with_edges(&mut b);
    close_face_as_solid(
        &mut b,
        f0,
        &[
            (e0, Orientation::Forward),
            (e1, Orientation::Forward),
            (e2, Orientation::Forward),
        ],
        30,
    );
    let store = b.build().expect("build");
    assert_eq!(store.vertex_count(), 3);
    assert_eq!(store.edge_count(), 3);
    assert_eq!(store.coedge_count(), 6);
    assert_eq!(store.loop_count(), 2);
    assert_eq!(store.face_count(), 2);
    assert_eq!(store.shell_count(), 1);
    assert_eq!(store.region_count(), 1);
    assert_eq!(store.body_count(), 1);
}

// ── T1.3 Reference validation and stale handle detection ─────────────────────

#[test]
fn stale_vertex_handle_from_other_builder() {
    // Create a vertex in snapshot CC, then try to use it in a different snapshot DD.
    let v_stale = {
        let mut b0 = TopologyBuilder::with_lineage_and_snapshot(test_lineage(), test_snapshot());
        b0.add_vertex(VertexParams {
            position: pos(0.0, 0.0, 0.0),
            tolerance: tol(),
            provenance: prov(1),
        })
        .expect("v")
    };

    // Build topology with that stale vertex in one edge – build() must reject it
    // because the snapshot ID in v_stale (CC) != b2's snapshot (DD).
    let mut b2 = TopologyBuilder::with_lineage_and_snapshot(test_lineage(), test_snapshot_b());
    let vgood = b2
        .add_vertex(VertexParams {
            position: pos(1.0, 0.0, 0.0),
            tolerance: tol(),
            provenance: prov(6),
        })
        .expect("vg");
    let vgood2 = b2
        .add_vertex(VertexParams {
            position: pos(0.0, 1.0, 0.0),
            tolerance: tol(),
            provenance: prov(7),
        })
        .expect("vg2");
    // e_bad uses v_stale (snapshot=CC) as start; builder is snapshot=DD → WrongSnapshot
    let e_bad = b2
        .add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: v_stale,
            end_vertex: vgood,
            tolerance: tol(),
            provenance: prov(8),
        })
        .expect("e");
    let e1 = b2
        .add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: vgood,
            end_vertex: vgood2,
            tolerance: tol(),
            provenance: prov(9),
        })
        .expect("e1");
    let e2 = b2
        .add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: vgood2,
            end_vertex: vgood,
            tolerance: tol(),
            provenance: prov(10),
        })
        .expect("e2");
    let f1 = add_tri_face(
        &mut b2,
        [
            (e_bad, Orientation::Forward),
            (e1, Orientation::Forward),
            (e2, Orientation::Forward),
        ],
        11,
    );
    add_single_face_body(
        &mut b2,
        f1,
        &[
            (e_bad, Orientation::Forward),
            (e1, Orientation::Forward),
            (e2, Orientation::Forward),
        ],
    );
    let result = b2.build();
    assert!(
        matches!(
            result,
            Err(TopologyError::WrongSnapshot {
                kind: TopologyKind::Vertex,
                ..
            })
        ),
        "expected wrong snapshot, got: {result:?}"
    );
}

#[test]
fn stale_handle_store_lookup() {
    let mut b0 = TopologyBuilder::with_lineage_and_snapshot(test_lineage(), test_snapshot());
    let (f0, _, _, _, e0, e1, e2) = build_triangle_face_with_edges(&mut b0);
    close_face_as_solid(
        &mut b0,
        f0,
        &[
            (e0, Orientation::Forward),
            (e1, Orientation::Forward),
            (e2, Orientation::Forward),
        ],
        30,
    );
    let store0 = b0.build().expect("store0");

    // Build a successor store with snapshot_b (generation 1); slot 0 exists but snapshot differs.
    let mut b1 = store0
        .successor_builder(test_snapshot_b())
        .expect("successor");
    let (f1, _, _, _, e0, e1, e2) = build_triangle_face_with_edges(&mut b1);
    close_face_as_solid(
        &mut b1,
        f1,
        &[
            (e0, Orientation::Forward),
            (e1, Orientation::Forward),
            (e2, Orientation::Forward),
        ],
        30,
    );
    let store1 = b1.build().expect("store1");

    // VertexId from store0 (snapshot=CC) used against store1 (snapshot=DD)
    let v_from_store0 = store0.vertices().next().expect("vertex").id();
    let result = store1.vertex(v_from_store0);
    assert!(
        matches!(
            result,
            Err(TopologyError::WrongSnapshot {
                kind: TopologyKind::Vertex,
                ..
            })
        ),
        "expected wrong snapshot: {result:?}"
    );
}

#[test]
fn wrong_lineage_handle_rejected() {
    let mut b0 = TopologyBuilder::with_lineage_and_snapshot(test_lineage(), test_snapshot());
    let (f0, v_l0, _, _, e0, e1, e2) = build_triangle_face_with_edges(&mut b0);
    close_face_as_solid(
        &mut b0,
        f0,
        &[
            (e0, Orientation::Forward),
            (e1, Orientation::Forward),
            (e2, Orientation::Forward),
        ],
        30,
    );
    let _store0 = b0.build().expect("store0");

    let lineage_b = TopologyLineageId::new(SemanticId::from_bytes([0xBB; 16]));
    let snapshot_b = TopologySnapshotId::new(SemanticId::from_bytes([0xBB; 16]));
    let mut b1 = TopologyBuilder::with_lineage_and_snapshot(lineage_b, snapshot_b);
    let (f1, _, _, _, e3, e4, e5) = build_triangle_face_with_edges(&mut b1);
    close_face_as_solid(
        &mut b1,
        f1,
        &[
            (e3, Orientation::Forward),
            (e4, Orientation::Forward),
            (e5, Orientation::Forward),
        ],
        30,
    );
    let store1 = b1.build().expect("store1");

    let err = store1.vertex(v_l0).expect_err("wrong lineage");
    assert!(
        matches!(err, TopologyError::WrongLineage { .. }),
        "expected WrongLineage, got: {err:?}"
    );
}

#[test]
fn missing_entity_slot() {
    let mut b = test_builder();
    let (f0, _, _, _, e0, e1, e2) = build_triangle_face_with_edges(&mut b);
    close_face_as_solid(
        &mut b,
        f0,
        &[
            (e0, Orientation::Forward),
            (e1, Orientation::Forward),
            (e2, Orientation::Forward),
        ],
        30,
    );
    let store = b.build().expect("store");
    // Slot 99 does not exist (right lineage/snapshot, valid generation, out-of-range slot)
    let bad_id = VertexId::new(99, 0, test_lineage(), test_snapshot());
    assert!(matches!(
        store.vertex(bad_id),
        Err(TopologyError::MissingEntity {
            kind: TopologyKind::Vertex,
            slot: 99,
            ..
        })
    ));
}

#[test]
fn stale_handle_takes_precedence_over_missing_slot() {
    let (store, _, _) = build_tetrahedron();
    let wrong_gen_id = VertexId::new(
        999,
        store.generation() + 1,
        store.lineage(),
        store.snapshot(),
    );

    let result = store.vertex(wrong_gen_id);
    assert!(
        matches!(result, Err(TopologyError::StaleHandle { .. })),
        "expected StaleHandle (wrong gen takes precedence over missing slot), got: {result:?}"
    );
}

// ── T1.2 Rejection of structurally invalid input ──────────────────────────────

#[test]
fn empty_body_rejected() {
    let mut b = test_builder();
    b.add_body(BodyParams {
        regions: vec![],
        provenance: prov(1),
    })
    .expect("add");
    let err = b.build().expect_err("should fail");
    assert!(
        matches!(err, TopologyError::EmptyBody { .. }),
        "got: {err:?}"
    );
}

#[test]
fn empty_shell_rejected() {
    let mut b = test_builder();
    let s0 = b
        .add_shell(ShellParams {
            kind: ShellKind::Closed,
            faces: vec![],
            provenance: prov(1),
        })
        .expect("shell");
    let r0 = b
        .add_region(RegionParams {
            outer_shell: s0,
            inner_shells: vec![],
            provenance: prov(2),
        })
        .expect("region");
    b.add_body(BodyParams {
        regions: vec![r0],
        provenance: prov(3),
    })
    .expect("body");
    let err = b.build().expect_err("should fail");
    assert!(
        matches!(err, TopologyError::EmptyShell { .. }),
        "got: {err:?}"
    );
}

#[test]
fn face_with_no_outer_loop_rejected() {
    let mut b = test_builder();
    let f0 = b
        .add_face(FaceParams {
            surface: fake_surface(),
            orientation: Orientation::Forward,
            provenance: prov(1),
        })
        .expect("face");
    // No loop references f0 → MissingOuterLoop
    let v0 = b
        .add_vertex(VertexParams {
            position: pos(0.0, 0.0, 0.0),
            tolerance: tol(),
            provenance: prov(2),
        })
        .expect("v0");
    let v1 = b
        .add_vertex(VertexParams {
            position: pos(1.0, 0.0, 0.0),
            tolerance: tol(),
            provenance: prov(3),
        })
        .expect("v1");
    let e0 = b
        .add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: v0,
            end_vertex: v1,
            tolerance: tol(),
            provenance: prov(4),
        })
        .expect("e");
    // Orphan face2 that will supply a loop (so we have at least one valid face)
    let f1 = b
        .add_face(FaceParams {
            surface: fake_surface(),
            orientation: Orientation::Forward,
            provenance: prov(5),
        })
        .expect("f1");
    let l1 = b
        .add_loop(LoopParams {
            face: f1,
            kind: LoopKind::Outer,
            provenance: prov(6),
        })
        .expect("l1");
    b.add_coedge(CoedgeParams {
        edge: e0,
        loop_id: l1,
        orientation: Orientation::Forward,
        pcurve: fake_curve2(),
        provenance: prov(7),
    })
    .expect("c");
    let s0 = b
        .add_shell(ShellParams {
            kind: ShellKind::Open,
            faces: vec![f0, f1],
            provenance: prov(8),
        })
        .expect("s");
    let r0 = b
        .add_region(RegionParams {
            outer_shell: s0,
            inner_shells: vec![],
            provenance: prov(9),
        })
        .expect("r");
    b.add_body(BodyParams {
        regions: vec![r0],
        provenance: prov(10),
    })
    .expect("body");
    let err = b.build().expect_err("should fail");
    // f0 has no outer loop
    assert!(
        matches!(err, TopologyError::MissingOuterLoop { .. })
            || matches!(err, TopologyError::Multiple(_)),
        "got: {err:?}"
    );
}

#[test]
fn face_with_duplicate_outer_loop_rejected() {
    let mut b = test_builder();
    let v0 = mk_v(&mut b, 0.0, 0.0, 1);
    let v1 = mk_v(&mut b, 1.0, 0.0, 2);
    let v2 = mk_v(&mut b, 0.0, 1.0, 3);
    let e0 = mk_e(&mut b, v0, v1, 4);
    let e1 = mk_e(&mut b, v1, v2, 5);
    let e2 = mk_e(&mut b, v2, v0, 6);
    let f0 = mk_face(&mut b, 7);
    // Two outer loops for the same face — must be rejected
    let l0 = mk_loop(&mut b, f0, LoopKind::Outer, 8);
    let l1 = mk_loop(&mut b, f0, LoopKind::Outer, 9);
    mk_coedge(&mut b, e0, l0, Orientation::Forward, 10);
    mk_coedge(&mut b, e1, l0, Orientation::Forward, 11);
    mk_coedge(&mut b, e2, l0, Orientation::Forward, 12);
    mk_coedge(&mut b, e2, l1, Orientation::Reversed, 13);
    mk_coedge(&mut b, e1, l1, Orientation::Reversed, 14);
    mk_coedge(&mut b, e0, l1, Orientation::Reversed, 15);
    let s0 = mk_shell(&mut b, ShellKind::Open, vec![f0], 16);
    let r0 = mk_region(&mut b, s0, vec![], 17);
    mk_body(&mut b, vec![r0], 18);
    let err = b.build().expect_err("duplicate outer loop");
    assert!(
        matches!(err, TopologyError::DuplicateOuterLoop { .. })
            || matches!(err, TopologyError::Multiple(_)),
        "got: {err:?}"
    );
}

#[test]
fn empty_loop_rejected() {
    let mut b = test_builder();
    let f0 = b
        .add_face(FaceParams {
            surface: fake_surface(),
            orientation: Orientation::Forward,
            provenance: prov(1),
        })
        .expect("f");
    // Loop references f0 but no coedges will reference this loop
    b.add_loop(LoopParams {
        face: f0,
        kind: LoopKind::Outer,
        provenance: prov(2),
    })
    .expect("l");
    let s0 = b
        .add_shell(ShellParams {
            kind: ShellKind::Closed,
            faces: vec![f0],
            provenance: prov(3),
        })
        .expect("s");
    let r0 = b
        .add_region(RegionParams {
            outer_shell: s0,
            inner_shells: vec![],
            provenance: prov(4),
        })
        .expect("r");
    b.add_body(BodyParams {
        regions: vec![r0],
        provenance: prov(5),
    })
    .expect("body");
    let err = b.build().expect_err("should fail: empty loop");
    assert!(
        matches!(err, TopologyError::EmptyLoop { .. }),
        "got: {err:?}"
    );
}

#[test]
fn open_loop_rejected() {
    let mut b = test_builder();
    // e2 ends at v3 (not v0) so the loop won't close
    let v0 = mk_v(&mut b, 0.0, 0.0, 1);
    let v1 = mk_v(&mut b, 1.0, 0.0, 2);
    let v2 = mk_v(&mut b, 0.0, 1.0, 3);
    let v3 = mk_v(&mut b, 2.0, 2.0, 4);
    let e0 = mk_e(&mut b, v0, v1, 5);
    let e1 = mk_e(&mut b, v1, v2, 6);
    let e2 = mk_e(&mut b, v2, v3, 7);
    let f0 = mk_face(&mut b, 8);
    let l0 = mk_loop(&mut b, f0, LoopKind::Outer, 9);
    mk_coedge(&mut b, e0, l0, Orientation::Forward, 10);
    mk_coedge(&mut b, e1, l0, Orientation::Forward, 11);
    mk_coedge(&mut b, e2, l0, Orientation::Forward, 12);
    let s0 = mk_shell(&mut b, ShellKind::Open, vec![f0], 13);
    let r0 = mk_region(&mut b, s0, vec![], 14);
    mk_body(&mut b, vec![r0], 15);
    let err = b.build().expect_err("open loop");
    assert!(
        matches!(err, TopologyError::LoopVertexMismatch { .. })
            || matches!(err, TopologyError::Multiple(_)),
        "got: {err:?}"
    );
}

#[test]
fn loop_vertex_mismatch_carries_loop_related_id() {
    let mut b = test_builder();
    let v0 = mk_v(&mut b, 0.0, 0.0, 1);
    let v1 = mk_v(&mut b, 1.0, 0.0, 2);
    let v2 = mk_v(&mut b, 0.0, 1.0, 3);
    let v3 = mk_v(&mut b, 2.0, 2.0, 4);
    let e0 = mk_e(&mut b, v0, v1, 5);
    let e1 = mk_e(&mut b, v1, v2, 6);
    let e2 = mk_e(&mut b, v2, v3, 7);
    let f0 = mk_face(&mut b, 8);
    let l0 = mk_loop(&mut b, f0, LoopKind::Outer, 9);
    mk_coedge(&mut b, e0, l0, Orientation::Forward, 10);
    mk_coedge(&mut b, e1, l0, Orientation::Forward, 11);
    mk_coedge(&mut b, e2, l0, Orientation::Forward, 12);
    let s0 = mk_shell(&mut b, ShellKind::Open, vec![f0], 13);
    let r0 = mk_region(&mut b, s0, vec![], 14);
    mk_body(&mut b, vec![r0], 15);

    let err = b.build().expect_err("open loop");
    let mismatch = find_nested_error(&err, |error| {
        matches!(error, TopologyError::LoopVertexMismatch { .. })
    })
    .expect("expected LoopVertexMismatch");
    let TopologyError::LoopVertexMismatch { related, .. } = mismatch else {
        panic!("expected LoopVertexMismatch, got {mismatch:?}");
    };
    assert_eq!(
        related,
        &sorted_semantic_ids(vec![
            sem(1),
            sem(4),
            sem(5),
            sem(7),
            sem(9),
            sem(10),
            sem(12),
        ])
    );
}

#[test]
fn non_manifold_edge_rejected() {
    // Three triangles sharing one edge (used 3 times) → non-manifold
    let mut b = test_builder();
    let v0 = mk_v(&mut b, 0.0, 0.0, 1);
    let v1 = mk_v(&mut b, 1.0, 0.0, 2);
    let v2 = mk_v(&mut b, 0.0, 1.0, 3);
    let v3 = mk_v(&mut b, 0.0, -1.0, 4);
    let v4 = mk_v(&mut b, -1.0, 0.0, 5);
    let shared = mk_e(&mut b, v0, v1, 6);
    let e1 = mk_e(&mut b, v1, v2, 7);
    let e2 = mk_e(&mut b, v2, v0, 8);
    let e3 = mk_e(&mut b, v1, v3, 9);
    let e4 = mk_e(&mut b, v3, v0, 10);
    let e5 = mk_e(&mut b, v1, v4, 11);
    let e6 = mk_e(&mut b, v4, v0, 12);
    // Face 0: v0-v1-v2 (shared Forward)
    let f0 = add_tri_face(
        &mut b,
        [
            (shared, Orientation::Forward),
            (e1, Orientation::Forward),
            (e2, Orientation::Forward),
        ],
        13,
    );
    // Face 1: v1-v3-v0 (shared Reversed — valid pair so far)
    let f1 = add_tri_face(
        &mut b,
        [
            (e3, Orientation::Forward),
            (e4, Orientation::Forward),
            (shared, Orientation::Reversed),
        ],
        16,
    );
    // Face 2: v1-v4-v0 (shared Reversed a third time — non-manifold!)
    let f2 = add_tri_face(
        &mut b,
        [
            (e5, Orientation::Forward),
            (e6, Orientation::Forward),
            (shared, Orientation::Reversed),
        ],
        19,
    );
    let s0 = mk_shell(&mut b, ShellKind::Open, vec![f0, f1, f2], 28);
    let r0 = mk_region(&mut b, s0, vec![], 29);
    mk_body(&mut b, vec![r0], 30);
    let err = b.build().expect_err("non-manifold");
    assert!(
        matches!(err, TopologyError::NonManifoldEdge { use_count: 3, .. })
            || matches!(err, TopologyError::Multiple(_)),
        "expected non-manifold error, got: {err:?}"
    );
}

#[test]
fn closed_edge_single_vertex_loop() {
    // A circle edge where start == end vertex, used in a single-coedge loop.
    let mut b = test_builder();
    let v0 = b
        .add_vertex(VertexParams {
            position: pos(0.0, 0.0, 0.0),
            tolerance: tol(),
            provenance: prov(1),
        })
        .expect("v0");
    // Closed edge: same vertex for start and end
    let eclosed = b
        .add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, std::f64::consts::TAU),
            start_vertex: v0,
            end_vertex: v0, // ← same vertex: closed edge
            tolerance: tol(),
            provenance: prov(2),
        })
        .expect("eclosed");
    let f0 = b
        .add_face(FaceParams {
            surface: fake_surface(),
            orientation: Orientation::Forward,
            provenance: prov(3),
        })
        .expect("f");
    let l0 = b
        .add_loop(LoopParams {
            face: f0,
            kind: LoopKind::Outer,
            provenance: prov(4),
        })
        .expect("l");
    b.add_coedge(CoedgeParams {
        edge: eclosed,
        loop_id: l0,
        orientation: Orientation::Forward,
        pcurve: fake_curve2(),
        provenance: prov(5),
    })
    .expect("c");
    let mirror_face = mk_face(&mut b, 40);
    let mirror_loop = mk_loop(&mut b, mirror_face, LoopKind::Outer, 41);
    mk_coedge(&mut b, eclosed, mirror_loop, Orientation::Reversed, 42);
    let s0 = mk_shell(&mut b, ShellKind::Closed, vec![f0, mirror_face], 43);
    let r0 = mk_region(&mut b, s0, vec![], 44);
    mk_body(&mut b, vec![r0], 45);
    let store = b.build().expect("closed-edge loop should build");
    // The loop has exactly 1 coedge
    let face = store.face(f0).expect("face");
    let lp = store.get_loop(face.outer_loop()).expect("loop");
    assert_eq!(lp.coedges().len(), 1);
    // Vertex incident edges: the closed edge contributes exactly once
    let edge_id = store.edges().next().expect("edge").id();
    let vertex = store.vertex(v0).expect("vertex");
    assert_eq!(vertex.incident_edges(), &[edge_id]);
}

// ── T1.4 Orientation-aware traversal ─────────────────────────────────────────

#[test]
fn coedge_start_end_forward() {
    let (store, f0, v0, v1, _) = build_open_triangle_store();
    let face = store.face(f0).expect("face");
    let lp = store.get_loop(face.outer_loop()).expect("loop");
    // First coedge is Forward; it traverses v0→v1
    let c0 = lp.coedges()[0];
    let start = coedge_start_vertex_id(&store, c0).expect("start");
    let end = coedge_end_vertex_id(&store, c0).expect("end");
    assert_eq!(start, v0);
    assert_eq!(end, v1);
}

#[test]
fn coedge_start_end_reversed() {
    let (store, c_rev, v0, v2) = build_reversed_triangle_store();
    let start = coedge_start_vertex_id(&store, c_rev).expect("start");
    let end = coedge_end_vertex_id(&store, c_rev).expect("end");
    assert_eq!(start, v2, "reversed start should be v2");
    assert_eq!(end, v0, "reversed end should be v0");
}

#[test]
fn loop_traversal_order() {
    let (store, f0, _, _, _) = build_open_triangle_store();
    let face = store.face(f0).expect("face");
    let loop_id = face.outer_loop();
    let coedges = loop_coedge_ids(&store, loop_id).expect("coedges");
    assert_eq!(coedges.len(), 3);
    // Check that consecutive coedges share a vertex.
    for i in 0..3 {
        let end = coedge_end_vertex_id(&store, coedges[i]).expect("end");
        let start = coedge_start_vertex_id(&store, coedges[(i + 1) % 3]).expect("start");
        assert_eq!(end, start, "loop discontinuity at position {i}");
    }
}

#[test]
fn loop_next_coedge_wraps() {
    let (store, f0, _, _, _) = build_open_triangle_store();
    let face = store.face(f0).expect("face");
    let loop_id = face.outer_loop();
    let coedges = loop_coedge_ids(&store, loop_id).expect("coedges");
    let next = loop_next_coedge_id(&store, loop_id, coedges[2]).expect("next");
    assert_eq!(next, coedges[0], "wrap-around to first coedge");
}

// ── T1.3 Back-references computed by builder ─────────────────────────────────

#[test]
fn vertex_incident_edges_back_ref() {
    let (store, _, v0, v1, _) = build_open_triangle_store();
    let vertex = store.vertex(v0).expect("v0");
    // v0 is used by two edges: e0 (v0→v1) and e2 (v2→v0)
    assert_eq!(vertex.incident_edges().len(), 2, "v0 incident edges");
    let vertex1 = store.vertex(v1).expect("v1");
    // v1 is used by e0 (v0→v1) and e1 (v1→v2)
    assert_eq!(vertex1.incident_edges().len(), 2, "v1 incident edges");
}

#[test]
fn edge_coedges_back_ref() {
    let (store, _, _, _, _) = build_open_triangle_store();
    for edge in store.edges() {
        assert_eq!(
            edge.coedges().len(),
            2,
            "edge {:?} should have 2 coedges",
            edge.id()
        );
    }
}

#[test]
fn loop_coedges_back_ref_computed() {
    let (store, f0, _, _, _) = build_open_triangle_store();
    let face = store.face(f0).expect("face");
    let lp = store.get_loop(face.outer_loop()).expect("loop");
    assert_eq!(lp.coedges().len(), 3);
}

#[test]
fn face_outer_and_inner_loops_back_ref() {
    let (store, shell_id, _) = build_tetrahedron();
    let shell = store.shell(shell_id).expect("shell");
    let face_id = shell.faces()[0];
    let face = store.face(face_id).expect("face");
    let outer_loop = store.get_loop(face.outer_loop()).expect("outer loop");
    assert_eq!(outer_loop.face(), face_id);
    assert!(face.inner_loops().is_empty());
}

// ── T1.5 Euler characteristic ─────────────────────────────────────────────────

#[test]
fn euler_single_triangle_face() {
    let (store, _, _, _, _) = build_open_triangle_store();
    let region = store.regions().next().expect("region");
    let m = shell_euler_metrics(&store, region.outer_shell()).expect("metrics");
    assert_eq!(m.vertices, 3);
    assert_eq!(m.edges, 3);
    assert_eq!(m.faces, 2);
    assert_eq!(
        m.euler_ve_f, 2,
        "V-E+F for closed triangle pair should be 2"
    );
    assert_eq!(m.inner_loops, 0);
}

/// Build a tetrahedron and verify the Euler characteristic χ = V - E + F = 2.
///
/// Tetrahedron: 4 vertices, 6 edges, 4 triangular faces.
fn build_tetrahedron() -> (TopologyStore, ShellId, BodyId) {
    let mut b = test_builder();

    let mk_v = |b: &mut TopologyBuilder, x, y, z, p| {
        b.add_vertex(VertexParams {
            position: pos(x, y, z),
            tolerance: tol(),
            provenance: prov(p),
        })
        .expect("v")
    };
    let mk_e = |b: &mut TopologyBuilder, sv, ev, p| {
        b.add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: sv,
            end_vertex: ev,
            tolerance: tol(),
            provenance: prov(p),
        })
        .expect("e")
    };

    let v0 = mk_v(&mut b, 1.0, 0.0, -0.707, 1);
    let v1 = mk_v(&mut b, -1.0, 0.0, -0.707, 2);
    let v2 = mk_v(&mut b, 0.0, 1.0, 0.707, 3);
    let v3 = mk_v(&mut b, 0.0, -1.0, 0.707, 4);

    let e01 = mk_e(&mut b, v0, v1, 5);
    let e02 = mk_e(&mut b, v0, v2, 6);
    let e03 = mk_e(&mut b, v0, v3, 7);
    let e12 = mk_e(&mut b, v1, v2, 8);
    let e13 = mk_e(&mut b, v1, v3, 9);
    let e23 = mk_e(&mut b, v2, v3, 10);

    // 4 triangular faces — each shared edge has opposing orientations on adjacent faces.
    let f0 = add_tri_face(
        &mut b,
        [
            (e01, Orientation::Forward),
            (e12, Orientation::Forward),
            (e02, Orientation::Reversed),
        ],
        11,
    );
    let f1 = add_tri_face(
        &mut b,
        [
            (e03, Orientation::Forward),
            (e13, Orientation::Reversed),
            (e01, Orientation::Reversed),
        ],
        14,
    );
    let f2 = add_tri_face(
        &mut b,
        [
            (e02, Orientation::Forward),
            (e23, Orientation::Forward),
            (e03, Orientation::Reversed),
        ],
        17,
    );
    let f3 = add_tri_face(
        &mut b,
        [
            (e12, Orientation::Reversed),
            (e13, Orientation::Forward),
            (e23, Orientation::Reversed),
        ],
        20,
    );

    let s0 = b
        .add_shell(ShellParams {
            kind: ShellKind::Closed,
            faces: vec![f0, f1, f2, f3],
            provenance: prov(31),
        })
        .expect("s");
    let r0 = b
        .add_region(RegionParams {
            outer_shell: s0,
            inner_shells: vec![],
            provenance: prov(32),
        })
        .expect("r");
    let body_id = b
        .add_body(BodyParams {
            regions: vec![r0],
            provenance: prov(33),
        })
        .expect("body");
    let store = b.build().expect("tetrahedron should build");
    (store, s0, body_id)
}

#[test]
fn euler_tetrahedron_characteristic_is_2() {
    let (store, s0, _) = build_tetrahedron();
    let m = shell_euler_metrics(&store, s0).expect("metrics");
    assert_eq!(m.vertices, 4, "V");
    assert_eq!(m.edges, 6, "E");
    assert_eq!(m.faces, 4, "F");
    assert_eq!(m.euler_ve_f, 2, "V-E+F for closed sphere-topology shell");
    assert_eq!(m.inner_loops, 0);
}

#[test]
fn is_closed_manifold_tetrahedron() {
    let (store, s0, _) = build_tetrahedron();
    let closed = is_closed_manifold_shell(&store, s0).expect("check");
    assert!(closed, "tetrahedron is closed manifold");
}

#[test]
fn is_not_closed_manifold_open_shell() {
    let (store, f0, _, _, _) = build_open_triangle_store();
    let region = store.regions().next().expect("region");
    let closed = is_closed_manifold_shell(&store, region.outer_shell()).expect("check");
    assert!(closed, "closed triangle pair is a closed manifold");
    assert_eq!(store.face(f0).expect("face").inner_loops().len(), 0);
}

#[test]
fn body_euler_ve_f_tetrahedron() {
    let (store, _, body_id) = build_tetrahedron();
    let chi = body_total_euler_ve_f(&store, body_id).expect("euler");
    assert_eq!(chi, 2, "body V-E+F = 2 for tetrahedron");
}

// ── T1.5 Adjacency queries ────────────────────────────────────────────────────

#[test]
fn face_adjacent_faces_tetrahedron() {
    let (store, s0, _) = build_tetrahedron();
    let shell = store.shell(s0).expect("shell");
    let f0 = shell.faces()[0];
    let adjacent = face_adjacent_face_ids(&store, f0).expect("adjacent");
    // In a tetrahedron each face is adjacent to exactly 3 others
    assert_eq!(adjacent.len(), 3, "tetrahedron face has 3 adjacent faces");
    assert!(!adjacent.contains(&f0), "face is not adjacent to itself");
}

#[test]
fn edge_adjacent_faces_manifold_edge() {
    let (store, s0, _) = build_tetrahedron();
    let shell = store.shell(s0).expect("shell");
    let face = store.face(shell.faces()[0]).expect("face");
    let lp = store.get_loop(face.outer_loop()).expect("loop");
    let coedge = store.coedge(lp.coedges()[0]).expect("coedge");
    let edge_id = coedge.edge();
    let adjacent_faces = edge_adjacent_face_ids(&store, edge_id).expect("adjacent");
    // Each edge in a tetrahedron is shared by exactly 2 faces
    assert_eq!(adjacent_faces.len(), 2);
}

#[test]
fn face_loop_ids_returns_outer_first() {
    let (store, f0, _, _, _) = build_open_triangle_store();
    let loops = face_loop_ids(&store, f0).expect("loops");
    assert_eq!(loops.len(), 1);
    let face = store.face(f0).expect("face");
    assert_eq!(loops[0], face.outer_loop());
}

// ── T1.5 Closed shell kind validation ────────────────────────────────────────

#[test]
fn closed_shell_kind_rejected_for_open_topology() {
    let mut b = test_builder();
    let (f0, _, _, _) = build_triangle_face(&mut b);
    // Claim Closed but topology is open (3 boundary edges, each with 1 coedge)
    let s0 = b
        .add_shell(ShellParams {
            kind: ShellKind::Closed,
            faces: vec![f0],
            provenance: prov(20),
        })
        .expect("s");
    let r0 = b
        .add_region(RegionParams {
            outer_shell: s0,
            inner_shells: vec![],
            provenance: prov(21),
        })
        .expect("r");
    b.add_body(BodyParams {
        regions: vec![r0],
        provenance: prov(22),
    })
    .expect("body");
    let err = b.build().expect_err("should fail: inconsistent shell kind");
    assert!(
        matches!(err, TopologyError::InconsistentShellKind { .. }),
        "got: {err:?}"
    );
}

// ── T1.6 Provenance preservation ─────────────────────────────────────────────

#[test]
fn provenance_semantic_id_preserved() {
    let mut b = test_builder();
    let vid = b
        .add_vertex(VertexParams {
            position: pos(1.0, 2.0, 3.0),
            tolerance: tol(),
            provenance: prov(0xAB),
        })
        .expect("v");
    let v1 = b
        .add_vertex(VertexParams {
            position: pos(0.0, 0.0, 0.0),
            tolerance: tol(),
            provenance: prov(1),
        })
        .expect("v1");
    let v2 = b
        .add_vertex(VertexParams {
            position: pos(0.0, 1.0, 0.0),
            tolerance: tol(),
            provenance: prov(2),
        })
        .expect("v2");
    let e0 = b
        .add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: vid,
            end_vertex: v1,
            tolerance: tol(),
            provenance: prov(3),
        })
        .expect("e0");
    let e1 = b
        .add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: v1,
            end_vertex: v2,
            tolerance: tol(),
            provenance: prov(4),
        })
        .expect("e1");
    let e2 = b
        .add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: v2,
            end_vertex: vid,
            tolerance: tol(),
            provenance: prov(5),
        })
        .expect("e2");
    let f0 = add_tri_face(
        &mut b,
        [
            (e0, Orientation::Forward),
            (e1, Orientation::Forward),
            (e2, Orientation::Forward),
        ],
        6,
    );
    add_single_face_body(
        &mut b,
        f0,
        &[
            (e0, Orientation::Forward),
            (e1, Orientation::Forward),
            (e2, Orientation::Forward),
        ],
    );
    let store = b.build().expect("store");
    let v = store.vertex(vid).expect("vertex");
    assert_eq!(v.provenance().semantic_id(), sem(0xAB));
    assert!(v.provenance().created_by().is_none());
}

#[test]
fn provenance_operation_id_preserved() {
    let mut b = test_builder();
    let vid = b
        .add_vertex(VertexParams {
            position: pos(0.0, 0.0, 0.0),
            tolerance: tol(),
            provenance: prov_with_op(0x11, 0x22),
        })
        .expect("v");
    let v1 = b
        .add_vertex(VertexParams {
            position: pos(1.0, 0.0, 0.0),
            tolerance: tol(),
            provenance: prov(1),
        })
        .expect("v1");
    let v2 = b
        .add_vertex(VertexParams {
            position: pos(0.0, 1.0, 0.0),
            tolerance: tol(),
            provenance: prov(2),
        })
        .expect("v2");
    let e0 = b
        .add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: vid,
            end_vertex: v1,
            tolerance: tol(),
            provenance: prov(3),
        })
        .expect("e0");
    let e1 = b
        .add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: v1,
            end_vertex: v2,
            tolerance: tol(),
            provenance: prov(4),
        })
        .expect("e1");
    let e2 = b
        .add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: v2,
            end_vertex: vid,
            tolerance: tol(),
            provenance: prov(5),
        })
        .expect("e2");
    let f0 = add_tri_face(
        &mut b,
        [
            (e0, Orientation::Forward),
            (e1, Orientation::Forward),
            (e2, Orientation::Forward),
        ],
        6,
    );
    add_single_face_body(
        &mut b,
        f0,
        &[
            (e0, Orientation::Forward),
            (e1, Orientation::Forward),
            (e2, Orientation::Forward),
        ],
    );
    let store = b.build().expect("store");
    let v = store.vertex(vid).expect("vertex");
    let op = v.provenance().created_by().expect("op id");
    assert_eq!(op, OperationId::from_bytes([0x22; 16]));
}

#[test]
fn provenance_derived_from_sorted_and_deduped() {
    let role = ProvenanceRole::try_new("test.derived").expect("role");
    let sources = vec![sem(3), sem(1), sem(2), sem(1)]; // duplicates, out of order
    let prov = amphion_topology::Provenance::new(sem(99), None, sources, role);
    // Provenance sorts and deduplicates source IDs
    let derived = prov.derived_from();
    assert_eq!(derived.len(), 3, "should deduplicate");
    assert!(derived.windows(2).all(|w| w[0] < w[1]), "should be sorted");
}

// ── Serialize-only tests (Deserialize removed from entity structs) ────────────

/// Vertex serializes to valid JSON (deserialization bypasses validation → not supported).
#[test]
fn vertex_serializes_to_json() {
    let (store, _, _, _, _) = build_open_triangle_store();
    let vertex = store.vertices().next().expect("vertex");
    let json = serde_json::to_string(vertex).expect("serialize");
    assert!(
        json.contains("position"),
        "JSON should contain position field"
    );
}

/// Serializing and re-serializing the same vertex produces identical JSON.
#[test]
fn edge_serializes_to_json() {
    let (store, _, _, _, _) = build_open_triangle_store();
    let edge = store.edges().next().expect("edge");
    let json = serde_json::to_string(edge).expect("serialize");
    assert!(json.contains("curve"), "JSON should contain curve field");
}

// ── Deterministic serialization hash ─────────────────────────────────────────

#[test]
fn two_identical_builds_produce_identical_json() {
    fn build_json() -> String {
        let mut b = test_builder();
        let (f0, _, _, _, e0, e1, e2) = build_triangle_face_with_edges(&mut b);
        let body_id = close_face_as_solid(
            &mut b,
            f0,
            &[
                (e0, Orientation::Forward),
                (e1, Orientation::Forward),
                (e2, Orientation::Forward),
            ],
            30,
        );
        let store = b.build().expect("store");
        serde_json::to_string(store.body(body_id).expect("body")).expect("json")
    }
    assert_eq!(build_json(), build_json());
}

// ── Orientation compose ───────────────────────────────────────────────────────

#[test]
fn orientation_compose() {
    use amphion_topology::Orientation::{Forward, Reversed};
    assert_eq!(Forward.compose(Forward), Forward);
    assert_eq!(Reversed.compose(Reversed), Forward);
    assert_eq!(Forward.compose(Reversed), Reversed);
    assert_eq!(Reversed.compose(Forward), Reversed);
    assert_eq!(Forward.reversed(), Reversed);
    assert_eq!(Reversed.reversed(), Forward);
}

// ── Multiple errors collated ──────────────────────────────────────────────────

#[test]
fn multiple_validation_errors_collated() {
    let mut b = test_builder();
    // Two bodies with no regions: expect Multiple wrapping two EmptyBody errors
    b.add_body(BodyParams {
        regions: vec![],
        provenance: prov(1),
    })
    .expect("b1");
    b.add_body(BodyParams {
        regions: vec![],
        provenance: prov(2),
    })
    .expect("b2");
    let err = b.build().expect_err("should fail");
    assert!(matches!(err, TopologyError::Multiple(_)), "got: {err:?}");
    if let TopologyError::Multiple(errs) = &err {
        assert_eq!(errs.len(), 2);
        assert!(
            errs.iter()
                .all(|e| matches!(e, TopologyError::EmptyBody { .. }))
        );
    }
}

// ── Builder constructor API ───────────────────────────────────────────────────

#[test]
fn builder_requires_explicit_lineage_and_snapshot() {
    // with_lineage_and_snapshot is the sole public root constructor.
    let b = TopologyBuilder::with_lineage_and_snapshot(test_lineage(), test_snapshot());
    assert_eq!(b.generation(), 0);
    assert_eq!(b.lineage(), test_lineage());
    assert_eq!(b.snapshot(), test_snapshot());
}

// ── Follow-up regression tests ────────────────────────────────────────────────
// These tests cover issues 1–9 from the second-pass review.

// Issue 1: cross-shell edge sharing rejected
// Issue 1: cross-shell edge sharing rejected
#[test]
#[allow(clippy::similar_names)]
fn cross_shell_edge_sharing_rejected() {
    // Two shells each use the same edge (e_shared) with one coedge each → cross-shell.
    let mut b = test_builder();
    let mk_v = |b: &mut TopologyBuilder, x, y, p| {
        b.add_vertex(VertexParams {
            position: pos(x, y, 0.0),
            tolerance: tol(),
            provenance: prov(p),
        })
        .expect("v")
    };
    let mk_e = |b: &mut TopologyBuilder, sv, ev, p| {
        b.add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: sv,
            end_vertex: ev,
            tolerance: tol(),
            provenance: prov(p),
        })
        .expect("e")
    };
    let v0 = mk_v(&mut b, 0.0, 0.0, 0);
    let v1 = mk_v(&mut b, 1.0, 0.0, 1);
    let v2 = mk_v(&mut b, 0.5, 1.0, 2);
    let v3 = mk_v(&mut b, 0.5, -1.0, 3);
    let e_shared = mk_e(&mut b, v0, v1, 4);
    let edge_a1 = mk_e(&mut b, v0, v2, 5);
    let e_a2 = mk_e(&mut b, v1, v2, 6);
    let edge_b1 = mk_e(&mut b, v0, v3, 7);
    let e_b2 = mk_e(&mut b, v1, v3, 8);
    // Shell A: triangle (e_shared Fwd, e_a2 Fwd, edge_a1 Rev)
    let fa = add_tri_face(
        &mut b,
        [
            (e_shared, Orientation::Forward),
            (e_a2, Orientation::Forward),
            (edge_a1, Orientation::Reversed),
        ],
        9,
    );
    // Shell B: triangle (e_shared Rev, edge_b1 Fwd, e_b2 Rev)
    let fb = add_tri_face(
        &mut b,
        [
            (edge_b1, Orientation::Forward),
            (e_b2, Orientation::Reversed),
            (e_shared, Orientation::Reversed),
        ],
        12,
    );
    let sa = b
        .add_shell(ShellParams {
            kind: ShellKind::Open,
            faces: vec![fa],
            provenance: prov(19),
        })
        .expect("sa");
    let sb = b
        .add_shell(ShellParams {
            kind: ShellKind::Open,
            faces: vec![fb],
            provenance: prov(20),
        })
        .expect("sb");
    let ra = b
        .add_region(RegionParams {
            outer_shell: sa,
            inner_shells: vec![],
            provenance: prov(21),
        })
        .expect("ra");
    let rb = b
        .add_region(RegionParams {
            outer_shell: sb,
            inner_shells: vec![],
            provenance: prov(22),
        })
        .expect("rb");
    b.add_body(BodyParams {
        regions: vec![ra, rb],
        provenance: prov(23),
    })
    .expect("body");
    let err = b.build().expect_err("cross-shell edge must be rejected");
    assert!(
        matches!(err, TopologyError::CrossShellEdge { .. })
            || matches!(&err, TopologyError::Multiple(v) if v.iter().any(|e| matches!(e, TopologyError::CrossShellEdge { .. }))),
        "expected CrossShellEdge, got {err}"
    );
}

// Issue 2: same-direction edge pair rejected for closed shell
#[test]
fn same_direction_edge_pair_rejected() {
    let err = same_direction_edge_pair_error();
    let has_same_direction = matches!(err, TopologyError::SameDirectionEdgePair { .. })
        || matches!(&err, TopologyError::Multiple(v) if v.iter().any(
            |e| matches!(e, TopologyError::SameDirectionEdgePair { .. }),
        ));
    let only_loop_mismatch = matches!(err, TopologyError::LoopVertexMismatch { .. });
    assert!(
        has_same_direction,
        "expected SameDirectionEdgePair, got {err}"
    );
    assert!(!only_loop_mismatch, "unexpected pure loop mismatch: {err}");
}

#[test]
fn same_direction_edge_pair_carries_related_ids() {
    let err = same_direction_edge_pair_error();
    let mismatch = find_nested_error(&err, |error| {
        matches!(error, TopologyError::SameDirectionEdgePair { .. })
    })
    .expect("expected SameDirectionEdgePair");
    let TopologyError::SameDirectionEdgePair { related, .. } = mismatch else {
        panic!("expected SameDirectionEdgePair, got {mismatch:?}");
    };
    assert_eq!(
        related,
        &sorted_semantic_ids(vec![sem(3), sem(11), sem(14), sem(16)])
    );
}

#[test]
fn valid_seam_pair_accepted() {
    let mut b = test_builder();
    let v_top = mk_v(&mut b, 0.0, 0.0, 1);
    let v_bottom = mk_v(&mut b, 0.0, 1.0, 2);
    let seam = mk_e(&mut b, v_top, v_bottom, 3);
    let f0 = add_face_with_loop(
        &mut b,
        &[(seam, Orientation::Forward), (seam, Orientation::Reversed)],
        LoopKind::Outer,
        4,
    );
    let s0 = mk_shell(&mut b, ShellKind::Closed, vec![f0], 8);
    let r0 = mk_region(&mut b, s0, vec![], 9);
    mk_body(&mut b, vec![r0], 10);
    let store = b.build().expect("valid seam pair");
    let shell = store.shell(s0).expect("shell");
    assert_eq!(shell.faces(), &[f0]);
}

#[test]
fn same_direction_seam_rejected() {
    let mut b = test_builder();
    let v_top = mk_v(&mut b, 0.0, 0.0, 1);
    let v_bottom = mk_v(&mut b, 0.0, 1.0, 2);
    let seam = mk_e(&mut b, v_top, v_bottom, 3);
    let f0 = add_face_with_loop(
        &mut b,
        &[(seam, Orientation::Forward), (seam, Orientation::Forward)],
        LoopKind::Outer,
        4,
    );
    let s0 = mk_shell(&mut b, ShellKind::Closed, vec![f0], 8);
    let r0 = b
        .add_region(RegionParams {
            outer_shell: s0,
            inner_shells: vec![],
            provenance: prov(9),
        })
        .expect("r");
    b.add_body(BodyParams {
        regions: vec![r0],
        provenance: prov(10),
    })
    .expect("body");
    let err = b.build().expect_err("same-direction seam must fail");
    let has_same_direction = matches!(err, TopologyError::SameDirectionEdgePair { .. })
        || matches!(&err, TopologyError::Multiple(v) if v.iter().any(
            |e| matches!(e, TopologyError::SameDirectionEdgePair { .. }),
        ));
    assert!(
        has_same_direction,
        "expected SameDirectionEdgePair, got {err}"
    );
}

#[test]
fn open_shell_labeled_closed_rejected() {
    // A triangle (all edges have 1 coedge use) labeled Closed → rejected.
    let mut b = test_builder();
    let (f0, _, _, _) = build_triangle_face(&mut b);
    let s0 = b
        .add_shell(ShellParams {
            kind: ShellKind::Closed,
            faces: vec![f0],
            provenance: prov(20),
        })
        .expect("s");
    let r0 = b
        .add_region(RegionParams {
            outer_shell: s0,
            inner_shells: vec![],
            provenance: prov(21),
        })
        .expect("r");
    b.add_body(BodyParams {
        regions: vec![r0],
        provenance: prov(22),
    })
    .expect("body");
    let err = b.build().expect_err("mislabeled Closed must be rejected");
    let has_err = matches!(err, TopologyError::InconsistentShellKind { .. })
        || matches!(&err, TopologyError::Multiple(v) if v.iter().any(|e| matches!(e, TopologyError::InconsistentShellKind { .. })));
    assert!(has_err, "expected InconsistentShellKind, got {err}");
}

// Issue 3: closed shell labeled Open is rejected (all edges have 2 uses but kind=Open)
// Issue 3: closed topology labeled Open is rejected (all edges have 2 uses but kind=Open)
#[test]
fn closed_topology_labeled_open_rejected() {
    // The digon has each edge with 2 uses; labeling it Open should fail.
    let mut b = test_builder();
    let v0 = b
        .add_vertex(VertexParams {
            position: pos(0.0, 0.0, 0.0),
            tolerance: tol(),
            provenance: prov(0),
        })
        .expect("v0");
    let v1 = b
        .add_vertex(VertexParams {
            position: pos(1.0, 0.0, 0.0),
            tolerance: tol(),
            provenance: prov(1),
        })
        .expect("v1");
    let v2 = b
        .add_vertex(VertexParams {
            position: pos(0.5, 1.0, 0.0),
            tolerance: tol(),
            provenance: prov(2),
        })
        .expect("v2");
    let e01 = b
        .add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: v0,
            end_vertex: v1,
            tolerance: tol(),
            provenance: prov(3),
        })
        .expect("e01");
    let e12 = b
        .add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: v1,
            end_vertex: v2,
            tolerance: tol(),
            provenance: prov(4),
        })
        .expect("e12");
    let e20 = b
        .add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: v2,
            end_vertex: v0,
            tolerance: tol(),
            provenance: prov(5),
        })
        .expect("e20");
    let f0 = add_tri_face(
        &mut b,
        [
            (e01, Orientation::Forward),
            (e12, Orientation::Forward),
            (e20, Orientation::Forward),
        ],
        6,
    );
    let f1 = add_tri_face(
        &mut b,
        [
            (e01, Orientation::Reversed),
            (e20, Orientation::Reversed),
            (e12, Orientation::Reversed),
        ],
        9,
    );
    // Mislabel as Open even though all edges have 2 uses (no boundary edges).
    let s0 = b
        .add_shell(ShellParams {
            kind: ShellKind::Open,
            faces: vec![f0, f1],
            provenance: prov(16),
        })
        .expect("s");
    let r0 = b
        .add_region(RegionParams {
            outer_shell: s0,
            inner_shells: vec![],
            provenance: prov(17),
        })
        .expect("r");
    b.add_body(BodyParams {
        regions: vec![r0],
        provenance: prov(18),
    })
    .expect("body");
    let err = b
        .build()
        .expect_err("all-edges-2-uses Open shell must be rejected");
    let has_err = matches!(err, TopologyError::OpenShellHasNoBoundaryEdge { .. })
        || matches!(&err, TopologyError::Multiple(v) if v.iter().any(|e| matches!(e, TopologyError::OpenShellHasNoBoundaryEdge { .. })));
    assert!(has_err, "expected OpenShellHasNoBoundaryEdge, got {err}");
}

// Issue 3: disconnected shell (two triangles not sharing edges)
#[test]
fn disconnected_shell_rejected() {
    // Two triangle faces in the same shell but sharing no edges → disconnected.
    let mut b = test_builder();
    let mk_v = |b: &mut TopologyBuilder, x, y, p| {
        b.add_vertex(VertexParams {
            position: pos(x, y, 0.0),
            tolerance: tol(),
            provenance: prov(p),
        })
        .expect("v")
    };
    let mk_e = |b: &mut TopologyBuilder, sv, ev, p| {
        b.add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: sv,
            end_vertex: ev,
            tolerance: tol(),
            provenance: prov(p),
        })
        .expect("e")
    };
    let v0 = mk_v(&mut b, 0.0, 0.0, 0);
    let v1 = mk_v(&mut b, 1.0, 0.0, 1);
    let v2 = mk_v(&mut b, 0.5, 1.0, 2);
    let v3 = mk_v(&mut b, 5.0, 0.0, 3);
    let v4 = mk_v(&mut b, 6.0, 0.0, 4);
    let v5 = mk_v(&mut b, 5.5, 1.0, 5);
    let ea0 = mk_e(&mut b, v0, v1, 6);
    let ea1 = mk_e(&mut b, v1, v2, 7);
    let ea2 = mk_e(&mut b, v2, v0, 8);
    let eb0 = mk_e(&mut b, v3, v4, 14);
    let eb1 = mk_e(&mut b, v4, v5, 15);
    let eb2 = mk_e(&mut b, v5, v3, 16);
    let fa = add_tri_face(
        &mut b,
        [
            (ea0, Orientation::Forward),
            (ea1, Orientation::Forward),
            (ea2, Orientation::Forward),
        ],
        9,
    );
    let fb = add_tri_face(
        &mut b,
        [
            (eb0, Orientation::Forward),
            (eb1, Orientation::Forward),
            (eb2, Orientation::Forward),
        ],
        17,
    );
    // Both faces in a single Open shell — disconnected
    let s0 = b
        .add_shell(ShellParams {
            kind: ShellKind::Open,
            faces: vec![fa, fb],
            provenance: prov(22),
        })
        .expect("s");
    let r0 = b
        .add_region(RegionParams {
            outer_shell: s0,
            inner_shells: vec![],
            provenance: prov(23),
        })
        .expect("r");
    b.add_body(BodyParams {
        regions: vec![r0],
        provenance: prov(24),
    })
    .expect("body");
    let err = b.build().expect_err("disconnected shell must be rejected");
    let has_err = matches!(err, TopologyError::DisconnectedShell { .. })
        || matches!(&err, TopologyError::Multiple(v) if v.iter().any(|e| matches!(e, TopologyError::DisconnectedShell { .. })));
    assert!(has_err, "expected DisconnectedShell, got {err}");
}

#[test]
fn disconnected_shell_carries_related_id() {
    let mut b = test_builder();
    let va0 = mk_v(&mut b, 0.0, 0.0, 1);
    let va1 = mk_v(&mut b, 1.0, 0.0, 2);
    let va2 = mk_v(&mut b, 0.0, 1.0, 3);
    let ea0 = mk_e(&mut b, va0, va1, 4);
    let ea1 = mk_e(&mut b, va1, va2, 5);
    let ea2 = mk_e(&mut b, va2, va0, 6);
    let fa = add_tri_face(
        &mut b,
        [
            (ea0, Orientation::Forward),
            (ea1, Orientation::Forward),
            (ea2, Orientation::Forward),
        ],
        7,
    );

    let vb0 = mk_v(&mut b, 10.0, 0.0, 11);
    let vb1 = mk_v(&mut b, 11.0, 0.0, 12);
    let vb2 = mk_v(&mut b, 10.0, 1.0, 13);
    let eb0 = mk_e(&mut b, vb0, vb1, 14);
    let eb1 = mk_e(&mut b, vb1, vb2, 15);
    let eb2 = mk_e(&mut b, vb2, vb0, 16);
    let fb = add_tri_face(
        &mut b,
        [
            (eb0, Orientation::Forward),
            (eb1, Orientation::Forward),
            (eb2, Orientation::Forward),
        ],
        17,
    );

    let s0 = mk_shell(&mut b, ShellKind::Open, vec![fa, fb], 22);
    let r0 = mk_region(&mut b, s0, vec![], 23);
    mk_body(&mut b, vec![r0], 24);

    let err = b.build().expect_err("disconnected shell must be rejected");
    let disconnected = find_nested_error(&err, |error| {
        matches!(error, TopologyError::DisconnectedShell { .. })
    })
    .expect("expected DisconnectedShell");
    let TopologyError::DisconnectedShell { related, .. } = disconnected else {
        panic!("expected DisconnectedShell, got {disconnected:?}");
    };
    assert_eq!(related, &vec![sem(22)]);
}

#[test]
fn face_owned_by_multiple_shells_rejected() {
    let mut b = test_builder();
    let (f0, _, _, _) = build_triangle_face(&mut b);
    // Two shells both claiming the same face.
    let s0 = b
        .add_shell(ShellParams {
            kind: ShellKind::Open,
            faces: vec![f0],
            provenance: prov(20),
        })
        .expect("s0");
    let s1 = b
        .add_shell(ShellParams {
            kind: ShellKind::Open,
            faces: vec![f0],
            provenance: prov(21),
        })
        .expect("s1");
    let r0 = b
        .add_region(RegionParams {
            outer_shell: s0,
            inner_shells: vec![],
            provenance: prov(22),
        })
        .expect("r0");
    let r1 = b
        .add_region(RegionParams {
            outer_shell: s1,
            inner_shells: vec![],
            provenance: prov(23),
        })
        .expect("r1");
    b.add_body(BodyParams {
        regions: vec![r0, r1],
        provenance: prov(24),
    })
    .expect("body");
    let err = b
        .build()
        .expect_err("face in multiple shells must be rejected");
    let has_err = matches!(err, TopologyError::FaceOwnershipConflict { .. })
        || matches!(&err, TopologyError::Multiple(v) if v.iter().any(|e| matches!(e, TopologyError::FaceOwnershipConflict { .. })));
    assert!(has_err, "expected FaceOwnershipConflict, got {err}");
}

#[test]
fn multi_owner_face_carries_all_related_ids() {
    let mut b = test_builder();
    let (f0, _, _, _) = build_triangle_face(&mut b);
    let s0 = b
        .add_shell(ShellParams {
            kind: ShellKind::Open,
            faces: vec![f0],
            provenance: prov(20),
        })
        .expect("s0");
    let s1 = b
        .add_shell(ShellParams {
            kind: ShellKind::Open,
            faces: vec![f0],
            provenance: prov(21),
        })
        .expect("s1");
    let r0 = mk_region(&mut b, s0, vec![], 22);
    let r1 = mk_region(&mut b, s1, vec![], 23);
    mk_body(&mut b, vec![r0, r1], 24);

    let err = b
        .build()
        .expect_err("face in multiple shells must be rejected");
    let conflict = find_nested_error(&err, |error| {
        matches!(error, TopologyError::FaceOwnershipConflict { .. })
    })
    .expect("expected FaceOwnershipConflict");
    let TopologyError::FaceOwnershipConflict { related, .. } = conflict else {
        panic!("expected FaceOwnershipConflict, got {conflict:?}");
    };
    assert_eq!(
        related,
        &sorted_semantic_ids(vec![sem(7), sem(20), sem(21)])
    );
}

// Issue 4: shell owned by multiple regions rejected
#[test]
fn shell_owned_by_multiple_regions_rejected() {
    let mut b = test_builder();
    let (f0, _, _, _) = build_triangle_face(&mut b);
    let s0 = b
        .add_shell(ShellParams {
            kind: ShellKind::Open,
            faces: vec![f0],
            provenance: prov(20),
        })
        .expect("s");
    // Two regions both claiming the same shell.
    let r0 = b
        .add_region(RegionParams {
            outer_shell: s0,
            inner_shells: vec![],
            provenance: prov(21),
        })
        .expect("r0");
    let r1 = b
        .add_region(RegionParams {
            outer_shell: s0,
            inner_shells: vec![],
            provenance: prov(22),
        })
        .expect("r1");
    b.add_body(BodyParams {
        regions: vec![r0, r1],
        provenance: prov(23),
    })
    .expect("body");
    let err = b
        .build()
        .expect_err("shell in multiple regions must be rejected");
    let has_err = matches!(err, TopologyError::ShellOwnershipConflict { .. })
        || matches!(&err, TopologyError::Multiple(v) if v.iter().any(|e| matches!(e, TopologyError::ShellOwnershipConflict { .. })));
    assert!(has_err, "expected ShellOwnershipConflict, got {err}");
}

#[test]
fn multi_owner_shell_carries_all_related_ids() {
    let mut b = test_builder();
    let (f0, _, _, _) = build_triangle_face(&mut b);
    let s0 = b
        .add_shell(ShellParams {
            kind: ShellKind::Open,
            faces: vec![f0],
            provenance: prov(20),
        })
        .expect("s");
    let r0 = b
        .add_region(RegionParams {
            outer_shell: s0,
            inner_shells: vec![],
            provenance: prov(21),
        })
        .expect("r0");
    let r1 = b
        .add_region(RegionParams {
            outer_shell: s0,
            inner_shells: vec![],
            provenance: prov(22),
        })
        .expect("r1");
    mk_body(&mut b, vec![r0, r1], 23);

    let err = b
        .build()
        .expect_err("shell in multiple regions must be rejected");
    let conflict = find_nested_error(&err, |error| {
        matches!(error, TopologyError::ShellOwnershipConflict { .. })
    })
    .expect("expected ShellOwnershipConflict");
    let TopologyError::ShellOwnershipConflict { related, .. } = conflict else {
        panic!("expected ShellOwnershipConflict, got {conflict:?}");
    };
    assert_eq!(
        related,
        &sorted_semantic_ids(vec![sem(20), sem(21), sem(22)])
    );
}

// Issue 4: region owned by multiple bodies rejected
#[test]
fn region_owned_by_multiple_bodies_rejected() {
    let mut b = test_builder();
    let (f0, _, _, _) = build_triangle_face(&mut b);
    let s0 = b
        .add_shell(ShellParams {
            kind: ShellKind::Open,
            faces: vec![f0],
            provenance: prov(20),
        })
        .expect("s");
    let r0 = b
        .add_region(RegionParams {
            outer_shell: s0,
            inner_shells: vec![],
            provenance: prov(21),
        })
        .expect("r");
    // Two bodies both claiming the same region.
    let b0 = b
        .add_body(BodyParams {
            regions: vec![r0],
            provenance: prov(22),
        })
        .expect("b0");
    let b1 = b
        .add_body(BodyParams {
            regions: vec![r0],
            provenance: prov(23),
        })
        .expect("b1");
    let _ = (b0, b1);
    let err = b
        .build()
        .expect_err("region in multiple bodies must be rejected");
    let has_err = matches!(err, TopologyError::RegionOwnershipConflict { .. })
        || matches!(&err, TopologyError::Multiple(v) if v.iter().any(|e| matches!(e, TopologyError::RegionOwnershipConflict { .. })));
    assert!(has_err, "expected RegionOwnershipConflict, got {err}");
}

#[test]
fn multi_owner_region_carries_all_related_ids() {
    let mut b = test_builder();
    let (f0, _, _, _) = build_triangle_face(&mut b);
    let s0 = b
        .add_shell(ShellParams {
            kind: ShellKind::Open,
            faces: vec![f0],
            provenance: prov(20),
        })
        .expect("s");
    let r0 = b
        .add_region(RegionParams {
            outer_shell: s0,
            inner_shells: vec![],
            provenance: prov(21),
        })
        .expect("r");
    b.add_body(BodyParams {
        regions: vec![r0],
        provenance: prov(22),
    })
    .expect("b0");
    b.add_body(BodyParams {
        regions: vec![r0],
        provenance: prov(23),
    })
    .expect("b1");

    let err = b
        .build()
        .expect_err("region in multiple bodies must be rejected");
    let conflict = find_nested_error(&err, |error| {
        matches!(error, TopologyError::RegionOwnershipConflict { .. })
    })
    .expect("expected RegionOwnershipConflict");
    let TopologyError::RegionOwnershipConflict { related, .. } = conflict else {
        panic!("expected RegionOwnershipConflict, got {conflict:?}");
    };
    assert_eq!(
        related,
        &sorted_semantic_ids(vec![sem(21), sem(22), sem(23)])
    );
}

// Issue 4: duplicate face ID in a shell rejected
#[test]
fn duplicate_face_id_in_shell_rejected() {
    let mut b = test_builder();
    let (f0, _, _, _) = build_triangle_face(&mut b);
    // Shell lists f0 twice.
    let s0 = b
        .add_shell(ShellParams {
            kind: ShellKind::Open,
            faces: vec![f0, f0],
            provenance: prov(20),
        })
        .expect("s");
    let r0 = b
        .add_region(RegionParams {
            outer_shell: s0,
            inner_shells: vec![],
            provenance: prov(21),
        })
        .expect("r");
    b.add_body(BodyParams {
        regions: vec![r0],
        provenance: prov(22),
    })
    .expect("body");
    let err = b
        .build()
        .expect_err("duplicate face in shell must be rejected");
    let has_err = matches!(
        err,
        TopologyError::DuplicateIdInCollection {
            kind: TopologyKind::Face,
            ..
        }
    ) || matches!(&err, TopologyError::Multiple(v) if v.iter().any(|e|
            matches!(e, TopologyError::DuplicateIdInCollection { kind: TopologyKind::Face, .. })));
    assert!(
        has_err,
        "expected DuplicateIdInCollection for Face, got {err}"
    );
}

// Issue 4: cavity (inner) shell not Closed is rejected
#[test]
fn open_cavity_shell_rejected() {
    let mut b = test_builder();
    let (f0, _, _, _) = build_triangle_face(&mut b);
    // Outer shell (open triangle - allowed)
    let s_outer = b
        .add_shell(ShellParams {
            kind: ShellKind::Open,
            faces: vec![f0],
            provenance: prov(20),
        })
        .expect("outer");
    // Inner shell (open triangle - NOT allowed as cavity)
    let (f1, _, _, _) = build_triangle_face(&mut b);
    let s_inner = b
        .add_shell(ShellParams {
            kind: ShellKind::Open,
            faces: vec![f1],
            provenance: prov(21),
        })
        .expect("inner");
    let r0 = b
        .add_region(RegionParams {
            outer_shell: s_outer,
            inner_shells: vec![s_inner],
            provenance: prov(22),
        })
        .expect("r");
    b.add_body(BodyParams {
        regions: vec![r0],
        provenance: prov(23),
    })
    .expect("body");
    let err = b.build().expect_err("open cavity shell must be rejected");
    let has_err = matches!(err, TopologyError::OuterShellMustBeClosed { .. })
        || matches!(&err, TopologyError::Multiple(v) if v.iter().any(|e| matches!(e, TopologyError::OuterShellMustBeClosed { .. })));
    assert!(has_err, "expected OuterShellMustBeClosed, got {err}");
}

#[test]
fn open_outer_shell_rejected() {
    let mut b = test_builder();
    let (f0, _, _, _) = build_triangle_face(&mut b);
    let s_outer = mk_shell(&mut b, ShellKind::Open, vec![f0], 20);
    let r0 = mk_region(&mut b, s_outer, vec![], 21);
    mk_body(&mut b, vec![r0], 22);
    let err = b.build().expect_err("open outer shell must fail");
    let has_err = matches!(err, TopologyError::OuterShellMustBeClosed { .. })
        || matches!(&err, TopologyError::Multiple(v) if v.iter().any(
            |e| matches!(e, TopologyError::OuterShellMustBeClosed { .. }),
        ));
    assert!(has_err, "expected OuterShellMustBeClosed, got {err}");
}

// Issue 4: orphan vertex (not reachable from any body)
#[test]
fn orphan_vertex_rejected() {
    let mut b = test_builder();
    let (f0, _, _, _) = build_triangle_face(&mut b);
    // Extra vertex not connected to anything.
    b.add_vertex(VertexParams {
        position: pos(99.0, 0.0, 0.0),
        tolerance: tol(),
        provenance: prov(50),
    })
    .expect("orphan vertex");
    let s0 = b
        .add_shell(ShellParams {
            kind: ShellKind::Open,
            faces: vec![f0],
            provenance: prov(20),
        })
        .expect("s");
    let r0 = b
        .add_region(RegionParams {
            outer_shell: s0,
            inner_shells: vec![],
            provenance: prov(21),
        })
        .expect("r");
    b.add_body(BodyParams {
        regions: vec![r0],
        provenance: prov(22),
    })
    .expect("body");
    let err = b.build().expect_err("orphan vertex must be rejected");
    let has_err = matches!(
        err,
        TopologyError::OrphanEntity {
            kind: TopologyKind::Vertex,
            ..
        }
    ) || matches!(&err, TopologyError::Multiple(v) if v.iter().any(|e|
            matches!(e, TopologyError::OrphanEntity { kind: TopologyKind::Vertex, .. })));
    assert!(has_err, "expected OrphanEntity(Vertex), got {err}");
}

// Issue 7: loop_next_coedge_id reports CoedgeNotInLoop when coedge is in a different loop
#[test]
fn loop_next_coedge_not_in_loop_returns_error() {
    let mut b = test_builder();
    let (f0, _, _, _, e0, e1, e2) = build_triangle_face_with_edges(&mut b);
    // Add a second face/loop with its own coedge.
    close_face_as_solid(
        &mut b,
        f0,
        &[
            (e0, Orientation::Forward),
            (e1, Orientation::Forward),
            (e2, Orientation::Forward),
        ],
        30,
    );
    let (f1, _, _, _, e3, e4, e5) = build_triangle_face_with_edges(&mut b);
    close_face_as_solid(
        &mut b,
        f1,
        &[
            (e3, Orientation::Forward),
            (e4, Orientation::Forward),
            (e5, Orientation::Forward),
        ],
        40,
    );
    let store = b.build().expect("two-triangle store");
    // Get the first coedge from loop 1 (second loop)
    let loop0 = store
        .get_loop(store.loops().next().expect("loop0").id())
        .expect("lp0");
    let loop1_opt = store.loops().nth(1);
    if let Some(loop1) = loop1_opt {
        let loop1 = store.get_loop(loop1.id()).expect("lp1");
        let ce_in_loop1 = *loop1.coedges().first().expect("ce");
        // Ask loop0 for a coedge that belongs to loop1 → CoedgeNotInLoop
        let result = loop_next_coedge_id(&store, loop0.id(), ce_in_loop1);
        assert!(
            matches!(result, Err(TopologyError::CoedgeNotInLoop { .. })),
            "expected CoedgeNotInLoop, got {result:?}"
        );
    }
}

// Issue 8: next_generation increments and overflow is detected
#[test]
fn next_generation_increments() {
    let (store, _, _) = build_tetrahedron();
    let next = store
        .next_generation()
        .expect("next generation should succeed");
    assert_eq!(next, store.generation() + 1);
}

#[test]
fn successor_builder_inherits_lineage() {
    let (store, _, _) = build_tetrahedron();
    let successor = store
        .successor_builder(test_snapshot_b())
        .expect("successor builder");
    assert_eq!(successor.lineage(), store.lineage());
    assert_eq!(successor.generation(), store.generation() + 1);
    assert_eq!(successor.snapshot(), test_snapshot_b());
}

#[test]
fn diagnostic_code_stable() {
    let err = TopologyError::MissingEntity {
        kind: TopologyKind::Vertex,
        slot: 3,
        referrer: None,
    };
    assert_eq!(err.code(), "TOPOLOGY.MISSING_ENTITY");
    let diagnostic = err.try_to_diagnostic().expect("not Multiple");
    assert_eq!(diagnostic.code().as_str(), "TOPOLOGY.MISSING_ENTITY");
    assert_eq!(diagnostic.severity(), Severity::Error);
    assert_eq!(diagnostic.path().len(), 1);
}

#[test]
fn try_to_diagnostic_multiple_returns_children() {
    let face_id = FaceId::new(0, 0, test_lineage(), test_snapshot());
    let body_id = BodyId::new(0, 0, test_lineage(), test_snapshot());
    let err1 = TopologyError::MissingOuterLoop {
        face_id,
        related: vec![],
    };
    let err2 = TopologyError::EmptyBody {
        body_id,
        related: vec![],
    };
    let err = TopologyError::Multiple(vec![err1.clone(), err2.clone()]);

    let children = err
        .try_to_diagnostic()
        .expect_err("Multiple should preserve child diagnostics");
    assert_eq!(children, &[err1, err2]);
}

#[test]
fn empty_loop_diagnostic_includes_related_semantic_id() {
    let mut b = test_builder();
    let face = mk_face(&mut b, 1);
    let _loop_id = mk_loop(&mut b, face, LoopKind::Outer, 2);

    let err = b.build().expect_err("empty loop should fail");
    let empty_loop = match &err {
        e @ TopologyError::EmptyLoop { .. } => e,
        TopologyError::Multiple(errs) => errs
            .iter()
            .find(|e| matches!(e, TopologyError::EmptyLoop { .. }))
            .expect("expected EmptyLoop error"),
        other => panic!("expected EmptyLoop, got: {other:?}"),
    };
    let diag = empty_loop.try_to_diagnostic().expect("not Multiple");
    assert_eq!(diag.related(), &[sem(2)]);
}

#[test]
fn outer_shell_must_be_closed_diagnostic_includes_region_and_shell_semantics() {
    let mut b = test_builder();
    let v0 = mk_v(&mut b, 0.0, 0.0, 1);
    let v1 = mk_v(&mut b, 1.0, 0.0, 2);
    let v2 = mk_v(&mut b, 0.0, 1.0, 3);
    let e0 = mk_e(&mut b, v0, v1, 4);
    let e1 = mk_e(&mut b, v1, v2, 5);
    let e2 = mk_e(&mut b, v2, v0, 6);
    let face = add_tri_face(
        &mut b,
        [
            (e0, Orientation::Forward),
            (e1, Orientation::Forward),
            (e2, Orientation::Forward),
        ],
        7,
    );
    let shell = mk_shell(&mut b, ShellKind::Open, vec![face], 20);
    let region = mk_region(&mut b, shell, vec![], 21);
    mk_body(&mut b, vec![region], 22);

    let err = b.build().expect_err("open region shell should fail");
    let outer_shell_err = match &err {
        e @ TopologyError::OuterShellMustBeClosed { .. } => e,
        TopologyError::Multiple(errs) => errs
            .iter()
            .find(|e| matches!(e, TopologyError::OuterShellMustBeClosed { .. }))
            .expect("expected OuterShellMustBeClosed error"),
        other => panic!("expected OuterShellMustBeClosed, got: {other:?}"),
    };
    let diag = outer_shell_err.try_to_diagnostic().expect("not Multiple");
    assert_eq!(diag.related(), &[sem(20), sem(21)]);
}

// Issue 6: euler arithmetic overflow test (pathological case via ArithmeticOverflow)
#[test]
fn body_euler_checked_accumulation() {
    // Normal tetrahedron: no overflow expected.
    let (store, _, body_id) = build_tetrahedron();
    let result = body_total_euler_ve_f(&store, body_id);
    assert!(
        result.is_ok(),
        "tetrahedron euler should not overflow: {result:?}"
    );
}

#[test]
fn reachability_is_linear_in_entity_count() {
    let mut b = test_builder();
    for index in 0_u32..200 {
        let base_x = f64::from(index) * 10.0;
        let v0 = mk_v(&mut b, base_x, 0.0, 1);
        let v1 = mk_v(&mut b, base_x + 1.0, 0.0, 2);
        let v2 = mk_v(&mut b, base_x, 1.0, 3);
        let e0 = mk_e(&mut b, v0, v1, 4);
        let e1 = mk_e(&mut b, v1, v2, 5);
        let e2 = mk_e(&mut b, v2, v0, 6);
        let f0 = add_tri_face(
            &mut b,
            [
                (e0, Orientation::Forward),
                (e1, Orientation::Forward),
                (e2, Orientation::Forward),
            ],
            7,
        );
        close_face_as_solid(
            &mut b,
            f0,
            &[
                (e0, Orientation::Forward),
                (e1, Orientation::Forward),
                (e2, Orientation::Forward),
            ],
            30,
        );
    }
    let store = b.build().expect("large linear build");
    assert_eq!(store.body_count(), 200);
}

#[test]
fn euler_metrics_large_linear() {
    let mut b = test_builder();
    let face_count = 100usize;
    let mut vertices = Vec::with_capacity(face_count);
    let angle_step = std::f64::consts::TAU / 100.0;
    let mut angle: f64 = 0.0;
    for _ in 0..face_count {
        vertices.push(mk_v(&mut b, angle.cos(), angle.sin(), 1));
        angle += angle_step;
    }
    let mut edges = Vec::with_capacity(face_count);
    for index in 0..face_count {
        edges.push(mk_e(
            &mut b,
            vertices[index],
            vertices[(index + 1) % face_count],
            2,
        ));
    }
    let outer_edges: Vec<(EdgeId, Orientation)> = edges
        .iter()
        .copied()
        .map(|edge_id| (edge_id, Orientation::Forward))
        .collect();
    let inner_edges: Vec<(EdgeId, Orientation)> = edges
        .iter()
        .rev()
        .copied()
        .map(|edge_id| (edge_id, Orientation::Reversed))
        .collect();
    let f0 = add_face_with_loop(&mut b, &outer_edges, LoopKind::Outer, 3);
    let f1 = add_face_with_loop(&mut b, &inner_edges, LoopKind::Outer, 6);
    let s0 = mk_shell(&mut b, ShellKind::Closed, vec![f0, f1], 9);
    let r0 = mk_region(&mut b, s0, vec![], 10);
    mk_body(&mut b, vec![r0], 11);
    let store = b.build().expect("large shell");
    let metrics = shell_euler_metrics(&store, s0).expect("metrics");
    assert_eq!(metrics.vertices, face_count as u64);
    assert_eq!(metrics.edges, face_count as u64);
    assert_eq!(metrics.faces, 2);
    assert_eq!(metrics.euler_ve_f, 2);
}

// ── T1.4 Referrer context in diagnostic paths ─────────────────────────────────

/// Helper: create a wrong-lineage `VertexId` (BB…BB lineage, right slot/gen for
/// a freshly created vertex in a `test_lineage` builder).
fn bad_vertex_id(slot: u32) -> VertexId {
    let bad_lineage = TopologyLineageId::new(SemanticId::from_bytes([0xBB; 16]));
    let bad_snapshot = TopologySnapshotId::new(SemanticId::from_bytes([0xBB; 16]));
    VertexId::new(slot, 0, bad_lineage, bad_snapshot)
}

fn bad_edge_id(slot: u32) -> EdgeId {
    let bad_lineage = TopologyLineageId::new(SemanticId::from_bytes([0xBB; 16]));
    let bad_snapshot = TopologySnapshotId::new(SemanticId::from_bytes([0xBB; 16]));
    EdgeId::new(slot, 0, bad_lineage, bad_snapshot)
}

fn bad_loop_id(slot: u32) -> LoopId {
    let bad_lineage = TopologyLineageId::new(SemanticId::from_bytes([0xBB; 16]));
    let bad_snapshot = TopologySnapshotId::new(SemanticId::from_bytes([0xBB; 16]));
    LoopId::new(slot, 0, bad_lineage, bad_snapshot)
}

fn bad_face_id(slot: u32) -> FaceId {
    let bad_lineage = TopologyLineageId::new(SemanticId::from_bytes([0xBB; 16]));
    let bad_snapshot = TopologySnapshotId::new(SemanticId::from_bytes([0xBB; 16]));
    FaceId::new(slot, 0, bad_lineage, bad_snapshot)
}

fn bad_shell_id(slot: u32) -> ShellId {
    let bad_lineage = TopologyLineageId::new(SemanticId::from_bytes([0xBB; 16]));
    let bad_snapshot = TopologySnapshotId::new(SemanticId::from_bytes([0xBB; 16]));
    ShellId::new(slot, 0, bad_lineage, bad_snapshot)
}

fn bad_region_id(slot: u32) -> RegionId {
    let bad_lineage = TopologyLineageId::new(SemanticId::from_bytes([0xBB; 16]));
    let bad_snapshot = TopologySnapshotId::new(SemanticId::from_bytes([0xBB; 16]));
    RegionId::new(slot, 0, bad_lineage, bad_snapshot)
}

/// edge → `start_vertex`: referrer=(Edge, slot, `"start_vertex"`)
#[test]
fn referrer_path_edge_start_vertex() {
    let mut b = test_builder();
    let v1 = mk_v(&mut b, 1.0, 0.0, 1);
    let bad_v = bad_vertex_id(0); // wrong lineage
    let e_bad = b
        .add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: bad_v,
            end_vertex: v1,
            tolerance: tol(),
            provenance: prov(2),
        })
        .expect("edge");
    let _ = e_bad;
    let result = b.build();
    let check = |e: &TopologyError| {
        if let TopologyError::WrongLineage { referrer, .. } = e {
            let ctx = referrer.as_ref().expect("referrer");
            assert_eq!(ctx.kind, TopologyKind::Edge);
            assert_eq!(ctx.field, "start_vertex");
            assert_eq!(ctx.index, None);
            let diag = e.try_to_diagnostic().expect("not Multiple");
            assert_eq!(diag.path().len(), 3, "path: {:?}", diag.path());
            return true;
        }
        false
    };
    match result {
        Err(ref e @ TopologyError::WrongLineage { .. }) => {
            assert!(check(e));
        }
        Err(TopologyError::Multiple(ref errs)) => {
            let found = errs.iter().any(check);
            assert!(found, "no WrongLineage(start_vertex) in: {result:?}");
        }
        other => panic!("expected WrongLineage error, got: {other:?}"),
    }
}

/// body → regions collection: referrer=(Body, slot, "regions", index)
#[test]
fn referrer_path_body_to_region() {
    let mut b = test_builder();
    let bad_r = bad_region_id(0);
    b.add_body(BodyParams {
        regions: vec![bad_r],
        provenance: prov(1),
    })
    .expect("body");
    let result = b.build();
    let check = |e: &TopologyError| {
        if let TopologyError::WrongLineage { referrer, .. } = e {
            let ctx = referrer.as_ref().expect("referrer must be set");
            assert_eq!(ctx.kind, TopologyKind::Body);
            assert_eq!(ctx.field, "regions");
            assert_eq!(ctx.index, Some(0));
            let diag = e.try_to_diagnostic().expect("not Multiple");
            // path: [body, field, index, region] = 4 segments
            assert_eq!(diag.path().len(), 4, "wrong path len: {:?}", diag.path());
            return true;
        }
        false
    };
    match result {
        Err(ref e @ TopologyError::WrongLineage { .. }) => {
            assert!(check(e));
        }
        Err(TopologyError::Multiple(ref errs)) => {
            let found = errs.iter().any(check);
            assert!(found, "no matching WrongLineage in: {result:?}");
        }
        other => panic!("expected WrongLineage error, got: {other:?}"),
    }
}

/// region → `outer_shell`: referrer=(Region, slot, `"outer_shell"`, None)
#[test]
fn referrer_path_region_outer_shell() {
    let mut b = test_builder();
    let bad_s = bad_shell_id(0);
    b.add_region(RegionParams {
        outer_shell: bad_s,
        inner_shells: vec![],
        provenance: prov(1),
    })
    .expect("region");
    let result = b.build();
    let check = |e: &TopologyError| {
        if let TopologyError::WrongLineage { referrer, .. } = e {
            let ctx = referrer.as_ref().expect("referrer");
            assert_eq!(ctx.kind, TopologyKind::Region);
            assert_eq!(ctx.field, "outer_shell");
            assert_eq!(ctx.index, None);
            let diag = e.try_to_diagnostic().expect("not Multiple");
            // path: [region, field, shell] = 3 segments
            assert_eq!(diag.path().len(), 3, "wrong path len: {:?}", diag.path());
            return true;
        }
        false
    };
    match result {
        Err(ref e @ TopologyError::WrongLineage { .. }) => {
            assert!(check(e));
        }
        Err(TopologyError::Multiple(ref errs)) => {
            let found = errs.iter().any(check);
            assert!(found, "no WrongLineage in: {result:?}");
        }
        other => panic!("expected WrongLineage error, got: {other:?}"),
    }
}

/// shell → faces collection: referrer=(Shell, slot, "faces", Some(index))
#[test]
fn referrer_path_shell_to_face() {
    let mut b = test_builder();
    let bad_f = bad_face_id(0);
    b.add_shell(ShellParams {
        kind: ShellKind::Open,
        faces: vec![bad_f],
        provenance: prov(1),
    })
    .expect("shell");
    let result = b.build();
    let check = |e: &TopologyError| {
        if let TopologyError::WrongLineage { referrer, .. } = e {
            let ctx = referrer.as_ref().expect("referrer");
            assert_eq!(ctx.kind, TopologyKind::Shell);
            assert_eq!(ctx.field, "faces");
            assert_eq!(ctx.index, Some(0));
            let diag = e.try_to_diagnostic().expect("not Multiple");
            assert_eq!(diag.path().len(), 4, "wrong path len: {:?}", diag.path());
            return true;
        }
        false
    };
    match result {
        Err(ref e @ TopologyError::WrongLineage { .. }) => {
            assert!(check(e));
        }
        Err(TopologyError::Multiple(ref errs)) => {
            let found = errs.iter().any(check);
            assert!(found, "no WrongLineage in: {result:?}");
        }
        other => panic!("expected WrongLineage error, got: {other:?}"),
    }
}

/// loop → face: referrer=(Loop, slot, "face", None)
#[test]
fn referrer_path_loop_to_face() {
    let mut b = test_builder();
    let bad_f = bad_face_id(0);
    b.add_loop(LoopParams {
        face: bad_f,
        kind: LoopKind::Outer,
        provenance: prov(1),
    })
    .expect("loop");
    let result = b.build();
    let check = |e: &TopologyError| {
        if let TopologyError::WrongLineage { referrer, .. } = e {
            let ctx = referrer.as_ref().expect("referrer");
            assert_eq!(ctx.kind, TopologyKind::Loop);
            assert_eq!(ctx.field, "face");
            assert_eq!(ctx.index, None);
            let diag = e.try_to_diagnostic().expect("not Multiple");
            assert_eq!(diag.path().len(), 3, "path: {:?}", diag.path());
            return true;
        }
        false
    };
    match result {
        Err(ref e @ TopologyError::WrongLineage { .. }) => {
            assert!(check(e));
        }
        Err(TopologyError::Multiple(ref errs)) => {
            let found = errs.iter().any(check);
            assert!(found, "no WrongLineage in: {result:?}");
        }
        other => panic!("expected WrongLineage error, got: {other:?}"),
    }
}

/// coedge → edge: referrer=(Coedge, slot, "edge", None)
#[test]
fn referrer_path_coedge_to_edge() {
    let mut b = test_builder();
    let f = mk_face(&mut b, 1);
    let l = mk_loop(&mut b, f, LoopKind::Outer, 2);
    let bad_e = bad_edge_id(0);
    b.add_coedge(CoedgeParams {
        edge: bad_e,
        loop_id: l,
        orientation: Orientation::Forward,
        pcurve: fake_curve2(),
        provenance: prov(3),
    })
    .expect("coedge");
    let result = b.build();
    let check = |e: &TopologyError| {
        if let TopologyError::WrongLineage { referrer, .. } = e {
            let ctx = referrer.as_ref().expect("referrer");
            assert_eq!(ctx.kind, TopologyKind::Coedge);
            assert_eq!(ctx.field, "edge");
            assert_eq!(ctx.index, None);
            let diag = e.try_to_diagnostic().expect("not Multiple");
            assert_eq!(diag.path().len(), 3, "path: {:?}", diag.path());
            return true;
        }
        false
    };
    match result {
        Err(ref e @ TopologyError::WrongLineage { .. }) => {
            assert!(check(e));
        }
        Err(TopologyError::Multiple(ref errs)) => {
            let found = errs.iter().any(check);
            assert!(found, "no WrongLineage in: {result:?}");
        }
        other => panic!("expected WrongLineage error, got: {other:?}"),
    }
}

/// coedge → `loop_id`: referrer=(Coedge, slot, `"loop_id"`, None)
#[test]
fn referrer_path_coedge_to_loop() {
    let mut b = test_builder();
    let v0 = mk_v(&mut b, 0.0, 0.0, 1);
    let v1 = mk_v(&mut b, 1.0, 0.0, 2);
    let e = mk_e(&mut b, v0, v1, 3);
    let bad_l = bad_loop_id(0);
    b.add_coedge(CoedgeParams {
        edge: e,
        loop_id: bad_l,
        orientation: Orientation::Forward,
        pcurve: fake_curve2(),
        provenance: prov(4),
    })
    .expect("coedge");
    let result = b.build();
    let check = |err: &TopologyError| {
        if let TopologyError::WrongLineage { referrer, .. } = err {
            let ctx = referrer.as_ref().expect("referrer");
            assert_eq!(ctx.kind, TopologyKind::Coedge);
            assert_eq!(ctx.field, "loop_id");
            assert_eq!(ctx.index, None);
            let diag = err.try_to_diagnostic().expect("not Multiple");
            assert_eq!(diag.path().len(), 3, "path: {:?}", diag.path());
            return true;
        }
        false
    };
    match result {
        Err(ref e @ TopologyError::WrongLineage { .. }) => {
            assert!(check(e));
        }
        Err(TopologyError::Multiple(ref errs)) => {
            let found = errs.iter().any(check);
            assert!(found, "no WrongLineage in: {result:?}");
        }
        other => panic!("expected WrongLineage error, got: {other:?}"),
    }
}

/// edge → `end_vertex`: referrer=(Edge, slot, `"end_vertex"`, None)
#[test]
fn referrer_path_edge_end_vertex() {
    let mut b = test_builder();
    let v0 = mk_v(&mut b, 0.0, 0.0, 1);
    let bad_v = bad_vertex_id(99); // out-of-range slot, same lineage would give MissingEntity
    // Use wrong lineage so WrongLineage fires
    let e = b
        .add_edge(EdgeParams {
            curve: fake_curve3(),
            parameter_interval: interval(0.0, 1.0),
            start_vertex: v0,
            end_vertex: bad_v,
            tolerance: tol(),
            provenance: prov(2),
        })
        .expect("edge");
    let f = mk_face(&mut b, 3);
    let l = mk_loop(&mut b, f, LoopKind::Outer, 4);
    mk_coedge(&mut b, e, l, Orientation::Forward, 5);
    let result = b.build();
    let check = |err: &TopologyError| {
        if let TopologyError::WrongLineage { referrer, .. } = err {
            let ctx = referrer.as_ref().expect("referrer");
            assert_eq!(ctx.kind, TopologyKind::Edge);
            assert_eq!(ctx.field, "end_vertex");
            let diag = err.try_to_diagnostic().expect("not Multiple");
            assert!(diag.path().len() >= 3, "path too short: {:?}", diag.path());
            return true;
        }
        false
    };
    match result {
        Err(ref e @ TopologyError::WrongLineage { .. }) => {
            assert!(check(e));
        }
        Err(TopologyError::Multiple(ref errs)) => {
            let found = errs.iter().any(check);
            assert!(found, "no WrongLineage in: {result:?}");
        }
        other => panic!("expected WrongLineage error, got: {other:?}"),
    }
}

/// Two stores built with distinct lineages: a handle from one is rejected by
/// the other even when slot and generation match.
#[test]
fn two_independent_lineages_reject_cross_store_handle() {
    let lineage_a = TopologyLineageId::new(SemanticId::from_bytes([0xAA; 16]));
    let lineage_b = TopologyLineageId::new(SemanticId::from_bytes([0xBB; 16]));
    let snap_a = TopologySnapshotId::new(SemanticId::from_bytes([0xA1; 16]));
    let snap_b = TopologySnapshotId::new(SemanticId::from_bytes([0xB1; 16]));

    // Build store A
    let (_store_a, v_from_a) = {
        let mut b = TopologyBuilder::with_lineage_and_snapshot(lineage_a, snap_a);
        let v0 = mk_v(&mut b, 0.0, 0.0, 1);
        let v1 = mk_v(&mut b, 1.0, 0.0, 2);
        let v2 = mk_v(&mut b, 0.0, 1.0, 3);
        let e0 = mk_e(&mut b, v0, v1, 4);
        let e1 = mk_e(&mut b, v1, v2, 5);
        let e2 = mk_e(&mut b, v2, v0, 6);
        let f0 = add_tri_face(
            &mut b,
            [
                (e0, Orientation::Forward),
                (e1, Orientation::Forward),
                (e2, Orientation::Forward),
            ],
            7,
        );
        close_face_as_solid(
            &mut b,
            f0,
            &[
                (e0, Orientation::Forward),
                (e1, Orientation::Forward),
                (e2, Orientation::Forward),
            ],
            30,
        );
        let st = b.build().expect("store_a");
        (st, v0)
    };

    // Build store B with a distinct lineage
    let store_b = {
        let mut b = TopologyBuilder::with_lineage_and_snapshot(lineage_b, snap_b);
        let v0 = mk_v(&mut b, 0.0, 0.0, 1);
        let v1 = mk_v(&mut b, 1.0, 0.0, 2);
        let v2 = mk_v(&mut b, 0.0, 1.0, 3);
        let e0 = mk_e(&mut b, v0, v1, 4);
        let e1 = mk_e(&mut b, v1, v2, 5);
        let e2 = mk_e(&mut b, v2, v0, 6);
        let f0 = add_tri_face(
            &mut b,
            [
                (e0, Orientation::Forward),
                (e1, Orientation::Forward),
                (e2, Orientation::Forward),
            ],
            7,
        );
        close_face_as_solid(
            &mut b,
            f0,
            &[
                (e0, Orientation::Forward),
                (e1, Orientation::Forward),
                (e2, Orientation::Forward),
            ],
            30,
        );
        b.build().expect("store_b")
    };

    // Handle from store_a (lineage AA) used against store_b (lineage BB)
    let result = store_b.vertex(v_from_a);
    assert!(
        matches!(result, Err(TopologyError::WrongLineage { .. })),
        "expected WrongLineage, got: {result:?}"
    );
}

// ── Fifth commit: new regression tests ───────────────────────────────────────

/// Two branches from the same store (same lineage, same generation + 1) have
/// different snapshot IDs. Handles from branch-A reject at store-B with
/// `WrongSnapshot` and vice versa.
#[test]
fn wrong_snapshot_two_branches_rejected() {
    let (root_store, _, _) = build_tetrahedron();

    // Branch A: snapshot_b
    let snap_a = TopologySnapshotId::new(SemanticId::from_bytes([0xA2; 16]));
    let snap_b = TopologySnapshotId::new(SemanticId::from_bytes([0xB2; 16]));

    let store_branch_a = {
        let mut b = root_store
            .successor_builder(snap_a)
            .expect("branch A builder");
        let (f, _, _, _, e0, e1, e2) = build_triangle_face_with_edges(&mut b);
        close_face_as_solid(
            &mut b,
            f,
            &[
                (e0, Orientation::Forward),
                (e1, Orientation::Forward),
                (e2, Orientation::Forward),
            ],
            30,
        );
        b.build().expect("branch A")
    };

    let store_branch_b = {
        let mut b = root_store
            .successor_builder(snap_b)
            .expect("branch B builder");
        let (f, _, _, _, e0, e1, e2) = build_triangle_face_with_edges(&mut b);
        close_face_as_solid(
            &mut b,
            f,
            &[
                (e0, Orientation::Forward),
                (e1, Orientation::Forward),
                (e2, Orientation::Forward),
            ],
            30,
        );
        b.build().expect("branch B")
    };

    // Handle from branch_a used against store_branch_b → WrongSnapshot
    let v_from_a = store_branch_a.vertices().next().expect("vertex").id();
    let result = store_branch_b.vertex(v_from_a);
    assert!(
        matches!(result, Err(TopologyError::WrongSnapshot { .. })),
        "expected WrongSnapshot for cross-branch handle, got: {result:?}"
    );

    // Handle from branch_b used against store_branch_a → WrongSnapshot
    let v_from_b = store_branch_b.vertices().next().expect("vertex").id();
    let result2 = store_branch_a.vertex(v_from_b);
    assert!(
        matches!(result2, Err(TopologyError::WrongSnapshot { .. })),
        "expected WrongSnapshot for reverse cross-branch handle, got: {result2:?}"
    );
}

/// `loop_next_coedge_id` must validate coedge existence before membership.
/// A completely out-of-range slot must return `MissingEntity`, not
/// `CoedgeNotInLoop`.
#[test]
fn loop_next_coedge_id_validates_coedge_slot_before_membership() {
    let (store, _, _) = build_tetrahedron();
    // Get any valid loop.
    let loop_id = store.loops().next().expect("loop").id();
    // Coedge slot 99 does not exist.
    let nonexistent_coedge = CoedgeId::new(99, 0, test_lineage(), test_snapshot());
    let result = loop_next_coedge_id(&store, loop_id, nonexistent_coedge);
    assert!(
        matches!(
            result,
            Err(TopologyError::MissingEntity {
                kind: TopologyKind::Coedge,
                ..
            })
        ),
        "expected MissingEntity for nonexistent coedge slot, got: {result:?}"
    );
}

/// `loop_next_coedge_id` with a wrong-lineage coedge must return `WrongLineage`,
/// not `CoedgeNotInLoop`.
#[test]
fn loop_next_coedge_id_wrong_lineage_rejected() {
    let (store, _, _) = build_tetrahedron();
    let loop_id = store.loops().next().expect("loop").id();
    let wrong_lineage = TopologyLineageId::new(SemanticId::from_bytes([0xFF; 16]));
    let wrong_snapshot = TopologySnapshotId::new(SemanticId::from_bytes([0xFF; 16]));
    let bad_coedge = CoedgeId::new(0, 0, wrong_lineage, wrong_snapshot);
    let result = loop_next_coedge_id(&store, loop_id, bad_coedge);
    assert!(
        matches!(result, Err(TopologyError::WrongLineage { .. })),
        "expected WrongLineage for wrong-lineage coedge, got: {result:?}"
    );
}

/// `loop_next_coedge_id` with a wrong-snapshot coedge must return `WrongSnapshot`.
#[test]
fn loop_next_coedge_id_wrong_snapshot_rejected() {
    let (store, _, _) = build_tetrahedron();
    let loop_id = store.loops().next().expect("loop").id();
    // Same lineage as the store, but different snapshot.
    let wrong_snapshot = TopologySnapshotId::new(SemanticId::from_bytes([0xFF; 16]));
    let bad_coedge = CoedgeId::new(0, 0, test_lineage(), wrong_snapshot);
    let result = loop_next_coedge_id(&store, loop_id, bad_coedge);
    assert!(
        matches!(result, Err(TopologyError::WrongSnapshot { .. })),
        "expected WrongSnapshot for wrong-snapshot coedge, got: {result:?}"
    );
}

/// `to_diagnostics()` flattens `Multiple` errors recursively in deterministic order.
#[test]
fn multi_error_diagnostics_flattened_in_order() {
    let face_id = FaceId::new(0, 0, test_lineage(), test_snapshot());
    let err1 = TopologyError::MissingOuterLoop {
        face_id,
        related: vec![],
    };
    let err2 = TopologyError::EmptyShell {
        shell_id: ShellId::new(0, 0, test_lineage(), test_snapshot()),
        related: vec![],
    };
    let err3 = TopologyError::MissingEntity {
        kind: TopologyKind::Vertex,
        slot: 5,
        referrer: None,
    };
    // Nested: err1 + err2 in inner; outer wraps inner + err3
    let inner = TopologyError::Multiple(vec![err1, err2]);
    let outer = TopologyError::Multiple(vec![inner, err3]);
    let diags = outer.to_diagnostics();
    assert_eq!(diags.len(), 3, "should flatten to 3 diagnostics");
    assert_eq!(diags[0].code().as_str(), "TOPOLOGY.MISSING_OUTER_LOOP");
    assert_eq!(diags[1].code().as_str(), "TOPOLOGY.EMPTY_SHELL");
    assert_eq!(diags[2].code().as_str(), "TOPOLOGY.MISSING_ENTITY");
}
