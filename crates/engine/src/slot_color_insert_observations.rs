//! Numeric observations from our original 16-slot probe in installed Spine
//! 4.3.26 (November 14, 2026), without a Color dialog. Key Shown inserted at
//! 0.4, then a separate held-out insertion at 0.6. The linear/stepped controls
//! and an all-flat interval canonicalized by native import are tested elsewhere.
//!
//! Each row keeps four ORIGINAL absolute X-handle pairs and four independently
//! measured subdivision indices (u * 300), inferred from native exported
//! control geometry. Expected values do not call the production scan formula.
//! The public behavioral reference is https://esotericsoftware.com/spine-graph#New-keys.
use crate::color_curve_split;
use peachy_document::Curve;

type Observation = (&'static str, [[f32; 2]; 4], [u16; 4]);

const INITIAL: [Observation; 13] = [
    ("thirds", [[0.26666668, 0.53333336]; 4], [151; 4]),
    ("half", [[0.4, 0.4]; 4], [151; 4]),
    ("quarter", [[0.2, 0.6]; 4], [150; 4]),
    ("tenth", [[0.08, 0.72]; 4], [150; 4]),
    ("asymmetric15_60", [[0.12, 0.48]; 4], [176; 4]),
    ("asymmetric40_95", [[0.32, 0.76]; 4], [118; 4]),
    ("crossing80_20", [[0.64, 0.16]; 4], [151; 4]),
    ("outside_negative30_130", [[-0.24, 1.04]; 4], [151; 4]),
    ("start_flat", [[0., 0.]; 4], [239; 4]),
    ("end_flat", [[0.8, 0.8]; 4], [62; 4]),
    (
        "mixed_channels",
        [[0.04, 0.32], [0.48, 0.76], [0.3992, 0.4008], [0.016, 0.784]],
        [204, 97, 151, 150],
    ),
    (
        "flat_rgb_alpha_fade",
        [[0.12, 0.48], [0.32, 0.76], [0.64, 0.16], [0.08, 0.72]],
        [176, 118, 151, 150],
    ),
    ("flat_endpoints_value_wiggles", [[0.2, 0.6]; 4], [150; 4]),
];

// These original control inputs are the native first pass's exported geometry.
// They were frozen BEFORE collecting the nonzero-origin held-out insertion.
const HELD_OUT: [Observation; 13] = [
    ("thirds", [[0.5351111, 0.6675556]; 4], [150; 4]),
    ("half", [[0.5013377, 0.6013334]; 4], [179; 4]),
    ("quarter", [[0.55, 0.7]; 4], [137; 4]),
    ("tenth", [[0.58, 0.76]; 4], [116; 4]),
    ("asymmetric15_60", [[0.5286329, 0.6677333]; 4], [152; 4]),
    ("asymmetric40_95", [[0.6042489, 0.7757333]; 4], [107; 4]),
    ("crossing80_20", [[0.4405458, 0.4821334]; 4], [221; 4]),
    ("outside_negative30_130", [[0.6634497, 0.9192]; 4], [80; 4]),
    ("start_flat", [[0.5077422, 0.6373333]; 4], [166; 4]),
    ("end_flat", [[0.8, 0.8]; 4], [62; 4]),
    (
        "mixed_channels",
        [
            [0.51328, 0.6464],
            [0.635976, 0.7729333],
            [0.5015404, 0.6017306],
            [0.596, 0.792],
        ],
        [162, 99, 178, 107],
    ),
    (
        "flat_rgb_alpha_fade",
        [
            [0.5286329, 0.6677333],
            [0.6042489, 0.7757333],
            [0.4405458, 0.4821334],
            [0.58, 0.76],
        ],
        [152, 107, 221, 116],
    ),
    ("flat_endpoints_value_wiggles", [[0.55, 0.7]; 4], [137; 4]),
];

fn measured_parameters(times: [f32; 3], observations: &[Observation]) {
    let [start, time, end] = times;
    let span = end - start;
    let fraction = (f64::from(time) - f64::from(start)) / (f64::from(end) - f64::from(start));
    let mut count = 0;
    for &(label, controls, expected) in observations {
        for channel in 0..4 {
            // Match the document's native normalization and reconstruct the
            // parameter from PUBLIC split control geometry, not scan internals.
            let x1 = (controls[channel][0] - start) / span;
            let x2 = (controls[channel][1] - start) / span;
            let (before, after) = color_curve_split::split(
                Curve::Bezier([x1, 0.2, x2, 0.8]),
                times,
                [0., 0.5, 1.],
                0.5,
            )
            .unwrap();
            let (Curve::Bezier(before), Curve::Bezier(after)) = (before, after) else {
                panic!("Expected normalized split Beziers");
            };
            let actual = if x1 != 0. {
                f64::from(before[0]) * fraction / f64::from(x1)
            } else {
                (fraction + f64::from(after[2]) * (1. - fraction) - f64::from(x2))
                    / (1. - f64::from(x2))
            };
            let expected = f64::from(expected[channel]) / 300.;
            assert!(
                (actual - expected).abs() < 3e-6,
                "{label} channel {channel} at {time}: {actual} vs observed {expected}"
            );
            count += 1;
        }
    }
    assert_eq!(count, 52);
}

#[test]
fn slot_color_insert_matches_52_original_native_midpoint_parameters() {
    measured_parameters([0., 0.4, 0.8], &INITIAL);
}

#[test]
fn slot_color_insert_matches_52_held_out_nonzero_origin_native_parameters() {
    measured_parameters([0.4, 0.6, 0.8], &HELD_OUT);
}
