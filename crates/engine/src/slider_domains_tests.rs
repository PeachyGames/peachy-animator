//! Goldens are retained original public-API observations, not formulas copied
//! from the implementation. These tests exercise the pure live-state boundary.
use super::*;
use crate::{SliderPose, sample_authored_pose};
use peachy_document::*;
use std::collections::BTreeMap;

fn key(time: f32, value: f32) -> Keyframe {
    Keyframe {
        time,
        value,
        curve: Curve::Linear,
    }
}
fn target(slot: &str, placeholder: &str) -> AttachmentRef {
    AttachmentRef {
        skin: "default".into(),
        slot: slot.into(),
        placeholder: placeholder.into(),
    }
}
fn region() -> Attachment {
    Attachment::Region(Region {
        name: None,
        path: "blue_cutout".into(),
        width: 128.,
        height: 128.,
        transform: RegionTransform::default(),
        color: WHITE,
        sequence: None,
    })
}
fn mesh(weighted: bool) -> Attachment {
    let positions = vec![[-10., -10.], [10., -10.], [10., 10.], [-10., 10.]];
    let vertices = if weighted {
        MeshVertices::Weighted {
            influences: positions
                .iter()
                .map(|&position| {
                    vec![
                        BoneInfluence {
                            bone: 1,
                            position,
                            weight: 0.25,
                        },
                        BoneInfluence {
                            bone: 2,
                            position: [position[0] + 3., position[1] - 2.],
                            weight: 0.75,
                        },
                    ]
                })
                .collect(),
        }
    } else {
        MeshVertices::SlotLocal { positions }
    };
    Attachment::Mesh(Mesh {
        name: None,
        path: "blue_cutout".into(),
        vertices,
        uvs: vec![[0., 1.], [1., 1.], [1., 0.], [0., 0.]],
        triangles: vec![0, 1, 2, 2, 3, 0],
        hull: 4,
        width: 128.,
        height: 128.,
        edges: vec![],
        color: WHITE,
        link: None,
        sequence: None,
    })
}
fn document() -> Document {
    let mut doc = Document::new("Original Slider domain lab".into(), 60).unwrap();
    for name in ["driver", "follower"] {
        doc.bones.push(Bone {
            name: name.into(),
            parent: Some(0),
            length: 20.,
            setup: Transform::default(),
            inherit: Inherit::Normal,
        });
    }
    for (name, bone, attachment) in [
        ("driver_marker", 1, "original"),
        ("follower_bar", 2, "bar"),
        ("plain", 1, "quad"),
        ("weighted", 1, "limb"),
    ] {
        doc.slots.push(Slot {
            name: name.into(),
            bone,
            attachment: Some(attachment.into()),
            color: WHITE,
            blend: BlendMode::Normal,
        });
    }
    doc.skins.insert(
        "default".into(),
        BTreeMap::from([
            (
                "driver_marker".into(),
                BTreeMap::from([
                    ("original".into(), region()),
                    ("alternate".into(), region()),
                ]),
            ),
            (
                "follower_bar".into(),
                BTreeMap::from([("bar".into(), region())]),
            ),
            (
                "plain".into(),
                BTreeMap::from([("quad".into(), mesh(false))]),
            ),
            (
                "weighted".into(),
                BTreeMap::from([("limb".into(), mesh(true))]),
            ),
        ]),
    );
    doc.constraints
        .push(Constraint::Slider(SliderConstraint::new("calibration", 0)));
    doc.constraints.push(Constraint::Ik(IkConstraint {
        name: "aim".into(),
        bones: vec![2],
        target: 1,
        setup: IkSetup {
            mix: 0.25,
            bend_positive: true,
        },
        stretch: false,
    }));
    doc.constraints.push(Constraint::Path(PathConstraint {
        name: "ride".into(),
        bones: vec![2],
        slot: 0,
        position_mode: PositionMode::Percent,
        spacing_mode: SpacingMode::Fixed,
        rotate_mode: RotateMode::Tangent,
        offset_rotation: 0.,
        setup: PathSetup {
            position: 0.15,
            spacing: 8.,
            ..PathSetup::default()
        },
    }));
    let mut second = SliderConstraint::new("second", 1);
    second.setup = SliderSetup {
        time: 0.125,
        mix: 0.25,
    };
    doc.constraints.push(Constraint::Slider(second));
    doc.animations = vec![
        Animation::new("reference", vec![]),
        Animation::new("host", vec![]),
    ];
    doc.validate().unwrap();
    doc
}
fn applied_state(doc: &Document) -> AppliedState {
    let authored = sample_authored_pose(doc, Some(1), 0.).unwrap();
    AppliedState {
        locals: authored.locals,
        inherits: authored.inherits,
        ik: authored.ik,
        sliders: authored.sliders,
        paths: authored.paths,
        active: authored.active,
        deforms: authored.deforms,
        slot_attachments: authored.slot_attachments,
        slot_colors: authored.slot_colors,
        draw_order: authored.draw_order,
        sequence_frames: authored.sequence_frames,
        skin: authored.skin,
    }
}
fn deforms(doc: &mut Document) {
    for (slot, placeholder, a, b, incoming) in [
        (
            "plain",
            "quad",
            (1..=8).map(|v| v as f32).collect::<Vec<_>>(),
            vec![-2., -1., 0., 1., 2., 3., 4., 5.],
            vec![-3., 6., -9., 12., -15., 18., -21., 24.],
        ),
        (
            "weighted",
            "limb",
            (1..=16).map(|v| v as f32).collect(),
            (0..16).map(|v| 4. - v as f32).collect(),
            (0..16)
                .map(|v| {
                    if v % 2 == 0 {
                        v as f32 + 2.
                    } else {
                        -(v as f32 + 2.)
                    }
                })
                .collect(),
        ),
    ] {
        doc.animations[0].deform_tracks.push(DeformTrack {
            attachment: target(slot, placeholder),
            keys: vec![
                DeformKey {
                    time: 0.25,
                    curve: Curve::Linear,
                    offsets: a,
                },
                DeformKey {
                    time: 1.,
                    curve: Curve::Linear,
                    offsets: b,
                },
            ],
        });
        doc.animations[1].deform_tracks.push(DeformTrack {
            attachment: target(slot, placeholder),
            keys: vec![DeformKey {
                time: 0.,
                curve: Curve::Linear,
                offsets: incoming,
            }],
        });
    }
}
fn close(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (&actual, &expected) in actual.iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 0.000002,
            "{actual} != {expected}"
        );
    }
}

