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

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct AnimationKeyInput(pub peachy_engine::AnimationKey);

/// New group-copy input has object-only identities and string-only property
/// tokens without changing the established single-owner clipboard wire.
#[derive(Debug)]
pub struct GroupAnimationKeyInput(pub peachy_engine::AnimationKey);

impl<'de> Deserialize<'de> for GroupAnimationKeyInput {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct AttachmentInput(peachy_document::AttachmentRef);
        impl<'de> Deserialize<'de> for AttachmentInput {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                Ok(Self(deserialize_object(deserializer)?))
            }
        }
        #[derive(Deserialize)]
        #[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
        enum Wire {
            Bone {
                bone: usize,
                property: String,
                time: f32,
            },
            Ik {
                constraint: usize,
                time: f32,
            },
            Slider {
                constraint: usize,
                property: String,
                time: f32,
            },
            Deform {
                attachment: AttachmentInput,
                time: f32,
            },
            SlotAttachment {
                slot: usize,
                time: f32,
            },
            SlotColor {
                slot: usize,
                time: f32,
            },
            DrawOrder {
                time: f32,
            },
        }
        use peachy_document::{Property, SliderProperty};
        use peachy_engine::AnimationKey;
        Ok(Self(match deserialize_object(deserializer)? {
            Wire::Bone {
                bone,
                property,
                time,
            } => AnimationKey::Bone {
                bone,
                property: match property.as_str() {
                    "x" => Property::X,
                    "y" => Property::Y,
                    "rotation" => Property::Rotation,
                    "scaleX" => Property::ScaleX,
                    "scaleY" => Property::ScaleY,
                    "shearX" => Property::ShearX,
                    "shearY" => Property::ShearY,
                    _ => return Err(D::Error::custom("Unknown bone property")),
                },
                time,
            },
            Wire::Ik { constraint, time } => AnimationKey::Ik { constraint, time },
            Wire::Slider {
                constraint,
                property,
                time,
            } => AnimationKey::Slider {
                constraint,
                property: match property.as_str() {
                    "time" => SliderProperty::Time,
                    "mix" => SliderProperty::Mix,
                    _ => return Err(D::Error::custom("Unknown Slider property")),
                },
                time,
            },
            Wire::Deform { attachment, time } => AnimationKey::Deform {
                attachment: attachment.0,
                time,
            },
            Wire::SlotAttachment { slot, time } => AnimationKey::SlotAttachment { slot, time },
            Wire::SlotColor { slot, time } => AnimationKey::SlotColor { slot, time },
            Wire::DrawOrder { time } => AnimationKey::DrawOrder { time },
        }))
    }
}

impl JsonSchema for GroupAnimationKeyInput {
    fn schema_name() -> Cow<'static, str> {
        "PeachyGroupAnimationKey".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        Schema::try_from(animation_key_schema()).expect("Animation key schema is an object")
    }
}

/// Enforce JSON objects before delegating strict field decoding. Derived
/// structs otherwise also accept positional arrays, contrary to the schema.
pub(super) fn deserialize_object<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct ObjectVisitor<T>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for ObjectVisitor<T> {
        type Value = T;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("an object")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
            T::deserialize(serde::de::value::MapAccessDeserializer::new(map))
        }
    }
    deserializer.deserialize_map(ObjectVisitor(std::marker::PhantomData))
}

#[derive(Debug)]
pub struct KeyOwnerInput(pub peachy_engine::KeyOwner);

impl<'de> Deserialize<'de> for KeyOwnerInput {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct AttachmentInput(peachy_document::AttachmentRef);
        impl<'de> Deserialize<'de> for AttachmentInput {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                Ok(Self(deserialize_object(deserializer)?))
            }
        }
        #[derive(Deserialize)]
        #[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
        enum Wire {
            Bone { bone: usize },
            Ik { constraint: usize },
            Slider { constraint: usize },
            Deform { attachment: AttachmentInput },
            Slot { slot: usize },
            DrawOrder {},
        }
        use peachy_engine::KeyOwner;
        Ok(Self(match deserialize_object(deserializer)? {
            Wire::Bone { bone } => KeyOwner::Bone { bone },
            Wire::Ik { constraint } => KeyOwner::Ik { constraint },
            Wire::Slider { constraint } => KeyOwner::Slider { constraint },
            Wire::Deform { attachment } => KeyOwner::Deform {
                attachment: attachment.0,
            },
            Wire::Slot { slot } => KeyOwner::Slot { slot },
            Wire::DrawOrder {} => KeyOwner::DrawOrder {},
        }))
    }
}

impl JsonSchema for KeyOwnerInput {
    fn schema_name() -> Cow<'static, str> {
        "PeachyKeyOwner".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        Schema::try_from(key_owner_schema()).expect("Key owner schema is an object")
    }
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct TransformPropertyInput(pub peachy_document::TransformProperty);

impl JsonSchema for TransformPropertyInput {
    fn schema_name() -> Cow<'static, str> {
        "PeachyTransformProperty".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        Schema::try_from(transform_property_schema())
            .expect("Transform property schema is an object")
    }
}

fn slider_property_schema() -> Value {
    json!({"type":"string","enum":["time","mix"]})
}

fn transform_property_schema() -> Value {
    json!({"type":"string","enum":["rotate","x","y","scaleX","scaleY","shearY"]})
}

impl JsonSchema for AnimationKeyInput {
    fn schema_name() -> Cow<'static, str> {
        "PeachyAnimationKey".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        Schema::try_from(animation_key_schema()).expect("Animation key schema is an object")
    }
}

