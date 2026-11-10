use crate::*;
use peachy_document::*;
use std::collections::BTreeMap;

fn frame(time: f32, mix: f32, bend_positive: bool, curve: IkCurve) -> IkKeyframe {
    IkKeyframe {
        time,
        mix,
        bend_positive,
        curve,
    }
}
fn track(keys: Vec<IkKeyframe>) -> IkTrack {
    IkTrack {
        constraint: 0,
        keys,
    }
}
fn setup() -> IkSetup {
    IkSetup {
        mix: 0.25,
        bend_positive: false,
    }
}

fn rig() -> Document {
    Document {
        format: FORMAT_VERSION,
        name: "Original animated IK".into(),
        fps: 30,
        bones: vec![
            Bone {
                name: "root".into(),
                parent: None,
                length: 0.,
                setup: Transform::default(),
                inherit: peachy_document::Inherit::Normal,
            },
            Bone {
                name: "arm".into(),
                parent: Some(0),
                length: 10.,
                setup: Transform::default(),
                inherit: peachy_document::Inherit::Normal,
            },
            Bone {
                name: "target".into(),
                parent: Some(0),
                length: 0.,
                setup: Transform {
                    y: 10.,
                    ..Transform::default()
                },
                inherit: peachy_document::Inherit::Normal,
            },
        ],
        constraints: vec![Constraint::Ik(IkConstraint {
            name: "aim".into(),
            bones: vec![1],
            target: 2,
            setup: IkSetup {
                mix: 0.,
                bend_positive: true,
            },
            stretch: false,
        })],
        slots: vec![],
        skins: BTreeMap::new(),
        animations: vec![Animation {
            name: "mix".into(),
            tracks: vec![],
            ik_tracks: vec![track(vec![
                frame(0., 0., true, IkCurve::Linear),
                frame(1., 1., false, IkCurve::Linear),
            ])],
            physics: vec![],
            physics_resets: vec![],
            inherit_tracks: vec![],
            deform_tracks: vec![],
            attachment_tracks: vec![],
            color_tracks: vec![],
            events: vec![],
            draw_order: vec![],
            path_tracks: vec![],
        }],
        skin_scope: Default::default(),
        events: vec![],
        reference_scale: 100.0,
    }
}

#[test]
fn grouped_absolute_settings_hold_setup_before_first_and_flip_bend_at_exact_key() {
    let mut keys = track(vec![
        frame(0.25, 1., true, IkCurve::Linear),
        frame(1., 0.5, false, IkCurve::Linear),
    ]);
    assert_eq!(
        sample_ik(&keys, setup(), f32::from_bits(0.25_f32.to_bits() - 1)).unwrap(),
        IkPose::from(setup())
    );
    assert_eq!(sample_ik(&keys, setup(), 0.25).unwrap().mix, 1.);
    assert_eq!(
        sample_ik(&keys, setup(), 0.5).unwrap().mix,
        0.8333333333333334
    );
    assert!(
        sample_ik(&keys, setup(), f32::from_bits(1_f32.to_bits() - 1))
            .unwrap()
            .bend_positive
    );
    assert!(!sample_ik(&keys, setup(), 1.).unwrap().bend_positive);
    assert_eq!(sample_ik(&keys, setup(), 2.).unwrap().mix, 0.5);
    keys.keys[0].curve = IkCurve::Stepped;
    assert_eq!(sample_ik(&keys, setup(), 0.9).unwrap().mix, 1.);
    assert_eq!(
        sample_ik(&track(vec![]), setup(), 0.5).unwrap(),
        setup().into()
    );
}

#[test]
fn absolute_mix_controls_preserve_equal_endpoint_excursions_and_unclamped_samples() {
    let mut keys = track(vec![
        frame(
            0.,
            0.5,
            true,
            IkCurve::Bezier {
                time: [0.25, 0.75],
                mix: [0.9, 0.9],
            },
        ),
        frame(1., 0.5, true, IkCurve::Linear),
    ]);
    assert_eq!(
        sample_ik(&keys, setup(), 0.5).unwrap().mix,
        0.7999999523162842
    );
    keys.keys[0].mix = 0.;
    keys.keys[1].mix = 1.;
    keys.keys[0].curve = IkCurve::Bezier {
        time: [0.25, 0.75],
        mix: [2., 2.],
    };
    assert_eq!(sample_ik(&keys, setup(), 0.5).unwrap().mix, 1.625);
    keys.keys[0].curve = IkCurve::Bezier {
        time: [0.25, 0.75],
        mix: [-1., -1.],
    };
    assert_eq!(sample_ik(&keys, setup(), 0.5).unwrap().mix, -0.625);
}

