use super::*;
use peachy_document::*;

#[test]
fn invalid_delete_times_cannot_erase_keys_or_change_history() {
    let mut session = Session::new(doc()).unwrap();
    session
        .dispatch(Command::Batch {
            commands: vec![key(0.0, 10.0), key(1.0, 90.0)],
        })
        .unwrap();
    let before = session.document().clone();
    for time in [f32::NAN, f32::INFINITY, -1.0, 2.0] {
        assert!(
            session
                .dispatch(Command::DeleteKey {
                    animation: 0,
                    bone: 0,
                    property: Property::Rotation,
                    time
                })
                .is_err()
        );
        assert_eq!(session.document(), &before);
    }
    session.undo();
    assert_eq!(session.document(), &doc());
}

#[test]
fn pure_pose_edits_snap_frames_keep_curves_and_convert_setup_offsets() {
    let mut before = doc();
    before.bones[0].setup = Transform {
        x: 10.0,
        rotation: 45.0,
        scale_x: 2.0,
        ..Default::default()
    };
    let commands = vec![Command::SetKey {
        animation: 0,
        bone: 0,
        property: Property::Rotation,
        key: Keyframe {
            time: 0.5,
            value: 0.0,
            curve: Curve::Stepped,
        },
    }];
    before = apply_command(&before, Command::Batch { commands }).unwrap();
    let after = apply_command(
        &before,
        Command::SetPose {
            bone: 0,
            target: EditTarget::Animate {
                animation: 0,
                time: 0.501,
            },
            transform: Transform {
                x: 13.0,
                rotation: 75.0,
                scale_x: 3.0,
                ..Default::default()
            },
            properties: vec![Property::X, Property::Rotation, Property::ScaleX],
        },
    )
    .unwrap();
    assert_eq!(before.animations[0].tracks.len(), 1);
    let pose = local_pose(&after, Some(0), 0.5);
    assert_eq!(pose[0].x, 13.0);
    assert_eq!(pose[0].rotation, 75.0);
    assert_eq!(pose[0].scale_x, 3.0);
    assert_eq!(after.animations[0].tracks[0].keys[0].curve, Curve::Stepped);
    assert!(
        after.animations[0]
            .tracks
            .iter()
            .all(|t| t.keys[0].time == 0.5)
    );
    let mut singular = before.clone();
    singular.bones[0].setup.scale_x = 0.0;
    assert!(
        apply_command(
            &singular,
            Command::SetPose {
                bone: 0,
                target: EditTarget::Animate {
                    animation: 0,
                    time: 0.5
                },
                transform: Transform::default(),
                properties: vec![Property::X, Property::ScaleX]
            }
        )
        .is_err()
    );
}

#[test]
fn gestures_have_one_undo_step_and_cancel_preserves_the_redo_branch() {
    let mut session = Session::new(doc()).unwrap();
    session.dispatch(key(0.0, 5.0)).unwrap();
    let start = session.document().clone();
    session.begin_transaction();
    for angle in [10.0, 20.0, 30.0] {
        session.dispatch(key(0.0, angle)).unwrap();
    }
    session.commit_transaction();
    let final_doc = session.document().clone();
    session.undo();
    assert_eq!(session.document(), &start);
    session.begin_transaction();
    session.dispatch(key(0.0, 99.0)).unwrap();
    assert!(!session.can_redo());
    session.cancel_transaction();
    assert!(session.can_redo());
    session.redo();
    assert_eq!(session.document(), &final_doc);
    session.undo();
    session.begin_transaction();
    session.dispatch(key(0.0, 42.0)).unwrap();
    session.commit_transaction();
    assert!(!session.can_redo());
}

