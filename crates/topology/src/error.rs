//! Structured errors for topology construction and reference validation.
//!
//! Every error variant carries enough context to produce a deterministic
//! diagnostic path without additional queries against the topology store.

use core::fmt;

use amphion_foundation::{Diagnostic, DiagnosticCode, DiagnosticPathSegment, SemanticId, Severity};

use crate::id::{BodyId, CoedgeId, EdgeId, FaceId, LoopId, RegionId, ShellId};
use crate::reference::TopologyKind;

/// Builder-side referrer context attached to cross-reference validation failures.
///
/// Carries the entity that holds the failing reference, the field name within
/// that entity, (for collection fields) the 0-based position, and the
/// referrer's semantic ID where available.
///
/// [`try_to_diagnostic`]: TopologyError::try_to_diagnostic
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferrerContext {
    /// The entity kind that holds the reference.
    pub kind: TopologyKind,
    /// The arena slot of the entity that holds the reference.
    pub slot: u32,
    /// The field name within the referrer (e.g. `"start_vertex"`,
    /// `"outer_shell"`, `"faces"`, `"regions"`).
    pub field: &'static str,
    /// For collection fields, the 0-based index within the collection.
    pub index: Option<u32>,
    /// Stable semantic identity of the referrer entity, when available.
    pub semantic_id: Option<SemanticId>,
}

