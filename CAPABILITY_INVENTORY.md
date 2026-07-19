# Amphion capability inventory

Inventory version: `0.1.0-research`

Reference snapshot: Autodesk Fusion public documentation accessed 2026-07-19.

This is the long-lived clean-room behavior inventory for Fusion-familiar
Amphion clients. It records workflows and command semantics, not Autodesk
source code, artwork, icons, wording, branding, or visual trade dress.

Every row is in the finished-product scope unless its target is explicitly
changed by a documented product decision. A row is not complete merely because
a similarly named command exists: `parity` requires the full contract from
[PRODUCT_PLAN.md](PRODUCT_PLAN.md), automated behavioral tests, interaction
tests, accessibility tests, and migration testing.

## Inventory rules

Stable IDs use `DOMAIN.WORKSPACE.GROUP.CAPABILITY[.VARIANT]`. IDs are never
reused. A renamed capability keeps its ID. A split capability receives new IDs
and leaves a redirect note in the changelog.

| Field | Values |
| --- | --- |
| Target | `required`, `improved`, `deferred`, or `excluded` |
| Evidence | `official`, `official-overview`, `secondary`, `product-decision`, or `research-needed` |
| Status | `research`, `specified`, `blocked`, `prototype`, `partial`, `parity`, or `improved` |
| Milestone | First milestone that promises the capability; not a delivery date |

`Reference name` is retained only to make migration research auditable.
Amphion's final labels, descriptions, icons, and layouts will be original.
Rows marked `research-needed` remain required targets, but their exact
reference behavior must be verified before detailed specification.

## Application shell, command system, and selection

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `UI.SHELL.APPLICATION_BAR` | Application Bar | Document, project, workspace, save/sync, help, and account entry points | improved | official | M1 | specified |
| `UI.SHELL.DOCUMENT_TABS` | Document Tabs | Switch and compare open documents without losing command state | improved | official | M1 | specified |
| `UI.SHELL.WORKSPACE_SWITCHER` | Workspace Switcher | Change product workspace while preserving document and selection context | required | official | M7 | research |
| `UI.SHELL.CONTEXT_TOOLBAR` | Toolbar | Stable workspace groups with contextual tools and responsive overflow | improved | official | M1 | specified |
| `UI.SHELL.MODEL_BROWSER` | Browser | Hierarchical document, component, origin, sketch, body, construction, joint, and analysis tree | required | official | M2 | specified |
| `UI.SHELL.COMMAND_INSPECTOR` | Command Dialog | Dockable live parameters, selection slots, preview validity, confirm, and cancel | improved | official | M1 | specified |
| `UI.SHELL.TIMELINE` | Timeline | Ordered feature history, edit, suppress, rollback, reorder, diagnostics, and playback | required | official | M4 | specified |
| `UI.SHELL.NAVIGATION_BAR` | Navigation Bar | Camera, display, grid, selection, and fit controls | required | official | M1 | specified |
| `UI.COMMAND.SEARCH` | Design Shortcuts / `S` | Search every available command and invoke it from the keyboard | improved | official | M1 | specified |
| `UI.COMMAND.MARKING_MENU` | Marking Menu | Context-sensitive radial menu plus complete overflow list | required | official | M4 | research |
| `UI.COMMAND.REPEAT_LAST` | Repeat Last Command | Repeat the last compatible command without re-navigation | required | secondary | M4 | research |
| `UI.COMMAND.PRESELECTION` | Selection before command | Seed compatible commands from an existing selection | required | official | M1 | specified |
| `UI.COMMAND.POSTSELECTION` | Command before selection | Collect required selections after invocation | required | official | M1 | specified |
| `UI.COMMAND.LIVE_PREVIEW` | Live Preview | Update transient geometry and diagnostics without mutating the document | improved | official | M1 | specified |
| `UI.COMMAND.CONFIRM` | OK / Enter | Commit one valid transaction | required | official | M1 | specified |
| `UI.COMMAND.CANCEL` | Cancel / Escape | Back out one level or cancel without a history entry | required | official | M1 | specified |
| `UI.HISTORY.UNDO` | Undo | Reverse complete document transactions | improved | official | M2 | specified |
| `UI.HISTORY.REDO` | Redo | Reapply complete reverted transactions | improved | official | M2 | specified |
| `UI.HISTORY.PERSISTENT_UNDO` | Session-only undo | Persist a bounded undo history across reopen | improved | product-decision | M7 | specified |
| `UI.SELECT.WINDOW.ENCLOSED` | Window Selection | Left-to-right window selects only fully enclosed entities | required | secondary | M3 | research |
| `UI.SELECT.WINDOW.CROSSING` | Crossing Selection | Right-to-left window selects every touched entity | required | secondary | M3 | research |
| `UI.SELECT.FREEFORM` | Freeform Selection | Lasso arbitrary entities | required | official | M7 | research |
| `UI.SELECT.PAINT` | Paint Selection | Add entities continuously while dragging | required | official | M7 | research |
| `UI.SELECT.FILTERS` | Selection Filters | Enable entity-type filters and selection priorities | improved | official | M3 | research |
| `UI.SELECT.SETS` | Selection Sets | Save, name, recall, and update reusable selections | required | official | M7 | research |
| `UI.SELECT.SEMANTIC` | Persistent selection | Resolve visible selections through stable semantic IDs after recompute | improved | product-decision | M2 | specified |

## Navigation, camera, and input profiles

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `NAV.CUBE.SNAP_FACE` | ViewCube face | Snap to six orthographic face views | required | official | M1 | specified |
| `NAV.CUBE.SNAP_EDGE` | ViewCube edge | Snap to twelve two-face views | required | official | M1 | specified |
| `NAV.CUBE.SNAP_CORNER` | ViewCube corner | Snap to eight isometric corner views | required | official | M1 | specified |
| `NAV.CUBE.DRAG_ORBIT` | ViewCube drag | Orbit continuously by dragging the cube | required | official | M1 | specified |
| `NAV.CUBE.GO_HOME` | Home | Restore the document home camera | required | official | M1 | specified |
| `NAV.CUBE.SET_HOME` | Set Current View as Home | Save a custom document home camera | required | official | M1 | specified |
| `NAV.CUBE.NAMED_VIEWS` | Named Views | Save, rename, recall, and delete document views | required | official | M7 | research |
| `NAV.CUBE.ACCESSIBILITY` | ViewCube | Full keyboard, focus, screen-reader, touch-target, and reduced-motion support | improved | product-decision | M1 | specified |
| `NAV.CAMERA.ORBIT` | Orbit | Orbit around a deterministic pivot | required | official | M1 | specified |
| `NAV.CAMERA.PAN` | Pan | Translate the camera parallel to the image plane | required | official | M1 | specified |
| `NAV.CAMERA.ZOOM_CONTINUOUS` | Zoom | Continuous zoom with bounded speed | required | official | M1 | specified |
| `NAV.CAMERA.ZOOM_TO_CURSOR` | Zoom | Preserve the world point under the pointer or gesture centroid | improved | product-decision | M1 | specified |
| `NAV.CAMERA.FIT_ALL` | Fit | Frame all visible geometry | required | official | M1 | specified |
| `NAV.CAMERA.FIT_SELECTION` | Fit Selection | Frame selected geometry | required | official | M1 | specified |
| `NAV.CAMERA.FOCUS_SELECTION` | Set Orbit Center | Set the orbit pivot from geometry | required | official | M1 | specified |
| `NAV.CAMERA.LOOK_AT` | Look At | Align the camera normal to selected planar geometry | required | official | M1 | specified |
| `NAV.CAMERA.ROLL` | Roll | Roll around the view axis | required | official-overview | M7 | research |
| `NAV.CAMERA.ORTHOGRAPHIC` | Orthographic | Parallel projection camera | required | official | M1 | specified |
| `NAV.CAMERA.PERSPECTIVE` | Perspective | Perspective projection camera | required | official | M1 | specified |
| `NAV.CAMERA.TRANSITION` | View transition | Animated, interruptible, reduced-motion-aware view changes | improved | product-decision | M1 | specified |
| `INPUT.PROFILE.AMPHION` | - | Accessible Amphion keyboard, mouse, trackpad, pen, and touch defaults | improved | product-decision | M1 | specified |
| `INPUT.PROFILE.FUSION_MOUSE` | Fusion mouse profile | MMB pan, Shift+MMB orbit, wheel zoom, double-MMB fit | required | secondary | M1 | specified |
| `INPUT.PROFILE.TRACKPAD` | Trackpad navigation | Native pan, pinch zoom, orbit, fit, and gesture conflict handling | improved | product-decision | M1 | specified |
| `INPUT.PROFILE.REMAPPING` | Keyboard Shortcuts | Per-command remapping with conflict detection and profile import/export | improved | official-overview | M7 | specified |
| `INPUT.PROFILE.DEVICE_INDEPENDENT` | - | Map all devices to shared camera and command intents | improved | product-decision | M1 | specified |

## Sketch environment

### Create geometry

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `DESIGN.SKETCH.CREATE.LINE` | Line | Create connected line and tangent-arc chains | required | official | M3 | research |
| `DESIGN.SKETCH.CREATE.MIDPOINT_LINE` | Midpoint Line | Create a line symmetrically from its midpoint | required | official | M3 | research |
| `DESIGN.SKETCH.CREATE.RECTANGLE_2_POINT` | 2-Point Rectangle | Rectangle from diagonal corners | required | research-needed | M3 | research |
| `DESIGN.SKETCH.CREATE.RECTANGLE_3_POINT` | 3-Point Rectangle | Rotated rectangle from an edge and width | required | research-needed | M3 | research |
| `DESIGN.SKETCH.CREATE.RECTANGLE_CENTER` | Center Rectangle | Rectangle from center and corner | required | research-needed | M3 | research |
| `DESIGN.SKETCH.CREATE.ARC_3_POINT` | 3-Point Arc | Arc through start, end, and a point on the arc | required | official | M3 | research |
| `DESIGN.SKETCH.CREATE.ARC_CENTER_POINT` | Center Point Arc | Arc from center, start, and end | required | official | M3 | research |
| `DESIGN.SKETCH.CREATE.ARC_TANGENT` | Tangent Arc | Arc tangent to connected sketch geometry | required | official | M3 | research |
| `DESIGN.SKETCH.CREATE.CIRCLE_CENTER_DIAMETER` | Center Diameter Circle | Circle from center and diameter | required | official | M3 | research |
| `DESIGN.SKETCH.CREATE.CIRCLE_2_POINT` | 2-Point Circle | Circle from diameter endpoints | required | official | M3 | research |
| `DESIGN.SKETCH.CREATE.CIRCLE_3_POINT` | 3-Point Circle | Circle through three points | required | official | M3 | research |
| `DESIGN.SKETCH.CREATE.CIRCLE_2_TANGENT` | 2-Tangent Circle | Circle tangent to two lines with a radius | required | official | M3 | research |
| `DESIGN.SKETCH.CREATE.CIRCLE_3_TANGENT` | 3-Tangent Circle | Circle tangent to three curves | required | official | M3 | research |
| `DESIGN.SKETCH.CREATE.ELLIPSE` | Ellipse | Ellipse from center and principal radii | required | research-needed | M3 | research |
| `DESIGN.SKETCH.CREATE.CONIC` | Conic Curve | Rational conic controlled by endpoints, tangent point, and rho | required | research-needed | M3 | research |
| `DESIGN.SKETCH.CREATE.SPLINE_FIT_POINT` | Fit Point Spline | Interpolating spline through fit points | required | official | M3 | research |
| `DESIGN.SKETCH.CREATE.SPLINE_CONTROL_POINT` | Control Point Spline | Cage-controlled approximating spline | required | official | M3 | research |
| `DESIGN.SKETCH.CREATE.SLOT_CENTER_TO_CENTER` | Center to Center Slot | Straight slot from two arc centers and width | required | official | M3 | research |
| `DESIGN.SKETCH.CREATE.SLOT_OVERALL` | Overall Slot | Straight slot from overall endpoints and width | required | official | M3 | research |
| `DESIGN.SKETCH.CREATE.SLOT_CENTER_TO_POINT` | Center to Point Slot | Straight slot from center, arc center, and width | required | official | M3 | research |
| `DESIGN.SKETCH.CREATE.SLOT_3_POINT_ARC` | 3 Point Arc Slot | Curved slot from a three-point center arc and width | required | official | M3 | research |
| `DESIGN.SKETCH.CREATE.SLOT_CENTER_POINT_ARC` | Center Point Arc Slot | Curved slot from center arc and width | required | official | M3 | research |
| `DESIGN.SKETCH.CREATE.POLYGON_CIRCUMSCRIBED` | Circumscribed Polygon | Regular polygon whose edge midpoints lie on a circle | required | official | M3 | research |
| `DESIGN.SKETCH.CREATE.POLYGON_INSCRIBED` | Inscribed Polygon | Regular polygon whose vertices lie on a circle | required | official | M3 | research |
| `DESIGN.SKETCH.CREATE.POLYGON_EDGE` | Edge Polygon | Regular polygon from one edge and side count | required | official | M3 | research |
| `DESIGN.SKETCH.CREATE.POINT` | Point | Sketch point usable by constraints and dimensions | required | research-needed | M3 | research |
| `DESIGN.SKETCH.CREATE.TEXT` | Text | Editable profile-generating sketch text | improved | research-needed | M3 | research |
| `DESIGN.SKETCH.CREATE.MIRROR` | Mirror | Mirror selected sketch entities across a line | required | official-overview | M3 | research |
| `DESIGN.SKETCH.CREATE.PATTERN_RECTANGULAR` | Rectangular Pattern | Associative two-direction sketch pattern | required | official-overview | M3 | research |
| `DESIGN.SKETCH.CREATE.PATTERN_CIRCULAR` | Circular Pattern | Associative sketch pattern around a center | required | official-overview | M3 | research |
| `DESIGN.SKETCH.CREATE.MODE_3D` | 3D Sketch | Create supported sketch geometry outside one plane | improved | official | M8 | research |

