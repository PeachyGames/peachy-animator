//! Linked meshes share their source's geometry: source edits propagate, edits
//! to a link alone are refused, and deform keys follow `timelines` (checked
//! end to end against the external runtime by the Linked Mesh Lab fixture).
use crate::*;
use peachy_document::*;

fn lab() -> Document {
    serde_json::from_str(include_str!(
        "../../../examples/peach-bot/linked-mesh-bot.peachy"
    ))
    .unwrap()
}

fn key(skin: &str, slot: &str, placeholder: &str) -> AttachmentRef {
    AttachmentRef {
        skin: skin.into(),
        slot: slot.into(),
        placeholder: placeholder.into(),
    }
}

fn mesh<'a>(doc: &'a Document, key: &AttachmentRef) -> &'a Mesh {
    let Some(Attachment::Mesh(mesh)) = doc.attachment(key) else {
        panic!("mesh {key:?}")
    };
    mesh
}

#[test]
fn source_edits_reach_links_and_link_edits_are_refused() {
    let doc = lab();
    let (source, link) = (key("peach", "body", "body"), key("mint", "body", "body"));
    let edited = apply_command(
        &doc,
        Command::SetMeshVertex {
            attachment: source.clone(),
            vertex: 0,
            position: [-3., 4.],
        },
    )
    .unwrap();
    assert!(mesh(&edited, &link).same_geometry(mesh(&edited, &source)));
    assert_ne!(mesh(&edited, &link).vertices, mesh(&doc, &link).vertices);
    // The link keeps its own image and tint.
    assert_eq!(mesh(&edited, &link).path, "torso-mint");
    assert_eq!(mesh(&edited, &link).color, [200, 255, 220, 255]);
    let error = apply_command(
        &doc,
        Command::SetMeshVertex {
            attachment: link,
            vertex: 0,
            position: [-3., 4.],
        },
    )
    .unwrap_err();
    assert!(error.contains("edit the source mesh"), "{error}");
}

#[test]
fn deform_keys_follow_the_source_only_with_timelines() {
    let doc = lab();
    let (glove, mitt, hand) = (
        key("default", "hand.R", "glove"),
        key("default", "hand.R", "mitt"),
        key("default", "hand.R", "hand.R"),
    );
    assert_eq!(doc.deform_target(&glove), hand);
    assert_eq!(doc.deform_target(&mitt), mitt);
    let draw = |time: f32, wanted: &AttachmentRef| {
        checked_evaluate_scene(&doc, Some(0), time, "default", None)
            .unwrap()
            .attachments
            .into_iter()
            .find(|d| d.attachment == *wanted)
            .unwrap()
            .vertices
    };
    let undeformed = mesh_world_vertices(
        mesh(&doc, &glove),
        doc.slots
            .iter()
            .position(|s| s.name == "hand.R")
            .map(|s| doc.slots[s].bone)
            .unwrap(),
        &checked_evaluate_scene(&doc, Some(0), 1.0, "default", None)
            .unwrap()
            .world,
    );
    assert_ne!(
        draw(1.0, &glove),
        undeformed,
        "glove follows hand.R deform keys"
    );
    // A deform track naming a linked mesh that follows its source is refused.
    let mut bad = doc.clone();
    bad.animations[0].deform_tracks[0].attachment = glove;
    assert!(bad.validate().unwrap_err().contains("follows its source"));
}