/// An error produced during topology construction, lookup, or traversal.
///
/// Invalid input is always rejected; the kernel never silently heals topology.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum TopologyError {
    /// An arena's slot space was exhausted.
    ArenaOverflow,
    /// An arithmetic operation overflowed.
    ArithmeticOverflow,
    /// The snapshot generation counter would overflow `u32::MAX`.
    GenerationOverflow,
    /// A handle's generation does not match the store's snapshot generation.
    ///
    /// Generation checking is reliable only within a single lineage chain
    /// (stores produced via [`TopologyStore::successor_builder`]).
    ///
    /// [`TopologyStore::successor_builder`]: crate::store::TopologyStore::successor_builder
    StaleHandle {
        /// Entity family.
        kind: TopologyKind,
        /// Arena slot encoded in the handle.
        slot: u32,
        /// Generation encoded in the caller's handle.
        handle_generation: u32,
        /// Generation stored in the arena slot.
        store_generation: u32,
        /// Builder-side referrer context.
        ///
        /// `Some` when detected during builder validation (identifies the
        /// field that holds the stale reference). `None` for direct store
        /// lookups.
        referrer: Option<ReferrerContext>,
    },
    /// A handle's lineage does not match the store's lineage.
    ///
    /// Handles are only valid for the store/lineage they were created in.
    /// Cross-lineage usage is always an error, even when slot and generation
    /// happen to match.
    WrongLineage {
        /// Entity family of the target.
        kind: TopologyKind,
        /// Slot encoded in the handle.
        slot: u32,
        /// Lineage carried by the handle (16 bytes).
        handle_lineage: [u8; 16],
        /// Lineage of the store being queried (16 bytes).
        store_lineage: [u8; 16],
        /// Builder-side referrer context. `None` for direct store lookups.
        referrer: Option<ReferrerContext>,
    },
    /// A handle's snapshot does not match the store's snapshot.
    ///
    /// Each snapshot instance (including each branch from a common predecessor)
    /// must carry a distinct [`TopologySnapshotId`]. Handles from one snapshot
    /// are non-interchangeable with handles from any other snapshot, even at
    /// the same lineage and generation.
    ///
    /// [`TopologySnapshotId`]: crate::id::TopologySnapshotId
    WrongSnapshot {
        /// Entity family of the target.
        kind: TopologyKind,
        /// Slot encoded in the handle.
        slot: u32,
        /// Snapshot carried by the handle (16 bytes).
        handle_snapshot: [u8; 16],
        /// Snapshot of the store being queried (16 bytes).
        store_snapshot: [u8; 16],
        /// Builder-side referrer context. `None` for direct store lookups.
        referrer: Option<ReferrerContext>,
    },
    /// A referenced entity slot is out of range.
    MissingEntity {
        /// Entity family.
        kind: TopologyKind,
        /// Slot that was absent.
        slot: u32,
        /// Builder-side referrer context. `None` for direct store lookups.
        referrer: Option<ReferrerContext>,
    },
    /// A coedge exists in the store but does not belong to the given loop.
    CoedgeNotInLoop {
        /// The queried loop.
        loop_id: LoopId,
        /// The coedge that was not found in that loop.
        coedge_id: CoedgeId,
        /// Semantic IDs that explain the failing loop/coedge provenance.
        related: Vec<SemanticId>,
    },
    /// A loop's coedge chain is not topologically closed.
    OpenLoop {
        /// Loop whose coedges do not form a closed vertex cycle.
        loop_id: LoopId,
        /// Semantic IDs that explain the failing loop provenance.
        related: Vec<SemanticId>,
    },
    /// A loop has no coedges.
    EmptyLoop {
        /// Loop with an empty coedge list.
        loop_id: LoopId,
        /// Semantic IDs that explain the failing loop provenance.
        related: Vec<SemanticId>,
    },
    /// A face has no loop with [`crate::orientation::LoopKind::Outer`].
    MissingOuterLoop {
        /// Face missing its required outer boundary.
        face_id: FaceId,
        /// Semantic IDs that explain the failing face provenance.
        related: Vec<SemanticId>,
    },
    /// A face has more than one outer loop.
    DuplicateOuterLoop {
        /// Face with conflicting outer loop declarations.
        face_id: FaceId,
        /// Semantic IDs that explain the failing face provenance.
        related: Vec<SemanticId>,
    },
    /// An edge has more coedge uses than the manifold limit (2) within a
    /// single shell.
    ///
    /// Non-manifold topology must be requested through a future dedicated API.
    NonManifoldEdge {
        /// Over-used edge.
        edge_id: EdgeId,
        /// Actual number of coedge uses found.
        use_count: usize,
        /// Semantic IDs that explain the failing edge provenance.
        related: Vec<SemanticId>,
    },
    /// An edge is used by coedges in two different shells.
    ///
    /// In a valid B-Rep, shells are geometrically disjoint; an edge may only
    /// be used within a single shell.
    CrossShellEdge {
        /// The edge used by two shells.
        edge_id: EdgeId,
        /// Semantic IDs that explain the failing edge/shell provenance.
        related: Vec<SemanticId>,
    },
    /// A `Closed` shell contains an edge whose two coedge uses have the same
    /// traversal direction.
    ///
    /// A valid closed orientable shell requires opposite orientations on the
    /// two uses of every edge.
    SameDirectionEdgePair {
        /// The edge with two same-direction uses.
        edge_id: EdgeId,
        /// The shell making the `Closed` claim.
        shell_id: ShellId,
        /// Semantic IDs that explain the failing edge/shell provenance.
        related: Vec<SemanticId>,
    },
    /// An `Open` shell contains no boundary edge (every edge has two coedge
    /// uses within the shell).
    ///
    /// A shell with no boundary edges must be declared `Closed`.
    OpenShellHasNoBoundaryEdge {
        /// Shell with the inconsistent `Open` declaration.
        shell_id: ShellId,
        /// Semantic IDs that explain the failing shell provenance.
        related: Vec<SemanticId>,
    },
    /// The faces in a shell do not form a single connected component.
    ///
    /// Each connected component of faces must be its own shell.
    DisconnectedShell {
        /// Shell whose face-adjacency graph is disconnected.
        shell_id: ShellId,
        /// Semantic IDs that explain the failing shell provenance.
        related: Vec<SemanticId>,
    },
    /// A body has no regions.
    EmptyBody {
        /// Body with an empty region list.
        body_id: BodyId,
        /// Semantic IDs that explain the failing body provenance.
        related: Vec<SemanticId>,
    },
    /// A shell has no faces.
    EmptyShell {
        /// Shell with an empty face list.
        shell_id: ShellId,
        /// Semantic IDs that explain the failing shell provenance.
        related: Vec<SemanticId>,
    },
    /// A shell's [`crate::orientation::ShellKind`] is inconsistent with its
    /// actual edge topology.
    InconsistentShellKind {
        /// Shell whose claimed kind differs from what the edges imply.
        shell_id: ShellId,
        /// Semantic IDs that explain the failing shell provenance.
        related: Vec<SemanticId>,
    },
    /// Consecutive coedges in a loop do not share a vertex.
    LoopVertexMismatch {
        /// Loop containing the break.
        loop_id: LoopId,
        /// Zero-based position of the first coedge in the mismatched pair.
        position: usize,
        /// Semantic IDs that explain the failing loop provenance.
        related: Vec<SemanticId>,
    },
    /// A face appears in more than one shell.
    FaceOwnershipConflict {
        /// Face claimed by multiple shells.
        face_id: FaceId,
        /// Semantic IDs that explain the conflicting face provenance.
        related: Vec<SemanticId>,
    },
    /// A shell appears in more than one region (or as both outer boundary and
    /// cavity within the same region).
    ShellOwnershipConflict {
        /// Shell claimed by multiple regions or roles.
        shell_id: ShellId,
        /// Semantic IDs that explain the conflicting shell provenance.
        related: Vec<SemanticId>,
    },
    /// A region appears in more than one body.
    RegionOwnershipConflict {
        /// Region claimed by multiple bodies.
        region_id: RegionId,
        /// Semantic IDs that explain the conflicting region provenance.
        related: Vec<SemanticId>,
    },
    /// A collection (shell faces, region shells, body regions) contains the
    /// same ID more than once.
    DuplicateIdInCollection {
        /// Entity family of the duplicated item.
        kind: TopologyKind,
        /// Slot of the duplicated ID.
        slot: u32,
        /// Semantic IDs that explain the duplicated entity provenance.
        related: Vec<SemanticId>,
    },
    /// An entity was registered in the builder but is not reachable from any
    /// body through the standard ownership hierarchy.
    OrphanEntity {
        /// Entity family.
        kind: TopologyKind,
        /// Slot of the unreachable entity.
        slot: u32,
        /// Semantic IDs that identify the orphaned entity provenance.
        related: Vec<SemanticId>,
    },
    /// A region's outer boundary shell or cavity shell is not `Closed`.
    ///
    /// Shells that bound material regions must be topologically closed.
    OuterShellMustBeClosed {
        /// Region whose shell is not closed.
        region_id: RegionId,
        /// Shell that should be `Closed` but is not.
        shell_id: ShellId,
        /// Semantic IDs that identify the region and shell provenance.
        related: Vec<SemanticId>,
    },
    /// A region referenced an ID that belongs to a vertex, not a shell.
    ///
    /// Used when cross-entity kind validation detects an impossible reference.
    UnexpectedEntityKind {
        /// Entity kind that was found.
        found: TopologyKind,
        /// Entity kind that was required.
        expected: TopologyKind,
    },
    /// Multiple validation errors collected in one pass.
    Multiple(Vec<TopologyError>),
}

