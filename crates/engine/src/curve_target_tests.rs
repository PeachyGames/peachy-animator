//! Exact typed Curves selection and raw channel inspection never change source
//! selection, curve data or values. Terminal selection policy is presentation.
use crate::*;
use peachy_document::*;

fn times() -> [f32; 3] {
    [0.25, f32::from_bits(0.25_f32.to_bits() + 1), 0.8]
}
fn scalar(property: Property, time: f32) -> AnimationKey {
    AnimationKey::Bone {
        bone: 0,
        property,
        time,
    }
}
fn source() -> AttachmentRef {
    AttachmentRef {
        skin: "peach".into(),
        slot: "body".into(),
        placeholder: "body".into(),
    }
}
fn linked(placeholder: &str) -> AttachmentRef {
    AttachmentRef {
        skin: "mint".into(),
        slot: "body".into(),
        placeholder: placeholder.into(),
    }
}
fn deform(attachment: AttachmentRef, time: f32) -> AnimationKey {
    AnimationKey::Deform { attachment, time }
}
fn target(key: AnimationKey, next_time: f32) -> CurveTarget {
    CurveTarget { key, next_time }
}
fn raw_ik() -> IkCurve {
    IkCurve::Bezier {
        time: [0.3333333333333333, 0.6666666666666666],
        mix: [0.875, 0.125],
    }
}

fn fixture() -> (Document, usize) {
    let mut doc: Document =
        serde_json::from_str(include_str!("../../../examples/peach-bot/ik-bot.peachy")).unwrap();
    let slot = doc.slots.iter().position(|s| s.name == "body").unwrap();
    let source_mesh = match doc.attachment(&source()).unwrap() {
        Attachment::Mesh(mesh) => mesh.clone(),
        _ => panic!("source mesh"),
    };
    let count = source_mesh.vertices.deform_len();
    for (placeholder, timelines) in [("shared", true), ("independent", false)] {
        let mut mesh = source_mesh.clone();
        mesh.link = Some(MeshLink {
            skin: "peach".into(),
            source: "body".into(),
            timelines,
        });
        doc.skins
            .get_mut("mint")
            .unwrap()
            .entry("body".into())
            .or_default()
            .insert(placeholder.into(), Attachment::Mesh(mesh));
    }
    let t = times();
    let clip = &mut doc.animations[0];
    clip.tracks = [Property::X, Property::Y]
        .into_iter()
        .map(|property| Track {
            bone: 0,
            property,
            keys: t
                .into_iter()
                .enumerate()
                .map(|(i, time)| Keyframe {
                    time,
                    value: i as f32,
                    curve: Curve::BezierOffset([0.2, 1e-7, 0.8, -1e-7]),
                })
                .collect(),
        })
        .collect();
    clip.ik_tracks = vec![IkTrack {
        constraint: 0,
        keys: t
            .into_iter()
            .enumerate()
            .map(|(i, time)| IkKeyframe {
                time,
                mix: if i == 1 { -0. } else { 0.75 },
                bend_positive: i != 1,
                curve: raw_ik(),
            })
            .collect(),
    }];
    clip.deform_tracks = [source(), linked("independent")]
        .into_iter()
        .map(|attachment| DeformTrack {
            attachment,
            keys: t
                .into_iter()
                .map(|time| DeformKey {
                    time,
                    offsets: vec![0.; count],
                    curve: Curve::Bezier([0.25, 0.1, 0.75, 0.9]),
                })
                .collect(),
        })
        .collect();
    clip.color_tracks = vec![ColorTrack {
        slot,
        keys: t
            .into_iter()
            .enumerate()
            .map(|(i, time)| ColorKey {
                time,
                color: [64, 128, 192, [255, 128, 0][i]],
                curves: [
                    Curve::BezierOffset([0.2, 1e-7, 0.8, -1e-7]),
                    Curve::Linear,
                    Curve::Bezier([0.2, 0.1, 0.8, 0.9]),
                    Curve::Linear,
                ],
            })
            .collect(),
    }];
    clip.attachment_tracks = vec![AttachmentTrack {
        slot,
        keys: t
            .into_iter()
            .enumerate()
            .map(|(i, time)| AttachmentKey {
                time,
                attachment: (i != 1).then(|| "body".into()),
            })
            .collect(),
    }];
    doc.validate().unwrap();
    (doc, slot)
}

