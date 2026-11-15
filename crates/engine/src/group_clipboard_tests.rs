//! Original grouped timing goldens and shared raw-payload/atomicity boundaries.
use crate::*;
use peachy_document::*;

fn key(time: f32, value: f32) -> Keyframe {
    Keyframe {
        time,
        value,
        curve: Curve::BezierOffset([0.2, -0., 0.8, 7.25]),
    }
}
fn bone(bone: usize, time: f32) -> AnimationKey {
    AnimationKey::Bone {
        bone,
        property: Property::Rotation,
        time,
    }
}
fn timing_lab() -> Document {
    let mut doc = Document::new("Original grouped Wind timing".into(), 60).unwrap();
    for name in ["first", "second"] {
        doc.bones.push(Bone {
            name: name.into(),
            parent: Some(0),
            length: 10.,
            setup: Transform::default(),
            inherit: Inherit::Normal,
        });
    }
    doc.animations = vec![
        Animation::new(
            "source",
            vec![
                Track {
                    bone: 1,
                    property: Property::Rotation,
                    keys: vec![key(5. / 60., 11.), key(25. / 60., -17.)],
                },
                Track {
                    bone: 2,
                    property: Property::Rotation,
                    keys: vec![key(12. / 60., 23.), key(35. / 60., -31.)],
                },
            ],
        ),
        Animation::new("target", vec![]),
    ];
    doc.validate().unwrap();
    doc
}
fn mapping(copy: &AnimationKeyGroupCopy) -> Vec<AnimationKeyGroupTarget> {
    copy.source_owners
        .iter()
        .cloned()
        .enumerate()
        .map(|(entry, target)| AnimationKeyGroupTarget { entry, target })
        .collect()
}
fn paste(
    animation: usize,
    time: f32,
    clipboard: &AnimationKeyGroupClipboard,
    targets: &[AnimationKeyGroupTarget],
) -> Command {
    Command::PasteAnimationKeyGroup {
        animation,
        time,
        clipboard: clipboard.clone(),
        targets: targets.to_vec(),
    }
}

#[test]
fn grouped_clipboard_native_unequal_starts_share_one_global_origin_and_keep_first_owner_order() {
    // Installed 4.3.26: frames5/25 and12/35 pasted at50 =>50/70 and57/80.
    // Source selection starts on the later owner; the global zero is in entry1.
    let doc = timing_lab();
    let copied = copy_animation_key_group(
        &doc,
        0,
        &[
            bone(2, 12. / 60.),
            bone(1, 25. / 60.),
            bone(2, 35. / 60.),
            bone(1, 5. / 60.),
        ],
    )
    .unwrap();
    assert_eq!(
        copied.source_owners,
        vec![KeyOwner::Bone { bone: 2 }, KeyOwner::Bone { bone: 1 }]
    );
    let AnimationKeyClipboard::Bone { keys } = &copied.clipboard.entries[0] else {
        unreachable!()
    };
    assert_eq!(
        keys[0].key.time.to_bits(),
        (12_f32 / 60. - 5_f32 / 60.).to_bits()
    );
    assert_eq!(
        keys[1].key.time.to_bits(),
        (35_f32 / 60. - 5_f32 / 60.).to_bits()
    );
    assert!(keys[0].key.time > 0.);
    let mut targets = mapping(&copied);
    targets.reverse(); // Mapping list order is not entry order.
    let accepted =
        pasted_animation_key_group(&doc, 1, 50. / 60., &copied.clipboard, &targets).unwrap();
    let result = apply_command(&doc, paste(1, 50. / 60., &copied.clipboard, &targets)).unwrap();
    assert_eq!(
        accepted.iter().map(AnimationKey::owner).collect::<Vec<_>>(),
        vec![
            KeyOwner::Bone { bone: 2 },
            KeyOwner::Bone { bone: 2 },
            KeyOwner::Bone { bone: 1 },
            KeyOwner::Bone { bone: 1 }
        ]
    );
    for (bone, frames) in [(1, [50., 70.]), (2, [57., 80.])] {
        let keys = &result.animations[1]
            .tracks
            .iter()
            .find(|track| track.bone == bone)
            .unwrap()
            .keys;
        for (key, frame) in keys.iter().zip(frames) {
            assert!((key.time * 60. - frame).abs() < 0.00002);
        }
    }
    assert_eq!(result.animations[0], doc.animations[0]);
    // Existing single-owner payloads still require their own zero anchor.
    assert!(
        apply_command(
            &doc,
            Command::PasteAnimationKeys {
                animation: 1,
                target: KeyOwner::Bone { bone: 2 },
                time: 0.,
                clipboard: copied.clipboard.entries[0].clone()
            }
        )
        .is_err()
    );
    assert!(copy_animation_keys(&doc, 0, &[bone(1, 5. / 60.), bone(2, 12. / 60.)]).is_err());
}