### Project and include

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `DESIGN.SKETCH.PROJECT.PROJECT` | Project | Project selected faces, edges, points, or bodies to the sketch plane with optional associativity | required | official | M3 | research |
| `DESIGN.SKETCH.PROJECT.INTERSECT` | Intersect | Create the section where geometry crosses the sketch plane | required | official | M3 | research |
| `DESIGN.SKETCH.PROJECT.SPUN_PROFILE` | Spun Profile | Project the revolved envelope of 3D geometry | required | official | M8 | research |
| `DESIGN.SKETCH.PROJECT.INCLUDE_3D` | Include 3D Geometry | Include edges, work geometry, and sketch curves in a 3D sketch | required | official | M8 | research |
| `DESIGN.SKETCH.PROJECT.TO_SURFACE` | Project To Surface | Project sketch geometry and points onto selected faces | required | official | M8 | research |
| `DESIGN.SKETCH.PROJECT.INTERSECTION_CURVE` | Intersection Curve | Create a 3D curve from intersecting geometry | required | official | M8 | research |
| `DESIGN.SKETCH.PROJECT.ISOPARAMETRIC_CURVE` | Isoparametric Curve | Extract linked U, V, or distributed UV curves from a surface | required | official | M8 | research |

### Modify geometry

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `DESIGN.SKETCH.MODIFY.FILLET` | Fillet | Replace a corner with a tangent arc | required | official | M3 | research |
| `DESIGN.SKETCH.MODIFY.CHAMFER_EQUAL_DISTANCE` | Equal Distance Chamfer | Chamfer a corner with equal offsets | required | official | M3 | research |
| `DESIGN.SKETCH.MODIFY.CHAMFER_DISTANCE_ANGLE` | Distance And Angle Chamfer | Chamfer from one distance and one angle | required | official | M3 | research |
| `DESIGN.SKETCH.MODIFY.CHAMFER_TWO_DISTANCE` | Two Distance Chamfer | Chamfer from two independent offsets | required | official | M3 | research |
| `DESIGN.SKETCH.MODIFY.TRIM` | Trim | Remove a bounded curve segment at intersections | required | official | M3 | research |
| `DESIGN.SKETCH.MODIFY.EXTEND` | Extend | Extend a curve to its nearest eligible intersection | required | official | M3 | research |
| `DESIGN.SKETCH.MODIFY.BREAK` | Break | Split sketch geometry at an intersection or selected point | required | official | M3 | research |
| `DESIGN.SKETCH.MODIFY.SCALE` | Sketch Scale | Uniformly scale sketch geometry from a point | required | official | M3 | research |
| `DESIGN.SKETCH.MODIFY.OFFSET` | Offset | Create one- or two-sided linked offsets with topology matching | required | official | M3 | research |
| `DESIGN.SKETCH.MODIFY.BLEND_CURVE` | Blend Curve | Create a G1- or G2-continuous curve between two curves | required | official | M3 | research |
| `DESIGN.SKETCH.MODIFY.MOVE_COPY` | Move/Copy | Translate, rotate, or copy geometry, including supported off-plane moves | required | official | M3 | research |
| `DESIGN.SKETCH.MODIFY.CHANGE_PARAMETERS` | Change Parameters | Open and edit document parameters from sketch context | required | official | M4 | research |

### Geometric constraints

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `DESIGN.SKETCH.CONSTRAINT.HORIZONTAL_VERTICAL` | Horizontal/Vertical | Make a line or point pair horizontal or vertical, choosing the nearer relation | required | official | M3 | research |
| `DESIGN.SKETCH.CONSTRAINT.COINCIDENT` | Coincident | Make two points coincide or place a point on a curve | required | official | M3 | research |
| `DESIGN.SKETCH.CONSTRAINT.TANGENT` | Tangent | Impose first-order tangency between eligible curves | required | official | M3 | research |
| `DESIGN.SKETCH.CONSTRAINT.EQUAL` | Equal | Equalize compatible lengths or radii | required | official | M3 | research |
| `DESIGN.SKETCH.CONSTRAINT.PARALLEL` | Parallel | Make two lines parallel | required | official | M3 | research |
| `DESIGN.SKETCH.CONSTRAINT.PERPENDICULAR` | Perpendicular | Make eligible entities perpendicular | required | official | M3 | research |
| `DESIGN.SKETCH.CONSTRAINT.FIX_UNFIX` | Fix/UnFix | Lock or release geometry at its current size and position | required | official | M3 | research |
| `DESIGN.SKETCH.CONSTRAINT.MIDPOINT` | Midpoint | Place a point or object at another object's midpoint | required | official | M3 | research |
| `DESIGN.SKETCH.CONSTRAINT.CONCENTRIC` | Concentric | Make arcs, circles, or ellipses share a center | required | official | M3 | research |
| `DESIGN.SKETCH.CONSTRAINT.COLLINEAR` | Collinear | Put lines on the same infinite line | required | official | M3 | research |
| `DESIGN.SKETCH.CONSTRAINT.SYMMETRY` | Symmetry | Make geometry symmetric about an axis | required | official | M3 | research |
| `DESIGN.SKETCH.CONSTRAINT.CURVATURE` | Curvature | Impose G2 continuity from a spline to an adjacent curve | required | official | M3 | research |

`Smooth` is not a sketch constraint; it is a Form modify command. A separate
`Coplanar` 3D-sketch constraint was not found in the official constraint list
and is intentionally absent pending authoritative evidence.

### Dimensions, solver state, and palette

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `DESIGN.SKETCH.DIMENSION.GENERAL` | Sketch Dimension | Infer and create linear, aligned, angular, radial, or diametric dimensions from selection | required | official-overview | M3 | research |
| `DESIGN.SKETCH.DIMENSION.DRIVING` | Driving Dimension | Drive geometry from a value, expression, or named parameter | required | official-overview | M3 | research |
| `DESIGN.SKETCH.DIMENSION.DRIVEN` | Driven Dimension | Report geometry without constraining it | required | official-overview | M3 | research |
| `DESIGN.SKETCH.SOLVER.DOF` | Degrees of Freedom | Show remaining translational/rotational degrees of freedom | improved | official-overview | M3 | specified |
| `DESIGN.SKETCH.SOLVER.CONFLICTS` | Constraint conflict display | Identify redundant and conflicting constraints with actionable explanations | improved | official-overview | M3 | specified |
| `DESIGN.SKETCH.SOLVER.STATUS_COLORS` | Constraint status colors | Distinguish underconstrained, fully constrained, conflicted, fixed, construction, and projected geometry without relying only on color | improved | secondary | M3 | specified |
| `DESIGN.SKETCH.PALETTE.SHOW_PROFILE` | Show Profile | Toggle closed-profile shading | required | official-overview | M3 | research |
| `DESIGN.SKETCH.PALETTE.SHOW_POINTS` | Show Points | Toggle sketch point visibility | required | official-overview | M3 | research |
| `DESIGN.SKETCH.PALETTE.SHOW_DIMENSIONS` | Show Dimensions | Toggle dimensions | required | official-overview | M3 | research |
| `DESIGN.SKETCH.PALETTE.SHOW_CONSTRAINTS` | Show Constraints | Toggle constraint glyphs | required | official-overview | M3 | research |
| `DESIGN.SKETCH.PALETTE.GRID` | Show/Hide Grid | Toggle the sketch grid | required | official-overview | M3 | research |
| `DESIGN.SKETCH.PALETTE.SNAP_GRID` | Snap to Grid | Toggle grid snapping | required | official-overview | M3 | research |
| `DESIGN.SKETCH.PALETTE.CONSTRUCTION` | Construction | Toggle selected or newly created reference geometry | required | official-overview | M3 | research |
| `DESIGN.SKETCH.PALETTE.AUTO_PROJECT` | Auto Project Edges | Automatically create eligible linked projections | required | official-overview | M3 | research |
| `DESIGN.SKETCH.PALETTE.LOOK_AT` | Look At | Align the camera normal to the sketch plane | required | official-overview | M3 | research |

## Design: solid modeling

