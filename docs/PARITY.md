# Spine parity tracking

The behavior target is the installed Spine Editor **4.3.26** and compatible 4.3
runtime JSON. A feature is accepted through independent implementation, command/
history checks, native interaction where applicable, and a controlled editor/runtime
comparison. The native `.peachy` format is our own editable model.

| Spine capability | Current implementation | Evidence or remaining gate |
| --- | --- | --- |
| Workspace, controls and conventions | First alignment pass; explicit ongoing requirement | See [UI parity](UI_PARITY.md); finish interaction gaps before weight-painting UI |
| Bone hierarchy and transforms | Normal inheritance, translation, rotation, scale | Sampled world matrices; still needs shear and other inheritance modes |
| Region attachments and skins | Image transforms, named skins, default fallback, fixed slot draw order | Native editor round trip and external runtime |
| Unweighted mesh attachments | Import/export explicit UVs, topology, hull, setup vertices | Three original meshes round-trip through editor with identical data; runtime world vertices and atlas UVs agree |
| Region → mesh | Four corners preserve the textured geometry and slot position | Engine checks under animation, reflection and nonuniform scale; native conversion check |
| Setup mesh transforms | Single-vertex drag and numeric local coordinates, fixed UVs | Native mouse drag, one-step undo, redo, no UV changes |
| Mesh topology editing | Split an existing triangle at its centroid | Preserves hull, indices and UV interpolation; freeform hull, UV/edge editing and general retriangulation remain |
| Weights and binding | Pending, following UI alignment | Typed bone influences, binding, deterministic weighted geometry, normalized edits and black-box vertex comparisons |
| Deform animation and linked meshes | Pending | Requires explicit model, interpolation, shared-mesh semantics and round-trip cases |
| Bone animation and curves | Scalar transform keys, linear/stepped/Bezier, playback and scrubbing | External runtime boundaries, backwards scrubbing, full-turn rotation, native key/history checks |
| Timeline authoring | Basic key insertion/removal and curve controls | Still needs selection, retiming, copy/paste, richer animation management and mixing |
| Rig/artwork authoring | Add child bones; work with existing loose PNGs | Still needs image import, attachment creation, pivots, stable IDs before delete/reparent |
| IK/transform/path/physics constraints | Pending | Each needs controlled inputs, public behavior specification and runtime boundary checks |
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