#[test]
fn independently_observed_large_cancellation_is_retained_instead_of_idealized_zero() {
    let m = 2e38_f32;
    let mut keys = track(vec![
        frame(
            0.,
            -m,
            true,
            IkCurve::Bezier {
                time: [0.25, 0.75],
                mix: [-m, m],
            },
        ),
        frame(1., m, true, IkCurve::Linear),
    ]);
    let before = keys.clone();
    assert_eq!(
        sample_ik(&keys, setup(), 0.5).unwrap().mix,
        -7.999997664749159e28
    );
    assert_eq!(keys, before);
    keys.keys[0].mix = 0.5;
    keys.keys[1].mix = 0.5;
    assert_eq!(
        sample_ik(&keys, setup(), 0.5).unwrap().mix,
        -1.1999996969360386e29
    );
    keys.keys[0].mix = -m;
    keys.keys[1].mix = m;
    keys.keys[0].curve = IkCurve::Linear;
    assert_eq!(sample_ik(&keys, setup(), 0.5).unwrap().mix, 0.);
}

#[test]
fn absolute_time_point_quantization_and_wide_ratio_serde_match_original_inputs() {
    let keys = track(vec![
        frame(
            100000.,
            0.5,
            true,
            IkCurve::Bezier {
                time: [0.25, 0.75],
                mix: [0.9, 0.9],
            },
        ),
        frame(100000.125, 0.5, true, IkCurve::Linear),
    ]);
    assert_eq!(
        sample_ik(&keys, setup(), (100000_f64 + 0.015625) as f32)
            .unwrap()
            .mix,
        0.6499999761581421
    );
    assert_eq!(
        sample_ik(&keys, setup(), (100000_f64 + 0.03125) as f32)
            .unwrap()
            .mix,
        0.7519999742507935
    );
    let curve = IkCurve::Bezier {
        time: [
            (f64::from(0.4_f32) - 0.25) / 0.75,
            (f64::from(0.775_f32) - 0.25) / 0.75,
        ],
        mix: [-1., 1.],
    };
    let bytes = serde_json::to_vec(&curve).unwrap();
    let recovered: IkCurve = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(recovered, curve);
    let keys = track(vec![
        frame(0.25, -1., true, recovered),
        frame(1., 1., true, IkCurve::Linear),
    ]);
    assert_eq!(
        sample_ik(&keys, setup(), 0.625).unwrap().mix,
        0.09779735142944698
    );
}

#[test]
fn complete_authored_capsule_carries_ik_only_animation_and_binding_solves_it_once() {
    let mut session = Session::new(rig()).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.5,
    };
    let zero = session
        .authored_pose(EditTarget::Animate {
            animation: 0,
            time: 0.,
        })
        .unwrap();
    let half = session.authored_pose(target).unwrap();
    assert_eq!(zero.bone_locals(), half.bone_locals());
    assert_eq!(half.ik_poses()[0].mix, 0.5);
    assert_eq!(session.document().animations[0].duration(), 1.);
    let solved = session.evaluated_pose(target).unwrap();
    assert_eq!(solved.authored_locals()[1].rotation, 0.);
    assert!((solved.applied_locals()[1].rotation - 45.).abs() < 0.00001);
    let bind = session.mesh_bind_pose(target).unwrap();
    for (snapshot, solved) in world_pose(session.document(), &bind.locals)
        .iter()
        .zip(solved.world())
    {
        assert_eq!(
            [
                snapshot.a, snapshot.b, snapshot.c, snapshot.d, snapshot.x, snapshot.y
            ],
            [solved.a, solved.b, solved.c, solved.d, solved.x, solved.y]
        );
    }
    session
        .edit_pose(
            2,
            target,
            Transform {
                x: 10.,
                y: 10.,
                ..Transform::default()
            },
            &[Property::X],
            false,
        )
        .unwrap();
    let pending = session.evaluated_pose(target).unwrap();
    assert!((pending.applied_locals()[1].rotation - 22.5).abs() < 0.00001);
    assert_eq!(pending.ik_poses()[0].mix, 0.5);
    assert!(!session.is_dirty());
}

#[test]
fn native7_validation_covers_group_identity_required_fields_and_finite_unbounded_mix() {
    let mut document = rig();
    let Constraint::Ik(ik) = &mut document.constraints[0] else {
        unreachable!("fixture constraint is IK")
    };
    ik.setup.mix = -1.5;
    document.animations[0].ik_tracks[0].keys[0].mix = 2.;
    document.validate().unwrap();
    let mut value = serde_json::to_value(&document).unwrap();
    value["animations"][0]
        .as_object_mut()
        .unwrap()
        .remove("ik_tracks");
    assert!(serde_json::from_value::<Document>(value).is_err());
    let bytes = serde_json::to_vec(&document).unwrap();
    assert_eq!(
        serde_json::from_slice::<Document>(&bytes).unwrap(),
        document
    );
    let duplicate = document.animations[0].ik_tracks[0].clone();
    document.animations[0].ik_tracks.push(duplicate);
    assert!(document.validate().is_err());
    for invalid in [
        r#"{"type":"linear","softness":0}"#,
        r#"{"type":"bezier","time":[0,1,1],"mix":[0,1]}"#,
        r#"{"type":"bezier","time":[0,1],"mix":[0,1],"unknown":0}"#,
    ] {
        assert!(serde_json::from_str::<IkCurve>(invalid).is_err());
    }
}

