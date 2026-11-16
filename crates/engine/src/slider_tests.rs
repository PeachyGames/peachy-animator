//! Raw-data/API scaffold gates only; no measured Slider evaluation is claimed.
use crate::*;
use peachy_document::*;

fn document() -> Document {
    let mut doc = Document::new("Slider raw lab".into(), 60).unwrap();
    doc.animations.push(Animation::new("reference", vec![]));
    doc.animations.push(Animation::new("host", vec![]));
    doc.constraints
        .push(Constraint::Slider(SliderConstraint::new("slider", 0)));
    doc
}

fn key(time: f32, value: f32) -> Keyframe {
    Keyframe {
        time,
        value,
        curve: Curve::Linear,
    }
}

fn set(property: SliderProperty, key: Keyframe) -> Command {
    Command::SetSliderKey {
        animation: 1,
        constraint: 0,
        property,
        key,
    }
}

#[test]
fn slider_driven_setup_time_is_inactive_data_but_explicit_pending_time_remains_a_pose_override() {
    // Original skin/source controls store JSON time:.5 while the public
    // consumer's before-world Time is0 for a source bone, and .5 manually.
    let mut doc = document();
    let Constraint::Slider(slider) = &mut doc.constraints[0] else {
        unreachable!()
    };
    slider.setup = SliderSetup { time: 0.5, mix: 0. };
    let manual = sample_authored_pose(&doc, Some(1), 0.).unwrap();
    assert_eq!(manual.slider_poses()[0].time, 0.5);
    let Constraint::Slider(slider) = &mut doc.constraints[0] else {
        unreachable!()
    };
    slider.bone = Some(0);
    slider.property = Some(TransformProperty::X);
    slider.local = true;
    let original = doc.clone();
    for inactive_constraint in [false, true] {
        if inactive_constraint {
            doc.skin_scope.required_constraints.insert(0);
        }
        let authored = sample_authored_pose(&doc, Some(1), 0.).unwrap();
        assert_eq!(authored.slider_poses()[0].time, 0.);
        assert_eq!(
            checked_pose_from_authored(&doc, &authored)
                .unwrap()
                .applied_slider_poses()[0]
                .time,
            0.
        );
        assert_eq!(doc.constraints[0].as_slider().unwrap().setup.time, 0.5);
    }
    let mut session = Session::new(original.clone()).unwrap();
    let target = EditTarget::Animate {
        animation: 1,
        time: 0.,
    };
    // Independent source-Mix holdouts explicitly set pose Time=.375. At Mix0
    // that override survives; it is not the ignored raw setup-Time field.
    session
        .edit_slider_pose(0, target, &[SliderPoseEdit::Time { value: 0.375 }], false)
        .unwrap();
    assert_eq!(
        session.authored_pose(target).unwrap().slider_poses()[0].time,
        0.375
    );
    assert_eq!(
        session
            .evaluated_pose(target)
            .unwrap()
            .applied_slider_poses()[0]
            .time,
        0.375
    );
    assert_eq!(session.document(), &original);
    assert!(!session.is_dirty());
    session.undo();
    assert_eq!(session.slider_pose(0, target).unwrap().time, 0.);
    assert_eq!(session.document(), &original);
}

