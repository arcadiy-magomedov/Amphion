//! Unit tests requiring crate-internal access.

#[cfg(test)]
mod unit_tests {
    use amphion_foundation::{LengthTolerance, OperationId, Point3, SemanticId};
    use amphion_geometry::{Curve2Id, Curve3Id, ParameterInterval, SurfaceId};

    use crate::builder::{
        BodyParams, CoedgeParams, EdgeParams, FaceParams, LoopParams, RegionParams, ShellParams,
        TopologyBuilder, VertexParams, take_connectivity_visit_count, take_incidence_visit_count,
        take_reachability_visit_count,
    };
    use crate::error::TopologyError;
    use crate::id::{BodyId, EdgeId, FaceId, TopologyLineageId, TopologySnapshotId, VertexId};
    use crate::orientation::{LoopKind, Orientation, ShellKind};
    use crate::provenance::{Provenance, ProvenanceRole};
    use crate::traversal::face_adjacent_face_ids;

    fn test_lineage() -> TopologyLineageId {
        TopologyLineageId::new(SemanticId::from_bytes([0xAA; 16]))
    }

    fn test_snapshot() -> TopologySnapshotId {
        TopologySnapshotId::new(SemanticId::from_bytes([0xCC; 16]))
    }

    fn test_builder_gen(generation: u32) -> TopologyBuilder {
        TopologyBuilder::_with_generation_for_testing(test_lineage(), test_snapshot(), generation)
    }

    fn prov(n: u8) -> Provenance {
        Provenance::new(
            SemanticId::from_bytes([n; 16]),
            Some(OperationId::from_bytes([n; 16])),
            vec![],
            ProvenanceRole::try_new("unit-test").expect("valid provenance role"),
        )
    }

    fn pos(x: f64, y: f64, z: f64) -> Point3 {
        Point3::try_new(x, y, z).expect("finite point")
    }

    fn tol() -> LengthTolerance {
        LengthTolerance::try_new(1.0e-9).expect("positive tolerance")
    }

    fn interval(start: f64, end: f64) -> ParameterInterval {
        ParameterInterval::try_new(start, end).expect("valid interval")
    }

    fn fake_curve3() -> Curve3Id {
        Curve3Id::new(0xAB, 0)
    }

    fn fake_curve2() -> Curve2Id {
        Curve2Id::new(0xCD, 0)
    }

    fn fake_surface() -> SurfaceId {
        SurfaceId::new(0xEF, 0)
    }

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

    fn build_triangle_body(b: &mut TopologyBuilder, base_prov: u8) -> BodyId {
        let v0 = mk_v(b, 0.0, 0.0, base_prov);
        let v1 = mk_v(b, 1.0, 0.0, base_prov.wrapping_add(1));
        let v2 = mk_v(b, 0.0, 1.0, base_prov.wrapping_add(2));
        let e0 = mk_e(b, v0, v1, base_prov.wrapping_add(3));
        let e1 = mk_e(b, v1, v2, base_prov.wrapping_add(4));
        let e2 = mk_e(b, v2, v0, base_prov.wrapping_add(5));

        let face = b
            .add_face(FaceParams {
                surface: fake_surface(),
                orientation: Orientation::Forward,
                provenance: prov(base_prov.wrapping_add(6)),
            })
            .expect("face");
        let outer_loop = b
            .add_loop(LoopParams {
                face,
                kind: LoopKind::Outer,
                provenance: prov(base_prov.wrapping_add(7)),
            })
            .expect("outer loop");
        for (edge, orientation, offset) in [
            (e0, Orientation::Forward, 8_u8),
            (e1, Orientation::Forward, 9_u8),
            (e2, Orientation::Forward, 10_u8),
        ] {
            b.add_coedge(CoedgeParams {
                edge,
                loop_id: outer_loop,
                orientation,
                pcurve: fake_curve2(),
                provenance: prov(base_prov.wrapping_add(offset)),
            })
            .expect("outer coedge");
        }

        let mirror_face = b
            .add_face(FaceParams {
                surface: fake_surface(),
                orientation: Orientation::Forward,
                provenance: prov(base_prov.wrapping_add(11)),
            })
            .expect("mirror face");
        let mirror_loop = b
            .add_loop(LoopParams {
                face: mirror_face,
                kind: LoopKind::Outer,
                provenance: prov(base_prov.wrapping_add(12)),
            })
            .expect("mirror loop");
        for (edge, orientation, offset) in [
            (e2, Orientation::Reversed, 13_u8),
            (e1, Orientation::Reversed, 14_u8),
            (e0, Orientation::Reversed, 15_u8),
        ] {
            b.add_coedge(CoedgeParams {
                edge,
                loop_id: mirror_loop,
                orientation,
                pcurve: fake_curve2(),
                provenance: prov(base_prov.wrapping_add(offset)),
            })
            .expect("mirror coedge");
        }

        let shell = b
            .add_shell(ShellParams {
                kind: ShellKind::Closed,
                faces: vec![face, mirror_face],
                provenance: prov(base_prov.wrapping_add(16)),
            })
            .expect("shell");
        let region = b
            .add_region(RegionParams {
                outer_shell: shell,
                inner_shells: vec![],
                provenance: prov(base_prov.wrapping_add(17)),
            })
            .expect("region");
        b.add_body(BodyParams {
            regions: vec![region],
            provenance: prov(base_prov.wrapping_add(18)),
        })
        .expect("body")
    }