### Create and primitive commands

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `DESIGN.SOLID.CREATE.NEW_COMPONENT` | New Component | Create and activate a component node | required | official-overview | M7 | research |
| `DESIGN.SOLID.CREATE.SKETCH` | Create Sketch | Start a sketch on an origin plane, construction plane, or supported planar face | required | official | M3 | specified |
| `DESIGN.SOLID.CREATE.EXTRUDE` | Extrude | Add, cut, intersect, or create a body by linear profile sweep | required | official | M4 | research |
| `DESIGN.SOLID.CREATE.REVOLVE` | Revolve | Add, cut, intersect, or create a body by revolving profiles around an axis | required | official | M5 | research |
| `DESIGN.SOLID.CREATE.SWEEP` | Sweep | Sweep a profile along a path with supported guides | required | official | M8 | research |
| `DESIGN.SOLID.CREATE.LOFT` | Loft | Blend through ordered profiles with supported rails or centerline | required | official | M8 | research |
| `DESIGN.SOLID.CREATE.RIB` | Rib | Create a thin feature parallel to an open sketch profile | required | official | M8 | research |
| `DESIGN.SOLID.CREATE.WEB` | Web | Create a thin feature perpendicular to an open sketch profile | required | official | M8 | research |
| `DESIGN.SOLID.CREATE.EMBOSS` | Emboss | Raise or recess profiles relative to selected faces | required | official | M8 | research |
| `DESIGN.SOLID.CREATE.HOLE` | Hole | Create standard, counterbore, countersink, or tapped parametric holes | required | official-overview | M5 | research |
| `DESIGN.SOLID.CREATE.THREAD` | Thread | Add cosmetic or modeled threads using versioned standards data | required | official-overview | M8 | research |
| `DESIGN.SOLID.CREATE.BOX` | Box | Create a parametric solid box | required | official | M2 | research |
| `DESIGN.SOLID.CREATE.CYLINDER` | Cylinder | Create a parametric solid cylinder | required | official | M2 | research |
| `DESIGN.SOLID.CREATE.SPHERE` | Sphere | Create a parametric solid sphere | required | official | M5 | research |
| `DESIGN.SOLID.CREATE.TORUS` | Torus | Create a parametric solid torus | required | official | M8 | research |
| `DESIGN.SOLID.CREATE.COIL` | Coil | Create a helical solid or cut from pitch/revolution and section controls | required | official | M8 | research |
| `DESIGN.SOLID.CREATE.PIPE` | Pipe | Sweep a solid or hollow circular section along a path | required | official | M8 | research |
| `DESIGN.SOLID.CREATE.THICKEN` | Thicken | Offset a surface into a constant-thickness solid | required | official-overview | M8 | research |
| `DESIGN.SOLID.CREATE.BOUNDARY_FILL` | Boundary Fill | Select cells bounded by bodies and surfaces to create solids or surfaces | required | research-needed | M8 | research |
| `DESIGN.SOLID.CREATE.FORM` | Create Form | Enter the Form environment and convert accepted results back to B-Rep | required | official-overview | M8 | research |

All six primitive commands must expose `Join`, `Cut`, `Intersect`, `New Body`,
and `New Component` where geometrically applicable.

### Modify commands

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `DESIGN.SOLID.MODIFY.PRESS_PULL` | Press Pull | Dispatch by selection to face offset or compatible feature edit | required | official | M5 | research |
| `DESIGN.SOLID.MODIFY.FILLET` | Fillet | Constant, variable, and supported full-round edge blends | required | official | M5 | research |
| `DESIGN.SOLID.MODIFY.CHAMFER` | Chamfer | Equal-distance, two-distance, and distance-angle edge bevels | required | official | M5 | research |
| `DESIGN.SOLID.MODIFY.SHELL` | Shell | Hollow a body with removable faces and inside/outside/both offsets | required | official | M5 | research |
| `DESIGN.SOLID.MODIFY.DRAFT` | Draft | Apply fixed- or parting-line face draft | required | official | M8 | research |
| `DESIGN.SOLID.MODIFY.SCALE` | Scale | Uniform or non-uniform body scaling around a pivot | required | official | M8 | research |
| `DESIGN.SOLID.MODIFY.COMBINE` | Combine | Join, cut, or intersect target and tool bodies | required | official | M5 | research |
| `DESIGN.SOLID.MODIFY.OFFSET_FACE` | Offset Face | Offset selected faces while preserving supported adjacency | required | official | M8 | research |
| `DESIGN.SOLID.MODIFY.REPLACE_FACE` | Replace Face | Trim or extend a body to replacement faces | required | official | M8 | research |
| `DESIGN.SOLID.MODIFY.SPLIT_FACE` | Split Face | Partition faces with curves, planes, or surfaces | required | official | M8 | research |
| `DESIGN.SOLID.MODIFY.SPLIT_BODY` | Split Body | Partition a body with a plane, face, surface, or body | required | official | M8 | research |
| `DESIGN.SOLID.MODIFY.SILHOUETTE_SPLIT` | Silhouette Split | Split a body at its silhouette from a direction | required | official | M8 | research |
| `DESIGN.SOLID.MODIFY.ALIGN` | Align | Align objects from point, line, plane, circle, or coordinate-system references | required | official | M7 | research |
| `DESIGN.SOLID.MODIFY.MOVE_COPY` | Move/Copy | Translate, rotate, point-to-point move, or copy selected objects | required | research-needed | M5 | research |
| `DESIGN.SOLID.MODIFY.DELETE` | Delete | Delete supported bodies, faces, features, and components transactionally | required | research-needed | M4 | research |
| `DESIGN.SOLID.MODIFY.CHANGE_PARAMETERS` | Change Parameters | Edit named/user/model parameters and expressions | required | research-needed | M4 | research |
| `DESIGN.SOLID.MODIFY.REMOVE_FEATURES` | Remove Features | Defeature selected detail with explicit diagnostics | required | research-needed | M8 | research |

### Patterns and mirror

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `DESIGN.SOLID.PATTERN.MIRROR` | Mirror | Mirror faces, bodies, features, components, or construction geometry | required | official | M5 | research |
| `DESIGN.SOLID.PATTERN.RECTANGULAR` | Rectangular Pattern | Pattern supported objects along one or two linear directions | required | official | M5 | research |
| `DESIGN.SOLID.PATTERN.CIRCULAR` | Circular Pattern | Pattern supported objects around an axis | required | official | M5 | research |
| `DESIGN.SOLID.PATTERN.PATH` | Pattern on Path | Pattern supported objects along a path | required | official | M8 | research |
| `DESIGN.SOLID.PATTERN.GEOMETRIC` | Geometric Pattern | Vary size and distribution across a face | required | official | M8 | research |

## Design: surface modeling

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `DESIGN.SURFACE.CREATE.PATCH` | Patch | Fill a closed planar or 3D boundary with a surface | required | official | M8 | research |
| `DESIGN.SURFACE.CREATE.EXTRUDE` | Extrude | Extrude profiles, curves, planar faces, or supported text into surfaces | required | official | M8 | research |
| `DESIGN.SURFACE.CREATE.REVOLVE` | Revolve | Revolve profiles or planar faces into surfaces | required | official | M8 | research |
| `DESIGN.SURFACE.CREATE.SWEEP` | Sweep | Sweep a profile into a surface along a path | required | official | M8 | research |
| `DESIGN.SURFACE.CREATE.LOFT` | Loft | Create a surface transition across profiles | required | official | M8 | research |
| `DESIGN.SURFACE.CREATE.RULED` | Ruled | Create a ruled surface from selected edges with distance and angle | required | official | M8 | research |
| `DESIGN.SURFACE.CREATE.OFFSET` | Offset | Create an offset surface from selected faces | required | official | M8 | research |
| `DESIGN.SURFACE.MODIFY.PRESS_PULL` | Press Pull | Modify supported surface faces and edges by contextual offset/edit | required | official | M8 | research |
| `DESIGN.SURFACE.MODIFY.FILLET` | Fillet | Round supported surface-body edges | required | official | M8 | research |
| `DESIGN.SURFACE.MODIFY.CHAMFER` | Chamfer | Bevel supported surface-body edges | required | official | M8 | research |
| `DESIGN.SURFACE.MODIFY.TRIM` | Trim | Split intersecting surfaces and remove selected regions | required | official | M8 | research |
| `DESIGN.SURFACE.MODIFY.UNTRIM` | Untrim | Recover supported underlying surface regions and fill openings | required | official | M8 | research |
| `DESIGN.SURFACE.MODIFY.EXTEND` | Extend | Extend selected surface boundaries | required | official | M8 | research |
| `DESIGN.SURFACE.MODIFY.STITCH` | Stitch | Join surfaces into a quilt and close a solid when valid | required | official | M8 | research |
| `DESIGN.SURFACE.MODIFY.UNSTITCH` | Unstitch | Separate stitched faces into surface bodies | required | official | M8 | research |
| `DESIGN.SURFACE.MODIFY.MERGE` | Merge | Merge compatible surfaces within one body | required | official | M8 | research |
| `DESIGN.SURFACE.MODIFY.REVERSE_NORMAL` | Reverse Normal | Reverse a surface body's positive side | required | official | M8 | research |
| `DESIGN.SURFACE.MODIFY.SCALE` | Scale | Scale surface bodies | required | official | M8 | research |
| `DESIGN.SURFACE.MODIFY.SPLIT_FACE` | Split Face | Partition surface faces | required | official | M8 | research |
| `DESIGN.SURFACE.MODIFY.SPLIT_BODY` | Split Body | Partition surface bodies | required | official | M8 | research |

## Design: Form / T-Splines

### Create and display

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `DESIGN.FORM.CREATE.BOX` | Box | Create a face-count-controlled T-Spline box | required | official | M8 | research |
| `DESIGN.FORM.CREATE.PLANE` | Plane | Create a face-count-controlled T-Spline plane | required | official | M8 | research |
| `DESIGN.FORM.CREATE.CYLINDER` | Cylinder | Create a T-Spline cylinder | required | official | M8 | research |
| `DESIGN.FORM.CREATE.SPHERE` | Sphere | Create a T-Spline sphere | required | official | M8 | research |
| `DESIGN.FORM.CREATE.TORUS` | Torus | Create a T-Spline torus | required | official | M8 | research |
| `DESIGN.FORM.CREATE.QUADBALL` | Quadball | Create a cube-mapped T-Spline sphere | required | official | M8 | research |
| `DESIGN.FORM.CREATE.PIPE` | Pipe | Create a section-controlled T-Spline pipe along a path | required | official | M8 | research |
| `DESIGN.FORM.CREATE.FACE` | Face | Place vertices to create individual T-Spline faces | required | official | M8 | research |
| `DESIGN.FORM.CREATE.EXTRUDE` | Extrude | Extrude T-Spline faces or edges | required | research-needed | M8 | research |
| `DESIGN.FORM.CREATE.REVOLVE` | Revolve | Revolve T-Spline profiles | required | research-needed | M8 | research |
| `DESIGN.FORM.CREATE.SWEEP` | Sweep | Sweep T-Spline profiles | required | research-needed | M8 | research |
| `DESIGN.FORM.CREATE.LOFT` | Loft | Loft T-Spline profiles | required | research-needed | M8 | research |
| `DESIGN.FORM.DISPLAY.BOX` | Box Display | Show the control cage as boxes | required | secondary | M8 | research |
| `DESIGN.FORM.DISPLAY.CONTROL_FRAME` | Control Frame Display | Show cage edges and smooth surface | required | secondary | M8 | research |
| `DESIGN.FORM.DISPLAY.SMOOTH` | Smooth Display | Show the evaluated smooth surface | required | secondary | M8 | research |

### Modify

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `DESIGN.FORM.MODIFY.EDIT_FORM` | Edit Form | Transform T-Spline vertices, edges, and faces; support topology insertion gesture | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.EDIT_BY_CURVE` | Edit By Curve | Drive selected T-Spline edges with a curve | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.INSERT_EDGE` | Insert Edge | Insert and position edge loops with exact shape-preserving mode | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.SUBDIVIDE` | Subdivide | Divide selected faces | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.INSERT_POINT` | Insert Point | Insert topology between selected points with exact mode | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.MERGE_EDGE` | Merge Edge | Align and connect edge sets | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.BRIDGE` | Bridge | Connect opposing faces within or between bodies | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.FILL_HOLE` | Fill Hole | Fill an opening using supported star/reduced/collapse modes | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.ERASE_FILL` | Erase and Fill | Remove connected topology and refill the region | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.WELD_VERTICES` | Weld Vertices | Join selected or tolerance-matched vertices | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.UNWELD_EDGES` | UnWeld Edges | Disconnect selected edges or loops | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.CREASE` | Crease | Mark selected edges sharp | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.UNCREASE` | UnCrease | Restore smooth evaluation across creased edges | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.BEVEL_EDGE` | Bevel Edge | Replace an edge with adjacent edges and sharpness controls | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.SLIDE_EDGE` | Slide Edge | Move an edge along the control polygon | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.SMOOTH` | Smooth | Reduce local T-Spline irregularity | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.CYLINDRIFY` | Cylindrify | Fit selected topology to a smooth cylindrical shape | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.PULL` | Pull | Snap selected vertices to a face or surface | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.FLATTEN` | Flatten | Project vertices to a best-fit or selected plane | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.STRAIGHTEN` | Straighten | Fit vertices to a line | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.MATCH` | Match | Align T-Spline boundaries to curves or B-Rep edges | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.INTERPOLATE` | Interpolate | Switch fitting between surface and control-point locations | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.THICKEN` | Thicken | Offset T-Spline geometry into a body with explicit self-intersection diagnostics | improved | official | M8 | research |
| `DESIGN.FORM.MODIFY.FREEZE` | Freeze | Prevent edits to selected topology | required | official | M8 | research |
| `DESIGN.FORM.MODIFY.UNFREEZE` | Unfreeze | Re-enable edits to frozen topology | required | official | M8 | research |