#[test]
fn typed_ik_clipboard_owns_absolute_settings_and_rejects_wrong_kind_and_collapsed_targets() {
    let mut document = rig();
    let Constraint::Ik(mut other) = document.constraints[0].clone() else {
        unreachable!("fixture constraint is IK")
    };
    other.name = "other".into();
    document.constraints.push(Constraint::Ik(other));
    let close = f32::from_bits(0.25_f32.to_bits() + 1);
    document.animations[0].ik_tracks[0].keys = vec![
        frame(
            0.25,
            -0.5,
            false,
            IkCurve::Bezier {
                time: [0.8, 0.2],
                mix: [2., -1.],
            },
        ),
        frame(close, 1.5, true, IkCurve::Stepped),
    ];
    let selection = [
        AnimationKey::Ik {
            constraint: 0,
            time: close,
        },
        AnimationKey::Ik {
            constraint: 0,
            time: 0.25,
        },
    ];
    let clipboard = copy_animation_keys(&document, 0, &selection).unwrap();
    let AnimationKeyClipboard::Ik { keys } = &clipboard else {
        panic!("IK clipboard kind");
    };
    assert_eq!(keys[0].mix, -0.5);
    assert_eq!(keys[0].time, 0.);
    assert_eq!(keys[1].time, close - 0.25);
    document.animations[0].ik_tracks[0].keys[0].mix = 0.75;
    let before = document.clone();
    let pasted = apply_command(
        &document,
        Command::PasteAnimationKeys {
            animation: 0,
            target: KeyOwner::Ik { constraint: 1 },
            time: 0.125,
            clipboard: clipboard.clone(),
        },
    )
    .unwrap();
    let added = pasted.animations[0]
        .ik_tracks
        .iter()
        .find(|t| t.constraint == 1)
        .unwrap();
    assert_eq!(
        added.keys[0],
        IkKeyframe {
            time: 0.125,
            ..keys[0]
        }
    );
    assert_eq!(added.keys[1].time, 0.125 + close - 0.25);
    assert_eq!(document, before);
    for (target, time) in [
        (KeyOwner::Bone { bone: 0 }, 0.),
        (KeyOwner::Ik { constraint: 1 }, 2.),
        (KeyOwner::Ik { constraint: 99 }, 0.),
    ] {
        assert!(
            apply_command(
                &document,
                Command::PasteAnimationKeys {
                    animation: 0,
                    target,
                    time,
                    clipboard: clipboard.clone()
                }
            )
            .is_err()
        );
        assert_eq!(document, before);
    }
    assert!(copy_animation_keys(&document, 0, &[selection[0], selection[0]]).is_err());
}

#[test]
fn grouped_exact_moves_restore_overwrites_duration_pending_pose_and_redo_from_fixed_base() {
    let mut document = rig();
    document.animations[0].ik_tracks[0].keys = vec![
        frame(0., 0.25, false, IkCurve::Stepped),
        frame(0.5, 0.5, true, IkCurve::Linear),
        frame(1., 1.5, false, IkCurve::Linear),
    ];
    let mut session = Session::new(document).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.5,
    };
    session
        .edit_pose(
            2,
            target,
            Transform {
                x: 2.,
                y: 10.,
                ..Transform::default()
            },
            &[Property::X],
            false,
        )
        .unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "renamed".into(),
        })
        .unwrap();
    session.undo();
    let base = session.document().clone();
    let pose = serde_json::to_value(session.pose_edits()).unwrap();
    let selected = vec![
        AnimationKey::Ik {
            constraint: 0,
            time: 0.,
        },
        AnimationKey::Ik {
            constraint: 0,
            time: 0.5,
        },
    ];
    session.begin_transaction();
    session
        .preview_transaction(Command::MoveAnimationKeys {
            animation: 0,
            keys: selected.clone(),
            delta: 0.5,
            copy: false,
        })
        .unwrap();
    assert_eq!(
        session.document().animations[0].ik_tracks[0].keys[1].mix,
        0.5
    );
    session
        .preview_transaction(Command::MoveAnimationKeys {
            animation: 0,
            keys: selected.clone(),
            delta: 0.25,
            copy: false,
        })
        .unwrap();
    assert_eq!(
        session.document().animations[0].ik_tracks[0]
            .keys
            .last()
            .unwrap()
            .mix,
        1.5
    );
    session.cancel_transaction();
    assert_eq!(session.document(), &base);
    assert!(session.can_redo());
    session.begin_transaction();
    session
        .preview_transaction(Command::MoveAnimationKeys {
            animation: 0,
            keys: selected,
            delta: 0.5,
            copy: false,
        })
        .unwrap();
    session.commit_transaction();
    assert!(!session.can_redo());
    session.undo();
    assert_eq!(session.document(), &base);
    assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pose);
    session
        .dispatch(Command::DeleteAnimationKeys {
            animation: 0,
            keys: vec![AnimationKey::Ik {
                constraint: 0,
                time: 1.,
            }],
        })
        .unwrap();
    assert_eq!(session.document().animations[0].duration(), 0.5);
    session.undo();
    assert_eq!(session.document(), &base);
}

