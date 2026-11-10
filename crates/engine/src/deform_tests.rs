//! Deform sampling and its effect on mesh geometry (checked end to end against
//! the external runtime by the Deform Lab fixture).
use crate::*;
use peachy_document::*;

fn lab() -> Document {
    serde_json::from_str(include_str!(
        "../../../examples/peach-bot/deform-bot.peachy"
    ))
    .unwrap()
}

#[test]
fn sampling_holds_setup_before_keys_interpolates_curves_and_holds_the_last_key() {
    let doc = lab();
    let track = doc.animations[0].deform_tracks[0].clone();
    let at = |time| crate::ik::sample_deform(&track, time);
    let mut late = track.clone();
    late.keys[0].time = 0.2;
    assert!(crate::ik::sample_deform(&late, 0.1).is_none());
    assert_eq!(at(0.6).unwrap(), track.keys[1].offsets);
    // Stepped holds the left key until the next key time.
    assert_eq!(at(1.5).unwrap(), track.keys[2].offsets);
    assert_eq!(at(9.).unwrap(), track.keys.last().unwrap().offsets);
    let mid = at(2.1).unwrap();
    for ((m, a), b) in mid
        .iter()
        .zip(&track.keys[3].offsets)
        .zip(&track.keys[4].offsets)
    {
        assert!((m - (a + (b - a) * 0.5)).abs() < 1e-4);
    }
}

#[test]
fn deform_moves_weighted_and_unweighted_geometry_and_slot_renames_keep_it() {
    let doc = lab();
    let mut bare = doc.clone();
    bare.animations[0].deform_tracks.clear();
    let moved = checked_evaluate_scene(&doc, Some(0), 0.6, "peach", None).unwrap();
    let still = checked_evaluate_scene(&bare, Some(0), 0.6, "peach", None).unwrap();
    for slot in ["hand.R", "body"] {
        let index = doc.slots.iter().position(|s| s.name == slot).unwrap();
        let vertices = |scene: &Scene| {
            scene
                .attachments
                .iter()
                .find(|d| d.slot == index)
                .unwrap()
                .vertices
                .clone()
        };
        assert_ne!(vertices(&moved), vertices(&still), "{slot}");
    }
    let body = doc.slots.iter().position(|s| s.name == "body").unwrap();
    let renamed = apply_command(
        &doc,
        Command::RenameSlot {
            slot: body,
            name: "torso".into(),
        },
    )
    .unwrap();
    assert!(
        renamed.animations[0]
            .deform_tracks
            .iter()
            .any(|t| t.attachment.slot == "torso")
    );
    // Changing a keyed mesh's vertex count is refused instead of corrupting keys.
    let key = AttachmentRef {
        skin: "peach".into(),
        slot: "body".into(),
        placeholder: "body".into(),
    };
    assert!(
        apply_command(
            &doc,
            Command::SplitMeshTriangle {
                attachment: key,
                triangle: 0
            }
        )
        .is_err()
    );
}
