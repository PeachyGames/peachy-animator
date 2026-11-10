//! Skin-required bones and constraints, following observed runtime activity.
use crate::*;
use peachy_document::*;

fn skin_lab() -> Document {
    serde_json::from_str(include_str!("../../../examples/peach-bot/skin-bot.peachy")).unwrap()
}

#[test]
fn listing_activates_bones_with_their_ancestors_and_constraints_need_active_bones() {
    let doc = skin_lab();
    let badge = doc.bones.iter().position(|b| b.name == "badge").unwrap();
    for skin in ["default", "peach", "unknown"] {
        let activity = doc.activity(skin);
        assert!(!activity.bones[badge], "{skin}");
        // The required IK and the non-required transform both touch the badge.
        assert_eq!(activity.constraints, [false, false], "{skin}");
    }
    let mint = doc.activity("mint");
    assert!(mint.bones.iter().all(|&active| active));
    assert_eq!(mint.constraints, [true, true]);
    // Listing a required child activates its required parent too.
    let mut nested = doc.clone();
    nested.bones.push(Bone {
        name: "badge pin".into(),
        parent: Some(badge),
        length: 5.,
        setup: Transform::default(),
        inherit: Inherit::Normal,
    });
    let pin = nested.bones.len() - 1;
    nested.skin_scope.required_bones.insert(pin);
    nested.skin_scope.skins.get_mut("mint").unwrap().bones = vec![pin];
    nested.validate().unwrap();
    assert!(nested.activity("mint").bones[badge]);
}

#[test]
fn inactive_slots_are_not_drawn_and_the_session_skin_selects_constraints() {
    let doc = skin_lab();
    let badge_slot = doc.slots.iter().position(|s| s.name == "badge").unwrap();
    let peach = checked_evaluate_scene(&doc, Some(0), 0.5, "peach", None).unwrap();
    assert!(peach.attachments.iter().all(|d| d.slot != badge_slot));
    let mint = checked_evaluate_scene(&doc, Some(0), 0.5, "mint", None).unwrap();
    assert!(mint.attachments.iter().any(|d| d.slot == badge_slot));
    let badge = doc.bones.iter().position(|b| b.name == "badge").unwrap();
    assert!(mint.active_bones[badge] && !peach.active_bones[badge]);
    // The IK aim only runs while its skin is active in the session.
    let mut session = Session::new(doc).unwrap();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.5,
    };
    let off = session.evaluated_pose(target).unwrap().applied_locals()[badge];
    session.set_skin("mint");
    let on = session.evaluated_pose(target).unwrap().applied_locals()[badge];
    assert_ne!(off.rotation, on.rotation);
}

#[test]
fn invalid_scopes_are_refused() {
    let doc = skin_lab();
    let badge = doc.bones.iter().position(|b| b.name == "badge").unwrap();
    let mut listed_not_required = doc.clone();
    listed_not_required.skin_scope.required_bones.clear();
    assert!(listed_not_required.validate().is_err());
    let mut unknown_skin = doc.clone();
    let members = unknown_skin.skin_scope.skins.remove("mint").unwrap();
    unknown_skin
        .skin_scope
        .skins
        .insert("ghost".into(), members);
    assert!(unknown_skin.validate().is_err());
    let mut orphan = doc;
    orphan.bones.push(Bone {
        name: "plain child".into(),
        parent: Some(badge),
        length: 1.,
        setup: Transform::default(),
        inherit: Inherit::Normal,
    });
    assert!(
        orphan
            .validate()
            .unwrap_err()
            .contains("must be skin-required like its parent")
    );
}