fn reference(placeholder: &str) -> AttachmentRef {
    AttachmentRef {
        skin: "default".into(),
        slot: "hand.R".into(),
        placeholder: placeholder.into(),
    }
}
fn all_typed_lab() -> (Document, Vec<AnimationKey>) {
    let mut doc = crate::native_fixture::decode(include_str!(
        "../../../examples/peach-bot/linked-mesh-bot.peachy"
    ))
    .unwrap();
    doc.constraints = vec![
        Constraint::Ik(IkConstraint {
            name: "group IK".into(),
            bones: vec![3],
            target: 2,
            setup: IkSetup {
                mix: 0.5,
                bend_positive: true,
            },
            stretch: false,
        }),
        Constraint::Slider(SliderConstraint::new("group Slider", 1)),
    ];
    let clip = &mut doc.animations[0];
    clip.tracks
        .retain(|track| !(track.bone == 0 && track.property == Property::Rotation));
    clip.tracks.push(Track {
        bone: 0,
        property: Property::Rotation,
        keys: vec![key(0.125, 17.)],
    });
    clip.ik_tracks = vec![IkTrack {
        constraint: 0,
        keys: vec![IkKeyframe {
            time: 0.25,
            mix: 0.25,
            bend_positive: false,
            curve: IkCurve::Bezier {
                time: [0.123456789012345, 0.876543210987654],
                mix: [0.125, 0.875],
            },
        }],
    }];
    clip.slider_tracks = vec![SliderTrack {
        constraint: 1,
        property: SliderProperty::Mix,
        keys: vec![key(0.375, -0.5)],
    }];
    clip.attachment_tracks = vec![AttachmentTrack {
        slot: 13,
        keys: vec![AttachmentKey {
            time: 0.5,
            attachment: Some("glove".into()),
        }],
    }];
    clip.color_tracks = vec![ColorTrack {
        slot: 13,
        keys: vec![ColorKey {
            time: 0.625,
            color: [20, 40, 80, 160],
            curves: [
                Curve::Linear,
                Curve::Bezier([0.2, -0.5, 0.8, 1.25]),
                Curve::BezierOffset([0.2, -0., 0.8, 0.375]),
                Curve::Linear,
            ],
        }],
    }];
    clip.draw_order = vec![DrawOrderKey {
        time: 0.75,
        offsets: vec![
            DrawOrderOffset { slot: 0, offset: 0 },
            DrawOrderOffset { slot: 1, offset: 0 },
        ],
    }];
    let deform = clip
        .deform_tracks
        .iter()
        .find(|track| track.attachment == reference("hand.R"))
        .unwrap()
        .keys[0]
        .time;
    let selected = vec![
        bone(0, 0.125),
        AnimationKey::Ik {
            constraint: 0,
            time: 0.25,
        },
        AnimationKey::Slider {
            constraint: 1,
            property: SliderProperty::Mix,
            time: 0.375,
        },
        AnimationKey::SlotAttachment {
            slot: 13,
            time: 0.5,
        },
        AnimationKey::SlotColor {
            slot: 13,
            time: 0.625,
        },
        AnimationKey::DrawOrder { time: 0.75 },
        AnimationKey::Deform {
            attachment: reference("glove"),
            time: deform,
        },
    ];
    doc.validate().unwrap();
    (doc, selected)
}

