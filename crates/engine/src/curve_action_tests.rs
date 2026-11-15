//! Original native observations establish selection, independent reset and
//! actual-time/value outer-angle matching. Live Match copies one handle only.
use crate::*;
use peachy_document::*;

fn curve(h: [f32; 4], times: [f32; 2], values: [f32; 2]) -> Curve {
    if values[0] == values[1] {
        return Curve::BezierOffset([
            (h[0] - times[0]) / (times[1] - times[0]),
            h[1] - values[0],
            (h[2] - times[0]) / (times[1] - times[0]),
            h[3] - values[0],
        ]);
    }
    Curve::Bezier([
        (h[0] - times[0]) / (times[1] - times[0]),
        (h[1] - values[0]) / (values[1] - values[0]),
        (h[2] - times[0]) / (times[1] - times[0]),
        (h[3] - values[0]) / (values[1] - values[0]),
    ])
}
fn fixture() -> Document {
    let mut d = apply_command(
        &Document::new("Original curve actions".into(), 30).unwrap(),
        Command::AddAnimation {
            name: "curves".into(),
            duration: 0.,
        },
    )
    .unwrap();
    d.animations[0].tracks = vec![
        Track {
            bone: 0,
            property: Property::Rotation,
            keys: vec![
                Keyframe {
                    time: 0.,
                    value: 0.,
                    curve: curve([0.06, 12.6, 0.24, 29.384615], [0., 0.3], [0., 30.]),
                },
                Keyframe {
                    time: 0.3,
                    value: 30.,
                    curve: curve([0.5333333, 30., 0.7666666, 10.], [0.3, 1.], [30., 10.]),
                },
                Keyframe {
                    time: 1.,
                    value: 10.,
                    curve: Curve::Linear,
                },
            ],
        },
        Track {
            bone: 0,
            property: Property::X,
            keys: vec![
                Keyframe {
                    time: 0.,
                    value: 0.,
                    curve: curve([0.05, 3., 0.4, 13.5], [0., 0.5], [0., 15.]),
                },
                Keyframe {
                    time: 0.5,
                    value: 15.,
                    curve: curve([0.675, 13., 0.825, -3.], [0.5, 1.], [15., -5.]),
                },
                Keyframe {
                    time: 1.,
                    value: -5.,
                    curve: Curve::Stepped,
                },
            ],
        },
    ];
    d.validate().unwrap();
    d
}
fn key(property: Property, time: f32) -> AnimationKey {
    AnimationKey::Bone {
        bone: 0,
        property,
        time,
    }
}
fn selection(property: Property, time: f32) -> CurveSelection {
    CurveSelection {
        key: key(property, time),
        channel: 0,
    }
}
fn preset(selection: Vec<CurveSelection>, preset: CurvePreset, separate: bool) -> Command {
    Command::ApplyCurvePreset {
        animation: 0,
        selection,
        preset,
        separate,
    }
}
fn handles(d: &Document, property: Property, index: usize) -> [f32; 4] {
    let keys = &d.animations[0]
        .tracks
        .iter()
        .find(|t| t.property == property)
        .unwrap()
        .keys;
    let (a, b) = (keys[index], keys[index + 1]);
    let (h, span) = match a.curve {
        Curve::Bezier(h) => (h, b.value - a.value),
        Curve::BezierOffset(h) => (h, 1.),
        _ => panic!("Bezier"),
    };
    [
        a.time + (b.time - a.time) * h[0],
        a.value + span * h[1],
        a.time + (b.time - a.time) * h[2],
        a.value + span * h[3],
    ]
}
fn close(actual: [f32; 4], expected: [f32; 4]) {
    for (a, b) in actual.into_iter().zip(expected) {
        assert!((a - b).abs() < 0.00002, "{a} vs {b}");
    }
}