#[test]
fn slider_inactive_host_time_keys_skip_while_named_host_and_explicit_pending_values_remain_distinct()
 {
    let mut doc = document();
    let Constraint::Slider(slider) = &mut doc.constraints[0] else {
        unreachable!()
    };
    slider.bone = Some(0);
    slider.property = Some(TransformProperty::X);
    slider.local = true;
    slider.setup = SliderSetup { time: 0.5, mix: 0. };
    doc.animations[1].slider_tracks.push(SliderTrack {
        constraint: 0,
        property: SliderProperty::Time,
        keys: vec![key(0., 0.75), key(0.5, 1.25)],
    });
    doc.skins.insert("awake".into(), Default::default());
    doc.skin_scope.required_constraints.insert(0);
    doc.skin_scope.skins.insert(
        "awake".into(),
        SkinMembers {
            bones: vec![],
            constraints: vec![0],
        },
    );
    doc.validate().unwrap();
    for (time, expected) in [(0., 0.75), (0.5, 1.25)] {
        let inactive = sample_authored_pose_with_skin(&doc, Some(1), time, "default").unwrap();
        let active = sample_authored_pose_with_skin(&doc, Some(1), time, "awake").unwrap();
        assert_eq!(inactive.slider_poses()[0].time, 0.);
        assert_eq!(active.slider_poses()[0].time, expected);
    }
    let mut session = Session::new(doc.clone()).unwrap();
    session.set_skin("awake");
    let target = EditTarget::Animate {
        animation: 1,
        time: 0.,
    };
    session
        .edit_slider_pose(0, target, &[SliderPoseEdit::Time { value: 0.375 }], false)
        .unwrap();
    session.set_skin("default");
    assert_eq!(
        session.authored_pose(target).unwrap().slider_poses()[0].time,
        0.375
    );
    assert_eq!(
        session
            .evaluated_pose(target)
            .unwrap()
            .applied_slider_poses()[0]
            .time,
        0.375
    );
    assert!(session.pose_edits().has_slider(1, 0, SliderProperty::Time));
    session.set_skin("awake");
    session
        .edit_slider_pose(0, target, &[SliderPoseEdit::Time { value: 0.75 }], false)
        .unwrap();
    assert!(!session.pose_edits().has_slider(1, 0, SliderProperty::Time));
    assert_eq!(session.slider_pose(0, target).unwrap().time, 0.75);
    assert_eq!(session.document(), &doc);

    // Source eligibility does not gate host keys when the Slider itself is
    // active: its later source solve skips independently.
    doc.skin_scope.required_constraints.clear();
    doc.skin_scope.required_bones.insert(0);
    doc.validate().unwrap();
    let authored = sample_authored_pose(&doc, Some(1), 0.).unwrap();
    assert!(!authored.activity().bones[0] && authored.activity().constraints[0]);
    assert_eq!(authored.slider_poses()[0].time, 0.75);
    assert_eq!(
        checked_pose_from_authored(&doc, &authored)
            .unwrap()
            .applied_slider_poses()[0]
            .time,
        0.75
    );
}

#[test]
fn slider_inactive_host_mix_keeps_setup_while_explicit_pending_and_active_host_keys_apply() {
    let mut doc = document();
    let Constraint::Slider(slider) = &mut doc.constraints[0] else {
        unreachable!()
    };
    slider.bone = Some(0);
    slider.property = Some(TransformProperty::X);
    slider.local = true;
    slider.setup = SliderSetup {
        time: 0.5,
        mix: 0.5,
    };
    doc.animations[1].slider_tracks.push(SliderTrack {
        constraint: 0,
        property: SliderProperty::Mix,
        keys: vec![key(0., 0.25), key(0.5, 0.75)],
    });
    doc.skins.insert("awake".into(), Default::default());
    doc.skin_scope.required_constraints.insert(0);
    doc.skin_scope.skins.insert(
        "awake".into(),
        SkinMembers {
            bones: vec![],
            constraints: vec![0],
        },
    );
    doc.validate().unwrap();
    for (time, expected) in [(0., 0.25), (0.5, 0.75)] {
        let inactive = sample_authored_pose_with_skin(&doc, Some(1), time, "default").unwrap();
        let active = sample_authored_pose_with_skin(&doc, Some(1), time, "awake").unwrap();
        assert_eq!(inactive.slider_poses()[0].mix, 0.5);
        assert_eq!(active.slider_poses()[0].mix, expected);
    }
    let mut session = Session::new(doc.clone()).unwrap();
    let target = EditTarget::Animate {
        animation: 1,
        time: 0.,
    };
    session
        .edit_slider_pose(0, target, &[SliderPoseEdit::Mix { value: 0.625 }], false)
        .unwrap();
    assert_eq!(
        session.authored_pose(target).unwrap().slider_poses()[0].mix,
        0.625
    );
    assert_eq!(
        session
            .evaluated_pose(target)
            .unwrap()
            .applied_slider_poses()[0]
            .mix,
        0.625
    );
    session.set_skin("awake");
    assert_eq!(session.slider_pose(0, target).unwrap().mix, 0.625);
    session
        .edit_slider_pose(0, target, &[SliderPoseEdit::Mix { value: 0.25 }], false)
        .unwrap();
    assert!(!session.pose_edits().has_slider(1, 0, SliderProperty::Mix));
    assert_eq!(session.slider_pose(0, target).unwrap().mix, 0.25);
    assert_eq!(session.document(), &doc);

    doc.skin_scope.required_constraints.clear();
    doc.skin_scope.required_bones.insert(0);
    doc.validate().unwrap();
    let authored = sample_authored_pose(&doc, Some(1), 0.).unwrap();
    assert!(!authored.activity().bones[0] && authored.activity().constraints[0]);
    assert_eq!(authored.slider_poses()[0].mix, 0.25);
    assert_eq!(
        checked_pose_from_authored(&doc, &authored)
            .unwrap()
            .applied_slider_poses()[0]
            .mix,
        0.25
    );
}

