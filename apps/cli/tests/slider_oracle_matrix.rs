//! Opt-in comparison against retained original public-API observations.
//!
//! Root alone runs this during the coordinated Slider acceptance lane:
//! PEACHY_SLIDER_ORACLE_REPORT=/absolute/private/report.json \
//! PEACHY_SLIDER_MATRIX_OUTPUT=/absolute/private/matrix.json \
//! cargo test -p peachy-cli --test slider_oracle_matrix -j2 -- --ignored --test-threads=2
//!
//! No oracle/runtime is loaded, no process is spawned, and input files are read-only.
//! An incomplete matrix is reported as incomplete, even when all refusals are
//! expected. Set PEACHY_SLIDER_MATRIX_REQUIRE_COMPLETE=1 for the final full gate.
//! Optional PEACHY_SLIDER_ORACLE_REFUSALS names a JSON object mapping case IDs
//! to {"stage":"import|override|evaluate","errorContains":"specific reason"}.
//! Capability errors explicitly saying "awaits calibrated"/"awaits calibration"
//! are separately counted by default. Other errors require that exact case policy.
use peachy_document::{
    Constraint, Curve, Document, Inherit, Keyframe, MeshVertices, PathProperty, PathTrack,
    Property, Transform,
};
use peachy_engine::{EditTarget, EvaluatedPose, IkPoseEdit, Session, SliderPoseEdit};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const TOLERANCE: f64 = 0.001;
const BONE_FIELDS: [(&str, Property); 7] = [
    ("x", Property::X),
    ("y", Property::Y),
    ("rotation", Property::Rotation),
    ("scaleX", Property::ScaleX),
    ("scaleY", Property::ScaleY),
    ("shearX", Property::ShearX),
    ("shearY", Property::ShearY),
];

// JSON.stringify erases signed zero in the observer's case metadata. When
// an original case file is supplied, verify every semantic input is identical
// (apart from that zero sign) before replaying its exact original numeric bits.
fn canonical_case(value: &Value) -> Value {
    match value {
        Value::Number(number) => {
            let number = number.as_f64().expect("Finite case number");
            json!(if number == 0. { 0. } else { number })
        }
        Value::Array(values) => Value::Array(values.iter().map(canonical_case).collect()),
        Value::Object(values) => Value::Object(
            values
                .iter()
                .map(|(key, value)| (key.clone(), canonical_case(value)))
                .collect(),
        ),
        other => other.clone(),
    }
}

#[derive(Default)]
struct Comparison {
    numbers: usize,
    discrete: usize,
    max_error: f64,
    mismatch_count: usize,
    mismatches: Vec<String>,
}

impl Comparison {
    fn mismatch(&mut self, message: String) {
        self.mismatch_count += 1;
        if self.mismatches.len() < 32 {
            self.mismatches.push(message);
        }
    }
    fn near(&mut self, label: &str, actual: f64, expected: &Value) {
        self.numbers += 1;
        let Some(expected) = expected.as_f64().filter(|value| value.is_finite()) else {
            self.mismatch(format!("{label}: missing finite observation {expected}"));
            return;
        };
        if !actual.is_finite() {
            self.mismatch(format!("{label}: engine returned nonfinite {actual}"));
            return;
        }
        let error = (actual - expected).abs();
        self.max_error = self.max_error.max(error);
        if error > TOLERANCE {
            self.mismatch(format!(
                "{label}: engine={actual:?}, oracle={expected:?}, absError={error:?}"
            ));
        }
    }
    fn exact(&mut self, label: &str, actual: Value, expected: &Value) {
        self.discrete += 1;
        if &actual != expected {
            self.mismatch(format!("{label}: engine={actual}, oracle={expected}"));
        }
    }
    fn array(&mut self, label: &str, actual: &[f64], expected: &Value) {
        let Some(expected) = expected.as_array() else {
            self.mismatch(format!("{label}: missing array"));
            return;
        };
        self.exact(
            &format!("{label}.len"),
            json!(actual.len()),
            &json!(expected.len()),
        );
        for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            self.near(&format!("{label}[{index}]"), *actual, expected);
        }
    }
}

fn read_json(path: &Path) -> Result<Value, String> {
    serde_json::from_slice(&fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?)
        .map_err(|e| format!("{}: {e}", path.display()))
}

fn finite32(value: &Value, label: &str, conversions: &mut Vec<Value>) -> Result<f32, String> {
    let raw = value
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("{label}: expected finite number"))?;
    let stored = raw as f32;
    if !stored.is_finite() {
        return Err(format!("{label}: value is outside native f32 storage"));
    }
    if f64::from(stored) != raw
        || (stored == 0. && stored.is_sign_negative() != raw.is_sign_negative())
    {
        conversions.push(json!({"field":label,"input":raw,"stored":stored}));
    }
    Ok(stored)
}

