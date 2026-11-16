use super::*;
use peachy_document::*;
use std::path::Path;

fn input(owners: Value) -> Value {
    json!({"expected_revision":7,"animation":0,"owners":owners})
}

#[test]
fn animation_keys_query_schema_and_decoder_require_explicit_strict_owner_objects() {
    let shape = schema::animation_keys_query_schema();
    assert_eq!(shape["type"], "object");
    assert_eq!(shape["additionalProperties"], false);
    assert_eq!(
        shape["required"],
        json!(["expected_revision", "animation", "owners"])
    );
    assert_eq!(shape["properties"]["owners"]["uniqueItems"], true);
    let owners = json!([
        {"type":"bone","bone":0}, {"type":"ik","constraint":1},
        {"type":"slider","constraint":0},
        {"type":"deform","attachment":{"skin":"default","slot":"body","placeholder":"mesh"}},
        {"type":"slot","slot":0}, {"type":"drawOrder"}
    ]);
    let decoded: AnimationKeysInput = serde_json::from_value(input(owners.clone())).unwrap();
    assert_eq!(decoded.expected_revision, 7);
    assert_eq!(decoded.animation, 0);
    assert_eq!(
        decoded
            .owners
            .into_iter()
            .map(|owner| serde_json::to_value(owner.0).unwrap())
            .collect::<Vec<_>>(),
        owners.as_array().unwrap().clone()
    );
    assert!(serde_json::from_value::<AnimationKeysInput>(input(json!([]))).is_ok());
    for field in ["expected_revision", "animation", "owners"] {
        let mut missing = input(owners.clone());
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<AnimationKeysInput>(missing).is_err());
    }
    for (field, value) in [
        ("expected_revision", json!(-1)),
        ("expected_revision", Value::Null),
        ("expected_revision", json!("7")),
        ("animation", json!(-1)),
        ("animation", Value::Null),
        ("animation", json!(0.5)),
        ("owners", Value::Null),
        ("owners", json!({})),
        ("future", json!(true)),
    ] {
        let mut invalid = input(owners.clone());
        invalid[field] = value;
        assert!(serde_json::from_value::<AnimationKeysInput>(invalid).is_err());
    }
    for invalid in [json!(null), json!([7, 0, []]), json!("query"), json!(true)] {
        assert!(serde_json::from_value::<AnimationKeysInput>(invalid).is_err());
    }
    for owner in [
        json!(null),
        json!([]),
        json!(["bone", 0]),
        json!("bone"),
        json!(1),
        json!(true),
        json!({}),
        json!({"type":null,"bone":0}),
        json!({"type":{"bone":null},"bone":0}),
        json!({"type":"bone"}),
        json!({"type":"bone","bone":null}),
        json!({"type":"bone","bone":-1}),
        json!({"type":"bone","bone":0,"slot":0}),
        json!({"type":"drawOrder","slot":0}),
        json!({"type":"unknown"}),
        json!({"type":"deform","attachment":["default","body","mesh"]}),
        json!({"type":"deform","attachment":null}),
        json!({"type":"deform","attachment":{"skin":"default","slot":"body"}}),
        json!({"type":"deform","attachment":{"skin":1,"slot":"body","placeholder":"mesh"}}),
        json!({"type":"deform","attachment":{"skin":"default","slot":"body","placeholder":"mesh","future":0}}),
    ] {
        assert!(
            serde_json::from_value::<AnimationKeysInput>(input(json!([owner.clone()]))).is_err(),
            "Accepted owner {owner}"
        );
    }
}

#[test]
fn animation_keys_query_duplicate_fields_are_rejected_before_execution() {
    for invalid in [
        r#"{"expected_revision":7,"expected_revision":7,"animation":0,"owners":[]}"#,
        r#"{"expected_revision":7,"animation":0,"animation":0,"owners":[]}"#,
        r#"{"expected_revision":7,"animation":0,"owners":[],"owners":[]}"#,
        r#"{"expected_revision":7,"animation":0,"owners":[{"type":"bone","type":"bone","bone":0}]}"#,
        r#"{"expected_revision":7,"animation":0,"owners":[{"type":"bone","bone":0,"bone":1}]}"#,
        r#"{"expected_revision":7,"animation":0,"owners":[{"type":"deform","attachment":{"skin":"default","skin":"other","slot":"body","placeholder":"mesh"}}]}"#,
        r#"{"expected_revision":7,"animation":0,"owners":[{"type":"deform","attachment":{"skin":"default","slot":"body","slot":"other","placeholder":"mesh"}}]}"#,
        r#"{"expected_revision":7,"animation":0,"owners":[{"type":"deform","attachment":{"skin":"default","slot":"body","placeholder":"mesh","placeholder":"other"}}]}"#,
    ] {
        assert!(
            serde_json::from_str::<AnimationKeysInput>(invalid).is_err(),
            "Accepted {invalid}"
        );
    }
}