#[test]
fn grouped_clipboard_all_typed_payloads_and_f64_ik_survive_query_paste_history_and_pending() {
    let (doc, selected) = all_typed_lab();
    let copied = copy_animation_key_group(&doc, 0, &selected).unwrap();
    assert_eq!(copied.clipboard.entries.len(), 6);
    assert_eq!(
        copied.source_owners.last(),
        Some(&KeyOwner::Deform {
            attachment: reference("hand.R")
        })
    );
    let targets = mapping(&copied);
    let expected = pasted_animation_key_group(&doc, 1, 4., &copied.clipboard, &targets).unwrap();
    let mut session = Session::new(doc.clone()).unwrap();
    let target = EditTarget::Animate {
        animation: 1,
        time: 0.125,
    };
    let mut local = doc.bones[0].setup;
    local.x += 3.5;
    session
        .edit_pose(0, target, local, &[Property::X], false)
        .unwrap();
    session
        .edit_slider_pose(1, target, &[SliderPoseEdit::Mix { value: 0.625 }], false)
        .unwrap();
    let pending = serde_json::to_value(session.pose_edits()).unwrap();
    session
        .dispatch(paste(1, 4., &copied.clipboard, &targets))
        .unwrap();
    let after = session.document().clone();
    let copied_back = copy_animation_key_group(&after, 1, &expected).unwrap();
    assert_eq!(copied_back.clipboard, copied.clipboard);
    assert_eq!(
        after.animations[1]
            .ik_tracks
            .iter()
            .find(|track| track.constraint == 0)
            .unwrap()
            .keys[0]
            .curve,
        IkCurve::Bezier {
            time: [0.123456789012345, 0.876543210987654],
            mix: [0.125, 0.875]
        }
    );
    assert_eq!(after.animations[0], doc.animations[0]);
    assert_eq!(after.skins, doc.skins);
    assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pending);
    session.undo();
    assert_eq!(session.document(), &doc);
    assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pending);
    session.redo();
    assert_eq!(session.document(), &after);
    assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pending);
    let receipt = serde_json::to_value(copied).unwrap();
    assert!(receipt.get("sourceOwners").is_some());
    assert!(receipt.get("source_owners").is_none());
}

#[test]
fn grouped_clipboard_mapping_aliases_collapses_and_bad_batches_are_atomic() {
    let (doc, selected) = all_typed_lab();
    let copied = copy_animation_key_group(&doc, 0, &selected).unwrap();
    let targets = mapping(&copied);
    assert!(
        copy_animation_key_group(&doc, 0, &[selected[0].clone(), selected[0].clone()]).is_err()
    );
    let deform = selected.last().unwrap().time();
    assert!(
        copy_animation_key_group(
            &doc,
            0,
            &[
                AnimationKey::Deform {
                    attachment: reference("hand.R"),
                    time: deform
                },
                AnimationKey::Deform {
                    attachment: reference("glove"),
                    time: deform
                }
            ]
        )
        .is_err()
    );
    let mut session = Session::new(doc.clone()).unwrap();
    session
        .dispatch(Command::RenameBone {
            bone: 0,
            name: "redo".into(),
        })
        .unwrap();
    session.undo();
    let receipt = session.last_edit_effects().clone();
    let mut invalids = vec![targets[..targets.len() - 1].to_vec()];
    let mut duplicate = targets.clone();
    duplicate[1].entry = duplicate[0].entry;
    invalids.push(duplicate);
    let mut outside = targets.clone();
    outside[0].entry = usize::MAX;
    invalids.push(outside);
    let mut unknown = targets.clone();
    unknown[0].target = KeyOwner::Bone { bone: usize::MAX };
    invalids.push(unknown);
    let mut wrong = targets.clone();
    wrong[0].target = KeyOwner::Slot { slot: 0 };
    invalids.push(wrong);
    for invalid in invalids {
        assert!(pasted_animation_key_group(&doc, 1, 2., &copied.clipboard, &invalid).is_err());
        assert!(
            session
                .dispatch(Command::Batch {
                    commands: vec![
                        Command::RenameBone {
                            bone: 0,
                            name: "discarded".into()
                        },
                        paste(1, 2., &copied.clipboard, &invalid)
                    ]
                })
                .is_err()
        );
        assert_eq!(session.document(), &doc);
        assert_eq!(session.last_edit_effects(), &receipt);
        assert!(session.can_redo());
    }
    let deform_entry = copied.clipboard.entries.last().unwrap().clone();
    let alias_clip = AnimationKeyGroupClipboard {
        entries: vec![deform_entry.clone(), deform_entry],
    };
    let alias_targets = vec![
        AnimationKeyGroupTarget {
            entry: 0,
            target: KeyOwner::Deform {
                attachment: reference("hand.R"),
            },
        },
        AnimationKeyGroupTarget {
            entry: 1,
            target: KeyOwner::Deform {
                attachment: reference("glove"),
            },
        },
    ];
    assert!(
        pasted_animation_key_group(&doc, 1, 0., &alias_clip, &alias_targets)
            .unwrap_err()
            .contains("unique canonical")
    );
    let mut close = timing_lab();
    let a = 0.25_f32;
    let b = f32::from_bits(a.to_bits() + 1);
    close.animations[0].tracks[0].keys = vec![key(a, 1.), key(b, 2.)];
    close.validate().unwrap();
    let copied = copy_animation_key_group(&close, 0, &[bone(1, a), bone(1, b)]).unwrap();
    assert!(
        pasted_animation_key_group(&close, 1, 16., &copied.clipboard, &mapping(&copied))
            .unwrap_err()
            .contains("collapse")
    );
    assert!(apply_command(&close, paste(1, 16., &copied.clipboard, &mapping(&copied))).is_err());
}