#[test]
fn curve_preset_matches_original_separate_and_unequal_span_outer_angle_observations() {
    let before = fixture();
    let independent = apply_command(
        &before,
        preset(
            vec![selection(Property::Rotation, 0.)],
            CurvePreset::Bezier,
            true,
        ),
    )
    .unwrap();
    close(
        handles(&independent, Property::Rotation, 0),
        [0.1, 0., 0.2, 30.],
    );
    assert_eq!(
        independent.animations[0].tracks[0].keys[1],
        before.animations[0].tracks[0].keys[1]
    );
    let last = apply_command(
        &before,
        preset(
            vec![selection(Property::Rotation, 1.)],
            CurvePreset::Bezier,
            false,
        ),
    )
    .unwrap();
    close(
        handles(&last, Property::Rotation, 1),
        [0.5333333, 32.39316, 0.7666666, 10.],
    );
    assert_eq!(
        last.animations[0].tracks[0].keys[0],
        before.animations[0].tracks[0].keys[0]
    );
    let first = apply_command(
        &last,
        preset(
            vec![selection(Property::Rotation, 0.)],
            CurvePreset::Bezier,
            false,
        ),
    )
    .unwrap();
    close(
        handles(&first, Property::Rotation, 0),
        [0.1, 0., 0.2, 28.97436],
    );
    assert_eq!(
        first.animations[0].tracks[0].keys[1],
        last.animations[0].tracks[0].keys[1]
    );
    assert_eq!(
        first.animations[0].tracks[1],
        before.animations[0].tracks[1]
    );
    for (old, new) in before.animations[0].tracks[0]
        .keys
        .iter()
        .zip(&first.animations[0].tracks[0].keys)
    {
        assert_eq!(old.time.to_bits(), new.time.to_bits());
        assert_eq!(old.value.to_bits(), new.value.to_bits());
    }
}

#[test]
fn curve_preset_rgba_channels_keep_bytes_and_grouped_steps_while_custom_ik_keeps_wide_time() {
    let mut d = fixture();
    d.slots.push(Slot {
        name: "paint".into(),
        bone: 0,
        attachment: None,
        color: WHITE,
        blend: BlendMode::Normal,
    });
    d.animations[0].color_tracks = vec![ColorTrack {
        slot: 0,
        keys: vec![
            ColorKey {
                time: 0.,
                color: [255, 128, 64, 255],
                curves: [Curve::Stepped; 4],
            },
            ColorKey {
                time: 1.,
                color: [0, 128, 128, 0],
                curves: [Curve::Linear; 4],
            },
        ],
    }];
    let selected = CurveSelection {
        key: AnimationKey::SlotColor { slot: 0, time: 0. },
        channel: 2,
    };
    let custom = [0.125, 0.25, 0.875, 0.75];
    let applied = apply_command(
        &d,
        preset(vec![selected.clone()], CurvePreset::Custom(custom), true),
    )
    .unwrap();
    assert_eq!(
        applied.animations[0].color_tracks[0].keys[0].curves,
        [
            Curve::Linear,
            Curve::Linear,
            Curve::Bezier(custom.map(|v| v as f32)),
            Curve::Linear
        ]
    );
    assert_eq!(
        applied.animations[0].color_tracks[0].keys[0].color,
        d.animations[0].color_tracks[0].keys[0].color
    );
    let stepped = apply_command(
        &applied,
        preset(vec![selected], CurvePreset::Stepped, false),
    )
    .unwrap();
    assert_eq!(
        stepped.animations[0].color_tracks[0].keys[0].curves,
        [Curve::Stepped; 4]
    );
    let mut ik: Document =
        serde_json::from_str(include_str!("../../../examples/peach-bot/ik-bot.peachy")).unwrap();
    ik.animations[0].ik_tracks = vec![IkTrack {
        constraint: 0,
        keys: vec![
            IkKeyframe {
                time: 0.,
                mix: 0.25,
                bend_positive: false,
                curve: IkCurve::Linear,
            },
            IkKeyframe {
                time: 1.,
                mix: 0.75,
                bend_positive: true,
                curve: IkCurve::Linear,
            },
        ],
    }];
    let wide = [0.49999999999999994, 0.1, 0.7500000000000001, 0.9];
    let after = apply_command(
        &ik,
        preset(
            vec![CurveSelection {
                key: AnimationKey::Ik {
                    constraint: 0,
                    time: 0.,
                },
                channel: 0,
            }],
            CurvePreset::Custom(wide),
            true,
        ),
    )
    .unwrap();
    let c = after.animations[0].ik_tracks[0].keys[0];
    let IkCurve::Bezier { time, mix } = c.curve else {
        panic!("IK")
    };
    assert_eq!(time, [wide[0], wide[2]]);
    assert_eq!(mix, [0.3, 0.7]);
    assert_eq!(c.mix.to_bits(), 0.25_f32.to_bits());
    assert!(!c.bend_positive);
}