    fn next_prov(next: &mut u8) -> Provenance {
        let value = *next;
        *next = next.wrapping_add(1);
        prov(value)
    }

    #[allow(clippy::too_many_lines)]
    fn build_pyramid(side_count: usize) -> (crate::store::TopologyStore, FaceId) {
        assert!(side_count >= 3);
        let mut b = TopologyBuilder::with_lineage_and_snapshot(test_lineage(), test_snapshot());
        let mut next = 1_u8;
        let apex = b
            .add_vertex(VertexParams {
                position: pos(0.0, 1.0, 1.0),
                tolerance: tol(),
                provenance: next_prov(&mut next),
            })
            .expect("apex");
        let base_vertices: Vec<_> = (0..side_count)
            .map(|index| {
                let coordinate = f64::from(u32::try_from(index).expect("test index fits u32"));
                b.add_vertex(VertexParams {
                    position: pos(coordinate, 0.0, 0.0),
                    tolerance: tol(),
                    provenance: next_prov(&mut next),
                })
                .expect("base vertex")
            })
            .collect();
        let base_edges: Vec<_> = (0..side_count)
            .map(|index| {
                b.add_edge(EdgeParams {
                    curve: fake_curve3(),
                    parameter_interval: interval(0.0, 1.0),
                    start_vertex: base_vertices[index],
                    end_vertex: base_vertices[(index + 1) % side_count],
                    tolerance: tol(),
                    provenance: next_prov(&mut next),
                })
                .expect("base edge")
            })
            .collect();
        let spokes: Vec<_> = base_vertices
            .iter()
            .map(|&vertex| {
                b.add_edge(EdgeParams {
                    curve: fake_curve3(),
                    parameter_interval: interval(0.0, 1.0),
                    start_vertex: apex,
                    end_vertex: vertex,
                    tolerance: tol(),
                    provenance: next_prov(&mut next),
                })
                .expect("spoke")
            })
            .collect();

        let base_face = b
            .add_face(FaceParams {
                surface: fake_surface(),
                orientation: Orientation::Forward,
                provenance: next_prov(&mut next),
            })
            .expect("base face");
        let base_loop = b
            .add_loop(LoopParams {
                face: base_face,
                kind: LoopKind::Outer,
                provenance: next_prov(&mut next),
            })
            .expect("base loop");
        for index in (0..side_count).rev() {
            b.add_coedge(CoedgeParams {
                edge: base_edges[index],
                loop_id: base_loop,
                orientation: Orientation::Reversed,
                pcurve: fake_curve2(),
                provenance: next_prov(&mut next),
            })
            .expect("base coedge");
        }

        let mut faces = Vec::with_capacity(side_count + 1);
        faces.push(base_face);
        for index in 0..side_count {
            let face = b
                .add_face(FaceParams {
                    surface: fake_surface(),
                    orientation: Orientation::Forward,
                    provenance: next_prov(&mut next),
                })
                .expect("side face");
            let loop_id = b
                .add_loop(LoopParams {
                    face,
                    kind: LoopKind::Outer,
                    provenance: next_prov(&mut next),
                })
                .expect("side loop");
            for (edge, orientation) in [
                (spokes[index], Orientation::Forward),
                (base_edges[index], Orientation::Forward),
                (spokes[(index + 1) % side_count], Orientation::Reversed),
            ] {
                b.add_coedge(CoedgeParams {
                    edge,
                    loop_id,
                    orientation,
                    pcurve: fake_curve2(),
                    provenance: next_prov(&mut next),
                })
                .expect("side coedge");
            }
            faces.push(face);
        }

        let shell = b
            .add_shell(ShellParams {
                kind: ShellKind::Closed,
                faces,
                provenance: next_prov(&mut next),
            })
            .expect("pyramid shell");
        let region = b
            .add_region(RegionParams {
                outer_shell: shell,
                inner_shells: vec![],
                provenance: next_prov(&mut next),
            })
            .expect("pyramid region");
        b.add_body(BodyParams {
            regions: vec![region],
            provenance: next_prov(&mut next),
        })
        .expect("pyramid body");

        (b.build().expect("valid pyramid topology"), base_face)
    }

