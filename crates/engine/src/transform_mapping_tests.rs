//! Native range field observations plus raw preservation and atomic candidates.
use crate::*;
use peachy_document::*;

fn fixture() -> Document {
    let mut doc = Document::new("Original mapping ranges".into(), 60).unwrap();
    for name in ["driver", "follower"] {
        doc.bones.push(Bone {
            name: name.into(),
            parent: Some(0),
            length: 20.,
            inherit: Inherit::Normal,
            setup: Transform::default(),
        });
    }
    doc.constraints
        .push(Constraint::Transform(TransformConstraint {
            name: "X to Rotate".into(),
            bones: vec![2],
            source: 1,
            offsets: TransformValues::default(),
            mix: TransformValues {
                rotate: 1.,
                ..Default::default()
            },
            mappings: vec![TransformMapping {
                from: TransformProperty::X,
                offset: 0.,
                to: vec![TransformTarget::identity(TransformProperty::Rotate)],
            }],
            local_source: false,
            local_target: false,
            additive: false,
            clamp: false,
        }));
    doc.animations.push(Animation::new("calibration", vec![]));
    doc
}

fn constraint(doc: &Document) -> &TransformConstraint {
    doc.constraints[0].as_transform().unwrap()
}
fn constraint_mut(doc: &mut Document) -> &mut TransformConstraint {
    let Constraint::Transform(value) = &mut doc.constraints[0] else {
        panic!()
    };
    value
}
fn command(field: TransformMappingRangeField, value: f64) -> Command {
    Command::EditTransformMappingRange {
        constraint: 0,
        from: TransformProperty::X,
        to: TransformProperty::Rotate,
        field,
        value,
    }
}
fn range(doc: &Document) -> TransformMappingRange {
    transform_mapping_range(doc, 0, TransformProperty::X, TransformProperty::Rotate).unwrap()
}
fn edit(doc: &Document, field: TransformMappingRangeField, value: f64) -> Document {
    apply_command(doc, command(field, value)).unwrap()
}
fn near(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 0.0001,
        "{actual} vs {expected}"
    );
}
fn assert_range(doc: &Document, expected: [f64; 4]) {
    let value = range(doc);
    for (actual, expected) in [
        value.source_min,
        value.source_max.unwrap(),
        value.target_min,
        value.target_max,
    ]
    .into_iter()
    .zip(expected)
    {
        near(actual, expected);
    }
}

#[test]
fn transform_mapping_ranges_follow_native_typed_endpoints_repairs_and_reversal() {
    use TransformMappingRangeField::*;
    let mut doc = fixture();
    for (field, value, expected, scale) in [
        (SourceMax, 100., [0., 100., 0., 1.], 0.01),
        (SourceMin, 10., [10., 100., 0., 1.], 1. / 90.),
        (TargetMin, -30., [10., 100., -30., 1.], 31. / 90.),
        (TargetMax, 150., [10., 100., -30., 150.], 2.),
        (SourceMax, 10., [10., 110., -30., 150.], 1.8),
        (SourceMax, -30., [10., -30., -30., 150.], -4.5),
        (TargetMax, -30., [10., -30., -30., 70.], -2.5),
        (SourceMin, -30., [-30., 70., -30., 70.], 1.),
        (TargetMin, 70., [-30., 70., 70., 170.], 1.),
    ] {
        let old = doc.clone();
        doc = edit(&doc, field, value);
        assert_range(&doc, expected);
        near(f64::from(constraint(&doc).mappings[0].to[0].scale), scale);
        assert_eq!(doc.bones, old.bones);
        assert_eq!(doc.animations, old.animations);
        assert_eq!(constraint(&doc).mix, constraint(&old).mix);
        assert_eq!(constraint(&doc).offsets, constraint(&old).offsets);
    }
}