## Design: plastic features

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `DESIGN.PLASTIC.CREATE.BOSS` | Boss | Create a configurable fastener boss | required | official | M8 | research |
| `DESIGN.PLASTIC.CREATE.SNAP_FIT` | Snap Fit | Create automatic or manually placed cantilever snap fits | required | official | M8 | research |
| `DESIGN.PLASTIC.CREATE.LIP` | Lip | Create lip, groove, or combined lip-and-groove edge features | required | official | M8 | research |
| `DESIGN.PLASTIC.CREATE.REST` | Rest | Create a flat intersecting support from a closed profile | required | official | M8 | research |
| `DESIGN.PLASTIC.CREATE.RIB` | Rib | Create mold-aware thin reinforcement geometry | required | official | M8 | research |
| `DESIGN.PLASTIC.CREATE.WEB` | Web | Create mold-aware thin wall geometry | required | official | M8 | research |
| `DESIGN.PLASTIC.ANALYZE.DRAFT` | Draft Analysis | Evaluate mold pull direction and draft | required | official-overview | M8 | research |
| `DESIGN.PLASTIC.ANALYZE.ACCESSIBILITY` | Accessibility Analysis | Evaluate tool access along a direction | required | official-overview | M8 | research |
| `DESIGN.PLASTIC.ANALYZE.THICKNESS` | Thickness Analysis | Evaluate wall-thickness extrema and thresholds | required | official-overview | M8 | research |

## Assemble

### Commands and motion

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `ASSEMBLE.COMMAND.JOINT` | Joint | Position two components and assign a joint type from selected origins | required | official | M7 | research |
| `ASSEMBLE.COMMAND.AS_BUILT_JOINT` | As-Built Joint | Assign motion between already positioned components | required | official | M7 | research |
| `ASSEMBLE.COMMAND.JOINT_ORIGIN` | Joint Origin | Create a named point-axis-plane frame for joints | required | official | M7 | research |
| `ASSEMBLE.COMMAND.RIGID_GROUP` | Rigid Group | Lock multiple components into one rigid motion group | required | official-overview | M7 | research |
| `ASSEMBLE.COMMAND.DRIVE_JOINT` | Drive Joints | Animate joints through bounded values | required | official-overview | M7 | research |
| `ASSEMBLE.COMMAND.MOTION_LINK` | Motion Link | Couple two joint coordinates with a ratio or expression | required | official-overview | M7 | research |
| `ASSEMBLE.COMMAND.CONTACT_SET` | Enable Contact Sets | Enforce selected component contacts during motion | required | official-overview | M7 | research |
| `ASSEMBLE.COMMAND.CAPTURE_POSITION` | Capture Position | Record a component-position snapshot in history | required | official-overview | M7 | research |
| `ASSEMBLE.COMMAND.GROUND` | Ground Component | Fix a component to world coordinates | required | official-overview | M7 | research |

### Joint types

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `ASSEMBLE.JOINT.RIGID` | Rigid | Lock all six relative degrees of freedom | required | official | M7 | research |
| `ASSEMBLE.JOINT.REVOLUTE` | Revolute | Permit rotation around one axis | required | official | M7 | research |
| `ASSEMBLE.JOINT.SLIDER` | Slider | Permit translation along one axis | required | official | M7 | research |
| `ASSEMBLE.JOINT.CYLINDRICAL` | Cylindrical | Permit rotation and translation along one axis | required | official | M7 | research |
| `ASSEMBLE.JOINT.PIN_SLOT` | Pin-Slot | Permit one rotation and translation along a separate axis | required | official | M7 | research |
| `ASSEMBLE.JOINT.PLANAR` | Planar | Permit two in-plane translations and rotation about the plane normal | required | official | M7 | research |
| `ASSEMBLE.JOINT.BALL` | Ball | Permit three rotations around one origin | required | official | M7 | research |

There is no `Groove` joint in the verified seven-type reference set.

### Assembly constraints

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `ASSEMBLE.CONSTRAINT.ALIGN` | Align | Constrain selections as coplanar, coincident, or concentric | required | official | M7 | research |
| `ASSEMBLE.CONSTRAINT.ANGLE` | Angle | Constrain relative angle with optional reference axis | required | official | M7 | research |
| `ASSEMBLE.CONSTRAINT.CENTER` | Center | Center one component between two faces using supported modes | required | official | M7 | research |
| `ASSEMBLE.CONSTRAINT.TANGENT` | Tangent | Maintain tangency between selected component geometry | required | official | M7 | research |
| `ASSEMBLE.CONSTRAINT.SETS` | Constraint Sets | Edit multiple component constraints as one ordered set | required | official | M7 | research |
| `ASSEMBLE.CONSTRAINT.OFFSET_LIMITS` | Offset Limits | Bound applicable align and angle offsets | required | official | M7 | research |

## Construct

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `DESIGN.CONSTRUCT.UCS` | User Coordinate System | Create a named coordinate frame usable by modeling, assembly, and manufacturing | required | official | M7 | research |
| `DESIGN.CONSTRUCT.PLANE.OFFSET` | Offset Plane | Plane at a distance or through a target object | required | official | M3 | research |
| `DESIGN.CONSTRUCT.PLANE.ANGLE` | Plane at Angle | Plane rotated around a line or axis | required | official | M5 | research |
| `DESIGN.CONSTRUCT.PLANE.TANGENT` | Tangent Plane | Plane tangent to a cylinder or cone at an angular position | required | official | M5 | research |
| `DESIGN.CONSTRUCT.PLANE.MIDPLANE` | Midplane | Plane equidistant between two references | required | official | M5 | research |
| `DESIGN.CONSTRUCT.PLANE.ALONG_PATH` | Plane Along Path | Plane normal to a path at a proportional or physical distance | required | official | M8 | research |
| `DESIGN.CONSTRUCT.PLANE.THREE_POINTS` | Plane Through Three Points | Plane through three non-collinear points | required | official | M5 | research |
| `DESIGN.CONSTRUCT.PLANE.TWO_EDGES` | Plane Through Two Edges | Plane through two eligible lines or edges | required | official | M8 | research |
| `DESIGN.CONSTRUCT.PLANE.PERPENDICULAR` | Perpendicular Plane | Plane perpendicular to an edge at a point | required | research-needed | M8 | research |
| `DESIGN.CONSTRUCT.GEOMETRY.RESIZE` | Resize construction plane | Control display extent independently of geometry | improved | official | M7 | research |
| `DESIGN.CONSTRUCT.GEOMETRY.MIRROR` | Mirror construction geometry | Mirror supported planes, axes, and points | required | official | M8 | research |
| `DESIGN.CONSTRUCT.GEOMETRY.PATTERN` | Pattern construction geometry | Pattern supported planes, axes, and points | required | official | M8 | research |

### Construction axes and points

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `DESIGN.CONSTRUCT.AXIS.TWO_POINTS` | Axis Through Two Points | Axis through two points or vertices | required | official | M5 | research |
| `DESIGN.CONSTRUCT.AXIS.TWO_PLANES` | Axis Through Two Planes | Axis at the intersection of two non-parallel planes | required | official | M5 | research |
| `DESIGN.CONSTRUCT.AXIS.EDGE` | Axis Through Edge | Axis coincident with a straight edge or sketch line | required | official | M5 | research |
| `DESIGN.CONSTRUCT.AXIS.PERPENDICULAR_FACE_POINT` | Axis Perpendicular to Face at Point | Face-normal axis through a selected point | required | research-needed | M8 | research |
| `DESIGN.CONSTRUCT.AXIS.CYLINDER_SPHERE_TORUS` | Axis Through Cylinder/Sphere/Torus | Axis through the center of supported analytic faces | required | research-needed | M8 | research |
| `DESIGN.CONSTRUCT.POINT.VERTEX` | Point At Vertex | Point at a vertex, face center, edge point, or supported snap | required | official | M5 | research |
| `DESIGN.CONSTRUCT.POINT.TWO_EDGES` | Point Through Two Edges | Point at an intersection or extended intersection of two edges | required | official | M5 | research |
| `DESIGN.CONSTRUCT.POINT.THREE_PLANES` | Point Through Three Planes | Point at the intersection of three non-parallel planes | required | official | M5 | research |
| `DESIGN.CONSTRUCT.POINT.ALONG_PATH` | Point Along Path | Point at a proportional or physical distance along a path | required | official | M8 | research |
| `DESIGN.CONSTRUCT.POINT.EDGE_PLANE` | Point At Edge and Plane | Point at the intersection of an edge and plane | required | research-needed | M8 | research |
| `DESIGN.CONSTRUCT.POINT.SPHERE_CENTER` | Point At Center of Sphere | Point at the center of a spherical face | required | research-needed | M8 | research |

## Design: mesh

### Create, modify, and prepare

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `DESIGN.MESH.CREATE.INSERT` | Insert Mesh | Import STL, OBJ, or 3MF as a mesh body | required | official | M8 | research |
| `DESIGN.MESH.CREATE.TESSELLATE` | Tessellate | Derive a mesh and face groups from solid or surface bodies | required | official | M8 | research |
| `DESIGN.MESH.CREATE.SECTION_SKETCH` | Create Mesh Section Sketch | Create sketch cross-sections from a mesh and plane | required | official | M8 | research |
| `DESIGN.MESH.MODE.DIRECT_EDIT` | Direct Edit | Enter face-level direct mesh editing mode | required | official | M8 | research |
| `DESIGN.MESH.MODIFY.REMESH` | Remesh | Regenerate face distribution with controlled density | required | official | M8 | research |
| `DESIGN.MESH.MODIFY.REDUCE` | Reduce | Reduce mesh face count under geometric error controls | required | official | M8 | research |
| `DESIGN.MESH.MODIFY.PLANE_CUT` | Plane Cut | Cut a mesh with a plane and control retained regions/fill | required | official | M8 | research |
| `DESIGN.MESH.MODIFY.SHELL` | Shell | Hollow a mesh to a wall thickness | required | official | M8 | research |
| `DESIGN.MESH.MODIFY.COMBINE` | Combine | Combine mesh bodies | required | official | M8 | research |
| `DESIGN.MESH.MODIFY.SMOOTH` | Smooth | Smooth selected mesh regions | required | official | M8 | research |
| `DESIGN.MESH.MODIFY.REVERSE_NORMAL` | Reverse Normal | Reverse selected mesh normals | required | official | M8 | research |
| `DESIGN.MESH.MODIFY.ERASE_FILL` | Erase And Fill | Remove selected faces and fill the opening | required | official | M8 | research |
| `DESIGN.MESH.MODIFY.ALIGN` | Mesh Align | Align mesh bodies to reference geometry | required | official | M8 | research |
| `DESIGN.MESH.MODIFY.TEXTURE_EXTRUDE` | Texture Extrude | Displace/extrude mesh faces from an image map | required | official | M8 | research |
| `DESIGN.MESH.MODIFY.SEPARATE` | Separate | Split disconnected regions into separate mesh bodies | required | official | M8 | research |
| `DESIGN.MESH.MODIFY.MOVE_COPY` | Move/Copy | Transform or copy mesh bodies | required | official | M8 | research |
| `DESIGN.MESH.MODIFY.SCALE` | Scale Mesh | Scale mesh bodies | required | official | M8 | research |
| `DESIGN.MESH.MODIFY.DELETE` | Delete | Delete selected mesh objects transactionally | required | official | M8 | research |
| `DESIGN.MESH.CONVERT.FACETED` | Convert Mesh: Faceted | Convert each triangle into B-Rep topology with explicit complexity diagnostics | required | official | M8 | research |
| `DESIGN.MESH.CONVERT.PRISMATIC` | Convert Mesh: Prismatic | Recognize analytic/prismatic regions and convert to B-Rep | required | official | M8 | research |
| `DESIGN.MESH.CONVERT.ORGANIC` | Convert Mesh: Organic | Convert supported smooth meshes through the Form/T-Spline pipeline | required | official | M8 | research |
| `DESIGN.MESH.PREPARE.REPAIR` | Repair | Detect and explicitly repair selected holes, intersections, orientation, and manifold defects | improved | official | M8 | research |
| `DESIGN.MESH.PREPARE.GENERATE_FACE_GROUPS` | Generate Face Groups | Group faces from normal and size criteria | required | official | M8 | research |
| `DESIGN.MESH.PREPARE.COMBINE_FACE_GROUPS` | Combine Face Groups | Merge adjacent face groups | required | official | M8 | research |
| `DESIGN.MESH.PREPARE.CREATE_FACE_GROUP` | Create Face Group | Group manually selected mesh faces | required | official | M8 | research |
| `DESIGN.MESH.PROPERTIES.PHYSICAL_MATERIAL` | Physical Material | Assign engineering material metadata | required | official | M8 | research |
| `DESIGN.MESH.PROPERTIES.APPEARANCE` | Appearance | Assign rendering appearance metadata | required | official | M8 | research |