#[test]
fn curve_targets_single_terminal_redirects_but_multiple_terminals_do_not() {
    let (doc, _) = fixture();
    let before = doc.clone();
    let [first, close, last] = times();
    let selected = vec![scalar(Property::X, last)];
    let original = selected.clone();
    assert_eq!(
        curve_targets(&doc, 0, &selected).unwrap(),
        vec![target(scalar(Property::X, close), last)]
    );
    assert_eq!(selected, original);
    assert!(
        curve_targets(
            &doc,
            0,
            &[scalar(Property::X, last), scalar(Property::Y, last)]
        )
        .unwrap()
        .is_empty()
    );
    assert_eq!(
        curve_targets(
            &doc,
            0,
            &[scalar(Property::X, first), scalar(Property::Y, last)]
        )
        .unwrap(),
        vec![target(scalar(Property::X, first), close)]
    );
    let mut one = doc.clone();
    one.animations[0].tracks[1].keys = vec![one.animations[0].tracks[1].keys[2]];
    assert!(
        curve_targets(&one, 0, &[scalar(Property::Y, last)])
            .unwrap()
            .is_empty()
    );
    assert!(curve_targets(&doc, 0, &[]).unwrap().is_empty());
    assert_eq!(doc, before);
}

#[test]
fn curve_targets_deduplicate_before_counting_and_attachment_keys_still_count() {
    let (doc, slot) = fixture();
    let [first, close, last] = times();
    let selected = scalar(Property::X, last);
    assert_eq!(
        curve_targets(&doc, 0, &[selected.clone(), selected.clone()]).unwrap(),
        vec![target(scalar(Property::X, close), last)]
    );
    let attachment = AnimationKey::SlotAttachment { slot, time: last };
    assert!(
        curve_targets(&doc, 0, &[selected, attachment.clone()])
            .unwrap()
            .is_empty()
    );
    assert!(
        curve_targets(&doc, 0, std::slice::from_ref(&attachment))
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        curve_targets(&doc, 0, &[scalar(Property::X, first), attachment]).unwrap(),
        vec![target(scalar(Property::X, first), close)]
    );
    assert_eq!(
        curve_targets(&doc, 0, &[AnimationKey::SlotColor { slot, time: last }]).unwrap(),
        vec![target(AnimationKey::SlotColor { slot, time: close }, last)]
    );
}

#[test]
fn curve_targets_keep_adjacent_f32_intervals_and_reject_missing_stale_or_invalid_sources() {
    let (doc, slot) = fixture();
    let [first, close, last] = times();
    assert_eq!(
        curve_targets(
            &doc,
            0,
            &[scalar(Property::X, first), scalar(Property::X, close)]
        )
        .unwrap(),
        vec![
            target(scalar(Property::X, first), close),
            target(scalar(Property::X, close), last)
        ]
    );
    for bad in [
        scalar(Property::X, f32::from_bits(close.to_bits() + 1)),
        scalar(Property::X, f32::NAN),
        scalar(Property::X, -0.1),
        AnimationKey::Bone {
            bone: usize::MAX,
            property: Property::X,
            time: first,
        },
        AnimationKey::SlotAttachment { slot, time: 0.4 },
        AnimationKey::SlotColor {
            slot: usize::MAX,
            time: first,
        },
    ] {
        assert!(curve_targets(&doc, 0, &[bad]).is_err());
    }
    let mut changed = doc.clone();
    changed.animations[0].tracks[0].keys.remove(1);
    assert!(curve_targets(&changed, 0, &[scalar(Property::X, close)]).is_err());
    assert!(curve_targets(&doc, usize::MAX, &[]).is_err());
}