impl fmt::Display for TopologyError {
    #[allow(clippy::too_many_lines)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArenaOverflow => f.write_str("topology arena slot space exhausted"),
            Self::ArithmeticOverflow => f.write_str("topology arithmetic overflow"),
            Self::GenerationOverflow => {
                f.write_str("snapshot generation counter overflow (u32::MAX reached)")
            }
            Self::StaleHandle {
                kind,
                slot,
                handle_generation,
                store_generation,
                referrer,
            } => {
                write!(
                    f,
                    "stale {kind:?} handle at slot {slot}: \
                     handle generation {handle_generation} != store generation {store_generation}"
                )?;
                fmt_referrer(f, referrer.as_ref())
            }
            Self::MissingEntity {
                kind,
                slot,
                referrer,
            } => {
                write!(f, "missing {kind:?} entity at slot {slot}")?;
                fmt_referrer(f, referrer.as_ref())
            }
            Self::WrongLineage {
                kind,
                slot,
                handle_lineage,
                store_lineage,
                referrer,
            } => {
                write!(
                    f,
                    "wrong-lineage {kind:?} handle at slot {slot}: \
                     handle lineage {handle_lineage:02x?} != store lineage {store_lineage:02x?}"
                )?;
                fmt_referrer(f, referrer.as_ref())
            }
            Self::WrongSnapshot {
                kind,
                slot,
                handle_snapshot,
                store_snapshot,
                referrer,
            } => {
                write!(
                    f,
                    "wrong-snapshot {kind:?} handle at slot {slot}: \
                     handle snapshot {handle_snapshot:02x?} != store snapshot \
                     {store_snapshot:02x?}"
                )?;
                fmt_referrer(f, referrer.as_ref())
            }
            Self::CoedgeNotInLoop {
                loop_id, coedge_id, ..
            } => write!(
                f,
                "coedge (slot {}) does not belong to loop (slot {})",
                coedge_id.handle().slot(),
                loop_id.handle().slot()
            ),
            Self::OpenLoop { loop_id, .. } => write!(
                f,
                "loop (slot {}) is not closed: end vertex of last coedge != \
                 start vertex of first coedge",
                loop_id.handle().slot()
            ),
            Self::EmptyLoop { loop_id, .. } => {
                write!(f, "loop (slot {}) has no coedges", loop_id.handle().slot())
            }
            Self::MissingOuterLoop { face_id, .. } => write!(
                f,
                "face (slot {}) has no outer loop",
                face_id.handle().slot()
            ),
            Self::DuplicateOuterLoop { face_id, .. } => write!(
                f,
                "face (slot {}) has more than one outer loop",
                face_id.handle().slot()
            ),
            Self::NonManifoldEdge {
                edge_id, use_count, ..
            } => write!(
                f,
                "edge (slot {}) has {use_count} coedge uses within one shell; \
                 non-manifold topology requires a dedicated API",
                edge_id.handle().slot()
            ),
            Self::CrossShellEdge { edge_id, .. } => write!(
                f,
                "edge (slot {}) is used by coedges in two different shells",
                edge_id.handle().slot()
            ),
            Self::SameDirectionEdgePair {
                edge_id, shell_id, ..
            } => write!(
                f,
                "edge (slot {}) in shell (slot {}) has two coedge uses with the same \
                 traversal direction; closed orientable shells require opposite directions",
                edge_id.handle().slot(),
                shell_id.handle().slot()
            ),
            Self::OpenShellHasNoBoundaryEdge { shell_id, .. } => write!(
                f,
                "shell (slot {}) is declared Open but has no boundary edge; \
                 shells with no boundary edges must be declared Closed",
                shell_id.handle().slot()
            ),
            Self::DisconnectedShell { shell_id, .. } => write!(
                f,
                "shell (slot {}) is disconnected; each connected face-component \
                 must be its own shell",
                shell_id.handle().slot()
            ),
            Self::EmptyBody { body_id, .. } => {
                write!(f, "body (slot {}) has no regions", body_id.handle().slot())
            }
            Self::EmptyShell { shell_id, .. } => {
                write!(f, "shell (slot {}) has no faces", shell_id.handle().slot())
            }
            Self::InconsistentShellKind { shell_id, .. } => write!(
                f,
                "shell (slot {}) ShellKind claim is inconsistent with its edge topology",
                shell_id.handle().slot()
            ),
            Self::LoopVertexMismatch {
                loop_id, position, ..
            } => write!(
                f,
                "loop (slot {}) has a vertex mismatch at coedge position {position}",
                loop_id.handle().slot()
            ),
            Self::FaceOwnershipConflict { face_id, .. } => write!(
                f,
                "face (slot {}) is claimed by more than one shell",
                face_id.handle().slot()
            ),
            Self::ShellOwnershipConflict { shell_id, .. } => write!(
                f,
                "shell (slot {}) is claimed by more than one region or role",
                shell_id.handle().slot()
            ),
            Self::RegionOwnershipConflict { region_id, .. } => write!(
                f,
                "region (slot {}) is claimed by more than one body",
                region_id.handle().slot()
            ),
            Self::DuplicateIdInCollection { kind, slot, .. } => write!(
                f,
                "{kind:?} ID at slot {slot} appears more than once in a collection"
            ),
            Self::OrphanEntity { kind, slot, .. } => write!(
                f,
                "{kind:?} entity at slot {slot} is not reachable from any body"
            ),
            Self::OuterShellMustBeClosed {
                region_id,
                shell_id,
                ..
            } => write!(
                f,
                "region (slot {}) references shell (slot {}) as a boundary, \
                 but that shell is not Closed",
                region_id.handle().slot(),
                shell_id.handle().slot()
            ),
            Self::UnexpectedEntityKind { found, expected } => {
                write!(f, "expected {expected:?} entity but found {found:?}")
            }
            Self::Multiple(errors) => write!(f, "{} topology errors", errors.len()),
        }
    }
}