fn fixture() -> Document {
    let mut document = Document::new("Original inventory query lab".into(), 60).unwrap();
    for name in ["driver", "target"] {
        document.bones.push(Bone {
            name: name.into(),
            parent: Some(0),
            length: 10.,
            setup: Transform::default(),
            inherit: Inherit::Normal,
        });
    }
    let at = f32::from_bits(0.125_f32.to_bits() + 1);
    document.animations = vec![
        Animation::new(
            "host",
            vec![Track {
                bone: 0,
                property: Property::Rotation,
                keys: vec![Keyframe {
                    time: at,
                    value: 17.,
                    curve: Curve::BezierOffset([0.2, -0., 0.8, 0.375]),
                }],
            }],
        ),
        Animation::new(
            "reference",
            vec![Track {
                bone: 2,
                property: Property::X,
                keys: vec![Keyframe {
                    time: 1.5,
                    value: 0.,
                    curve: Curve::Linear,
                }],
            }],
        ),
    ];
    document.constraints = vec![
        Constraint::Slider(SliderConstraint::new("slider", 1)),
        Constraint::Ik(IkConstraint {
            name: "aim".into(),
            bones: vec![1],
            target: 2,
            setup: IkSetup {
                mix: 0.5,
                bend_positive: true,
            },
            stretch: false,
        }),
    ];
    document.animations[0].ik_tracks.push(IkTrack {
        constraint: 1,
        keys: vec![IkKeyframe {
            time: at,
            mix: 0.25,
            bend_positive: false,
            curve: IkCurve::Bezier {
                time: [0.123456789012345, 0.876543210987654],
                mix: [0.125, 0.875],
            },
        }],
    });
    document.animations[0].inherit_tracks.push(InheritTrack {
        bone: 0,
        keys: vec![InheritKey {
            time: at,
            inherit: Inherit::NoScale,
        }],
    });
    document.validate().unwrap();
    document
}

#[test]
fn animation_keys_query_success_and_failures_preserve_history_pending_and_preferences() {
    let server = AnimatorServer::new(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
    let mut session = Session::new(fixture()).unwrap();
    session.set_slider_source(0, Some(1)).unwrap();
    session
        .set_slider_source_property(0, TransformProperty::ScaleX)
        .unwrap();
    let target = peachy_engine::EditTarget::Animate {
        animation: 0,
        time: 0.,
    };
    session
        .edit_slider_pose(
            0,
            target,
            &[peachy_engine::SliderPoseEdit::Mix { value: 0.625 }],
            false,
        )
        .unwrap();
    session
        .dispatch(Command::CreateSliderConstraint { animation: 1 })
        .unwrap();
    session.undo();
    let document = session.document().clone();
    let pending = serde_json::to_value(session.pose_edits()).unwrap();
    let preferences = session.slider_source_preferences().clone();
    let focus = session.slider_source_focus();
    let effects = session.last_edit_effects().clone();
    {
        let mut state = server.state.lock().unwrap();
        state.revision = 7;
        state.project = Some(WorkingProject {
            session,
            source: None,
            asset_root: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        });
    }
    let before = server.get_document().structured_content.unwrap();
    let queried = server.get_animation_keys(Parameters(
        serde_json::from_value(input(json!([
            {"type":"ik","constraint":1}, {"type":"bone","bone":0}
        ])))
        .unwrap(),
    ));
    assert_ne!(queried.is_error, Some(true));
    let value = queried.structured_content.unwrap();
    assert_eq!(value["revision"], 7);
    assert!(value.get("effects").is_none());
    assert_eq!(value["selection"]["keys"][0]["type"], "ik");
    assert_eq!(value["selection"]["keys"][1]["type"], "bone");
    assert_eq!(
        value["selection"]["keys"][0]["time"],
        json!(f64::from(f32::from_bits(0.125_f32.to_bits() + 1)))
    );
    assert_eq!(
        value["inventory"]["unsupportedTimelines"][0]["type"],
        "inherit"
    );
    assert_eq!(
        value["selection"]["unsupportedTimelines"][0]["type"],
        "inherit"
    );
    let empty = server.get_animation_keys(Parameters(
        serde_json::from_value(input(json!([]))).unwrap(),
    ));
    assert_eq!(
        empty.structured_content.unwrap()["selection"]["keys"],
        json!([])
    );
    for invalid in [
        input(json!([{"type":"bone","bone":999}])),
        input(json!([{"type":"ik","constraint":0}])),
        input(json!([{"type":"bone","bone":0},{"type":"bone","bone":0}])),
        json!({"expected_revision":6,"animation":0,"owners":[]}),
        json!({"expected_revision":7,"animation":99,"owners":[]}),
    ] {
        let result =
            server.get_animation_keys(Parameters(serde_json::from_value(invalid).unwrap()));
        assert_eq!(result.is_error, Some(true));
        assert!(result.structured_content.unwrap().get("effects").is_none());
        assert_eq!(server.get_document().structured_content.unwrap(), before);
    }
    let state = server.state.lock().unwrap();
    let session = &state.project().unwrap().session;
    assert_eq!(state.revision, 7);
    assert_eq!(session.document(), &document);
    assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pending);
    assert_eq!(session.slider_source_preferences(), &preferences);
    assert_eq!(session.slider_source_focus(), focus);
    assert_eq!(session.last_edit_effects(), &effects);
    assert!(session.can_redo());
    assert_eq!(
        session.document().animations[0].ik_tracks[0].keys[0].curve,
        IkCurve::Bezier {
            time: [0.123456789012345, 0.876543210987654],
            mix: [0.125, 0.875]
        }
    );
}
