//! Headless MCP shell over the same command/session engine as the desktop.
mod paths;
mod schema;

use paths::WorkspaceRoot;
use peachy_engine::{
    Command, Session, checked_pose_from_authored, checked_scene_from_evaluated,
    sample_authored_pose_with_skin,
};
use rmcp::{
    ServerHandler, handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_handler,
    tool_router,
};
use schema::{AnimationKeyInput, CommandInput, TransformPropertyInput};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

struct WorkingProject {
    session: Session,
    source: Option<PathBuf>,
    asset_root: PathBuf,
}

struct State {
    root: WorkspaceRoot,
    project: Option<WorkingProject>,
    revision: u64,
}

impl State {
    fn check_revision(&self, expected: u64) -> Result<(), String> {
        if expected != self.revision {
            return Err(format!(
                "Stale revision: expected {expected}, current {}. Read get_document before retrying.",
                self.revision
            ));
        }
        if self.revision == u64::MAX {
            return Err("Revision exhausted; start a new server session".into());
        }
        Ok(())
    }

    fn project(&self) -> Result<&WorkingProject, String> {
        self.project
            .as_ref()
            .ok_or_else(|| "Open or create a project first".into())
    }
    fn project_mut(&mut self) -> Result<&mut WorkingProject, String> {
        self.project
            .as_mut()
            .ok_or_else(|| "Open or create a project first".into())
    }

    fn summary(&self) -> Value {
        match &self.project {
            None => {
                json!({"revision":self.revision,"open":false,"dirty":false,"canUndo":false,"canRedo":false})
            }
            Some(project) => {
                let doc = project.session.document();
                json!({"revision":self.revision,"open":true,"source":project.source.as_ref().map(|source|self.root.display(source)),"dirty":project.session.is_dirty(),"canUndo":project.session.can_undo(),"canRedo":project.session.can_redo(),"name":doc.name,"format":doc.format,"fps":doc.fps,"bones":doc.bones.len(),"slots":doc.slots.len(),"constraints":doc.constraints.len(),"skins":doc.skins.keys().collect::<Vec<_>>(),"animations":doc.animations.iter().enumerate().map(|(index,animation)|json!({"index":index,"name":animation.name,"duration":animation.duration()})).collect::<Vec<_>>()})
            }
        }
    }
}

#[derive(Clone)]
pub struct AnimatorServer {
    state: Arc<Mutex<State>>,
}

impl AnimatorServer {
    pub fn new(root: &Path) -> Result<Self, String> {
        Ok(Self {
            state: Arc::new(Mutex::new(State {
                root: WorkspaceRoot::new(root)?,
                project: None,
                revision: 0,
            })),
        })
    }