fn sequences(doc: &mut Document, mode: SequenceMode) {
    let Attachment::Region(region) = doc
        .skins
        .get_mut("default")
        .unwrap()
        .get_mut("driver_marker")
        .unwrap()
        .get_mut("original")
        .unwrap()
    else {
        unreachable!()
    };
    region.path = "blue_sequence".into();
    region.sequence = Some(SequenceDef {
        count: 3,
        start: 1,
        digits: 2,
        setup_index: 1,
    });
    let attachment = target("driver_marker", "original");
    doc.animations[0].sequence_tracks = vec![SequenceTrack {
        attachment: attachment.clone(),
        keys: vec![SequenceKey {
            time: 0.25,
            mode,
            index: 0,
            delay: 0.25,
        }],
    }];
    doc.animations[1].sequence_tracks = vec![SequenceTrack {
        attachment,
        keys: vec![SequenceKey {
            time: 0.,
            mode: SequenceMode::Hold,
            index: 2,
            delay: 0.25,
        }],
    }];
}

fn sequence_frame(state: &AppliedState) -> Option<u32> {
    state
        .sequence_frames
        .iter()
        .find(|(key, _)| *key == target("driver_marker", "original"))
        .map(|(_, frame)| *frame)
}

#[test]
fn slider_domains_sequences_match_all_observed_modes_and_preserve_real_host_or_setup_before_first()
{
    // sequence-04 original aliases: count3/start1/digits2/setup1, hostindex2;
    // query times .25/.5/.75/1.25 after a reference key at .25 with delay .25.
    for (mode, expected) in [
        (SequenceMode::Hold, [0, 0, 0, 0]),
        (SequenceMode::Once, [0, 1, 2, 2]),
        (SequenceMode::Loop, [0, 1, 2, 1]),
        (SequenceMode::Pingpong, [0, 1, 2, 0]),
        (SequenceMode::OnceReverse, [2, 1, 0, 0]),
        (SequenceMode::LoopReverse, [2, 1, 0, 1]),
        (SequenceMode::PingpongReverse, [2, 1, 0, 2]),
    ] {
        let mut doc = document();
        sequences(&mut doc, mode);
        doc.validate().unwrap();
        for (time, expected) in [0.25, 0.5, 0.75, 1.25].into_iter().zip(expected) {
            let mut state = applied_state(&doc);
            apply(&doc, &doc.animations[0], time, 1., false, &mut state).unwrap();
            assert_eq!(sequence_frame(&state), Some(expected), "{mode:?} t={time}");
        }
        for additive in [false, true] {
            for mix in [-0.5, 0., 0.001, 0.5, 1.5] {
                let mut state = applied_state(&doc);
                apply(&doc, &doc.animations[0], 0.5, mix, additive, &mut state).unwrap();
                assert_eq!(
                    sequence_frame(&state),
                    Some(if mix == 0. {
                        2
                    } else if mode == SequenceMode::Hold {
                        0
                    } else {
                        1
                    })
                );
            }
        }
        let mut state = applied_state(&doc);
        apply(&doc, &doc.animations[0], 0.1, 1., false, &mut state).unwrap();
        assert_eq!(sequence_frame(&state), Some(2));
        doc.animations[1].sequence_tracks.clear();
        let mut state = applied_state(&doc);
        apply(&doc, &doc.animations[0], 0.1, 1., false, &mut state).unwrap();
        assert_eq!(sequence_frame(&state), None);
        let sequence = doc
            .attachment(&target("driver_marker", "original"))
            .unwrap()
            .sequence()
            .unwrap();
        assert_eq!(
            sequence.frame_path("blue_sequence", sequence.setup_index),
            "blue_sequence02"
        );
    }
}