fn patch_slider_input(input: &mut Value, case: &Value) -> Result<(), String> {
    let allowed = [
        "animation",
        "bone",
        "property",
        "time",
        "mix",
        "local",
        "loop",
        "additive",
        "from",
        "to",
        "scale",
        "max",
        "skin",
    ];
    for group in ["sliderData", "omitSliderData"] {
        let Some(overrides) = case.get(group) else {
            continue;
        };
        for (name, edits) in overrides
            .as_object()
            .ok_or("Slider overrides must be objects")?
        {
            let constraints = input["constraints"]
                .as_array_mut()
                .ok_or("Missing unified constraints array")?;
            let indices: Vec<_> = constraints
                .iter()
                .enumerate()
                .filter_map(|(i, c)| {
                    (c["type"] == "slider" && c["name"] == name.as_str()).then_some(i)
                })
                .collect();
            if indices.len() != 1 {
                return Err(format!("Need exactly one raw Slider named {name}"));
            }
            let target = constraints[indices[0]]
                .as_object_mut()
                .ok_or("Invalid Slider object")?;
            if group == "sliderData" {
                for (field, value) in edits
                    .as_object()
                    .ok_or("Slider data patch must be an object")?
                {
                    if !allowed.contains(&field.as_str()) || value.is_array() || value.is_object() {
                        return Err(format!("Unsupported raw Slider patch {name}/{field}"));
                    }
                    target.insert(field.clone(), value.clone());
                }
            } else {
                for field in edits.as_array().ok_or("Omitted fields must be an array")? {
                    let field = field.as_str().ok_or("Omitted field must be a string")?;
                    if !allowed.contains(&field) {
                        return Err(format!("Unknown omitted field {field}"));
                    }
                    target.remove(field);
                }
            }
        }
    }
    Ok(())
}

fn target_for(
    document: &Document,
    case: &Value,
    conversions: &mut Vec<Value>,
) -> Result<EditTarget, String> {
    let Some(name) = case.get("animation").filter(|name| !name.is_null()) else {
        return Ok(EditTarget::Setup);
    };
    let name = name
        .as_str()
        .ok_or("Host animation must be a string or null")?;
    let animation = document
        .animations
        .iter()
        .position(|clip| clip.name == name)
        .ok_or_else(|| format!("Unknown host animation {name}"))?;
    if case["loop"].as_bool() == Some(true) {
        return Err("Host AnimationState looping has no public Session playback query in this harness; not substituting Slider Loop".into());
    }
    let raw = case.get("time").cloned().unwrap_or(json!(0));
    let time = finite32(&raw, "hostTime", conversions)?;
    if time < 0. {
        return Err("Host time must be nonnegative".into());
    }
    Ok(EditTarget::Animate { animation, time })
}

/// At the exact host cursor a typed absolute key is the supported equivalent of
/// the observer's incoming Path override. Only host tracks change; no Setup or
/// referenced animation is used as a substitute for the incoming pose.
fn prepare_host_path_keys(
    doc: &mut Document,
    case: &Value,
    target: EditTarget,
    conversions: &mut Vec<Value>,
) -> Result<Vec<Value>, String> {
    let original = doc.clone();
    let mut prepared = Vec::new();
    if let Some(overrides) = case["constraintPose"].as_object() {
        for (name, fields) in overrides {
            let indices: Vec<_> = doc
                .constraints
                .iter()
                .enumerate()
                .filter_map(|(i, c)| (c.name() == name).then_some(i))
                .collect();
            if indices.len() != 1 {
                return Err(format!("Need exactly one constraint pose target {name}"));
            }
            let constraint = indices[0];
            if doc.constraints[constraint].as_path().is_none() {
                continue;
            }
            let EditTarget::Animate { animation, time } = target else {
                return Err("Path incoming override has no public Setup-pose capsule API".into());
            };
            if doc.constraints.iter().any(|c| {
                c.as_slider()
                    .is_some_and(|slider| slider.animation == animation)
            }) {
                return Err("Path host-key preparation would change a referenced animation; no public pending API substitutes for that case".into());
            }
            for (field, value) in fields
                .as_object()
                .ok_or("Path pose override must be an object")?
            {
                let property = match field.as_str() {
                    "position" => PathProperty::Position,
                    "spacing" => PathProperty::Spacing,
                    _ => {
                        return Err(format!(
                            "Path pose field {field} has no public authoring representation"
                        ));
                    }
                };
                let value = finite32(
                    value,
                    &format!("constraintPose.{name}.{field}"),
                    conversions,
                )?;
                let clip = &mut doc.animations[animation];
                let index = clip
                    .path_tracks
                    .iter()
                    .position(|track| track.constraint == constraint && track.property == property)
                    .unwrap_or_else(|| {
                        clip.path_tracks.push(PathTrack {
                            constraint,
                            property,
                            keys: vec![],
                        });
                        clip.path_tracks.len() - 1
                    });
                let keys = &mut clip.path_tracks[index].keys;
                match keys.binary_search_by(|key| key.time.total_cmp(&time)) {
                    Ok(index) => keys[index].value = value,
                    Err(index) => keys.insert(
                        index,
                        Keyframe {
                            time,
                            value,
                            curve: Curve::Linear,
                        },
                    ),
                }
                prepared.push(
                    json!({"method":"typedAbsoluteHostPathKey","constraint":constraint,
                    "property":field,"animation":animation,"time":time,"value":value}),
                );
            }
        }
    }
    assert_eq!(
        doc.bones, original.bones,
        "Path incoming override changed Setup bones"
    );
    assert_eq!(
        doc.constraints, original.constraints,
        "Path incoming override changed constraint Setup"
    );
    assert_eq!(doc.slots, original.slots);
    assert_eq!(doc.skins, original.skins);
    if let EditTarget::Animate { animation, .. } = target {
        for (index, clip) in doc.animations.iter().enumerate() {
            if index != animation {
                assert_eq!(
                    clip, &original.animations[index],
                    "Referenced animation changed"
                );
            }
        }
    }
    doc.validate()?;
    Ok(prepared)
}

