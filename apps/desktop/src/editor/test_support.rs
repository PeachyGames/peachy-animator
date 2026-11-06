//! Shared original-document editor fixture for shell regression tests.
use super::dopesheet_selection::KeyId;
use super::*;

pub(super) fn editor_fixture() -> Editor {
    let mut doc: Document = serde_json::from_str(include_str!(
        "../../../../examples/peach-bot/peach-bot.peachy"
    ))
    .unwrap();
    let track = |bone, property, times: &[f32]| Track {
        bone,
        property,
        keys: times
            .iter()
            .map(|&time| Keyframe {
                time,
                value: property.neutral(),
                curve: Curve::Linear,
            })
            .collect(),
    };
    doc.animations[0].tracks = vec![
        track(8, Property::Rotation, &[0., 0.3, 0.300005, 0.6]),
        track(8, Property::X, &[0., 0.4]),
        track(8, Property::Y, &[0.2]),
        track(8, Property::ScaleX, &[0., 0.8]),
        track(7, Property::Rotation, &[0., 0.5]),
    ];
    let tree = TreeState::for_document(&doc, "peach");
    let mut editor = Editor {
        session: Session::new(doc).unwrap(),
        project_path: None,
        asset_root: PathBuf::new(),
        textures: BTreeMap::new(),
        selection: Some(TreeItem::Bone(8)),
        selection_history: Default::default(),
        key_selection: Default::default(),
        key_drag: None,
        timeline_clipboard: None,
        timeline_input: Default::default(),
        tree,
        hidden_bones: BTreeSet::new(),
        hidden_slots: BTreeSet::new(),
        animation: 0,
        skin: "peach".into(),
        setup_mode: false,
        playing: false,
        play_direction: PlayDirection::Forward,
        looping: true,
        auto_key: false,
        time: 0.42,
        pose_view_generation: 0,
        show_bones: true,
        zoom: 1.,
        pan: Vec2::ZERO,
        fit: false,
        tool: Tool::Rotate,
        last_tool: Tool::Move,
        axes: Axes::Parent,
        show_tree: true,
        show_dopesheet: true,
        hidden_views: None,
        drag: None,
        mesh_vertex: 0,
        mesh_triangle: 0,
        mesh_drag: None,
        capture_targets: BTreeMap::new(),
        mesh_handles: Vec::new(),
        status: String::new(),
        error: None,
        new_bone: String::new(),
        rename: None,
        duplicate_animation: None,
        capture: None,
        capture_directory: PathBuf::new(),
        frames: 0,
    };
    editor.reconcile_selection();
    select_rotation(&mut editor);
    editor
}

pub(super) fn select_rotation(editor: &mut Editor) {
    editor.key_selection.select(
        KeyId {
            property: Property::Rotation,
            time: 0.3,
        },
        false,
    );
}