#[test]
fn scene_uses_skin_fallback_draw_order_and_one_geometry_for_bounds() {
    let fixture: Document =
        serde_json::from_str(include_str!("../../../examples/peach-bot/peach-bot.peachy")).unwrap();
    fixture.validate().unwrap();
    let peach = evaluate_scene(&fixture, Some(0), 0.3, "peach", None);
    let mint = evaluate_scene(&fixture, Some(0), 0.3, "mint", None);
    assert_eq!(peach.attachments.len(), 14);
    assert_eq!(mint.attachments.len(), 14);
    assert!(mint.attachments.iter().any(|r| r.image == "torso-mint"));
    assert!(mint.attachments.iter().any(|r| r.image == "foot"));
    assert!(mint.attachments.windows(2).all(|w| w[0].slot < w[1].slot));
    let bounds = mint.bounds.unwrap();
    for p in mint
        .attachments
        .iter()
        .flat_map(|r| r.vertices.iter().copied())
    {
        for (i, value) in p.into_iter().enumerate() {
            assert!(value >= bounds.min[i] && value <= bounds.max[i]);
        }
    }
    let mut changed = fixture.bones[0].setup;
    changed.x += 20.0;
    let preview = evaluate_scene(&fixture, Some(0), 0.3, "mint", Some((0, changed)));
    assert!(
        (preview.attachments[0].vertices[0][0] - mint.attachments[0].vertices[0][0] - 20.0).abs()
            < 0.001
    );
    assert_eq!(fixture.bones[0].setup.x, 0.0);
}
fn doc() -> Document {
    Document {
        format: FORMAT_VERSION,
        name: "fixture".into(),
        fps: 30,
        bones: vec![Bone {
            name: "root".into(),
            parent: None,
            length: 50.0,
            setup: Transform::default(),
        }],
        slots: vec![],
        skins: Default::default(),
        animations: vec![Animation {
            name: "wave".into(),
            duration: 1.0,
            tracks: vec![],
        }],
    }
}
fn key(time: f32, value: f32) -> Command {
    Command::SetKey {
        animation: 0,
        bone: 0,
        property: Property::Rotation,
        key: Keyframe {
            time,
            value,
            curve: Curve::Linear,
        },
    }
}
#[test]
fn commands_undo_redo_and_failed_batches_are_atomic() {
    let mut s = Session::new(doc()).unwrap();
    let original = s.document().clone();
    assert!(
        s.dispatch(Command::Batch {
            commands: vec![key(0.0, 20.0), key(2.0, 90.0)]
        })
        .is_err()
    );
    assert_eq!(*s.document(), original);
    assert!(!s.can_undo());
    s.dispatch(key(0.0, 20.0)).unwrap();
    assert!(s.is_dirty());
    s.undo();
    assert_eq!(*s.document(), original);
    assert!(!s.is_dirty());
    s.redo();
    assert_eq!(local_pose(s.document(), Some(0), 0.0)[0].rotation, 20.0);
    s.mark_saved();
    assert!(!s.is_dirty());
    s.undo();
    assert!(s.is_dirty());
    s.dispatch(key(0.0, 30.0)).unwrap();
    assert!(!s.can_redo());
}
#[test]
fn interpolation_is_deterministic_when_scrubbing_backwards() {
    let mut s = Session::new(doc()).unwrap();
    s.dispatch(Command::Batch {
        commands: vec![key(0.0, 0.0), key(1.0, 90.0)],
    })
    .unwrap();
    assert_eq!(local_pose(s.document(), Some(0), 0.5)[0].rotation, 45.0);
    let a = evaluate(s.document(), Some(0), 0.25);
    evaluate(s.document(), Some(0), 0.9);
    assert_eq!(a[0].a, evaluate(s.document(), Some(0), 0.25)[0].a);
}
#[test]
fn hierarchy_composes_rotation_and_nonuniform_scale() {
    let mut d = doc();
    d.bones[0].setup.rotation = 90.0;
    d.bones[0].setup.scale_x = 2.0;
    d.bones.push(Bone {
        name: "child".into(),
        parent: Some(0),
        length: 10.0,
        setup: Transform {
            x: 10.0,
            ..Default::default()
        },
    });
    let w = evaluate(&d, None, 0.0);
    assert!(w[1].x.abs() < 0.0001);
    assert!((w[1].y - 20.0).abs() < 0.0001);
    let p = w[0].inverse_point(w[1].x, w[1].y).unwrap();
    assert!((p[0] - 10.0).abs() < 0.0001);
}
#[test]
fn stepped_and_bezier_curves() {
    assert_eq!(curve_progress(Curve::Stepped, 0.8), 0.0);
    assert!((curve_progress(Curve::Bezier([0.42, 0.0, 0.58, 1.0]), 0.5) - 0.5).abs() < 0.0001);
    assert!(curve_progress(Curve::Bezier([0.42, 0.0, 0.58, 1.0]), 0.2) < 0.2);
}
#[test]
fn validates_cycles_and_nonfinite_transforms() {
    let mut d = doc();
    d.bones[0].parent = Some(0);
    assert!(d.validate().is_err());
    d.bones[0].parent = None;
    d.bones[0].setup.x = f32::INFINITY;
    assert!(d.validate().is_err());
}

