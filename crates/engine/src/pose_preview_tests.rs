use super::*;
use peachy_document::*;

fn fixture() -> Document {
    Document {
        constraints: vec![],
        format: FORMAT_VERSION,
        name: "Original pose preview fixture".into(),
        fps: 30,
        bones: vec![
            Bone {
                name: "root".into(),
                parent: None,
                length: 20.,
                setup: Transform {
                    x: 3.,
                    y: -2.,
                    rotation: 10.,
                    scale_x: 1.25,
                    scale_y: 0.8,
                    shear_x: 0.0,
                    shear_y: 0.0,
                },
            },
            Bone {
                name: "child".into(),
                parent: Some(0),
                length: 10.,
                setup: Transform {
                    x: 7.,
                    y: 4.,
                    rotation: -5.,
                    scale_x: 1.5,
                    scale_y: 1.,
                    shear_x: 0.0,
                    shear_y: 0.0,
                },
            },
        ],
        slots: vec![],
        skins: Default::default(),
        animations: vec![Animation {
            name: "pose".into(),
            tracks: vec![
                Track {
                    bone: 0,
                    property: Property::X,
                    keys: vec![
                        Keyframe {
                            time: 0.4,
                            value: 4.,
                            curve: Curve::Bezier([0.2, 0.1, 0.7, 0.9]),
                        },
                        Keyframe {
                            time: 0.400005,
                            value: 6.,
                            curve: Curve::Stepped,
                        },
                        Keyframe {
                            time: 1.,
                            value: 8.,
                            curve: Curve::Linear,
                        },
                    ],
                },
                Track {
                    bone: 0,
                    property: Property::Y,
                    keys: vec![
                        Keyframe {
                            time: 0.4,
                            value: -3.,
                            curve: Curve::Stepped,
                        },
                        Keyframe {
                            time: 1.,
                            value: 2.,
                            curve: Curve::Linear,
                        },
                    ],
                },
            ],
        }],
    }
}

fn animate(time: f32) -> EditTarget {
    EditTarget::Animate { animation: 0, time }
}

fn pending_session() -> Session {
    let mut session = Session::new(fixture()).unwrap();
    let mut root = session.local_pose(Some(0), 0.407)[0];
    root.x = 25.;
    root.y = 17.;
    root.rotation = 50.;
    session
        .edit_pose(
            0,
            animate(0.407),
            root,
            &[Property::X, Property::Y, Property::Rotation],
            false,
        )
        .unwrap();
    let mut child = session.local_pose(Some(0), 0.407)[1];
    child.rotation = 77.;
    session
        .edit_pose(1, animate(0.407), child, &[Property::Rotation], false)
        .unwrap();
    session
}

fn track(session: &Session, property: Property) -> &Track {
    session.document().animations[0]
        .tracks
        .iter()
        .find(|track| track.bone == 0 && track.property == property)
        .unwrap()
}

#[test]
fn unkeyed_previews_restore_all_baseline_pending_properties_at_fractional_time() {
    let mut session = pending_session();
    let document = session.document().clone();
    let pending = session.pose_edits().clone();
    let locals = session.local_pose(Some(0), 0.407);
    session.begin_transaction();
    let mut root = locals[0];
    root.rotation = 91.;
    session
        .preview_edit_pose(0, animate(0.407), root, &[Property::Rotation], false)
        .unwrap();
    assert_eq!(session.local_pose(Some(0), 0.407)[0].rotation, 91.);
    assert_eq!(session.local_pose(Some(0), 0.407)[1], locals[1]);
    assert_eq!(session.document(), &document);
    assert!(!session.is_dirty());

    // Replacing the edited property must restore the original pending rotation,
    // rather than carrying the previous preview into the new candidate.
    root.scale_y = 2.;
    session
        .preview_edit_pose(0, animate(0.407), root, &[Property::ScaleY], false)
        .unwrap();
    assert_eq!(session.local_pose(Some(0), 0.407)[0].rotation, 50.);
    assert_eq!(session.local_pose(Some(0), 0.407)[0].scale_y, 2.);
    assert_eq!(session.local_pose(Some(0), 0.407)[0].x, 25.);
    assert_eq!(session.document().animations, document.animations);

    // A different explicit bone also starts from the full baseline pose.
    let mut child = locals[1];
    child.rotation = -31.;
    session
        .preview_edit_pose(1, animate(0.407), child, &[Property::Rotation], false)
        .unwrap();
    assert_eq!(session.local_pose(Some(0), 0.407)[0], locals[0]);
    assert_eq!(session.local_pose(Some(0), 0.407)[1].rotation, -31.);
    let preview = session.pose_edits().clone();
    let snapshot = session.mesh_bind_pose(animate(0.407)).unwrap();
    assert_eq!(snapshot.locals, session.local_pose(Some(0), 0.407));
    assert_eq!(session.pose_edits(), &preview);
    session.commit_transaction();
    session.undo();
    assert_eq!(session.document(), &document);
    assert_eq!(session.pose_edits(), &pending);
    session.redo();
    assert_eq!(session.pose_edits(), &preview);
    assert!(!session.is_dirty()); // No keys or setup edits were created.
}