pub fn animation_key_schema() -> Value {
    let property =
        json!({"type":"string","enum":["x","y","rotation","scaleX","scaleY","shearX","shearY"]});
    let reference = object(&[("skin", text()), ("slot", text()), ("placeholder", text())]);
    let tag = |name: &str| json!({"const":name,"type":"string"});
    json!({"oneOf":[
        object(&[("type",tag("bone")),("bone",index()),("property",property),("time",number())]),
        object(&[("type",tag("ik")),("constraint",index()),("time",number())]),
        object(&[("type",tag("deform")),("attachment",reference),("time",number())]),
        object(&[("type",tag("slotAttachment")),("slot",index()),("time",number())]),
        object(&[("type",tag("slotColor")),("slot",index()),("time",number())]),
        object(&[("type",tag("drawOrder")),("time",number())]),
        object(&[("type",tag("slider")),("constraint",index()),("property",slider_property_schema()),("time",number())]),
    ]})
}

fn key_owner_schema() -> Value {
    let tag = |name: &str| json!({"const":name,"type":"string"});
    json!({"oneOf":[
        object(&[("type",tag("bone")),("bone",index())]),
        object(&[("type",tag("ik")),("constraint",index())]),
        object(&[("type",tag("slider")),("constraint",index())]),
        object(&[("type",tag("deform")),("attachment",object(&[("skin",text()),("slot",text()),("placeholder",text())]))]),
        object(&[("type",tag("slot")),("slot",index())]),
        object(&[("type",tag("drawOrder"))]),
    ]})
}

pub(super) fn animation_keys_query_schema() -> Value {
    object(&[
        ("expected_revision", index()),
        ("animation", index()),
        (
            "owners",
            json!({"type":"array","items":key_owner_schema(),"uniqueItems":true,
            "description":"Required explicit represented owners. Empty requests full inventory with no selected keys. Duplicate canonical linked-owner aliases are also rejected."}),
        ),
    ])
}

pub(super) fn copy_animation_key_group_schema() -> Value {
    object(&[
        ("expected_revision", index()),
        ("animation", index()),
        (
            "keys",
            json!({"type":"array","items":animation_key_schema(),"minItems":1,"uniqueItems":true}),
        ),
    ])
}

pub(super) fn transform_bones_tool_schema() -> Value {
    object(&[
        ("expected_revision", index()),
        (
            "target",
            json!({"oneOf":[
                object(&[("mode", json!({"type":"string","const":"setup"}))]),
                object(&[("mode", json!({"type":"string","const":"animate"})), ("animation", index()),
                    ("time", json!({"type":"number","minimum":0}))])
            ]}),
        ),
        ("skin", text()),
        (
            "bones",
            json!({"type":"array","minItems":1,"uniqueItems":true,"items":index()}),
        ),
        ("primary", index()),
        (
            "operation",
            object(&[
                ("type", json!({"type":"string","const":"translate"})),
                (
                    "axes",
                    json!({"type":"string","enum":["local","parent","world"]}),
                ),
                ("delta", object(&[("x", number()), ("y", number())])),
            ]),
        ),
        ("auto_key", json!({"type":"boolean"})),
    ])
}