impl core::error::Error for TopologyError {}

impl TopologyError {
    /// Returns a stable uppercase diagnostic code for this error.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::ArenaOverflow => "TOPOLOGY.ARENA_OVERFLOW",
            Self::ArithmeticOverflow => "TOPOLOGY.ARITHMETIC_OVERFLOW",
            Self::GenerationOverflow => "TOPOLOGY.GENERATION_OVERFLOW",
            Self::StaleHandle { .. } => "TOPOLOGY.STALE_HANDLE",
            Self::WrongLineage { .. } => "TOPOLOGY.WRONG_LINEAGE",
            Self::WrongSnapshot { .. } => "TOPOLOGY.WRONG_SNAPSHOT",
            Self::MissingEntity { .. } => "TOPOLOGY.MISSING_ENTITY",
            Self::CoedgeNotInLoop { .. } => "TOPOLOGY.COEDGE_NOT_IN_LOOP",
            Self::OpenLoop { .. } => "TOPOLOGY.OPEN_LOOP",
            Self::EmptyLoop { .. } => "TOPOLOGY.EMPTY_LOOP",
            Self::MissingOuterLoop { .. } => "TOPOLOGY.MISSING_OUTER_LOOP",
            Self::DuplicateOuterLoop { .. } => "TOPOLOGY.DUPLICATE_OUTER_LOOP",
            Self::NonManifoldEdge { .. } => "TOPOLOGY.NON_MANIFOLD_EDGE",
            Self::CrossShellEdge { .. } => "TOPOLOGY.CROSS_SHELL_EDGE",
            Self::SameDirectionEdgePair { .. } => "TOPOLOGY.SAME_DIRECTION_EDGE_PAIR",
            Self::OpenShellHasNoBoundaryEdge { .. } => "TOPOLOGY.OPEN_SHELL_NO_BOUNDARY",
            Self::DisconnectedShell { .. } => "TOPOLOGY.DISCONNECTED_SHELL",
            Self::EmptyBody { .. } => "TOPOLOGY.EMPTY_BODY",
            Self::EmptyShell { .. } => "TOPOLOGY.EMPTY_SHELL",
            Self::InconsistentShellKind { .. } => "TOPOLOGY.INCONSISTENT_SHELL_KIND",
            Self::LoopVertexMismatch { .. } => "TOPOLOGY.LOOP_VERTEX_MISMATCH",
            Self::FaceOwnershipConflict { .. } => "TOPOLOGY.FACE_OWNERSHIP_CONFLICT",
            Self::ShellOwnershipConflict { .. } => "TOPOLOGY.SHELL_OWNERSHIP_CONFLICT",
            Self::RegionOwnershipConflict { .. } => "TOPOLOGY.REGION_OWNERSHIP_CONFLICT",
            Self::DuplicateIdInCollection { .. } => "TOPOLOGY.DUPLICATE_ID",
            Self::OrphanEntity { .. } => "TOPOLOGY.ORPHAN_ENTITY",
            Self::OuterShellMustBeClosed { .. } => "TOPOLOGY.OUTER_SHELL_MUST_BE_CLOSED",
            Self::UnexpectedEntityKind { .. } => "TOPOLOGY.UNEXPECTED_ENTITY_KIND",
            Self::Multiple(_) => "TOPOLOGY.MULTIPLE",
        }
    }

    /// Returns every individual diagnostic in deterministic order.
    ///
    /// This is the primary diagnostic API. [`TopologyError::Multiple`] is
    /// recursively flattened so no child error information is discarded; all
    /// other variants produce exactly one [`Diagnostic`].
    #[must_use]
    pub fn to_diagnostics(&self) -> Vec<Diagnostic> {
        match self {
            Self::Multiple(errs) => errs
                .iter()
                .flat_map(TopologyError::to_diagnostics)
                .collect(),
            other => vec![other.build_single_diagnostic()],
        }
    }

    /// Converts this error into a single [`Diagnostic`].
    ///
    /// Returns `Err(errs.as_slice())` for [`TopologyError::Multiple`] so that no
    /// child error information is silently discarded. All other variants produce
    /// exactly one diagnostic.
    ///
    /// Use [`to_diagnostics`][Self::to_diagnostics] when `Multiple` must be handled.
    ///
    /// # Errors
    ///
    /// Returns the child error slice for [`TopologyError::Multiple`].
    pub fn try_to_diagnostic(&self) -> Result<Diagnostic, &[TopologyError]> {
        match self {
            Self::Multiple(errs) => Err(errs.as_slice()),
            other => Ok(other.build_single_diagnostic()),
        }
    }

    /// Builds the single diagnostic for a non-`Multiple` error.
    ///
    /// # Panics
    ///
    /// Panics only if one of this type's own hard-coded diagnostic codes is
    /// not a valid [`DiagnosticCode`].
    #[allow(clippy::too_many_lines)]
    #[must_use]
    fn build_single_diagnostic(&self) -> Diagnostic {
        let code = DiagnosticCode::try_new(self.code())
            .expect("all topology codes are valid DiagnosticCode values");
        let message = self.to_string();
        let path = match self {
            Self::StaleHandle {
                kind,
                slot,
                referrer,
                ..
            }
            | Self::WrongLineage {
                kind,
                slot,
                referrer,
                ..
            }
            | Self::WrongSnapshot {
                kind,
                slot,
                referrer,
                ..
            }
            | Self::MissingEntity {
                kind,
                slot,
                referrer,
            } => referrer_path(*kind, *slot, referrer.as_ref()),
            Self::DuplicateIdInCollection { kind, slot, .. }
            | Self::OrphanEntity { kind, slot, .. } => {
                vec![entity_path(*kind, *slot)]
            }
            Self::CoedgeNotInLoop { loop_id, .. }
            | Self::OpenLoop { loop_id, .. }
            | Self::EmptyLoop { loop_id, .. }
            | Self::LoopVertexMismatch { loop_id, .. } => {
                vec![entity_path(TopologyKind::Loop, loop_id.handle().slot())]
            }
            Self::MissingOuterLoop { face_id, .. }
            | Self::DuplicateOuterLoop { face_id, .. }
            | Self::FaceOwnershipConflict { face_id, .. } => {
                vec![entity_path(TopologyKind::Face, face_id.handle().slot())]
            }
            Self::NonManifoldEdge { edge_id, .. } | Self::CrossShellEdge { edge_id, .. } => {
                vec![entity_path(TopologyKind::Edge, edge_id.handle().slot())]
            }
            Self::SameDirectionEdgePair { shell_id, .. }
            | Self::OpenShellHasNoBoundaryEdge { shell_id, .. }
            | Self::DisconnectedShell { shell_id, .. }
            | Self::EmptyShell { shell_id, .. }
            | Self::InconsistentShellKind { shell_id, .. }
            | Self::ShellOwnershipConflict { shell_id, .. } => {
                vec![entity_path(TopologyKind::Shell, shell_id.handle().slot())]
            }
            Self::EmptyBody { body_id, .. } => {
                vec![entity_path(TopologyKind::Body, body_id.handle().slot())]
            }
            Self::RegionOwnershipConflict { region_id, .. }
            | Self::OuterShellMustBeClosed { region_id, .. } => {
                vec![entity_path(TopologyKind::Region, region_id.handle().slot())]
            }
            Self::ArenaOverflow
            | Self::ArithmeticOverflow
            | Self::GenerationOverflow
            | Self::UnexpectedEntityKind { .. } => Vec::new(),
            Self::Multiple(_) => {
                unreachable!("Multiple is handled by try_to_diagnostic/to_diagnostics")
            }
        };
        let mut related = match self {
            Self::StaleHandle { referrer, .. }
            | Self::WrongLineage { referrer, .. }
            | Self::WrongSnapshot { referrer, .. }
            | Self::MissingEntity { referrer, .. } => {
                referrer.iter().filter_map(|ctx| ctx.semantic_id).collect()
            }
            Self::EmptyLoop { related, .. }
            | Self::MissingOuterLoop { related, .. }
            | Self::DuplicateOuterLoop { related, .. }
            | Self::NonManifoldEdge { related, .. }
            | Self::CrossShellEdge { related, .. }
            | Self::OpenLoop { related, .. }
            | Self::SameDirectionEdgePair { related, .. }
            | Self::OpenShellHasNoBoundaryEdge { related, .. }
            | Self::DisconnectedShell { related, .. }
            | Self::EmptyBody { related, .. }
            | Self::EmptyShell { related, .. }
            | Self::InconsistentShellKind { related, .. }
            | Self::LoopVertexMismatch { related, .. }
            | Self::DuplicateIdInCollection { related, .. }
            | Self::CoedgeNotInLoop { related, .. }
            | Self::FaceOwnershipConflict { related, .. }
            | Self::ShellOwnershipConflict { related, .. }
            | Self::RegionOwnershipConflict { related, .. }
            | Self::OuterShellMustBeClosed { related, .. }
            | Self::OrphanEntity { related, .. } => related.clone(),
            _ => Vec::new(),
        };
        related.sort_unstable();
        related.dedup();
        Diagnostic::new(Severity::Error, code, message, path, related)
    }
}

