//! Original source-transition cases: persistent data and process view state have
//! different lifetimes, while failed edits and canceled gestures publish neither.
use crate::*;
use peachy_document::*;

fn key(time: f32, value: f32) -> Keyframe {
    Keyframe {
        time,
        value,
        curve: Curve::BezierOffset([0.25, -0., 0.75, 0.125]),
    }
}
fn document() -> Document {
    let mut doc = Document::new("Original source choices".into(), 60).unwrap();
    for name in ["driver", "other driver"] {
        doc.bones.push(Bone {
            name: name.into(),
            parent: Some(0),
            length: 0.,
            setup: Transform::default(),
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
        Animation::new("other host", vec![]),
    ];
    for name in ["first", "second"] {
        let mut slider = SliderConstraint::new(name, 0);
        slider.setup = SliderSetup {
            time: 0.75,
            mix: 0.5,
        };
        doc.constraints.push(Constraint::Slider(slider));
    }
    for clip in &mut doc.animations[1..] {
        for constraint in 0..2 {
            for property in SliderProperty::ALL {
                clip.slider_tracks.push(SliderTrack {
                    constraint,
                    property,
                    keys: vec![key(
                        0.,
                        if property == SliderProperty::Time {
                            0.5
                        } else {
                            0.25
                        },
                    )],
                });
            }
        }
    }
    doc.validate().unwrap();
    doc
}
fn slider(doc: &Document, index: usize) -> &SliderConstraint {
    doc.constraints[index].as_slider().unwrap()
}
fn pending(session: &mut Session) {
    let target = EditTarget::Animate {
        animation: 1,
        time: 0.125,
    };
    for constraint in 0..2 {
        session
            .edit_slider_pose(
                constraint,
                target,
                &[
                    SliderPoseEdit::Time { value: 0.875 },
                    SliderPoseEdit::Mix { value: 0.625 },
                ],
                false,
            )
            .unwrap();
    }
}
fn range(min: f32, max: f32) -> SliderSourceRange {
    SliderSourceRange { min, max }
}
fn replace_range(session: &mut Session, constraint: usize, min: f32, max: f32) {
    let current = slider(session.document(), constraint);
    let mut value = current.clone();
    value.from = min;
    value.scale = (value.max - value.to) / (max - min);
    session
        .dispatch(Command::SetSliderConstraint { constraint, value })
        .unwrap();
}

#[test]
fn slider_source_assignment_cleans_only_time_and_undo_restores_document_and_pending() {
    let original = document();
    let mut session = Session::new(original.clone()).unwrap();
    pending(&mut session);
    let prior_pose = serde_json::to_value(session.pose_edits()).unwrap();
    session.set_slider_source(0, Some(1)).unwrap();
    let bound = slider(session.document(), 0);
    assert_eq!(bound.property, Some(TransformProperty::Rotate));
    assert!(bound.local);
    assert_eq!((bound.from, bound.to, bound.max), (0., 0., 1.5));
    assert_eq!(
        bound.setup,
        SliderSetup {
            time: 0.75,
            mix: 0.5
        }
    );
    assert_eq!(
        session
            .slider_source_property_range(0, TransformProperty::Rotate)
            .unwrap(),
        range(0., 100.)
    );
    for (after, before) in session
        .document()
        .animations
        .iter()
        .zip(&original.animations)
    {
        let expected: Vec<_> = before
            .slider_tracks
            .iter()
            .filter(|track| !(track.constraint == 0 && track.property == SliderProperty::Time))
            .cloned()
            .collect();
        assert_eq!(after.slider_tracks, expected);
    }
    let target = EditTarget::Animate {
        animation: 1,
        time: 0.125,
    };
    assert_eq!(
        session.slider_pose(0, target).unwrap(),
        SliderPose {
            time: 0.,
            mix: 0.625
        }
    );
    assert_eq!(
        session.slider_pose(1, target).unwrap(),
        SliderPose {
            time: 0.875,
            mix: 0.625
        }
    );
    let committed = session.document().clone();
    session.undo();
    assert_eq!(session.document(), &original);
    assert_eq!(
        serde_json::to_value(session.pose_edits()).unwrap(),
        prior_pose
    );
    session.redo();
    assert_eq!(session.document(), &committed);
    assert_eq!(session.slider_pose(0, target).unwrap().mix, 0.625);
    let hidden = slider(session.document(), 0).clone();
    session.set_slider_source(0, None).unwrap();
    let mut expected = hidden.clone();
    expected.bone = None;
    assert_eq!(slider(session.document(), 0), &expected);
    session.set_slider_source(0, Some(2)).unwrap();
    expected.bone = Some(2);
    assert_eq!(slider(session.document(), 0), &expected);
}

#[test]
fn slider_source_nested_structural_batch_cleans_the_correct_pending_identity() {
    let mut session = Session::new(document()).unwrap();
    pending(&mut session);
    let before = session.document().clone();
    let pose = serde_json::to_value(session.pose_edits()).unwrap();
    session
        .dispatch(Command::Batch {
            commands: vec![
                Command::CreateSliderConstraint { animation: 0 },
                Command::Batch {
                    commands: vec![
                        Command::SetSliderSource {
                            constraint: 1,
                            bone: Some(1),
                        },
                        Command::CreateSliderConstraint { animation: 0 },
                    ],
                },
            ],
        })
        .unwrap();
    let target = EditTarget::Animate {
        animation: 1,
        time: 0.125,
    };
    assert_eq!(
        session.slider_pose(2, target).unwrap(),
        SliderPose {
            time: 0.,
            mix: 0.625
        }
    );
    assert_eq!(
        session.slider_pose(3, target).unwrap(),
        SliderPose {
            time: 0.875,
            mix: 0.625
        }
    );
    assert_eq!(session.slider_source_focus(), Some(2));
    assert!(
        !session.document().animations[1]
            .slider_tracks
            .iter()
            .any(|track| track.constraint == 2 && track.property == SliderProperty::Time)
    );
    session.undo();
    assert_eq!(session.document(), &before);
    assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pose);
    assert_eq!(session.slider_source_focus(), Some(0));
    session.redo();
    assert_eq!(session.slider_source_focus(), Some(2));
}

#[test]
fn slider_source_choices_are_global_per_property_readonly_and_factory_bind_ignores_them() {
    let mut session = Session::new(document()).unwrap();
    for property in TransformProperty::ALL {
        let expected = if matches!(
            property,
            TransformProperty::ScaleX | TransformProperty::ScaleY
        ) {
            range(1., 2.)
        } else {
            range(0., 100.)
        };
        assert_eq!(
            session.slider_source_preference(property).unwrap(),
            expected
        );
    }
    session.set_slider_source(0, Some(1)).unwrap();
    replace_range(&mut session, 0, 12., 100.);
    session
        .set_slider_source_property(0, TransformProperty::ScaleX)
        .unwrap();
    replace_range(&mut session, 0, 4., 2.);
    session
        .set_slider_source_property(0, TransformProperty::Rotate)
        .unwrap();
    assert_eq!(
        session
            .slider_source_property_range(0, TransformProperty::Rotate)
            .unwrap(),
        range(12., 100.)
    );
    session.set_slider_source(1, Some(2)).unwrap();
    // Native first assignment is a factory range even after another Slider
    // remembers a nondefault Rotate range. Choice queries remain contextual.
    assert_eq!(slider(session.document(), 1).from, 0.);
    let prefs = session.slider_source_preferences().clone();
    let focus = session.slider_source_focus();
    let before = session.document().clone();
    let history = (session.can_undo(), session.can_redo());
    assert_eq!(
        session
            .slider_source_property_range(1, TransformProperty::ScaleX)
            .unwrap(),
        range(4., 2.)
    );
    let command = session
        .slider_source_property_command(1, TransformProperty::ScaleX)
        .unwrap();
    assert_eq!(session.slider_source_preferences(), &prefs);
    assert_eq!(session.slider_source_focus(), focus);
    assert_eq!(session.document(), &before);
    assert_eq!((session.can_undo(), session.can_redo()), history);
    let old = slider(session.document(), 1).clone();
    session.dispatch(command).unwrap();
    let changed = slider(session.document(), 1);
    assert_eq!((changed.from, changed.scale), (4., -0.75));
    assert_eq!(
        (changed.to, changed.max, changed.local, changed.setup),
        (old.to, old.max, old.local, old.setup)
    );
    replace_range(&mut session, 1, 5., 2.);
    session
        .set_slider_source_property(1, TransformProperty::Rotate)
        .unwrap();
    session.focus_slider_source(Some(0)).unwrap();
    session
        .set_slider_source_property(0, TransformProperty::ScaleX)
        .unwrap();
    assert_eq!(
        session
            .slider_source_property_range(0, TransformProperty::ScaleX)
            .unwrap(),
        range(5., 2.)
    );
}

#[test]
fn slider_source_previews_cancel_memo_and_opaque_open_transfer_remembers_accepted_context() {
    let mut session = Session::new(document()).unwrap();
    session.set_slider_source(0, Some(1)).unwrap();
    session
        .set_slider_source_property(0, TransformProperty::ScaleX)
        .unwrap();
    let original = session.document().clone();
    let prefs = session.slider_source_preferences().clone();
    session
        .dispatch(Command::RenameBone {
            bone: 0,
            name: "temporary".into(),
        })
        .unwrap();
    session.undo();
    assert!(session.can_redo());
    session.begin_transaction();
    let mut candidate = slider(session.document(), 0).clone();
    candidate.from = 8.;
    candidate.scale = -0.25;
    session
        .preview_transaction(Command::SetSliderConstraint {
            constraint: 0,
            value: candidate,
        })
        .unwrap();
    assert_eq!(session.slider_source_preferences(), &prefs);
    assert!(session.focus_slider_source(Some(1)).is_err());
    session.cancel_transaction();
    assert_eq!(session.document(), &original);
    assert_eq!(session.slider_source_preferences(), &prefs);
    assert!(session.can_redo());
    replace_range(&mut session, 0, 8., 2.);
    let before_transfer = session.slider_source_preferences().clone();
    let transfer = session.slider_source_preferences_for_transfer();
    assert_eq!(session.slider_source_preferences(), &before_transfer);
    let mut opened = Session::new(document()).unwrap();
    opened.set_slider_source_preferences(transfer).unwrap();
    assert_eq!(opened.slider_source_focus(), None);
    assert_eq!(
        opened
            .slider_source_preference(TransformProperty::ScaleX)
            .unwrap(),
        range(8., 2.)
    );
    assert_eq!(opened.document(), &document());
    assert!(!opened.can_undo());
    session
        .dispatch(Command::CreateSliderConstraint { animation: 0 })
        .unwrap();
    session.focus_slider_source(Some(0)).unwrap();
    session.undo();
    assert_eq!(session.slider_source_focus(), None); // Removed new Slider must not redirect to old0.
    session.redo();
    assert_eq!(session.slider_source_focus(), None);
}

#[test]
fn slider_source_failures_and_exact_noops_preserve_redo_pending_and_process_preferences() {
    let mut session = Session::new(document()).unwrap();
    pending(&mut session);
    session.set_slider_source(0, Some(1)).unwrap();
    session
        .dispatch(Command::RenameBone {
            bone: 0,
            name: "temporary".into(),
        })
        .unwrap();
    session.undo();
    let doc = session.document().clone();
    let pose = serde_json::to_value(session.pose_edits()).unwrap();
    let prefs = session.slider_source_preferences().clone();
    let focus = session.slider_source_focus();
    session.set_slider_source(0, Some(1)).unwrap();
    session
        .dispatch(Command::SetSliderSourceProperty {
            constraint: 0,
            property: TransformProperty::Rotate,
            source_range: range(50., 51.),
        })
        .unwrap();
    assert!(session.can_redo());
    for bad in [
        Command::SetSliderSource {
            constraint: 0,
            bone: Some(99),
        },
        Command::SetSliderSourceProperty {
            constraint: 0,
            property: TransformProperty::X,
            source_range: range(1., 1.),
        },
        Command::SetSliderSourceProperty {
            constraint: 0,
            property: TransformProperty::X,
            source_range: range(0., f32::INFINITY),
        },
        Command::Batch {
            commands: vec![
                Command::SetSliderSourceProperty {
                    constraint: 0,
                    property: TransformProperty::ScaleY,
                    source_range: range(9., 2.),
                },
                Command::SetSliderSource {
                    constraint: 99,
                    bone: None,
                },
            ],
        },
    ] {
        assert!(session.dispatch(bad).is_err());
    }
    assert_eq!(session.document(), &doc);
    assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pose);
    assert_eq!(session.slider_source_preferences(), &prefs);
    assert_eq!(session.slider_source_focus(), focus);
    assert!(session.can_redo());
    for bad in [
        serde_json::json!({"command":"setSliderSource","constraint":0}),
        serde_json::json!({"command":"setSliderSource","constraint":0,"bone":null,"range":{}}),
        serde_json::json!({"command":"setSliderSourceProperty","constraint":0,"property":"x","source_range":{"min":0,"max":100,"extra":0}}),
    ] {
        assert!(serde_json::from_value::<Command>(bad).is_err());
    }
    assert!(
        serde_json::from_value::<Command>(
            serde_json::json!({"command":"setSliderSource","constraint":0,"bone":null})
        )
        .is_ok()
    );
}