fn apply_pose_overrides(
    session: &mut Session,
    case: &Value,
    target: EditTarget,
    conversions: &mut Vec<Value>,
) -> Result<(), String> {
    if matches!(target, EditTarget::Setup)
        && ["bonePose", "sliderPose", "constraintPose"]
            .iter()
            .any(|field| {
                case[*field]
                    .as_object()
                    .is_some_and(|object| !object.is_empty())
            })
    {
        return Err("Setup-world pose overrides have no public non-Setup Session target; Setup bases are not rewritten".into());
    }
    if let Some(bones) = case["bonePose"].as_object() {
        for (name, edits) in bones {
            let bone = session
                .document()
                .bones
                .iter()
                .position(|bone| bone.name == *name)
                .ok_or_else(|| format!("Unknown override bone {name}"))?;
            let mut value = session.authored_pose(target)?.bone_locals()[bone];
            let mut properties = Vec::new();
            for (field, raw) in edits
                .as_object()
                .ok_or("Bone pose override must be an object")?
            {
                let property = BONE_FIELDS
                    .iter()
                    .find(|(label, _)| *label == field)
                    .map(|(_, property)| *property)
                    .ok_or_else(|| format!("Unsupported bone pose field {field}"))?;
                property.write(
                    &mut value,
                    finite32(raw, &format!("bonePose.{name}.{field}"), conversions)?,
                );
                properties.push(property);
            }
            session.edit_pose(bone, target, value, &properties, false)?;
        }
    }
    if let Some(sliders) = case["sliderPose"].as_object() {
        for (name, edits) in sliders {
            let constraint = session
                .document()
                .constraints
                .iter()
                .position(|constraint| {
                    constraint
                        .as_slider()
                        .is_some_and(|slider| slider.name == *name)
                })
                .ok_or_else(|| format!("Unknown override Slider {name}"))?;
            let mut changes = Vec::new();
            for (field, value) in edits
                .as_object()
                .ok_or("Slider pose override must be an object")?
            {
                let value = finite32(value, &format!("sliderPose.{name}.{field}"), conversions)?;
                changes.push(match field.as_str() {
                    "time" => SliderPoseEdit::Time { value },
                    "mix" => SliderPoseEdit::Mix { value },
                    _ => return Err(format!("Unsupported Slider pose field {field}")),
                });
            }
            session.edit_slider_pose(constraint, target, &changes, false)?;
        }
    }
    if let Some(constraints) = case["constraintPose"].as_object() {
        for (name, edits) in constraints {
            let indices: Vec<_> = session
                .document()
                .constraints
                .iter()
                .enumerate()
                .filter_map(|(i, c)| (c.name() == name).then_some(i))
                .collect();
            if indices.len() != 1 {
                return Err(format!("Need exactly one constraint pose target {name}"));
            }
            let constraint = indices[0];
            if session.document().constraints[constraint]
                .as_path()
                .is_some()
            {
                // Position/Spacing were represented as exact typed host keys
                // before Session creation, with Setup/reference invariants.
                continue;
            }
            if session.document().constraints[constraint].as_ik().is_none() {
                return Err(format!(
                    "Constraint pose override {name} has no public Session pending API (Path Setup is not rewritten)"
                ));
            }
            let mut changes = Vec::new();
            for (field, value) in edits
                .as_object()
                .ok_or("Constraint pose override must be an object")?
            {
                changes.push(match field.as_str() {
                    "mix" => IkPoseEdit::Mix {
                        value: finite32(value, &format!("constraintPose.{name}.mix"), conversions)?,
                    },
                    "bendDirection" => match value.as_i64() {
                        Some(1) => IkPoseEdit::BendPositive { value: true },
                        Some(-1) => IkPoseEdit::BendPositive { value: false },
                        _ => return Err("Bend direction must be exactly -1 or 1".into()),
                    },
                    _ => {
                        return Err(format!(
                            "IK pose field {name}/{field} has no public Session pending API"
                        ));
                    }
                });
            }
            session.edit_ik_pose(constraint, target, &changes, false)?;
        }
    }
    Ok(())
}