#[test]
fn typed_key_commands_use_exact_identities_and_strict_payloads_without_aliasing_bones() {
    let mut document = rig();
    document.animations[0].tracks.push(Track {
        bone: 0,
        property: Property::X,
        keys: vec![Keyframe {
            time: 0.,
            value: 0.,
            curve: Curve::Linear,
        }],
    });
    let before = document.clone();
    let command = Command::DeleteAnimationKeys {
        animation: 0,
        keys: vec![AnimationKey::Ik {
            constraint: 0,
            time: 0.,
        }],
    };
    let bytes = serde_json::to_vec(&command).unwrap();
    let deleted = apply_command(&document, serde_json::from_slice(&bytes).unwrap()).unwrap();
    assert_eq!(deleted.animations[0].tracks, before.animations[0].tracks);
    assert_eq!(deleted.animations[0].ik_tracks[0].keys.len(), 1);
    for value in [
        serde_json::json!({"type":"ik","constraint":0,"time":0,"bone":0}),
        serde_json::json!({"type":"bone","bone":0,"property":"mix","time":0}),
    ] {
        assert!(serde_json::from_value::<AnimationKey>(value).is_err());
    }
    let invalid = Command::MoveAnimationKeys {
        animation: 0,
        keys: vec![AnimationKey::Ik {
            constraint: 0,
            time: 0.1,
        }],
        delta: 1.,
        copy: false,
    };
    assert!(apply_command(&document, invalid).is_err());
    assert_eq!(document, before);
}

#[test]
fn explicit_set_ik_key_preserves_every_close_identity_including_ambiguous_neighborhoods() {
    let mut document = rig();
    document.animations[0].ik_tracks.clear();
    let first = 0.25_f32;
    let close = f32::from_bits(first.to_bits() + 1);
    let third = f32::from_bits(first.to_bits() + 2);
    for (time, mix) in [(first, -0.5), (close, 0.5), (third, 1.5)] {
        document = apply_command(
            &document,
            Command::SetIkKey {
                animation: 0,
                constraint: 0,
                key: frame(time, mix, true, IkCurve::Linear),
            },
        )
        .unwrap();
    }
    let keys = &document.animations[0].ik_tracks[0].keys;
    assert_eq!(
        keys.iter().map(|k| k.time.to_bits()).collect::<Vec<_>>(),
        vec![first.to_bits(), close.to_bits(), third.to_bits()]
    );
    assert_eq!(
        keys.iter().map(|k| k.mix).collect::<Vec<_>>(),
        vec![-0.5, 0.5, 1.5]
    );
    let query = first - 0.000001;
    assert!(ik_keyframe_at(keys, query).is_err());
    document = apply_command(
        &document,
        Command::SetIkKey {
            animation: 0,
            constraint: 0,
            key: frame(close, 2., false, IkCurve::Stepped),
        },
    )
    .unwrap();
    let keys = &document.animations[0].ik_tracks[0].keys;
    assert_eq!(keys.len(), 3);
    assert_eq!(keys[0].mix, -0.5);
    assert_eq!(keys[2].mix, 1.5);
    assert_eq!(keys[1].mix, 2.);
}

#[test]
fn setup_command_target_rejects_unknown_fields_instead_of_silently_discarding_them() {
    for invalid in [
        serde_json::json!({"mode":"setup","unsupported":true}),
        serde_json::json!({"mode":"setup","animation":0,"time":1}),
    ] {
        assert!(serde_json::from_value::<EditTarget>(invalid.clone()).is_err());
        let mut command = serde_json::to_value(Command::SetPose {
            bone: 1,
            target: EditTarget::Setup,
            transform: Transform::default(),
            properties: vec![Property::X],
        })
        .unwrap();
        command["target"] = invalid;
        assert!(serde_json::from_value::<Command>(command).is_err());
    }
    let target: EditTarget = serde_json::from_str(r#"{"mode":"setup"}"#).unwrap();
    assert_eq!(
        serde_json::to_value(target).unwrap(),
        serde_json::json!({"mode":"setup"})
    );
}

fn pending_value(session: &Session, time: f32) -> IkPose {
    session
        .authored_pose(EditTarget::Animate { animation: 0, time })
        .unwrap()
        .ik_poses()[0]
}
fn pose_bytes(session: &Session) -> Vec<u8> {
    serde_json::to_vec(session.pose_edits()).unwrap()
}

#[test]
fn ik_pending_is_per_property_and_fixed_base_preview_restores_bone_pose_history_and_redo() {
    let mut session = Session::new(rig()).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.5,
    };
    session
        .edit_pose(
            2,
            target,
            Transform {
                x: 2.,
                y: 10.,
                ..Transform::default()
            },
            &[Property::X],
            false,
        )
        .unwrap();
    session
        .edit_ik_pose(
            0,
            target,
            &[IkPoseEdit::BendPositive { value: false }],
            false,
        )
        .unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "temporary".into(),
        })
        .unwrap();
    session.undo();
    let document = session.document().clone();
    let pending = pose_bytes(&session);
    session.begin_transaction();
    session
        .preview_edit_ik_pose(0, target, &[IkPoseEdit::Mix { value: 0.3 }], false)
        .unwrap();
    assert_eq!(
        pending_value(&session, 0.5),
        IkPose {
            mix: f64::from(0.3_f32),
            bend_positive: false
        }
    );
    assert_eq!(
        session.authored_pose(target).unwrap().bone_locals()[2].x,
        2.
    );
    let last_valid = pose_bytes(&session);
    assert!(
        session
            .preview_edit_ik_pose(0, target, &[IkPoseEdit::Mix { value: f32::NAN }], false)
            .is_err()
    );
    assert_eq!(pose_bytes(&session), last_valid);
    assert_eq!(session.document(), &document);
    session
        .preview_edit_ik_pose(
            0,
            target,
            &[IkPoseEdit::BendPositive { value: true }],
            false,
        )
        .unwrap();
    assert_eq!(pending_value(&session, 0.5).mix, 0.5);
    session.cancel_transaction();
    assert_eq!(pose_bytes(&session), pending);
    assert!(session.can_redo());
    assert!(!session.is_dirty());
    session.begin_transaction();
    session
        .preview_edit_ik_pose(0, target, &[IkPoseEdit::Mix { value: 0.3 }], false)
        .unwrap();
    session.commit_transaction();
    let edited = pose_bytes(&session);
    session.undo();
    assert_eq!(pose_bytes(&session), pending);
    session.redo();
    assert_eq!(pose_bytes(&session), edited);
    // Editing only Mix does not freeze the unedited sampled bend at another time.
    session
        .edit_ik_pose(
            0,
            target,
            &[IkPoseEdit::BendPositive { value: true }],
            false,
        )
        .unwrap();
    assert!(!pending_value(&session, 1.).bend_positive);
    let solved = session.evaluated_pose(target).unwrap();
    let bind = session.mesh_bind_pose(target).unwrap();
    for (captured, world) in world_pose(session.document(), &bind.locals)
        .iter()
        .zip(solved.world())
    {
        assert_eq!(
            [
                captured.a, captured.b, captured.c, captured.d, captured.x, captured.y
            ],
            [world.a, world.b, world.c, world.d, world.x, world.y]
        );
    }
}

