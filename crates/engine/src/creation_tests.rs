use crate::*;
use peachy_document::*;

fn slot(name: &str, bone: usize) -> Slot {
    Slot {
        name: name.into(),
        bone,
        attachment: None,
        color: [255; 4],
        blend: BlendMode::Normal,
    }
}

fn reference(slot: &str) -> AttachmentRef {
    AttachmentRef {
        skin: "default".into(),
        slot: slot.into(),
        placeholder: slot.into(),
    }
}

fn region() -> Region {
    Region {
        name: None,
        path: "original/ribbon".into(),
        width: 20.,
        height: 10.,
        transform: RegionTransform::default(),
        color: [255; 4],
        sequence: None,
    }
}

fn creation_commands() -> Vec<Command> {
    let mut mesh = region_to_mesh(&region());
    let positions = mesh.slot_local_positions().unwrap().to_vec();
    // Creation accepts explicitly authored, valid weights. Each influence uses
    // its own bone-local bind point; the child has a setup X translation of10.
    mesh.vertices = MeshVertices::Weighted {
        influences: positions
            .iter()
            .map(|&position| {
                vec![
                    BoneInfluence {
                        bone: 0,
                        position,
                        weight: 0.25,
                    },
                    BoneInfluence {
                        bone: 1,
                        position: [position[0] - 10., position[1]],
                        weight: 0.75,
                    },
                ]
            })
            .collect(),
    };
    vec![
        Command::AddBone {
            bone: Bone {
                name: "ribbon".into(),
                parent: Some(0),
                length: 20.,
                setup: Transform {
                    x: 10.,
                    ..Default::default()
                },
                inherit: Inherit::Normal,
            },
        },
        Command::AddSlot {
            slot: slot("body", 0),
        },
        Command::AddAttachment {
            attachment: reference("body"),
            value: Attachment::Region(region()),
        },
        Command::SetSlotAttachment {
            slot: 0,
            attachment: Some("body".into()),
        },
        Command::AddSlot {
            slot: slot("ribbon", 1),
        },
        Command::AddAttachment {
            attachment: reference("ribbon"),
            value: Attachment::Mesh(mesh),
        },
        Command::SetSlotAttachment {
            slot: 1,
            attachment: Some("ribbon".into()),
        },
    ]
}

#[test]
fn creation_root_factory_is_empty_valid_and_uses_current_native_format() {
    let document = Document::new("Authored scene".into(), 30).unwrap();
    assert_eq!(document.format, FORMAT_VERSION);
    assert_eq!(document.bones.len(), 1);
    assert_eq!(document.bones[0].name, "root");
    assert_eq!(document.bones[0].parent, None);
    assert_eq!(document.bones[0].setup, Transform::default());
    assert!(document.slots.is_empty());
    assert!(document.constraints.is_empty());
    assert!(document.animations.is_empty());
    assert_eq!(document.skins.len(), 1);
    assert!(document.skins["default"].is_empty());
    assert!(document.image_paths().is_empty());
    let scene = checked_evaluate_scene(&document, None, 0., "default", None).unwrap();
    assert!(scene.attachments.is_empty());
    assert!(scene.bounds.is_none());
    for (name, fps) in [("", 30), (" \t", 30), ("Scene", 0), ("Scene", 241)] {
        assert!(Document::new(name.into(), fps).is_err());
    }
}

#[test]
fn creation_from_root_to_regions_and_weighted_mesh_is_one_undoable_batch() {
    let original = Document::new("Authored scene".into(), 30).unwrap();
    let mut session = Session::new(original.clone()).unwrap();
    session
        .dispatch(Command::Batch {
            commands: creation_commands(),
        })
        .unwrap();
    let created = session.document().clone();
    assert_eq!(created.bones.len(), 2);
    assert_eq!(
        created
            .slots
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        vec!["body", "ribbon"]
    );
    let scene = checked_evaluate_scene(&created, None, 0., "default", None).unwrap();
    assert_eq!(scene.attachments.len(), 2);
    assert_eq!(scene.attachments[0].slot, 0);
    assert_eq!(scene.attachments[1].slot, 1);
    assert_eq!(
        scene.attachments[1].vertices,
        region_to_mesh(&region()).slot_local_positions().unwrap()
    );
    assert!(session.is_dirty());
    session.undo();
    assert_eq!(session.document(), &original);
    assert!(!session.can_undo());
    session.redo();
    assert_eq!(session.document(), &created);

    // A newly authored mesh can immediately receive keys and evaluate; no file
    // import or whole-document replacement is needed to start animation.
    session
        .dispatch(Command::AddAnimation {
            name: "wind".into(),
            duration: 0.,
        })
        .unwrap();
    session
        .dispatch(Command::SetKey {
            animation: 0,
            bone: 1,
            property: Property::Rotation,
            key: Keyframe {
                time: 1.,
                value: 15.,
                curve: Curve::Linear,
            },
        })
        .unwrap();
    let animated =
        checked_evaluate_scene(session.document(), Some(0), 1., "default", None).unwrap();
    assert_ne!(
        animated.attachments[1].vertices,
        scene.attachments[1].vertices
    );
    assert_eq!(session.document().skins, created.skins);
}

