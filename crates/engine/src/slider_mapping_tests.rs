use super::*;
use crate::{Command, Session};
use peachy_document::{
    Animation, Curve, Keyframe, Property, SliderConstraint, Track, TransformProperty,
};

fn fixture() -> Document {
    let mut doc = Document::new("Original Slider mapping query".into(), 60).unwrap();
    doc.animations.push(Animation::new(
        "reference",
        vec![Track {
            bone: 0,
            property: Property::X,
            keys: vec![Keyframe {
                time: 1.5,
                value: 0.,
                curve: Curve::Linear,
            }],
        }],
    ));
    let mut slider = SliderConstraint::new("slider", 0);
    slider.bone = Some(0);
    slider.property = Some(TransformProperty::Rotate);
    // Installed 4.3.26: source Min10, Frame Min20/Max90 at60FPS.
    slider.from = 10.;
    slider.to = 0.3333333;
    slider.max = 1.5;
    slider.scale = 0.012962963;
    slider.setup.time = 0.75;
    doc.constraints.push(Constraint::Slider(slider));
    doc.validate().unwrap();
    doc
}

fn slider(doc: &mut Document) -> &mut SliderConstraint {
    let Constraint::Slider(value) = &mut doc.constraints[0] else {
        unreachable!()
    };
    value
}

fn edit_command(field: SliderMappingRangeField, value: f64) -> Command {
    Command::EditSliderMappingRange {
        constraint: 0,
        field,
        value,
    }
}

#[test]
fn slider_mapping_query_retains_native_precision_seconds_raw_data_and_history() {
    let doc = fixture();
    let bytes = serde_json::to_vec(&doc).unwrap();
    let mut session = Session::new(doc).unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo witness".into(),
        })
        .unwrap();
    session.undo();
    let range = slider_mapping_range(session.document(), 0).unwrap();
    assert_eq!(range.source_min, 10.);
    assert_eq!(range.target_min, f64::from(0.3333333_f32));
    assert_eq!(range.target_max, 1.5);
    let source_max = range.source_max.unwrap();
    assert!((source_max - 100.).abs() < 0.00001);
    assert_ne!(source_max, 100.); // Display rounding must not replace raw readback.
    assert_eq!(serde_json::to_vec(session.document()).unwrap(), bytes);
    assert!(!session.can_undo());
    assert!(session.can_redo());
    assert!(!session.is_dirty());
    assert_eq!(
        session.document().constraints[0]
            .as_slider()
            .unwrap()
            .setup
            .time,
        0.75
    );
    let value = serde_json::to_value(range).unwrap();
    assert_eq!(value.as_object().unwrap().len(), 4);
    assert_eq!(value["sourceMin"], 10.);
    assert!(value.get("sourceMax").is_some());
    assert_eq!(value["targetMax"], 1.5);
    let mut other_fps = session.document().clone();
    other_fps.fps = 30;
    assert_eq!(slider_mapping_range(&other_fps, 0).unwrap(), range);
}

#[test]
fn slider_mapping_query_keeps_zero_scale_undefined_and_reversed_or_wide_ranges_raw() {
    let mut doc = fixture();
    for scale in [0., -0.] {
        slider(&mut doc).scale = scale;
        let range = slider_mapping_range(&doc, 0).unwrap();
        assert_eq!(range.source_max, None);
        assert_eq!(
            serde_json::to_value(range).unwrap()["sourceMax"],
            serde_json::Value::Null
        );
        assert_eq!(slider(&mut doc).scale.to_bits(), scale.to_bits());
    }
    let value = slider(&mut doc);
    value.from = 10.;
    value.to = 2.;
    value.max = -2.;
    value.scale = 0.5;
    let range = slider_mapping_range(&doc, 0).unwrap();
    assert_eq!(range.source_max, Some(2.));
    assert_eq!(range.target_min, 2.);
    assert_eq!(range.target_max, -2.);
    let value = slider(&mut doc);
    value.from = 0.;
    value.to = -f32::MAX;
    value.max = f32::MAX;
    value.scale = f32::MIN_POSITIVE;
    let range = slider_mapping_range(&doc, 0).unwrap();
    assert!(range.source_max.unwrap().is_finite());
    assert!(range.source_max.unwrap() > f64::from(f32::MAX));
}