fn slot_lab() -> Document {
    let mut doc = timing_lab();
    for name in ["first slot", "second slot"] {
        doc.slots.push(Slot {
            name: name.into(),
            bone: 0,
            attachment: None,
            color: WHITE,
            blend: BlendMode::Normal,
        });
    }
    doc.animations[1].attachment_tracks = vec![AttachmentTrack {
        slot: 0,
        keys: vec![AttachmentKey {
            time: 2.,
            attachment: Some("untouched".into()),
        }],
    }];
    doc.validate().unwrap();
    doc
}
fn missing(time: f32) -> AttachmentKey {
    AttachmentKey {
        time,
        attachment: Some("not available in any skin".into()),
    }
}

#[test]
fn grouped_clipboard_skips_keep_global_origin_and_all_skipped_is_a_true_noop() {
    let doc = slot_lab();
    let clipboard = AnimationKeyGroupClipboard {
        entries: vec![
            AnimationKeyClipboard::Slot {
                attachment_keys: vec![missing(0.)],
                color_keys: vec![],
            },
            AnimationKeyClipboard::Slot {
                attachment_keys: vec![
                    missing(0.5),
                    AttachmentKey {
                        time: 1.,
                        attachment: None,
                    },
                ],
                color_keys: vec![ColorKey {
                    time: 0.75,
                    color: [20, 40, 80, 160],
                    curves: [Curve::Linear; 4],
                }],
            },
        ],
    };
    let targets = vec![
        AnimationKeyGroupTarget {
            entry: 0,
            target: KeyOwner::Slot { slot: 0 },
        },
        AnimationKeyGroupTarget {
            entry: 1,
            target: KeyOwner::Slot { slot: 1 },
        },
    ];
    let accepted = pasted_animation_key_group(&doc, 1, 2., &clipboard, &targets).unwrap();
    assert_eq!(
        accepted,
        vec![
            AnimationKey::SlotAttachment { slot: 1, time: 3. },
            AnimationKey::SlotColor {
                slot: 1,
                time: 2.75
            }
        ]
    );
    let after = apply_command(&doc, paste(1, 2., &clipboard, &targets)).unwrap();
    assert_eq!(
        after.animations[1]
            .attachment_tracks
            .iter()
            .find(|track| track.slot == 0),
        doc.animations[1].attachment_tracks.first()
    );
    assert!(!accepted.contains(&AnimationKey::SlotAttachment { slot: 0, time: 2. }));
    let all_skipped = AnimationKeyGroupClipboard {
        entries: vec![
            AnimationKeyClipboard::Slot {
                attachment_keys: vec![missing(0.)],
                color_keys: vec![],
            },
            AnimationKeyClipboard::Slot {
                attachment_keys: vec![missing(0.5)],
                color_keys: vec![],
            },
        ],
    };
    assert!(
        pasted_animation_key_group(&doc, 1, 2., &all_skipped, &targets)
            .unwrap()
            .is_empty()
    );
    let mut session = Session::new(doc.clone()).unwrap();
    let target = EditTarget::Animate {
        animation: 1,
        time: 0.125,
    };
    let mut local = doc.bones[0].setup;
    local.x = 3.5;
    session
        .edit_pose(0, target, local, &[Property::X], false)
        .unwrap();
    session
        .dispatch(Command::RenameBone {
            bone: 0,
            name: "redo".into(),
        })
        .unwrap();
    session.undo();
    let pending = serde_json::to_value(session.pose_edits()).unwrap();
    let history = (session.can_undo(), session.can_redo());
    session
        .dispatch(paste(1, 2., &all_skipped, &targets))
        .unwrap();
    assert_eq!(session.document(), &doc);
    assert_eq!(serde_json::to_value(session.pose_edits()).unwrap(), pending);
    assert_eq!((session.can_undo(), session.can_redo()), history);
    let duplicate = vec![
        targets[0].clone(),
        AnimationKeyGroupTarget {
            entry: 1,
            target: KeyOwner::Slot { slot: 0 },
        },
    ];
    assert!(
        pasted_animation_key_group(&doc, 1, 2., &all_skipped, &duplicate)
            .unwrap_err()
            .contains("unique canonical")
    );
}

