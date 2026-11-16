use crate::*;
use peachy_document::*;

fn session() -> Session {
    let mut document = Document::new("Original generation fixture".into(), 30).unwrap();
    document.animations.push(Animation::new("host", vec![]));
    Session::new(document).unwrap()
}
fn rename(name: &str) -> Command {
    Command::RenameAnimation {
        animation: 0,
        name: name.into(),
    }
}

#[test]
fn document_generation_tracks_publication_preview_restoration_and_history_only() {
    let mut session = session();
    let initial = session.document_generation();
    session.dispatch(rename("host")).unwrap();
    assert_eq!(session.document_generation(), initial);
    session.dispatch(rename("accepted")).unwrap();
    let accepted = session.document_generation();
    assert_ne!(accepted, initial);
    session.begin_transaction();
    session.preview_transaction(rename("preview")).unwrap();
    let preview = session.document_generation();
    assert_ne!(preview, accepted);
    session.preview_transaction(rename("preview")).unwrap();
    assert_eq!(session.document_generation(), preview);
    session.preview_transaction(rename("accepted")).unwrap();
    assert_ne!(session.document_generation(), preview);
    let restored = session.document_generation();
    session.cancel_transaction();
    assert_eq!(session.document_generation(), restored);
    session.begin_transaction();
    session.preview_transaction(rename("cancel me")).unwrap();
    let changed = session.document_generation();
    session.cancel_transaction();
    assert_ne!(session.document_generation(), changed);
    let canceled = session.document_generation();
    session.undo();
    assert_ne!(session.document_generation(), canceled);
    let undone = session.document_generation();
    session.redo();
    assert_ne!(session.document_generation(), undone);
    let redone = session.document_generation();
    session.mark_saved();
    session.set_skin("default");
    assert_eq!(session.document_generation(), redone);
    assert!(
        session
            .dispatch(Command::RenameAnimation {
                animation: 999,
                name: "bad".into()
            })
            .is_err()
    );
    assert_eq!(session.document_generation(), redone);
}

#[test]
fn document_generation_pending_only_preserves_token_but_autokey_advances_it() {
    let mut session = session();
    let target = EditTarget::Animate {
        animation: 0,
        time: 0.5,
    };
    let start = session.document_generation();
    session
        .edit_pose(
            0,
            target,
            Transform {
                x: 7.,
                ..Transform::default()
            },
            &[Property::X],
            false,
        )
        .unwrap();
    assert_eq!(session.document_generation(), start);
    session.undo();
    session.redo();
    assert_eq!(session.document_generation(), start);
    session.begin_transaction();
    session
        .preview_edit_pose(
            0,
            target,
            Transform {
                y: 9.,
                ..Transform::default()
            },
            &[Property::Y],
            false,
        )
        .unwrap();
    session.cancel_transaction();
    assert_eq!(session.document_generation(), start);
    session
        .edit_pose(
            0,
            target,
            Transform {
                x: 11.,
                ..Transform::default()
            },
            &[Property::X],
            true,
        )
        .unwrap();
    assert_ne!(session.document_generation(), start);
    let keyed = session.document_generation();
    session.begin_transaction();
    session
        .preview_edit_pose(
            0,
            target,
            Transform {
                y: 13.,
                ..Transform::default()
            },
            &[Property::Y],
            true,
        )
        .unwrap();
    assert_ne!(session.document_generation(), keyed);
    let preview = session.document_generation();
    session.cancel_transaction();
    assert_ne!(session.document_generation(), preview);
}
