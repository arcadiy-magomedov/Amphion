# Amphion product plan

## Product target

Amphion is not only a geometry kernel. The finished product is a family of CAD
clients built on one headless modeling platform.

The long-term parity target includes the complete Fusion-class workflow:

- parametric sketching and constraints;
- solid, surface, mesh, sheet-metal, and plastic design;
- construction geometry and inspection;
- assemblies and joints;
- drawings, rendering, and animation;
- simulation and generative workflows;
- manufacturing, additive, and CAM;
- electronics and PCB workflows;
- scripting, extensions, data management, versions, and collaboration.

The versioned [capability inventory](CAPABILITY_INVENTORY.md) is a product
contract, not a claim that all domains will ship in the first release. Each
capability must have an explicit dependency graph, delivery milestone,
platform status, and acceptance tests.

## Clean-room Fusion familiarity

The goal is interaction compatibility for users who already know Autodesk
Fusion. Amphion will use original code, icons, text, components, visual tokens,
and branding while preserving the useful mental model and muscle memory.

The familiarity contract covers:

- recognizable workspaces and tool grouping;
- the same broad document anatomy and spatial map;
- familiar selection-first and command-first workflows;
- preview-driven command dialogs with explicit confirm and cancel;
- browser tree, properties, history timeline, and canvas relationships;
- compatible default shortcuts where they are useful;
- compatible mouse navigation as a built-in input profile;
- first-class trackpad navigation rather than mouse-event emulation;
- an orientation cube with faces, edges, corners, named views, and Home;
- command search, context menus, undo/redo, and selection filters.

Amphion may intentionally improve discoverability, diagnostics, accessibility,
touch support, customization, and failure handling. Compatibility must not
freeze defects or force the same visual assets.

## Stable application anatomy

The browser client uses stable functional regions:

```text
+---------------------------------------------------------------------+
| application / document tabs / workspace switcher / account and sync |
+----------------------+----------------------------------------------+
| contextual command toolbar and command search                       |
+----------------------+----------------------------------------------+
| model browser        |                                      cube +  |
| origins              |                canvas                Home     |
| sketches             |                                             |
| bodies/components    |                                             |
| analyses             |                                             |
|                      |                              command inspector|
+----------------------+----------------------------------------------+
| feature timeline / playback / diagnostics                           |
+---------------------------------------------------------------------+
| navigation / display / grid / selection / status                    |
+---------------------------------------------------------------------+
```

Panels can collapse, resize, and move within supported layouts, but commands
must not unpredictably relocate between sessions or screen sizes.

## Command interaction contract

Every modeling command follows one state machine:

```text
idle -> collecting input -> valid preview -> committed
                       \-> invalid preview
                       \-> cancelled
```

Requirements:

1. Selection before command and command before selection both work whenever the
   operation permits it.
2. The canvas and inspector show the same live parameters and validity state.
3. Invalid geometry remains visible with an actionable explanation; it never
   disappears as if the command succeeded.
4. Enter confirms a valid command and Escape moves one level back or cancels.
5. Undo/redo acts on complete transactions, not internal solver steps.
6. Reopening a feature uses the same command surface with its previous inputs.
7. Every pointer workflow has a keyboard path, and every complex gesture has a
   single-pointer alternative.
8. Long-running operations expose progress, cancellation, and deterministic
   diagnostics.

## Input and navigation system

Input bindings are data, not hard-coded event branches. The command system
supports:

- an Amphion default profile;
- a Fusion-compatible mouse and shortcut profile;
- additional CAD profiles later;
- per-command remapping with conflict detection;
- separate bindings for keyboard, mouse, trackpad, pen, and touch;
- export and import of user profiles.

The orientation cube is an Amphion component, not an Autodesk asset. It must:

- snap to six orthographic faces, twelve edge views, and eight corner views;
- orbit continuously by dragging;
- expose Home and user-defined home views;
- restore front/top conventions per document;
- support perspective and orthographic cameras;
- remain keyboard and screen-reader operable;
- use at least 44 by 44 CSS-pixel pointer targets on touch hardware.

Navigation behavior is specified through device-independent intents:

```text
Orbit
Pan
ZoomContinuous
ZoomToCursor
FitAll
FitSelection
FocusSelection
LookAtSelection
SetHome
GoHome
SnapView
RollCamera
```

Mouse buttons, modifier keys, wheel input, trackpad gestures, pen gestures, and
touch gestures map to these intents. This lets compatibility profiles change
without branching camera logic.

## Capability inventory schema

Every Fusion-class capability receives one stable Amphion ID and one row in the
inventory:

| Field | Meaning |
| --- | --- |
| `id` | Stable identifier such as `DESIGN.SKETCH.CONSTRAINT.TANGENT` |
| `domain` | Design, Drawing, Manufacture, Electronics, Simulation, and so on |
| `workspace` | User-visible workspace or mode |
| `group` | Create, Modify, Construct, Inspect, Assemble, or equivalent |
| `name` | Generic Amphion command name |
| `reference_name` | Familiar Fusion label used only for parity research |
| `behavior` | Concise functional contract |
| `dependencies` | Kernel, sketch, document, renderer, cloud, or device capabilities |
| `selection_contract` | Accepted entities and selection order |
| `parameters` | Inputs, defaults, units, ranges, and expressions |
| `preview` | Required transient geometry and diagnostics |
| `entry_points` | Toolbar, search, shortcut, context menu, API |
| `input_bindings` | Default and compatibility-profile bindings |
| `platforms` | Browser, desktop, tablet, and service availability |
| `milestone` | First milestone that promises the capability |
| `status` | Research, specified, blocked, prototype, partial, parity, improved |
| `tests` | Conformance, interaction, accessibility, and regression cases |
| `sources` | Public behavior references used for clean-room research |