## Design: sheet metal

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `DESIGN.SHEET_METAL.CREATE.BASE_FLANGE` | Base Flange | Create a founding sheet-metal body from a closed profile | required | official | M8 | research |
| `DESIGN.SHEET_METAL.CREATE.EDGE_FLANGE` | Edge Flange | Extend selected edges into angle/height-controlled flanges | required | official | M8 | research |
| `DESIGN.SHEET_METAL.CREATE.CONTOUR_FLANGE` | Contour Flange | Create a possibly multi-bend flange from an open profile | required | official | M8 | research |
| `DESIGN.SHEET_METAL.CREATE.HEM` | Hem | Create flat, open, teardrop, rolled, and double hems | required | official | M8 | research |
| `DESIGN.SHEET_METAL.CREATE.LOFTED_FLANGE` | Lofted Flange | Connect compatible open or closed profiles with a developable transition | required | official | M8 | research |
| `DESIGN.SHEET_METAL.MODIFY.CORNER_CLOSURE` | Corner Closure | Miter and close adjacent planar flanges with manufacturing gaps | required | official | M8 | research |
| `DESIGN.SHEET_METAL.MODIFY.BEND` | Bend | Add a bend along a sketch line on a flat face | required | research-needed | M8 | research |
| `DESIGN.SHEET_METAL.MODIFY.CUT` | Cut | Remove material while preserving sheet-metal semantics | required | research-needed | M8 | research |
| `DESIGN.SHEET_METAL.MODIFY.CONVERT` | Convert to Sheet Metal | Convert supported thin solids into ruled sheet-metal bodies | required | research-needed | M8 | research |
| `DESIGN.SHEET_METAL.MODIFY.UNFOLD` | Unfold | Temporarily flatten selected bends for cross-bend editing | required | official | M8 | research |
| `DESIGN.SHEET_METAL.MODIFY.REFOLD` | Refold | Restore an unfolded body and propagate edits | required | official-overview | M8 | research |
| `DESIGN.SHEET_METAL.OUTPUT.FLAT_PATTERN` | Flat Pattern | Create a derived manufacturing flat representation | required | official | M8 | research |
| `DESIGN.SHEET_METAL.RULES.MANAGE` | Sheet Metal Rules | Version thickness, bend radius, K-factor, relief, and unfold rules | improved | research-needed | M8 | research |
| `DESIGN.SHEET_METAL.OUTPUT.DXF` | Export Flat Pattern DXF | Export contours, bend lines, and metadata from the flat pattern | required | official-overview | M8 | research |

## Insert and utilities

### Insert

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `DESIGN.INSERT.COMPONENT` | Insert Component | Insert an external associatively linked design component | required | official | M7 | research |
| `DESIGN.INSERT.FASTENER` | Insert Fastener | Search a fastener library and place hardware on compatible geometry | improved | official | M7 | research |
| `DESIGN.INSERT.DERIVE` | Insert Derive | Derive selected components, bodies, sketches, construction geometry, flat patterns, and parameters | required | official | M7 | research |
| `DESIGN.INSERT.DECAL` | Decal | Place an image across selected faces | required | official | M8 | research |
| `DESIGN.INSERT.CANVAS` | Canvas | Place and calibrate a reference image on a plane | required | official | M3 | research |
| `DESIGN.INSERT.SVG` | Insert SVG | Import SVG geometry into a sketch | required | official | M3 | research |
| `DESIGN.INSERT.DXF` | Insert DXF | Import DXF geometry into one or more sketches | required | official | M3 | research |
| `DESIGN.INSERT.MCMASTER_CARR` | Insert McMaster-Carr Component | Search the external catalog and import selected neutral CAD | required | official | M7 | research |
| `DESIGN.INSERT.MESH` | Insert Mesh | Import supported polygon-mesh formats | required | official | M8 | research |
| `DESIGN.INSERT.MANUFACTURER_PART` | Insert a manufacturer part | Search supplier catalogs for configurable CAD parts | required | official | M7 | research |
| `DESIGN.INSERT.TRACEPARTS` | Insert TraceParts Supplier Components | Search and import TraceParts catalog models | required | official | M7 | research |

### Utilities

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `DESIGN.UTILITIES.CHANGE_PARAMETERS` | Change Parameters | Create and edit user, model, and derived parameters | required | research-needed | M4 | research |
| `DESIGN.UTILITIES.PHYSICAL_MATERIAL` | Physical Material | Assign engineering materials used by mass and simulation | required | research-needed | M7 | research |
| `DESIGN.UTILITIES.APPEARANCE` | Appearance | Assign visual materials without changing physical properties | required | research-needed | M7 | research |
| `DESIGN.UTILITIES.MAKE_COMPONENTS` | Make Components | Convert selected bodies into components | required | research-needed | M7 | research |
| `DESIGN.UTILITIES.SCRIPTS_ADD_INS` | Scripts and Add-Ins | Run and manage automation packages | improved | official-overview | M9 | research |
| `DESIGN.UTILITIES.COMPUTE_ALL` | Compute All | Recompute the complete feature dependency graph | improved | research-needed | M4 | research |

## Inspect and analysis

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `DESIGN.INSPECT.MEASURE` | Measure | Report positions, lengths, areas, angles, and minimum distances | required | official | M1 | specified |
| `DESIGN.INSPECT.INTERFERENCE` | Interference | Detect and report intersecting solid bodies/components and volumes | required | official | M7 | research |
| `DESIGN.INSPECT.CURVATURE_COMB` | Curvature Comb Analysis | Plot curvature magnitude and continuity along edges | required | official | M8 | research |
| `DESIGN.INSPECT.ZEBRA` | Zebra Analysis | Display reflection stripes for continuity inspection | required | official | M8 | research |
| `DESIGN.INSPECT.DRAFT` | Draft Analysis | Color faces by angle to a pull direction | required | official | M8 | research |
| `DESIGN.INSPECT.FASTENER_STACK` | Fastener Stack Analysis | Validate fastener-stack geometry and report issues | required | official | M8 | research |
| `DESIGN.INSPECT.CURVATURE_MAP` | Curvature Map Analysis | Display Gaussian or principal curvature maps | required | official | M8 | research |
| `DESIGN.INSPECT.ISOCURVE` | Isocurve Analysis | Display UV isocurves and curvature combs on surfaces | required | official | M8 | research |
| `DESIGN.INSPECT.ACCESSIBILITY` | Accessibility Analysis | Classify faces by visibility from a direction | required | official | M8 | research |
| `DESIGN.INSPECT.MINIMUM_RADIUS` | Minimum Radius Analysis | Locate concave regions below a tool/radius threshold | required | official | M8 | research |
| `DESIGN.INSPECT.SECTION` | Section Analysis | Persist an interactive cut-plane analysis in the document | required | official | M2 | research |
| `DESIGN.INSPECT.CENTER_OF_MASS` | Center of Mass | Display and measure the computed mass center | required | official | M7 | research |
| `DESIGN.INSPECT.COMPONENT_COLORS` | Display Component Colors | Apply deterministic per-component viewport colors | required | official | M7 | research |
| `DATA.SEARCH.SIMILAR_COMPONENTS` | Find Similar Components | Search indexed projects for geometrically similar components | improved | official | M9 | research |

## Drawing

### Views, geometry, and modification

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `DRAWING.VIEW.BASE` | Base View | Place the parent design, component, animation, or sheet-metal view | required | official | M8 | research |
| `DRAWING.VIEW.PROJECTED` | Projected View | Place one of eight linked orthographic/isometric projections | required | official | M8 | research |
| `DRAWING.VIEW.SECTION` | Section View | Derive an interior view from a cutting line | required | official | M8 | research |
| `DRAWING.VIEW.DETAIL` | Detail View | Place an enlarged linked region | required | official | M8 | research |
| `DRAWING.VIEW.BREAK` | Break View | Foreshorten a long view by removing a region | required | official | M8 | research |
| `DRAWING.CREATE.SKETCH` | Create Sketch | Create drawing-sheet sketch geometry | required | official | M8 | research |
| `DRAWING.MODIFY.MOVE` | Move | Move supported sheet objects | required | official | M8 | research |
| `DRAWING.MODIFY.ROTATE` | Rotate | Rotate supported sheet objects | required | official | M8 | research |
| `DRAWING.MODIFY.DELETE` | Delete | Delete supported views and annotations transactionally | required | official | M8 | research |
| `DRAWING.GEOMETRY.CENTER_LINE` | Center Line | Add associative center lines | required | official | M8 | research |
| `DRAWING.GEOMETRY.CENTER_MARK` | Center Mark | Add associative circular center marks | required | official | M8 | research |
| `DRAWING.GEOMETRY.CENTER_MARK_PATTERN` | Center Mark Pattern | Add associative center marks to circular arrays | required | official | M8 | research |
| `DRAWING.GEOMETRY.EDGE_EXTENSION` | Edge Extension | Add drawing edge extensions | required | official | M8 | research |