#[test]
fn slider_mapping_query_rejects_missing_sources_invalid_owners_and_nonfinite_data() {
    let baseline = fixture();
    assert!(slider_mapping_range(&baseline, 1).is_err());
    let mut doc = baseline.clone();
    slider(&mut doc).bone = None;
    assert!(slider_mapping_range(&doc, 0).is_err());
    let mut doc = baseline.clone();
    slider(&mut doc).property = None;
    assert!(slider_mapping_range(&doc, 0).is_err());
    let mut doc = baseline.clone();
    let missing_bone = doc.bones.len();
    slider(&mut doc).bone = Some(missing_bone);
    assert!(slider_mapping_range(&doc, 0).is_err());
    let mut doc = baseline.clone();
    let missing_animation = doc.animations.len();
    slider(&mut doc).animation = missing_animation;
    assert!(slider_mapping_range(&doc, 0).is_err());
    for field in 0..6 {
        for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut doc = baseline.clone();
            let value = slider(&mut doc);
            match field {
                0 => value.from = invalid,
                1 => value.to = invalid,
                2 => value.max = invalid,
                3 => value.scale = invalid,
                4 => value.setup.time = invalid,
                _ => value.setup.mix = invalid,
            }
            assert!(slider_mapping_range(&doc, 0).is_err());
        }
    }
    let mut doc = baseline;
    doc.constraints[0] = Constraint::Ik(peachy_document::IkConstraint {
        name: "different kind".into(),
        bones: vec![0],
        target: 0,
        setup: peachy_document::IkSetup {
            mix: 1.,
            bend_positive: true,
        },
        stretch: false,
    });
    assert!(slider_mapping_range(&doc, 0).is_err());
}

#[test]
fn slider_mapping_edits_match_native_equal_repairs_and_reversed_ranges() {
    use SliderMappingRangeField::*;
    let original = fixture();
    let original_range = slider_mapping_range(&original, 0).unwrap();
    // Native4.3.26 original six-bone probe. Target values are seconds; the
    // observed Frame20 equality uses its encoded stored partner on import.
    for (field, value, expected, expected_scale) in [
        (SourceMin, 100., [100., 200., 20. / 60., 1.5], 0.011666667),
        (SourceMax, 10., [10., 110., 20. / 60., 1.5], 0.011666667),
        (TargetMin, 1.5, [10., 100., 1.5, 3.], 0.01666667),
        (
            TargetMax,
            original_range.target_min,
            [10., 100., 20. / 60., 110. / 60.],
            0.01666667,
        ),
        (
            TargetMax,
            10. / 60.,
            [10., 100., 20. / 60., 10. / 60.],
            -0.0018518518,
        ),
        (SourceMin, 150., [150., 100., 20. / 60., 1.5], -0.023333333),
    ] {
        let value = edited_slider_mapping_range(
            original.constraints[0].as_slider().unwrap(),
            1.5,
            field,
            value,
        )
        .unwrap();
        let mut after = original.clone();
        after.constraints[0] = Constraint::Slider(value.clone());
        let range = slider_mapping_range(&after, 0).unwrap();
        for (actual, expected) in [
            range.source_min,
            range.source_max.unwrap(),
            range.target_min,
            range.target_max,
        ]
        .into_iter()
        .zip(expected)
        {
            assert!(
                (actual - expected).abs() < 0.00002,
                "{actual} != {expected}"
            );
        }
        assert!((f64::from(value.scale) - expected_scale).abs() < 0.00000001);
        let before = original.constraints[0].as_slider().unwrap();
        assert_eq!(value.setup, before.setup);
        assert_eq!(value.name, before.name);
        assert_eq!(value.animation, before.animation);
        assert_eq!(value.bone, before.bone);
        assert_eq!(value.property, before.property);
        assert_eq!(value.local, before.local);
        assert_eq!(value.looped, before.looped);
        assert_eq!(value.additive, before.additive);
    }
}

