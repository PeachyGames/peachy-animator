//! Bone Inherit checkboxes drive one Setup command each; Animate is read-only.
use super::test_support::{editor_context, editor_fixture, workspace_frame};
use super::*;
use peachy_document::Inherit;

fn rect(editor: &Editor, target: &str) -> egui::Rect {
    let [left, top, right, bottom] = *editor.capture_targets.get(target).unwrap_or_else(|| {
        panic!(
            "Missing UI target {target}; available: {:?}",
            editor.capture_targets.keys().collect::<Vec<_>>()
        )
    });
    egui::Rect::from_min_max(pos2(left, top), pos2(right, bottom))
}

fn click(editor: &mut Editor, ctx: &egui::Context, target: &str) {
    let pos = rect(editor, target).center();
    for pressed in [true, false] {
        workspace_frame(
            editor,
            ctx,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    workspace_frame(editor, ctx, vec![]);
}

#[test]
fn inherit_checkboxes_map_to_modes_with_one_undo_step_each() {
    let mut editor = editor_fixture();
    editor.setup_mode = true;
    editor.select(TreeItem::Bone(2));
    let original = editor.session.document().clone();
    let ctx = editor_context();
    workspace_frame(&mut editor, &ctx, vec![]);
    let clip = rect(&editor, "properties");
    for target in [
        "boneInheritRotation",
        "boneInheritScale",
        "boneInheritReflection",
    ] {
        assert!(clip.contains_rect(rect(&editor, target)), "{target}");
    }
    let mode = |editor: &Editor| editor.session.document().bones[2].inherit;
    click(&mut editor, &ctx, "boneInheritScale");
    assert_eq!(mode(&editor), Inherit::NoScale);
    click(&mut editor, &ctx, "boneInheritReflection");
    assert_eq!(mode(&editor), Inherit::NoScaleOrReflection);
    click(&mut editor, &ctx, "boneInheritRotation");
    assert_eq!(mode(&editor), Inherit::OnlyTranslation);
    click(&mut editor, &ctx, "boneInheritScale");
    assert_eq!(mode(&editor), Inherit::NoRotationOrReflection);
    for expected in [
        Inherit::OnlyTranslation,
        Inherit::NoScaleOrReflection,
        Inherit::NoScale,
        Inherit::Normal,
    ] {
        editor.undo();
        assert_eq!(mode(&editor), expected);
    }
    assert_eq!(editor.session.document(), &original);
    // Animate shows the mode but cannot change it.
    editor.set_setup_mode(false);
    workspace_frame(&mut editor, &ctx, vec![]);
    click(&mut editor, &ctx, "boneInheritScale");
    assert_eq!(editor.session.document(), &original);
}
