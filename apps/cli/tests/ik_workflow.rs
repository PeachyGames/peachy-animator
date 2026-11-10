use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

fn folder() -> PathBuf {
    // Parallel tests can read the same clock tick; a counter keeps names unique.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "peachy-ik-cli-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    fs::create_dir(&path).unwrap();
    path
}
fn run(args: &[&Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_peachy-cli"))
        .args(args)
        .output()
        .unwrap()
}
fn success(args: &[&Path]) -> Output {
    let output = run(args);
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
fn write(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn read(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn pose(source: &Path) -> Value {
    serde_json::from_slice(
        &success(&[Path::new("pose"), source, Path::new("held"), Path::new("0")]).stdout,
    )
    .unwrap()
}
fn source() -> Value {
    json!({"skeleton":{"spine":"4.3.26"},"bones":[{"name":"root"},
        {"name":"arm","parent":"root","length":10},
        {"name":"target","parent":"root","y":10}],
        "slots":[{"name":"cloth","bone":"arm","attachment":"cloth"}],
        "skins":[{"name":"default","attachments":{"cloth":{"cloth":{
            "type":"mesh","path":"cloth","width":2,"height":2,
            "vertices":[0,0,2,0,0,2],"uvs":[0,0,1,0,0,1],"triangles":[0,1,2],"hull":3}}}}],
        "animations":{"held":{}}})
}
fn compare_vertices(a: &Value, b: &Value) {
    let points = |v: &Value| {
        v["attachments"][0]["vertices"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|p| p.as_array().unwrap().iter().map(|c| c.as_f64().unwrap()))
            .collect::<Vec<_>>()
    };
    let a = points(a);
    let b = points(b);
    assert_eq!(a.len(), b.len());
    for (a, b) in a.iter().zip(&b) {
        assert!((a - b).abs() <= 0.001, "Vertex drift: {a} / {b}");
    }
}

#[test]
fn authored_and_applied_cli_poses_feed_one_fk_binding_snapshot_without_double_solve() {
    let folder = folder();
    let input = folder.join("original.json");
    write(&input, &source());
    let original = fs::read(&input).unwrap();
    let native = folder.join("original.peachy");
    success(&[Path::new("import"), &input, &native]);
    let before_native = fs::read(&native).unwrap();
    let commands = folder.join("add.json");
    let constraint =
        json!({"name":"aim","bones":[1],"target":2,"setup":{"mix":0.5,"bend_positive":true}});
    write(
        &commands,
        &json!([{"command":"addIkConstraint","constraint":constraint}]),
    );
    let edited = folder.join("aim.peachy");
    success(&[Path::new("apply"), &native, &commands, &edited]);
    assert_eq!(fs::read(&native).unwrap(), before_native);
    let scene = pose(&edited);
    assert_eq!(scene["locals"][1]["rotation"], 0.0);
    assert!((scene["appliedLocals"][1]["rotation"].as_f64().unwrap() - 45.0).abs() < 0.0001);
    assert_eq!(read(&edited)["bones"][1]["setup"]["rotation"], 0.0);
    assert_eq!(read(&edited)["animations"][0]["tracks"], json!([]));

    let bind = folder.join("bind.json");
    write(
        &bind,
        &json!([{"command":"bindMeshToBone","bone":0,
        "attachment":{"skin":"default","slot":"cloth","placeholder":"cloth"},
        "pose":{"locals":scene["appliedLocals"]}}]),
    );
    let bound = folder.join("bound.peachy");
    success(&[Path::new("apply"), &edited, &bind, &bound]);
    compare_vertices(&scene, &pose(&bound));

    // A serialized applied snapshot is an FK pose, not authored input for IK.
    let mut snapshot = read(&edited);
    for (bone, applied) in snapshot["bones"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .zip(scene["appliedLocals"].as_array().unwrap())
    {
        bone["setup"] = applied.clone();
    }
    snapshot["constraints"] = json!([]);
    let fk = folder.join("applied-once.peachy");
    write(&fk, &snapshot);
    compare_vertices(&scene, &pose(&fk));
    let rename = folder.join("rename.json");
    write(
        &rename,
        &json!([
            {"command":"renameBone","bone":1,"name":"renamed-arm"},
            {"command":"renameBone","bone":2,"name":"renamed-target"}
        ]),
    );
    let renamed = folder.join("renamed.peachy");
    success(&[Path::new("apply"), &bound, &rename, &renamed]);
    compare_vertices(&pose(&bound), &pose(&renamed));
    let exported = folder.join("bound.json");
    let reopened = folder.join("roundtrip.peachy");
    success(&[Path::new("export"), &renamed, &exported]);
    assert_eq!(
        read(&exported)["constraints"][0]["bones"],
        json!(["renamed-arm"])
    );
    assert_eq!(
        read(&exported)["constraints"][0]["target"],
        "renamed-target"
    );
    success(&[Path::new("import"), &exported, &reopened]);
    compare_vertices(&pose(&bound), &pose(&reopened));
    assert_eq!(read(&reopened)["constraints"], read(&bound)["constraints"]);

    let invalid = folder.join("invalid-batch.json");
    let unpublished = folder.join("unpublished.peachy");
    let mut bad_constraint = constraint;
    bad_constraint["target"] = json!(99);
    write(
        &invalid,
        &json!([{"command":"setBoneLength","bone":0,"length":123},
        {"command":"setIkConstraint","constraint":0,"value":bad_constraint}]),
    );
    let before = fs::read(&edited).unwrap();
    let rejected = run(&[Path::new("apply"), &edited, &invalid, &unpublished]);
    assert!(!rejected.status.success());
    assert!(rejected.stdout.is_empty());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("valid bones and a target"));
    assert!(!unpublished.exists());
    assert_eq!(fs::read(&edited).unwrap(), before);
    assert_eq!(fs::read(&input).unwrap(), original);
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn adapter_source_preflight_and_dynamic_query_failures_remain_separate_atomic_boundaries() {
    let folder = folder();
    let mut input = source();
    input["bones"][2]["y"] = json!(0);
    input["constraints"] = json!([{"type":"ik","name":"aim","bones":["arm"],"target":"target"}]);
    let source = folder.join("coincident.json");
    write(&source, &input);
    let original = fs::read(&source).unwrap();
    let native = folder.join("coincident.peachy");
    success(&[Path::new("import"), &source, &native]);
    let exported = folder.join("coincident-export.json");
    success(&[Path::new("export"), &native, &exported]);
    let rejected = run(&[
        Path::new("pose"),
        &native,
        Path::new("held"),
        Path::new("0"),
    ]);
    assert!(!rejected.status.success());
    assert!(rejected.stdout.is_empty());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("coincident"));
    let mut raw = read(&native);
    // A nonuniformly scaled constrained bone stays outside IK's domain; ancestors may scale.
    raw["bones"][1]["setup"]["scale_x"] = json!(2);
    let unsupported = folder.join("transformed.peachy");
    write(&unsupported, &raw);
    let unpublished = folder.join("unpublished.json");
    let rejected = run(&[Path::new("export"), &unsupported, &unpublished]);
    assert!(!rejected.status.success());
    assert!(!unpublished.exists());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("equal-magnitude scale"));
    let mut alias = input;
    alias["ik"] = json!([]);
    let alias_source = folder.join("alias.json");
    write(&alias_source, &alias);
    let unpublished = folder.join("alias.peachy");
    let rejected = run(&[Path::new("import"), &alias_source, &unpublished]);
    assert!(!rejected.status.success());
    assert!(!unpublished.exists());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("Legacy root ik"));
    assert_eq!(fs::read(&source).unwrap(), original);
    fs::remove_dir_all(folder).unwrap();
}