#[test]
fn slider_mapping_noops_preserve_raw_baseline_and_redo_while_real_edit_is_one_undo() {
    use SliderMappingRangeField::*;
    let mut doc = fixture();
    slider(&mut doc).local = true;
    slider(&mut doc).setup.time = -0.;
    let bytes = serde_json::to_vec(&doc).unwrap();
    let mut session = Session::new(doc).unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo witness".into(),
        })
        .unwrap();
    session.undo();
    let range = slider_mapping_range(session.document(), 0).unwrap();
    for field in [SourceMin, SourceMax, TargetMin, TargetMax] {
        session
            .dispatch(edit_command(field, range.value(field).unwrap()))
            .unwrap();
        assert_eq!(serde_json::to_vec(session.document()).unwrap(), bytes);
        assert!(session.can_redo());
        assert!(!session.can_undo());
    }
    // A derived100.0000009 field already encodes100. Typing100 alone must not
    // normalize its scale or consume a redo entry.
    session.dispatch(edit_command(SourceMax, 100.)).unwrap();
    assert_eq!(serde_json::to_vec(session.document()).unwrap(), bytes);
    assert!(session.can_redo());
    let wire = serde_json::to_value(SourceMin).unwrap();
    assert_eq!(wire, "sourceMin");
    for invalid in ["source_min", "time", "SourceMax"] {
        assert!(
            serde_json::from_value::<SliderMappingRangeField>(serde_json::json!(invalid)).is_err()
        );
    }
    for invalid in [
        serde_json::json!({"sourceMin": null}),
        serde_json::json!(["sourceMin"]),
        serde_json::Value::Null,
        serde_json::json!(0),
        serde_json::json!(true),
    ] {
        assert!(serde_json::from_value::<SliderMappingRangeField>(invalid.clone()).is_err());
        assert!(
            serde_json::from_value::<Command>(serde_json::json!({
                "command": "editSliderMappingRange",
                "constraint": 0,
                "field": invalid,
                "value": 10.
            }))
            .is_err()
        );
    }
    assert!(
        serde_json::from_value::<Command>(serde_json::json!({
            "command": "editSliderMappingRange",
            "constraint": 0,
            "field": "sourceMin",
            "value": 10.
        }))
        .is_ok()
    );
    session.dispatch(edit_command(SourceMin, 100.)).unwrap();
    assert!(session.can_undo());
    assert!(!session.can_redo());
    let changed = serde_json::to_vec(session.document()).unwrap();
    session.undo();
    assert_eq!(serde_json::to_vec(session.document()).unwrap(), bytes);
    session.redo();
    assert_eq!(serde_json::to_vec(session.document()).unwrap(), changed);
}

#[test]
fn slider_mapping_undefined_ranges_require_explicit_recovery_without_empty_duration_guesses() {
    use SliderMappingRangeField::*;
    let mut doc = fixture();
    slider(&mut doc).scale = 0.;
    let before = slider(&mut doc).clone();
    for (field, value) in [(SourceMin, 11.), (TargetMin, 0.), (TargetMax, 2.)] {
        assert!(edited_slider_mapping_range(&before, 1.5, field, value).is_err());
    }
    assert_eq!(
        edited_slider_mapping_range(&before, 1.5, SourceMin, 10.).unwrap(),
        before
    );
    let recovered = edited_slider_mapping_range(&before, 1.5, SourceMax, 100.).unwrap();
    assert_ne!(recovered.scale, 0.);
    let mut flat = before.clone();
    flat.max = flat.to;
    assert!(edited_slider_mapping_range(&flat, 1.5, SourceMax, 100.).is_err());
    let regular = fixture().constraints[0].as_slider().unwrap().clone();
    assert!(edited_slider_mapping_range(&regular, 0., TargetMin, 1.5).is_err());
    assert!(edited_slider_mapping_range(&regular, 0., SourceMin, 100.).is_ok());
    for invalid in [f64::NAN, f64::INFINITY, f64::MAX] {
        assert!(edited_slider_mapping_range(&regular, 1.5, SourceMin, invalid).is_err());
    }
    let mut session = Session::new(fixture()).unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo witness".into(),
        })
        .unwrap();
    session.undo();
    let bytes = serde_json::to_vec(session.document()).unwrap();
    assert!(session.dispatch(edit_command(SourceMin, f64::NAN)).is_err());
    assert_eq!(serde_json::to_vec(session.document()).unwrap(), bytes);
    assert!(session.can_redo());
    assert!(!session.can_undo());
}
