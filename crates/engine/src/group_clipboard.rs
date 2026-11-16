//! Portable multi-owner clipboard data. Existing leaf codecs remain unchanged;
//! this new boundary enforces their published object/token shapes explicitly.
use crate::{AnimationKeyClipboard, ClipboardKey, DeformLayout, KeyOwner, SliderClipboardKey};
use peachy_document::{
    AttachmentKey, AttachmentRef, ColorKey, Curve, DeformKey, DrawOrderKey, DrawOrderOffset,
    IkCurve, IkKeyframe, Keyframe, Property, SliderProperty,
};
use serde::{
    Deserialize, Deserializer, Serialize,
    de::{Error, MapAccess, Visitor},
};
use std::{fmt, marker::PhantomData};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AnimationKeyGroupClipboard {
    /// First source-owner appearance order. Every key is relative to the same
    /// group origin; an individual entry need not contain a zero-time key.
    pub entries: Vec<AnimationKeyClipboard>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationKeyGroupCopy {
    pub clipboard: AnimationKeyGroupClipboard,
    /// Separate transient receipt, not automatic mapping in portable data.
    pub source_owners: Vec<KeyOwner>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AnimationKeyGroupTarget {
    pub entry: usize,
    pub target: KeyOwner,
}

fn object<'de, D: Deserializer<'de>, T: Deserialize<'de>>(deserializer: D) -> Result<T, D::Error> {
    struct ObjectVisitor<T>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>> Visitor<'de> for ObjectVisitor<T> {
        type Value = T;
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an object")
        }
        fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
            T::deserialize(serde::de::value::MapAccessDeserializer::new(map))
        }
    }
    deserializer.deserialize_map(ObjectVisitor(PhantomData))
}

struct Object<T>(T);
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Object<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self(object(deserializer)?))
    }
}

struct PropertyInput(Property);
impl<'de> Deserialize<'de> for PropertyInput {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self(match String::deserialize(deserializer)?.as_str() {
            "x" => Property::X,
            "y" => Property::Y,
            "rotation" => Property::Rotation,
            "scaleX" => Property::ScaleX,
            "scaleY" => Property::ScaleY,
            "shearX" => Property::ShearX,
            "shearY" => Property::ShearY,
            other => {
                return Err(D::Error::unknown_variant(
                    other,
                    &["x", "y", "rotation", "scaleX", "scaleY", "shearX", "shearY"],
                ));
            }
        }))
    }
}

