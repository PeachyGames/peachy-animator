//! Native v11 → v12 adds an empty skin scope; older formats cannot carry one.
use peachy_document::FORMAT_VERSION;
use peachy_io::decode_project;
use serde_json::{Value, json};

const V11: &str = include_str!("fixtures/native-v11-pre-skin-scope/peach-bot.peachy");

#[test]
fn v11_gains_an_empty_skin_scope_and_legacy_formats_refuse_one() {
    let old: Value = serde_json::from_str(V11).unwrap();
    assert_eq!(old["format"], 11);
    let migrated = decode_project(V11).unwrap();
    assert_eq!(migrated.format, FORMAT_VERSION);
    assert!(migrated.skin_scope.is_empty() && migrated.skin_scope.skins.is_empty());
    let current =
        decode_project(include_str!("../../../examples/peach-bot/peach-bot.peachy")).unwrap();
    assert_eq!(migrated, current);
    let mut smuggled = old;
    smuggled["skin_scope"] = json!({"required_bones":[],"required_constraints":[],"skins":{}});
    assert!(
        decode_project(&smuggled.to_string())
            .unwrap_err()
            .contains("cannot contain skin scope")
    );
}