#[test]
fn curve_handle_match_copies_only_chosen_normalized_coordinates_and_primary_uses_full_selection() {
    let before = fixture();
    let chosen = selection(Property::Rotation, 0.);
    let selections = vec![chosen.clone(), selection(Property::X, 0.)];
    let action = |match_curves| Command::EditCurveHandle {
        animation: 0,
        selection: selections.clone(),
        primary: chosen.clone(),
        handle: 0,
        point: [0.05932204, 5.982906],
        match_curves,
        separate: true,
    };
    let matched = apply_command(&before, action(true)).unwrap();
    close(
        handles(&matched, Property::X, 0),
        [0.09887007, 2.991453, 0.4, 13.5],
    );
    for index in 0..2 {
        let Curve::Bezier(old) = before.animations[0].tracks[index].keys[0].curve else {
            panic!()
        };
        let Curve::Bezier(new) = matched.animations[0].tracks[index].keys[0].curve else {
            panic!()
        };
        assert_eq!(old[2].to_bits(), new[2].to_bits());
        assert_eq!(old[3].to_bits(), new[3].to_bits());
    }
    let unmatched = apply_command(&before, action(false)).unwrap();
    assert_eq!(
        unmatched.animations[0].tracks[1],
        before.animations[0].tracks[1]
    );
    let hidden_primary = Command::EditCurveHandle {
        animation: 0,
        selection: vec![
            selection(Property::Rotation, 1.),
            selection(Property::X, 1.),
        ],
        primary: selection(Property::Rotation, 0.3),
        handle: 0,
        point: [0.6, 31.],
        match_curves: false,
        separate: true,
    };
    assert!(
        apply_command(&before, hidden_primary)
            .unwrap_err()
            .contains("effective member")
    );
    let footprint = curve_edit_targets(&before, 0, &[selection(Property::Rotation, 1.)]).unwrap();
    assert_eq!(footprint.len(), 2);
    assert!(
        footprint
            .iter()
            .any(|t| t.key == key(Property::Rotation, 0.))
    );
    assert!(
        footprint
            .iter()
            .any(|t| t.key == key(Property::Rotation, 0.3))
    );
}