#[test]
fn keyed_previews_restore_overwritten_keys_curves_and_pending_pairs_from_the_base() {
    let mut session = pending_session();
    let document = session.document().clone();
    let pending = session.pose_edits().clone();
    let mut root = session.local_pose(Some(0), 0.407)[0];
    session.begin_transaction();
    root.x = 33.;
    session
        .preview_edit_pose(0, animate(0.407), root, &[Property::X], true)
        .unwrap();
    assert_eq!(track(&session, Property::X).keys[0].time, 0.4);
    assert_eq!(track(&session, Property::X).keys[0].value, 30.);
    assert_eq!(
        track(&session, Property::X).keys[0].curve,
        document.animations[0].tracks[0].keys[0].curve
    );
    assert_eq!(
        track(&session, Property::X).keys[1],
        document.animations[0].tracks[0].keys[1]
    );
    assert_eq!(track(&session, Property::Y).keys[0].value, 19.);
    assert_eq!(track(&session, Property::Y).keys[0].curve, Curve::Stepped);
    assert_eq!(session.local_pose(Some(0), 0.407)[1].rotation, 77.);

    root.rotation = 81.;
    session
        .preview_edit_pose(0, animate(0.407), root, &[Property::Rotation], true)
        .unwrap();
    assert_eq!(
        track(&session, Property::X),
        &document.animations[0].tracks[0]
    );
    assert_eq!(
        track(&session, Property::Y),
        &document.animations[0].tracks[1]
    );
    assert_eq!(session.local_pose(Some(0), 0.407)[0].x, 25.);
    assert_eq!(session.local_pose(Some(0), 0.407)[0].y, 17.);
    assert_eq!(track(&session, Property::Rotation).keys[0].value, 71.);

    root.x = 45.;
    session
        .preview_edit_pose(0, animate(0.407), root, &[Property::X], true)
        .unwrap();
    assert!(
        !session.document().animations[0]
            .tracks
            .iter()
            .any(|t| t.property == Property::Rotation)
    );
    assert_eq!(track(&session, Property::X).keys[0].value, 42.);
    assert_eq!(track(&session, Property::Y).keys[0].value, 19.);
    assert_eq!(session.local_pose(Some(0), 0.407)[0].rotation, 50.);
    assert_eq!(session.local_pose(Some(0), 0.407)[1].rotation, 77.);
    let keyed = session.document().clone();
    let remaining_pending = session.pose_edits().clone();
    session.commit_transaction();
    session.undo();
    assert_eq!(session.document(), &document);
    assert_eq!(session.pose_edits(), &pending);
    assert!(!session.is_dirty());
    session.redo();
    assert_eq!(session.document(), &keyed);
    assert_eq!(session.pose_edits(), &remaining_pending);
}

#[test]
fn setup_previews_replace_the_full_prior_preview_and_preserve_pending_animation_poses() {
    let mut session = pending_session();
    let before = session.document().clone();
    let pending = session.pose_edits().clone();
    session.begin_transaction();
    let mut root = before.bones[0].setup;
    root.x = 61.;
    session
        .preview_edit_pose(0, EditTarget::Setup, root, &[Property::X], false)
        .unwrap();
    let mut child = before.bones[1].setup;
    child.rotation = 123.;
    session
        .preview_edit_pose(1, EditTarget::Setup, child, &[Property::Rotation], true)
        .unwrap();
    assert_eq!(session.document().bones[0], before.bones[0]);
    assert_eq!(session.document().bones[1].setup, child);
    assert_eq!(session.document().animations, before.animations);
    assert_eq!(session.document().format, FORMAT_VERSION);
    assert_eq!(session.pose_edits(), &pending);
    assert!(session.is_dirty());
    session.commit_transaction();
    session.undo();
    assert_eq!(session.document(), &before);
    assert_eq!(session.pose_edits(), &pending);
    assert!(!session.is_dirty());
    session.redo();
    assert_eq!(session.document().bones[1].setup, child);
}