struct CurveInput(Curve);
impl<'de> Deserialize<'de> for CurveInput {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct CurveVisitor;
        impl<'de> Visitor<'de> for CurveVisitor {
            type Value = CurveInput;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("linear, stepped, or one Bezier control object")
            }
            fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
                match value {
                    "linear" => Ok(CurveInput(Curve::Linear)),
                    "stepped" => Ok(CurveInput(Curve::Stepped)),
                    other => Err(E::unknown_variant(other, &["linear", "stepped"])),
                }
            }
            fn visit_string<E: Error>(self, value: String) -> Result<Self::Value, E> {
                self.visit_str(&value)
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let name = map
                    .next_key::<String>()?
                    .ok_or_else(|| A::Error::custom("Missing Bezier controls"))?;
                let curve = match name.as_str() {
                    "bezier" => Curve::Bezier(map.next_value::<[f32; 4]>()?),
                    "bezierOffset" => Curve::BezierOffset(map.next_value::<[f32; 4]>()?),
                    other => {
                        return Err(A::Error::unknown_field(other, &["bezier", "bezierOffset"]));
                    }
                };
                if map.next_key::<String>()?.is_some() {
                    return Err(A::Error::custom(
                        "Curve object needs exactly one control field",
                    ));
                }
                Ok(CurveInput(curve))
            }
        }
        deserializer.deserialize_any(CurveVisitor)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KeyInput {
    time: f32,
    value: f32,
    curve: CurveInput,
}
impl From<KeyInput> for Keyframe {
    fn from(key: KeyInput) -> Self {
        Self {
            time: key.time,
            value: key.value,
            curve: key.curve.0,
        }
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BoneInput {
    property: PropertyInput,
    key: Object<KeyInput>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SliderInput {
    property: SliderProperty,
    key: Object<KeyInput>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IkInput {
    time: f32,
    mix: f32,
    bend_positive: bool,
    curve: Object<IkCurve>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeformInput {
    time: f32,
    curve: CurveInput,
    offsets: Vec<f32>,
}
fn required_name<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(deserializer)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AttachmentInput {
    time: f32,
    #[serde(deserialize_with = "required_name")]
    attachment: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ColorInput {
    time: f32,
    color: [u8; 4],
    curves: [CurveInput; 4],
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OrderInput {
    time: f32,
    offsets: Vec<Object<DrawOrderOffset>>,
}

struct LeafInput(AnimationKeyClipboard);
impl<'de> Deserialize<'de> for LeafInput {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
        enum Wire {
            Bone {
                keys: Vec<Object<BoneInput>>,
            },
            Slider {
                keys: Vec<Object<SliderInput>>,
            },
            Ik {
                keys: Vec<Object<IkInput>>,
            },
            Deform {
                layout: Object<DeformLayout>,
                keys: Vec<Object<DeformInput>>,
            },
            Slot {
                attachment_keys: Vec<Object<AttachmentInput>>,
                color_keys: Vec<Object<ColorInput>>,
            },
            DrawOrder {
                slots: Vec<String>,
                keys: Vec<Object<OrderInput>>,
            },
        }
        Ok(Self(match object(deserializer)? {
            Wire::Bone { keys } => AnimationKeyClipboard::Bone {
                keys: keys
                    .into_iter()
                    .map(|key| ClipboardKey {
                        property: key.0.property.0,
                        key: key.0.key.0.into(),
                    })
                    .collect(),
            },
            Wire::Slider { keys } => AnimationKeyClipboard::Slider {
                keys: keys
                    .into_iter()
                    .map(|key| SliderClipboardKey {
                        property: key.0.property,
                        key: key.0.key.0.into(),
                    })
                    .collect(),
            },
            Wire::Ik { keys } => AnimationKeyClipboard::Ik {
                keys: keys
                    .into_iter()
                    .map(|key| IkKeyframe {
                        time: key.0.time,
                        mix: key.0.mix,
                        bend_positive: key.0.bend_positive,
                        curve: key.0.curve.0,
                    })
                    .collect(),
            },
            Wire::Deform { layout, keys } => AnimationKeyClipboard::Deform {
                layout: layout.0,
                keys: keys
                    .into_iter()
                    .map(|key| DeformKey {
                        time: key.0.time,
                        curve: key.0.curve.0,
                        offsets: key.0.offsets,
                    })
                    .collect(),
            },
            Wire::Slot {
                attachment_keys,
                color_keys,
            } => AnimationKeyClipboard::Slot {
                attachment_keys: attachment_keys
                    .into_iter()
                    .map(|key| AttachmentKey {
                        time: key.0.time,
                        attachment: key.0.attachment,
                    })
                    .collect(),
                color_keys: color_keys
                    .into_iter()
                    .map(|key| ColorKey {
                        time: key.0.time,
                        color: key.0.color,
                        curves: key.0.curves.map(|curve| curve.0),
                    })
                    .collect(),
            },
            Wire::DrawOrder { slots, keys } => AnimationKeyClipboard::DrawOrder {
                slots,
                keys: keys
                    .into_iter()
                    .map(|key| DrawOrderKey {
                        time: key.0.time,
                        offsets: key.0.offsets.into_iter().map(|offset| offset.0).collect(),
                    })
                    .collect(),
            },
        }))
    }
}

impl<'de> Deserialize<'de> for AnimationKeyGroupClipboard {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            entries: Vec<LeafInput>,
        }
        let fields: Fields = object(deserializer)?;
        Ok(Self {
            entries: fields.entries.into_iter().map(|entry| entry.0).collect(),
        })
    }
}

struct OwnerInput(KeyOwner);
impl<'de> Deserialize<'de> for OwnerInput {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
        enum Wire {
            Bone { bone: usize },
            Ik { constraint: usize },
            Slider { constraint: usize },
            Deform { attachment: Object<AttachmentRef> },
            Slot { slot: usize },
            DrawOrder {},
        }
        Ok(Self(match object(deserializer)? {
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

impl<'de> Deserialize<'de> for AnimationKeyGroupTarget {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            entry: usize,
            target: OwnerInput,
        }
        let fields: Fields = object(deserializer)?;
        Ok(Self {
            entry: fields.entry,
            target: fields.target.0,
        })
    }
}