#[test]
fn curve_actions_fail_atomically_and_noop_terminal_presets_keep_redo() {
    let before = fixture();
    let mut session = Session::new(before.clone()).unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo".into(),
        })
        .unwrap();
    session.undo();
    let no_op = preset(
        vec![
            selection(Property::Rotation, 1.),
            selection(Property::X, 1.),
        ],
        CurvePreset::Bezier,
        false,
    );
    session.dispatch(no_op).unwrap();
    assert_eq!(session.document(), &before);
    assert!(session.can_redo());
    let bad = [
        preset(
            vec![selection(Property::Rotation, 0.)],
            CurvePreset::Custom([f64::NAN, 0., 1., 1.]),
            true,
        ),
        preset(
            vec![CurveSelection {
                key: key(Property::X, 0.),
                channel: 1,
            }],
            CurvePreset::Linear,
            true,
        ),
        // Adjacent OFF preset ordering is deliberately unclaimed. Failure
        // must roll back the outer batch and retain the preexisting redo.
        preset(
            vec![
                selection(Property::Rotation, 0.),
                selection(Property::Rotation, 0.3),
            ],
            CurvePreset::Bezier,
            false,
        ),
        Command::EditCurveHandle {
            animation: 0,
            selection: vec![selection(Property::Rotation, 0.)],
            primary: selection(Property::Rotation, 0.),
            handle: 2,
            point: [0.1, 1.],
            match_curves: false,
            separate: true,
        },
    ];
    for action in bad {
        assert!(
            session
                .dispatch(Command::Batch {
                    commands: vec![
                        Command::RenameAnimation {
                            animation: 0,
                            name: "must rollback".into()
                        },
                        action
                    ]
                })
                .is_err()
        );
        assert_eq!(session.document(), &before);
        assert!(session.can_redo());
        assert!(!session.can_undo());
    }
    let wire = serde_json::to_value(preset(
        vec![selection(Property::Rotation, 0.)],
        CurvePreset::Custom([0.2, 0.1, 0.8, 0.9]),
        true,
    ))
    .unwrap();
    assert_eq!(
        serde_json::to_value(serde_json::from_value::<Command>(wire.clone()).unwrap()).unwrap(),
        wire
    );
    let mut unknown = wire;
    unknown["curve"] = serde_json::json!("linear");
    assert!(serde_json::from_value::<Command>(unknown).is_err());
}