#[test]
fn curve_targets_linked_aliases_count_once_while_independent_deforms_keep_their_own_identity() {
    let (doc, _) = fixture();
    let [_, close, last] = times();
    assert_eq!(
        curve_targets(
            &doc,
            0,
            &[deform(linked("shared"), last), deform(source(), last)]
        )
        .unwrap(),
        vec![target(deform(source(), close), last)]
    );
    assert!(
        curve_targets(
            &doc,
            0,
            &[deform(linked("independent"), last), deform(source(), last)]
        )
        .unwrap()
        .is_empty()
    );
    assert_eq!(
        curve_targets(&doc, 0, &[deform(linked("independent"), last)]).unwrap(),
        vec![target(deform(linked("independent"), close), last)]
    );
}

#[test]
fn curve_channels_keep_raw_standard_ik_rgba_and_deform_domains_without_flat_normalization() {
    let (doc, slot) = fixture();
    let [first, close, last] = times();
    let before = doc.clone();
    let bone = curve_channels(&doc, 0, &target(scalar(Property::X, first), close)).unwrap();
    assert_eq!(bone[0].values, [0., 1.]);
    assert_eq!(
        bone[0].curve,
        CurveData::Standard(doc.animations[0].tracks[0].keys[0].curve)
    );
    let ik = curve_channels(
        &doc,
        0,
        &target(
            AnimationKey::Ik {
                constraint: 0,
                time: first,
            },
            close,
        ),
    )
    .unwrap();
    assert_eq!(ik[0].curve, CurveData::Ik(raw_ik()));
    let CurveData::Ik(IkCurve::Bezier { time, .. }) = ik[0].curve else {
        panic!("IK precision")
    };
    assert_eq!(time[0].to_bits(), 0.3333333333333333_f64.to_bits());
    assert_ne!(time[0], f64::from(time[0] as f32));
    assert_eq!(ik[0].values[1].to_bits(), (-0_f64).to_bits());
    let deform = curve_channels(&doc, 0, &target(deform(source(), first), close)).unwrap();
    assert_eq!(deform[0].values, [0., 1.]);
    let rgba = curve_channels(
        &doc,
        0,
        &target(AnimationKey::SlotColor { slot, time: first }, close),
    )
    .unwrap();
    assert_eq!(rgba.len(), 4);
    for (channel, data) in rgba.iter().enumerate() {
        assert_eq!(data.channel, channel);
        assert_eq!(
            data.values,
            [
                f64::from(doc.animations[0].color_tracks[0].keys[0].color[channel]) / 255.,
                f64::from(doc.animations[0].color_tracks[0].keys[1].color[channel]) / 255.
            ]
        );
        assert_eq!(
            data.curve,
            CurveData::Standard(doc.animations[0].color_tracks[0].keys[0].curves[channel])
        );
    }
    assert!(curve_channels(&doc, 0, &target(scalar(Property::X, first), last)).is_err());
    assert!(curve_channels(&doc, 0, &target(scalar(Property::X, last), last)).is_err());
    assert!(
        curve_channels(
            &doc,
            0,
            &target(AnimationKey::SlotAttachment { slot, time: first }, close)
        )
        .is_err()
    );
    let targets = [
        target(scalar(Property::X, first), close),
        target(
            AnimationKey::Ik {
                constraint: 0,
                time: first,
            },
            close,
        ),
        target(
            AnimationKey::Deform {
                attachment: source(),
                time: first,
            },
            close,
        ),
        target(AnimationKey::SlotColor { slot, time: first }, close),
    ];
    let bulk = curve_channels_for_targets(&doc, 0, &targets).unwrap();
    assert_eq!(bulk.len(), 7);
    assert_eq!(
        bulk,
        targets
            .iter()
            .flat_map(|target| curve_channels(&doc, 0, target).unwrap())
            .collect::<Vec<_>>()
    );
    let mut stale = targets.to_vec();
    stale[3].next_time = last;
    assert!(curve_channels_for_targets(&doc, 0, &stale).is_err());
    assert_eq!(doc, before);
}