#[test]
fn autokey_snapshots_both_ik_fields_and_live_preview_keying_is_one_undo() {
    let mut document = rig();
    document.animations[0].ik_tracks[0].keys.insert(
        1,
        frame(
            0.5,
            0.5,
            true,
            IkCurve::Bezier {
                time: [0.8, 0.2],
                mix: [0.9, 0.7],
            },
        ),
    );
    let curve = document.animations[0].ik_tracks[0].keys[1].curve;
    let mut session = Session::new(document).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.5,
    };
    session
        .edit_pose(
            2,
            target,
            Transform {
                x: 2.,
                y: 10.,
                ..Transform::default()
            },
            &[Property::X],
            false,
        )
        .unwrap();
    session
        .edit_ik_pose(
            0,
            target,
            &[
                IkPoseEdit::Mix { value: 0.3 },
                IkPoseEdit::BendPositive { value: false },
            ],
            false,
        )
        .unwrap();
    let base = session.document().clone();
    let pending = pose_bytes(&session);
    session.begin_transaction();
    session
        .preview_edit_ik_pose(0, target, &[IkPoseEdit::Mix { value: 0.7 }], true)
        .unwrap();
    assert!(!session.pose_edits().has_ik(0, 0));
    assert!(session.pose_edits().has_bone(0, 2));
    let key = session.document().animations[0].ik_tracks[0].keys[1];
    assert_eq!(key.mix, 0.7);
    assert!(!key.bend_positive);
    assert_eq!(key.curve, curve);
    session.cancel_transaction();
    assert_eq!(session.document(), &base);
    assert_eq!(pose_bytes(&session), pending);
    session.begin_transaction();
    session
        .preview_edit_ik_pose(0, target, &[IkPoseEdit::Mix { value: 0.7 }], false)
        .unwrap();
    session.key_ik_pose(0, 0.5, 0).unwrap();
    session.commit_transaction();
    let keyed = session.document().clone();
    assert_eq!(keyed.animations[0].ik_tracks[0].keys[1].mix, 0.7);
    assert_eq!(
        session
            .pose_edits()
            .ik_key_state(session.document(), 0, 0.5, 0)
            .unwrap(),
        KeyState::Keyed
    );
    session.undo();
    assert_eq!(session.document(), &base);
    assert_eq!(pose_bytes(&session), pending);
    session.redo();
    assert_eq!(session.document(), &keyed);
}

