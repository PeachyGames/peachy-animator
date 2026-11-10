//! Editor guides must use the same evaluated attachment/skin/pose as rendering.
use crate::*;
use peachy_document::*;

fn path_lab() -> Document {
    serde_json::from_str(include_str!("../../../examples/peach-bot/path-bot.peachy")).unwrap()
}

#[test]
fn path_segments_skip_unused_handles_and_join_knots_in_order() {
    let points = [
        [-1., 0.],
        [0., 0.],
        [1., 0.],
        [2., 1.],
        [3., 1.],
        [4., 1.],
        [5., 0.],
        [6., 0.],
        [7., 0.],
    ];
    let open = path_cubics(&points, false);
    assert_eq!(
        open,
        vec![
            [points[1], points[2], points[3], points[4]],
            [points[4], points[5], points[6], points[7]]
        ]
    );
    let closed = path_cubics(&points, true);
    assert_eq!(&closed[..2], &open);
    assert_eq!(closed[2], [points[7], points[8], points[0], points[1]]);
    assert!(path_cubics(&points[..5], false).is_empty());
    assert!(path_cubics(&[], false).is_empty());
}

#[test]
fn guides_use_evaluated_deforms_for_plain_and_weighted_paths_without_textured_draws() {
    let doc = path_lab();
    let before = doc.clone();
    let scene = checked_evaluate_scene(&doc, Some(0), 1., "peach", None).unwrap();
    assert_eq!(scene.overlays.len(), 2);
    let pose = checked_pose_from_authored(
        &doc,
        &sample_authored_pose(&doc, Some(0), 1.)
            .unwrap()
            .with_skin(&doc, "peach"),
    )
    .unwrap();
    let mut weighted = false;
    for guide in &scene.overlays {
        let Attachment::Path(path) = doc.attachment(&guide.attachment).unwrap() else {
            panic!()
        };
        weighted |= matches!(path.vertices, MeshVertices::Weighted { .. });
        let offsets = pose
            .deforms
            .iter()
            .find(|(key, _)| key == &guide.attachment)
            .unwrap();
        assert_eq!(
            guide.vertices,
            deformed_world_vertices(
                &path.vertices,
                doc.slots[guide.slot].bone,
                &scene.world,
                &offsets.1
            )
        );
        assert_eq!(guide.color, path.color);
        assert!(
            scene
                .attachments
                .iter()
                .all(|draw| draw.attachment != guide.attachment)
        );
    }
    assert!(weighted);
    assert_eq!(doc, before);
}

#[test]
fn guides_resolve_active_skin_fallback_and_keyed_visibility() {
    let mut doc = path_lab();
    let key = AttachmentRef {
        skin: "default".into(),
        slot: "rail".into(),
        placeholder: "rail".into(),
    };
    let mut replacement = doc.attachment(&key).unwrap().clone();
    let Attachment::Path(path) = &mut replacement else {
        panic!()
    };
    path.color = Some([32, 64, 128, 160]);
    doc.skins
        .get_mut("mint")
        .unwrap()
        .entry("rail".into())
        .or_default()
        .insert("rail".into(), replacement);
    let mint = checked_evaluate_scene(&doc, None, 0., "mint", None).unwrap();
    let rail = mint
        .overlays
        .iter()
        .find(|g| g.attachment.slot == "rail")
        .unwrap();
    assert_eq!(rail.attachment.skin, "mint");
    assert_eq!(rail.color, Some([32, 64, 128, 160]));
    let peach = checked_evaluate_scene(&doc, None, 0., "peach", None).unwrap();
    assert_eq!(
        peach
            .overlays
            .iter()
            .find(|g| g.slot == rail.slot)
            .unwrap()
            .attachment
            .skin,
        "default"
    );
    doc.animations[0].attachment_tracks.push(AttachmentTrack {
        slot: rail.slot,
        keys: vec![AttachmentKey {
            time: 0.5,
            attachment: None,
        }],
    });
    doc.validate().unwrap();
    let hidden = checked_evaluate_scene(&doc, Some(0), 0.75, "mint", None).unwrap();
    assert!(hidden.overlays.iter().all(|g| g.slot != rail.slot));
}

#[test]
fn clipping_guides_preserve_authored_polygon_even_when_clipping_uses_convex_hull() {
    let doc: Document =
        serde_json::from_str(include_str!("../../../examples/peach-bot/clip-bot.peachy")).unwrap();
    let scene = checked_evaluate_scene(&doc, Some(0), 1.2, "peach", None).unwrap();
    assert_eq!(scene.overlays.len(), 2);
    assert!(
        scene
            .overlays
            .iter()
            .all(|g| g.kind == OverlayKind::Clipping)
    );
    let convex = scene
        .overlays
        .iter()
        .find(
            |g| matches!(doc.attachment(&g.attachment), Some(Attachment::Clipping(c)) if c.convex),
        )
        .unwrap();
    let Attachment::Clipping(clip) = doc.attachment(&convex.attachment).unwrap() else {
        panic!()
    };
    assert_eq!(convex.vertices.len(), clip.vertex_count());
    assert_eq!(
        convex.vertices,
        world_vertices(&clip.vertices, doc.slots[convex.slot].bone, &scene.world)
    );
    assert_eq!(convex.color, clip.color);
}

#[test]
fn nonfinite_guide_geometry_is_rejected_even_without_a_path_constraint() {
    let mut doc = path_lab();
    doc.constraints.clear();
    for animation in &mut doc.animations {
        animation.path_tracks.clear();
        animation.deform_tracks.clear();
    }
    let root = doc.slots[0].bone;
    doc.bones[root].setup.scale_x = 2.;
    let Attachment::Path(path) = doc
        .skins
        .get_mut("default")
        .unwrap()
        .get_mut("rail")
        .unwrap()
        .get_mut("rail")
        .unwrap()
    else {
        panic!()
    };
    let MeshVertices::SlotLocal { positions } = &mut path.vertices else {
        panic!()
    };
    positions[1][0] = f32::MAX;
    doc.validate().unwrap();
    assert!(
        checked_evaluate_scene(&doc, None, 0., "peach", None)
            .err()
            .unwrap()
            .contains("overlay geometry")
    );
}