#[test]
fn curve_preset_matches_twelve_original_native_middle_key_resets() {
    // Actual 4.3.26 data from twelve original numeric cases (November14),
    // all middle keys selected, SeparateOFF. No implementation-derived expected
    // outputs: source angles, flat endpoints and both outgoing limits differ.
    #[derive(Clone, Copy)]
    struct Case {
        name: &'static str,
        prior_y: f32,
        prior: Option<Curve>,
        next: u8,
        end: f32,
        previous_start: f32,
        previous_x: f32,
        expected: [f32; 4],
    }
    let base = Case {
        name: "",
        prior_y: 29.,
        prior: None,
        next: 0,
        end: 10.,
        previous_start: 0.,
        previous_x: 0.4,
        expected: [0.; 4],
    };
    let cases = [
        Case {
            name: "prior29",
            expected: [0.6666667, 31.666666, 0.8333333, 10.],
            ..base
        },
        Case {
            name: "prior31",
            prior_y: 31.,
            expected: [0.6666667, 28.333334, 0.8333333, 10.],
            ..base
        },
        Case {
            name: "outgoing_lower",
            prior_y: 9.,
            expected: [0.5952381, 50., 0.8333333, 10.],
            ..base
        },
        Case {
            name: "outgoing_upper",
            prior_y: 58.,
            expected: [0.5714285, 10., 0.8333333, 10.],
            ..base
        },
        Case {
            name: "prior_linear",
            prior: Some(Curve::Linear),
            expected: [0.6666667, 30., 0.8333333, 10.],
            ..base
        },
        Case {
            name: "prior_stepped",
            prior: Some(Curve::Stepped),
            expected: [0.6666667, 30., 0.8333333, 10.],
            ..base
        },
        Case {
            name: "next_linear",
            next: 1,
            expected: [0.6666667, 31.666666, 0.8333333, 10.],
            ..base
        },
        Case {
            name: "next_stepped",
            next: 2,
            expected: [0.6666667, 31.666666, 0.8333333, 10.],
            ..base
        },
        Case {
            name: "next_bezier",
            next: 3,
            expected: [0.6666667, 31.666666, 0.8333333, 8.333333],
            ..base
        },
        Case {
            name: "flat_target",
            end: 30.,
            next: 3,
            expected: [0.6666667, 31.666666, 0.8333333, 28.333334],
            ..base
        },
        Case {
            name: "unequal_prior",
            previous_start: 0.3,
            previous_x: 0.46,
            expected: [0.6666667, 34.16666, 0.8333333, 10.],
            ..base
        },
        Case {
            name: "rising_target",
            prior_y: 9.,
            end: 50.,
            expected: [0.5952381, 50., 0.8333333, 50.],
            ..base
        },
    ];
    for case in cases {
        let mut before = fixture();
        before.fps = 60;
        let prior = case.prior.unwrap_or_else(|| {
            curve(
                [
                    if case.previous_start == 0. { 0.1 } else { 0.34 },
                    0.,
                    case.previous_x,
                    case.prior_y,
                ],
                [case.previous_start, 0.5],
                [0., 30.],
            )
        });
        let next = match case.next {
            2 => Curve::Stepped,
            3 => curve([1.1, case.end + 1., 1.4, 40.], [1., 1.5], [case.end, 40.]),
            _ => Curve::Linear,
        };
        let old_handles = if case.end == 30. {
            [0.65, 32., 0.85, 28.]
        } else {
            [0.65, 25., 0.85, 15.]
        };
        let mut keys = vec![
            Keyframe {
                time: case.previous_start,
                value: 0.,
                curve: prior,
            },
            Keyframe {
                time: 0.5,
                value: 30.,
                curve: curve(old_handles, [0.5, 1.], [30., case.end]),
            },
            Keyframe {
                time: 1.,
                value: case.end,
                curve: next,
            },
        ];
        if case.next != 0 {
            keys.push(Keyframe {
                time: 1.5,
                value: 40.,
                curve: Curve::Linear,
            });
        }
        before.animations[0].tracks = vec![Track {
            bone: 0,
            property: Property::Rotation,
            keys,
        }];
        let after = apply_command(
            &before,
            preset(
                vec![selection(Property::Rotation, 0.5)],
                CurvePreset::Bezier,
                false,
            ),
        )
        .unwrap_or_else(|error| panic!("{}: {error}", case.name));
        let actual = handles(&after, Property::Rotation, 1);
        for (actual, expected) in actual.into_iter().zip(case.expected) {
            assert!(
                (actual - expected).abs() < 0.00002,
                "{}: {actual} vs {expected}",
                case.name
            );
        }
        assert_eq!(
            after.animations[0].tracks[0].keys[0], before.animations[0].tracks[0].keys[0],
            "{} previous curve",
            case.name
        );
        assert_eq!(
            after.animations[0].tracks[0].keys[2..],
            before.animations[0].tracks[0].keys[2..],
            "{} following curve",
            case.name
        );
        for (old, new) in before.animations[0].tracks[0]
            .keys
            .iter()
            .zip(&after.animations[0].tracks[0].keys)
        {
            assert_eq!(old.time.to_bits(), new.time.to_bits());
            assert_eq!(old.value.to_bits(), new.value.to_bits());
        }
    }
}

fn incoming_fixture(end: f32, next_y: f32) -> Document {
    let mut document = fixture();
    document.animations[0].tracks[0].keys = vec![
        Keyframe {
            time: 0.,
            value: 0.,
            curve: curve([0.1, 0., 0.4, 29.], [0., 0.5], [0., 30.]),
        },
        Keyframe {
            time: 0.5,
            value: 30.,
            curve: curve(
                if end == 30. {
                    [0.65, 32., 0.85, 28.]
                } else {
                    [0.65, 25., 0.85, 15.]
                },
                [0.5, 1.],
                [30., end],
            ),
        },
        Keyframe {
            time: 1.,
            value: end,
            curve: curve([1.1, next_y, 1.4, 40.], [1., 1.5], [end, 40.]),
        },
        Keyframe {
            time: 1.5,
            value: 40.,
            curve: Curve::Linear,
        },
    ];
    document
}