fn compare_transform(check: &mut Comparison, label: &str, actual: Transform, expected: &Value) {
    for (field, property) in BONE_FIELDS {
        check.near(
            &format!("{label}.{field}"),
            f64::from(property.read(actual)),
            &expected[field],
        );
    }
}

fn inherit_index(value: Inherit) -> usize {
    Inherit::ALL
        .iter()
        .position(|candidate| *candidate == value)
        .unwrap()
}

fn compare_order(check: &mut Comparison, label: &str, actual: &[usize], expected: &Value) {
    let observed = expected
        .as_array()
        .map(|slots| Value::Array(slots.iter().map(|slot| slot["index"].clone()).collect()));
    check.exact(
        label,
        json!(actual),
        observed.as_ref().unwrap_or(&Value::Null),
    );
}

fn attachment_name(
    doc: &Document,
    skin: &str,
    slot: usize,
    placeholder: Option<&str>,
) -> Option<String> {
    placeholder
        .and_then(|name| doc.resolve_attachment(skin, slot, name))
        .map(|(key, attachment)| attachment.name().unwrap_or(&key.placeholder).to_owned())
}

fn compare_slots(check: &mut Comparison, doc: &Document, pose: &EvaluatedPose, after: &Value) {
    let observed = after["slots"]
        .as_array()
        .expect("Report slots must be an array");
    check.exact("slots.len", json!(doc.slots.len()), &json!(observed.len()));
    for (index, slot) in doc.slots.iter().enumerate() {
        let expected = &after["slots"][index];
        let label = format!("slot.{}", slot.name);
        check.exact(
            &format!("{label}.name"),
            json!(slot.name),
            &expected["name"],
        );
        let name = attachment_name(
            doc,
            pose.skin(),
            index,
            pose.slot_attachments()[index].as_deref(),
        );
        check.exact(
            &format!("{label}.attachment"),
            json!(name),
            &expected["applied"]["attachment"]
                .get("name")
                .cloned()
                .unwrap_or(Value::Null),
        );
        for (channel, field) in ["r", "g", "b", "a"].into_iter().enumerate() {
            check.near(
                &format!("{label}.color.{field}"),
                f64::from(pose.slot_colors()[index][channel]),
                &expected["applied"]["color"]["values"][field],
            );
        }
        let Some((key, attachment)) = pose.slot_attachments()[index]
            .as_deref()
            .and_then(|name| doc.resolve_attachment(pose.skin(), index, name))
        else {
            continue;
        };
        if let Some(vertices) = attachment.vertices() {
            let source = doc.deform_target(&key);
            let actual = pose
                .deforms()
                .iter()
                .find(|(key, _)| key == &source)
                .map(|(_, values)| values.as_slice())
                .unwrap_or(&[]);
            let expected = expected["applied"]["deform"]
                .as_array()
                .expect("Observed deform must be an array");
            let len = vertices.deform_len();
            let actual = if actual.is_empty() {
                vec![0.; len]
            } else {
                actual.iter().map(|v| f64::from(*v)).collect()
            };
            let mut offsets: Vec<f64> = if expected.is_empty() {
                vec![0.; len]
            } else {
                expected
                    .iter()
                    .map(|v| v.as_f64().expect("Finite observed deform"))
                    .collect()
            };
            if !expected.is_empty()
                && let MeshVertices::SlotLocal { positions } = vertices
            {
                for (value, base) in offsets.iter_mut().zip(positions.iter().flatten()) {
                    *value -= f64::from(*base);
                }
            }
            check.array(&format!("{label}.deformOffsets"), &actual, &json!(offsets));
        }
        if let Some(sequence) = attachment.sequence() {
            let expected_index =
                expected["applied"]["sequence"]["values"]["sequenceIndex"].as_i64();
            let expected_frame = expected_index
                .filter(|index| *index >= 0)
                .map(|i| i as u32)
                .unwrap_or(sequence.setup_index);
            let actual = pose
                .sequence_frames()
                .iter()
                .find(|(entry, _)| entry == &key)
                .map_or(sequence.setup_index, |(_, frame)| *frame);
            check.exact(
                &format!("{label}.sequenceFrame"),
                json!(actual),
                &json!(expected_frame),
            );
        }
    }
    compare_order(
        check,
        "drawOrder.applied",
        pose.draw_order(),
        &after["drawOrder"]["applied"],
    );
    // Raw array presence is not compared: runtime unweighted deform arrays
    // contain positions, while native/engine arrays contain offsets. Both are
    // converted above against the unchanged imported Setup basis.
}

