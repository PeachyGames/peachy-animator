use super::*;
use crate::{Animation, Curve, SliderSetup, TransformProperty};

fn document() -> Document {
    let mut doc = Document::new("Slider structure".into(), 60).unwrap();
    doc.animations.push(Animation::new("source", vec![]));
    doc.animations.push(Animation::new("host", vec![]));
    doc.constraints
        .push(Constraint::Slider(SliderConstraint::new("manual", 0)));
    doc
}

#[test]
fn slider_manual_defaults_and_complete_independent_fields_validate() {
    let mut doc = document();
    let slider = doc.constraints[0].as_slider().unwrap();
    assert_eq!(slider.setup, SliderSetup { time: 0., mix: 1. });
    assert_eq!((slider.bone, slider.property), (None, None));
    assert!(!slider.local && !slider.looped && !slider.additive);
    assert_eq!([slider.from, slider.to, slider.scale, slider.max], [0.; 4]);
    let Constraint::Slider(slider) = &mut doc.constraints[0] else {
        unreachable!()
    };
    slider.setup = SliderSetup {
        time: -0.25,
        mix: 1.5,
    };
    slider.from = 10.;
    slider.to = 0.3333333;
    slider.max = -2.;
    slider.scale = -0.012962963;
    slider.property = Some(TransformProperty::Rotate);
    doc.validate().unwrap();
    let slider = doc.constraints[0].as_slider().unwrap();
    assert_eq!(slider.setup.time, -0.25);
    assert_eq!(slider.to.to_bits(), 0.3333333f32.to_bits());
    assert_eq!(slider.from, 10.);
}

#[test]
fn slider_references_property_requirement_and_distinct_tracks_are_structural() {
    let mut doc = document();
    let Constraint::Slider(slider) = &mut doc.constraints[0] else {
        unreachable!()
    };
    slider.bone = Some(0);
    assert!(doc.validate().unwrap_err().contains("needs a property"));
    let Constraint::Slider(slider) = &mut doc.constraints[0] else {
        unreachable!()
    };
    slider.property = Some(TransformProperty::Rotate);
    doc.validate().unwrap();
    doc.animations[1].slider_tracks = SliderProperty::ALL
        .into_iter()
        .map(|property| SliderTrack {
            constraint: 0,
            property,
            keys: vec![Keyframe {
                time: 2.,
                value: -0.5,
                curve: Curve::Linear,
            }],
        })
        .collect();
    doc.validate().unwrap();
    assert_eq!(doc.animations[1].duration(), 2.);
    assert_eq!(
        doc.animations[1].key_times().collect::<Vec<_>>(),
        vec![2., 2.]
    );
    let duplicate = doc.animations[1].slider_tracks[0].clone();
    doc.animations[1].slider_tracks.push(duplicate);
    assert!(doc.validate().unwrap_err().contains("once per property"));
    doc.animations[1].slider_tracks.pop();
    let Constraint::Slider(slider) = &mut doc.constraints[0] else {
        unreachable!()
    };
    slider.animation = 2;
    assert!(doc.validate().unwrap_err().contains("referenced animation"));
}

#[test]
fn slider_raw_graph_does_not_invent_a_ban_on_nested_or_cyclic_tracks() {
    let mut doc = document();
    doc.constraints
        .push(Constraint::Slider(SliderConstraint::new("other", 1)));
    for (animation, constraint) in [(0, 1), (1, 0)] {
        doc.animations[animation].slider_tracks.push(SliderTrack {
            constraint,
            property: SliderProperty::Time,
            keys: vec![Keyframe {
                time: 0.,
                value: 0.5,
                curve: Curve::Linear,
            }],
        });
    }
    doc.validate().unwrap();
    // Storage acceptance is deliberately not a claim about cycle evaluation.
    doc.animations[0].slider_tracks[0].keys[0].value = f32::INFINITY;
    assert!(doc.validate().is_err());
}

#[test]
fn slider_skin_activity_flag_is_distinct_from_source_bone_eligibility() {
    let mut doc = document();
    let Constraint::Slider(slider) = &mut doc.constraints[0] else {
        unreachable!()
    };
    slider.bone = Some(0);
    slider.property = Some(TransformProperty::Rotate);
    doc.skin_scope.required_bones.insert(0);
    doc.validate().unwrap();
    let activity = doc.activity("default");
    assert!(!activity.bones[0]);
    assert!(activity.constraints[0]);
    doc.skin_scope.required_constraints.insert(0);
    assert!(!doc.activity("default").constraints[0]);
    doc.skins.insert("both".into(), crate::Skin::new());
    doc.skin_scope.skins.insert(
        "both".into(),
        crate::SkinMembers {
            bones: vec![0],
            constraints: vec![0],
        },
    );
    doc.validate().unwrap();
    let activity = doc.activity("both");
    assert!(activity.bones[0] && activity.constraints[0]);
}