#[test]
fn ik_curve_only_edits_preserve_f64_controls_mix_bend_and_exact_key_identities() {
    let (doc, _) = fixture();
    let [_, close, last] = times();
    let curve = IkCurve::Bezier {
        time: [0.49999999999999994, 0.7500000000000001],
        mix: [-0.5, 1.5],
    };
    let after = apply_command(
        &doc,
        Command::SetIkCurve {
            animation: 0,
            constraint: 0,
            time: close,
            curve,
        },
    )
    .unwrap();
    let mut expected = doc.clone();
    expected.animations[0].ik_tracks[0].keys[1].curve = curve;
    assert_eq!(after, expected);
    for (old, new) in doc.animations[0].ik_tracks[0]
        .keys
        .iter()
        .zip(&after.animations[0].ik_tracks[0].keys)
    {
        assert_eq!(old.time.to_bits(), new.time.to_bits());
        assert_eq!(old.mix.to_bits(), new.mix.to_bits());
        assert_eq!(old.bend_positive, new.bend_positive);
    }
    let last_edit = apply_command(
        &doc,
        Command::SetIkCurve {
            animation: 0,
            constraint: 0,
            time: last,
            curve: IkCurve::Stepped,
        },
    )
    .unwrap();
    assert_eq!(
        last_edit.animations[0].ik_tracks[0].keys[1],
        doc.animations[0].ik_tracks[0].keys[1]
    );
    assert_eq!(
        last_edit.animations[0].ik_tracks[0].keys[2].curve,
        IkCurve::Stepped
    );
}

#[test]
fn ik_curve_only_failures_and_undo_keep_pending_pose_redo_and_strict_wire_contracts() {
    let (doc, _) = fixture();
    let [first, close, _] = times();
    let mut session = Session::new(doc.clone()).unwrap();
    session
        .edit_ik_pose(
            0,
            EditTarget::Animate {
                animation: 0,
                time: 0.5,
            },
            &[IkPoseEdit::Mix { value: 0.42 }],
            false,
        )
        .unwrap();
    let pending = session.pose_edits().clone();
    session
        .dispatch(Command::SetIkCurve {
            animation: 0,
            constraint: 0,
            time: close,
            curve: IkCurve::Stepped,
        })
        .unwrap();
    session.undo();
    assert_eq!(session.document(), &doc);
    assert_eq!(session.pose_edits(), &pending);
    assert!(session.can_redo());
    for (constraint, time, curve) in [
        (0, f32::from_bits(close.to_bits() + 1), IkCurve::Linear),
        (usize::MAX, first, IkCurve::Linear),
        (
            0,
            first,
            IkCurve::Bezier {
                time: [-0.1, 0.8],
                mix: [0., 1.],
            },
        ),
        (
            0,
            first,
            IkCurve::Bezier {
                time: [0.2, f64::NAN],
                mix: [0., 1.],
            },
        ),
        (
            0,
            first,
            IkCurve::Bezier {
                time: [0.2, 0.8],
                mix: [f32::INFINITY, 1.],
            },
        ),
    ] {
        assert!(
            session
                .dispatch(Command::Batch {
                    commands: vec![
                        Command::SetKeyCurve {
                            animation: 0,
                            bone: 0,
                            property: Property::X,
                            time: first,
                            curve: Curve::Linear
                        },
                        Command::SetIkCurve {
                            animation: 0,
                            constraint,
                            time,
                            curve
                        }
                    ]
                })
                .is_err()
        );
        assert_eq!(session.document(), &doc);
        assert_eq!(session.pose_edits(), &pending);
        assert!(session.can_redo());
    }
    let wire = serde_json::to_value(Command::SetIkCurve {
        animation: 0,
        constraint: 0,
        time: first,
        curve: raw_ik(),
    })
    .unwrap();
    assert_eq!(wire["command"], "setIkCurve");
    assert_eq!(
        serde_json::to_value(serde_json::from_value::<Command>(wire.clone()).unwrap()).unwrap(),
        wire
    );
    let mut unknown = wire.clone();
    unknown["mix"] = serde_json::json!(0.5);
    assert!(serde_json::from_value::<Command>(unknown).is_err());
    let mut unknown_curve = wire;
    unknown_curve["curve"] = serde_json::json!({"type":"linear","extra":true});
    assert!(serde_json::from_value::<Command>(unknown_curve).is_err());
}