fn compare_pose(
    session: &Session,
    target: EditTarget,
    pose: &EvaluatedPose,
    observation: &Value,
) -> Comparison {
    let doc = session.document();
    let before = &observation["beforeWorldUpdate"];
    let after = &observation["updates"][0]["snapshot"];
    let mut check = Comparison::default();
    check.exact(
        "bones.len",
        json!(doc.bones.len()),
        &json!(after["bones"].as_array().expect("Report bones").len()),
    );
    for (index, bone) in doc.bones.iter().enumerate() {
        let expected = &after["bones"][index];
        let label = format!("bone.{}", bone.name);
        check.exact(
            &format!("{label}.name"),
            json!(bone.name),
            &expected["name"],
        );
        check.exact(
            &format!("{label}.active"),
            json!(pose.active_bones()[index]),
            &expected["active"],
        );
        compare_transform(
            &mut check,
            &format!("{label}.authored"),
            pose.authored_locals()[index],
            &before["bones"][index]["pose"]["values"],
        );
        compare_transform(
            &mut check,
            &format!("{label}.applied"),
            pose.applied_locals()[index],
            &expected["applied"]["values"],
        );
        check.exact(
            &format!("{label}.inherit"),
            json!(inherit_index(pose.inherits()[index])),
            &expected["applied"]["values"]["inherit"],
        );
        if pose.active_bones()[index] && expected["active"] == true {
            let world = pose.world()[index];
            check.array(
                &format!("{label}.world"),
                &[world.a, world.b, world.c, world.d, world.x, world.y].map(f64::from),
                &expected["world"],
            );
        }
        // The public runtime leaves inactive world caches uncomputed. Activity,
        // authored/applied local data and our own FK consistency remain checked.
    }
    let activity = session
        .authored_pose(target)
        .expect("Already prepared authored pose")
        .activity()
        .clone();
    for slider in after["sliders"].as_array().expect("Report Sliders") {
        let index = slider["index"].as_u64().expect("Slider constraint index") as usize;
        let label = format!("slider.{}", doc.constraints[index].name());
        check.exact(
            &format!("{label}.active"),
            json!(activity.constraints[index]),
            &slider["active"],
        );
        for (field, actual) in [
            ("time", pose.applied_slider_poses()[index].time),
            ("mix", pose.applied_slider_poses()[index].mix),
        ] {
            check.near(
                &format!("{label}.applied.{field}"),
                actual,
                &slider["applied"]["values"][field],
            );
        }
        let original = before["sliders"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["index"] == slider["index"])
            .unwrap();
        // Native public Slider.pose.time may be overwritten by its source.
        // Our authored capsule is deliberately compared to the pre-solve pose.
        for (field, actual) in [
            ("time", pose.authored_slider_poses()[index].time),
            ("mix", pose.authored_slider_poses()[index].mix),
        ] {
            check.near(
                &format!("{label}.authored.{field}"),
                actual,
                &original["pose"]["values"][field],
            );
        }
    }
    for constraint in after["constraints"]
        .as_array()
        .expect("Report constraint poses")
    {
        let index = constraint["index"].as_u64().unwrap() as usize;
        let label = format!("constraint.{}", doc.constraints[index].name());
        check.exact(
            &format!("{label}.active"),
            json!(activity.constraints[index]),
            &constraint["active"],
        );
        match &doc.constraints[index] {
            Constraint::Ik(ik) => {
                let value = pose.applied_ik_poses()[index];
                check.near(
                    &format!("{label}.mix"),
                    value.mix,
                    &constraint["applied"]["values"]["mix"],
                );
                check.exact(
                    &format!("{label}.bendDirection"),
                    json!(if value.bend_positive { 1 } else { -1 }),
                    &constraint["applied"]["values"]["bendDirection"],
                );
                // The importer rejects nonzero softness/compress, and the
                // supported model stores stretch as static configuration.
                check.near(
                    &format!("{label}.softness"),
                    0.,
                    &constraint["applied"]["values"]["softness"],
                );
                check.exact(
                    &format!("{label}.compress"),
                    json!(false),
                    &constraint["applied"]["values"]["compress"],
                );
                check.exact(
                    &format!("{label}.stretch"),
                    json!(ik.stretch),
                    &constraint["applied"]["values"]["stretch"],
                );
            }
            Constraint::Path(path) => {
                let value = pose.path_poses()[index];
                for (field, value) in [
                    ("position", value.position),
                    ("spacing", value.spacing),
                    ("mixRotate", path.setup.mix_rotate),
                    ("mixX", path.setup.mix_x),
                    ("mixY", path.setup.mix_y),
                ] {
                    check.near(
                        &format!("{label}.{field}"),
                        f64::from(value),
                        &constraint["applied"]["values"][field],
                    );
                }
            }
            _ => check.mismatch(format!(
                "{label}: unsupported observed constraint pose kind"
            )),
        }
    }
    compare_slots(&mut check, doc, pose, after);
    check
}

