//! Clipping cuts later draws to a polygon (checked end to end against the
//! runtime's headless renderer by the Clip Lab fixture).
use crate::clipping::ClipRegion;
use crate::*;
use peachy_document::*;

fn area(g: &ClippedGeometry) -> f64 {
    g.triangles
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| g.vertices[i as usize].map(f64::from));
            ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() / 2.
        })
        .sum()
}

const SQUARE: [[f32; 2]; 4] = [[-20., -20.], [20., -20.], [20., 20.], [-20., 20.]];
const UVS: [[f32; 2]; 4] = [[0., 1.], [1., 1.], [1., 0.], [0., 0.]];

#[test]
fn concave_polygons_cut_exactly_and_convex_uses_the_hull() {
    // An L-shaped (concave) polygon covering three of four 10×10 cells.
    let l_shape = [
        [-10., -10.],
        [10., -10.],
        [10., 0.],
        [0., 0.],
        [0., 10.],
        [-10., 10.],
    ];
    let exact = ClipRegion::new(&l_shape, false, None).clip(&SQUARE, &UVS, &[0, 1, 2, 2, 3, 0]);
    assert!((area(&exact) - 300.).abs() < 1e-3, "{}", area(&exact));
    let hull = ClipRegion::new(&l_shape, true, None).clip(&SQUARE, &UVS, &[0, 1, 2, 2, 3, 0]);
    assert!((area(&hull) - 350.).abs() < 1e-3, "{}", area(&hull));
    // UVs are interpolated from the source triangle: the clip's centre maps to
    // the image centre.
    let centre = exact.vertices.iter().position(|v| *v == [0., 0.]).unwrap();
    assert_eq!(exact.uvs[centre], [0.5, 0.5]);
}

#[test]
fn clips_run_from_their_slot_through_the_end_slot_or_to_the_end() {
    let doc: Document =
        serde_json::from_str(include_str!("../../../examples/peach-bot/clip-bot.peachy")).unwrap();
    let scene = checked_evaluate_scene(&doc, Some(0), 0.5, "peach", None).unwrap();
    let clipped: Vec<&str> = scene
        .attachments
        .iter()
        .filter(|d| d.clipped.is_some())
        .map(|d| doc.slots[d.slot].name.as_str())
        .collect();
    assert_eq!(
        clipped,
        [
            "forearm.L",
            "hand.L",
            "body",
            "head",
            "upper-arm.R",
            "forearm.R",
            "hand.R"
        ]
    );
    // Clipping slots draw nothing themselves.
    assert!(
        scene
            .attachments
            .iter()
            .all(|d| !doc.slots[d.slot].name.ends_with("-clip"))
    );
}