#[test]
fn slider_domains_sequence_key_selection_keeps_wide_time_and_matches_observed_early_tick_boundary()
{
    let mut doc = document();
    sequences(&mut doc, SequenceMode::Loop);
    let mut state = applied_state(&doc);
    apply(&doc, &doc.animations[0], 0.499999, 1., false, &mut state).unwrap();
    assert_eq!(sequence_frame(&state), Some(1));
    let right = f32::from_bits(1_f32.to_bits() + 1);
    doc.animations[0].sequence_tracks[0].keys = vec![
        SequenceKey {
            time: 1.,
            mode: SequenceMode::Hold,
            index: 0,
            delay: 0.25,
        },
        SequenceKey {
            time: right,
            mode: SequenceMode::Hold,
            index: 2,
            delay: 0.25,
        },
    ];
    doc.validate().unwrap();
    let query = 1. + (f64::from(right) - 1.) * 0.75;
    assert_eq!(query as f32, right);
    let mut state = applied_state(&doc);
    apply(&doc, &doc.animations[0], query, 1., false, &mut state).unwrap();
    assert_eq!(sequence_frame(&state), Some(0));
}

#[test]
fn slider_domains_skip_inactive_targets_and_keep_global_order_including_inactive_slots() {
    let mut doc = document();
    deforms(&mut doc);
    sequences(&mut doc, SequenceMode::Loop);
    doc.skin_scope.required_bones.insert(1);
    doc.skin_scope.required_constraints.extend([1, 2, 3]);
    doc.animations[0].attachment_tracks.push(AttachmentTrack {
        slot: 0,
        keys: vec![AttachmentKey {
            time: 0.,
            attachment: None,
        }],
    });
    doc.animations[0].color_tracks.push(ColorTrack {
        slot: 0,
        keys: vec![ColorKey {
            time: 0.,
            color: [0, 64, 128, 255],
            curves: [Curve::Linear; 4],
        }],
    });
    doc.animations[0].inherit_tracks.push(InheritTrack {
        bone: 1,
        keys: vec![InheritKey {
            time: 0.,
            inherit: Inherit::NoRotationOrReflection,
        }],
    });
    doc.animations[0].ik_tracks.push(IkTrack {
        constraint: 1,
        keys: vec![IkKeyframe {
            time: 0.,
            mix: 0.5,
            bend_positive: false,
            curve: IkCurve::Linear,
        }],
    });
    doc.animations[0].path_tracks = vec![
        PathTrack {
            constraint: 2,
            property: PathProperty::Position,
            keys: vec![key(0., 0.5)],
        },
        PathTrack {
            constraint: 2,
            property: PathProperty::Spacing,
            keys: vec![key(0., 17.5)],
        },
    ];
    doc.animations[0].slider_tracks = vec![
        SliderTrack {
            constraint: 3,
            property: SliderProperty::Time,
            keys: vec![key(0., 0.5)],
        },
        SliderTrack {
            constraint: 3,
            property: SliderProperty::Mix,
            keys: vec![key(0., 0.75)],
        },
    ];
    doc.animations[1].draw_order = vec![DrawOrderKey {
        time: 0.,
        offsets: vec![DrawOrderOffset { slot: 1, offset: 2 }],
    }];
    doc.animations[0].draw_order = vec![DrawOrderKey {
        time: 0.,
        offsets: vec![DrawOrderOffset { slot: 0, offset: 3 }],
    }];
    doc.validate().unwrap();
    for additive in [false, true] {
        for mix in [-0.5, 0., 0.5, 1.] {
            let mut state = applied_state(&doc);
            assert!(!state.active.bones[1]);
            assert!(state.active.constraints[0]);
            assert!(state.active.constraints[1..].iter().all(|active| !active));
            state.ik[1].mix = 0.6;
            state.paths[2] = crate::PathPose {
                position: -0.2,
                spacing: 13.,
            };
            let before = (
                state.locals.clone(),
                state.inherits.clone(),
                state.slot_attachments.clone(),
                state.slot_colors.clone(),
                state.deforms.clone(),
                state.sequence_frames.clone(),
                state.ik.clone(),
                state.paths.clone(),
                state.sliders.clone(),
            );
            assert!(
                apply(&doc, &doc.animations[0], 0.75, mix, additive, &mut state)
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(
                (
                    state.locals,
                    state.inherits,
                    state.slot_attachments,
                    state.slot_colors,
                    state.deforms,
                    state.sequence_frames,
                    state.ik,
                    state.paths,
                    state.sliders
                ),
                before
            );
            assert_eq!(
                state.draw_order,
                if mix == 0. {
                    vec![0, 2, 3, 1]
                } else {
                    vec![1, 2, 3, 0]
                }
            );
        }
    }
}

#[test]
fn slider_domains_checked_sequence_phase_cannot_commit_prior_staged_writes_on_failure() {
    let mut doc = document();
    sequences(&mut doc, SequenceMode::Loop);
    doc.animations[0].sequence_tracks[0].keys[0].delay = f32::from_bits(1);
    doc.animations[0].attachment_tracks.push(AttachmentTrack {
        slot: 1,
        keys: vec![AttachmentKey {
            time: 0.,
            attachment: None,
        }],
    });
    doc.validate().unwrap();
    let mut state = applied_state(&doc);
    let before = (
        state.slot_attachments.clone(),
        state.sequence_frames.clone(),
    );
    assert!(apply(&doc, &doc.animations[0], f64::MAX, 1., false, &mut state).is_err());
    assert_eq!((state.slot_attachments, state.sequence_frames), before);
}

#[test]
fn slider_domains_nonempty_plain_and_weighted_deform_match_observed_signed_and_additive_goldens() {
    let mut doc = document();
    deforms(&mut doc);
    doc.validate().unwrap();
    for (additive, mix, plain, weighted) in [
        (
            false,
            -0.5,
            [-4., 9., -14., 17.],
            [1.5, -5.8333335, 4.8333335, -8.5],
        ),
        (
            false,
            0.5,
            [-2., 3., -4., 7.],
            [2.5, -0.16666667, 3.1666667, -1.5],
        ),
        (false, 1.5, [0., -3., 6., -3.], [3.5, 5.5, 1.5, 5.5]),
        (
            true,
            -0.5,
            [-2.5, 6., -9.5, 11.],
            [0.5, -4.3333335, 2.8333333, -6.],
        ),
        (
            true,
            0.5,
            [-3.5, 6., -8.5, 13.],
            [3.5, -1.6666666, 5.1666665, -4.],
        ),
        (true, 1.5, [-4.5, 6., -7.5, 15.], [6.5, 1., 7.5, -2.]),
    ] {
        let mut state = applied_state(&doc);
        assert!(
            apply(&doc, &doc.animations[0], 0.75, mix, additive, &mut state)
                .unwrap()
                .is_empty()
        );
        close(
            &state
                .deforms
                .iter()
                .find(|(key, _)| key.slot == "plain")
                .unwrap()
                .1[..4],
            &plain,
        );
        close(
            &state
                .deforms
                .iter()
                .find(|(key, _)| key.slot == "weighted")
                .unwrap()
                .1[..4],
            &weighted,
        );
    }
}

#[test]
fn slider_domains_before_first_and_zero_mix_preserve_host_values_and_discrete_keys_have_no_half_mix_threshold()
 {
    let mut doc = document();
    deforms(&mut doc);
    doc.animations[1].attachment_tracks.push(AttachmentTrack {
        slot: 0,
        keys: vec![AttachmentKey {
            time: 0.,
            attachment: Some("alternate".into()),
        }],
    });
    doc.animations[1].draw_order = vec![DrawOrderKey {
        time: 0.,
        offsets: vec![DrawOrderOffset { slot: 1, offset: 2 }],
    }];
    doc.animations[1].inherit_tracks.push(InheritTrack {
        bone: 1,
        keys: vec![InheritKey {
            time: 0.,
            inherit: Inherit::NoScale,
        }],
    });
    doc.animations[0].attachment_tracks.push(AttachmentTrack {
        slot: 0,
        keys: vec![AttachmentKey {
            time: 0.5,
            attachment: None,
        }],
    });
    doc.animations[0].draw_order = vec![DrawOrderKey {
        time: 0.25,
        offsets: vec![DrawOrderOffset { slot: 0, offset: 3 }],
    }];
    doc.animations[0].inherit_tracks.push(InheritTrack {
        bone: 1,
        keys: vec![InheritKey {
            time: 0.25,
            inherit: Inherit::NoRotationOrReflection,
        }],
    });
    doc.validate().unwrap();
    for (time, mix) in [(0.1, 1.), (0.75, 0.)] {
        let mut state = applied_state(&doc);
        let incoming = state.deforms.clone();
        assert!(
            apply(&doc, &doc.animations[0], time, mix, false, &mut state)
                .unwrap()
                .is_empty()
        );
        assert_eq!(state.deforms, incoming);
        assert_eq!(state.slot_attachments[0].as_deref(), Some("alternate"));
        assert_eq!(state.draw_order, [0, 2, 3, 1]);
        assert_eq!(state.inherits[1], Inherit::NoScale);
    }
    for mix in [-0.0000001, 0.0000001] {
        let mut state = applied_state(&doc);
        assert_eq!(
            apply(&doc, &doc.animations[0], 0.75, mix, true, &mut state).unwrap(),
            [1]
        );
        assert_eq!(state.slot_attachments[0], None);
        assert_eq!(state.draw_order, [1, 2, 3, 0]);
        assert_eq!(state.inherits[1], Inherit::NoRotationOrReflection);
    }
    let mut state = applied_state(&doc);
    apply(
        &doc,
        &doc.animations[0],
        0.25_f64 - 0.000000001,
        1.,
        false,
        &mut state,
    )
    .unwrap();
    assert_eq!(state.inherits[1], Inherit::NoScale);
}

#[test]
fn slider_domains_ik_spacing_and_position_match_distinct_incoming_observations() {
    let mut doc = document();
    doc.animations[0].ik_tracks.push(IkTrack {
        constraint: 1,
        keys: vec![
            IkKeyframe {
                time: 0.,
                mix: 0.,
                bend_positive: false,
                curve: IkCurve::Linear,
            },
            IkKeyframe {
                time: 1.,
                mix: 1.,
                bend_positive: true,
                curve: IkCurve::Linear,
            },
        ],
    });
    doc.animations[0].path_tracks = vec![
        PathTrack {
            constraint: 2,
            property: PathProperty::Position,
            keys: vec![key(0., 0.25), key(1., 0.75)],
        },
        PathTrack {
            constraint: 2,
            property: PathProperty::Spacing,
            keys: vec![key(0., 5.), key(1., 30.)],
        },
    ];
    doc.validate().unwrap();
    for (additive, mix, ik, position, spacing) in [
        (false, -0.5, 0.65, -0.55, 10.75),
        (false, 0.5, 0.55, 0.15, 15.25),
        (false, 1.5, 0.45, 0.85, 19.75),
        (true, -0.5, 0.65, -0.45, 10.75),
        (true, 0.5, 0.55, 0.05, 15.25),
        (true, 1.5, 0.45, 0.55, 19.75),
    ] {
        let mut state = applied_state(&doc);
        state.ik[1].mix = 0.6;
        state.paths[2] = crate::PathPose {
            position: -0.2,
            spacing: 13.,
        };
        apply(&doc, &doc.animations[0], 0.5, mix, additive, &mut state).unwrap();
        assert!((state.ik[1].mix - ik).abs() < 0.0000001);
        assert!(!state.ik[1].bend_positive);
        close(
            &[state.paths[2].position, state.paths[2].spacing],
            &[position, spacing],
        );
    }
}

#[test]
fn slider_domains_other_slider_time_and_mix_add_absolute_samples_without_recursive_application() {
    let mut doc = document();
    doc.animations[0].slider_tracks = vec![
        SliderTrack {
            constraint: 3,
            property: SliderProperty::Time,
            keys: vec![key(0., 0.25), key(1., 0.75)],
        },
        SliderTrack {
            constraint: 3,
            property: SliderProperty::Mix,
            keys: vec![key(0., 0.5), key(1., 1.)],
        },
    ];
    doc.animations[1].tracks.push(Track {
        bone: 2,
        property: Property::X,
        keys: vec![key(0., 99.)],
    });
    doc.validate().unwrap();
    for (additive, mix, expected_time, expected_mix) in [
        (false, -0.5, -0.0625, 0.),
        (false, 0.5, 0.3125, 0.5),
        (false, 1.5, 0.6875, 1.),
        (true, -0.5, -0.125, -0.125),
        (true, 0.5, 0.375, 0.625),
        (true, 1.5, 0.875, 1.375),
    ] {
        let mut state = applied_state(&doc);
        let before = state.locals.clone();
        apply(&doc, &doc.animations[0], 0.5, mix, additive, &mut state).unwrap();
        assert_eq!(
            state.sliders[3],
            SliderPose {
                time: expected_time,
                mix: expected_mix
            }
        );
        assert_eq!(state.locals, before);
    }
}

#[test]
fn slider_domains_color_ignores_additive_and_clamps_both_ends_without_byte_quantization() {
    let mut doc = document();
    doc.animations[1].color_tracks.push(ColorTrack {
        slot: 0,
        keys: vec![ColorKey {
            time: 0.,
            color: [64, 128, 192, 224],
            curves: [Curve::Linear; 4],
        }],
    });
    doc.animations[0].color_tracks.push(ColorTrack {
        slot: 0,
        keys: vec![
            ColorKey {
                time: 0.25,
                color: [32, 0, 192, 64],
                curves: [Curve::Linear; 4],
            },
            ColorKey {
                time: 1.,
                color: [192, 224, 64, 160],
                curves: [Curve::Linear; 4],
            },
        ],
    });
    doc.validate().unwrap();
    for additive in [false, true] {
        let mut state = applied_state(&doc);
        apply(&doc, &doc.animations[0], 0.75, 0.5, additive, &mut state).unwrap();
        close(
            &state.slot_colors[0],
            &[0.39738563, 0.5437909, 0.58562094, 0.6901961],
        );
        for (mix, expected) in [
            (-3., [0., 0.2509805, 1., 1.]),
            (3., [1., 0.75294113, 0., 0.]),
        ] {
            let mut state = applied_state(&doc);
            apply(&doc, &doc.animations[0], 0.75, mix, additive, &mut state).unwrap();
            close(&state.slot_colors[0], &expected);
        }
    }
}

#[test]
fn slider_domains_stage_all_writes_and_refuse_uncalibrated_data_even_at_zero_mix() {
    let mut doc = document();
    doc.animations[0].attachment_tracks.push(AttachmentTrack {
        slot: 0,
        keys: vec![AttachmentKey {
            time: 0.,
            attachment: None,
        }],
    });
    doc.animations[0].path_tracks.push(PathTrack {
        constraint: 2,
        property: PathProperty::Spacing,
        keys: vec![key(0., f32::MAX)],
    });
    doc.validate().unwrap();
    let mut state = applied_state(&doc);
    let before = state.slot_attachments.clone();
    assert!(
        apply(&doc, &doc.animations[0], 0., 2., false, &mut state)
            .unwrap_err()
            .contains("finite native")
    );
    assert_eq!(state.slot_attachments, before);
    doc.animations[0].path_tracks.clear();
    doc.animations[0].physics_resets.push(PhysicsReset {
        constraint: None,
        times: vec![0.],
    });
    doc.validate().unwrap();
    assert!(
        apply(&doc, &doc.animations[0], 0., 0., false, &mut state)
            .unwrap_err()
            .contains("physics reset")
    );
    assert_eq!(state.slot_attachments, before);
}