fn expected_refusal(policy: &Value, id: &str, stage: &str, error: &str) -> bool {
    if stage == "evaluate"
        && (error.contains("awaits calibrated") || error.contains("awaits calibration"))
    {
        return true;
    }
    let entry = &policy[id];
    entry["stage"] == stage
        && entry["errorContains"]
            .as_str()
            .is_some_and(|fragment| !fragment.is_empty() && error.contains(fragment))
}

fn refusal(policy: &Value, id: &str, stage: &str, error: String) -> Value {
    json!({"id":id,"status":if expected_refusal(policy,id,stage,&error){"deferredRefusal"}else{"unexpectedRefusal"},"stage":stage,"reason":error})
}

fn run_case(observation: &Value, report_path: &Path, policy: &Value) -> Value {
    let case = &observation["case"];
    let id = case["id"].as_str().expect("Every oracle case needs an ID");
    if let Some(error) = observation.get("error") {
        return json!({"id":id,"status":"oracleError","reason":error});
    }
    if observation["updates"][0]["snapshot"]["slots"]
        .as_array()
        .is_some_and(|slots| {
            slots.iter().any(|slot| {
                slot["applied"]["darkColor"]["values"]
                    .as_object()
                    .is_some_and(|color| !color.is_empty())
            })
        })
    {
        return json!({"id":id,"status":"pendingMetadataApi",
            "reason":"Applied dark-color comparison has no public native model/API; not counting this case as fully compared"});
    }
    let input = PathBuf::from(case["input"].as_str().expect("Original input path"));
    let input = if input.is_absolute() {
        input
    } else {
        report_path.parent().unwrap().join(input)
    };
    let bytes = fs::read(&input).expect("Original oracle input must remain readable");
    let mut raw: Value = serde_json::from_slice(&bytes).expect("Original oracle JSON");
    if let Err(error) = patch_slider_input(&mut raw, case) {
        return refusal(policy, id, "import", error);
    }
    let mut doc = match peachy_spine::import_json(&raw, "Slider compatibility input") {
        Ok(doc) => doc,
        Err(error) => return refusal(policy, id, "import", error),
    };
    // A neutral empty host expresses public unkeyed pose overrides at Setup
    // without rewriting Setup bases or sampling any existing host animation.
    let mut effective_case = case.clone();
    let has_override = ["bonePose", "sliderPose", "constraintPose"]
        .iter()
        .any(|field| {
            case[*field]
                .as_object()
                .is_some_and(|values| !values.is_empty())
        });
    let synthetic_host = if case["animation"].is_null() && has_override {
        let mut suffix = 0;
        let name = loop {
            let candidate = format!("__oracle_neutral_host_{suffix}");
            if !doc.animations.iter().any(|clip| clip.name == candidate) {
                break candidate;
            }
            suffix += 1;
        };
        let index = doc.animations.len();
        doc.animations
            .push(peachy_document::Animation::new(name.clone(), vec![]));
        effective_case["animation"] = json!(name);
        effective_case["time"] = json!(0.);
        Some(
            json!({"method":"emptyHostForUnkeyedSetupOverrides","animation":index,"name":name,
            "setupBasesChanged":false,"existingHostOrReferenceClipsChanged":false}),
        )
    } else {
        None
    };
    let mut conversions = Vec::new();
    let target = match target_for(&doc, &effective_case, &mut conversions) {
        Ok(target) => target,
        Err(error) => return json!({"id":id,"status":"pendingOverrideApi","reason":error}),
    };
    let host_keys =
        match prepare_host_path_keys(&mut doc, &effective_case, target, &mut conversions) {
            Ok(prepared) => prepared,
            Err(error) => return json!({"id":id,"status":"pendingOverrideApi","reason":error}),
        };
    let baseline = doc.clone();
    let mut session = Session::new(doc).expect("Importer returns a valid document");
    let skin = case["skin"].as_str().unwrap_or("default");
    session.set_skin(skin);
    if let Err(error) =
        apply_pose_overrides(&mut session, &effective_case, target, &mut conversions)
    {
        assert_eq!(
            session.document(),
            &baseline,
            "Override must not change Setup bases"
        );
        return if error.contains("no public") {
            json!({"id":id,"status":"pendingOverrideApi","reason":error,"storageConversions":conversions})
        } else {
            refusal(policy, id, "override", error)
        };
    }
    assert_eq!(
        session.document(),
        &baseline,
        "Authored overrides must not change any document data"
    );
    let pending = serde_json::to_value(session.pose_edits()).unwrap();
    let pose = match session.evaluated_pose(target) {
        Ok(pose) => pose,
        Err(error) => return refusal(policy, id, "evaluate", error),
    };
    let mut check = compare_pose(&session, target, &pose, observation);
    let reconstructed =
        peachy_engine::world_pose_with(session.document(), pose.applied_locals(), pose.inherits());
    let mut fk_max_residual = 0f64;
    for (index, (actual, retained)) in reconstructed.iter().zip(pose.world()).enumerate() {
        for (field, a, b) in [
            ("a", actual.a, retained.a),
            ("b", actual.b, retained.b),
            ("c", actual.c, retained.c),
            ("d", actual.d, retained.d),
            ("x", actual.x, retained.x),
            ("y", actual.y, retained.y),
        ] {
            fk_max_residual = fk_max_residual.max((f64::from(a) - f64::from(b)).abs());
            check.near(
                &format!("engineAppliedFk.{index}.{field}"),
                f64::from(a),
                &json!(b),
            );
        }
    }
    let first = format!("{pose:?}");
    for repeat in 2..=3 {
        match session.evaluated_pose(target) {
            Ok(pose) if format!("{pose:?}") == first => {}
            Ok(_) => check.mismatch(format!("repeat{repeat}: complete EvaluatedPose changed")),
            Err(error) => check.mismatch(format!("repeat{repeat}: evaluation failed: {error}")),
        }
    }
    assert_eq!(
        session.document(),
        &baseline,
        "Evaluation must not bake Setup data"
    );
    assert_eq!(
        serde_json::to_value(session.pose_edits()).unwrap(),
        pending,
        "Evaluation must not mutate pending overrides"
    );
    assert_eq!(
        fs::read(&input).unwrap(),
        bytes,
        "Original oracle input changed"
    );
    json!({"id":id,"status":if check.mismatch_count==0{"compared"}else{"mismatch"},
        "numericComparisons":check.numbers,"discreteComparisons":check.discrete,"maxAbsoluteError":check.max_error,
        "mismatchCount":check.mismatch_count,"mismatches":check.mismatches,"storageConversions":conversions,
        "incomingHostPathKeys":host_keys,"neutralUnkeyedHost":synthetic_host,"engineAppliedFkMaxResidual":fk_max_residual,
        "inactiveWorldCachesUncompared":session.document().bones.iter().enumerate().filter_map(|(index,bone)|(!pose.active_bones()[index]).then_some(bone.name.as_str())).collect::<Vec<_>>(),
        "setupUnchanged":true,"pendingUnchanged":true,"threeEvaluationsExact":!check.mismatches.iter().any(|error|error.starts_with("repeat"))})
}