#[test]
fn curve_preset_matches_native_incoming_limits_and_flat_excursions() {
    // Four original 4.3.26 inputs/exports, November14: both signs of incoming
    // clipping are centered on END, and a flat interval keeps actual offsets.
    for (end, next_y, expected) in [
        (10., 11., [0.6666667, 31.666666, 0.8333333, 8.333333]),
        (10., 31., [0.6666667, 31.666666, 0.9047619, -10.]),
        (10., -18., [0.6666667, 31.666666, 0.9285714, 30.]),
        (30., 29., [0.6666667, 31.666666, 0.8333333, 31.666666]),
    ] {
        let before = incoming_fixture(end, next_y);
        let after = apply_command(
            &before,
            preset(
                vec![selection(Property::Rotation, 0.5)],
                CurvePreset::Bezier,
                false,
            ),
        )
        .unwrap();
        close(handles(&after, Property::Rotation, 1), expected);
        assert_eq!(
            after.animations[0].tracks[0].keys[0],
            before.animations[0].tracks[0].keys[0]
        );
        assert_eq!(
            after.animations[0].tracks[0].keys[2..],
            before.animations[0].tracks[0].keys[2..]
        );
        assert_eq!(
            after.animations[0].tracks[1],
            before.animations[0].tracks[1]
        );
        for (old, new) in before.animations[0].tracks[0]
            .keys
            .iter()
            .zip(&after.animations[0].tracks[0].keys)
        {
            assert_eq!(old.time.to_bits(), new.time.to_bits());
            assert_eq!(old.value.to_bits(), new.value.to_bits());
        }
    }
}

fn live_command(handle: usize, point: [f64; 2]) -> Command {
    Command::EditCurveHandle {
        animation: 0,
        selection: vec![selection(Property::Rotation, 0.5)],
        primary: selection(Property::Rotation, 0.5),
        handle,
        point,
        match_curves: false,
        separate: false,
    }
}

#[test]
fn curve_live_linked_handles_match_original_native_normal_single_moves() {
    // Independent native before/after values, including the native angle
    // conversion's small horizontal drift. Our fixed-extent policy preserves
    // exact stored horizontal controls, so the comparison allows 0.0002 native
    // value units rather than claiming bit-identical editor trigonometry.
    let mut before = incoming_fixture(10., 11.);
    before.animations[0].tracks[0].keys[1].curve = curve(
        [0.6666667, 31.666666, 0.8333333, 8.333333],
        [0.5, 1.],
        [30., 10.],
    );
    for (handle, point, outer_index, expected) in [
        (0, [0.6666667, 28.409304], 0, [0.1, 0., 0.3999985, 30.95443]),
        (1, [0.8333333, 12.098766], 2, [1.099991, 8.740855, 1.4, 40.]),
    ] {
        let after = apply_command(&before, live_command(handle, point)).unwrap();
        let actual = handles(&after, Property::Rotation, outer_index);
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!((actual - expected).abs() < 0.0002, "{actual} vs {expected}");
        }
        let old_primary = match before.animations[0].tracks[0].keys[1].curve {
            Curve::Bezier(h) => h,
            _ => panic!(),
        };
        let new_primary = match after.animations[0].tracks[0].keys[1].curve {
            Curve::Bezier(h) => h,
            _ => panic!(),
        };
        let other = (1 - handle) * 2;
        assert_eq!(old_primary[other].to_bits(), new_primary[other].to_bits());
        assert_eq!(
            old_primary[other + 1].to_bits(),
            new_primary[other + 1].to_bits()
        );
        let old_outer = match before.animations[0].tracks[0].keys[outer_index].curve {
            Curve::Bezier(h) => h,
            _ => panic!(),
        };
        let new_outer = match after.animations[0].tracks[0].keys[outer_index].curve {
            Curve::Bezier(h) => h,
            _ => panic!(),
        };
        // Neighbor horizontal lengths and its unrelated handle retain bits.
        assert_eq!(old_outer[0].to_bits(), new_outer[0].to_bits());
        assert_eq!(old_outer[2].to_bits(), new_outer[2].to_bits());
        assert_eq!(
            old_outer[handle * 2 + 1].to_bits(),
            new_outer[handle * 2 + 1].to_bits()
        );
        let untouched_index = if handle == 0 { 2 } else { 0 };
        assert_eq!(
            after.animations[0].tracks[0].keys[untouched_index],
            before.animations[0].tracks[0].keys[untouched_index]
        );
        assert_eq!(
            after.animations[0].tracks[1],
            before.animations[0].tracks[1]
        );
        for (old, new) in before.animations[0].tracks[0]
            .keys
            .iter()
            .zip(&after.animations[0].tracks[0].keys)
        {
            assert_eq!(old.time.to_bits(), new.time.to_bits());
            assert_eq!(old.value.to_bits(), new.value.to_bits());
        }
        let mut session = Session::new(before.clone()).unwrap();
        session.dispatch(live_command(handle, point)).unwrap();
        assert_eq!(session.document(), &after);
        session.undo();
        assert_eq!(session.document(), &before);
        assert!(!session.can_undo());
        session.redo();
        assert_eq!(session.document(), &after);
    }
}

