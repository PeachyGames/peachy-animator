//! Setup edits use one actor-turn snapshot and preserve raw replacement paths.
use crate::*;
use peachy_document::*;

fn key(time: f32, value: f32) -> Keyframe {
    Keyframe {
        time,
        value,
        curve: Curve::Linear,
    }
}
fn document() -> Document {
    let mut doc = Document::new("Original Slider Setup lab".into(), 60).unwrap();
    for (name, x) in [("source", 27.), ("follower", 0.)] {
        doc.bones.push(Bone {
            name: name.into(),
            parent: Some(0),
            length: 0.,
            setup: Transform {
                x,
                ..Transform::default()
            },
            inherit: Inherit::Normal,
        });
    }
    doc.animations = vec![
        Animation::new(
            "reference",
            vec![Track {
                bone: 2,
                property: Property::X,
                keys: vec![key(0., 0.), key(1.5, 15.)],
            }],
        ),
        Animation::new("host", vec![]),
    ];
    let mut slider = SliderConstraint::new("calibration", 0);
    slider.bone = Some(1);
    slider.property = Some(TransformProperty::X);
    slider.local = true;
    slider.scale = 0.0625;
    slider.max = 1.5;
    slider.looped = true;
    slider.setup = SliderSetup {
        time: 0.75,
        mix: 1.,
    };
    doc.constraints.push(Constraint::Slider(slider));
    doc.validate().unwrap();
    doc
}
fn slider(doc: &Document) -> &SliderConstraint {
    doc.constraints[0].as_slider().unwrap()
}
fn slider_mut(doc: &mut Document) -> &mut SliderConstraint {
    let Constraint::Slider(value) = &mut doc.constraints[0] else {
        unreachable!()
    };
    value
}
fn edit(mix: f32) -> Command {
    Command::SetSliderSetupPose {
        constraint: 0,
        edits: vec![SliderPoseEdit::Mix { value: mix }],
        skin: "default".into(),
    }
}

#[test]
fn slider_setup_own_turn_metadata_survives_self_overwrites_and_later_source_changes() {
    for overwrite_self in [false, true] {
        let mut doc = document();
        if overwrite_self {
            doc.animations[0].slider_tracks = vec![
                SliderTrack {
                    constraint: 0,
                    property: SliderProperty::Time,
                    keys: vec![key(0., 0.5)],
                },
                SliderTrack {
                    constraint: 0,
                    property: SliderProperty::Mix,
                    keys: vec![key(0., 0.25)],
                },
            ];
        }
        doc.animations.push(Animation::new(
            "late source",
            vec![Track {
                bone: 1,
                property: Property::X,
                keys: vec![key(0., 100.)],
            }],
        ));
        doc.constraints
            .push(Constraint::Slider(SliderConstraint::new("later", 2)));
        doc.validate().unwrap();
        let session = Session::new(doc).unwrap();
        let calls = crate::ik::checked_pose_calls();
        let pose = session.evaluated_pose(EditTarget::Setup).unwrap();
        assert_eq!(crate::ik::checked_pose_calls(), calls + 1);
        let turn = pose.slider_evaluations()[0].unwrap();
        assert_eq!(turn.eligibility, SliderEligibility::Eligible);
        assert_eq!(turn.incoming, SliderPose { time: 0., mix: 1. });
        assert_eq!(
            turn.applied,
            SliderPose {
                time: 1.6875,
                mix: 1.
            }
        );
        assert_eq!(turn.sample_time, Some(0.1875));
        assert_eq!(pose.applied_locals()[1].x, 127.);
        assert_eq!(pose.applied_locals()[2].x, 1.875);
        if overwrite_self {
            assert_eq!(
                pose.applied_slider_poses()[0],
                SliderPose {
                    time: 0.5,
                    mix: 0.25
                }
            );
        } else {
            assert_eq!(pose.applied_slider_poses()[0], turn.applied);
        }
    }
}

#[test]
fn slider_setup_mix_capture_uses_sampled_own_turn_once_and_history_preserves_pending() {
    let doc = document();
    let mut session = Session::new(doc.clone()).unwrap();
    let target = EditTarget::Animate {
        animation: 1,
        time: 0.,
    };
    let mut local = doc.bones[2].setup;
    local.x = 3.;
    session
        .edit_pose(2, target, local, &[Property::X], false)
        .unwrap();
    session
        .edit_slider_pose(0, target, &[SliderPoseEdit::Mix { value: 0.125 }], false)
        .unwrap();
    let pending = serde_json::to_value(session.pose_edits()).unwrap();
    session
        .dispatch(Command::RenameBone {
            bone: 0,
            name: "temporary".into(),
        })
        .unwrap();
    session.undo();
    assert!(session.can_redo());
    let calls = crate::ik::checked_pose_calls();
    session
        .edit_slider_pose(
            0,
            EditTarget::Setup,
            &[SliderPoseEdit::Mix { value: 1. }],
            false,
        )
        .unwrap();
    assert_eq!(crate::ik::checked_pose_calls(), calls);
    assert!(session.can_redo());
    assert_eq!(slider(session.document()).setup.time, 0.75);
    session
        .edit_slider_pose(
            0,
            EditTarget::Setup,
            &[SliderPoseEdit::Mix { value: 0.5 }],
            false,
        )
        .unwrap();
    assert_eq!(crate::ik::checked_pose_calls(), calls + 1);
    assert_eq!(
        slider(session.document()).setup,
        SliderSetup {
            time: 0.1875,
            mix: 0.5
        }
    );
    assert_eq!(session.document().animations, doc.animations);
    assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pending);
    let after = session.document().clone();
    session.undo();
    assert_eq!(session.document(), &doc);
    assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pending);
    session.redo();
    assert_eq!(session.document(), &after);
    assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pending);
}