### Dimensions and annotations

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `DRAWING.DIMENSION.GENERAL` | Dimension | Infer linear, aligned, angular, radius, or diameter dimension | required | official | M8 | research |
| `DRAWING.DIMENSION.AUTO` | Auto Dimension | Generate dimensions from a selectable strategy | improved | official | M8 | research |
| `DRAWING.DIMENSION.TIDY_UP` | Tidy Up | Rearrange dimensions for legibility | improved | official | M8 | research |
| `DRAWING.DIMENSION.ORDINATE` | Ordinate Dimension | Measure X/Y offsets from a datum origin | required | official | M8 | research |
| `DRAWING.DIMENSION.LINEAR` | Linear Dimension | Measure horizontal or vertical distance | required | official | M8 | research |
| `DRAWING.DIMENSION.ALIGNED` | Aligned Dimension | Measure distance parallel to selected geometry | required | official | M8 | research |
| `DRAWING.DIMENSION.ANGULAR` | Angular Dimension | Measure an angle | required | official | M8 | research |
| `DRAWING.DIMENSION.RADIUS` | Radius Dimension | Measure a radius | required | official | M8 | research |
| `DRAWING.DIMENSION.DIAMETER` | Diameter Dimension | Measure a diameter | required | official | M8 | research |
| `DRAWING.DIMENSION.JOGGED_RADIAL` | Jogged Radial Dimension | Place a foreshortened radius with overridden center display | required | official | M8 | research |
| `DRAWING.DIMENSION.ARC_LENGTH` | Arc Length Dimension | Measure full or partial arc length | required | official | M8 | research |
| `DRAWING.DIMENSION.CURVE_MIN_MAX` | Curve Min Max Dimension | Measure minimum or maximum curve distance | required | official | M8 | research |
| `DRAWING.DIMENSION.BASELINE` | Baseline Dimension | Create dimensions from a common baseline | required | official | M8 | research |
| `DRAWING.DIMENSION.CHAIN` | Chain Dimension | Create sequential dimensions | required | official | M8 | research |
| `DRAWING.DIMENSION.ARRANGE` | Arrange Dimensions | Stack or align dimension sets | required | official | M8 | research |
| `DRAWING.DIMENSION.MATCH` | Match Dimension | Copy dimension formatting | required | official | M8 | research |
| `DRAWING.DIMENSION.FLIP_ARROWS` | Flip Arrows | Reverse dimension-arrow direction | required | official | M8 | research |
| `DRAWING.DIMENSION.BREAK` | Dimension Break | Add intersection breaks in dimensions/leaders | required | official | M8 | research |
| `DRAWING.TEXT.TEXT` | Text | Add sheet text | required | official | M8 | research |
| `DRAWING.TEXT.LEADER` | Leader | Add leader-connected annotations | required | official | M8 | research |
| `DRAWING.SYMBOL.SURFACE_TEXTURE` | Surface Texture | Add ASME/ISO surface-finish symbols | required | official | M8 | research |
| `DRAWING.SYMBOL.FEATURE_CONTROL_FRAME` | Feature Control Frame | Add GD&T feature-control frames | required | official | M8 | research |
| `DRAWING.SYMBOL.DATUM_IDENTIFIER` | Datum Identifier | Add datum identifiers | required | official | M8 | research |
| `DRAWING.SYMBOL.WELDING` | Welding | Add standards-aware welding symbols | required | official | M8 | research |
| `DRAWING.SYMBOL.TAPER_SLOPE` | Taper and Slope | Add taper/slope symbols | required | official | M8 | research |
| `DRAWING.SYMBOL.EDGE` | Edge Symbol | Add edge-processing requirements | required | official | M8 | research |
| `DRAWING.SYMBOL.SKETCH` | Sketch Symbol | Create and insert reusable custom symbols | required | official | M8 | research |
| `DRAWING.TABLE.PARTS_LIST` | Table | Create assembly parts lists/BOMs | required | official | M8 | research |
| `DRAWING.TABLE.BALLOON` | Balloon | Link item balloons to parts lists | required | official | M8 | research |
| `DRAWING.TABLE.BEND_IDENTIFIER` | Bend Identifier | Identify sheet-metal bends | required | official | M8 | research |
| `DRAWING.TABLE.RENUMBER` | Renumber | Synchronize balloon and parts-list numbering | required | official | M8 | research |
| `DRAWING.TABLE.ALIGN_BALLOON` | Align Balloon | Align balloon groups | required | official | M8 | research |
| `DRAWING.EXPORT.PDF` | Export PDF | Export sheets as PDF | required | official | M8 | research |
| `DRAWING.EXPORT.DWG` | Export DWG | Export drawings as DWG | required | official | M8 | research |
| `DRAWING.EXPORT.DXF` | Export Sheet as DXF | Export a sheet as DXF | required | official | M8 | research |
| `DRAWING.EXPORT.CSV` | Export CSV | Export table data as CSV | required | official | M8 | research |

## Render and animation

### Render

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `RENDER.SETUP.APPEARANCE` | Appearance | Assign opaque, transparent, metal, layered, and solid-wood appearances | required | official | M8 | research |
| `RENDER.SETUP.SCENE` | Scene Settings | Configure HDRI lighting, brightness, background, ground, and camera | required | official | M8 | research |
| `RENDER.SETUP.DECAL` | Decal | Place images on selected faces | required | official | M8 | research |
| `RENDER.SETUP.TEXTURE_MAPPING` | Texture Mapping | Control texture orientation and mapping | required | official | M8 | research |
| `RENDER.OUTPUT.RENDER` | Render | Produce deterministic local renders and optional remote renders | improved | official-overview | M8 | research |
| `RENDER.OUTPUT.GALLERY` | Render Gallery | Browse, compare, and export render results | required | research-needed | M8 | research |

### Animation

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `ANIMATION.TIMELINE.STORYBOARD` | Storyboard | Create multiple named action timelines | required | official | M8 | research |
| `ANIMATION.EXPLODE.MANUAL` | Manual Explode | Move components along an axis at timeline positions | required | official | M8 | research |
| `ANIMATION.EXPLODE.AUTO_ALL` | Auto Explode: All Levels | Generate one-step or sequential multi-level explosions | required | official | M8 | research |
| `ANIMATION.ACTION.TRANSFORM` | Transform | Keyframe component translation and rotation | required | research-needed | M8 | research |
| `ANIMATION.ACTION.CALLOUT` | Callout | Attach timed text callouts to components | required | research-needed | M8 | research |
| `ANIMATION.ACTION.VIEW` | View | Keyframe camera states | required | research-needed | M8 | research |
| `ANIMATION.OUTPUT.PUBLISH` | Publish | Export storyboard video | required | research-needed | M8 | research |
| `ANIMATION.TIMELINE.EDIT` | Animation Timeline | Move and resize actions in time | required | official-overview | M8 | research |

## Simulation and generative design

### Simulation studies

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `SIMULATION.STUDY.STATIC_STRESS` | Static Stress | Linear small-displacement structural analysis | required | official | M9 | research |
| `SIMULATION.STUDY.NONLINEAR_STATIC_STRESS` | Nonlinear Static Stress | Nonlinear material, contact, and large-deformation analysis | required | official | M9 | research |
| `SIMULATION.STUDY.EVENT_QUASI_STATIC` | Quasi-static Event Simulation | Ramped nonlinear event analysis | required | official | M9 | research |
| `SIMULATION.STUDY.EVENT_DYNAMIC` | Dynamic Event Simulation | Time-dependent impact/event analysis | required | official | M9 | research |
| `SIMULATION.STUDY.MODAL_FREQUENCIES` | Modal Frequencies | Natural modes and mass participation | required | official | M9 | research |
| `SIMULATION.STUDY.SHAPE_OPTIMIZATION` | Shape Optimization | Remove material under structural objectives | required | official | M9 | research |
| `SIMULATION.STUDY.STRUCTURAL_BUCKLING` | Structural Buckling | Compute critical load multipliers and mode shapes | required | official | M9 | research |
| `SIMULATION.STUDY.THERMAL` | Thermal | Steady-state temperature and heat-flow analysis | required | official | M9 | research |
| `SIMULATION.STUDY.THERMAL_STRESS` | Thermal Stress | Coupled temperature-expansion structural analysis | required | official | M9 | research |
| `SIMULATION.STUDY.PLASTIC_INJECTION` | Plastic Injection Molding | Predict filling, visual defects, and warpage | required | official | M9 | research |
| `SIMULATION.STUDY.ELECTRONICS_COOLING` | Electronics Cooling | Analyze PCB heat loads, sinks, fans, and component risk | deferred | official | M9 | research |

Electronics Cooling is a reference **Tech Preview**, so Amphion records it but
does not promise behavioral parity until the reference behavior stabilizes.

### Generative design

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `GENERATIVE.STUDY.CREATE` | New Generative Study | Define a versioned generative design study | required | official-overview | M9 | research |
| `GENERATIVE.GEOMETRY.PRESERVE` | Preserve Geometry | Mark required material regions | required | official-overview | M9 | research |
| `GENERATIVE.GEOMETRY.OBSTACLE` | Obstacle Geometry | Mark forbidden material regions | required | official-overview | M9 | research |
| `GENERATIVE.CONSTRAINT.STRUCTURAL` | Structural Constraints | Apply supports and motion constraints | required | official-overview | M9 | research |
| `GENERATIVE.LOAD.STRUCTURAL` | Structural Loads | Apply forces, pressures, moments, and bearing loads | required | official-overview | M9 | research |
| `GENERATIVE.OBJECTIVE.MANUFACTURING` | Manufacturing Constraints | Constrain outcomes for additive, milling, casting, or unrestricted processes | required | official-overview | M9 | research |
| `GENERATIVE.EXPLORE.OUTCOMES` | Explore Outcomes | Compare, filter, inspect, and promote generated outcomes | required | official-overview | M9 | research |

## Manufacture and additive

### Milling and drilling

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `MANUFACTURE.MILLING.2D.ADAPTIVE` | 2D Adaptive Clearing | Rough pockets with controlled tool engagement | required | official | M9 | research |
| `MANUFACTURE.MILLING.2D.CONTOUR` | 2D Contour | Finish selected boundary chains | required | official-overview | M9 | research |
| `MANUFACTURE.MILLING.2D.POCKET` | 2D Pocket | Clear closed regions by depth | required | official-overview | M9 | research |
| `MANUFACTURE.MILLING.2D.SLOT` | Slot | Machine slots from centerlines | required | official-overview | M9 | research |
| `MANUFACTURE.MILLING.2D.TRACE` | Trace | Follow selected chains | required | official-overview | M9 | research |
| `MANUFACTURE.MILLING.2D.THREAD` | Thread | Helically mill internal or external threads | required | official-overview | M9 | research |
| `MANUFACTURE.MILLING.2D.CIRCULAR` | Circular | Machine circular holes or bosses | required | official-overview | M9 | research |
| `MANUFACTURE.MILLING.2D.ENGRAVE` | Engrave | Machine fine vector geometry and text | required | official-overview | M9 | research |
| `MANUFACTURE.MILLING.2D.BORE` | Bore | Finish holes with helical interpolation | required | official-overview | M9 | research |
| `MANUFACTURE.DRILLING.DRILL` | Drill | Generate sorted drilling cycles | required | official | M9 | research |
| `MANUFACTURE.DRILLING.HOLE_RECOGNITION` | Hole Recognition | Recognize multi-diameter cylindrical features for drilling | required | official | M9 | research |
| `MANUFACTURE.MILLING.3D.ADAPTIVE` | Adaptive Clearing | Rough 3D stock with rest machining | required | official | M9 | research |
| `MANUFACTURE.MILLING.3D.CONTOUR` | Contour | Finish constant-Z regions | required | official | M9 | research |
| `MANUFACTURE.MILLING.3D.PARALLEL` | Parallel | Finish with parallel passes | required | official | M9 | research |
| `MANUFACTURE.MILLING.3D.SCALLOP` | Scallop | Finish with constant stepover | required | official | M9 | research |
| `MANUFACTURE.MILLING.3D.PENCIL` | Pencil | Finish internal corners and tight fillets | required | official | M9 | research |
| `MANUFACTURE.MILLING.3D.HORIZONTAL` | Horizontal | Finish flat and near-horizontal regions | required | official | M9 | research |
| `MANUFACTURE.MILLING.3D.RADIAL` | Radial | Generate radial finishing passes | required | official | M9 | research |
| `MANUFACTURE.MILLING.3D.SPIRAL` | Spiral | Generate continuous spiral finishing passes | required | official | M9 | research |
| `MANUFACTURE.MILLING.3D.POCKET` | Pocket Clearing | Rough enclosed 3D pockets | required | official | M9 | research |
| `MANUFACTURE.MILLING.3D.MORPHED_SPIRAL` | Morphed Spiral | Morph a spiral between boundaries | required | official | M9 | research |
| `MANUFACTURE.MILLING.3D.RAMP` | Ramp | Finish steep regions with ramped passes | required | official | M9 | research |
| `MANUFACTURE.MILLING.3D.MORPH` | Morph | Morph passes between edge chains | required | official | M9 | research |
| `MANUFACTURE.MILLING.3D.PROJECT` | Project | Project curves onto model surfaces as toolpaths | required | official | M9 | research |
| `MANUFACTURE.MILLING.3D.FLOW` | Flow | Follow surface UV flow | required | official | M9 | research |
| `MANUFACTURE.MILLING.3D.STEEP_SHALLOW` | Steep and Shallow | Combine steep and shallow finishing strategies | required | official-overview | M9 | research |