#[test]
fn transform_mapping_raw_queries_and_exact_field_noops_preserve_bits_and_redo() {
    use TransformMappingRangeField::*;
    let mut doc = fixture();
    let c = constraint_mut(&mut doc);
    c.mappings[0].offset = -0.;
    c.mappings[0].to[0].offset = -0.;
    c.mappings[0].to[0].max = 0.12345679;
    c.mappings[0].to[0].scale = -0.33333334;
    let bytes = serde_json::to_string(&doc).unwrap();
    let mut session = Session::new(doc.clone()).unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo".into(),
        })
        .unwrap();
    session.undo();
    let queried = range(&doc);
    assert!(queried.source_max.unwrap() < 0.);
    for field in [SourceMin, SourceMax, TargetMin, TargetMax] {
        session
            .dispatch(command(field, queried.value(field).unwrap()))
            .unwrap();
        assert!(session.can_redo());
        assert!(!session.can_undo());
        assert_eq!(serde_json::to_string(session.document()).unwrap(), bytes);
    }
    let serialized = serde_json::to_value(queried).unwrap();
    assert!(serialized.get("sourceMin").is_some());
    assert!(serialized.get("sourceMax").is_some());
    assert!(serialized.get("source_min").is_none());
    assert_eq!(
        constraint(session.document()).mappings[0].offset.to_bits(),
        (-0_f32).to_bits()
    );
}

#[test]
fn transform_mapping_zero_scale_stays_explicit_and_source_max_can_resolve_it() {
    use TransformMappingRangeField::*;
    let mut doc = fixture();
    constraint_mut(&mut doc).mappings[0].to[0].scale = 0.;
    assert_eq!(range(&doc).source_max, None);
    assert_eq!(
        serde_json::to_value(range(&doc)).unwrap()["sourceMax"],
        serde_json::Value::Null
    );
    assert_eq!(edit(&doc, SourceMin, 0.), doc);
    for (field, value) in [(SourceMin, 10.), (TargetMin, -30.), (TargetMax, 150.)] {
        assert!(
            apply_command(&doc, command(field, value))
                .unwrap_err()
                .contains("undefined")
        );
    }
    let resolved = edit(&doc, SourceMax, 100.);
    assert_range(&resolved, [0., 100., 0., 1.]);
    constraint_mut(&mut doc).mappings[0].to[0].max = 0.;
    assert!(
        apply_command(&doc, command(SourceMax, 100.))
            .unwrap_err()
            .contains("zero span")
    );
    assert_eq!(
        constraint(&doc).mappings[0].to[0].scale.to_bits(),
        0_f32.to_bits()
    );
}

#[test]
fn transform_mapping_candidates_keep_repaired_partner_and_preview_one_pending_history_step() {
    use TransformMappingRangeField::*;
    let doc = edit(&edit(&fixture(), SourceMax, 100.), SourceMin, 10.);
    let mut session = Session::new(doc.clone()).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.25,
    };
    let mut transform = session.authored_pose(target).unwrap().bone_locals()[1];
    transform.x = 47.25;
    session
        .edit_pose(1, target, transform, &[Property::X], false)
        .unwrap();
    let pending = session.pose_edits().clone();
    let first = edited_transform_mapping_range(
        constraint(&doc),
        TransformProperty::X,
        TransformProperty::Rotate,
        SourceMax,
        10.,
    )
    .unwrap();
    let second = edited_transform_mapping_range(
        &first,
        TransformProperty::X,
        TransformProperty::Rotate,
        SourceMin,
        11.,
    )
    .unwrap();
    session.begin_transaction();
    for candidate in [first, second] {
        session
            .preview_transaction(Command::SetTransformConstraint {
                constraint: 0,
                value: candidate,
            })
            .unwrap();
        assert_eq!(session.pose_edits(), &pending);
    }
    assert_range(session.document(), [11., 110., 0., 1.]);
    let edited = session.document().clone();
    session.commit_transaction();
    session.undo();
    assert_eq!(session.document(), &doc);
    assert_eq!(session.pose_edits(), &pending);
    session.redo();
    assert_eq!(session.document(), &edited);
    assert_eq!(session.pose_edits(), &pending);
}