    fn shared_edge_incidence_visits(shell_count: usize) -> usize {
        take_incidence_visit_count();
        let mut b = TopologyBuilder::with_lineage_and_snapshot(test_lineage(), test_snapshot());
        let mut next = 1_u8;
        let v0 = mk_v(&mut b, 0.0, 0.0, next);
        next = next.wrapping_add(1);
        let v1 = mk_v(&mut b, 1.0, 0.0, next);
        next = next.wrapping_add(1);
        let edge = mk_e(&mut b, v0, v1, next);
        next = next.wrapping_add(1);

        for _ in 0..shell_count {
            let mut faces = Vec::with_capacity(2);
            for orientation in [Orientation::Forward, Orientation::Reversed] {
                let face = b
                    .add_face(FaceParams {
                        surface: fake_surface(),
                        orientation: Orientation::Forward,
                        provenance: next_prov(&mut next),
                    })
                    .expect("face");
                let loop_id = b
                    .add_loop(LoopParams {
                        face,
                        kind: LoopKind::Outer,
                        provenance: next_prov(&mut next),
                    })
                    .expect("loop");
                b.add_coedge(CoedgeParams {
                    edge,
                    loop_id,
                    orientation,
                    pcurve: fake_curve2(),
                    provenance: next_prov(&mut next),
                })
                .expect("coedge");
                faces.push(face);
            }
            let shell = b
                .add_shell(ShellParams {
                    kind: ShellKind::Closed,
                    faces,
                    provenance: next_prov(&mut next),
                })
                .expect("shell");
            let region = b
                .add_region(RegionParams {
                    outer_shell: shell,
                    inner_shells: vec![],
                    provenance: next_prov(&mut next),
                })
                .expect("region");
            b.add_body(BodyParams {
                regions: vec![region],
                provenance: next_prov(&mut next),
            })
            .expect("body");
        }

        b.build().expect_err("cross-shell edge must fail");
        take_incidence_visit_count()
    }