### Turning, machines, and output

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `MANUFACTURE.TURNING.PROFILE_ROUGHING` | Profile Roughing | Rough rotational profiles | required | research-needed | M9 | research |
| `MANUFACTURE.TURNING.PROFILE_FINISHING` | Profile Finishing | Finish rotational profiles | required | research-needed | M9 | research |
| `MANUFACTURE.TURNING.FACING` | Facing | Face rotational stock | required | research-needed | M9 | research |
| `MANUFACTURE.TURNING.GROOVING` | Grooving | Machine axial or radial grooves | required | research-needed | M9 | research |
| `MANUFACTURE.TURNING.PARTING` | Parting | Separate completed parts from stock | required | research-needed | M9 | research |
| `MANUFACTURE.TURNING.THREADING` | Threading | Cut rotational threads | required | research-needed | M9 | research |
| `MANUFACTURE.TURNING.BORING` | Boring | Machine internal rotational profiles | required | research-needed | M9 | research |
| `MANUFACTURE.TURNING.ADAPTIVE_ROUGHING` | Turning Adaptive Roughing | Rough rotational stock with controlled engagement | required | research-needed | M9 | research |
| `MANUFACTURE.OUTPUT.POST_PROCESS` | Post Process | Generate NC code through a selected post | required | official | M9 | research |
| `MANUFACTURE.LIBRARY.POST` | Post Library | Manage installed, personal, cloud, and community posts | required | official | M9 | research |
| `MANUFACTURE.LIBRARY.MACHINE` | Machine Library | Manage machine definitions and models | required | official | M9 | research |
| `MANUFACTURE.MACHINE.BUILDER` | Machine Builder | Create and edit kinematic machine definitions | required | official | M9 | research |
| `MANUFACTURE.MACHINE.SIMULATION` | Machine Simulation | Simulate machine motion and detect collisions | required | official | M9 | research |
| `MANUFACTURE.MODEL.MANUFACTURING` | Manufacturing Model | Maintain manufacturing-only geometry independently of design intent | improved | official | M9 | research |
| `MANUFACTURE.INSPECTION.PART_SETTING` | Part Setting | Probe stock/work offsets | required | official-overview | M9 | research |
| `MANUFACTURE.INSPECTION.PART_ALIGNMENT` | Part Alignment | Align machining coordinates from inspection | required | official-overview | M9 | research |
| `MANUFACTURE.INSPECTION.GEOMETRIC` | Geometric Inspection | Inspect analytic features and dimensions | required | official-overview | M9 | research |
| `MANUFACTURE.INSPECTION.SURFACE` | Surface Inspection | Compare measured surfaces with design geometry | required | official-overview | M9 | research |
| `MANUFACTURE.INSPECTION.MANUAL` | Manual Inspection | Record manually acquired measurements | required | official-overview | M9 | research |
| `MANUFACTURE.FABRICATION.WATERJET` | Water Jet | Generate 2D profile cutting paths | required | official-overview | M9 | research |
| `MANUFACTURE.FABRICATION.LASER` | Laser Cutting | Generate laser profile paths | required | official-overview | M9 | research |
| `MANUFACTURE.FABRICATION.NESTING` | Nesting | Arrange profiles on sheets with material and process constraints | required | official-overview | M9 | research |
| `MANUFACTURE.ADDITIVE.FFF` | FFF | Prepare and slice fused-filament builds | required | official | M9 | research |
| `MANUFACTURE.ADDITIVE.SLA_DLP` | SLA/DLP | Prepare vat-photopolymer builds | required | official | M9 | research |
| `MANUFACTURE.ADDITIVE.MJF` | MJF | Prepare multi-jet-fusion builds | required | official | M9 | research |
| `MANUFACTURE.ADDITIVE.SLS` | SLS | Prepare polymer powder-bed builds | required | official | M9 | research |
| `MANUFACTURE.ADDITIVE.MPBF` | MPBF | Prepare metal powder-bed builds | required | official | M9 | research |
| `MANUFACTURE.ADDITIVE.EBEAM` | eBeam | Prepare electron-beam builds | required | official | M9 | research |
| `MANUFACTURE.ADDITIVE.BINDER_JETTING` | Binder Jetting | Prepare binder-jet builds | required | official | M9 | research |
| `MANUFACTURE.ADDITIVE.DED` | Directed Energy Deposition | Prepare directed-energy-deposition builds | required | official | M9 | research |

## Electronics

Command-level reference documentation remains incomplete, so uncertain rows are
kept visible rather than silently omitted.

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `ELECTRONICS.LIBRARY.MANAGE` | Manage electronic component libraries | Create, version, search, import, and distribute symbols, footprints, devices, and 3D packages | improved | official-overview | M9 | research |
| `ELECTRONICS.SCHEMATIC.CREATE` | Create a schematic design | Create hierarchical sheets and place electrical devices | required | official-overview | M9 | research |
| `ELECTRONICS.SCHEMATIC.WIRE` | Wire | Connect pins into named electrical nets | required | official-overview | M9 | research |
| `ELECTRONICS.SCHEMATIC.POWER` | Power rails | Place supply symbols and power nets | required | official-overview | M9 | research |
| `ELECTRONICS.SCHEMATIC.ERC` | Electrical Rule Check | Validate connectivity and electrical pin rules | required | research-needed | M9 | research |
| `ELECTRONICS.PCB.OUTLINE` | PCB Shape | Define and synchronize board geometry with the mechanical model | improved | official-overview | M9 | research |
| `ELECTRONICS.PCB.PLACE` | Place Components | Place and arrange packages | required | official-overview | M9 | research |
| `ELECTRONICS.PCB.ROUTE_MANUAL` | Manual Routing | Route copper traces interactively | required | official | M9 | research |
| `ELECTRONICS.PCB.ROUTE_DIFFERENTIAL` | Differential Pair Routing | Route constrained differential pairs | required | research-needed | M9 | research |
| `ELECTRONICS.PCB.POLYGON` | Polygon Pour | Define and calculate copper regions | required | research-needed | M9 | research |
| `ELECTRONICS.PCB.VIA` | Via | Change routing layers through vias | required | research-needed | M9 | research |
| `ELECTRONICS.PCB.LAYER_STACK` | Layer Stack | Configure stackup, materials, and layer constraints | required | official-overview | M9 | research |
| `ELECTRONICS.PCB.DRC` | Design Rule Check | Validate board geometry and manufacturing constraints | required | research-needed | M9 | research |
| `ELECTRONICS.PCB.THREE_D` | 3D PCB | Synchronize board, packages, and enclosure for 3D inspection | improved | official-overview | M9 | research |
| `ELECTRONICS.DATA.BULK_EDIT` | Review and edit project data | Review and modify project records in bulk | required | official-overview | M9 | research |
| `ELECTRONICS.SYNC.SCHEMATIC_PCB` | Synchronizing designs | Maintain and compare schematic/PCB correspondence | improved | official | M9 | research |
| `ELECTRONICS.IMPORT.ALTIUM` | Altium Project Import | Import supported Altium projects with diagnostics | required | official | M9 | research |

## Data, collaboration, and automation

### File and neutral-data exchange

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `EXCHANGE.NATIVE.ARCHIVE` | F3D / F3Z | Open, save, and package complete Amphion documents with history, components, assets, and provenance | improved | product-decision | M2 | specified |
| `EXCHANGE.STEP.AP242.IMPORT` | STEP Import | Transactionally import the declared analytic AP242 subset without silent healing | improved | official-overview | M6 | specified |
| `EXCHANGE.STEP.AP242.EXPORT` | STEP Export | Deterministically export the declared analytic AP242 subset | improved | official-overview | M6 | specified |
| `EXCHANGE.STEP.AP242.ROUND_TRIP` | STEP Round Trip | Verify semantic B-Rep, units, orientation, p-curves, and tolerance across export/import | improved | product-decision | M6 | specified |
| `EXCHANGE.IGES.IMPORT` | IGES Import | Import supported wire, curve, and surface entities transactionally | required | official-overview | M8 | research |
| `EXCHANGE.IGES.EXPORT` | IGES Export | Export supported wire, curve, and surface entities | required | official-overview | M8 | research |
| `EXCHANGE.DXF.IMPORT` | DXF Import | Import supported 2D geometry into sketches and drawings | required | official | M3 | research |
| `EXCHANGE.DXF.EXPORT` | DXF Export | Export sketches, drawings, and flat patterns | required | official | M8 | research |
| `EXCHANGE.DWG.IMPORT` | DWG Import | Import supported drawing geometry and metadata | required | official-overview | M8 | research |
| `EXCHANGE.DWG.EXPORT` | DWG Export | Export supported drawings and metadata | required | official | M8 | research |
| `EXCHANGE.MESH.STL` | STL | Import and export binary/ASCII triangle meshes with explicit unit handling | improved | official-overview | M8 | research |
| `EXCHANGE.MESH.OBJ` | OBJ | Import and export polygon meshes with supported material references | required | official-overview | M8 | research |
| `EXCHANGE.MESH.THREE_MF` | 3MF | Import and export unit-aware additive packages | required | official-overview | M9 | research |
| `EXCHANGE.MESH.FBX` | FBX | Import and export supported visualization meshes and scene metadata | deferred | official-overview | M9 | research |
| `EXCHANGE.ACIS.SAT_SMT` | SAT / SMT | Import and export supported ACIS exchange data through a separately licensed adapter | deferred | official-overview | M9 | blocked |
| `EXCHANGE.SKETCHUP.SKP` | SKP | Import and export supported SketchUp geometry | deferred | official-overview | M9 | research |
| `EXCHANGE.INVENTOR.IPT_IAM` | IPT / IAM | Import supported Inventor parts and assemblies through a legal adapter | deferred | official-overview | M9 | blocked |
| `EXCHANGE.SOLIDWORKS.SLDPRT_SLDASM` | SLDPRT / SLDASM | Import supported SolidWorks parts and assemblies through a legal adapter | deferred | official-overview | M9 | blocked |
| `EXCHANGE.CATIA.CATPART_CATPRODUCT` | CATPart / CATProduct | Import supported CATIA parts and products through a legal adapter | deferred | official-overview | M9 | blocked |
| `EXCHANGE.USD.USDZ` | USD / USDZ | Import and export supported visualization scenes | required | official-overview | M9 | research |
| `EXCHANGE.SVG.IMPORT` | SVG Import | Import vector paths into sketches | required | official | M3 | research |

Proprietary native-format adapters are tracked rather than assumed. Their
implementation requires documented format rights or separately licensed SDKs;
no compatibility target authorizes copying proprietary code or bypassing
technical protection.