#[test]
fn creation_never_overwrites_artwork_or_creates_unknown_skin_slot_or_bone() {
    let document = apply_command(
        &Document::new("Authored scene".into(), 30).unwrap(),
        Command::Batch {
            commands: creation_commands(),
        },
    )
    .unwrap();
    let mut invalid = vec![
        Command::AddSlot {
            slot: slot("body", 0),
        },
        Command::AddSlot {
            slot: slot("missing-bone", 99),
        },
        Command::AddSlot {
            slot: Slot {
                attachment: Some("body".into()),
                ..slot("preset", 0)
            },
        },
        Command::AddAttachment {
            attachment: reference("body"),
            value: Attachment::Region(region()),
        },
        Command::AddAttachment {
            attachment: reference("missing-slot"),
            value: Attachment::Region(region()),
        },
        Command::AddAttachment {
            attachment: AttachmentRef {
                skin: "missing-skin".into(),
                ..reference("body")
            },
            value: Attachment::Region(region()),
        },
        Command::AddAttachment {
            attachment: AttachmentRef {
                placeholder: " ".into(),
                ..reference("body")
            },
            value: Attachment::Region(region()),
        },
    ];
    let mut bad_path = region();
    bad_path.path = "../outside-root".into();
    invalid.push(Command::AddAttachment {
        attachment: AttachmentRef {
            placeholder: "bad-path".into(),
            ..reference("body")
        },
        value: Attachment::Region(bad_path),
    });
    let mut bad_mesh = region_to_mesh(&region());
    bad_mesh.vertices = MeshVertices::Weighted {
        influences: vec![
            vec![BoneInfluence {
                bone: 99,
                position: [0., 0.],
                weight: 1.
            }];
            4
        ],
    };
    invalid.push(Command::AddAttachment {
        attachment: AttachmentRef {
            placeholder: "bad-bone".into(),
            ..reference("body")
        },
        value: Attachment::Mesh(bad_mesh),
    });
    let mut bad_topology = region_to_mesh(&region());
    bad_topology.triangles[0] = 999;
    invalid.push(Command::AddAttachment {
        attachment: AttachmentRef {
            placeholder: "bad-topology".into(),
            ..reference("body")
        },
        value: Attachment::Mesh(bad_topology),
    });

    let mut session = Session::new(document.clone()).unwrap();
    session
        .dispatch(Command::SetBoneLength {
            bone: 0,
            length: 42.,
        })
        .unwrap();
    session.undo();
    for command in invalid {
        assert!(session.dispatch(command).is_err());
        assert_eq!(session.document(), &document);
        assert!(!session.can_undo());
        assert!(session.can_redo());
    }
    assert!(
        session
            .dispatch(Command::Batch {
                commands: vec![
                    Command::AddSlot {
                        slot: slot("temporary", 0)
                    },
                    Command::AddAttachment {
                        attachment: AttachmentRef {
                            skin: "missing".into(),
                            ..reference("temporary")
                        },
                        value: Attachment::Region(region())
                    },
                ]
            })
            .is_err()
    );
    assert_eq!(session.document(), &document);
    assert!(session.can_redo());
}

#[test]
fn creation_commands_reject_unknown_fields_through_native_serde() {
    let command = Command::AddAttachment {
        attachment: reference("new"),
        value: Attachment::Region(region()),
    };
    let value = serde_json::to_value(command).unwrap();
    assert!(serde_json::from_value::<Command>(value.clone()).is_ok());
    let mut unknown = value.clone();
    unknown["future"] = serde_json::json!(true);
    assert!(serde_json::from_value::<Command>(unknown).is_err());
    let mut unknown_region = value;
    unknown_region["value"]["future"] = serde_json::json!(true);
    assert!(serde_json::from_value::<Command>(unknown_region).is_err());
    let mut unknown_slot = serde_json::to_value(Command::AddSlot {
        slot: slot("new", 0),
    })
    .unwrap();
    unknown_slot["slot"]["future"] = serde_json::json!(true);
    assert!(serde_json::from_value::<Command>(unknown_slot).is_err());
}
