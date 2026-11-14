//! Protocol schemas belong to this shell. Execution still decodes engine Commands.
use peachy_engine::Command;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, de::Error};
use serde_json::{Map, Value, json};
use std::borrow::Cow;

#[derive(Debug)]
pub struct CommandInput(pub Command);

impl<'de> Deserialize<'de> for CommandInput {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let command = Command::deserialize(deserializer)?;
        if matches!(command, Command::Batch { .. }) {
            return Err(D::Error::custom(
                "Use a flat commands list; the server batches it atomically",
            ));
        }
        Ok(Self(command))
    }
}

impl JsonSchema for CommandInput {
    fn schema_name() -> Cow<'static, str> {
        "PeachyCommand".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        Schema::try_from(command_schema()).expect("Command schema is an object")
    }
}

fn object(fields: &[(&str, Value)]) -> Value {
    let properties: Map<String, Value> = fields
        .iter()
        .map(|(k, v)| ((*k).into(), v.clone()))
        .collect();
    json!({"type":"object","properties":properties,"required":fields.iter().map(|(k,_)|*k).collect::<Vec<_>>(),"additionalProperties":false})
}

fn with_optional(mut schema: Value, fields: &[(&str, Value)]) -> Value {
    let properties = schema["properties"]
        .as_object_mut()
        .expect("Object schema has properties");
    properties.extend(
        fields
            .iter()
            .map(|(name, value)| ((*name).into(), value.clone())),
    );
    schema
}

fn nullable(schema: Value) -> Value {
    json!({"oneOf":[{"type":"null"},schema]})
}

fn list(item: Value) -> Value {
    json!({"type":"array","items":item})
}
fn index() -> Value {
    json!({"type":"integer","minimum":0})
}
fn number() -> Value {
    json!({"type":"number"})
}
fn text() -> Value {
    json!({"type":"string"})
}
fn command(name: &str, fields: &[(&str, Value)]) -> Value {
    let mut fields = fields.to_vec();
    fields.push(("command", json!({"const":name,"type":"string"})));
    object(&fields)
}

