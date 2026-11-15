//! Sequence frame selection, against values observed with the external runtime
//! (key at 0.3, index 1, delay 0.2, five frames; times 0.4 … 2.5 step 0.1).
use crate::*;

#[test]
fn every_mode_matches_the_observed_runtime_table() {
    let observed = [
        (
            SequenceMode::Hold,
            "1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1 1",
        ),
        (
            SequenceMode::Once,
            "1 2 2 3 3 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4 4",
        ),
        (
            SequenceMode::Loop,
            "1 2 2 3 3 4 4 0 0 1 1 2 2 3 3 4 4 0 0 1 1 2",
        ),
        (
            SequenceMode::Pingpong,
            "1 2 2 3 3 4 4 3 3 2 2 1 1 0 0 1 1 2 2 3 3 4",
        ),
        (
            SequenceMode::OnceReverse,
            "3 2 2 1 1 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
        ),
        (
            SequenceMode::LoopReverse,
            "3 2 2 1 1 0 0 4 4 3 3 2 2 1 1 0 0 4 4 3 3 2",
        ),
        (
            SequenceMode::PingpongReverse,
            "3 2 2 1 1 0 0 1 1 2 2 3 3 4 4 3 3 2 2 1 1 0",
        ),
    ];
    for (mode, expected) in observed {
        let key = SequenceKey {
            time: 0.3,
            mode,
            index: 1,
            delay: 0.2,
        };
        let frames: Vec<String> = (4..=25)
            .map(|i| {
                let time = (i as f32 * 0.1 * 1e4).round() / 1e4;
                let frame = key.frame(time, 5);
                assert_eq!(
                    key.frame_at_time_checked(f64::from(time), 5).unwrap(),
                    frame
                );
                frame.to_string()
            })
            .collect();
        assert_eq!(frames.join(" "), expected, "{mode:?}");
    }
}

#[test]
fn frames_advance_with_the_observed_tolerance_and_paths_pad() {
    // Frame k starts at k·delay − 1e-5·delay.
    let key = SequenceKey {
        time: 0.0,
        mode: SequenceMode::Loop,
        index: 0,
        delay: 0.1,
    };
    assert_eq!(key.frame(0.3, 8), 3);
    assert_eq!(key.frame(0.299_98, 8), 2);
    let def = SequenceDef {
        count: 96,
        start: 1,
        digits: 2,
        setup_index: 0,
    };
    assert_eq!(def.frame_path("Layer_", 0), "Layer_01");
    assert_eq!(def.frame_path("Layer_", 95), "Layer_96");
}

#[test]
fn sequence_wide_reference_time_keeps_a_frame_between_adjacent_f32_times() {
    let left = 1f32;
    let right = f32::from_bits(left.to_bits() + 1);
    let time = (f64::from(left) + f64::from(right)) * 0.5;
    for (mode, initial, advanced) in [
        (SequenceMode::Hold, 0, 0),
        (SequenceMode::Once, 0, 1),
        (SequenceMode::Loop, 0, 1),
        (SequenceMode::Pingpong, 0, 1),
        (SequenceMode::OnceReverse, 4, 3),
        (SequenceMode::LoopReverse, 4, 3),
        (SequenceMode::PingpongReverse, 4, 3),
    ] {
        let key = SequenceKey {
            time: left,
            mode,
            index: 0,
            delay: (right - left) * 0.5,
        };
        assert_eq!(key.frame_at_time(time, 5), advanced, "{mode:?}");
        assert_eq!(
            key.frame_at_time_checked(time, 5).unwrap(),
            advanced,
            "{mode:?}"
        );
        assert_eq!(key.frame(time as f32, 5), initial, "{mode:?}");
    }
}

#[test]
fn sequence_checked_sampling_rejects_unrepresentable_advancing_phase_without_panicking() {
    let mut key = SequenceKey {
        time: 0.,
        mode: SequenceMode::Loop,
        index: 1,
        delay: f32::MIN_POSITIVE,
    };
    let huge = f64::from(f32::MAX);
    assert!(
        key.frame_at_time_checked(huge, 3)
            .unwrap_err()
            .contains("counter range")
    );
    key.mode = SequenceMode::Hold;
    assert_eq!(key.frame_at_time_checked(huge, 3).unwrap(), 1);
    key.mode = SequenceMode::Loop;
    key.delay = 1.;
    let max_phase = f64::from(i32::MAX);
    assert!(key.frame_at_time_checked(max_phase - 1., 3).is_ok());
    assert!(
        key.frame_at_time_checked(max_phase, 3)
            .unwrap_err()
            .contains("index plus steps")
    );
    assert!(key.frame_at_time_checked(f64::INFINITY, 3).is_err());

    // sequence-integer-holdout: the final nonnegative counter sample is valid;
    // doubling Time enters observed raw -1/setup fallback behavior, deferred.
    key.time = 0.25;
    key.delay = 0.25;
    key.index = 0;
    assert_eq!(key.frame_at_time_checked(536_870_912., 3).unwrap(), 1);
    assert!(
        key.frame_at_time_checked(1_073_741_824., 3)
            .unwrap_err()
            .contains("counter range")
    );
}