#[test]
fn finite_sample_key_conversion_rejects_a_visible_tip_jump_and_global_key_failure_is_atomic() {
    let mut document = rig();
    document.bones[1].length = 10000.;
    document.bones[2].setup.x = 10.;
    document.bones.push(Bone {
        name: "unrelated".into(),
        parent: Some(0),
        length: 1.,
        setup: Transform::default(),
        inherit: peachy_document::Inherit::Normal,
    });
    let next = f32::from_bits(2_f32.to_bits() + 1);
    document.animations[0].ik_tracks[0].keys = vec![
        frame(0., 2., true, IkCurve::Linear),
        frame(1., next, true, IkCurve::Linear),
    ];
    let mut session = Session::new(document).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.5,
    };
    session
        .edit_pose(
            3,
            target,
            Transform {
                x: 5.,
                ..Transform::default()
            },
            &[Property::X],
            false,
        )
        .unwrap();
    session
        .edit_ik_pose(
            0,
            target,
            &[IkPoseEdit::BendPositive { value: false }],
            false,
        )
        .unwrap();
    let before = session.evaluated_pose(target).unwrap();
    assert_eq!(pending_value(&session, 0.5).mix, 2.0000001192092896);
    // Both rounded Mix endpoints can yield individually valid frames; the
    // conversion itself still changes the long bone's visible endpoint.
    let mut rounded = session.authored_pose(target).unwrap();
    rounded.ik[0].mix = 2.;
    let after = checked_pose_from_authored(session.document(), &rounded).unwrap();
    let old_tip = before.world()[1].point(10000., 0.);
    let new_tip = after.world()[1].point(10000., 0.);
    assert!((old_tip[0] - new_tip[0]).abs() > 0.001);
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo".into(),
        })
        .unwrap();
    session.undo();
    let document = session.document().clone();
    let pending = pose_bytes(&session);
    assert!(
        session
            .key_edited(0, 0.5, None)
            .unwrap_err()
            .contains("precision")
    );
    assert_eq!(session.document(), &document);
    assert_eq!(pose_bytes(&session), pending);
    assert!(session.can_redo());
    assert!(!session.is_dirty());
    assert!(session.key_ik_pose(0, 0.5, 0).is_err());
    assert_eq!(session.document(), &document);
    assert_eq!(pose_bytes(&session), pending);
    assert!(session.can_redo());
}

#[test]
fn fractional_key_that_changes_the_actual_cursor_pose_fails_without_losing_a_valid_preview() {
    let mut session = Session::new(rig()).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.257,
    };
    session
        .edit_ik_pose(
            0,
            target,
            &[IkPoseEdit::BendPositive { value: false }],
            false,
        )
        .unwrap();
    let base = session.document().clone();
    let pending = pose_bytes(&session);
    session.begin_transaction();
    session
        .preview_edit_ik_pose(0, target, &[IkPoseEdit::Mix { value: 0.4 }], false)
        .unwrap();
    let last = pose_bytes(&session);
    assert!(session.key_ik_pose(0, 0.257, 0).is_err());
    assert_eq!(pose_bytes(&session), last);
    assert_eq!(session.document(), &base);
    session.cancel_transaction();
    assert_eq!(pose_bytes(&session), pending);
}

#[test]
fn global_key_success_clears_both_pending_kinds_atomically_while_selected_bone_key_stays_local() {
    let mut session = Session::new(rig()).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.5,
    };
    session
        .edit_pose(
            2,
            target,
            Transform {
                x: 2.,
                y: 10.,
                ..Transform::default()
            },
            &[Property::X],
            false,
        )
        .unwrap();
    session
        .edit_ik_pose(0, target, &[IkPoseEdit::Mix { value: 0.7 }], false)
        .unwrap();
    let base = session.document().clone();
    let pending = pose_bytes(&session);
    session.key_edited(0, 0.5, Some(2)).unwrap();
    assert!(session.pose_edits().has_ik(0, 0));
    assert!(!session.pose_edits().has_bone(0, 2));
    session.undo();
    assert_eq!(session.document(), &base);
    assert_eq!(pose_bytes(&session), pending);
    session.key_edited(0, 0.5, None).unwrap();
    assert!(session.pose_edits().is_empty());
    assert!(!session.document().animations[0].tracks.is_empty());
    assert_eq!(
        session.document().animations[0].ik_tracks[0].keys[1].mix,
        0.7
    );
    session.undo();
    assert_eq!(session.document(), &base);
    assert_eq!(pose_bytes(&session), pending);
}

#[test]
fn invalid_ik_edits_noops_discard_and_setup_changes_preserve_pending_and_redo_contracts() {
    let mut session = Session::new(rig()).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.5,
    };
    session
        .edit_ik_pose(0, target, &[IkPoseEdit::Mix { value: 0.4 }], false)
        .unwrap();
    session
        .edit_pose(
            2,
            target,
            Transform {
                x: 2.,
                y: 10.,
                ..Transform::default()
            },
            &[Property::X],
            false,
        )
        .unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo".into(),
        })
        .unwrap();
    session.undo();
    let base = session.document().clone();
    let pending = pose_bytes(&session);
    for edits in [
        vec![IkPoseEdit::Mix {
            value: f32::INFINITY,
        }],
        vec![
            IkPoseEdit::Mix { value: 0.3 },
            IkPoseEdit::Mix { value: 0.4 },
        ],
    ] {
        assert!(session.edit_ik_pose(0, target, &edits, false).is_err());
        assert_eq!(session.document(), &base);
        assert_eq!(pose_bytes(&session), pending);
        assert!(session.can_redo());
    }
    assert!(session.preview_edit_ik_pose(0, target, &[], false).is_err());
    session.edit_ik_pose(0, target, &[], true).unwrap();
    assert_eq!(pose_bytes(&session), pending);
    assert!(session.can_redo());
    session
        .edit_ik_pose(
            0,
            EditTarget::Setup,
            &[IkPoseEdit::Mix { value: -0.5 }],
            false,
        )
        .unwrap();
    assert_eq!(pose_bytes(&session), pending);
    session.undo();
    assert_eq!(session.document(), &base);
    assert_eq!(pose_bytes(&session), pending);
    session.discard_pose(false);
    assert!(session.pose_edits().is_empty());
    session.undo();
    assert!(session.pose_edits().is_empty());
}

