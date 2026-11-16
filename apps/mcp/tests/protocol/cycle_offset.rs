use super::*;

fn fixture(root: &Path) -> Vec<u8> {
    use peachy_document::{
        Animation, Bone, Curve, Document, Inherit, Keyframe, Property, Track, Transform,
    };
    let mut document = Document::new("Original complete-track Offset lab".into(), 60).unwrap();
    for name in ["driver", "follower"] {
        document.bones.push(Bone {
            name: name.into(),
            parent: Some(0),
            length: 60.,
            setup: Transform::default(),
            inherit: Inherit::Normal,
        });
    }
    let track = |bone, property, points: &[(f32, f32)]| Track {
        bone,
        property,
        keys: points
            .iter()
            .map(|&(time, value)| Keyframe {
                time,
                value,
                curve: Curve::Linear,
            })
            .collect(),
    };
    document.animations.push(Animation::new(
        "closed",
        vec![
            track(
                1,
                Property::Y,
                &[(0., 0.), (0.25, 10.), (0.5, -10.), (1., 0.)],
            ),
            track(
                2,
                Property::Rotation,
                &[(0., 5.), (0.25, 25.), (0.75, -5.), (1., 5.)],
            ),
            track(0, Property::Rotation, &[(0., 3.), (1., 9.)]),
        ],
    ));
    document
        .animations
        .push(Animation::new("untouched", vec![]));
    peachy_io::create_project(&root.join("offset.peachy"), &document).unwrap();
    fs::read(root.join("offset.peachy")).unwrap()
}

fn selection(document: &Value, bones: &[usize]) -> Value {
    let mut keys = Vec::new();
    for bone in bones {
        for track in document["animations"][0]["tracks"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|track| track["bone"] == *bone)
        {
            for key in track["keys"].as_array().unwrap() {
                keys.push(json!({"type":"bone","bone":bone,"property":track["property"],"time":key["time"]}));
            }
        }
    }
    json!(keys)
}

async fn mutate(client: &RunningService<RoleClient, ()>, name: &str, mut args: Value) -> Value {
    args["expected_revision"] = doc(client).await["state"]["revision"].clone();
    success(call(client, name, args).await)
}

async fn offset(client: &RunningService<RoleClient, ()>, bones: &[usize], delta: f32) -> Value {
    let current = doc(client).await;
    mutate(client,"offset_animation_keys",json!({"animation":0,"keys":selection(&current["document"],bones),"range":{"start":0,"end":1},"delta":delta})).await
}

async fn rejected(client: &RunningService<RoleClient, ()>, tool: &str, args: Value) {
    let before = doc(client).await;
    let result = client
        .call_tool(
            CallToolRequestParams::new(tool.to_owned())
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await;
    assert!(
        result.is_err()
            || result
                .as_ref()
                .is_ok_and(|result| result.is_error == Some(true)),
        "Accepted {args}"
    );
    assert_eq!(
        doc(client).await,
        before,
        "Rejected request changed authoring state/revision/memory presence"
    );
}

fn phases(receipt: &Value) -> Vec<f64> {
    receipt["phases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|phase| phase["phase"].as_f64().unwrap())
        .collect()
}

fn assert_phases(receipt: &Value, expected: &[f64]) {
    let actual = phases(receipt);
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 0.0000001,
            "{actual} != {expected}"
        );
    }
}