pub fn command_schema() -> Value {
    let property =
        json!({"type":"string","enum":["x","y","rotation","scaleX","scaleY","shearX","shearY"]});
    let transform = object(
        &[
            "x", "y", "rotation", "scale_x", "scale_y", "shear_x", "shear_y",
        ]
        .map(|name| (name, number())),
    );
    let region = object(&["x", "y", "rotation", "scale_x", "scale_y"].map(|name| (name, number())));
    let attachment = object(&[("skin", text()), ("slot", text()), ("placeholder", text())]);
    let rgba = json!({"type":"array","items":{"type":"integer","minimum":0,"maximum":255},"minItems":4,"maxItems":4});
    let vector = json!({"type":"array","items":{"type":"number"},"minItems":2,"maxItems":2});
    let sequence = object(&[
        ("count", json!({"type":"integer","minimum":1})),
        ("start", index()),
        ("digits", index()),
        ("setup_index", index()),
    ]);
    let region_attachment = with_optional(
        object(&[
            ("type", json!({"type":"string","const":"region"})),
            ("path", text()),
            ("width", number()),
            ("height", number()),
            ("transform", region.clone()),
        ]),
        &[
            ("name", nullable(text())),
            ("color", rgba.clone()),
            ("sequence", nullable(sequence.clone())),
        ],
    );
    let mesh_vertices = json!({"oneOf":[
        object(&[
            ("mode",json!({"type":"string","const":"slotLocal"})),
            ("positions",json!({"type":"array","items":vector.clone(),"minItems":3})),
        ]),
        object(&[
            ("mode",json!({"type":"string","const":"weighted"})),
            ("influences",json!({"type":"array","minItems":3,"items":{
                "type":"array","minItems":1,"items":object(&[
                    ("bone",index()),("position",vector.clone()),
                    ("weight",json!({"type":"number","minimum":0,"maximum":1})),
                ])
            }})),
        ]),
    ]});
    let mesh_link = object(&[
        ("skin", text()),
        ("source", text()),
        ("timelines", json!({"type":"boolean"})),
    ]);
    let mesh_attachment = with_optional(
        object(&[
            ("type", json!({"type":"string","const":"mesh"})),
            ("path", text()),
            ("vertices", mesh_vertices),
            ("uvs", json!({"type":"array","items":vector,"minItems":3})),
            (
                "triangles",
                json!({"type":"array","items":index(),"minItems":3}),
            ),
            ("hull", json!({"type":"integer","minimum":3})),
            ("width", number()),
            ("height", number()),
        ]),
        &[
            ("name", nullable(text())),
            ("edges", list(index())),
            ("color", rgba.clone()),
            ("link", nullable(mesh_link)),
            ("sequence", nullable(sequence)),
        ],
    );
    let slot = with_optional(
        object(&[
            ("name", text()),
            ("bone", index()),
            (
                "attachment",
                json!({"type":"null","description":"New slots start empty. Activate an attachment afterward with setSlotAttachment."}),
            ),
            ("color", rgba),
        ]),
        &[(
            "blend",
            json!({"type":"string","enum":["normal","additive","multiply","screen"]}),
        )],
    );
    let target = json!({"oneOf":[object(&[("mode",json!({"const":"setup","type":"string"}))]),object(&[("mode",json!({"const":"animate","type":"string"})),("animation",index()),("time",number())])]});
    let curve_handles = json!({"type":"array","items":{"type":"number"},"minItems":4,"maxItems":4});
    let curve = json!({"oneOf":[
        {"type":"string","enum":["linear","stepped"]},
        object(&[("bezier",curve_handles.clone())]),
        object(&[("bezierOffset",curve_handles)]),
    ]});
    let key = object(&[
        ("time", number()),
        ("value", number()),
        ("curve", curve.clone()),
    ]);
    let deform_key = object(&[
        ("time", number()),
        ("curve", curve),
        ("offsets", list(number())),
    ]);
    let timeline_key = object(&[
        ("bone", index()),
        ("property", property.clone()),
        ("time", number()),
    ]);
    let clipboard = object(&[(
        "keys",
        list(object(&[
            ("property", property.clone()),
            ("key", key.clone()),
        ])),
    )]);
    let bind_pose = object(&[("locals", list(transform.clone()))]);
    let ik = with_optional(
        object(&[
            ("name", text()),
            ("bones", list(index())),
            ("target", index()),
            (
                "setup",
                object(&[
                    ("mix", number()),
                    ("bend_positive", json!({"type":"boolean"})),
                ]),
            ),
        ]),
        &[("stretch", json!({"type":"boolean","default":false}))],
    );
    let inherit = json!({"type":"string","enum":["normal","onlyTranslation","noRotationOrReflection","noScale","noScaleOrReflection"]});
    let settings = [
        "inertia", "strength", "damping", "mass", "wind", "gravity", "mix",
    ];
    let transform_property =
        json!({"type":"string","enum":["rotate","x","y","scaleX","scaleY","shearY"]});
    let transform_values =
        object(&["rotate", "x", "y", "scale_x", "scale_y", "shear_y"].map(|name| (name, number())));
    let transform_constraint = object(&[
        ("name", text()),
        ("bones", list(index())),
        ("source", index()),
        ("offsets", transform_values.clone()),
        ("mix", transform_values),
        (
            "mappings",
            list(object(&[
                ("from", transform_property.clone()),
                ("offset", number()),
                (
                    "to",
                    list(object(&[
                        ("property", transform_property),
                        ("offset", number()),
                        ("max", number()),
                        ("scale", number()),
                    ])),
                ),
            ])),
        ),
        ("local_source", json!({"type":"boolean"})),
        ("local_target", json!({"type":"boolean"})),
        ("additive", json!({"type":"boolean"})),
        ("clamp", json!({"type":"boolean"})),
    ]);
    let mode = |values: &[&str]| json!({"type":"string","enum":values});
    let path_constraint = object(&[
        ("name", text()),
        ("bones", list(index())),
        ("slot", index()),
        ("position_mode", mode(&["fixed", "percent"])),
        (
            "spacing_mode",
            mode(&["length", "fixed", "percent", "proportional"]),
        ),
        ("rotate_mode", mode(&["tangent", "chain", "chainScale"])),
        ("offset_rotation", number()),
        (
            "setup",
            object(&[
                ("position", number()),
                ("spacing", number()),
                ("mix_rotate", number()),
                ("mix_x", number()),
                ("mix_y", number()),
            ]),
        ),
    ]);
    let physics = object(&[
        ("name", text()),
        ("bone", index()),
        (
            "influence",
            object(&["x", "y", "rotate", "scale_x", "shear_x"].map(|name| (name, number()))),
        ),
        (
            "scale_y_mode",
            json!({"type":"string","enum":["none","uniform","volume"]}),
        ),
        ("limit", number()),
        ("fps", number()),
        ("setup", object(&settings.map(|name| (name, number())))),
        (
            "global",
            object(&settings.map(|name| (name, json!({"type":"boolean"})))),
        ),
    ]);
    let tag = |name: &str| json!({"const":name,"type":"string"});
    let pair = json!({"type":"array","items":{"type":"number"},"minItems":2,"maxItems":2});
    let ik_curve = json!({"oneOf":[
        object(&[("type", tag("linear"))]),
        object(&[("type", tag("stepped"))]),
        object(&[("type", tag("bezier")), ("time", pair.clone()), ("mix", pair)]),
    ]});
    let ik_key = object(&[
        ("time", number()),
        ("mix", number()),
        ("bend_positive", json!({"type":"boolean"})),
        ("curve", ik_curve),
    ]);
    let key_owner = json!({"oneOf":[
        object(&[("type", tag("bone")), ("bone", index())]),
        object(&[("type", tag("ik")), ("constraint", index())]),
    ]});
    let animation_key = json!({"oneOf":[
        object(&[("type", tag("bone")), ("bone", index()), ("property", property.clone()), ("time", number())]),
        object(&[("type", tag("ik")), ("constraint", index()), ("time", number())]),
    ]});
    let animation_clipboard = json!({"oneOf":[
        object(&[("type", tag("bone")), ("keys", clipboard["properties"]["keys"].clone())]),
        object(&[("type", tag("ik")), ("keys", list(ik_key.clone()))]),
    ]});
    let variants = vec![
        command("addSlot", &[("slot", slot)]),
        command(
            "addAttachment",
            &[
                ("attachment", attachment.clone()),
                (
                    "value",
                    json!({"oneOf":[region_attachment,mesh_attachment]}),
                ),
            ],
        ),
        command(
            "setIkKey",
            &[
                ("animation", index()),
                ("constraint", index()),
                ("key", ik_key),
            ],
        ),
        command(
            "deleteIkKey",
            &[
                ("animation", index()),
                ("constraint", index()),
                ("time", number()),
            ],
        ),
        command(
            "setDeformKey",
            &[
                ("animation", index()),
                ("attachment", attachment.clone()),
                ("key", deform_key),
            ],
        ),
        command(
            "deleteDeformKey",
            &[
                ("animation", index()),
                ("attachment", attachment.clone()),
                ("time", number()),
            ],
        ),
        command(
            "moveAnimationKeys",
            &[
                ("animation", index()),
                ("keys", list(animation_key.clone())),
                ("delta", number()),
                ("copy", json!({"type":"boolean"})),
            ],
        ),
        command(
            "deleteAnimationKeys",
            &[("animation", index()), ("keys", list(animation_key))],
        ),
        command(
            "pasteAnimationKeys",
            &[
                ("animation", index()),
                ("target", key_owner),
                ("time", number()),
                ("clipboard", animation_clipboard),
            ],
        ),
        command("addIkConstraint", &[("constraint", ik.clone())]),
        command("setIkConstraint", &[("constraint", index()), ("value", ik)]),
        command(
            "addTransformConstraint",
            &[("constraint", transform_constraint.clone())],
        ),
        command(
            "setTransformConstraint",
            &[("constraint", index()), ("value", transform_constraint)],
        ),
        command(
            "setPathConstraint",
            &[("constraint", index()), ("value", path_constraint)],
        ),
        command("addPhysicsConstraint", &[("constraint", physics.clone())]),
        command(
            "setPhysicsConstraint",
            &[("constraint", index()), ("value", physics)],
        ),
        command(
            "setPose",
            &[
                ("bone", index()),
                ("target", target),
                ("transform", transform.clone()),
                ("properties", list(property.clone())),
            ],
        ),
        command(
            "setSetup",
            &[("bone", index()), ("transform", transform.clone())],
        ),
        command("renameBone", &[("bone", index()), ("name", text())]),
        command("renameSlot", &[("slot", index()), ("name", text())]),
        command("setBoneLength", &[("bone", index()), ("length", number())]),
        command(
            "setRegionTransform",
            &[("attachment", attachment.clone()), ("transform", region)],
        ),
        command(
            "setSlotAttachment",
            &[
                ("slot", index()),
                ("attachment", json!({"type":["string","null"]})),
            ],
        ),
        command(
            "setSlotColor",
            &[
                ("slot", index()),
                (
                    "color",
                    json!({"type":"array","items":{"type":"integer","minimum":0,"maximum":255},"minItems":4,"maxItems":4}),
                ),
            ],
        ),
        command(
            "setKey",
            &[
                ("animation", index()),
                ("bone", index()),
                ("property", property.clone()),
                ("key", key),
            ],
        ),
        command(
            "deleteKey",
            &[
                ("animation", index()),
                ("bone", index()),
                ("property", property),
                ("time", number()),
            ],
        ),
        command(
            "moveKeys",
            &[
                ("animation", index()),
                ("keys", list(timeline_key)),
                ("delta", number()),
                ("copy", json!({"type":"boolean"})),
            ],
        ),
        command(
            "pasteKeys",
            &[
                ("animation", index()),
                ("bone", index()),
                ("time", number()),
                ("clipboard", clipboard),
            ],
        ),
        command(
            "addBone",
            &[(
                "bone",
                object(&[
                    ("name", text()),
                    ("parent", json!({"type":["integer","null"],"minimum":0})),
                    ("length", number()),
                    ("setup", transform),
                    ("inherit", inherit.clone()),
                ]),
            )],
        ),
        command("setBoneInherit", &[("bone", index()), ("inherit", inherit)]),
        command("addAnimation", &[("name", text()), ("duration", number())]),
        command(
            "renameAnimation",
            &[("animation", index()), ("name", text())],
        ),
        command(
            "duplicateAnimation",
            &[("animation", index()), ("name", text())],
        ),
        command("convertRegionToMesh", &[("attachment", attachment.clone())]),
        command(
            "setMeshVertex",
            &[
                ("attachment", attachment.clone()),
                ("vertex", index()),
                (
                    "position",
                    json!({"type":"array","items":{"type":"number"},"minItems":2,"maxItems":2}),
                ),
            ],
        ),
        command(
            "splitMeshTriangle",
            &[("attachment", attachment.clone()), ("triangle", index())],
        ),
        command(
            "bindMeshToBone",
            &[
                ("attachment", attachment.clone()),
                ("bone", index()),
                ("pose", bind_pose.clone()),
            ],
        ),
        command(
            "unbindMesh",
            &[
                ("attachment", attachment.clone()),
                ("pose", bind_pose.clone()),
            ],
        ),
        command(
            "addMeshBoneBinding",
            &[
                ("attachment", attachment.clone()),
                ("bone", index()),
                ("pose", bind_pose.clone()),
            ],
        ),
        command(
            "setMeshBoneWeight",
            &[
                ("attachment", attachment),
                ("vertex", index()),
                ("bone", index()),
                ("weight", number()),
                ("pose", bind_pose),
            ],
        ),
    ];
    json!({"oneOf":variants,"description":"Native engine command. Indices address the current get_document result; a commands list is one atomic undo step. Transform payloads require all seven fields. Bind poses use evaluated appliedLocals, not authored locals."})
}

