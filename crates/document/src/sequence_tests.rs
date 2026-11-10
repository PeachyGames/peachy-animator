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
                key.frame((i as f32 * 0.1 * 1e4).round() / 1e4, 5)
                    .to_string()
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