#[test]
fn slider_setup_raw_replacement_and_manual_edits_do_not_capture_or_normalize() {
    let doc = document();
    let mut value = slider(&doc).clone();
    value.setup.mix = 0.25;
    let calls = crate::ik::checked_pose_calls();
    let raw = apply_command(
        &doc,
        Command::SetSliderConstraint {
            constraint: 0,
            value,
        },
    )
    .unwrap();
    assert_eq!(crate::ik::checked_pose_calls(), calls);
    assert_eq!(
        slider(&raw).setup,
        SliderSetup {
            time: 0.75,
            mix: 0.25
        }
    );
    let mut manual = doc;
    let value = slider_mut(&mut manual);
    value.bone = None;
    value.property = None;
    let result = apply_command(
        &manual,
        Command::SetSliderSetupPose {
            constraint: 0,
            edits: vec![
                SliderPoseEdit::Time { value: 2.25 },
                SliderPoseEdit::Mix { value: 0.5 },
            ],
            skin: "default".into(),
        },
    )
    .unwrap();
    assert_eq!(crate::ik::checked_pose_calls(), calls);
    assert_eq!(
        slider(&result).setup,
        SliderSetup {
            time: 2.25,
            mix: 0.5
        }
    );
    assert!(
        apply_command(
            &manual,
            Command::SetSliderSetupPose {
                constraint: 0,
                edits: vec![SliderPoseEdit::Time { value: -0.25 }],
                skin: "default".into()
            }
        )
        .is_err()
    );
    let valid = serde_json::json!({"command":"setSliderSetupPose","constraint":0,"edits":[{"type":"mix","value":0.5}],"skin":"default"});
    assert!(serde_json::from_value::<Command>(valid.clone()).is_ok());
    let mut wrong = valid;
    wrong["edits"][0] = serde_json::json!({"property":"mix","value":0.5});
    assert!(serde_json::from_value::<Command>(wrong).is_err());
}

#[test]
fn slider_setup_inactive_and_zero_mix_turns_retain_hidden_baseline_without_forced_mapping() {
    for mode in [0, 1, 2] {
        let mut doc = document();
        match mode {
            0 => {
                doc.skin_scope.required_constraints.insert(0);
            }
            1 => {
                doc.skin_scope.required_bones.insert(1);
            }
            _ => slider_mut(&mut doc).setup.mix = 0.,
        }
        let session = Session::new(doc.clone()).unwrap();
        let turn = session.slider_evaluation(0, EditTarget::Setup).unwrap();
        assert_eq!(turn.sample_time, None);
        assert_eq!(
            turn.eligibility,
            match mode {
                0 => SliderEligibility::InactiveConstraint,
                1 => SliderEligibility::InactiveSource,
                _ => SliderEligibility::Eligible,
            }
        );
        let after = apply_command(&doc, edit(0.5)).unwrap();
        assert_eq!(
            slider(&after).setup,
            SliderSetup {
                time: 0.75,
                mix: 0.5
            }
        );
    }
}

#[test]
fn slider_setup_invalid_edits_and_unrepresentable_capture_fail_atomically() {
    let doc = document();
    for command in [
        Command::SetSliderSetupPose {
            constraint: 0,
            edits: vec![SliderPoseEdit::Time { value: 0. }],
            skin: "default".into(),
        },
        Command::SetSliderSetupPose {
            constraint: 0,
            edits: vec![SliderPoseEdit::Mix { value: 1. }],
            skin: "unknown".into(),
        },
        Command::SetSliderSetupPose {
            constraint: 0,
            edits: vec![
                SliderPoseEdit::Mix { value: 0.5 },
                SliderPoseEdit::Mix { value: 0.5 },
            ],
            skin: "default".into(),
        },
    ] {
        assert!(apply_command(&doc, command).is_err());
    }
    let mut huge = doc.clone();
    huge.bones[1].setup.x = f32::MAX;
    let value = slider_mut(&mut huge);
    value.looped = false;
    value.scale = 2.;
    let mut session = Session::new(huge.clone()).unwrap();
    session
        .dispatch(Command::RenameBone {
            bone: 0,
            name: "temporary".into(),
        })
        .unwrap();
    session.undo();
    let history = (session.can_undo(), session.can_redo());
    let error = session.dispatch(edit(0.5)).unwrap_err();
    assert!(error.contains("captured Setup Frame") && error.contains("finite native range"));
    assert_eq!(session.document(), &huge);
    assert_eq!((session.can_undo(), session.can_redo()), history);
    let mut malformed = slider(&doc).clone();
    malformed.bone = Some(usize::MAX);
    assert!(
        apply_command(
            &doc,
            Command::Batch {
                commands: vec![
                    Command::SetSliderConstraint {
                        constraint: 0,
                        value: malformed
                    },
                    edit(0.5),
                ]
            }
        )
        .is_err()
    );
    assert_eq!(doc, document());
}