#[test]
fn grouped_clipboard_empty_unanchored_nonfinite_layout_and_kind_failures_never_publish() {
    let doc = timing_lab();
    let copied =
        copy_animation_key_group(&doc, 0, &[bone(1, 5. / 60.), bone(2, 12. / 60.)]).unwrap();
    let targets = mapping(&copied);
    assert!(copy_animation_key_group(&doc, 0, &[]).is_err());
    let mut failures = vec![AnimationKeyGroupClipboard { entries: vec![] }];
    let mut empty = copied.clipboard.clone();
    empty.entries[0] = AnimationKeyClipboard::Bone { keys: vec![] };
    failures.push(empty);
    let mut unanchored = copied.clipboard.clone();
    if let AnimationKeyClipboard::Bone { keys } = &mut unanchored.entries[0] {
        keys[0].key.time = 0.25;
    }
    failures.push(unanchored);
    let mut invalid_value = copied.clipboard.clone();
    if let AnimationKeyClipboard::Bone { keys } = &mut invalid_value.entries[0] {
        keys[0].key.value = f32::INFINITY;
    }
    failures.push(invalid_value);
    let mut invalid_time = copied.clipboard.clone();
    if let AnimationKeyClipboard::Bone { keys } = &mut invalid_time.entries[1] {
        keys[0].key.time = -0.25;
    }
    failures.push(invalid_time);
    let mut invalid_curve = copied.clipboard.clone();
    if let AnimationKeyClipboard::Bone { keys } = &mut invalid_curve.entries[1] {
        keys[0].key.curve = Curve::Bezier([0.2, f32::NAN, 0.8, 1.]);
    }
    failures.push(invalid_curve);
    let mut session = Session::new(doc.clone()).unwrap();
    for clipboard in failures {
        assert!(pasted_animation_key_group(&doc, 1, 1., &clipboard, &targets).is_err());
        assert!(
            session
                .dispatch(paste(1, 1., &clipboard, &targets))
                .is_err()
        );
        assert_eq!(session.document(), &doc);
        assert!(!session.can_undo());
    }
    for time in [-1., f32::INFINITY, f32::NAN] {
        assert!(pasted_animation_key_group(&doc, 1, time, &copied.clipboard, &targets).is_err());
    }
    let overflow = AnimationKeyGroupClipboard {
        entries: vec![AnimationKeyClipboard::Bone {
            keys: vec![
                ClipboardKey {
                    property: Property::Rotation,
                    key: key(0., 1.),
                },
                ClipboardKey {
                    property: Property::Rotation,
                    key: key(f32::MAX, 2.),
                },
            ],
        }],
    };
    assert!(pasted_animation_key_group(&doc, 1, f32::MAX, &overflow, &targets[..1]).is_err());
    let (all, selected) = all_typed_lab();
    let copied = copy_animation_key_group(&all, 0, &selected).unwrap();
    let targets = mapping(&copied);
    let mut layout = copied.clipboard.clone();
    let AnimationKeyClipboard::Deform { layout: shape, .. } = layout.entries.last_mut().unwrap()
    else {
        unreachable!()
    };
    *shape = DeformLayout::Weighted {
        influence_counts: vec![2; 3],
    };
    assert!(pasted_animation_key_group(&all, 1, 1., &layout, &targets).is_err());
}

