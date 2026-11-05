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

#[test]
fn contextual_edits_change_only_the_addressed_entity_and_undo_atomically() {
    let before = original();
    let mut session = Session::new(before.clone()).unwrap();
    let head = AttachmentRef {
        skin: "peach".into(),
        slot: "head".into(),
        placeholder: "head".into(),
    };
    session.begin_transaction();
    for x in [2., 4., 9.] {
        session
            .dispatch(Command::SetRegionTransform {
                attachment: head.clone(),
                transform: Transform {
                    x,
                    rotation: 25.,
                    ..Default::default()
                },
            })
            .unwrap();
    }
    session.commit_transaction();
    let edited = session.document().clone();
    assert_eq!(edited.bones, before.bones);
    assert_eq!(edited.slots, before.slots);
    assert_eq!(edited.skins["mint"], before.skins["mint"]);
    session.undo();
    assert_eq!(session.document(), &before);
    session.redo();
    assert_eq!(session.document(), &edited);
    let slot = before.slots.iter().position(|s| s.name == "head").unwrap();
    session
        .dispatch(Command::SetSlotAttachment {
            slot,
            attachment: None,
        })
        .unwrap();
    assert!(
        session
            .document()
            .resolved_attachment("peach", slot)
            .is_none()
    );
    session.undo();
    assert_eq!(session.document(), &edited);
    for command in [
        Command::SetSlotAttachment {
            slot,
            attachment: Some("unknown".into()),
        },
        Command::SetBoneLength {
            bone: 0,
            length: -1.,
        },
        Command::SetBoneLength {
            bone: 0,
            length: f32::NAN,
        },
        Command::SetRegionTransform {
            attachment: head.clone(),
            transform: Transform {
                x: f32::INFINITY,
                ..Default::default()
            },
        },
        Command::Batch {
            commands: vec![
                Command::SetSlotAttachment {
                    slot,
                    attachment: None,
                },
                Command::SetRegionTransform {
                    attachment: hand(),
                    transform: Transform::default(),
                },
                Command::SetSlotAttachment {
                    slot: 99,
                    attachment: None,
                },
            ],
        },
    ] {
        assert!(session.dispatch(command).is_err());
        assert_eq!(session.document(), &edited);
    }
}