pub(super) fn bone_pose_tool_schema(kind: &str) -> Value {
    let properties = json!({"type":"array","minItems":1,"uniqueItems":true,
        "items":{"type":"string","enum":["x","y","rotation","scaleX","scaleY","shearX","shearY"]}});
    match kind {
        "edit" => object(&[
            ("expected_revision", index()),
            (
                "target",
                json!({"oneOf":[
                    object(&[("mode", json!({"type":"string","const":"setup"}))]),
                    object(&[("mode", json!({"type":"string","const":"animate"})),
                             ("animation", index()), ("time", json!({"type":"number","minimum":0}))])
                ]}),
            ),
            (
                "edits",
                json!({"type":"array","minItems":1,"items":object(&[
                    ("bone", index()),
                    ("transform", object(&["x","y","rotation","scale_x","scale_y","shear_x","shear_y"].map(|name|(name,number())))),
                    ("properties", properties)
                ])}),
            ),
            ("auto_key", json!({"type":"boolean"})),
        ]),
        "key" => object(&[
            ("expected_revision", index()),
            ("animation", index()),
            ("time", json!({"type":"number","minimum":0})),
            (
                "keys",
                json!({"type":"array","minItems":1,"items":object(&[
                    ("bone", index()), ("properties", properties)
                ])}),
            ),
        ]),
        "edited" => object(&[
            ("expected_revision", index()),
            ("animation", index()),
            ("time", json!({"type":"number","minimum":0})),
        ]),
        _ => unreachable!("Unknown bone pose tool schema"),
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
            ("color", rgba.clone()),
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
        ("curve", curve.clone()),
        ("offsets", list(number())),
    ]);
    let attachment_key = object(&[("time", number()), ("attachment", nullable(text()))]);
    let color_curves = json!({"type":"array","items":curve.clone(),"minItems":4,"maxItems":4});
    let color_key = object(&[
        ("time", number()),
        ("color", rgba),
        ("curves", color_curves.clone()),
    ]);
    let draw_order_key = object(&[
        ("time", number()),
        (
            "offsets",
            list(object(&[
                ("slot", index()),
                (
                    "offset",
                    json!({"type":"integer","minimum":i32::MIN,"maximum":i32::MAX}),
                ),
            ])),
        ),
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
    let bind_pose = with_optional(
        object(&[("locals", list(transform.clone()))]),
        &[("inherits", list(inherit.clone()))],
    );
    let settings = [
        "inertia", "strength", "damping", "mass", "wind", "gravity", "mix",
    ];
    let transform_property = transform_property_schema();
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
                        ("property", transform_property.clone()),
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
    let slider_constraint = object(&[
        ("name", text()),
        ("animation", index()),
        ("bone", nullable(index())),
        ("property", nullable(transform_property_schema())),
        ("local", json!({"type":"boolean"})),
        ("loop", json!({"type":"boolean"})),
        ("additive", json!({"type":"boolean"})),
        ("from", number()),
        ("to", number()),
        ("scale", number()),
        ("max", number()),
        ("setup", object(&[("time", number()), ("mix", number())])),
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
        ("curve", ik_curve.clone()),
    ]);
    let key_owner = key_owner_schema();
    let animation_key = animation_key_schema();
    let curve_selection = object(&[
        ("key", animation_key.clone()),
        ("channel", json!({"type":"integer","minimum":0,"maximum":3})),
    ]);
    let curve_preset = json!({"oneOf":[
        {"type":"string","enum":["linear","stepped","bezier"]},
        object(&[("custom",json!({"type":"array","items":{"type":"number"},"minItems":4,"maxItems":4}))]),
    ]});
    let deform_layout = json!({"oneOf":[
        object(&[("type",tag("unweighted")),("vertices",index())]),
        object(&[("type",tag("weighted")),("influence_counts",list(index()))]),
    ]});
    let animation_clipboard = json!({"oneOf":[
        object(&[("type", tag("bone")), ("keys", clipboard["properties"]["keys"].clone())]),
        object(&[("type", tag("ik")), ("keys", list(ik_key.clone()))]),
        object(&[("type", tag("slider")), ("keys", list(object(&[
            ("property", slider_property_schema()), ("key", key.clone())
        ])))]),
        object(&[("type",tag("deform")),("layout",deform_layout),("keys",list(deform_key.clone()))]),
        object(&[("type",tag("slot")),("attachment_keys",list(attachment_key.clone())),("color_keys",list(color_key.clone()))]),
        object(&[("type",tag("drawOrder")),
            ("slots",json!({"type":"array","items":{"type":"string","minLength":1},"uniqueItems":true})),
            ("keys",json!({"type":"array","items":draw_order_key.clone(),"minItems":1}))]),
    ]});
    let group_clipboard = object(&[(
        "entries",
        json!({"type":"array","items":animation_clipboard.clone(),"minItems":1}),
    )]);
    let group_targets = json!({"type":"array","items":object(&[("entry",index()),("target",key_owner.clone())]),"minItems":1,"uniqueItems":true});
    let variants = vec![
        command("createSliderConstraint", &[("animation", index())]),
        command(
            "setSliderSource",
            &[("constraint", index()), ("bone", nullable(index()))],
        ),
        command(
            "setSliderSourceProperty",
            &[
                ("constraint", index()),
                ("property", transform_property_schema()),
                (
                    "source_range",
                    object(&[("min", number()), ("max", number())]),
                ),
            ],
        ),
        command(
            "setSliderSetupPose",
            &[
                ("constraint", index()),
                (
                    "edits",
                    list(json!({"oneOf":[
                        object(&[("type",tag("time")),("value",number())]),
                        object(&[("type",tag("mix")),("value",number())])
                    ]})),
                ),
                ("skin", text()),
            ],
        ),
        command(
            "editSliderMappingRange",
            &[
                ("constraint", index()),
                (
                    "field",
                    json!({"type":"string","enum":["sourceMin","sourceMax","targetMin","targetMax"]}),
                ),
                ("value", number()),
            ],
        ),
        command(
            "addSliderConstraint",
            &[("constraint", slider_constraint.clone())],
        ),
        command(
            "setSliderConstraint",
            &[("constraint", index()), ("value", slider_constraint)],
        ),
        command(
            "setSliderKey",
            &[
                ("animation", index()),
                ("constraint", index()),
                ("property", slider_property_schema()),
                ("key", key.clone()),
            ],
        ),
        command(
            "insertSliderKey",
            &[
                ("animation", index()),
                ("constraint", index()),
                ("property", slider_property_schema()),
                ("time", number()),
                ("value", number()),
            ],
        ),
        command(
            "setSliderCurve",
            &[
                ("animation", index()),
                ("constraint", index()),
                ("property", slider_property_schema()),
                ("time", number()),
                ("curve", curve.clone()),
            ],
        ),
        command(
            "setSlotAttachmentKey",
            &[
                ("animation", index()),
                ("slot", index()),
                ("key", attachment_key),
            ],
        ),
        command(
            "setSlotColorKey",
            &[
                ("animation", index()),
                ("slot", index()),
                ("key", color_key),
            ],
        ),
        command(
            "setSlotColorCurves",
            &[
                ("animation", index()),
                ("slot", index()),
                ("time", number()),
                ("curves", color_curves),
            ],
        ),
        command(
            "insertSlotColorKey",
            &[
                ("animation", index()),
                ("slot", index()),
                ("time", number()),
                (
                    "color",
                    json!({"type":"array","items":{"type":"number"},"minItems":4,"maxItems":4,"description":"Unmultiplied normalized float RGBA, before explicit native-byte key encoding. Preserves neighboring curve transitions when inserting."}),
                ),
            ],
        ),
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
            "setIkCurve",
            &[
                ("animation", index()),
                ("constraint", index()),
                ("time", number()),
                ("curve", ik_curve),
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
            "setDeformCurve",
            &[
                ("animation", index()),
                ("attachment", attachment.clone()),
                ("time", number()),
                ("curve", curve.clone()),
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
            "pasteAnimationKeyGroup",
            &[
                ("animation", index()),
                ("time", number()),
                ("clipboard", group_clipboard),
                ("targets", group_targets),
            ],
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
            "editTransformMappingRange",
            &[
                ("constraint", index()),
                ("from", transform_property.clone()),
                ("to", transform_property),
                (
                    "field",
                    json!({"type":"string","enum":["sourceMin","sourceMax","targetMin","targetMax"]}),
                ),
                (
                    "value",
                    json!({"type":"number","description":"Finite f64 endpoint for the unique supported world X-to-Rotate mapping. Preserve the other endpoints; equal endpoints repair Max to Min+100, reversed ranges remain reversed. Use get_transform_mapping for exact readback. Raw zero-scale sourceMax is undefined until a resolving sourceMax edit."}),
                ),
            ],
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
            "setDrawOrderKey",
            &[("animation", index()), ("key", draw_order_key)],
        ),
        command(
            "setDrawOrder",
            &[
                ("animation", index()),
                ("time", number()),
                (
                    "order",
                    json!({"type":"array","items":index(),"uniqueItems":true,
                        "description":"Complete back-to-front permutation of every current slot index, including hidden and empty slots."}),
                ),
            ],
        ),
        command(
            "setKeyCurve",
            &[
                ("animation", index()),
                ("bone", index()),
                ("property", property.clone()),
                ("time", number()),
                ("curve", curve.clone()),
            ],
        ),
        command(
            "applyCurvePreset",
            &[
                ("animation", index()),
                ("selection", list(curve_selection.clone())),
                ("preset", curve_preset),
                ("separate", json!({"type":"boolean"})),
            ],
        ),
        command(
            "editCurveHandle",
            &[
                ("animation", index()),
                ("selection", list(curve_selection.clone())),
                ("primary", curve_selection),
                ("handle", json!({"type":"integer","minimum":0,"maximum":1})),
                (
                    "point",
                    json!({"type":"array","items":{"type":"number"},"minItems":2,"maxItems":2}),
                ),
                ("match_curves", json!({"type":"boolean"})),
                ("separate", json!({"type":"boolean"})),
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
                ("attachment", attachment.clone()),
                ("vertex", index()),
                ("bone", index()),
                ("weight", number()),
                ("pose", bind_pose.clone()),
            ],
        ),
        command(
            "editMeshBoneWeights",
            &[
                ("attachment", attachment),
                (
                    "vertices",
                    json!({"type":"array","items":{"type":"integer","minimum":0},"minItems":1,"uniqueItems":true,"description":"Exact selected vertex indices; one attachment and one globally bound bone. No implicit active selection."}),
                ),
                ("bone", index()),
                (
                    "edit",
                    json!({"description":"Absolute assigns every eligible vertex; sole-positive vertices without recipients stay unchanged. Relative describes one gesture with per-step clamps and frozen original recipient ratios. Positive missing influences use local [0,0] in consistent mesh binding order; ambiguous repeated or contradictory ordering fails atomically.","oneOf":[
                        object(&[("mode",json!({"const":"absolute"})),("weight",json!({"type":"number","minimum":0,"maximum":1}))]),
                        object(&[("mode",json!({"const":"relative"})),("deltas",json!({"type":"array","items":{"type":"number"},"minItems":1,"description":"Ordered signed weight-unit steps within ONE gesture, not pixels or a net delta. Clamp each step against frozen original recipient ratios."}))])
                    ]}),
                ),
                ("pose", bind_pose),
            ],
        ),
    ];
    json!({"oneOf":variants,"description":"Native engine command. Indices address the current get_document result; a commands list is one atomic undo step. Transform payloads require all seven fields. Bind poses use evaluated appliedLocals and inherits, not authored locals. Omitted inherits retains legacy Setup modes; supplied inherits must contain one mode per bone and cannot be null."})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slider_property_accepts_only_the_advertised_string_tokens() {
        for (token, expected) in [
            ("time", peachy_document::SliderProperty::Time),
            ("mix", peachy_document::SliderProperty::Mix),
        ] {
            assert_eq!(
                serde_json::from_value::<peachy_document::SliderProperty>(serde_json::json!(token))
                    .unwrap(),
                expected
            );
            assert!(
                serde_json::from_value::<peachy_document::SliderProperty>(
                    serde_json::json!({token:null})
                )
                .is_err()
            );
        }
        for invalid in [
            serde_json::json!(null),
            serde_json::json!(0),
            serde_json::json!("frame"),
            serde_json::json!([]),
        ] {
            assert!(serde_json::from_value::<peachy_document::SliderProperty>(invalid).is_err());
        }
    }

    #[test]
    fn slider_source_property_matches_the_nullable_string_schema() {
        let value =
            serde_json::to_value(peachy_document::SliderConstraint::new("slider", 0)).unwrap();
        for property in [
            json!({"rotate":null}),
            json!("rotation"),
            json!(0),
            json!([]),
        ] {
            let mut bad = value.clone();
            bad["property"] = property;
            assert!(serde_json::from_value::<peachy_document::SliderConstraint>(bad).is_err());
        }
        for property in [
            json!(null),
            json!("rotate"),
            json!("x"),
            json!("y"),
            json!("scaleX"),
            json!("scaleY"),
            json!("shearY"),
        ] {
            let mut valid = value.clone();
            valid["property"] = property;
            assert!(serde_json::from_value::<peachy_document::SliderConstraint>(valid).is_ok());
        }
    }

    #[test]
    fn slider_source_commands_require_explicit_bone_and_strict_property_snapshots() {
        let source = json!({"command":"setSliderSource","constraint":0,"bone":null});
        assert!(serde_json::from_value::<CommandInput>(source.clone()).is_ok());
        let mut missing = source;
        missing.as_object_mut().unwrap().remove("bone");
        assert!(serde_json::from_value::<CommandInput>(missing).is_err());
        let command = json!({"command":"setSliderSourceProperty","constraint":0,"property":"scaleX",
            "source_range":{"min":1.0,"max":2.0}});
        for property in ["rotate", "x", "y", "scaleX", "scaleY", "shearY"] {
            let mut valid = command.clone();
            valid["property"] = json!(property);
            assert!(serde_json::from_value::<CommandInput>(valid).is_ok());
        }
        for property in [
            json!({"scaleX":null}),
            json!(null),
            json!([]),
            json!("rotation"),
            json!(1),
        ] {
            let mut bad = command.clone();
            bad["property"] = property;
            assert!(serde_json::from_value::<CommandInput>(bad).is_err());
        }
        for range in [
            json!([1.0, 2.0]),
            json!({"min":1.0}),
            json!({"max":2.0}),
            json!({"min":1.0,"max":2.0,"extra":0}),
            json!(null),
        ] {
            let mut bad = command.clone();
            bad["source_range"] = range;
            assert!(serde_json::from_value::<CommandInput>(bad).is_err());
        }
    }

    #[test]
    fn slider_source_ranges_require_the_published_object_shape() {
        let schema = command_schema();
        let shape = schema["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .find(|shape| shape["properties"]["command"]["const"] == "setSliderSourceProperty")
            .unwrap();
        let range = &shape["properties"]["source_range"];
        assert_eq!(range["type"], "object");
        assert_eq!(range["required"], json!(["min", "max"]));
        assert_eq!(range["additionalProperties"], false);
        for range in [
            json!([1.0, 2.0]),
            json!([]),
            json!(true),
            json!(1.0),
            json!("1..2"),
        ] {
            let command = json!({"command":"setSliderSourceProperty","constraint":0,"property":"scaleX","source_range":range});
            assert!(serde_json::from_value::<CommandInput>(command).is_err());
        }
    }

    // Exhaustive matching makes any new engine variant a schema review gate.
    fn wire_name(command: &Command) -> &'static str {
        match command {
            Command::CreateSliderConstraint { .. } => "createSliderConstraint",
            Command::SetSliderSetupPose { .. } => "setSliderSetupPose",
            Command::SetSliderSource { .. } => "setSliderSource",
            Command::SetSliderSourceProperty { .. } => "setSliderSourceProperty",
            Command::EditSliderMappingRange { .. } => "editSliderMappingRange",
            Command::AddSliderConstraint { .. } => "addSliderConstraint",
            Command::SetSliderConstraint { .. } => "setSliderConstraint",
            Command::SetSliderKey { .. } => "setSliderKey",
            Command::InsertSliderKey { .. } => "insertSliderKey",
            Command::SetSliderCurve { .. } => "setSliderCurve",
            Command::AddSlot { .. } => "addSlot",
            Command::AddAttachment { .. } => "addAttachment",
            Command::AddIkConstraint { .. } => "addIkConstraint",
            Command::SetIkConstraint { .. } => "setIkConstraint",
            Command::AddTransformConstraint { .. } => "addTransformConstraint",
            Command::SetTransformConstraint { .. } => "setTransformConstraint",
            Command::EditTransformMappingRange { .. } => "editTransformMappingRange",
            Command::SetPathConstraint { .. } => "setPathConstraint",
            Command::AddPhysicsConstraint { .. } => "addPhysicsConstraint",
            Command::SetPhysicsConstraint { .. } => "setPhysicsConstraint",
            Command::SetIkKey { .. } => "setIkKey",
            Command::SetIkCurve { .. } => "setIkCurve",
            Command::DeleteIkKey { .. } => "deleteIkKey",
            Command::SetDeformKey { .. } => "setDeformKey",
            Command::DeleteDeformKey { .. } => "deleteDeformKey",
            Command::SetDeformCurve { .. } => "setDeformCurve",
            Command::MoveAnimationKeys { .. } => "moveAnimationKeys",
            Command::DeleteAnimationKeys { .. } => "deleteAnimationKeys",
            Command::PasteAnimationKeys { .. } => "pasteAnimationKeys",
            Command::PasteAnimationKeyGroup { .. } => "pasteAnimationKeyGroup",
            Command::SetPose { .. } => "setPose",
            Command::SetSetup { .. } => "setSetup",
            Command::RenameBone { .. } => "renameBone",
            Command::RenameSlot { .. } => "renameSlot",
            Command::SetBoneLength { .. } => "setBoneLength",
            Command::SetRegionTransform { .. } => "setRegionTransform",
            Command::SetSlotAttachment { .. } => "setSlotAttachment",
            Command::SetSlotColor { .. } => "setSlotColor",
            Command::SetSlotAttachmentKey { .. } => "setSlotAttachmentKey",
            Command::SetSlotColorKey { .. } => "setSlotColorKey",
            Command::SetDrawOrderKey { .. } => "setDrawOrderKey",
            Command::SetDrawOrder { .. } => "setDrawOrder",
            Command::SetSlotColorCurves { .. } => "setSlotColorCurves",
            Command::InsertSlotColorKey { .. } => "insertSlotColorKey",
            Command::SetKey { .. } => "setKey",
            Command::SetKeyCurve { .. } => "setKeyCurve",
            Command::ApplyCurvePreset { .. } => "applyCurvePreset",
            Command::EditCurveHandle { .. } => "editCurveHandle",
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
            Command::EditMeshBoneWeights { .. } => "editMeshBoneWeights",
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
        assert_eq!(variants.len(), 64);
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
    fn transform_mapping_ranges_publish_strict_f64_endpoints() {
        let schema = command_schema();
        let shape = schema["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .find(|shape| shape["properties"]["command"]["const"] == "editTransformMappingRange")
            .unwrap();
        assert_eq!(shape["additionalProperties"], false);
        assert_eq!(shape["properties"]["from"], transform_property_schema());
        assert_eq!(shape["properties"]["to"], transform_property_schema());
        assert_eq!(
            shape["properties"]["field"]["enum"],
            json!(["sourceMin", "sourceMax", "targetMin", "targetMax"])
        );
        let payload = json!({"command":"editTransformMappingRange","constraint":0,
            "from":"x","to":"rotate","field":"sourceMax","value":1.0000000000000002_f64});
        for field in ["sourceMin", "sourceMax", "targetMin", "targetMax"] {
            let mut command = payload.clone();
            command["field"] = json!(field);
            let decoded: CommandInput = serde_json::from_value(command.clone()).unwrap();
            assert_eq!(serde_json::to_value(decoded.0).unwrap(), command);
        }
        for (field, value) in [
            ("field", json!("source_min")),
            ("field", Value::Null),
            ("from", json!("rotation")),
            ("to", Value::Null),
            ("value", Value::Null),
            ("value", json!("100")),
            ("future", json!(true)),
        ] {
            let mut invalid = payload.clone();
            invalid[field] = value;
            assert!(serde_json::from_value::<CommandInput>(invalid).is_err());
        }
        for field in ["constraint", "from", "to", "field", "value"] {
            let mut invalid = payload.clone();
            invalid.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<CommandInput>(invalid).is_err());
        }
        assert!(serde_json::from_value::<TransformPropertyInput>(json!("rotation")).is_err());
        assert!(serde_json::from_value::<TransformPropertyInput>(Value::Null).is_err());
    }

    #[test]
    fn mesh_weight_edits_publish_strict_absolute_and_single_gesture_relative_shapes() {
        let schema = command_schema();
        let shape = schema["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .find(|shape| shape["properties"]["command"]["const"] == "editMeshBoneWeights")
            .unwrap();
        assert_eq!(shape["properties"]["vertices"]["minItems"], 1);
        assert_eq!(shape["properties"]["vertices"]["uniqueItems"], true);
        let mut payload = sample(shape);
        payload["vertices"] = json!([2, 3]);
        for edit in [
            json!({"mode":"absolute","weight":0.5}),
            json!({"mode":"relative","deltas":[2.0,-0.125]}),
        ] {
            payload["edit"] = edit;
            let command: CommandInput = serde_json::from_value(payload.clone()).unwrap();
            assert_eq!(serde_json::to_value(command.0).unwrap(), payload);
        }
        for edit in [
            Value::Null,
            json!({"mode":"future","weight":0.5}),
            json!({"mode":"absolute","weight":0.5,"deltas":[]}),
            json!({"mode":"relative","deltas":[0.1],"weight":0.5}),
        ] {
            payload["edit"] = edit;
            assert!(serde_json::from_value::<CommandInput>(payload.clone()).is_err());
        }
    }

    #[test]
    fn binding_pose_schema_preserves_legacy_and_explicit_inheritance_snapshots() {
        let schema = command_schema();
        for variant in schema["oneOf"].as_array().unwrap().iter().filter(|shape| {
            matches!(
                shape["properties"]["command"]["const"].as_str(),
                Some(
                    "bindMeshToBone"
                        | "unbindMesh"
                        | "addMeshBoneBinding"
                        | "setMeshBoneWeight"
                        | "editMeshBoneWeights"
                )
            )
        }) {
            let pose_schema = &variant["properties"]["pose"];
            assert_eq!(pose_schema["required"], json!(["locals"]));
            assert_eq!(pose_schema["properties"]["inherits"]["type"], "array");
            assert_eq!(
                pose_schema["properties"]["inherits"]["items"]["enum"],
                json!([
                    "normal",
                    "onlyTranslation",
                    "noRotationOrReflection",
                    "noScale",
                    "noScaleOrReflection"
                ])
            );
            let legacy = sample(variant);
            let decoded: CommandInput = serde_json::from_value(legacy.clone()).unwrap();
            assert_eq!(serde_json::to_value(decoded.0).unwrap(), legacy);
            let mut explicit = legacy.clone();
            explicit["pose"]["inherits"] = json!(["onlyTranslation"]);
            let decoded: CommandInput = serde_json::from_value(explicit.clone()).unwrap();
            assert_eq!(serde_json::to_value(decoded.0).unwrap(), explicit);
            for invalid in [Value::Null, json!("normal"), json!(["futureMode"])] {
                let mut malformed = legacy.clone();
                malformed["pose"]["inherits"] = invalid;
                assert!(serde_json::from_value::<CommandInput>(malformed).is_err());
            }
        }
    }

    #[test]
    fn draw_order_commands_publish_strict_raw_and_complete_permutation_payloads() {
        let raw = json!({"command":"setDrawOrderKey","animation":0,"key":{
            "time":0.5,"offsets":[{"slot":0,"offset":1},{"slot":1,"offset":-1},{"slot":2,"offset":0}]}});
        let semantic = json!({"command":"setDrawOrder","animation":0,"time":0.5,"order":[1,0,2]});
        for payload in [&raw, &semantic] {
            let decoded: CommandInput = serde_json::from_value(payload.clone()).unwrap();
            assert_eq!(serde_json::to_value(decoded.0).unwrap(), *payload);
            let mut unknown = payload.clone();
            unknown["bone"] = json!(0);
            assert!(serde_json::from_value::<CommandInput>(unknown).is_err());
        }
        for offset in [json!(2147483648i64), json!(-2147483649i64), json!(0.5)] {
            let mut bad = raw.clone();
            bad["key"]["offsets"][0]["offset"] = offset;
            assert!(serde_json::from_value::<CommandInput>(bad).is_err());
        }
        let mut missing = raw.clone();
        missing["key"].as_object_mut().unwrap().remove("offsets");
        assert!(serde_json::from_value::<CommandInput>(missing).is_err());
        let schema = command_schema();
        let variants = schema["oneOf"].as_array().unwrap();
        let semantic_schema = variants
            .iter()
            .find(|v| v["properties"]["command"]["const"] == "setDrawOrder")
            .unwrap();
        assert_eq!(semantic_schema["properties"]["order"]["uniqueItems"], true);
        let raw_schema = variants
            .iter()
            .find(|v| v["properties"]["command"]["const"] == "setDrawOrderKey")
            .unwrap();
        assert_eq!(
            raw_schema["properties"]["key"]["additionalProperties"],
            false
        );
        assert_eq!(
            raw_schema["properties"]["key"]["properties"]["offsets"]["items"]["additionalProperties"],
            false
        );
        let key = json!({"type":"drawOrder","time":0.5});
        let decoded: AnimationKeyInput = serde_json::from_value(key.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded.0).unwrap(), key);
        let paste = json!({"command":"pasteAnimationKeys","animation":0,"target":{"type":"drawOrder"},"time":0.5,
            "clipboard":{"type":"drawOrder","slots":["a","b"],"keys":[{"time":0.0,"offsets":[{"slot":0,"offset":1}]}]}});
        let decoded: CommandInput = serde_json::from_value(paste.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded.0).unwrap(), paste);
        let mut unknown_owner = paste;
        unknown_owner["target"]["slot"] = json!(0);
        assert!(serde_json::from_value::<CommandInput>(unknown_owner).is_err());
    }

    #[test]
    fn ik_curve_only_schema_keeps_f64_times_and_absolute_mix_controls() {
        let payload = json!({"command":"setIkCurve","animation":4,"constraint":0,"time":0.25,
            "curve":{"type":"bezier","time":[0.16666666666666669,0.8333333333333333],"mix":[-0.5,1.25]}});
        let decoded: CommandInput = serde_json::from_value(payload.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded.0).unwrap(), payload);
        for field in ["time", "curve", "constraint", "animation"] {
            let mut missing = payload.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<CommandInput>(missing).is_err());
        }
        for field in ["mix", "bend_positive", "key"] {
            let mut extra = payload.clone();
            extra[field] = json!(0);
            assert!(serde_json::from_value::<CommandInput>(extra).is_err());
        }
        let mut extra = payload;
        extra["curve"]["future"] = json!(true);
        assert!(serde_json::from_value::<CommandInput>(extra).is_err());
    }

    #[test]
    fn scalar_curve_only_schema_requires_exact_identity_without_values() {
        let schema = command_schema();
        let variant = schema["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .find(|value| value["properties"]["command"]["const"] == "setKeyCurve")
            .unwrap();
        assert_eq!(variant["additionalProperties"], false);
        assert!(variant["properties"].get("value").is_none());
        assert!(variant["properties"].get("key").is_none());
        for curve in [
            json!("linear"),
            json!("stepped"),
            json!({"bezier":[0.25,0.75,0.75,0.25]}),
            json!({"bezierOffset":[0.25,0.00000095367431640625,0.75,-0.00000095367431640625]}),
        ] {
            let payload = json!({"command":"setKeyCurve","animation":0,"bone":1,
                "property":"x","time":0.5,"curve":curve});
            let decoded: CommandInput = serde_json::from_value(payload.clone()).unwrap();
            assert_eq!(serde_json::to_value(decoded.0).unwrap(), payload);
            for name in ["time", "curve", "property", "bone", "animation"] {
                let mut missing = payload.clone();
                missing.as_object_mut().unwrap().remove(name);
                assert!(serde_json::from_value::<CommandInput>(missing).is_err());
            }
            let mut unsafe_payload = payload;
            unsafe_payload["value"] = json!(99);
            assert!(serde_json::from_value::<CommandInput>(unsafe_payload).is_err());
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
    #[test]
    fn deform_timeline_schema_preserves_dense_clipboards_and_strict_typed_owners() {
        let schema = command_schema();
        let reference = json!({"skin":"peach","slot":"body","placeholder":"body"});
        let identity = json!({"type":"deform","attachment":reference,"time":0.5});
        let mut commands = vec![
            json!({"command":"moveAnimationKeys","animation":0,"keys":[identity.clone()],"delta":0.25,"copy":false}),
            json!({"command":"deleteAnimationKeys","animation":0,"keys":[identity],}),
        ];
        for layout in [
            json!({"type":"unweighted","vertices":3}),
            json!({"type":"weighted","influence_counts":[1,2]}),
        ] {
            commands.push(json!({"command":"pasteAnimationKeys","animation":1,
                "target":{"type":"deform","attachment":reference},"time":1.0,
                "clipboard":{"type":"deform","layout":layout,"keys":[
                    {"time":0.0,"curve":{"bezier":[0.2f32,0.0,0.8f32,1.0]},"offsets":[0.0,1.0,2.0,3.0,4.0,5.0]}
                ]}}));
        }
        for curve in [
            json!("linear"),
            json!("stepped"),
            json!({"bezier":[0.2f32,0.0,0.8f32,1.0]}),
        ] {
            commands.push(json!({"command":"setDeformCurve","animation":0,"attachment":reference,"time":0.5,"curve":curve}));
        }
        for value in commands {
            let variant = schema["oneOf"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["properties"]["command"]["const"] == value["command"])
                .unwrap();
            assert_eq!(variant["additionalProperties"], false);
            let parsed: CommandInput = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(parsed.0).unwrap(), value);
            let mut bad = value;
            bad["extra"] = json!(true);
            assert!(serde_json::from_value::<CommandInput>(bad).is_err());
        }
        let bad = json!({"command":"pasteAnimationKeys","animation":0,"time":0.0,
            "target":{"type":"deform","attachment":reference},
            "clipboard":{"type":"deform","layout":{"type":"weighted","influence_counts":[1],"future":true},"keys":[]}});
        assert!(serde_json::from_value::<CommandInput>(bad).is_err());
    }

    #[test]
    fn slot_authoring_schema_retains_both_key_kinds_exact_bytes_and_channel_curves() {
        let curves = json!([{"bezier":[0.25,0.0,0.75,1.0]},"linear",{"bezierOffset":[0.25,2.0,0.75,-1.0]},"linear"]);
        let attachment = json!({"time":0.25,"attachment":null});
        let color = json!({"time":0.25,"color":[0,128,255,1],"curves":curves});
        let keys = json!([
            {"type":"slotAttachment","slot":8,"time":0.25},
            {"type":"slotColor","slot":8,"time":0.25}
        ]);
        let payloads = [
            json!({"command":"setSlotAttachmentKey","animation":0,"slot":8,"key":attachment}),
            json!({"command":"setSlotColorKey","animation":0,"slot":8,"key":color}),
            json!({"command":"setSlotColorCurves","animation":0,"slot":8,"time":0.25,"curves":curves}),
            json!({"command":"insertSlotColorKey","animation":0,"slot":8,"time":0.5,"color":[1.0,0.5,0.25,0.0]}),
            json!({"command":"moveAnimationKeys","animation":0,"keys":keys,"delta":0.5,"copy":false}),
            json!({"command":"deleteAnimationKeys","animation":0,"keys":keys}),
            json!({"command":"pasteAnimationKeys","animation":1,"target":{"type":"slot","slot":13},"time":1.0,
                "clipboard":{"type":"slot","attachment_keys":[attachment],"color_keys":[color]}}),
        ];
        for payload in &payloads {
            let parsed: CommandInput = serde_json::from_value(payload.clone()).unwrap();
            assert_eq!(serde_json::to_value(parsed.0).unwrap(), *payload);
            let mut bad = payload.clone();
            bad["futureField"] = json!(true);
            assert!(serde_json::from_value::<CommandInput>(bad).is_err());
        }
        for kind in ["slotAttachment", "slotColor"] {
            let value = json!({"type":kind,"slot":8,"time":0.25});
            let key: AnimationKeyInput = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(key.0).unwrap(), value);
            let mut bad = value;
            bad["bone"] = json!(0);
            assert!(serde_json::from_value::<AnimationKeyInput>(bad).is_err());
        }
        let schema = command_schema();
        let set_color = schema["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["properties"]["command"]["const"] == "setSlotColorKey")
            .unwrap();
        assert_eq!(
            set_color["properties"]["key"]["additionalProperties"],
            false
        );
        assert_eq!(
            set_color["properties"]["key"]["properties"]["color"]["maxItems"],
            4
        );
        assert_eq!(
            set_color["properties"]["key"]["properties"]["curves"]["minItems"],
            4
        );
        for invalid_color in [
            json!([256, 0, 0, 0]),
            json!([-1, 0, 0, 0]),
            json!([0.5, 0, 0, 0]),
            json!([0, 0, 0]),
        ] {
            let mut bad = payloads[1].clone();
            bad["key"]["color"] = invalid_color;
            assert!(serde_json::from_value::<CommandInput>(bad).is_err());
        }
        let mut missing_hide = payloads[0].clone();
        missing_hide["key"]
            .as_object_mut()
            .unwrap()
            .remove("attachment");
        // Option fields retain the native serde contract: omitted and explicit null both hide.
        assert!(serde_json::from_value::<CommandInput>(missing_hide).is_ok());
        let mut unknown = payloads[6].clone();
        unknown["clipboard"]["color_keys"][0]["futureField"] = json!(true);
        assert!(serde_json::from_value::<CommandInput>(unknown).is_err());
    }
}