#[test]
fn curve_live_uncalibrated_domains_fail_atomically_and_separate_bypasses_only_coupling() {
    let before = incoming_fixture(10., 11.);
    let mut session = Session::new(before.clone()).unwrap();
    session
        .dispatch(Command::RenameAnimation {
            animation: 0,
            name: "redo".into(),
        })
        .unwrap();
    session.undo();
    let mut adjacent = live_command(0, [0.6, 25.726496]);
    if let Command::EditCurveHandle {
        selection: chosen,
        match_curves,
        ..
    } = &mut adjacent
    {
        chosen.push(selection(Property::Rotation, 0.));
        *match_curves = true;
    }
    let mut flat = incoming_fixture(30., 29.);
    flat.animations[0].tracks[0].keys[0].value = 30.;
    let actions = [
        (live_command(0, [0.5183616, 10.004745]), "Extreme"),
        (live_command(0, [0.5, 29.]), "Vertical"),
        (live_command(1, [1.1, 11.]), "outside-time"),
        (adjacent, "adjacent selected"),
    ];
    for (action, message) in actions {
        let failed = Command::Batch {
            commands: vec![
                Command::RenameAnimation {
                    animation: 0,
                    name: "must rollback".into(),
                },
                action.clone(),
            ],
        };
        assert!(session.dispatch(failed).unwrap_err().contains(message));
        assert_eq!(session.document(), &before);
        assert!(session.can_redo());
        assert!(!session.can_undo());
        let Command::EditCurveHandle {
            mut selection,
            primary,
            handle,
            point,
            match_curves,
            ..
        } = action
        else {
            panic!()
        };
        // The same finite raw motion is valid with independent handles.
        selection.retain(|s| s == &primary);
        assert!(
            apply_command(
                &before,
                Command::EditCurveHandle {
                    animation: 0,
                    selection,
                    primary,
                    handle,
                    point,
                    match_curves,
                    separate: true,
                }
            )
            .is_ok()
        );
    }
    assert!(
        apply_command(&flat, live_command(0, [0.7, 31.]))
            .unwrap_err()
            .contains("flat endpoints")
    );
    // An unrelated outer interval has no role for the other handle.
    let mut linear = before.clone();
    linear.animations[0].tracks[0].keys[0].curve = Curve::Linear;
    let independent = apply_command(&linear, live_command(0, [0.7, 29.])).unwrap();
    assert_eq!(
        independent.animations[0].tracks[0].keys[0],
        linear.animations[0].tracks[0].keys[0]
    );
}