#[test]
fn slider_raw_complete_payloads_are_strict_and_references_stay_stable_on_rename() {
    let doc = document();
    let encoded = serde_json::to_value(&doc).unwrap();
    assert_eq!(
        serde_json::from_value::<Document>(encoded.clone()).unwrap(),
        doc
    );
    for field in [
        "bone", "property", "local", "loop", "from", "to", "scale", "max", "setup",
    ] {
        let mut malformed = encoded.clone();
        malformed["constraints"][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(
            serde_json::from_value::<Document>(malformed).is_err(),
            "{field}"
        );
    }
    let mut malformed = encoded.clone();
    malformed["animations"][0]
        .as_object_mut()
        .unwrap()
        .remove("slider_tracks");
    assert!(serde_json::from_value::<Document>(malformed).is_err());
    let mut malformed = encoded;
    malformed["constraints"][0]["extra"] = true.into();
    assert!(serde_json::from_value::<Document>(malformed).is_err());
    let renamed = apply_command(
        &doc,
        Command::RenameAnimation {
            animation: 0,
            name: "reference renamed".into(),
        },
    )
    .unwrap();
    assert_eq!(renamed.constraints[0].as_slider().unwrap().animation, 0);
    let duplicated = apply_command(
        &renamed,
        Command::DuplicateAnimation {
            animation: 0,
            name: "reference copy".into(),
        },
    )
    .unwrap();
    assert_eq!(duplicated.constraints[0], doc.constraints[0]);
    assert_eq!(duplicated.animations.len(), 3);
}

#[test]
fn slider_raw_exact_keys_curves_and_atomic_batches_preserve_time_and_value_bits() {
    let mut session = Session::new(document()).unwrap();
    let time = 1f32;
    let adjacent = f32::from_bits(time.to_bits() + 1);
    session
        .dispatch(Command::Batch {
            commands: vec![
                set(SliderProperty::Time, key(time, -0.25)),
                set(SliderProperty::Time, key(adjacent, 0.75)),
                set(SliderProperty::Mix, key(time, 1.5)),
            ],
        })
        .unwrap();
    let curve = Curve::BezierOffset([0.2, -0.0, 0.8, 0.75]);
    session
        .dispatch(Command::SetSliderCurve {
            animation: 1,
            constraint: 0,
            property: SliderProperty::Time,
            time,
            curve,
        })
        .unwrap();
    let track = &session.document().animations[1].slider_tracks[0];
    assert_eq!(track.keys[0].time.to_bits(), time.to_bits());
    assert_eq!(track.keys[0].value.to_bits(), (-0.25f32).to_bits());
    assert_eq!(track.keys[0].curve, curve);
    assert_eq!(track.keys[1].time.to_bits(), adjacent.to_bits());
    assert_eq!(track.keys[1].curve, Curve::Linear);
    let before = session.document().clone();
    let history = (session.can_undo(), session.can_redo());
    assert!(
        session
            .dispatch(Command::Batch {
                commands: vec![
                    set(SliderProperty::Mix, key(0.5, 0.25)),
                    Command::SetSliderCurve {
                        animation: 1,
                        constraint: 0,
                        property: SliderProperty::Mix,
                        time: 0.7,
                        curve
                    },
                ]
            })
            .is_err()
    );
    assert_eq!(session.document(), &before);
    assert_eq!((session.can_undo(), session.can_redo()), history);
    assert_eq!(
        slider_key_time(session.document(), 1, adjacent, 0, SliderProperty::Time).unwrap(),
        Some(adjacent)
    );
}

#[test]
fn slider_typed_channels_share_clipboard_origin_and_keep_time_mix_distinct() {
    let mut session = Session::new(document()).unwrap();
    session
        .dispatch(Command::Batch {
            commands: vec![
                set(SliderProperty::Time, key(0.25, 0.5)),
                set(SliderProperty::Mix, key(0.25, 0.2)),
                set(SliderProperty::Time, key(0.75, 1.25)),
            ],
        })
        .unwrap();
    let selection = vec![
        AnimationKey::Slider {
            constraint: 0,
            property: SliderProperty::Time,
            time: 0.25,
        },
        AnimationKey::Slider {
            constraint: 0,
            property: SliderProperty::Mix,
            time: 0.25,
        },
        AnimationKey::Slider {
            constraint: 0,
            property: SliderProperty::Time,
            time: 0.75,
        },
    ];
    let clipboard = copy_animation_keys(session.document(), 1, &selection).unwrap();
    let AnimationKeyClipboard::Slider { keys } = &clipboard else {
        panic!("Slider clipboard")
    };
    assert_eq!(keys.len(), 3);
    assert_eq!(keys[0].key.time, 0.);
    assert_eq!(keys[1].key.time, 0.);
    assert_eq!(keys[2].key.time, 0.5);
    let target = KeyOwner::Slider { constraint: 0 };
    let incoming =
        pasted_animation_keys(session.document(), 1, target.clone(), 2., &clipboard).unwrap();
    assert_eq!(incoming.len(), 3);
    session
        .dispatch(Command::PasteAnimationKeys {
            animation: 1,
            target,
            time: 2.,
            clipboard,
        })
        .unwrap();
    let channels = curve_channels_for_targets(
        session.document(),
        1,
        &curve_targets(session.document(), 1, &selection[..1]).unwrap(),
    )
    .unwrap();
    assert_eq!(channels[0].values, [0.5, 1.25]);
    session
        .dispatch(Command::DeleteAnimationKeys {
            animation: 1,
            keys: incoming,
        })
        .unwrap();
    assert_eq!(
        session.document().animations[1].slider_tracks[0].keys.len(),
        2
    );
    assert_eq!(
        session.document().animations[1].slider_tracks[1].keys.len(),
        1
    );
}

#[test]
fn slider_pending_channels_stay_outside_save_and_global_key_is_one_history_step() {
    let mut session = Session::new(document()).unwrap();
    let target = EditTarget::Animate {
        animation: 1,
        time: 0.5,
    };
    let transform = Transform {
        rotation: 15.,
        ..Transform::default()
    };
    session
        .edit_pose(0, target, transform, &[Property::Rotation], false)
        .unwrap();
    let persistent = session.document().clone();
    session
        .edit_slider_pose(
            0,
            target,
            &[
                SliderPoseEdit::Time { value: 0.75 },
                SliderPoseEdit::Mix { value: 0.25 },
            ],
            false,
        )
        .unwrap();
    assert_eq!(session.document(), &persistent);
    assert_eq!(
        serde_json::to_string(session.document()).unwrap(),
        serde_json::to_string(&persistent).unwrap()
    );
    assert_eq!(
        session.slider_pose(0, target).unwrap(),
        SliderPose {
            time: 0.75,
            mix: 0.25
        }
    );
    for property in SliderProperty::ALL {
        assert_eq!(
            session
                .pose_edits()
                .slider_key_state(session.document(), 1, 0.5, 0, property)
                .unwrap(),
            KeyState::Edited
        );
    }
    let pending = session.pose_edits().clone();
    session.key_edited(1, 0.5, Some(0)).unwrap();
    assert!(session.document().animations[1].slider_tracks.is_empty());
    assert!(session.pose_edits().has_slider(1, 0, SliderProperty::Time));
    session.undo();
    assert_eq!(session.document(), &persistent);
    assert_eq!(session.pose_edits(), &pending);
    session.key_edited(1, 0.5, None).unwrap();
    assert!(session.pose_edits().is_empty());
    assert_eq!(session.document().animations[1].tracks.len(), 1);
    assert_eq!(session.document().animations[1].slider_tracks.len(), 2);
    assert_eq!(
        session.slider_pose(0, target).unwrap(),
        SliderPose {
            time: 0.75,
            mix: 0.25
        }
    );
    session.undo();
    assert_eq!(session.document(), &persistent);
    assert_eq!(session.pose_edits(), &pending);
    session.redo();
    assert!(session.pose_edits().is_empty());
    let evaluated = session.evaluated_pose(target).unwrap();
    assert_eq!(
        evaluated.authored_slider_poses()[0],
        SliderPose {
            time: 0.75,
            mix: 0.25
        }
    );
    assert_eq!(
        evaluated.applied_slider_poses()[0],
        evaluated.authored_slider_poses()[0]
    );
}

#[test]
fn slider_pending_previews_cancel_or_commit_once_and_failures_keep_last_valid_candidate() {
    let mut session = Session::new(document()).unwrap();
    let target = EditTarget::Animate {
        animation: 1,
        time: 0.25,
    };
    session
        .dispatch(Command::RenameAnimation {
            animation: 1,
            name: "redo marker".into(),
        })
        .unwrap();
    session.undo();
    let before = session.document().clone();
    let pending = session.pose_edits().clone();
    assert!(session.can_redo());
    assert!(
        session
            .preview_edit_slider_pose(0, target, &[SliderPoseEdit::Mix { value: 0.5 }], false)
            .is_err()
    );
    session.begin_transaction();
    session
        .preview_edit_slider_pose(0, target, &[SliderPoseEdit::Time { value: 0.75 }], false)
        .unwrap();
    let valid = session.pose_edits().clone();
    assert!(
        session
            .preview_edit_slider_pose(
                0,
                target,
                &[SliderPoseEdit::Time { value: f32::NAN }],
                false
            )
            .is_err()
    );
    assert_eq!(session.pose_edits(), &valid);
    session
        .preview_edit_slider_pose(0, target, &[SliderPoseEdit::Mix { value: 0.25 }], false)
        .unwrap();
    assert_eq!(
        session.slider_pose(0, target).unwrap(),
        SliderPose {
            time: 0.,
            mix: 0.25
        }
    );
    session.cancel_transaction();
    assert_eq!(session.document(), &before);
    assert_eq!(session.pose_edits(), &pending);
    assert!(session.can_redo());
    session.begin_transaction();
    session
        .preview_edit_slider_pose(0, target, &[SliderPoseEdit::Mix { value: 0.25 }], false)
        .unwrap();
    session.commit_transaction();
    assert!(!session.can_redo());
    session.undo();
    assert_eq!(session.document(), &before);
    assert_eq!(session.pose_edits(), &pending);
    session.redo();
    assert_eq!(session.slider_pose(0, target).unwrap().mix, 0.25);
    assert_eq!(session.document(), &before);
}

#[test]
fn slider_setup_and_autokey_change_only_explicit_properties_preserving_raw_curves_and_redo_noops() {
    let mut session = Session::new(document()).unwrap();
    session
        .edit_slider_pose(
            0,
            EditTarget::Setup,
            &[SliderPoseEdit::Time { value: 0.5 }],
            true,
        )
        .unwrap();
    assert_eq!(
        session.document().constraints[0].as_slider().unwrap().setup,
        SliderSetup { time: 0.5, mix: 1. }
    );
    assert!(session.pose_edits().is_empty());
    assert!(session.document().animations[1].slider_tracks.is_empty());
    let fractional = 0.1234567f32;
    let curve = Curve::BezierOffset([0.2, -0.125, 0.8, 0.5]);
    session
        .dispatch(set(
            SliderProperty::Mix,
            Keyframe {
                time: fractional,
                value: 0.5,
                curve,
            },
        ))
        .unwrap();
    let target = EditTarget::Animate {
        animation: 1,
        time: fractional,
    };
    session
        .edit_slider_pose(0, target, &[SliderPoseEdit::Time { value: 0.75 }], false)
        .unwrap();
    session
        .edit_slider_pose(0, target, &[SliderPoseEdit::Mix { value: 0.25 }], true)
        .unwrap();
    assert!(session.pose_edits().has_slider(1, 0, SliderProperty::Time));
    assert!(!session.pose_edits().has_slider(1, 0, SliderProperty::Mix));
    let keys = &session.document().animations[1].slider_tracks[0].keys;
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].time.to_bits(), fractional.to_bits());
    assert_eq!(keys[0].curve, curve);
    assert_eq!(keys[0].value, 0.25);
    let document = session.document().clone();
    let pending = session.pose_edits().clone();
    session
        .dispatch(Command::RenameAnimation {
            animation: 1,
            name: "redo marker".into(),
        })
        .unwrap();
    session.undo();
    assert!(session.can_redo());
    session
        .edit_slider_pose(0, target, &[SliderPoseEdit::Mix { value: 0.25 }], true)
        .unwrap();
    assert_eq!(session.document(), &document);
    assert_eq!(session.pose_edits(), &pending);
    assert!(session.can_redo());
}