#[test]
fn picking_respects_triangles_draw_order_reflections_and_visibility_bounds() {
    let make = |slot, vertices: Vec<[f32; 2]>, triangles: Vec<u32>| AttachmentDraw {
        slot,
        attachment: hand(),
        image: "original".into(),
        hull: vertices.len(),
        uvs: vec![[0., 0.]; vertices.len()],
        vertices,
        triangles,
    };
    let mut draws = vec![
        make(
            0,
            vec![[-1., -1.], [1., -1.], [1., 1.], [-1., 1.]],
            vec![0, 1, 2, 0, 2, 3],
        ),
        make(1, vec![[0., 0.], [2., 0.], [0., 2.]], vec![0, 1, 2]),
    ];
    assert_eq!(pick_attachment(&draws, [0.2, 0.2]).unwrap().slot, 1);
    assert_eq!(pick_attachment(&draws, [-0.5, -0.5]).unwrap().slot, 0);
    assert!(pick_attachment(&draws, [1.5, 1.5]).is_none());
    assert!(pick_attachment(&draws, [f32::NAN, 0.]).is_none());
    for draw in &mut draws {
        for p in &mut draw.vertices {
            p[0] *= -1.;
        }
    }
    assert_eq!(pick_attachment(&draws, [-0.2, 0.2]).unwrap().slot, 1);
    let visible = &draws[..1];
    assert_eq!(pick_attachment(visible, [-0.2, 0.2]).unwrap().slot, 0);
    assert_eq!(attachment_bounds(visible).unwrap().max, [1., 1.]);
    assert_eq!(attachment_bounds(&draws).unwrap().min, [-2., -1.]);
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

#[test]
fn unkeyed_multi_bone_pose_keys_atomically_without_touching_setup() {
    let mut fixture = doc();
    fixture.bones[0].setup.x = 10.0;
    fixture.bones[0].setup.scale_x = -2.0;
    fixture.bones.push(Bone {
        name: "child".into(),
        parent: Some(0),
        length: 10.0,
        setup: Transform::default(),
    });
    let mut session = Session::new(fixture.clone()).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.5,
    };
    let mut pose = session.local_pose(Some(0), 0.5)[0];
    pose.x = 15.0;
    pose.scale_x = -3.0;
    session
        .edit_pose(0, target, pose, &[Property::X, Property::ScaleX], false)
        .unwrap();
    let child = Transform {
        rotation: 75.0,
        ..Default::default()
    };
    session
        .edit_pose(1, target, child, &[Property::Rotation], false)
        .unwrap();
    assert_eq!(session.document(), &fixture);
    assert!(!session.is_dirty());
    assert_eq!(session.local_pose(Some(0), 0.5)[0], pose);
    assert_eq!(session.local_pose(None, 0.5)[0], fixture.bones[0].setup);
    let scene = scene_from_locals(
        session.document(),
        "default",
        session.local_pose(Some(0), 0.5),
    );
    assert_eq!(scene.locals[1].rotation, 75.0);
    assert_eq!(
        session
            .pose_edits()
            .key_state(session.document(), 0, 0.5, 0, &[Property::X, Property::Y]),
        KeyState::Edited
    );
    session.key_edited(0, 0.5, None).unwrap();
    let keyed = session.document().clone();
    assert_eq!(keyed.bones, fixture.bones);
    assert_eq!(keyed.animations[0].tracks.len(), 5); // XY, scale XY, child's rotation.
    assert_eq!(local_pose(&keyed, Some(0), 0.5)[0], pose);
    assert_eq!(local_pose(&keyed, Some(0), 0.5)[1], child);
    assert!(session.pose_edits().is_empty());
    assert_eq!(
        session
            .pose_edits()
            .key_state(&keyed, 0, 0.5, 0, &[Property::X, Property::Y]),
        KeyState::Keyed
    );
    session.undo();
    assert_eq!(session.document(), &fixture);
    assert!(session.pose_edits().has_bone(0, 0) && session.pose_edits().has_bone(0, 1));
    session.redo();
    assert_eq!(session.document(), &keyed);
    assert!(session.pose_edits().is_empty());
}

#[test]
fn timeline_discards_pose_and_undo_recovers_at_new_frame_except_on_redo_branch() {
    let mut fixture = doc();
    fixture = apply_command(
        &fixture,
        Command::Batch {
            commands: vec![key(0.0, 0.0), key(1.0, 90.0)],
        },
    )
    .unwrap();
    let mut session = Session::new(fixture).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.0,
    };
    let pose = Transform {
        rotation: 20.0,
        ..Default::default()
    };
    session
        .edit_pose(0, target, pose, &[Property::Rotation], false)
        .unwrap();
    session.discard_pose(true);
    assert_eq!(session.local_pose(Some(0), 0.5)[0].rotation, 45.0);
    session.undo();
    assert_eq!(session.local_pose(Some(0), 0.5)[0].rotation, 20.0);
    session.key_edited(0, 0.5, None).unwrap();
    assert_eq!(
        local_pose(session.document(), Some(0), 0.5)[0].rotation,
        20.0
    );
    session.undo();
    assert!(!session.pose_edits().is_empty());
    session.discard_pose(true); // Existing redo prevents pose recovery.
    assert!(session.pose_edits().is_empty());
    session.undo();
    assert!(session.pose_edits().is_empty());
}

