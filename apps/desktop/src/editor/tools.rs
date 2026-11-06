//! Native controls translate input into shared engine transform operations.
use super::*;
use peachy_engine::{set_transform_value, transform_values, world_pose};

impl Tool {
    pub(super) fn transform_tool(self) -> Option<TransformTool> {
        match self {
            Self::Rotate => Some(TransformTool::Rotate),
            Self::Move => Some(TransformTool::Translate),
            Self::Scale => Some(TransformTool::Scale),
            Self::Shear => Some(TransformTool::Shear),
            Self::Mesh | Self::Weights => None,
        }
    }
    pub(super) fn properties(self) -> &'static [Property] {
        match self {
            Self::Rotate => &[Property::Rotation],
            Self::Move => &[Property::X, Property::Y],
            Self::Scale => &[Property::ScaleX, Property::ScaleY],
            Self::Shear => &[Property::ShearX, Property::ShearY],
            Self::Mesh | Self::Weights => &[],
        }
    }
}

pub(super) fn item_parent(item: &TreeItem, doc: &Document, world: &[Affine]) -> Option<Affine> {
    let identity = Affine::from_transform(Transform::default());
    match item {
        TreeItem::Bone(b) => Some(doc.bones.get(*b)?.parent.map_or(identity, |p| world[p])),
        TreeItem::Attachment(k) => Some(world[doc.slots.iter().find(|s| s.name == k.slot)?.bone]),
        _ => None,
    }
}
impl Editor {
    pub(super) fn selected_parent(&self) -> Option<Affine> {
        let world = world_pose(
            self.session.document(),
            &self.session.local_pose(self.active_animation(), self.time),
        );
        item_parent(self.selection.as_ref()?, self.session.document(), &world)
    }
    pub(super) fn set_axes(&mut self, axes: Axes) {
        if self.axes != axes {
            self.cancel_gesture();
            self.axes = axes;
        }
    }
    pub(super) fn edit_display_value(&mut self, property: Property, value: f32) {
        let Some((local, parent)) = self.selected_transform().zip(self.selected_parent()) else {
            return;
        };
        match set_transform_value(local, parent, self.axes, property, value) {
            Ok(mut transform) => {
                if self.setup_mode && property == Property::Rotation {
                    transform.rotation = transform.rotation.rem_euclid(360.);
                }
                let properties = if matches!(property, Property::X | Property::Y) {
                    &[Property::X, Property::Y][..]
                } else {
                    std::slice::from_ref(&property)
                };
                self.edit_transform(transform, properties);
            }
            Err(error) => self.error = Some(error),
        }
    }
    pub(super) fn nudge_transform(&mut self, delta: [f32; 2], fine: bool) {
        let Some((local, parent)) = self.selected_transform().zip(self.selected_parent()) else {
            return;
        };
        let shown = transform_values(local, parent, self.axes);
        let amount = if fine { 0.1 } else { 1. };
        match self.tool {
            Tool::Rotate => self.edit_display_value(
                Property::Rotation,
                shown.rotation + (delta[0] + delta[1]) * amount,
            ),
            Tool::Move => {
                let first = if delta[0] != 0. {
                    Property::X
                } else {
                    Property::Y
                };
                self.edit_display_value(first, first.read(shown) + (delta[0] + delta[1]) * amount);
            }
            Tool::Scale => {
                let property = if delta[0] != 0. {
                    Property::ScaleX
                } else {
                    Property::ScaleY
                };
                self.edit_display_value(
                    property,
                    property.read(local) + (delta[0] + delta[1]) * amount * 0.01,
                );
            }
            Tool::Shear => {
                let property = if delta[0] != 0. {
                    Property::ShearX
                } else {
                    Property::ShearY
                };
                self.edit_display_value(
                    property,
                    property.read(local) + (delta[0] + delta[1]) * amount,
                );
            }
            Tool::Mesh | Tool::Weights => {}
        }
    }
}
