//! IK with scaled, mirrored and stretched bones (checked end to end against the
//! external runtime by the IK Scale Lab fixture).
use crate::*;
use peachy_document::*;

fn lab() -> Document {
    serde_json::from_str(include_str!(
        "../../../examples/peach-bot/ik-scale-bot.peachy"
    ))
    .unwrap()
}

fn bone(doc: &Document, name: &str) -> usize {
    doc.bones.iter().position(|b| b.name == name).unwrap()
}

#[test]
fn stretch_scales_the_first_bone_by_mix_and_flattens_the_child_offset() {
    let doc = lab();
    let pose = checked_evaluate_pose(&doc, Some(0), 0.8).unwrap();
    let (leg, shin, head) = (
        bone(&doc, "leg.R"),
        bone(&doc, "shin.R"),
        bone(&doc, "head"),
    );
    assert!(pose.applied_locals()[leg].scale_x > 2.);
    assert_eq!(pose.applied_locals()[shin].y, 0.);
    assert_eq!(pose.authored_locals()[shin].y, 6.);
    assert!(pose.applied_locals()[head].scale_x > 1.);
    // Without stretch nothing scales and the child keeps its offset.
    let mut plain = doc.clone();
    for constraint in &mut plain.constraints {
        if let Constraint::Ik(ik) = constraint {
            ik.stretch = false;
        }
    }
    let pose = checked_evaluate_pose(&plain, Some(0), 0.8).unwrap();
    assert_eq!(pose.applied_locals()[shin].y, 6.);
    assert_eq!(pose.applied_locals()[head].scale_x, 1.);
}

#[test]
fn mirrored_first_bones_solve_and_nonuniform_scale_is_refused() {
    // The runtime's mirrored-chain result (not "tip on target") is checked by
    // the IK Scale Lab against the external runtime; here, the chain is solved.
    let doc = lab();
    let pose = checked_evaluate_pose(&doc, Some(0), 1.2).unwrap();
    let arm = bone(&doc, "arm.L");
    assert_ne!(
        pose.applied_locals()[arm].rotation,
        pose.authored_locals()[arm].rotation
    );
    assert_eq!(pose.applied_locals()[arm].scale_x, -1.);
    let mut uneven = doc.clone();
    uneven.bones[arm].setup.scale_y = 2.;
    assert!(
        checked_evaluate_pose(&uneven, None, 0.)
            .unwrap_err()
            .contains("equal-magnitude")
    );
}