#[test]
fn key_navigation_and_duration_union_bone_and_ik_keys_without_skipping_close_identities() {
    let mut document = rig();
    let close = f32::from_bits(0.25_f32.to_bits() + 1);
    document.animations[0].ik_tracks[0].keys = vec![
        frame(0.25, 0.5, true, IkCurve::Linear),
        frame(close, 0.7, false, IkCurve::Linear),
    ];
    document.animations[0].tracks.push(Track {
        bone: 2,
        property: Property::X,
        keys: vec![Keyframe {
            time: 0.5,
            value: 1.,
            curve: Curve::Linear,
        }],
    });
    assert_eq!(
        adjacent_key_time(&document, 0, 0., PlayDirection::Forward),
        Some(0.25)
    );
    assert_eq!(
        adjacent_key_time(&document, 0, 0.25, PlayDirection::Forward),
        Some(close)
    );
    assert_eq!(
        adjacent_key_time(&document, 0, 0.5, PlayDirection::Backward),
        Some(close)
    );
    assert_eq!(last_key_time(&document, 0), 0.5);
    document.animations[0].tracks.clear();
    assert_eq!(last_key_time(&document, 0), close);
    document.animations[0].ik_tracks[0].keys = vec![frame(0., -0.5, false, IkCurve::Linear)];
    assert_eq!(last_key_time(&document, 0), 0.);
    assert!(
        !advance_playback(
            0.,
            1.,
            last_key_time(&document, 0),
            true,
            PlayDirection::Forward
        )
        .playing
    );
    document.animations[0].ik_tracks[0].keys.clear();
    assert_eq!(last_key_time(&document, 0), 0.);
}

#[test]
fn selected_constraint_key_keeps_other_ik_pending_and_global_key_commits_all_as_one_state() {
    let mut document = rig();
    document.bones.push(Bone {
        name: "second".into(),
        parent: Some(0),
        length: 5.,
        setup: Transform::default(),
        inherit: peachy_document::Inherit::Normal,
    });
    document.constraints.push(Constraint::Ik(IkConstraint {
        name: "other".into(),
        bones: vec![3],
        target: 2,
        setup: IkSetup {
            mix: 0.25,
            bend_positive: false,
        },
        stretch: false,
    }));
    let mut session = Session::new(document).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.5,
    };
    session
        .edit_ik_pose(0, target, &[IkPoseEdit::Mix { value: 0.7 }], false)
        .unwrap();
    session
        .edit_ik_pose(
            1,
            target,
            &[
                IkPoseEdit::Mix { value: -0.5 },
                IkPoseEdit::BendPositive { value: true },
            ],
            false,
        )
        .unwrap();
    session
        .edit_pose(
            2,
            target,
            Transform {
                x: 2.,
                y: 10.,
                ..Transform::default()
            },
            &[Property::X],
            false,
        )
        .unwrap();
    let base = session.document().clone();
    let pending = pose_bytes(&session);
    session.key_ik_pose(0, 0.5, 0).unwrap();
    assert!(!session.pose_edits().has_ik(0, 0));
    assert!(session.pose_edits().has_ik(0, 1));
    assert!(session.pose_edits().has_bone(0, 2));
    let current = session.authored_pose(target).unwrap();
    assert_eq!(
        current.ik_poses()[1],
        IkPose {
            mix: -0.5,
            bend_positive: true
        }
    );
    session.undo();
    assert_eq!(session.document(), &base);
    assert_eq!(pose_bytes(&session), pending);
    session.key_edited(0, 0.5, None).unwrap();
    assert!(session.pose_edits().is_empty());
    assert_eq!(session.document().animations[0].ik_tracks.len(), 2);
    session.undo();
    assert_eq!(session.document(), &base);
    assert_eq!(pose_bytes(&session), pending);
}

#[test]
fn negative_mix_actively_solves_and_zero_pending_can_correct_a_query_invalid_native_pose() {
    let mut document = rig();
    let Constraint::Ik(ik) = &mut document.constraints[0] else {
        unreachable!("fixture constraint is IK")
    };
    ik.setup.mix = -0.5;
    document.validate().unwrap();
    let pose = checked_evaluate_pose(&document, None, 0.).unwrap();
    assert!((pose.applied_locals()[1].rotation + 45.).abs() < 0.00001);
    document.bones[1].setup.scale_x = 2.;
    document.animations[0].ik_tracks[0].keys = vec![
        frame(0., 0., true, IkCurve::Linear),
        frame(1., -0.5, false, IkCurve::Linear),
    ];
    let mut session = Session::new(document).unwrap();
    assert!(session.evaluated_pose(EditTarget::Setup).is_err());
    assert!(
        session
            .evaluated_pose(EditTarget::Animate {
                animation: 0,
                time: 0.
            })
            .is_ok()
    );
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.5,
    };
    assert!(session.evaluated_pose(target).is_err());
    let document = session.document().clone();
    session
        .edit_ik_pose(0, target, &[IkPoseEdit::Mix { value: 0. }], false)
        .unwrap();
    let corrected = session.evaluated_pose(target).unwrap();
    assert_eq!(corrected.applied_locals()[1].scale_x, 2.);
    assert_eq!(session.document(), &document);
    assert!(session.pose_edits().has_ik(0, 0));
    assert!(!session.is_dirty());
}