#[test]
fn slider_source_property_wire_requires_one_of_the_six_exact_strings() {
    for property in TransformProperty::ALL {
        let input = serde_json::json!({
            "command":"setSliderSourceProperty", "constraint":0,
            "property":property.spine_name(), "source_range":{"min":1.0,"max":2.0}
        });
        let command = serde_json::from_value::<Command>(input.clone()).unwrap();
        let Command::SetSliderSourceProperty {
            property: actual, ..
        } = &command
        else {
            panic!("Source property command decoded into another command");
        };
        assert_eq!(*actual, property);
        assert_eq!(serde_json::to_value(command).unwrap(), input);
    }
    for property in [
        serde_json::json!({"scaleX":null}),
        serde_json::json!({"x":null}),
        serde_json::json!({}),
        serde_json::Value::Null,
        serde_json::json!([]),
        serde_json::json!(["scaleX"]),
        serde_json::json!(true),
        serde_json::json!(0),
        serde_json::json!("ScaleX"),
        serde_json::json!("shearX"),
        serde_json::json!(""),
    ] {
        let input = serde_json::json!({
            "command":"setSliderSourceProperty", "constraint":0,
            "property":property, "source_range":{"min":1,"max":2}
        });
        assert!(
            serde_json::from_value::<Command>(input.clone()).is_err(),
            "Accepted {input}"
        );
    }
    assert!(
        serde_json::from_value::<Command>(serde_json::json!({
            "command":"setSliderSourceProperty", "constraint":0,
            "source_range":{"min":1,"max":2}
        }))
        .is_err()
    );
}