    fn access(&self, action: impl FnOnce(&mut State) -> Result<Value, String>) -> CallToolResult {
        match self.state.lock() {
            Ok(mut state) => match action(&mut state) {
                Ok(value) => CallToolResult::structured(value),
                Err(error) => CallToolResult::structured_error(
                    json!({"error":error,"revision":state.revision}),
                ),
            },
            Err(_) => CallToolResult::structured_error(
                json!({"error":"Server state is unavailable; restart the session"}),
            ),
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct FileInput {
    /// Native .peachy or supported Spine 4.3 JSON path, relative to startup --root (or an absolute path within it).
    path: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct OpenInput {
    path: String,
    /// Current revision from get_document; zero before the first open.
    expected_revision: u64,
    /// Explicitly discard an unsaved headless document when opening another. Defaults to false.
    #[serde(default)]
    discard_changes: bool,
}

fn default_fps() -> u32 {
    30
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CreateInput {
    expected_revision: u64,
    /// Nonempty name for a new root-only native document; no finished rig is loaded.
    name: String,
    /// Frames per second, 1–240. Defaults to 30.
    #[serde(default = "default_fps")]
    fps: u32,
    /// Existing directory within --root. Loose PNG references resolve through its images/ child.
    asset_root: String,
    /// Explicitly discard unsaved headless changes. Defaults to false.
    #[serde(default)]
    discard_changes: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ApplyInput {
    expected_revision: u64,
    /// Flat native commands, applied atomically as one undo step. Unknown fields and unsupported commands fail.
    commands: Vec<CommandInput>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
enum HistoryAction {
    Undo,
    Redo,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct HistoryInput {
    expected_revision: u64,
    action: HistoryAction,
}

fn default_skin() -> String {
    "default".into()
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EvaluateInput {
    /// Omit or use null for Setup. Otherwise use an exact animation name from get_document.
    #[serde(default)]
    animation: Option<String>,
    /// Seconds, finite and nonnegative. Defaults to zero.
    #[serde(default)]
    time: f32,
    /// Exact skin name; defaults to default. Attachments use the engine's default-skin fallback.
    #[serde(default = "default_skin")]
    skin: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CopyAnimationKeysInput {
    expected_revision: u64,
    animation: usize,
    /// Exact typed identities from one bone, IK/Slider constraint, Deform attachment, slot or global draw order.
    keys: Vec<AnimationKeyInput>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct MeshWeightsInput {
    expected_revision: u64,
    /// Exact attachment identity; no active-skin fallback or implicit current selection.
    skin: String,
    slot: String,
    placeholder: String,
    /// Nonempty unique vertex indices. Returned in ascending order.
    vertices: Vec<usize>,
    bone: usize,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct TransformMappingInput {
    expected_revision: u64,
    /// Exact constraint index from get_document.
    constraint: usize,
    from: TransformPropertyInput,
    to: TransformPropertyInput,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SliderPoseInput {
    expected_revision: u64,
    constraint: usize,
    /// Omit or use null for Setup; otherwise an exact host animation name.
    #[serde(default)]
    animation: Option<String>,
    /// Host cursor in finite nonnegative seconds, independent of referenced time.
    #[serde(default)]
    time: f32,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct SaveInput {
    expected_revision: u64,
    /// Fresh directory to create under --root. Its parent must exist. Writes project.peachy and loose images, never overwrites.
    directory: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
enum ExportKind {
    Json,
    Bundle,
}

#[derive(Clone, Copy, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
enum ExportTarget {
    #[default]
    SpineEditor43,
    Runtime43,
}

impl ExportTarget {
    fn adapter(self) -> peachy_io::ExportTarget {
        match self {
            Self::SpineEditor43 => peachy_io::ExportTarget::SpineEditor43,
            Self::Runtime43 => peachy_io::ExportTarget::Runtime43,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::SpineEditor43 => "spineEditor43",
            Self::Runtime43 => "runtime43",
        }
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ExportInput {
    expected_revision: u64,
    kind: ExportKind,
    /// For json: a new .json file. For bundle: an existing parent directory; a new timestamped bundle is created within it.
    path: String,
    /// Defaults to spineEditor43, which validates Editor naming restrictions. runtime43 preserves independent names/placeholders for runtime consumers; it is not an Editor interchange guarantee.
    #[serde(default)]
    target: ExportTarget,
}

#[tool_router]
impl AnimatorServer {
    #[tool(
        description = "Read the independent headless session's state and complete native document. This does not inspect or control an open desktop window.",
        annotations(read_only_hint = true, idempotent_hint = true, open_world_hint = false)
    )]
    fn get_document(&self) -> CallToolResult {
        self.access(|state|Ok(json!({"state":state.summary(),"document":state.project.as_ref().map(|project|project.session.document())})))
    }

    #[tool(
        description = "Create a new root-only native document in the independent headless session, without importing a finished rig or writing files. asset_root must be an existing directory inside --root; image paths resolve through its images/ child. A dirty session requires explicit discard_changes=true. Append bones, slots and region/mesh attachments through apply_commands, then save_project to a fresh directory.",
        annotations(
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn create_project(&self, Parameters(input): Parameters<CreateInput>) -> CallToolResult {
        self.access(|state| {
            state.check_revision(input.expected_revision)?;
            if state
                .project
                .as_ref()
                .is_some_and(|project| project.session.is_dirty())
                && !input.discard_changes
            {
                return Err(
                    "Unsaved headless changes; save_project or explicitly set discard_changes=true"
                        .into(),
                );
            }
            let asset_root = state.root.existing(&input.asset_root)?;
            if !asset_root.is_dir() {
                return Err("asset_root must be an existing directory".into());
            }
            let document = peachy_document::Document::new(input.name, input.fps)?;
            state.project = Some(WorkingProject {
                session: Session::new(document)?,
                source: None,
                asset_root,
            });
            state.revision += 1;
            Ok(state.summary())
        })
    }

    #[tool(
        description = "Inspect a native or Spine JSON file without opening it or changing session history. Unsupported Spine features are reported, never discarded.",
        annotations(read_only_hint = true, idempotent_hint = true, open_world_hint = false)
    )]
    fn inspect_file(&self, Parameters(input): Parameters<FileInput>) -> CallToolResult {
        self.access(|state| {
            let path=state.root.existing(&input.path)?;
            let text=fs::read_to_string(&path).map_err(|e|e.to_string())?;
            if path.extension().and_then(|p|p.to_str())==Some("peachy") {
                let doc=peachy_io::decode_project(&text)?;
                Ok(json!({"kind":"native","valid":true,"name":doc.name,"format":doc.format,"bones":doc.bones.len(),"slots":doc.slots.len(),"animations":doc.animations.len()}))
            }else{
                let value:Value=serde_json::from_str(&text).map_err(|e|e.to_string())?;
                Ok(json!({"kind":"spine","report":peachy_spine::inspect(&value)}))
            }
        })
    }

    #[tool(
        description = "Load a native .peachy or supported Spine 4.3 JSON into an independent in-memory headless session. Reads the input unchanged. A dirty session needs explicit discard_changes=true.",
        annotations(
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn open_project(&self, Parameters(input): Parameters<OpenInput>) -> CallToolResult {
        self.access(|state| {
            state.check_revision(input.expected_revision)?;
            if state
                .project
                .as_ref()
                .is_some_and(|project| project.session.is_dirty())
                && !input.discard_changes
            {
                return Err(
                    "Unsaved headless changes; save_project or explicitly set discard_changes=true"
                        .into(),
                );
            }
            let path = state.root.existing(&input.path)?;
            let document = peachy_io::load(&path)?;
            let session = Session::new(document)?;
            let asset_root = path
                .parent()
                .ok_or("Project needs an asset directory")?
                .to_owned();
            state.project = Some(WorkingProject {
                session,
                source: Some(path),
                asset_root,
            });
            state.revision += 1;
            Ok(state.summary())
        })
    }

    #[tool(
        description = "Return the supported native command JSON Schema. apply_commands exposes the same schema inline. Bind operations capture appliedLocals and inherits from evaluate_pose; bone key values are setup-relative offsets (scale multiplies); Slider keys are absolute referenced seconds or Mix.",
        annotations(read_only_hint = true, idempotent_hint = true, open_world_hint = false)
    )]
    fn get_command_schema(&self) -> CallToolResult {
        CallToolResult::structured(
            json!({"nativeFormat":peachy_document::FORMAT_VERSION,"schema":schema::command_schema()}),
        )
    }

    #[tool(
        description = "Read selected mesh/bone weights through the shared Direct query. Returns exact per-vertex values and a uniform or mixed aggregate, never an average or normalized display value. An absent influence of a globally bound bone reads as zero. Rejects empty, duplicate, ambiguous or stale selections. This changes no document, history or revision.",
        annotations(read_only_hint = true, idempotent_hint = true, open_world_hint = false)
    )]
    fn get_mesh_weights(&self, Parameters(input): Parameters<MeshWeightsInput>) -> CallToolResult {
        self.access(|state| {
            state.check_revision(input.expected_revision)?;
            let selection = peachy_engine::MeshWeightSelection {
                attachment: peachy_document::AttachmentRef {
                    skin: input.skin,
                    slot: input.slot,
                    placeholder: input.placeholder,
                },
                vertices: input.vertices,
                bone: input.bone,
            };
            let query = peachy_engine::mesh_bone_weights(state.project()?.session.document(), &selection)?;
            let value = match query.value {
                peachy_engine::MeshWeightValue::Uniform(weight) => json!({"kind":"uniform","weight":weight}),
                peachy_engine::MeshWeightValue::Mixed => json!({"kind":"mixed"}),
            };
            Ok(json!({"revision":state.revision,"attachment":query.selection.attachment,
                "vertices":query.selection.vertices,"bone":query.selection.bone,
                "weights":query.weights.iter().map(|entry|json!({"vertex":entry.vertex,"weight":entry.weight})).collect::<Vec<_>>(),"value":value}))
        })
    }

    #[tool(
        description = "Read exact endpoint ranges for one unique supported world X-to-Rotate Transform mapping. Returns sourceMin, sourceMax, targetMin and targetMax as raw f64 readback; sourceMax is null when raw zero scale leaves it undefined. No invented display span, document mutation, history or revision change. Use editTransformMappingRange through apply_commands to author an endpoint; the complete raw setTransformConstraint payload remains separate.",
        annotations(read_only_hint = true, idempotent_hint = true, open_world_hint = false)
    )]
    fn get_transform_mapping(
        &self,
        Parameters(input): Parameters<TransformMappingInput>,
    ) -> CallToolResult {
        self.access(|state| {
            state.check_revision(input.expected_revision)?;
            let range = peachy_engine::transform_mapping_range(
                state.project()?.session.document(),
                input.constraint,
                input.from.0,
                input.to.0,
            )?;
            Ok(
                json!({"revision":state.revision,"constraint":input.constraint,
                "from":input.from.0,"to":input.to.0,"range":range}),
            )
        })
    }

    #[tool(
        description = "Read a Slider's authored Time and Mix through the shared Session sampler. Time is referenced-animation seconds; the input time is the host cursor. This returns raw authored settings, including keyed values, and does not substitute a bone-derived time or solve geometry. No document, revision or history change.",
        annotations(read_only_hint = true, idempotent_hint = true, open_world_hint = false)
    )]
    fn get_slider_pose(&self, Parameters(input): Parameters<SliderPoseInput>) -> CallToolResult {
        self.access(|state| {
            state.check_revision(input.expected_revision)?;
            let project = state.project()?;
            let doc = project.session.document();
            let animation = input
                .animation
                .as_ref()
                .map(|name| {
                    doc.animations
                        .iter()
                        .position(|clip| &clip.name == name)
                        .ok_or("Unknown animation")
                })
                .transpose()?;
            // Validate the host cursor even when querying Setup.
            if !input.time.is_finite() || input.time < 0. {
                return Err("Host time must be finite and nonnegative".into());
            }
            let target = animation.map_or(peachy_engine::EditTarget::Setup, |animation| {
                peachy_engine::EditTarget::Animate {
                    animation,
                    time: input.time,
                }
            });
            let pose = project.session.slider_pose(input.constraint, target)?;
            Ok(
                json!({"revision":state.revision,"constraint":input.constraint,
                "animation":input.animation,"hostTime":input.time,"kind":"authored",
                "pose":pose}),
            )
        })
    }

    #[tool(
        description = "Copy exact bone, IK, Slider Time/Mix, Deform, slot attachment/color or global draw-order animation keys from one owner into a portable typed clipboard without changing history, revision or the document. Relative times share one zero origin; slot clipboards retain both key kinds and four color curves. Deform clipboards include dense offsets, curves and vertex/influence layout. Draw-order clipboards preserve raw offsets and require the identical ordered slot-name layout for pasteAnimationKeys.",
        annotations(read_only_hint = true, idempotent_hint = true, open_world_hint = false)
    )]
    fn copy_animation_keys(
        &self,
        Parameters(input): Parameters<CopyAnimationKeysInput>,
    ) -> CallToolResult {
        self.access(|state| {
            state.check_revision(input.expected_revision)?;
            let project = state.project()?;
            let keys = input.keys.into_iter().map(|key| key.0).collect::<Vec<_>>();
            let clipboard = peachy_engine::copy_animation_keys(
                project.session.document(),
                input.animation,
                &keys,
            )?;
            Ok(json!({"revision":state.revision,"clipboard":clipboard}))
        })
    }

    #[tool(
        description = "Apply a flat list of native engine commands as one atomic, undoable edit. Requires the latest revision; any failed command preserves document, revision and history. No file is written.",
        annotations(
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn apply_commands(&self, Parameters(input): Parameters<ApplyInput>) -> CallToolResult {
        self.access(|state| {
            state.check_revision(input.expected_revision)?;
            if input.commands.is_empty() {
                return Err("Provide at least one command".into());
            }
            state.project_mut()?.session.dispatch(Command::Batch {
                commands: input
                    .commands
                    .into_iter()
                    .map(|command| command.0)
                    .collect(),
            })?;
            state.revision += 1;
            Ok(state.summary())
        })
    }

    #[tool(
        description = "Undo or redo one complete headless edit through the shared engine history. Requires latest revision. Source files remain unchanged.",
        annotations(
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn history(&self, Parameters(input): Parameters<HistoryInput>) -> CallToolResult {
        self.access(|state| {
            state.check_revision(input.expected_revision)?;
            let session = &mut state.project_mut()?.session;
            match input.action {
                HistoryAction::Undo => {
                    if !session.can_undo() {
                        return Err("Nothing to undo".into());
                    }
                    session.undo();
                }
                HistoryAction::Redo => {
                    if !session.can_redo() {
                        return Err("Nothing to redo".into());
                    }
                    session.redo();
                }
            }
            state.revision += 1;
            Ok(state.summary())
        })
    }

    #[tool(
        description = "Evaluate Setup or an animation at a time using the shared checked pose/IK/mesh engine. Returns authored locals, appliedLocals, effective per-bone inherits modes, world transforms, slotAttachments, raw unmultiplied float slotColors, complete back-to-front drawOrder slot indices, textured geometry and bounds. Capture appliedLocals and inherits together for a binding/weight command pose. Slot arrays and drawOrder include hidden and empty slots, before tint and byte display rounding. All effective fields and geometry come from one solve. authoredSliderPoses/appliedSliderPoses are indexed by the full constraint list (non-Slider entries are inactive placeholders). This is geometry, not a raster preview or live desktop pose.",
        annotations(read_only_hint = true, idempotent_hint = true, open_world_hint = false)
    )]
    fn evaluate_pose(&self, Parameters(input): Parameters<EvaluateInput>) -> CallToolResult {
        self.access(|state|{
            let document=state.project()?.session.document();
            let animation=input.animation.as_ref().map(|name|document.animations.iter().position(|clip|&clip.name==name).ok_or("Unknown animation")).transpose()?;
            if !document.skins.is_empty() && !document.skins.contains_key(&input.skin){return Err("Unknown skin".into());}
            let authored=sample_authored_pose_with_skin(document,animation,input.time,&input.skin)?;
            let evaluated=checked_pose_from_authored(document,&authored)?;
            // Read every effective domain from the same ordered constraint solve
            // that supplies geometry and binding locals; never re-solve here.
            let effective=json!({"inherits":evaluated.inherits(),
                "slotAttachments":evaluated.slot_attachments(),"slotColors":evaluated.slot_colors(),
                "drawOrder":evaluated.draw_order(),
                "authoredSliderPoses":evaluated.authored_slider_poses(),
                "appliedSliderPoses":evaluated.applied_slider_poses()});
            let scene=checked_scene_from_evaluated(document,&input.skin,evaluated)?;
            let mut result=json!({"revision":state.revision,"animation":input.animation,"time":input.time,"skin":input.skin,"locals":scene.locals,"appliedLocals":scene.applied_locals,"world":scene.world,"attachments":scene.attachments,"bounds":scene.bounds.map(|bounds|json!({"min":bounds.min,"max":bounds.max}))});
            result.as_object_mut().expect("Pose response").extend(effective.as_object().expect("Effective pose response").clone());
            Ok(result)
        })
    }

    #[tool(
        description = "Save the headless document with copied loose images into a NEW directory as project.peachy. All inputs must resolve inside --root. Never overwrites a source or existing output. Successful save marks this session clean and advances revision.",
        annotations(
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn save_project(&self, Parameters(input): Parameters<SaveInput>) -> CallToolResult {
        self.access(|state| {
            state.check_revision(input.expected_revision)?;
            let output = state.root.fresh(&input.directory)?;
            let project = state.project()?;
            state
                .root
                .check_assets(project.session.document(), &project.asset_root)?;
            fs::create_dir(&output).map_err(|e| e.to_string())?;
            let path = output.join("project.peachy");
            if let Err(error) =
                peachy_io::save(&path, project.session.document(), &project.asset_root)
            {
                let _ = fs::remove_dir_all(&output);
                return Err(error);
            }
            let project = state.project_mut()?;
            project.source = Some(path.clone());
            project.asset_root = output;
            project.session.mark_saved();
            state.revision += 1;
            Ok(json!({"path":state.root.display(&path),"state":state.summary()}))
        })
    }

    #[tool(
        description = "Export the current headless document as new Spine 4.3 JSON or a packed JSON/atlas/PNG bundle. target defaults to spineEditor43 and rejects unsafe Editor naming; runtime43 preserves independent attachment names/placeholders. Neither target rewrites native identities. This does not publish a game, change the saved baseline or overwrite files. Bundle images must resolve inside --root.",
        annotations(
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    fn export_project(&self, Parameters(input): Parameters<ExportInput>) -> CallToolResult {
        self.access(|state|{
            state.check_revision(input.expected_revision)?;
            let project=state.project()?;
            let path=match input.kind {
                ExportKind::Json=>{let path=state.root.fresh(&input.path)?;peachy_io::export_json_for(&path,project.session.document(),input.target.adapter())?;path},
                ExportKind::Bundle=>{
                    let parent=state.root.existing(&input.path)?;
                    if !parent.is_dir(){return Err("Bundle path must be an existing directory".into());}
                    state.root.check_assets(project.session.document(),&project.asset_root)?;
                    peachy_io::export_bundle_for(&parent,project.session.document(),&project.asset_root,input.target.adapter())?
                },
            };
            Ok(json!({"path":state.root.display(&path),"revision":state.revision,"targetVersion":peachy_spine::TARGET_VERSION,"exportTarget":input.target.name()}))
        })
    }
}

#[tool_handler(
    name = "peachy-animator",
    version = "0.1.0",
    instructions = "Independent headless Peachy Animator session, not a connection to the desktop. Call get_document for state/revision; create_project for a fresh root-only document, or inspect_file then open_project for an existing one; use commands with expected_revision; copy_animation_keys for portable typed clipboards; evaluate_pose for checked geometry; save_project into a fresh directory. Paths and referenced images are confined to startup --root. Existing artist sources are never overwritten. Unsupported Spine features fail explicitly. No raster rendering or live GUI bridge yet."
)]
impl ServerHandler for AnimatorServer {}