### Data and collaboration

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `DATA.LOCAL.PROJECT` | - | Store complete projects locally without mandatory cloud services | improved | product-decision | M2 | specified |
| `DATA.HUB.HUB` | Hub | Administer collaborative organization spaces | required | official | M9 | research |
| `DATA.HUB.PROJECT` | Project | Organize folders, designs, roles, and independent permissions | required | official | M7 | research |
| `DATA.PANEL.BROWSE` | Data Panel | Browse, organize, upload, and share projects and libraries | required | official | M7 | research |
| `DATA.COLLABORATION.LIVE_EDITING` | Collaborative Editing | Coordinate live editing and real-time version tracking | improved | official | M9 | research |
| `DATA.FILES.UPLOAD` | Upload | Import local files into a project | required | official | M7 | research |
| `DATA.FILES.FOLDER` | New Folder | Create project folders | required | official | M7 | research |
| `DATA.HOME.RECENT` | Fusion Home | Show recent files, projects, and learning entry points | improved | official | M7 | research |
| `DATA.WEB.CLIENT` | Fusion web client | Manage projects, permissions, versions, and review in a browser | improved | official | M7 | research |
| `DATA.VERSION.SAVE` | Version control | Create immutable saved versions | improved | official | M2 | specified |
| `DATA.VERSION.HISTORY` | Version history | Browse, compare, label, and restore prior versions | improved | official | M7 | research |
| `DATA.SHARE.LINK` | Share Link | Share controlled review links without requiring an editor client | improved | official-overview | M7 | research |
| `DATA.COMMENTS.MARKUP` | Comments and Markup | Attach review comments and visual markup to model context | improved | official-overview | M9 | research |
| `DATA.CONFIGURATIONS.MODEL` | Configurations | Define parameter, feature, and property variants | required | official | M7 | research |

### Public automation

| ID | Reference name | Behavioral target | Target | Evidence | Milestone | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `API.LANGUAGE.RUST` | - | Native public Rust APIs for every headless Amphion subsystem | improved | product-decision | M1 | specified |
| `API.LANGUAGE.PYTHON` | Python API | Supported Python client and add-in API | required | official | M9 | research |
| `API.LANGUAGE.CPP` | C++ API | Stable C ABI usable from C++ | improved | official | M9 | research |
| `API.LANGUAGE.TYPESCRIPT` | TypeScript API | Generated TypeScript protocol client | improved | official | M7 | research |
| `API.PACKAGE.SCRIPT` | Scripts | Run one-shot automation packages | required | official | M9 | research |
| `API.PACKAGE.ADD_IN` | Add-Ins | Run lifecycle-managed persistent extensions | required | official | M9 | research |
| `API.MODEL.OBJECT_GRAPH` | Object Model | Navigate document, design, component, feature, sketch, and construction objects | improved | official | M7 | research |
| `API.MODEL.COLLECTIONS` | Collections | Deterministically enumerate and find typed objects | improved | official | M7 | research |
| `API.MODEL.INPUTS` | Input Objects | Construct validated command input objects | improved | official | M7 | research |
| `API.MODEL.DEFINITIONS` | Definition Objects | Inspect and edit existing parametric definitions | improved | official | M7 | research |
| `API.MODEL.VALUE_INPUT` | ValueInput | Accept values, units, expressions, and parameter references | required | official | M7 | research |
| `API.FEATURE.CUSTOM` | Custom Features | Register custom parametric features in history | improved | official | M9 | research |
| `API.UI.COMMANDS` | UI Customization | Add commands, buttons, tool groups, and context entry points | improved | official | M9 | research |
| `API.UI.PALETTES` | Palettes | Add sandboxed web-based extension panels | improved | official | M9 | research |
| `API.SAMPLES` | Sample Scripts | Maintain executable, versioned API examples | improved | official | M9 | research |
| `API.PROTOCOL.COMMAND_QUERY_EVENT` | - | Expose versioned command, query, and event contracts to every client | improved | product-decision | M1 | specified |

## Input-binding compatibility baseline

These defaults are migration targets, not claims about which reference
shortcuts are user-assignable. The Amphion profile itself is fully remappable.

| Command | Fusion-compatible Windows/macOS binding |
| --- | --- |
| Command search | `S` |
| Extrude | `E` |
| Hole | `H` |
| Press/Pull | `Q` |
| Fillet | `F` |
| Move/Copy | `M` |
| Appearance | `A` |
| Joint / As-Built Joint | `J` / `Shift+J` |
| Measure | `I` |
| Line | `L` |
| 2-Point Rectangle | `R` |
| Center Diameter Circle | `C` |
| Sketch Dimension | `D` |
| Trim | `T` |
| Offset | `O` |
| Project | `P` |
| Toggle construction | `X` |
| Repeat last command | `Space` |
| Undo / redo | `Ctrl+Z` / `Ctrl+Y` on Windows; `Cmd+Z` / `Cmd+Y` on macOS |
| Pan / orbit / zoom / fit | MMB drag / `Shift+MMB` drag / wheel / double-click MMB |

Default bindings are backed by interaction tests; conflicts are resolved per
profile rather than by hard-coded event branches.

## Source registry

Official Autodesk pages were fetched directly during the 2026-07-19 audit.
The registry groups rows sharing the same reference page; a
`research-needed` row deliberately makes no stronger evidence claim.

| Coverage | Source |
| --- | --- |
| Interface, browser, timeline, ViewCube | <https://help.autodesk.com/cloudhelp/ENU/Fusion-GetStarted/files/GS-THE-FUSION-INTERFACE.htm> |
| Sketch constraints | <https://help.autodesk.com/cloudhelp/ENU/Fusion-Sketch/files/SKT-CONSTRAINTS.htm> |
| Sketch lines, arcs, circles, splines, slots, polygons | <https://help.autodesk.com/cloudhelp/ENU/Fusion-Sketch/files/SKT-SKETCH-CREATE-LINES.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-Sketch/files/SKT-SKETCH-CREATE-ARCS.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-Sketch/files/SKT-SKETCH-CREATE-CIRCLES.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-Sketch/files/SKT-SKETCH-CREATE-SPLINES.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-Sketch/files/SKT-SKETCH-CREATE-SLOTS.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-Sketch/files/SKT-CREATE-POLYGONS.htm> |
| Sketch modify and Project/Include | <https://help.autodesk.com/cloudhelp/ENU/Fusion-Sketch/files/SKT-SKETCH-MODIFY-TOOLS.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-Sketch/files/SKT-SKETCH-CREATE-PROJECT-INCLUDE.htm> |
| Solid create, primitives, modify, patterns | <https://help.autodesk.com/cloudhelp/ENU/Fusion-Model/files/SLD-CREATE-SOLID-FROM-SKETCH.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-Model/files/SLD-CREATE-SOLID-PRIMITIVE.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-Model/files/SLD-MODIFY-SOLID-BODY.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-Model/files/SLD-PATTERNS.htm> |
| Surface create and modify | <https://help.autodesk.com/cloudhelp/ENU/Fusion-Patch/files/SFC-CREATE-SURFACE-FROM-SKETCH.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-Patch/files/SFC-MODIFY-SURFACE.htm> |
| Form create and modify | <https://help.autodesk.com/cloudhelp/ENU/Fusion-Sculpt/files/FRM-CREATE-FORM-PRIMITIVE.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-Sculpt/files/FRM-MODIFY-TOOLS.htm> |
| Assembly joints and constraints | <https://help.autodesk.com/cloudhelp/ENU/Fusion-Assemble/files/GUID-8818AE31-958A-4A59-989B-9875A174C67A.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-Assemble/files/ASM-CONSTRAIN-COMPONENTS.htm> |
| Construct | <https://help.autodesk.com/cloudhelp/ENU/Fusion-Model/files/SLD-CONSTRUCT-TOOLS.htm> |
| Mesh | <https://help.autodesk.com/cloudhelp/ENU/Fusion-Mesh/files/MESH-CREATE-TOOLS.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-Mesh/files/MESH-MODIFY-TOOLS.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-Mesh/files/MESH-PREPARE-TOOLS.htm> |
| Sheet metal | <https://help.autodesk.com/cloudhelp/ENU/Fusion-Sheet-Metal/files/SM-FLANGES.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-Sheet-Metal/files/SM-UNFOLD-IN-SM.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-Sheet-Metal/files/SM-CORNER-CLOSURE.htm> |
| Insert and Inspect | <https://help.autodesk.com/cloudhelp/ENU/Fusion-Model/files/SLD-INSERT-TOOLS.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-Model/files/SLD-INSPECT-TOOLS.htm> |
| Drawing | <https://help.autodesk.com/cloudhelp/ENU/Fusion-Drawing/files/DWG-REF-DRAWING-TAB.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-Drawing/files/DWG-DIMENSIONS.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-Drawing/files/DWG-SYMBOLS.htm> |
| Render and Animation | <https://help.autodesk.com/cloudhelp/ENU/Fusion-Render/files/RND-MATS-APPEARANCES.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-Render/files/RND-LIGHTING.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-Animate/files/ANI-STORYBOARD.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-Animate/files/ANI-EXPLODE-MANUAL.htm> |
| Simulation studies | <https://help.autodesk.com/cloudhelp/ENU/Fusion-Simulate/files/GUID-8CC26683-A6DD-4FDC-80BD-0DC40D7ACAF2.htm> |
| Manufacture | <https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/GUID-BEC5DEA9-AC3E-4FA8-998E-4AE8CD0D0B1E.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/MFG-TURNING-OVERVIEW.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/MFG-DRILLING-OVERVIEW.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/MFG-POST-PROCESSING-OVERVIEW.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-CAM/files/MFG-MACHINES.htm> |
| Electronics | <https://help.autodesk.com/cloudhelp/ENU/Fusion-ECAD/files/ECD-TUTORIALS-CPT.htm> |
| Data and collaboration | <https://help.autodesk.com/cloudhelp/ENU/Fusion-GetStarted/files/GS-HUBS-AND-PROJECTS.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-GetStarted/files/GS-FUSION-INTERFACES.htm> |
| Public API | <https://help.autodesk.com/cloudhelp/ENU/Fusion-360-API/files/BasicConcepts_UM.htm>, <https://help.autodesk.com/cloudhelp/ENU/Fusion-360-API/files/WritingDebugging_UM.htm> |
| Supported reference file formats | <https://www.autodesk.com/support/technical/article/caas/sfdcarticles/sfdcarticles/File-formats-supported-by-Fusion-360.html> |

Secondary shortcut evidence:
<https://productdesignonline.com/tips-and-tricks/autodesk-fusion-hotkeys-keyboard-shortcuts/>.
Shortcut assignability remains deliberately unclaimed.

## Known research gaps

The inventory is complete as a planning index, but these exact reference
details remain deliberately marked `research-needed`:

- complete Electronics command-level enumeration;
- individual Turning and multi-axis CAM command pages;
- remaining Sheet Metal modify/rule commands;
- several Construction axis/point variants;
- Render Gallery and Animation Transform/Callout/View/Publish;
- a dedicated Utilities-panel reference;
- full Generative Design command enumeration;
- exact default-shortcut behavior where only secondary evidence is public.

These gaps do not remove capabilities from Amphion's target. They block only a
claim of verified reference parity until authoritative behavior is captured.

## Inventory completion gate

Every row must eventually contain the remaining fields defined in
[PRODUCT_PLAN.md](PRODUCT_PLAN.md): dependencies, selection contract,
parameters, preview, entry points, input bindings, platforms, and named test
cases. A row advances to `parity` or `improved` only after:

1. headless behavior conformance tests pass;
2. client command-state and undo/redo tests pass;
3. keyboard, screen-reader, pointer, and touch accessibility tests pass;
4. the Fusion-compatible profile passes migration-task tests;
5. failure and cancellation paths pass deterministic regression tests.