No feature reaches `parity` from visual inspection alone. It must pass its
behavioral and interaction acceptance tests.

## Human-visible milestones

These are product demonstrations, not calendar promises.

### M0 - Repository bootstrap

Available now:

- versioned architecture and execution plans;
- Rust workspace and continuous integration;
- source-available and commercial licensing model.

### M1 - Geometry Microscope

A user can open Amphion Lab in a browser and:

- load the Rust kernel through WebAssembly;
- inspect Line, Circle, Plane, Cylinder, and Cone geometry;
- orbit, pan, zoom, fit, and use the orientation cube and Home;
- select geometry and inspect parameters, UV coordinates, normals, seams,
  tolerances, and validation diagnostics;
- switch mouse/trackpad navigation profiles.

This milestone proves the client/kernel boundary and input system before solid
editing exists.

### M2 - Primitive Studio

A user can:

- create and edit Cuboid, Cylinder, and Cone parameters;
- select Body, Face, Edge, and Vertex entities;
- inspect analytic properties and topology;
- view validation failures without losing the last valid result;
- save, reopen, and deterministically reproduce a model.

### M3 - Sketch Lab

A user can:

- create sketches on origin planes and supported planar faces;
- draw, trim, extend, offset, project, pattern, and dimension geometry;
- apply the complete planned set of geometric constraints;
- see remaining degrees of freedom, redundancy, conflicts, and solver
  diagnostics;
- edit with mouse, trackpad, keyboard, pen, or touch-compatible controls.

Sketch implementation and its test battery run in parallel with advanced solid
booleans.

### M4 - First Parametric Part

A user can:

- create a constrained sketch;
- extrude a selected profile;
- edit a dimension and observe deterministic recomputation;
- inspect the browser and feature timeline;
- undo, redo, save, reopen, and reproduce the same part.

This is the first milestone that behaves like a small practical CAD system.

### M5 - Solid Part Design

A user can:

- join, cut, and intersect solids;
- create pockets, holes, patterns, mirrors, chamfers, and supported fillets;
- edit earlier features while semantic references remain stable;
- receive precise diagnostics for unsupported or invalid configurations.

### M6 - STEP Interoperability

A user can:

- import the declared STEP AP242 subset by drag and drop;
- inspect the recovered analytic B-Rep;
- export and reopen it;
- compare semantic round-trip results and validation reports.

### M7 - Parametric Alpha

A user can complete representative multi-feature parts with:

- robust history recomputation and semantic references;
- components and basic assemblies;
- reusable parameters and expressions;
- command search, customizable shortcuts, selection filters, and named views;
- browser-first project and version management.

### M8 - Design Workspace Parity

The browser product reaches declared parity for mechanical Design workflows:

- sketch, solid, surface, mesh, sheet-metal, and construction workflows;
- assemblies, inspection, drawings, rendering, and animation;
- migration-focused interaction and shortcut acceptance tests.

### M9 - Engineering and Manufacturing Modules

Separate modules add declared parity targets for:

- simulation and generative design;
- CAM, additive manufacturing, and machine setup;
- electronics and PCB;
- automation, extensions, collaboration, and organization workflows.

The finished-product target is complete only when every capability inventory
row is either `parity`, `improved`, or explicitly removed by a documented
product decision.

## Migration acceptance tests

Recruit users who regularly use Fusion but have never seen Amphion. Without a
tutorial, they must be able to:

1. return the camera Home, select an origin plane, and start a sketch;
2. draw and fully constrain a dimensioned profile;
3. finish the sketch and extrude it;
4. create a second sketch on a face and cut a pocket;
5. edit the first sketch dimension and understand the recompute result;
6. measure geometry and run a section analysis;
7. find an unfamiliar command through search;
8. complete the flow using the Fusion-compatible mouse profile;
9. complete navigation with a trackpad and no external mouse;
10. recover from one deliberately invalid operation using the diagnostic.

Targets for a parity milestone:

- at least 90 percent task completion without documentation;
- median command-discovery time no worse than the reference workflow;
- no critical task requires a proprietary input device;
- all tasks are operable by keyboard;
- all persistent controls meet WCAG AA contrast and focus requirements;
- no participant mistakes Amphion for an Autodesk product.

## Client architecture constraints

- Browser/WASM is the first client; desktop and tablet follow later.
- UI clients issue versioned commands and queries through a public protocol.
- UI code never mutates B-Rep entities directly.
- The renderer consumes derived meshes plus stable semantic selection IDs.
- Camera and input systems are independent of modeling commands.
- The same behavioral conformance suite applies to every future client.
- Product UI code and the headless kernel remain separately replaceable.