#[test]
fn grouped_clipboard_receipt_remaps_explicit_targets_without_mutating_portable_payload() {
    let (doc, selected) = all_typed_lab();
    let copied = copy_animation_key_group(&doc, 0, &selected).unwrap();
    let portable = serde_json::to_vec(&copied.clipboard).unwrap();
    let mut session = Session::new(doc.clone()).unwrap();
    session
        .dispatch(Command::CreateSliderConstraint { animation: 1 })
        .unwrap();
    let after_creation = session.document().clone();
    let map = session
        .last_edit_effects()
        .constraint_indices
        .as_ref()
        .unwrap()
        .clone();
    let remapped: Vec<_> = copied
        .source_owners
        .iter()
        .enumerate()
        .map(|(entry, owner)| AnimationKeyGroupTarget {
            entry,
            target: map.map_owner(owner).unwrap().unwrap(),
        })
        .collect();
    assert!(
        session
            .dispatch(paste(1, 4., &copied.clipboard, &mapping(&copied)))
            .is_err()
    );
    assert_eq!(session.document(), &after_creation);
    session
        .dispatch(paste(1, 4., &copied.clipboard, &remapped))
        .unwrap();
    session.undo();
    assert_eq!(session.document(), &after_creation);
    session.undo();
    assert_eq!(session.document(), &doc);
    assert_eq!(serde_json::to_vec(&copied.clipboard).unwrap(), portable);
    let mut new = Session::new(timing_lab()).unwrap();
    new.dispatch(Command::CreateSliderConstraint { animation: 1 })
        .unwrap();
    new.dispatch(Command::SetSliderKey {
        animation: 0,
        constraint: 0,
        property: SliderProperty::Mix,
        key: key(0.125, 0.375),
    })
    .unwrap();
    let created = copy_animation_key_group(
        new.document(),
        0,
        &[AnimationKey::Slider {
            constraint: 0,
            property: SliderProperty::Mix,
            time: 0.125,
        }],
    )
    .unwrap();
    new.undo();
    new.undo();
    assert_eq!(
        new.last_edit_effects()
            .constraint_indices
            .as_ref()
            .unwrap()
            .map_owner(&created.source_owners[0])
            .unwrap(),
        None
    );
}