    fn repeated_large_region_reachability_visits(size: usize) -> usize {
        take_reachability_visit_count();
        let mut b = TopologyBuilder::with_lineage_and_snapshot(test_lineage(), test_snapshot());
        let mut next = 1_u8;
        let mut shells = Vec::with_capacity(size);
        for _ in 0..size {
            let face = b
                .add_face(FaceParams {
                    surface: fake_surface(),
                    orientation: Orientation::Forward,
                    provenance: next_prov(&mut next),
                })
                .expect("face");
            shells.push(
                b.add_shell(ShellParams {
                    kind: ShellKind::Closed,
                    faces: vec![face],
                    provenance: next_prov(&mut next),
                })
                .expect("shell"),
            );
        }
        let region = b
            .add_region(RegionParams {
                outer_shell: shells[0],
                inner_shells: shells[1..].to_vec(),
                provenance: next_prov(&mut next),
            })
            .expect("region");
        b.add_body(BodyParams {
            regions: vec![region; size],
            provenance: next_prov(&mut next),
        })
        .expect("body");

        b.build()
            .expect_err("duplicate region membership must fail");
        take_reachability_visit_count()
    }

    #[allow(clippy::too_many_lines)]
    fn high_degree_face_reuse_visits(size: usize) -> (usize, usize) {
        take_connectivity_visit_count();
        take_reachability_visit_count();
        let mut b = TopologyBuilder::with_lineage_and_snapshot(test_lineage(), test_snapshot());
        let mut next = 1_u8;
        let vertices: Vec<_> = (0..size)
            .map(|index| {
                let coordinate = f64::from(u32::try_from(index).expect("test index fits u32"));
                b.add_vertex(VertexParams {
                    position: pos(coordinate, 0.0, 0.0),
                    tolerance: tol(),
                    provenance: next_prov(&mut next),
                })
                .expect("vertex")
            })
            .collect();
        let edges: Vec<_> = (0..size)
            .map(|index| {
                b.add_edge(EdgeParams {
                    curve: fake_curve3(),
                    parameter_interval: interval(0.0, 1.0),
                    start_vertex: vertices[index],
                    end_vertex: vertices[(index + 1) % size],
                    tolerance: tol(),
                    provenance: next_prov(&mut next),
                })
                .expect("edge")
            })
            .collect();
        let hub = b
            .add_face(FaceParams {
                surface: fake_surface(),
                orientation: Orientation::Forward,
                provenance: next_prov(&mut next),
            })
            .expect("hub face");
        let hub_loop = b
            .add_loop(LoopParams {
                face: hub,
                kind: LoopKind::Outer,
                provenance: next_prov(&mut next),
            })
            .expect("hub loop");
        for &edge in &edges {
            b.add_coedge(CoedgeParams {
                edge,
                loop_id: hub_loop,
                orientation: Orientation::Forward,
                pcurve: fake_curve2(),
                provenance: next_prov(&mut next),
            })
            .expect("hub coedge");
        }

        let mut first_shell_faces = Vec::with_capacity(size + 1);
        first_shell_faces.push(hub);
        for &edge in &edges {
            let neighbor = b
                .add_face(FaceParams {
                    surface: fake_surface(),
                    orientation: Orientation::Forward,
                    provenance: next_prov(&mut next),
                })
                .expect("neighbor face");
            let neighbor_loop = b
                .add_loop(LoopParams {
                    face: neighbor,
                    kind: LoopKind::Outer,
                    provenance: next_prov(&mut next),
                })
                .expect("neighbor loop");
            b.add_coedge(CoedgeParams {
                edge,
                loop_id: neighbor_loop,
                orientation: Orientation::Reversed,
                pcurve: fake_curve2(),
                provenance: next_prov(&mut next),
            })
            .expect("neighbor coedge");
            first_shell_faces.push(neighbor);
        }
        let first_shell = b
            .add_shell(ShellParams {
                kind: ShellKind::Closed,
                faces: first_shell_faces,
                provenance: next_prov(&mut next),
            })
            .expect("first shell");
        let first_region = b
            .add_region(RegionParams {
                outer_shell: first_shell,
                inner_shells: vec![],
                provenance: next_prov(&mut next),
            })
            .expect("first region");
        b.add_body(BodyParams {
            regions: vec![first_region],
            provenance: next_prov(&mut next),
        })
        .expect("first body");

        for _ in 0..size {
            let other = b
                .add_face(FaceParams {
                    surface: fake_surface(),
                    orientation: Orientation::Forward,
                    provenance: next_prov(&mut next),
                })
                .expect("other face");
            let shell = b
                .add_shell(ShellParams {
                    kind: ShellKind::Closed,
                    faces: vec![hub, other],
                    provenance: next_prov(&mut next),
                })
                .expect("duplicate-owner shell");
            let region = b
                .add_region(RegionParams {
                    outer_shell: shell,
                    inner_shells: vec![],
                    provenance: next_prov(&mut next),
                })
                .expect("duplicate-owner region");
            b.add_body(BodyParams {
                regions: vec![region],
                provenance: next_prov(&mut next),
            })
            .expect("duplicate-owner body");
        }

        b.build().expect_err("multiply owned hub face must fail");
        (
            take_connectivity_visit_count(),
            take_reachability_visit_count(),
        )
    }