#[cfg(test)]
mod tests {
    use super::*;

    // Exhaustive matching makes any new engine variant a schema review gate.
    fn wire_name(command: &Command) -> &'static str {
        match command {
            Command::AddSlot { .. } => "addSlot",
            Command::AddAttachment { .. } => "addAttachment",
            Command::AddIkConstraint { .. } => "addIkConstraint",
            Command::SetIkConstraint { .. } => "setIkConstraint",
            Command::AddTransformConstraint { .. } => "addTransformConstraint",
            Command::SetTransformConstraint { .. } => "setTransformConstraint",
            Command::SetPathConstraint { .. } => "setPathConstraint",
            Command::AddPhysicsConstraint { .. } => "addPhysicsConstraint",
            Command::SetPhysicsConstraint { .. } => "setPhysicsConstraint",
            Command::SetIkKey { .. } => "setIkKey",
            Command::DeleteIkKey { .. } => "deleteIkKey",
            Command::SetDeformKey { .. } => "setDeformKey",
            Command::DeleteDeformKey { .. } => "deleteDeformKey",
            Command::MoveAnimationKeys { .. } => "moveAnimationKeys",
            Command::DeleteAnimationKeys { .. } => "deleteAnimationKeys",
            Command::PasteAnimationKeys { .. } => "pasteAnimationKeys",
            Command::SetPose { .. } => "setPose",
            Command::SetSetup { .. } => "setSetup",
            Command::RenameBone { .. } => "renameBone",
            Command::RenameSlot { .. } => "renameSlot",
            Command::SetBoneLength { .. } => "setBoneLength",
            Command::SetRegionTransform { .. } => "setRegionTransform",
            Command::SetSlotAttachment { .. } => "setSlotAttachment",
            Command::SetSlotColor { .. } => "setSlotColor",
            Command::SetKey { .. } => "setKey",
            Command::DeleteKey { .. } => "deleteKey",
            Command::MoveKeys { .. } => "moveKeys",
            Command::PasteKeys { .. } => "pasteKeys",
            Command::AddBone { .. } => "addBone",
            Command::SetBoneInherit { .. } => "setBoneInherit",
            Command::AddAnimation { .. } => "addAnimation",
            Command::RenameAnimation { .. } => "renameAnimation",
            Command::DuplicateAnimation { .. } => "duplicateAnimation",
            Command::ConvertRegionToMesh { .. } => "convertRegionToMesh",
            Command::SetMeshVertex { .. } => "setMeshVertex",
            Command::SplitMeshTriangle { .. } => "splitMeshTriangle",
            Command::BindMeshToBone { .. } => "bindMeshToBone",
            Command::UnbindMesh { .. } => "unbindMesh",
            Command::AddMeshBoneBinding { .. } => "addMeshBoneBinding",
            Command::SetMeshBoneWeight { .. } => "setMeshBoneWeight",
            Command::Batch { .. } => "batch",
        }
    }

    fn sample(schema: &Value) -> Value {
        if let Some(value) = schema.get("const") {
            return value.clone();
        }
        if let Some(values) = schema["enum"].as_array() {
            return values[0].clone();
        }
        if let Some(variants) = schema["oneOf"].as_array() {
            return sample(&variants[0]);
        }
        if schema["type"].is_array() {
            return Value::Null;
        }
        match schema["type"].as_str().unwrap() {
            "object" => schema["properties"]
                .as_object()
                .unwrap()
                .iter()
                .filter(|(key, _)| {
                    schema["required"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|required| required.as_str() == Some(key.as_str()))
                })
                .map(|(key, value)| (key.clone(), sample(value)))
                .collect(),
            "array" => Value::Array(
                (0..schema["minItems"].as_u64().unwrap_or(1))
                    .map(|_| sample(&schema["items"]))
                    .collect(),
            ),
            "string" => json!("sample"),
            "integer" => json!(0),
            "number" => json!(0.0),
            "boolean" => json!(true),
            "null" => Value::Null,
            kind => panic!("Unmodeled schema type {kind}"),
        }
    }

    #[test]
    fn every_published_shape_decodes_through_strict_engine_serde() {
        let schema = command_schema();
        let variants = schema["oneOf"].as_array().unwrap();
        assert_eq!(variants.len(), 40);
        for variant in variants {
            let payload = sample(variant);
            let command: CommandInput = serde_json::from_value(payload.clone()).unwrap();
            assert_eq!(payload["command"], wire_name(&command.0));
            assert_eq!(serde_json::to_value(command.0).unwrap(), payload);
            let mut unknown_field = payload;
            unknown_field["futureField"] = json!(true);
            assert!(serde_json::from_value::<CommandInput>(unknown_field).is_err());
        }
    }

    #[test]
    fn deform_keys_publish_dense_offsets_and_every_native_curve_shape() {
        let schema = command_schema();
        let set = schema["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .find(|variant| variant["properties"]["command"]["const"] == "setDeformKey")
            .unwrap();
        let key_schema = &set["properties"]["key"];
        assert_eq!(key_schema["additionalProperties"], false);
        assert_eq!(key_schema["properties"]["offsets"]["type"], "array");
        assert_eq!(
            key_schema["properties"]["offsets"]["items"]["type"],
            "number"
        );
        let curves = key_schema["properties"]["curve"]["oneOf"]
            .as_array()
            .unwrap();
        for variant in curves {
            if variant["type"] == "object" {
                assert_eq!(variant["additionalProperties"], false);
            }
        }
        for curve in [
            json!("linear"),
            json!("stepped"),
            json!({"bezier":[0.25,0.0,0.75,1.0]}),
            json!({"bezierOffset":[0.25,2.0,0.75,-1.0]}),
        ] {
            let payload = json!({
                "command":"setDeformKey",
                "animation":0,
                "attachment":{"skin":"default","slot":"hand.R","placeholder":"hand.R"},
                "key":{"time":0.5,"curve":curve,"offsets":[0.0,1.0,-2.0,3.0]},
            });
            let parsed: CommandInput = serde_json::from_value(payload.clone()).unwrap();
            assert_eq!(wire_name(&parsed.0), "setDeformKey");
            assert_eq!(serde_json::to_value(parsed.0).unwrap(), payload);
            for field in ["offsets", "curve", "time"] {
                let mut missing = payload.clone();
                missing["key"].as_object_mut().unwrap().remove(field);
                assert!(serde_json::from_value::<CommandInput>(missing).is_err());
            }
            let mut sparse = payload.clone();
            sparse["key"]["offset"] = json!(2);
            assert!(serde_json::from_value::<CommandInput>(sparse).is_err());
            let mut nested_offsets = payload.clone();
            nested_offsets["key"]["offsets"] = json!([[0.0, 1.0]]);
            assert!(serde_json::from_value::<CommandInput>(nested_offsets).is_err());
            let mut unknown_attachment = payload;
            unknown_attachment["attachment"]["futureField"] = json!(true);
            assert!(serde_json::from_value::<CommandInput>(unknown_attachment).is_err());
        }
        let delete = json!({
            "command":"deleteDeformKey",
            "animation":0,
            "attachment":{"skin":"default","slot":"hand.R","placeholder":"hand.R"},
            "time":0.5,
        });
        let parsed: CommandInput = serde_json::from_value(delete.clone()).unwrap();
        assert_eq!(wire_name(&parsed.0), "deleteDeformKey");
        assert_eq!(serde_json::to_value(parsed.0).unwrap(), delete);
        let mut unknown = delete;
        unknown["offsets"] = json!([0.0, 1.0]);
        assert!(serde_json::from_value::<CommandInput>(unknown).is_err());
    }

    #[test]
    fn attachment_creation_exposes_strict_regions_meshes_and_influence_payloads() {
        let schema = command_schema();
        let add = schema["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .find(|variant| variant["properties"]["command"]["const"] == "addAttachment")
            .unwrap();
        let variants = add["properties"]["value"]["oneOf"].as_array().unwrap();
        assert_eq!(variants.len(), 2);
        for variant in variants {
            assert_eq!(variant["additionalProperties"], false);
        }
        let sequence = json!({"count":2,"start":0,"digits":1,"setup_index":0});
        let region = json!({
            "type":"region","name":"body-name","path":"body","width":10.0,"height":12.0,
            "transform":{"x":0.0,"y":0.0,"rotation":0.0,"scale_x":1.0,"scale_y":1.0},
            "color":[128,255,255,255],"sequence":sequence
        });
        let mut mesh = json!({
            "type":"mesh","name":"mesh-name","path":"cloth","width":10.0,"height":12.0,
            "vertices":{"mode":"slotLocal","positions":[[0.0,0.0],[2.0,0.0],[0.0,2.0]]},
            "uvs":[[0.0,0.0],[1.0,0.0],[0.0,1.0]],"triangles":[0,1,2],"hull":3,
            "edges":[0,2,2,4,4,0],"color":[128,255,255,255],
            "link":{"skin":"default","source":"source","timelines":true},"sequence":sequence
        });
        let mut values = vec![region, mesh.clone()];
        mesh["vertices"] = json!({"mode":"weighted","influences":[
            [{"bone":0,"position":[0.0,0.0],"weight":0.5},{"bone":1,"position":[0.0,0.0],"weight":0.5}],
            [{"bone":0,"position":[2.0,0.0],"weight":1.0}],
            [{"bone":1,"position":[0.0,2.0],"weight":1.0}]
        ]});
        values.push(mesh);
        for value in values {
            let payload = json!({
                "command":"addAttachment",
                "attachment":{"skin":"default","slot":"body","placeholder":"body"},
                "value":value
            });
            let parsed: CommandInput = serde_json::from_value(payload.clone()).unwrap();
            assert_eq!(serde_json::to_value(parsed.0).unwrap(), payload);
            let mut unknown = payload;
            unknown["value"]["futureField"] = json!(true);
            assert!(serde_json::from_value::<CommandInput>(unknown).is_err());
        }
    }

    #[test]
    fn ik_constraint_schema_exposes_optional_stretch_without_weakening_strict_fields() {
        let schema = command_schema();
        for (name, field) in [
            ("addIkConstraint", "constraint"),
            ("setIkConstraint", "value"),
        ] {
            let variant = schema["oneOf"]
                .as_array()
                .unwrap()
                .iter()
                .find(|variant| variant["properties"]["command"]["const"] == name)
                .unwrap();
            let constraint_schema = &variant["properties"][field];
            assert_eq!(constraint_schema["additionalProperties"], false);
            assert_eq!(
                constraint_schema["properties"]["setup"]["additionalProperties"],
                false
            );
            assert_eq!(
                constraint_schema["properties"]["stretch"],
                json!({"type":"boolean","default":false})
            );
            assert!(
                constraint_schema["required"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|required| required != "stretch")
            );
            for stretch in [None, Some(false), Some(true)] {
                let mut constraint = json!({
                    "name":"leg","bones":[0,1],"target":2,
                    "setup":{"mix":1.0,"bend_positive":true}
                });
                if let Some(stretch) = stretch {
                    constraint["stretch"] = json!(stretch);
                }
                let mut payload = json!({"command":name});
                if name == "setIkConstraint" {
                    payload["constraint"] = json!(0);
                }
                payload[field] = constraint;
                let parsed: CommandInput = serde_json::from_value(payload.clone()).unwrap();
                let actual = match &parsed.0 {
                    Command::AddIkConstraint { constraint } => constraint.stretch,
                    Command::SetIkConstraint { value, .. } => value.stretch,
                    _ => panic!("Expected an IK constraint command"),
                };
                assert_eq!(actual, stretch.unwrap_or(false));
                let canonical = serde_json::to_value(parsed.0).unwrap();
                let mut expected = payload.clone();
                if !actual {
                    expected[field].as_object_mut().unwrap().remove("stretch");
                }
                assert_eq!(canonical, expected);
                let reopened: CommandInput = serde_json::from_value(canonical.clone()).unwrap();
                assert_eq!(serde_json::to_value(reopened.0).unwrap(), canonical);

                let mut unknown = payload.clone();
                unknown[field]["futureField"] = json!(true);
                assert!(serde_json::from_value::<CommandInput>(unknown).is_err());
                let mut invalid = payload;
                invalid[field]["stretch"] = json!("true");
                assert!(serde_json::from_value::<CommandInput>(invalid).is_err());
            }
        }
    }
}
