//! Headless MCP shell over the same command/session engine as the desktop.
mod paths;
mod schema;

use paths::WorkspaceRoot;
use peachy_engine::{Command, Session, checked_evaluate_scene};
use rmcp::{
    ServerHandler, handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_handler,
    tool_router,
};
use schema::CommandInput;
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

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ExportInput {
    expected_revision: u64,
    kind: ExportKind,
    /// For json: a new .json file. For bundle: an existing parent directory; a new timestamped bundle is created within it.
    path: String,
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
        description = "Return the supported native command JSON Schema. apply_commands exposes the same schema inline. Bind operations require appliedLocals from evaluate_pose; key values are setup-relative offsets (scale multiplies).",
        annotations(read_only_hint = true, idempotent_hint = true, open_world_hint = false)
    )]
    fn get_command_schema(&self) -> CallToolResult {
        CallToolResult::structured(
            json!({"nativeFormat":peachy_document::FORMAT_VERSION,"schema":schema::command_schema()}),
        )
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
        description = "Evaluate Setup or an animation at a time using the shared checked pose/IK/mesh engine. Returns authored locals, appliedLocals, world transforms, textured geometry and bounds. This is geometry, not a raster preview or live desktop pose.",
        annotations(read_only_hint = true, idempotent_hint = true, open_world_hint = false)
    )]
    fn evaluate_pose(&self, Parameters(input): Parameters<EvaluateInput>) -> CallToolResult {
        self.access(|state|{
            let document=state.project()?.session.document();
            let animation=input.animation.as_ref().map(|name|document.animations.iter().position(|clip|&clip.name==name).ok_or("Unknown animation")).transpose()?;
            if !document.skins.is_empty() && !document.skins.contains_key(&input.skin){return Err("Unknown skin".into());}
            let scene=checked_evaluate_scene(document,animation,input.time,&input.skin,None)?;
            Ok(json!({"revision":state.revision,"animation":input.animation,"time":input.time,"skin":input.skin,"locals":scene.locals,"appliedLocals":scene.applied_locals,"world":scene.world,"attachments":scene.attachments,"bounds":scene.bounds.map(|bounds|json!({"min":bounds.min,"max":bounds.max}))}))
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
        description = "Export the current headless document as new Spine 4.3 JSON or a packed JSON/atlas/PNG bundle. This does not publish a game, change the saved baseline or overwrite files. Bundle images must resolve inside --root.",
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
                ExportKind::Json=>{let path=state.root.fresh(&input.path)?;peachy_io::export_json(&path,project.session.document())?;path},
                ExportKind::Bundle=>{
                    let parent=state.root.existing(&input.path)?;
                    if !parent.is_dir(){return Err("Bundle path must be an existing directory".into());}
                    state.root.check_assets(project.session.document(),&project.asset_root)?;
                    peachy_io::export_bundle(&parent,project.session.document(),&project.asset_root)?
                },
            };
            Ok(json!({"path":state.root.display(&path),"revision":state.revision,"targetVersion":peachy_spine::TARGET_VERSION}))
        })
    }
}

#[tool_handler(
    name = "peachy-animator",
    version = "0.1.0",
    instructions = "Independent headless Peachy Animator session, not a connection to the desktop. Call get_document for state/revision; create_project for a fresh root-only document, or inspect_file then open_project for an existing one; use commands with expected_revision; evaluate_pose for checked geometry; save_project into a fresh directory. Paths and referenced images are confined to startup --root. Existing artist sources are never overwritten. Unsupported Spine features fail explicitly. No raster rendering or live GUI bridge yet."
)]
impl ServerHandler for AnimatorServer {}