    #[test]
    fn generation_is_propagated_to_all_ids() {
        let generation = 42_u32;
        let mut b = test_builder_gen(generation);
        let v0 = mk_v(&mut b, 0.0, 0.0, 1);
        let v1 = mk_v(&mut b, 1.0, 0.0, 2);
        let edge = mk_e(&mut b, v0, v1, 3);

        assert_eq!(v0.handle().generation(), generation, "vertex generation");
        assert_eq!(v1.handle().generation(), generation, "vertex generation");
        assert_eq!(edge.handle().generation(), generation, "edge generation");
        assert_eq!(b.generation(), generation, "builder generation");
    }

    #[test]
    fn next_generation_u32max_overflows() {
        let mut b = test_builder_gen(u32::MAX);
        build_triangle_body(&mut b, 1);
        let store = b.build().expect("u32::MAX generation store");
        let err = store.next_generation().expect_err("overflow should fail");
        assert!(matches!(err, TopologyError::GenerationOverflow));
    }

    #[test]
    fn connectivity_visits_scale_linearly() {
        fn build_n_triangles(n: u32) -> usize {
            take_connectivity_visit_count();
            let mut b = TopologyBuilder::with_lineage_and_snapshot(test_lineage(), test_snapshot());
            for i in 0..n {
                build_triangle_body(&mut b, (i % 200) as u8);
            }
            b.build().expect("valid n-triangle store");
            take_connectivity_visit_count()
        }

        let count_n = build_n_triangles(50);
        let double_count = build_n_triangles(100);
        assert!(count_n > 0, "counter must be non-zero");
        assert!(
            double_count <= count_n * 3,
            "connectivity visits not linear: count(50)={count_n}, count(100)={double_count}"
        );
        assert!(
            double_count >= count_n,
            "2N count should be >= N count: {double_count} < {count_n}"
        );
    }

    #[test]
    fn reachability_visits_scale_linearly() {
        fn build_n(n: u32) -> usize {
            take_reachability_visit_count();
            let mut b = TopologyBuilder::with_lineage_and_snapshot(test_lineage(), test_snapshot());
            for i in 0..n {
                build_triangle_body(&mut b, (i % 200) as u8);
            }
            b.build().expect("valid store");
            take_reachability_visit_count()
        }

        let count_n = build_n(50);
        let double_count = build_n(100);
        println!("reachability count(50)={count_n}, count(100)={double_count}");
        assert!(count_n > 0, "counter must be non-zero");
        assert!(
            double_count <= count_n * 3,
            "reachability visits not linear: count(50)={count_n}, count(100)={double_count}"
        );
        assert!(double_count >= count_n, "2N should be >= N");
    }

