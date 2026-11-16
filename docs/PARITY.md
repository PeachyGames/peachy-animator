# Spine parity tracking

The behavior target is the installed Spine Editor **4.3.26** and compatible 4.3
runtime JSON. A feature is accepted through independent implementation, command/
history checks, native interaction where applicable, and a controlled editor/runtime
comparison. The native `.peachy` format is our own editable model.

Latest **UI-only workspace checkpoint**, November 16: adaptive panel budgets pass
native acceptance on Desktop `d1fc05d9…14e8`. At 1120×740, the measured
Dopesheet/Timeline and Playback variants retain at least 206 canvas pixels and
four complete Bone rows; all 14 keyed channels of a two-Bone group are reachable
by actual scrolling. Layout/scroll/foreground yield preserve data, pending,
selection and redo. Native expansion restores default pane heights exactly;
custom divider preferences have source/event coverage only. Source03/04 focused
checks, Clippy/build and architecture pass; `divider` matched zero tests. No new
MCP/runtime proof or completed cycle Offset is implied. A separate combined
working-tree run passes five pane-resize tests alongside eight unfinished engine
Offset foundation tests; the UI checkpoint excludes that engine work. The earlier multi-Bone
checkpoint below stays separate. See [review](evidence/minimum-workspace.html),
[scalars](evidence/minimum-workspace.json) and [UI limits](UI_PARITY.md).

Current **bounded native/MCP checkpoint**: native **v23 /64 commands /23 tools**. Ordered
multi-Bone selection and shared atomic authored-pose edits/keying now support
pending poses and Auto Key; shared translation filters selected descendants and
uses one evaluated input frame. Actual blank-root MCP run03 passes 104 requests,
57 construction commands, 21 pose queries, four atomic negatives and exact
history/Save/reopen. Engine group8/Session9 and MCP6-unit/4-stdio focused checks
pass, as do final combined source gates. The MCP-authored `f3bdde6a…0983` result
passes 156 external-runtime poses at 0.001. **Bounded native acceptance now passes
on unchanged Desktop `1558c256…5763`**: ordered Tree/viewport selection, all three
Translate axes, ancestor filtering, Parent numeric assignment, changed held
preview cancellation, pending/keying/history, minimum controls and actual edited
Save/fresh-process recovery. The separate [native result](../examples/peach-bot/multi-bone-native-result.peachy)
is `14f2e703…13a2`; it has 36 actual MCP readback requests/14 pose queries, including
exact native authored/applied/world witnesses. It has no new external-runtime
comparison. Minimum acceptance covers visible transform/key controls, with a tiny
Tree and clipped rig. Hosted CI, pixel and full group-transform parity are not
claimed. All owned processes are closed. See
[checkpoint evidence](evidence/multi-bone-authoring.json), [review](evidence/multi-bone-authoring.html)
and [UI scope](UI_PARITY.md).

Preceding leaf-routing checkpoint: native **v23**, **64 commands /19 tools** unchanged.
Five native contexts match complete MCP71 case documents: no-owner source fallback,
explicit Bone, cross-animation, visible Animate Slot and true Region parent Bone.
Native Undo, same-case native/MCP Save byte equality and exact fresh reopen with
empty history/clipboard/one image pass. MCP/headless Undo/Redo is separate evidence.
All owned processes are closed. Slot Dopesheet scope remains global DrawOrder;
broader Slot/Region controls, Region rows/header styling, general Tree mapping,
Shift/Offset and Lock/Sync remain open. See [scope](slices/leaf-clipboard.md),
[review](evidence/leaf-clipboard.html) and [scalar evidence](evidence/leaf-clipboard.json).

Preceding grouped-clipboard checkpoint: native **v23** unchanged, **64 commands /
19 tools**. Bounded native acceptance now includes minimum layout and persistence.
Shared global-origin group copy/paste and explicit target mapping
pass focused MCP checks and actual 57-request blank-project run02. Two separate
other-target/cross-animation exports pass 208 runtime poses at 0.001. **Native
minimum 1120×740 and Save/fresh-process typed recovery pass** on the unchanged
binary; history/clipboard are empty and one image loads. The earlier `da1a1da`
code push was partial; its two gates are now closed. See the
[native result](../examples/peach-bot/group-clipboard-native-result.peachy).
All owned processes are closed. Original-selection
preservation is the measured native policy. Tree multi-target mapping,
Shift/Offset and Lock/Sync remain open; bounded no-Tree leaf fallback is accepted
by the later checkpoint above. See
[scope](slices/grouped-clipboard.md), [review](evidence/grouped-clipboard.html) and
[scalar evidence](evidence/grouped-clipboard.json).

