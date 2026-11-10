//! Path constraints (checked end to end against the external runtime by the
//! Path Lab fixture).
use crate::*;
use peachy_document::*;

fn lab() -> Document {
    serde_json::from_str(include_str!("../../../examples/peach-bot/path-bot.peachy")).unwrap()
}

#[test]
fn keyed_position_moves_the_bone_and_hidden_paths_leave_bones_alone() {
    let doc = lab();
    let hand = doc.bones.iter().position(|b| b.name == "hand.L").unwrap();
    let at = |doc: &Document, time| {
        let m = checked_evaluate_pose(doc, Some(0), time).unwrap().world()[hand];
        [m.a, m.b, m.c, m.d, m.x, m.y]
    };
    assert_ne!(at(&doc, 0.3)[4], at(&doc, 0.9)[4]);
    // A slot not showing a path attachment leaves the constrained bones alone.
    let mut hidden = doc.clone();
    hidden.slots[0].attachment = None;
    let mut unconstrained = doc.clone();
    unconstrained.constraints.remove(0);
    for animation in &mut unconstrained.animations {
        animation.path_tracks.retain(|t| t.constraint == 1);
        for track in &mut animation.path_tracks {
            track.constraint = 0;
        }
    }
    assert_eq!(at(&hidden, 0.9), at(&unconstrained, 0.9));
}

#[test]
fn closed_paths_fail_explicitly() {
    let mut doc = lab();
    let Attachment::Path(path) = doc
        .skins
        .get_mut("default")
        .unwrap()
        .get_mut("rail")
        .unwrap()
        .get_mut("rail")
        .unwrap()
    else {
        unreachable!()
    };
    path.closed = true;
    assert!(
        checked_evaluate_pose(&doc, None, 0.)
            .unwrap_err()
            .contains("closed paths")
    );
}

#[test]
fn deform_keys_reshape_the_path_bones_follow() {
    let doc = lab();
    let mut flat = doc.clone();
    for animation in &mut flat.animations {
        animation.deform_tracks.clear();
    }
    let hand = doc.bones.iter().position(|b| b.name == "hand.L").unwrap();
    let at = |doc: &Document| checked_evaluate_pose(doc, Some(0), 1.0).unwrap().world()[hand].x;
    assert!(
        (at(&doc) - at(&flat)).abs() > 1.,
        "deformed rail moves the hand"
    );
}