    #[test]
    fn incidence_visits_scale_linearly() {
        fn build_n(n: u32) -> usize {
            take_incidence_visit_count();
            let mut b = TopologyBuilder::with_lineage_and_snapshot(test_lineage(), test_snapshot());
            for i in 0..n {
                build_triangle_body(&mut b, (i % 200) as u8);
            }
            b.build().expect("valid store");
            take_incidence_visit_count()
        }

        let count_n = build_n(50);
        let double_count = build_n(100);
        println!("incidence count(50)={count_n}, count(100)={double_count}");
        assert!(count_n > 0, "counter must be non-zero");
        assert!(
            double_count <= count_n * 3,
            "incidence visits not linear: count(50)={count_n}, count(100)={double_count}"
        );
        assert!(double_count >= count_n, "2N should be >= N");
    }

    #[test]
    fn adversarial_duplicate_parent_exercises_early_exit() {
        take_reachability_visit_count();
        let n = 20_u32;
        let mut b = TopologyBuilder::with_lineage_and_snapshot(test_lineage(), test_snapshot());
        for i in 0..n {
            build_triangle_body(&mut b, (i % 200) as u8);
        }
        b.build().expect("valid store");
        let count = take_reachability_visit_count();
        println!("reachability count({n})={count}");
        let max_expected = n as usize * 100;
        assert!(
            count <= max_expected,
            "reachability count {count} exceeds expected linear bound {max_expected}"
        );
    }

    #[test]
    fn region_listed_n_times_in_body_linear_reachability() {
        take_reachability_visit_count();
        let n = 50_usize;
        let mut b = TopologyBuilder::with_lineage_and_snapshot(test_lineage(), test_snapshot());
        let v0 = mk_v(&mut b, 0.0, 0.0, 1);
        let v1 = mk_v(&mut b, 1.0, 0.0, 2);
        let v2 = mk_v(&mut b, 0.0, 1.0, 3);
        let e0 = mk_e(&mut b, v0, v1, 4);
        let e1 = mk_e(&mut b, v1, v2, 5);
        let e2 = mk_e(&mut b, v2, v0, 6);
        let face = b
            .add_face(FaceParams {
                surface: fake_surface(),
                orientation: Orientation::Forward,
                provenance: prov(7),
            })
            .expect("face");
        let loop_id = b
            .add_loop(LoopParams {
                face,
                kind: LoopKind::Outer,
                provenance: prov(8),
            })
            .expect("loop");
        for (edge, orientation) in [
            (e0, Orientation::Forward),
            (e1, Orientation::Forward),
            (e2, Orientation::Forward),
        ] {
            b.add_coedge(CoedgeParams {
                edge,
                loop_id,
                orientation,
                pcurve: fake_curve2(),
                provenance: prov(9),
            })
            .expect("coedge");
        }
        let mirror_face = b
            .add_face(FaceParams {
                surface: fake_surface(),
                orientation: Orientation::Forward,
                provenance: prov(10),
            })
            .expect("mirror face");
        let mirror_loop = b
            .add_loop(LoopParams {
                face: mirror_face,
                kind: LoopKind::Outer,
                provenance: prov(11),
            })
            .expect("mirror loop");
        for (edge, orientation) in [
            (e2, Orientation::Reversed),
            (e1, Orientation::Reversed),
            (e0, Orientation::Reversed),
        ] {
            b.add_coedge(CoedgeParams {
                edge,
                loop_id: mirror_loop,
                orientation,
                pcurve: fake_curve2(),
                provenance: prov(12),
            })
            .expect("mirror coedge");
        }
        let shell = b
            .add_shell(ShellParams {
                kind: ShellKind::Closed,
                faces: vec![face, mirror_face],
                provenance: prov(13),
            })
            .expect("shell");
        let region = b
            .add_region(RegionParams {
                outer_shell: shell,
                inner_shells: vec![],
                provenance: prov(14),
            })
            .expect("region");
        b.add_body(BodyParams {
            regions: vec![region; n],
            provenance: prov(15),
        })
        .expect("body");

        b.build().expect_err("duplicate regions must fail");
        let count = take_reachability_visit_count();
        println!("region-duplicate reachability count({n})={count}");
        let max_expected = n * 30;
        assert!(
            count <= max_expected,
            "reachability count {count} exceeds expected linear bound {max_expected} for N={n}"
        );
    }