fn original() -> Document {
    serde_json::from_str(include_str!("../../../examples/peach-bot/peach-bot.peachy")).unwrap()
}
fn hand() -> AttachmentRef {
    AttachmentRef {
        skin: "default".into(),
        slot: "hand.R".into(),
        placeholder: "hand.R".into(),
    }
}

#[test]
fn mesh_conversion_preserves_textured_geometry_under_animation_and_reflection() {
    let mut before = original();
    before.bones[0].setup.rotation = 37.0;
    before.bones[0].setup.scale_x = -1.3;
    before.bones[0].setup.scale_y = 0.8;
    let after =
        apply_command(&before, Command::ConvertRegionToMesh { attachment: hand() }).unwrap();
    assert!(matches!(
        before.attachment(&hand()),
        Some(Attachment::Region(_))
    ));
    assert!(matches!(
        after.attachment(&hand()),
        Some(Attachment::Mesh(_))
    ));
    for time in [0.0, 0.3, 1.1, 2.4] {
        let a = evaluate_scene(&before, Some(0), time, "peach", None);
        let b = evaluate_scene(&after, Some(0), time, "peach", None);
        for (a, b) in a.attachments.iter().zip(b.attachments) {
            assert_eq!(a.vertices, b.vertices);
            assert_eq!(a.uvs, b.uvs);
            assert_eq!(a.triangles, b.triangles);
        }
        assert_eq!(a.bounds, b.bounds);
    }
}

#[test]
fn subdivision_preserves_uv_mapping_hull_and_undo_and_rejects_invalid_batches() {
    let mut session = Session::new(original()).unwrap();
    session
        .dispatch(Command::ConvertRegionToMesh { attachment: hand() })
        .unwrap();
    let converted = session.document().clone();
    session
        .dispatch(Command::SplitMeshTriangle {
            attachment: hand(),
            triangle: 0,
        })
        .unwrap();
    let Some(Attachment::Mesh(mesh)) = session.document().attachment(&hand()) else {
        panic!()
    };
    assert_eq!(mesh.hull, 4);
    assert_eq!(mesh.vertices.len(), 5);
    assert_eq!(mesh.triangles.len(), 12);
    assert_eq!(mesh.uvs[4], [2.0 / 3.0, 2.0 / 3.0]);
    let split = session.document().clone();
    session.begin_transaction();
    for x in [2.0, 4.0, 9.0] {
        session
            .dispatch(Command::SetMeshVertex {
                attachment: hand(),
                vertex: 4,
                position: [x, 2.0],
            })
            .unwrap();
    }
    session.commit_transaction();
    session.undo();
    assert_eq!(session.document(), &split);
    session.undo();
    assert_eq!(session.document(), &converted);
    session.redo();
    assert_eq!(session.document(), &split);
    for command in [
        Command::SetMeshVertex {
            attachment: hand(),
            vertex: 99,
            position: [0., 0.],
        },
        Command::SetMeshVertex {
            attachment: hand(),
            vertex: 0,
            position: [f32::NAN, 0.],
        },
        Command::SplitMeshTriangle {
            attachment: hand(),
            triangle: usize::MAX,
        },
        Command::Batch {
            commands: vec![
                Command::SetMeshVertex {
                    attachment: hand(),
                    vertex: 0,
                    position: [4., 5.],
                },
                Command::SplitMeshTriangle {
                    attachment: hand(),
                    triangle: 99,
                },
            ],
        },
    ] {
        assert!(session.dispatch(command).is_err());
        assert_eq!(session.document(), &split);
    }
}

#[test]
fn explicit_attachment_keys_do_not_edit_named_skin_fallbacks() {
    let before = original();
    let (_, fallback) = before.resolved_attachment("mint", 13).unwrap();
    assert_eq!(fallback.path(), "hand");
    let key = AttachmentRef {
        skin: "mint".into(),
        ..hand()
    };
    assert!(apply_command(&before, Command::ConvertRegionToMesh { attachment: key }).is_err());
    let after =
        apply_command(&before, Command::ConvertRegionToMesh { attachment: hand() }).unwrap();
    assert!(matches!(
        after.resolved_attachment("mint", 13).unwrap().1,
        Attachment::Mesh(_)
    ));
}