#[test]
fn missing_transaction_and_failed_previews_leave_last_valid_pose_saved_state_and_redo_intact() {
    let mut session = pending_session();
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo preserved".into(),
        })
        .unwrap();
    session.mark_saved();
    session.undo();
    let before = session.document().clone();
    let pending = session.pose_edits().clone();
    let root = session.local_pose(Some(0), 0.407)[0];
    assert!(
        session
            .preview_edit_pose(0, animate(0.407), root, &[Property::X], false)
            .unwrap_err()
            .contains("transaction")
    );
    assert_eq!(session.document(), &before);
    assert_eq!(session.pose_edits(), &pending);
    assert!(session.can_redo());
    assert!(session.is_dirty()); // Saved baseline is the renamed clip.
    session.begin_transaction();
    let mut changed = root;
    changed.rotation = 66.;
    session
        .preview_edit_pose(0, animate(0.407), changed, &[Property::Rotation], false)
        .unwrap();
    let preview = session.pose_edits().clone();
    let mut nonfinite = root;
    nonfinite.y = f32::NAN;
    for (bone, target, transform) in [
        (usize::MAX, animate(0.407), root),
        (
            0,
            EditTarget::Animate {
                animation: usize::MAX,
                time: 0.407,
            },
            root,
        ),
        (0, animate(f32::INFINITY), root),
        (0, animate(-0.1), root),
        (0, animate(0.407), nonfinite),
        (0, EditTarget::Setup, nonfinite),
    ] {
        assert!(
            session
                .preview_edit_pose(bone, target, transform, &[Property::Rotation], false)
                .is_err()
        );
        assert_eq!(session.document(), &before);
        assert_eq!(session.pose_edits(), &preview);
        assert!(session.is_dirty());
    }
    session.cancel_transaction();
    assert_eq!(session.document(), &before);
    assert_eq!(session.pose_edits(), &pending);
    assert!(session.can_redo());
    session.redo();
    assert_eq!(session.document().animations[0].name, "redo preserved");
    assert!(!session.is_dirty());
    assert_eq!(session.pose_edits(), &pending);
}

#[test]
fn late_auto_key_failure_keeps_the_previous_valid_preview_and_cancels_back_to_redo() {
    let mut document = fixture();
    document.bones[0].setup.scale_y = 0.;
    let mut session = Session::new(document.clone()).unwrap();
    session
        .dispatch(Command::SetBoneLength {
            bone: 1,
            length: 37.,
        })
        .unwrap();
    session.undo();
    session.begin_transaction();
    let mut root = session.local_pose(Some(0), 0.407)[0];
    root.rotation = 63.;
    session
        .preview_edit_pose(0, animate(0.407), root, &[Property::Rotation], true)
        .unwrap();
    let valid = session.document().clone();
    root.scale_x = 2.;
    assert!(
        session
            .preview_edit_pose(0, animate(0.407), root, &[Property::ScaleX], true)
            .unwrap_err()
            .contains("zero setup scale")
    );
    assert_eq!(session.document(), &valid);
    assert!(session.pose_edits().is_empty());
    session.cancel_transaction();
    assert_eq!(session.document(), &document);
    assert!(!session.can_undo());
    assert!(session.can_redo());
    assert!(!session.is_dirty());
    session.redo();
    assert_eq!(session.document().bones[1].length, 37.);
}

#[test]
fn returning_setup_or_unkeyed_preview_to_its_base_commits_no_history_and_preserves_redo() {
    for target in [EditTarget::Setup, animate(0.37134)] {
        let mut session = Session::new(fixture()).unwrap();
        session
            .dispatch(Command::SetBoneLength {
                bone: 1,
                length: 37.,
            })
            .unwrap();
        session.undo();
        let base = session.local_pose(
            match target {
                EditTarget::Setup => None,
                _ => Some(0),
            },
            0.37134,
        )[0];
        session.begin_transaction();
        let mut changed = base;
        changed.rotation += 18.;
        session
            .preview_edit_pose(0, target, changed, &[Property::Rotation], false)
            .unwrap();
        session
            .preview_edit_pose(0, target, base, &[Property::Rotation], false)
            .unwrap();
        session.commit_transaction();
        assert_eq!(session.document(), &fixture());
        assert!(session.pose_edits().is_empty());
        assert!(!session.can_undo());
        assert!(session.can_redo());
        assert!(!session.is_dirty());
        session.redo();
        assert_eq!(session.document().bones[1].length, 37.);
    }
}