#[tokio::test]
async fn stdio_cycle_offset_track_cohorts_reselection_pending_undo_and_reopen() {
    let root = Sandbox::new();
    let source = fixture(&root.0);
    let client = start(&root.0).await;
    let tools = client.list_tools(Default::default()).await.unwrap().tools;
    assert_eq!(tools.len(), 25);
    for name in ["offset_animation_keys", "reset_cycle_offset_memory"] {
        let tool = tools.iter().find(|tool| tool.name == name).unwrap();
        assert_eq!(tool.input_schema["additionalProperties"], false);
    }
    success(
        call(
            &client,
            "open_project",
            json!({"expected_revision":0,"path":"offset.peachy"}),
        )
        .await,
    );
    let baseline = doc(&client).await;
    assert_eq!(baseline["state"]["hasCycleOffsetMemory"], false);
    let transform =
        json!({"x":17.,"y":0.,"rotation":0.,"scale_x":1.,"scale_y":1.,"shear_x":0.,"shear_y":0.});
    mutate(
        &client,
        "edit_bone_poses",
        json!({"target":{"mode":"animate","animation":0,"time":0.5},
        "edits":[{"bone":0,"transform":transform,"properties":["x"]}],"auto_key":false}),
    )
    .await;
    let pending = doc(&client).await;
    let first = offset(&client, &[1], 0.1).await;
    assert_phases(&first, &[0.1]);
    assert_eq!(first["boundaryKeys"].as_array().unwrap().len(), 2);
    assert_eq!(first["state"]["hasCycleOffsetMemory"], true);
    let first_state = doc(&client).await;
    assert_eq!(
        first_state["state"]["pendingPose"],
        pending["state"]["pendingPose"]
    );
    mutate(&client, "history", json!({"action":"undo"})).await;
    let undone = doc(&client).await;
    assert_eq!(undone["document"], baseline["document"]);
    assert_eq!(undone["state"]["hasCycleOffsetMemory"], false);
    assert_eq!(
        undone["state"]["pendingPose"],
        pending["state"]["pendingPose"]
    );
    mutate(&client, "history", json!({"action":"redo"})).await;
    assert_eq!(doc(&client).await["document"], first_state["document"]);
    // Native Driver+6 then both+6 changes the affected track cohort. Recapture
    // the CURRENT driver curve, retaining its prior seam as ordinary source data.
    let both = offset(&client, &[1, 2], 0.1).await;
    assert_phases(&both, &[0.1, 0.1]);
    let changed = doc(&client).await;
    assert_eq!(
        changed["document"]["animations"][0]["tracks"][0]["keys"]
            .as_array()
            .unwrap()
            .len(),
        6
    );
    assert_eq!(
        changed["document"]["animations"][0]["tracks"][1]["keys"]
            .as_array()
            .unwrap()
            .len(),
        5
    );
    assert_eq!(
        first_state["document"]["animations"][0]["tracks"][1],
        baseline["document"]["animations"][0]["tracks"][1]
    );
    // Different seed count/order within the SAME cohort must preserve origins.
    let all_reversed = selection(&changed["document"], &[2, 1]);
    let seeds = all_reversed.as_array().unwrap();
    let one_per_track = json!([seeds[0], seeds.last().unwrap()]);
    let reselected = mutate(
        &client,
        "offset_animation_keys",
        json!({"animation":0,"keys":one_per_track,
        "range":{"start":0,"end":1},"delta":0}),
    )
    .await;
    assert_phases(&reselected, &[0.1, 0.1]);
    assert_eq!(doc(&client).await["document"], changed["document"]);
    let continued = offset(&client, &[2, 1], 0.1).await;
    assert_phases(&continued, &[0.2, 0.2]);
    assert_eq!(continued["phases"][0]["source"]["bone"], 2);
    assert_eq!(continued["phases"][1]["source"]["bone"], 1);
    assert_eq!(
        continued["phases"][0]["range"],
        json!({"start":0.,"end":1.})
    );
    let final_state = doc(&client).await;
    assert_eq!(
        final_state["document"]["animations"][1],
        baseline["document"]["animations"][1]
    );
    assert_eq!(
        final_state["document"]["animations"][0]["tracks"][2],
        baseline["document"]["animations"][0]["tracks"][2]
    );
    assert_eq!(
        final_state["state"]["pendingPose"],
        pending["state"]["pendingPose"]
    );
    for (track, maximum) in final_state["document"]["animations"][0]["tracks"]
        .as_array()
        .unwrap()
        .iter()
        .take(2)
        .zip([6, 5])
    {
        assert!(
            track["keys"].as_array().unwrap().len() <= maximum,
            "Repeated edits within one cohort accumulated new seam keys"
        );
    }
    mutate(&client, "save_project", json!({"directory":"offset-saved"})).await;
    let saved = doc(&client).await;
    assert_eq!(saved["state"]["hasCycleOffsetMemory"], true);
    assert_eq!(
        saved["state"]["pendingPose"],
        pending["state"]["pendingPose"]
    );
    mutate(
        &client,
        "open_project",
        json!({"path":"offset-saved/project.peachy","discard_changes":true}),
    )
    .await;
    let reopened = doc(&client).await;
    assert_eq!(reopened["document"], final_state["document"]);
    assert_eq!(reopened["state"]["hasCycleOffsetMemory"], false);
    assert_eq!(reopened["state"]["hasPendingEdits"], false);
    assert_eq!(reopened["state"]["canUndo"], false);
    assert_eq!(reopened["state"]["canRedo"], false);
    assert_eq!(fs::read(root.0.join("offset.peachy")).unwrap(), source);
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn stdio_cycle_offset_stale_wire_atomicity_and_explicit_reset_preserve_redo() {
    let root = Sandbox::new();
    fixture(&root.0);
    let client = start(&root.0).await;
    success(
        call(
            &client,
            "open_project",
            json!({"expected_revision":0,"path":"offset.peachy"}),
        )
        .await,
    );
    offset(&client, &[1, 2], 0.125).await;
    mutate(
        &client,
        "apply_commands",
        json!({"commands":[{"command":"renameAnimation","animation":1,"name":"redo marker"}]}),
    )
    .await;
    mutate(&client, "history", json!({"action":"undo"})).await;
    let state = doc(&client).await;
    assert_eq!(state["state"]["canRedo"], true);
    let valid = json!({"expected_revision":state["state"]["revision"],"animation":0,
        "keys":selection(&state["document"],&[1,2]),"range":{"start":0,"end":1},"delta":0.125});
    let key = valid["keys"][0].clone();
    for (field, value) in [
        ("expected_revision", json!(0)),
        ("keys", json!([])),
        ("keys", json!([key.clone(), key.clone()])),
        (
            "keys",
            json!([key.clone(),{"type":"bone","bone":1,"property":"y","time":0.123456789}]),
        ),
        ("keys", json!([{"type":"drawOrder","time":0}])),
        (
            "keys",
            json!([{"type":"bone","bone":1,"property":{"y":null},"time":0}]),
        ),
        ("keys", json!([["bone", 1, "y", 0]])),
        ("keys", Value::Null),
        ("range", json!([0, 1])),
        ("range", json!({"start":0,"end":0})),
        ("range", json!({"start":0.1,"end":1})),
        ("range", json!({"start":0,"end":1,"extra":1})),
        ("delta", json!(1e100)),
        ("delta", Value::Null),
        ("extra", json!(true)),
    ] {
        let mut candidate = valid.clone();
        candidate[field] = value;
        rejected(&client, "offset_animation_keys", candidate).await;
    }
    rejected(
        &client,
        "reset_cycle_offset_memory",
        json!({"expected_revision":0}),
    )
    .await;
    rejected(
        &client,
        "reset_cycle_offset_memory",
        json!({"expected_revision":state["state"]["revision"],"range":{}}),
    )
    .await;
    // Installed single-key selection shifts its WHOLE property track. The
    // other owner's track and every unrelated animation stay untouched.
    let whole = mutate(
        &client,
        "offset_animation_keys",
        json!({"animation":0,"keys":[key],
        "range":{"start":0,"end":1},"delta":0.125}),
    )
    .await;
    // Shrinking to Driver preserves Driver's existing original curve and phase.
    assert_eq!(phases(&whole), vec![0.25]);
    assert!(whole["keys"].as_array().unwrap().len() > 1);
    assert!(
        whole["keys"]
            .as_array()
            .unwrap()
            .iter()
            .all(|key| key["bone"] == 1 && key["property"] == "y")
    );
    let single_seed = doc(&client).await;
    assert_ne!(
        single_seed["document"]["animations"][0]["tracks"][0],
        state["document"]["animations"][0]["tracks"][0]
    );
    assert_eq!(
        single_seed["document"]["animations"][0]["tracks"][1],
        state["document"]["animations"][0]["tracks"][1]
    );
    assert_eq!(
        single_seed["document"]["animations"][1],
        state["document"]["animations"][1]
    );
    // Follower departed on shrink. Its return is a new TrackId, so BOTH current
    // tracks are recaptured, rather than reviving Follower's old origin/phase.
    let returned = offset(&client, &[1, 2], 0.125).await;
    assert_eq!(phases(&returned), vec![0.125, 0.125]);
    mutate(&client, "history", json!({"action":"undo"})).await;
    assert_eq!(doc(&client).await["document"], single_seed["document"]);
    mutate(&client, "history", json!({"action":"undo"})).await;
    assert_eq!(doc(&client).await["document"], state["document"]);
    // Rename an affected owner: every key still exists, but its remembered context is stale.
    mutate(
        &client,
        "apply_commands",
        json!({"commands":[{"command":"renameBone","bone":1,"name":"changed source identity"}]}),
    )
    .await;
    let stale = doc(&client).await;
    rejected(
        &client,
        "offset_animation_keys",
        json!({"expected_revision":stale["state"]["revision"],"animation":0,
        "keys":selection(&stale["document"],&[2]),"range":{"start":0,"end":1},"delta":0.125}),
    )
    .await;
    mutate(&client, "history", json!({"action":"undo"})).await;
    let before_reset = doc(&client).await;
    assert_eq!(before_reset["state"]["canRedo"], true);
    let reset = mutate(&client, "reset_cycle_offset_memory", json!({})).await;
    assert_eq!(reset["hasCycleOffsetMemory"], false);
    let after_reset = doc(&client).await;
    assert_eq!(after_reset["document"], before_reset["document"]);
    for field in [
        "pendingPose",
        "hasPendingEdits",
        "canUndo",
        "canRedo",
        "dirty",
    ] {
        assert_eq!(after_reset["state"][field], before_reset["state"][field]);
    }
    assert_eq!(
        after_reset["state"]["revision"].as_u64().unwrap(),
        before_reset["state"]["revision"].as_u64().unwrap() + 1
    );
    // Explicit reset permits a fresh current-track cohort after stale validation.
    let recaptured = offset(&client, &[1, 2], 0.125).await;
    assert_eq!(phases(&recaptured), vec![0.125, 0.125]);
    mutate(&client, "history", json!({"action":"undo"})).await;
    assert_eq!(doc(&client).await["state"]["hasCycleOffsetMemory"], false);
    mutate(
        &client,
        "create_project",
        json!({"name":"New empty cache","fps":60,"asset_root":".","discard_changes":true}),
    )
    .await;
    assert_eq!(doc(&client).await["state"]["hasCycleOffsetMemory"], false);
    client.cancel().await.unwrap();
}