#[test]
fn transient_gestures_cancel_and_undo_without_losing_redo_or_saved_state() {
    let mut session = Session::new(doc()).unwrap();
    session.dispatch(key(0.0, 10.0)).unwrap();
    session.mark_saved();
    session.undo();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.5,
    };
    session.begin_transaction();
    for x in [1.0, 2.0, 3.0] {
        session
            .edit_pose(
                0,
                target,
                Transform {
                    x,
                    ..Default::default()
                },
                &[Property::X],
                false,
            )
            .unwrap();
    }
    assert!(!session.can_redo());
    session.cancel_transaction();
    assert!(session.can_redo());
    assert!(session.pose_edits().is_empty());
    session.redo();
    assert!(!session.is_dirty());
    session.begin_transaction();
    for x in [1.0, 2.0, 3.0] {
        session
            .edit_pose(
                0,
                target,
                Transform {
                    x,
                    ..Default::default()
                },
                &[Property::X],
                false,
            )
            .unwrap();
    }
    session.commit_transaction();
    session.undo();
    assert!(session.pose_edits().is_empty());
    assert!(!session.is_dirty());
    session.redo();
    assert_eq!(session.local_pose(Some(0), 0.5)[0].x, 3.0);
    session.mark_saved();
    assert!(!session.is_dirty());
    assert!(!session.pose_edits().is_empty()); // Saving must not pretend to key/discard the pose.
}

#[test]
fn manual_key_is_independent_of_auto_key_and_does_not_consume_other_edits() {
    let mut session = Session::new(doc()).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.5,
    };
    let pose = Transform {
        x: 5.0,
        rotation: 20.0,
        ..Default::default()
    };
    session
        .edit_pose(0, target, pose, &[Property::X, Property::Rotation], false)
        .unwrap();
    session.key_pose(0, 0.5, 0, &[Property::Rotation]).unwrap();
    assert_eq!(session.document().animations[0].tracks.len(), 1);
    assert_eq!(
        session
            .pose_edits()
            .key_state(session.document(), 0, 0.5, 0, &[Property::Rotation]),
        KeyState::Keyed
    );
    assert_eq!(
        session
            .pose_edits()
            .key_state(session.document(), 0, 0.5, 0, &[Property::X, Property::Y]),
        KeyState::Edited
    );
    let mut translated = pose;
    translated.x += 1.0;
    session
        .edit_pose(0, target, translated, &[Property::X], true)
        .unwrap();
    assert!(session.pose_edits().is_empty());
    assert_eq!(session.document().animations[0].tracks.len(), 3);
    assert_eq!(local_pose(session.document(), Some(0), 0.5)[0], translated);
    session.undo();
    assert_eq!(session.local_pose(Some(0), 0.5)[0], pose);
    assert!(!session.pose_edits().is_empty());
}

#[test]
fn failed_key_edited_keeps_every_bone_pose_and_history_intact() {
    let mut fixture = doc();
    fixture.bones[0].setup.scale_y = 0.0;
    let mut session = Session::new(fixture.clone()).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.5,
    };
    let pose = Transform {
        scale_x: 2.0,
        scale_y: 0.0,
        ..Default::default()
    };
    session
        .edit_pose(0, target, pose, &[Property::ScaleX], false)
        .unwrap();
    let pending = session.pose_edits().clone();
    assert!(session.key_edited(0, 0.5, None).is_err()); // Default paired scale includes singular Y.
    assert_eq!(session.document(), &fixture);
    assert_eq!(session.pose_edits(), &pending);
    let invalid = Transform {
        x: f32::NAN,
        ..pose
    };
    assert!(
        session
            .edit_pose(0, target, invalid, &[Property::X], false)
            .is_err()
    );
    assert_eq!(session.pose_edits(), &pending);
    session.undo();
    assert!(session.pose_edits().is_empty()); // Failure added no undo entry.
}

#[test]
fn returning_to_sampled_values_clears_orange_state_and_clip_switch_drops_old_pose_history() {
    let mut session = Session::new(doc()).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.5,
    };
    let pose = Transform {
        x: 4.0,
        ..Default::default()
    };
    session
        .edit_pose(0, target, pose, &[Property::X], false)
        .unwrap();
    session
        .edit_pose(0, target, Transform::default(), &[Property::X], false)
        .unwrap();
    assert!(session.pose_edits().is_empty());
    session.undo();
    assert!(!session.pose_edits().is_empty());
    session.discard_pose(false);
    session.undo();
    assert!(session.pose_edits().is_empty());
    assert_eq!(session.document(), &doc());
}