#[test]
fn transform_mapping_invalid_and_unrepresentable_requests_are_atomic() {
    use TransformMappingRangeField::*;
    let doc = fixture();
    let mut session = Session::new(doc.clone()).unwrap();
    session.dispatch(command(SourceMax, 100.)).unwrap();
    session.undo();
    let mut requests = vec![
        command(SourceMin, f64::NAN),
        command(TargetMax, f64::INFINITY),
        command(TargetMin, f64::NEG_INFINITY),
        command(SourceMax, f64::MAX),
        command(SourceMax, f64::from(f32::from_bits(1))),
        Command::EditTransformMappingRange {
            constraint: 99,
            from: TransformProperty::X,
            to: TransformProperty::Rotate,
            field: SourceMax,
            value: 100.,
        },
        Command::EditTransformMappingRange {
            constraint: 0,
            from: TransformProperty::Y,
            to: TransformProperty::Rotate,
            field: SourceMax,
            value: 100.,
        },
    ];
    let mut collapse = fixture();
    constraint_mut(&mut collapse).mappings[0].to[0].offset = 1e10;
    constraint_mut(&mut collapse).mappings[0].to[0].max = 1e10 + 2048.;
    assert!(
        apply_command(&collapse, command(TargetMax, 1e10 + 1.))
            .unwrap_err()
            .contains("collapses")
    );
    let mut underflow = fixture();
    constraint_mut(&mut underflow).mappings[0].to[0].max = f32::from_bits(1);
    assert!(
        apply_command(&underflow, command(SourceMax, f64::from(f32::MAX)))
            .unwrap_err()
            .contains("nonzero f32")
    );
    for request in requests.drain(..) {
        assert!(
            session
                .dispatch(Command::Batch {
                    commands: vec![
                        Command::RenameAnimation {
                            animation: 0,
                            name: "rollback".into()
                        },
                        request,
                    ]
                })
                .is_err()
        );
        assert_eq!(session.document(), &doc);
        assert!(!session.can_undo());
        assert!(session.can_redo());
        assert!(!session.is_dirty());
    }
}

#[test]
fn transform_mapping_edits_preserve_siblings_and_strict_wire_identity() {
    use TransformMappingRangeField::*;
    let mut doc = fixture();
    let c = constraint_mut(&mut doc);
    c.offsets = TransformValues {
        rotate: 75.,
        y: 3.,
        ..Default::default()
    };
    c.mix.y = -0.4;
    c.mappings.push(TransformMapping {
        from: TransformProperty::Y,
        offset: 7.,
        to: vec![TransformTarget {
            property: TransformProperty::Y,
            offset: -4.,
            max: 35.,
            scale: -0.7,
        }],
    });
    let changed = edit(&doc, SourceMax, 100.);
    assert_eq!(
        constraint(&changed).mappings[1],
        constraint(&doc).mappings[1]
    );
    assert_eq!(constraint(&changed).offsets, constraint(&doc).offsets);
    assert_eq!(constraint(&changed).mix, constraint(&doc).mix);
    assert_eq!(constraint(&changed).bones, constraint(&doc).bones);
    for field in [SourceMin, SourceMax, TargetMin, TargetMax] {
        let payload = serde_json::to_value(command(field, 10.25)).unwrap();
        let decoded: Command = serde_json::from_value(payload.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), payload);
        let mut unknown = payload.clone();
        unknown["field"] = serde_json::json!("sourceMinimum");
        assert!(serde_json::from_value::<Command>(unknown).is_err());
        let mut extra = payload;
        extra["range"] = serde_json::json!([0, 100, 0, 1]);
        assert!(serde_json::from_value::<Command>(extra).is_err());
    }
    constraint_mut(&mut doc).mappings.push(TransformMapping {
        from: TransformProperty::Rotate,
        offset: 0.,
        to: vec![TransformTarget::identity(TransformProperty::Rotate)],
    });
    assert!(
        transform_mapping_range(&doc, 0, TransformProperty::X, TransformProperty::Rotate)
            .unwrap_err()
            .contains("exactly one writer")
    );
    assert!(apply_command(&doc, command(SourceMax, 100.)).is_err());
}