#[test]
#[ignore = "requires explicit private PEACHY_SLIDER_ORACLE_REPORT; root owns matrix execution"]
fn slider_retained_public_api_matrix() {
    let report_path = PathBuf::from(
        std::env::var_os("PEACHY_SLIDER_ORACLE_REPORT")
            .expect("Set an explicit private oracle report path"),
    );
    let report = read_json(&report_path).unwrap();
    let policy = std::env::var_os("PEACHY_SLIDER_ORACLE_REFUSALS")
        .map(PathBuf::from)
        .map(|path| read_json(&path).unwrap())
        .unwrap_or_else(|| json!({}));
    let observations = report["observations"]
        .as_array()
        .expect("Oracle report observations array");
    assert!(
        !observations.is_empty(),
        "An empty oracle report is not a matrix"
    );
    let original_cases_path = std::env::var_os("PEACHY_SLIDER_ORACLE_CASES").map(PathBuf::from);
    let original_cases_bytes = original_cases_path
        .as_ref()
        .map(|path| fs::read(path).unwrap());
    let original_cases: Option<Value> = original_cases_bytes
        .as_ref()
        .map(|bytes| serde_json::from_slice(bytes).unwrap());
    let results: Vec<_> = observations.iter().map(|observation| {
        let mut retained = observation.clone();
        if let (Some(path),Some(cases)) = (&original_cases_path,&original_cases) {
            let path = path.canonicalize().unwrap();
            assert!(report["sourceHashes"].as_array().unwrap().iter().any(|source| {
                Path::new(source["path"].as_str().unwrap()).canonicalize().ok().as_ref() == Some(&path)
            }), "Original cases must be a recorded immutable oracle input; root verifies its SHA before execution");
            let matches: Vec<_> = cases.as_array().unwrap().iter().filter(|case|case["id"]==observation["case"]["id"]).collect();
            assert_eq!(matches.len(),1,"Every retained observation needs exactly one original case");
            let mut original = matches[0].clone();
            original["input"] = json!(path.parent().unwrap().join(original["input"].as_str().unwrap()).canonicalize().unwrap());
            let mut observed = observation["case"].clone();
            observed["input"] = json!(Path::new(observed["input"].as_str().unwrap()).canonicalize().unwrap());
            assert_eq!(canonical_case(&original),canonical_case(&observed),"Original input differs beyond JSON signed-zero loss");
            retained["case"] = original;
        }
        run_case(&retained,&report_path,&policy)
    }).collect();
    if let (Some(path), Some(bytes)) = (&original_cases_path, &original_cases_bytes) {
        assert_eq!(
            &fs::read(path).unwrap(),
            bytes,
            "Original case input changed during replay"
        );
    }
    let mut counts = BTreeMap::<String, usize>::new();
    for result in &results {
        *counts
            .entry(result["status"].as_str().unwrap().into())
            .or_default() += 1;
    }
    let complete = counts.get("compared").copied().unwrap_or(0) == results.len();
    let failed = results
        .iter()
        .filter(|result| {
            matches!(
                result["status"].as_str(),
                Some("mismatch" | "unexpectedRefusal" | "oracleError")
            )
        })
        .count();
    let summary = json!({"owner":"/root/deform_dopesheet_ui","executionOwner":"/root","purpose":"Same-evaluation public metadata comparison against retained original Slider oracle cases",
        "removalCondition":"Retain selected private matrix while needed for active Slider acceptance/PR review; retire superseded generated outputs after verified integration",
        "status":if failed>0{"failed"}else if complete{"complete"}else{"incomplete"},"complete":complete,"tolerance":TOLERANCE,
        "oracleReport":report_path,"originalCases":original_cases_path,"caseCount":results.len(),"counts":counts,"results":results,
        "runtimeLoaded":false,"processesSpawned":false,"setupOverrideSubstitution":false,
        "scope":["World matrices and applied/authored locals, inheritance/activity, Slider time/mix, IK/path pose metadata, slot attachment/RGBA/semantic Deform offsets, draw order and effective sequence frame where observed",
            "Native Slider authored state compares against pre-world-update oracle state; derived source time compares against applied state",
            "No artwork vertices/UV output or event log is present in these oracle reports; no geometry/event claim follows",
            "Unweighted public Deform positions convert to offsets against unchanged imported Setup; empty means zero offsets",
            "Original signed-zero case inputs are replayed exactly when an immutable recorded case file is supplied; world zero-sign observations are only available when the observer recorded them",
            "Inactive runtime world caches are uncomputed; their activity and local data plus all-engine FK consistency remain checked. Inactive slot data is compared without exclusions.",
            "A neutral empty host represents unkeyed Setup-world pose overrides without changing Setup bases or existing animation clips.",
            "Incoming Path Position/Spacing overrides use exact typed keys in the selected host clip; Setup and all referenced clips remain unchanged. Other missing pose override APIs stay pending."]});
    println!(
        "{}",
        json!({"status":summary["status"],"caseCount":summary["caseCount"],"counts":summary["counts"]})
    );
    if let Some(path) = std::env::var_os("PEACHY_SLIDER_MATRIX_OUTPUT") {
        let path = PathBuf::from(path);
        assert!(
            !path.exists(),
            "Refusing to overwrite an existing matrix artifact"
        );
        assert_ne!(
            path, report_path,
            "Output must not replace an oracle report"
        );
        fs::write(path, serde_json::to_vec_pretty(&summary).unwrap()).unwrap();
    }
    assert_eq!(
        failed, 0,
        "Unexpected matrix failures; expected refusals and missing override APIs are counted separately above"
    );
    if std::env::var("PEACHY_SLIDER_MATRIX_REQUIRE_COMPLETE").as_deref() == Ok("1") {
        assert!(
            complete,
            "Matrix is incomplete; deferred refusals/pending APIs are not compatibility passes"
        );
    }
}
