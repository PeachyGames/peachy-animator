# Spine parity tracking

The behavior target is the installed Spine Editor **4.3.26** and compatible 4.3
runtime JSON. A feature is accepted through independent implementation, command/
history checks, native interaction where applicable, and a controlled editor/runtime
comparison. The native `.peachy` format is our own editable model.

| Spine capability | Current implementation | Evidence or remaining gate |
| --- | --- | --- |
| Workspace, controls and conventions | Layout/shortcut alignment plus typed Tree, contextual selection, filters, activation dots and picking | See [UI parity](UI_PARITY.md); multi-selection, remaining tools and docking remain before weight-painting UI |
| Views discovery and auxiliary panels | Full 15-view public-guide inventory; bounded Animations/Skins/Metrics/Outline/Playback/Ghosting panels, alongside Tree/Dopesheet/Weights | Six unsupported views are explicitly disabled. Exact 4.3 mode lists, Graph, independent Preview/mixing and general docking remain gates. See [Views audit](VIEWS.md) |
| Transform tools and axes | Rotate/Translate/Scale plus bounded bone Shear controls/handles; numeric operators, active-tool nudges and fixed-baseline previews | Shear currently displays local values; exact installed World/Parent display, empty-space/Shift/handle behavior and visual alignment remain gates |
| Selection navigation | Page Up/Down history and Cmd/Ctrl+number storage / number recall for typed single-item groups | Session state only; multi-selection and persisted group preferences pending |
| Bone hierarchy and transforms | Normal inheritance, translation, rotation, scale and shear | Independent affine math and original shear interchange cases; other inheritance modes remain pending |
| Region attachments and skins | Image transforms, named skins, default fallback, fixed slot draw order | Native editor round trip and external runtime |
| Unweighted mesh attachments | Import/export explicit UVs, topology, hull, setup vertices | Three original meshes round-trip through editor with identical data; runtime world vertices and atlas UVs agree |
| Region → mesh | Four corners preserve the textured geometry and slot position | Engine checks under animation, reflection and nonuniform scale; native conversion check |
| Setup mesh transforms | Single-vertex drag and numeric local coordinates, fixed UVs | Native mouse drag, one-step undo, redo, no UV changes |
| Mesh topology editing | Split an existing triangle at its centroid | Preserves hull, indices and UV interpolation; freeform hull, UV/edge editing and general retriangulation remain |
| Weighted mesh data and preview | Native v4 influences, strict JSON import/export, shared bone-driven geometry and read-only per-vertex inspection | 7,800 weighted runtime coordinates pass at unchanged 0.001 tolerance; editor re-export quantizes coordinates and is not lossless |
| Binding and weight authoring | Initial Bind/Remove all, additional zero bindings, and single-vertex/bone Direct percentages through shared commands and a G Weights tool/view | Proportional redistribution is explicit; raw imports/unselected vertices are preserved. See latest validation for distinct core/CLI/runtime/native/editor gates. Brushes, locks, Update and multi-selection remain pending; exact dock placement is provisional |
| Deform animation and linked meshes | Pending | Requires explicit model, interpolation, shared-mesh semantics and round-trip cases |
| Bone animation and curves | Scalar transform keys, linear/stepped/Bezier, playback and scrubbing | External runtime boundaries, backwards scrubbing, full-turn rotation, native key/history checks |
| Unkeyed poses and Auto Key | Transient multi-bone poses, K/Tree/transform keying, colored state, paired XY defaults and unified undo | Bone-transform subset; slot/deform/constraint keying remains pending |
| Animation transport | A/D reverse/forward play, bidirectional loops, W/S key navigation, Q/E start/last key and shared Playback speed controls | Nonlooping playback stays within native duration; FPS retiming and interpolation overrides remain pending |
| Ghosting | Frame/key onion skins, keyed Current comparison, tint/opacity, Images/Solid, Anchor/Loop/On top, per-frame offsets and typed one-bone Selection/Lock/Refresh | Shared checked region/mesh/IK evaluation; bounded 64-pose sampling. Motion Vectors, Silhouette/Xray and exact installed comparison remain pending. See [Views scope](VIEWS.md) and [evidence](evidence/ghosting.json) |
| Timeline authoring | Key insertion, exact/row/all-shown selection, removal, retiming/copy gestures, Copy/Cut/Paste and curve controls; animation rename/duplicate | Key-derived ends, empty clips and beyond-end authoring implemented; still needs marquee/all-bone selection, advanced curves, clip management and mixing |
| Rig/artwork authoring | Add child bones, rename bones/slots; work with existing loose PNGs | Still needs image import, attachment creation, pivots, stable IDs before delete/reparent |
| IK constraints | Bounded static one-/two-bone normal IK, animated targets, authored/applied separation, Setup Mix/Positive/target/Rename | [Phase A](slices/ik-design.md) and [validation](VALIDATION.md#normal-ik-phase-a--november-6-2026-utc); animated settings, creation, dependent/transformed chains and exact installed UI remain gates |
| Transform/path/physics constraints | Pending | Each needs controlled inputs, public behavior specification and runtime boundary checks |
| Slot animation, draw order, events, clipping, sequences | Pending | Unsupported imports are refused; no silent approximation |
| Engine integration | Versioned Spine JSON plus single-page atlas export | Separate licensed runtime is an external consumer/oracle; native `.spine` and binary `.skel` decoding remain unsupported |

The mesh work maps to Spine's distinction between
[setup vertex transforms and Edit Mesh](https://esotericsoftware.com/spine-meshes).
Our fixed-UV deformation corresponds to setup vertex transforms. We do not yet
provide Spine's full UV topology editing workflow. Format details follow the
[public JSON contract](https://esotericsoftware.com/spine-json-format).

The [compatibility contract](COMPATIBILITY.md) contains exact accepted data and
limits. [Validation](VALIDATION.md) records measured results. Neither a successful
JSON load nor these finite fixture samples establish complete Spine compatibility.