#[test]
fn slider_source_range_wire_is_object_only_and_rejects_duplicate_or_missing_endpoints() {
    let input = serde_json::json!({"min":1.0,"max":2.0});
    let decoded = serde_json::from_value::<SliderSourceRange>(input.clone()).unwrap();
    assert_eq!(decoded, range(1., 2.));
    assert_eq!(serde_json::to_value(decoded).unwrap(), input);
    for invalid in [
        "[1,2]",
        "[]",
        "[1,2,3]",
        "null",
        "false",
        "1",
        "\"1,2\"",
        "{}",
        "{\"min\":1}",
        "{\"max\":2}",
        "{\"min\":1,\"max\":2,\"extra\":3}",
        "{\"min\":1,\"min\":2,\"max\":3}",
        "{\"min\":1,\"max\":2,\"max\":3}",
    ] {
        assert!(
            serde_json::from_str::<SliderSourceRange>(invalid).is_err(),
            "Accepted range {invalid}"
        );
        let command = format!(
            "{{\"command\":\"setSliderSourceProperty\",\"constraint\":0,\"property\":\"scaleX\",\"source_range\":{invalid}}}"
        );
        assert!(
            serde_json::from_str::<Command>(&command).is_err(),
            "Accepted command {command}"
        );
    }
}