    #[test]
    fn face_in_n_shells_linear_reachability() {
        take_reachability_visit_count();
        let n = 30_u32;
        let mut b = TopologyBuilder::with_lineage_and_snapshot(test_lineage(), test_snapshot());
        let v0 = mk_v(&mut b, 0.0, 0.0, 1);
        let v1 = mk_v(&mut b, 1.0, 0.0, 2);
        let v2 = mk_v(&mut b, 0.0, 1.0, 3);
        let e0 = mk_e(&mut b, v0, v1, 4);
        let e1 = mk_e(&mut b, v1, v2, 5);
        let e2 = mk_e(&mut b, v2, v0, 6);
        let face = b
            .add_face(FaceParams {
                surface: fake_surface(),
                orientation: Orientation::Forward,
                provenance: prov(7),
            })
            .expect("face");
        let loop_id = b
            .add_loop(LoopParams {
                face,
                kind: LoopKind::Outer,
                provenance: prov(8),
            })
            .expect("loop");
        for (edge, orientation) in [
            (e0, Orientation::Forward),
            (e1, Orientation::Forward),
            (e2, Orientation::Forward),
        ] {
            b.add_coedge(CoedgeParams {
                edge,
                loop_id,
                orientation,
                pcurve: fake_curve2(),
                provenance: prov(9),
            })
            .expect("coedge");
        }
        for i in 0..n {
            let offset = u8::try_from(i).expect("test index fits in u8");
            let base = 20_u8.wrapping_add(offset.wrapping_mul(3));
            let shell = b
                .add_shell(ShellParams {
                    kind: ShellKind::Open,
                    faces: vec![face],
                    provenance: prov(base),
                })
                .expect("shell");
            let region = b
                .add_region(RegionParams {
                    outer_shell: shell,
                    inner_shells: vec![],
                    provenance: prov(base.wrapping_add(1)),
                })
                .expect("region");
            b.add_body(BodyParams {
                regions: vec![region],
                provenance: prov(base.wrapping_add(2)),
            })
            .expect("body");
        }

        b.build().expect_err("multi-owned face must fail");
        let count = take_reachability_visit_count();
        println!("face-multishell reachability count({n})={count}");
        let max_expected = n as usize * 30;
        assert!(
            count <= max_expected,
            "reachability count {count} exceeds linear bound {max_expected} for N={n}"
        );
    }

    #[test]
    fn high_degree_face_adjacency_is_unique_and_sorted() {
        let (store, base_face) = build_pyramid(64);
        let adjacent = face_adjacent_face_ids(&store, base_face).expect("adjacency");
        assert_eq!(adjacent.len(), 64);
        assert!(adjacent.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn shared_edge_closed_shell_incidence_scales_linearly() {
        let small = shared_edge_incidence_visits(24);
        let large = shared_edge_incidence_visits(48);
        assert_eq!(small, 24 * 3);
        assert_eq!(large, 48 * 3);
    }

    #[test]
    fn repeated_large_region_reachability_scales_linearly() {
        let small = repeated_large_region_reachability_visits(24);
        let large = repeated_large_region_reachability_visits(48);
        assert!(small > 0);
        assert!(
            large <= small * 3,
            "repeated-region reachability is not linear: {small} -> {large}"
        );
    }

    #[test]
    fn high_degree_face_reuse_scales_linearly() {
        let (small_connectivity, small_reachability) = high_degree_face_reuse_visits(24);
        let (large_connectivity, large_reachability) = high_degree_face_reuse_visits(48);
        assert!(small_connectivity > 0 && small_reachability > 0);
        assert!(
            large_connectivity <= small_connectivity * 3,
            "face-reuse connectivity is not linear: {small_connectivity} -> {large_connectivity}"
        );
        assert!(
            large_reachability <= small_reachability * 3,
            "face-reuse reachability is not linear: {small_reachability} -> {large_reachability}"
        );
    }
}