Earlier multi-owner timing checkpoint: native **v23** unchanged, **63 commands /
18 tools**. The exact authored owner/key query and bounded all-owner Dopesheet
rows/ordinary retiming pass **bounded native acceptance** on the original
two-bone scalar fixture. Final MCP run03 passes 41 requests/25 authored commands,
mixed movement/history/reopen and read-only rejection checks. Baseline and moved
exports each pass 104 runtime poses at 0.001. Group clipboard, Shift/Offset,
Lock/Sync and general Tree multi-selection remain open. See
[scope](slices/multi-owner-timeline.md), [review](evidence/multi-owner-timeline.html)
and [scalar evidence](evidence/multi-owner-timeline.json).

Earlier source/creation continuation: native **v23** unchanged, **63 commands /
17 tools**. Implementation, 141 actual MCP requests/15 retained poses and 104 external
runtime poses pass. Bounded native creation/source input, Open/Save/history and
minimum checks pass. Native Source Min10→13 proves a changed held preview;
Escape restores exact document/redo, continued movement stays blocked until
release, and a fresh gesture commits with exact one-step Undo/Redo.
Installed observations now
define prepend/numeric naming, identity effects, source/property ranges, hidden
manual-Time capture on genuine Setup Mix edits, and own-turn derived Frame.
Process-view source-pair memory stays outside saved data. See the
[continuation contract](slices/slider-constraints.md#source-and-creation-continuation).
Source interchange geometry passes; four strict authored differences include
hidden-baseline loss. Property-object and positional-range wire bugs are fixed;
current checks total63 engine/33 desktop/16 schema/6 stdio. Final run04 passes
141 requests/15 samples and the same canonical hash; a separate10-request strict
range test and final read-only native reopen pass. Detailed native interactions
remain their own earlier-binary evidence. The separate held-range replay uses
final `6b638bb1…be3f`; hidden Time/Mix/pending/selection and the source file stay
unchanged. All owned processes are closed; 25 original disposable task files and
three held-range helper/scratch files are retired.

Verified manual November 15 checkpoint: native format **23**, **58 commands / 14 MCP
tools**. The bounded Slider manual/MCP checkpoint is **verified**, with full
Slider parity **open**: separate 170-case and 177-case
metadata/active-world matrices pass at unchanged 0.001 tolerance. An 84-request
MCP recreation and bounded native editing restore the same canonical rig;
independent native creation remains unverified. Installed interchange passes
returned geometry within tolerance but retains a failed strict authored report
for track/Bezier recanonicalization. Fresh public reopen on the final skin-aware
build, actual playback/stop and final affected matrix/build checks pass. Two raw
1e30 adversarial precision mismatches remain failures. See
[the bounded Slider contract](slices/slider-constraints.md),
[review gallery](evidence/slider-constraints.html) and
[scalar evidence](evidence/slider-constraints.json).

Earlier November 15 accepted mapping checkpoint: native format **22**, **53
commands / 13 MCP tools**.
World X→Rotate passes native input/history/cancellation, minimum layout, fresh
reopen/playback, actual MCP recreation, runtime and installed interchange. See
[scope](slices/transform-mapping-authoring.md), [scalar evidence](evidence/transform-mapping.json)
and [gallery](evidence/transform-mapping.html).
The accepted multi-vertex Direct scope has separate native/MCP/runtime and
installed-editor evidence; see the [contract](slices/weight-multivertex.md),
[scalar record](evidence/weight-multivertex.json) and
[gallery](evidence/weight-multivertex.html).

The bounded workspace continuation resolves shared-edge canvas/Tree resize
input. Native −2/0/+2 Tree grabs and Properties resizing pass without document,
history or selection changes; full canvas painting/projection and the 320-pixel
minimum budget stay unchanged. Five focused tests also cover the Weights-only/
Curves-only boundaries, pending poses, within-session size persistence and owned
canvas drags crossing the reserved input band. Those pane variants are headless
coverage, not new native or cross-restart docking claims. Deform key/Delete
captures now omit offscreen controls and their real-event test scrolls to visible
Properties buttons. Native unweighted nudge/Key/Undo, Properties shrinking with
pending pose/redo intact, wheel-to-visible Key and Key/Delete baseline recovery
pass with unchanged source artwork. Weighted variants have focused event coverage,
not a new native claim. All final affected local checks pass. See
[UI parity](UI_PARITY.md#workspace-divider-input--november-15-2026).

| Spine capability | Current implementation | Evidence or remaining gate |
| --- | --- | --- |
| Workspace, controls and conventions | Layout/shortcut alignment, adaptive pane budgets/scrolling, typed Tree, ordered Bone membership/primary snapshots, contextual selection, filters, activation dots, picking, bounded Direct vertex multi-selection and nonoverlapping canvas/divider input | Native adaptive 1120×740 Dopesheet/Timeline/Playback cases retain at least 206 canvas pixels and four complete Bone rows; all 14 grouped channels are reachable and default wide heights recover exactly. Earlier selection/Save/reopen proof stays valid. Full stacked Graph, group Properties, general overlaps and docking stay open. See [UI parity](UI_PARITY.md) |
| Views discovery and auxiliary panels | 16 bounded implemented views, including Curves, Metrics and a compact Timeline | Actual installed 4.3.26 menu has 17 entries: Problems is missing; Slot Color needs current Color behavior. Advanced Graph, Preview mixing and general docking remain. See [native inventory audit](slices/spine-blog-audit-20261114.md) |
| Transform tools and axes | Existing single-Bone tools plus shared multi-Bone Local/Parent/World translation, selected-ancestor filtering and separate all-member Parent numeric assignment; explicit authored results publish atomically | Actual MCP104, source checks and bounded Native1558 three-axis/ancestor/numeric/cancellation/keying checks pass. Runtime156 covers the MCP-authored result, not native14f2. Group Rotate/Scale/Shear, non-Parent numeric/group numeric drag, general transformed-parent domains and exact visual/tool conventions remain open |
| Selection navigation | Ordered Bone membership and explicit primary, Page Up/Down history and Cmd/Ctrl+number storage / number recall use complete selection snapshots | Native Cmd-add/remove and ancestor anchors pass nine Tree and three viewport cases; navigation/group storage has code/event coverage. General Shift-range and persisted group preferences remain open |
| Bone hierarchy and transforms | All five inheritance modes with stepped keyed changes; translation, rotation, scale and shear | Independent shear/affine math and black-box-derived inheritance formulas; Inherit Lab matches the external runtime. Degenerate zero-scale parents are refused. See [inheritance](slices/inherit-modes.md) |
| Export consumer targets | Native chooser, CLI flag and strict MCP export option share Editor/Runtime policies; unsafe name overrides and contradictory cross-vertex bone order fail Editor output before writing, while Runtime preserves raw identity/fallback and influence arrays | Actual native/CLI/MCP exports and bounded installed-editor/runtime checks pass, including the final ordered weight lab. Editor re-export can prune zero entries/round weights. No full .spine conversion or exact export-dialog layout claim. See [export contract](slices/export-targets.md) and [weight interchange](slices/weight-multivertex.md#editor-guard-and-exact-final-interchange) |
| Region attachments and skins | Image transforms, named skins, default fallback, Setup slot order; skin-required bones/constraints activated per skin (runtime-matched; see [skin scope](slices/skin-scope.md)) | Native editor round trip and external runtime |
| Unweighted mesh attachments | Import/export explicit UVs, topology, hull, setup vertices | Three original meshes round-trip through editor with identical data; runtime world vertices and atlas UVs agree |
| Region → mesh | Four corners preserve the textured geometry and slot position | Engine checks under animation, reflection and nonuniform scale; native conversion check |
| Setup mesh transforms | Single-vertex drag and numeric local coordinates, fixed UVs | Native mouse drag, one-step undo, redo, no UV changes |
| Mesh topology editing | Split an existing triangle at its centroid | Preserves hull, indices and UV interpolation; freeform hull, UV/edge editing and general retriangulation remain |
| Weighted mesh data and preview | Native v4 influences, strict JSON import/export, shared bone-driven geometry and read-only per-vertex inspection | 7,800 weighted runtime coordinates pass at unchanged 0.001 tolerance; editor re-export quantizes coordinates and is not lossless |
| Binding and weight authoring | Initial Bind/Remove all, additional zero bindings and one-mesh/one-bone multi-vertex Direct numeric/slider/viewport edits; explicit selection, frozen relative ratios, skip-sole behavior, ordered sparse origin and raw MCP mixed/uniform query | Actual native history/cancel/pending Save, fresh reopen/minimum/playback, root-only MCP recreation, runtime and exact Editor roundtrip pass. New entries follow consistent per-mesh precedence; raw arrays stay intact. Auto/Smooth, brushes, locks, Update, multiple bones, sparse Deform remapping, Pies/overlay and exact styling remain. See [scope](slices/weight-multivertex.md) and [review](evidence/weight-multivertex.html) |
| Deform animation | Single-vertex Animate authoring, pending poses and keying, plus typed attachment-owned Dopesheet rows, exact retime/copy/delete, Copy/Cut/Paste and normalized Deform Graph curve editing through shared engine/MCP commands | Existing focused/native/runtime checks pass. Shared all-owner represented rows have focused coverage and a separate bounded two-bone scalar native replay; this is not a new mixed-Deform native claim. Multiple vertices, Animate soft selection and full native hierarchy remain. See [Deform timeline scope](slices/deform-timeline.md), [vertex authoring](slices/deform-authoring.md) and [data contract](slices/deform-keys.md) |
| IK scale and stretch | Uniform/mirrored scaled chains (incl. keyed scale), one- and two-bone stretch with mix, off-axis children under stretch | IK Scale Lab matches the runtime; compress, uniform, softness and keyed stretch remain. See [IK scale and stretch](slices/ik-scale-stretch.md) |
| Path constraints | Open constant-speed paths (plain/weighted), percent/fixed position, length/fixed/percent spacing, tangent rotation, mixes, position/spacing keys | Path Lab matches the runtime at every sample; closed paths, chain rotation, proportional spacing, mix keys and path editing remain. See [path constraints](slices/path-constraints.md) |
| Bone animation and curves | Scalar transform keys, linear/stepped/Bezier, playback and scrubbing | External runtime boundaries, backwards scrubbing, full-turn rotation, native key/history checks |
| Unkeyed poses and Auto Key | Atomic explicit multi-Bone edits/keying, transient bone transforms, IK Mix/Positive, mesh Deform, slot attachment/RGBA and global draw-order poses; Auto Key, K/contextual keys and unified Session undo | MCP exposes pending readback and requested-skin poses; pending-only Open/New requires explicit discard. Native multi-Bone pending Save exclusion/live preservation, global K/full-set keys, history and exact fresh recovery pass. Global K keys mixed domains atomically. Exact native pending-history equivalence remains open; Peachy Undo intentionally restores pre-key pending poses |
| Animation transport | A/D reverse/forward play, bidirectional loops, W/S key navigation, Q/E start/last key and shared Playback speed controls | Nonlooping playback stays within native duration; FPS retiming and interpolation overrides remain pending |
| Ghosting | Frame/key onion skins, keyed Current comparison, tint/opacity, Images/Solid, Anchor/Loop/On top, per-frame offsets and typed one-bone Selection/Lock/Refresh | Shared checked region/mesh/IK evaluation; bounded 64-pose sampling. Motion Vectors, Silhouette/Xray and exact installed comparison remain pending. See [Views scope](VIEWS.md) and [evidence](evidence/ghosting.json) |
| Audio view | Bounded WAV waveform/independent audition, actual native clock, seek/zoom/pan/Loop/Volume/Mute and isolated lifecycle | Session-only; native macOS playback, waveform-only elsewhere. Audio attachment/event timelines, synchronization and export remain pending. See [scope](VIEWS.md) |
| Slot Color controls | Setup/Animate sampled and pending slot RGBA, inline chooser plus Properties Color dialog, contextual key controls and exact hex/integer channels | Actual modal entry/Cancel/history/Save exclusion pass. Full current Color targets, separate RGB/alpha and dark tint remain. See [slot authoring](slices/slot-authoring.md) |
| Graph and Curves | Scalar, Deform and RGBA Graph tangents/numerics share one controller; separate fitted relative Curves, exact typed selection, Match/Separate and engine/MCP actions | Native drag/typing/cancel/history/persistence and authored runtime comparisons pass. Repeated reset and single-final handling are measured. Preset library/Last chosen, grouped native properties, adjacent/extreme coupling, all-owner Graph and styling remain. See [measured scope](slices/curves-authoring.md) and [review](evidence/curves-authoring.html) |
| Preview view | Independent keyed clip/skin, clock/transport/Loop/Speed, own pan/zoom/Fit/Bones and localized checked-evaluation errors | Useful in Setup without pending-pose overrides. Single track and instant switches; mixing, layering and next-entry Repeat remain pending. See [scope](VIEWS.md) |
| Mesh Tools view | Size/Feather/Hull soft selection for unweighted Setup vertex drags and nudges; frozen atomic gestures, influence colors and radius rings | Single selected vertex; Rotate/Scale, weighted/Animate soft transforms and other vertex attachment types remain pending. See [scope](VIEWS.md) |
| Compact Timeline view | Shared cursor/transport, exact global key markers, decimal entry, snapped/Shift scrubbing, Loop wrapping, pan/zoom and Zoom Keys below the viewport | Independent of Dopesheet visibility; input/history/pose and minimum-window checks. Shared pan/zoom, free docking and exact installed interaction parity remain pending. See [scope](VIEWS.md) |
| Timeline authoring | Exact typed owner/key inventory, bounded all-owner rows/selection and ordinary mixed retiming/removal; grouped clipboard with global origin, separate source receipts and explicit engine/MCP target maps; animation rename/duplicate | Grouped MCP run02/57, source checks,208 runtime poses and bounded native authoring/minimum/Save/fresh-reopen pass. Paste preserves original selection; typed-document recovery is exact without raw JSON byte equality. Later leaf routing passes five native owner cases, native Undo and same-case Save/fresh typed reopen against MCP71. Compound direct Bone/Slot presentation and explicit Slot leaf targets pass MCP44 and bounded Native14b: mixed12-key retime/history/cancel, minimum timeline targets and Save/fresh reopen; separate MCP10 reads the edited native Save exactly. See [compound review](evidence/compound-dopesheet.html). Full Region controls/row styling, compound Graph/Deform hierarchy, general Tree mapping, Shift/Offset, marquee, Lock/Sync, unsupported timeline editing and clip mixing remain. See [leaf scope](slices/leaf-clipboard.md). See [group scope](slices/grouped-clipboard.md) and [ordinary timing](slices/multi-owner-timeline.md) |
| Rig/artwork authoring | Add child bones, rename bones/slots; fresh MCP root-only projects and append-only slot/region/weighted mesh creation through shared commands | Desktop image import/attachment creation and freeform topology remain; stable IDs required before delete/reparent. See [tutorial reconstruction](slices/training-recreation.md) |
| IK constraints | Bounded one-/two-bone normal IK, animated targets, authored/applied separation, Setup Mix/Positive/target/Rename; keyed Mix/Positive in grouped Dopesheet rows with typed retiming/clipboard and shared pending poses (native9) | [Phase A](slices/ik-design.md), [animated IK review](IK_ANIMATION_REVIEW.md); raw curve precision, creation, transformed chains, interactive review and exact installed UI remain gates |
| Nested IK chains and child offsets | Leg-then-toe style chains solve sequentially in list order (either order); two-bone children may sit off the parent axis | Original Chain Lab and offset variants match the external runtime in forward and reversed order; a 62-bone game rig with 6 IK and 16 physics constraints round-trips with matching poses |
| Physics constraints (data) | Import/edit/save/export every setting, Scale Y mode and Global flag; setting and reset timelines incl. global; Tree icon, typed Properties, rename, Timeline/W-S keys, Metrics | Installed 4.3.26 round trip and external 4.3.13 data/timeline checks pass; a game rig round-trips with identical simulated motion. **No editor simulation**, keying UI, creation UI, skin-required physics or reference scale yet. See [physics data](slices/physics-data.md) |
| Physics simulation preview | Pending | Needs an independently derived model calibrated against the external runtime, Simulate/Deterministic/Reset controls and wind/gravity reference scale |
| Transform constraints | Full 4.3 data; world identity Rotate, mapped X/Y and one X→Rotate writer; shared Min/Max commands/query, Setup Mix/Offset and bounded native Properties editing | Existing labs plus 101 new runtime cases and the 122-request original MCP lab pass; exact installed roundtrip passes. Actual native typing, held cancellation/history, minimum controls and fresh reopen/playback pass; native/MCP saves are byte-identical. Native Tree/Properties resizing exposes all controls and preserves held Mix/Escape. Precision-conflict/overflow poses refuse explicitly. Local/additive modes, other cross/scale/shear mappings, reconnection/creation and timelines remain. See [data slice](slices/transform-constraints.md) and [authoring scope](slices/transform-mapping-authoring.md) |
| Slider constraints (bounded manual/source checkpoints verified; full parity open) | Native v23 data, manual/six-property sources, independent Time/Mix curves, typed clipboard, pending poses and ordered evaluation; semantic prepend/identity effects, source/clear/property/range controls and MCP queries | Native typing/history/cache/Open/Save/minimum and changed held-range cancellation/commit checks pass, alongside141-request/15-pose MCP authoring. Current source runtime and installed returned geometry each cover104 poses at0.001. Strict interchange loses hidden baseline/rounds values; two raw1e30 failures and Loop/counter/domain guards remain. See [contract](slices/slider-constraints.md), [gallery](evidence/slider-constraints.html) and [scalar evidence](evidence/slider-constraints.json) |
| Slot animation | Attachment swaps/hide and RGBA authoring through shared native/MCP commands, typed Attach/RGBA rows, mixed clipboards and curve-preserving color insertion | Native input/history/persistence and both final authored exports pass; 104 split parameters match installed 4.3.26. Separate rgb/alpha/dark tint, sequences and name-override editor interchange remain. See [slot authoring](slices/slot-authoring.md) |
| Events | Definitions and timed keys (with per-key overrides) import, export and migrate; reference scale stored | The doggy-scene female opens and matches the runtime; no Events view or event authoring yet. See [events](slices/events.md) |
| Linked meshes | Import, export, drawing, deform-key following and source-edit propagation | Linked Mesh Lab matches the runtime in every skin; no create/unlink commands yet. See [linked meshes](slices/linked-mesh.md) |
| Attachment tint | Region/mesh color imports, exports and tints draws (slot × attachment) | Tint Lab matches the runtime color per sample; no editor control yet. See [attachment tint](slices/attachment-tint.md) |
| Draw order | Global Animate Tree movement/dragging and Reset, pending poses, Auto Key/K, Draw order/Default Dopesheet keys and typed exact retime/copy/paste/delete; shared raw/semantic MCP commands and full hidden-slot order readback | Native input/fresh reopen, both authored runtime checks and installed 4.3.26 interchange pass. Imported offsets stay raw; minimal-encoder ties are ours. Folders, Setup reordering, cross-layout clipboard mapping, compact native property sizing and exact native history parity remain deferred. See [review](evidence/draw-order-authoring.html), [authoring scope](slices/draw-order-authoring.md) and [data contract](slices/draw-order.md) |
| Clipping | Import/export, deforms, scene clipping (rendering and picking), concave/convex polygons, end slots, selectable viewport outlines | Clip Lab matches runtime coverage; native outline selection/visibility/frame checks pass. Polygon editing, translucent editor fill and inverse clipping remain. See [clipping](slices/clipping.md) |
| Path and clipping viewport guides | Evaluated curves/polygons, stored editor colors, selected vertices/handles and dashed path end directions; Fit includes guides | Original labs pass native mouse/keyboard checks; vertex authoring and exact installed styling remain. See [UI parity](UI_PARITY.md#path-and-clipping-viewport-guides--november-10-2026) |
| Sequences and blend modes | Region/mesh sequences with all seven playback modes, slot blend modes | Sequence Lab and the bonfire match the runtime frame by frame; viewport screen/multiply are approximations; no authoring yet. See [sequences and blending](slices/sequences-and-blending.md) |
| Engine integration | Versioned Spine JSON plus single-page atlas export | Separate licensed runtime is an external consumer/oracle; native `.spine` and binary `.skel` decoding remain unsupported |

The mesh work maps to Spine's distinction between
[setup vertex transforms and Edit Mesh](https://esotericsoftware.com/spine-meshes).
Our fixed-UV deformation corresponds to setup vertex transforms. We do not yet
provide Spine's full UV topology editing workflow. Format details follow the
[public JSON contract](https://esotericsoftware.com/spine-json-format).

The [compatibility contract](COMPATIBILITY.md) contains exact accepted data and
limits. [Validation](VALIDATION.md) records measured results. Neither a successful
JSON load nor these finite fixture samples establish complete Spine compatibility.