/// Builds a diagnostic path from a target entity and optional referrer context.
///
/// With referrer: `[entity(ref), field(field), index?(idx), entity(target)]`
/// Without referrer: `[entity(target)]`
fn referrer_path(
    target_kind: TopologyKind,
    target_slot: u32,
    referrer: Option<&ReferrerContext>,
) -> Vec<DiagnosticPathSegment> {
    if let Some(ctx) = referrer {
        let mut path = vec![
            entity_path(ctx.kind, ctx.slot),
            DiagnosticPathSegment::Field(ctx.field.to_owned()),
        ];
        if let Some(idx) = ctx.index {
            path.push(DiagnosticPathSegment::Index(u64::from(idx)));
        }
        path.push(entity_path(target_kind, target_slot));
        path
    } else {
        vec![entity_path(target_kind, target_slot)]
    }
}

/// Appends referrer context to a Display formatter.
fn fmt_referrer(f: &mut fmt::Formatter<'_>, referrer: Option<&ReferrerContext>) -> fmt::Result {
    if let Some(ctx) = referrer {
        write!(
            f,
            " (referrer: {:?} slot {} field '{}'",
            ctx.kind, ctx.slot, ctx.field
        )?;
        if let Some(idx) = ctx.index {
            write!(f, "[{idx}]")?;
        }
        write!(f, ")")?;
    }
    Ok(())
}