#[test]
fn malformed_typed_clipboards_and_group_commands_fail_atomically_with_pending_and_redo() {
    let mut session = Session::new(rig()).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.5,
    };
    session
        .edit_ik_pose(0, target, &[IkPoseEdit::Mix { value: 0.7 }], false)
        .unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo".into(),
        })
        .unwrap();
    session.undo();
    let document = session.document().clone();
    let pending = pose_bytes(&session);
    let a = frame(0., 0.5, true, IkCurve::Linear);
    for keys in [
        vec![],
        vec![frame(1., 0.5, true, IkCurve::Linear)],
        vec![a, a],
        vec![frame(0., f32::INFINITY, true, IkCurve::Linear)],
        vec![frame(
            0.,
            0.5,
            true,
            IkCurve::Bezier {
                time: [f64::NAN, 0.5],
                mix: [0., 1.],
            },
        )],
    ] {
        let command = Command::PasteAnimationKeys {
            animation: 0,
            target: KeyOwner::Ik { constraint: 0 },
            time: 0.5,
            clipboard: AnimationKeyClipboard::Ik { keys },
        };
        assert!(session.dispatch(command).is_err());
        assert_eq!(session.document(), &document);
        assert_eq!(pose_bytes(&session), pending);
        assert!(session.can_redo());
    }
    for command in [
        Command::SetIkKey {
            animation: 0,
            constraint: 999,
            key: a,
        },
        Command::DeleteIkKey {
            animation: 0,
            constraint: 0,
            time: f32::from_bits(1_f32.to_bits() + 1),
        },
        Command::Batch {
            commands: vec![
                Command::SetIkKey {
                    animation: 0,
                    constraint: 0,
                    key: a,
                },
                Command::SetIkKey {
                    animation: 0,
                    constraint: 999,
                    key: a,
                },
            ],
        },
    ] {
        assert!(session.dispatch(command).is_err());
        assert_eq!(session.document(), &document);
        assert_eq!(pose_bytes(&session), pending);
        assert!(session.can_redo());
    }
    for payload in [
        serde_json::json!({"type":"mix","value":0.5,"extra":true}),
        serde_json::json!({"type":"bendPositive","value":1}),
    ] {
        assert!(serde_json::from_value::<IkPoseEdit>(payload).is_err());
    }
}

#[test]
fn pending_and_auto_key_edit_actual_fractional_source_before_considering_the_frame_grid() {
    let first = 0.25_f32;
    let close = f32::from_bits(first.to_bits() + 1);
    let curve = IkCurve::Bezier {
        time: [0.8, 0.2],
        mix: [0.7, 0.3],
    };
    let mut document = rig();
    document.animations[0].ik_tracks[0].keys = vec![
        frame(first, 0.2, true, curve),
        frame(close, 0.5, false, IkCurve::Stepped),
        frame(1., 1., true, IkCurve::Linear),
    ];
    let mut session = Session::new(document).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: first,
    };
    session
        .edit_ik_pose(0, target, &[IkPoseEdit::Mix { value: 0.4 }], false)
        .unwrap();
    let base = session.document().clone();
    let pending = pose_bytes(&session);
    session.key_ik_pose(0, first, 0).unwrap();
    let keys = &session.document().animations[0].ik_tracks[0].keys;
    assert_eq!(keys.len(), 3);
    assert_eq!(keys[0].time, first);
    assert_eq!(keys[0].mix, 0.4);
    assert_eq!(keys[0].curve, curve);
    assert_eq!(keys[1].time, close);
    session.undo();
    assert_eq!(session.document(), &base);
    assert_eq!(pose_bytes(&session), pending);
    let target = EditTarget::Animate {
        animation: 0,
        time: close,
    };
    session.begin_transaction();
    session
        .preview_edit_ik_pose(0, target, &[IkPoseEdit::Mix { value: 0.6 }], true)
        .unwrap();
    let keys = &session.document().animations[0].ik_tracks[0].keys;
    assert_eq!(keys.len(), 3);
    assert_eq!(keys[0], base.animations[0].ik_tracks[0].keys[0]);
    assert_eq!(keys[1].time, close);
    assert_eq!(keys[1].mix, 0.6);
    assert_eq!(keys[1].curve, IkCurve::Stepped);
    assert!(!keys[1].bend_positive);
    session.cancel_transaction();
    assert_eq!(session.document(), &base);
    assert_eq!(pose_bytes(&session), pending);
}