#[test]
fn grouped_clipboard_wire_roundtrips_all_leaves_and_rejects_positional_or_loose_nested_shapes() {
    let (doc, selected) = all_typed_lab();
    let copied = copy_animation_key_group(&doc, 0, &selected).unwrap();
    let value = serde_json::to_value(&copied.clipboard).unwrap();
    assert_eq!(
        serde_json::from_value::<AnimationKeyGroupClipboard>(value.clone()).unwrap(),
        copied.clipboard
    );
    let target = mapping(&copied);
    let command = serde_json::to_value(paste(1, 4., &copied.clipboard, &target)).unwrap();
    let decoded: Command = serde_json::from_value(command.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), command);
    let mut bad = vec![
        serde_json::json!([]),
        serde_json::json!(null),
        serde_json::json!({}),
        serde_json::json!({"entries":[],"extra":0}),
    ];
    for (pointer, replacement) in [
        ("/entries", serde_json::json!({})),
        ("/entries/0", serde_json::json!([])),
        ("/entries/0/keys/0", serde_json::json!(["rotation", {}])),
        (
            "/entries/0/keys/0/property",
            serde_json::json!({"rotation":null}),
        ),
        (
            "/entries/0/keys/0/key",
            serde_json::json!([0.125, 17., "linear"]),
        ),
        (
            "/entries/0/keys/0/key/curve",
            serde_json::json!({"linear":null}),
        ),
        (
            "/entries/0/keys/0/key/curve",
            serde_json::json!({"bezier":[0.2,0.3,0.8,1.],"extra":0}),
        ),
        ("/entries/1/keys/0", serde_json::json!([])),
        ("/entries/1/keys/0/curve", serde_json::json!("linear")),
        (
            "/entries/2/keys/0/property",
            serde_json::json!({"mix":null}),
        ),
        (
            "/entries/3/attachment_keys/0",
            serde_json::json!([0.5, null]),
        ),
        (
            "/entries/3/attachment_keys/0",
            serde_json::json!({"time":0.5}),
        ),
        ("/entries/3/color_keys/0", serde_json::json!([])),
        (
            "/entries/3/color_keys/0/curves/0",
            serde_json::json!({"stepped":null}),
        ),
        ("/entries/4/keys/0", serde_json::json!([])),
        ("/entries/4/keys/0/offsets/0", serde_json::json!([0, 0])),
        ("/entries/5/layout", serde_json::json!(["weighted", [1, 1]])),
        ("/entries/5/keys/0", serde_json::json!([])),
    ] {
        let mut invalid = value.clone();
        *invalid.pointer_mut(pointer).unwrap() = replacement;
        bad.push(invalid);
    }
    for invalid in bad {
        assert!(
            serde_json::from_value::<AnimationKeyGroupClipboard>(invalid.clone()).is_err(),
            "Accepted {invalid}"
        );
    }
    let valid = serde_json::to_value(target.last().unwrap()).unwrap();
    for invalid in [
        serde_json::json!([]),
        serde_json::json!({"entry":0}),
        serde_json::json!({"target":{"type":"bone","bone":0}}),
        serde_json::json!({"entry":0,"target":["bone",0]}),
        serde_json::json!({"entry":0,"target":{"type":"drawOrder","slot":0}}),
        serde_json::json!({"entry":0,"target":{"type":"deform","attachment":["default","hand.R","hand.R"]}}),
    ] {
        assert!(serde_json::from_value::<AnimationKeyGroupTarget>(invalid).is_err());
    }
    assert_eq!(
        serde_json::from_value::<AnimationKeyGroupTarget>(valid).unwrap(),
        *target.last().unwrap()
    );
    for invalid in [
        r#"{"entries":[],"entries":[]}"#,
        r#"{"entries":[{"type":"bone","keys":[],"keys":[]}] }"#,
        r#"{"entries":[{"type":"bone","keys":[{"property":"x","property":"y","key":{"time":0,"value":1,"curve":"linear"}}]}]}"#,
        r#"{"entries":[{"type":"bone","keys":[{"property":"x","key":{"time":0,"time":1,"value":1,"curve":"linear"}}]}]}"#,
        r#"{"entries":[{"type":"bone","keys":[{"property":"x","key":{"time":0,"value":1,"curve":{"bezier":[0,0,1,1],"bezier":[0,0,1,1]}}}]}]}"#,
    ] {
        assert!(
            serde_json::from_str::<AnimationKeyGroupClipboard>(invalid).is_err(),
            "Accepted {invalid}"
        );
    }
    for invalid in [
        r#"{"entry":0,"entry":1,"target":{"type":"bone","bone":0}}"#,
        r#"{"entry":0,"target":{"type":"bone","bone":0,"bone":1}}"#,
        r#"{"entry":0,"target":{"type":"deform","attachment":{"skin":"default","skin":"other","slot":"s","placeholder":"p"}}}"#,
    ] {
        assert!(serde_json::from_str::<AnimationKeyGroupTarget>(invalid).is_err());
    }
}