fn entity_path(kind: TopologyKind, slot: u32) -> DiagnosticPathSegment {
    DiagnosticPathSegment::Entity {
        kind: format!("{kind:?}").to_lowercase(),
        id: u64::from(slot),
    }
}

/// Collapses a `Vec<TopologyError>` into a single [`TopologyError`].
///
/// Returns the single element directly for a one-element vector.
pub(crate) fn collect_errors(mut errors: Vec<TopologyError>) -> TopologyError {
    if errors.len() == 1 {
        errors.remove(0)
    } else {
        TopologyError::Multiple(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::{ReferrerContext, TopologyError, collect_errors};
    use crate::id::{BodyId, CoedgeId, FaceId, LoopId, TopologyLineageId, TopologySnapshotId};
    use crate::reference::TopologyKind;
    use amphion_foundation::{SemanticId, Severity};

    fn test_lineage() -> TopologyLineageId {
        TopologyLineageId::new(SemanticId::from_bytes([0xAA; 16]))
    }

    fn test_snapshot() -> TopologySnapshotId {
        TopologySnapshotId::new(SemanticId::from_bytes([0xCC; 16]))
    }

    #[test]
    fn display_stale_handle() {
        let err = TopologyError::StaleHandle {
            kind: TopologyKind::Vertex,
            slot: 3,
            handle_generation: 2,
            store_generation: 5,
            referrer: None,
        };
        let s = err.to_string();
        assert!(s.contains("Vertex"), "missing kind: {s}");
        assert!(s.contains("slot 3"), "missing slot: {s}");
    }

    #[test]
    fn display_coedge_not_in_loop() {
        let err = TopologyError::CoedgeNotInLoop {
            loop_id: LoopId::new(2, 0, test_lineage(), test_snapshot()),
            coedge_id: CoedgeId::new(7, 0, test_lineage(), test_snapshot()),
            related: vec![],
        };
        let s = err.to_string();
        assert!(s.contains("coedge"), "missing 'coedge': {s}");
        assert!(s.contains("loop"), "missing 'loop': {s}");
    }

    #[test]
    fn collect_single_error_unwraps() {
        let face_id = FaceId::new(0, 0, test_lineage(), test_snapshot());
        let err = TopologyError::MissingOuterLoop {
            face_id,
            related: vec![],
        };
        let collected = collect_errors(vec![err.clone()]);
        assert_eq!(collected, err);
    }

    #[test]
    fn collect_multiple_errors_wraps() {
        let face_id = FaceId::new(0, 0, test_lineage(), test_snapshot());
        let err = TopologyError::MissingOuterLoop {
            face_id,
            related: vec![],
        };
        let collected = collect_errors(vec![err.clone(), err.clone()]);
        assert!(matches!(collected, TopologyError::Multiple(ref v) if v.len() == 2));
    }

    #[test]
    fn diagnostic_code_is_stable() {
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
    fn wrong_snapshot_code_is_stable() {
        let err = TopologyError::WrongSnapshot {
            kind: TopologyKind::Vertex,
            slot: 1,
            handle_snapshot: [0xCC; 16],
            store_snapshot: [0xDD; 16],
            referrer: None,
        };
        assert_eq!(err.code(), "TOPOLOGY.WRONG_SNAPSHOT");
    }

    #[test]
    fn diagnostic_with_referrer_has_three_segments() {
        let err = TopologyError::MissingEntity {
            kind: TopologyKind::Shell,
            slot: 5,
            referrer: Some(ReferrerContext {
                kind: TopologyKind::Region,
                slot: 2,
                field: "outer_shell",
                index: None,
                semantic_id: None,
            }),
        };
        let diag = err.try_to_diagnostic().expect("not Multiple");
        assert_eq!(diag.path().len(), 3, "expected [region, field, shell]");
    }

    #[test]
    fn diagnostic_with_referrer_and_index_has_four_segments() {
        let err = TopologyError::MissingEntity {
            kind: TopologyKind::Region,
            slot: 7,
            referrer: Some(ReferrerContext {
                kind: TopologyKind::Body,
                slot: 0,
                field: "regions",
                index: Some(1),
                semantic_id: None,
            }),
        };
        let diag = err.try_to_diagnostic().expect("not Multiple");
        assert_eq!(
            diag.path().len(),
            4,
            "expected [body, field, index, region]"
        );
    }

    #[test]
    fn to_diagnostics_flattens_multiple() {
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
        let multiple = TopologyError::Multiple(vec![err1.clone(), err2.clone()]);
        let diags = multiple.to_diagnostics();
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].code().as_str(), "TOPOLOGY.MISSING_OUTER_LOOP");
        assert_eq!(diags[1].code().as_str(), "TOPOLOGY.EMPTY_BODY");
    }

    #[test]
    fn to_diagnostics_nested_multiple_flattened() {
        let body_id = BodyId::new(0, 0, test_lineage(), test_snapshot());
        let err = TopologyError::EmptyBody {
            body_id,
            related: vec![],
        };
        let inner = TopologyError::Multiple(vec![err.clone(), err.clone()]);
        let outer = TopologyError::Multiple(vec![inner, err.clone()]);
        let diags = outer.to_diagnostics();
        assert_eq!(
            diags.len(),
            3,
            "nested Multiple must be recursively flattened"
        );
    }

    #[test]
    fn try_to_diagnostic_rejects_multiple() {
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
        let multiple = TopologyError::Multiple(vec![err1.clone(), err2.clone()]);
        let children = multiple
            .try_to_diagnostic()
            .expect_err("Multiple must preserve children");
        assert_eq!(children, &[err1, err2]);
    }

    #[test]
    fn diagnostic_referrer_semantic_id_in_related() {
        let sem = SemanticId::from_bytes([0x42; 16]);
        let err = TopologyError::MissingEntity {
            kind: TopologyKind::Shell,
            slot: 5,
            referrer: Some(ReferrerContext {
                kind: TopologyKind::Region,
                slot: 2,
                field: "outer_shell",
                index: None,
                semantic_id: Some(sem),
            }),
        };
        let diag = err.try_to_diagnostic().expect("not Multiple");
        assert_eq!(diag.related().len(), 1);
        assert_eq!(diag.related()[0], sem);
    }
}