#[test]
fn slider_unrepresentable_snapped_key_and_invalid_batches_preserve_all_pending_state() {
    let mut session = Session::new(document()).unwrap();
    let target = EditTarget::Animate {
        animation: 1,
        time: 0.009,
    };
    session
        .edit_slider_pose(0, target, &[SliderPoseEdit::Time { value: 0.75 }], false)
        .unwrap();
    let before = session.document().clone();
    let pending = session.pose_edits().clone();
    let history = (session.can_undo(), session.can_redo());
    let error = session
        .key_slider_pose(1, 0.009, 0, &[SliderProperty::Time])
        .unwrap_err();
    assert!(error.contains("current authored pose"));
    assert_eq!(session.document(), &before);
    assert_eq!(session.pose_edits(), &pending);
    assert_eq!((session.can_undo(), session.can_redo()), history);
    assert!(
        session
            .edit_slider_pose(
                0,
                target,
                &[
                    SliderPoseEdit::Time { value: 0.5 },
                    SliderPoseEdit::Time { value: 1. },
                ],
                false
            )
            .is_err()
    );
    assert!(
        session
            .edit_slider_pose(
                0,
                target,
                &[
                    SliderPoseEdit::Time { value: 0.5 },
                    SliderPoseEdit::Mix {
                        value: f32::INFINITY
                    },
                ],
                true
            )
            .is_err()
    );
    assert_eq!(session.document(), &before);
    assert_eq!(session.pose_edits(), &pending);
}

#[test]
fn slider_pending_raw_commands_do_not_clear_other_channel_and_context_discard_uses_shared_history()
{
    let mut session = Session::new(document()).unwrap();
    let target = EditTarget::Animate {
        animation: 1,
        time: 0.5,
    };
    session
        .edit_slider_pose(0, target, &[SliderPoseEdit::Time { value: 0.75 }], false)
        .unwrap();
    session
        .dispatch(set(SliderProperty::Mix, key(0.5, 0.25)))
        .unwrap();
    assert!(session.pose_edits().has_slider(1, 0, SliderProperty::Time));
    assert_eq!(
        session.slider_pose(0, target).unwrap(),
        SliderPose {
            time: 0.75,
            mix: 0.25
        }
    );
    let pending = session.pose_edits().clone();
    session.discard_pose(true);
    assert!(session.pose_edits().is_empty());
    session.undo();
    assert_eq!(session.pose_edits(), &pending);
    session.discard_pose(false);
    assert!(session.pose_edits().is_empty());
    session.undo();
    assert!(session.pose_edits().is_empty());
}
