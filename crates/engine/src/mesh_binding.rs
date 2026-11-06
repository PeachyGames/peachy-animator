//! Explicit bind-pose transitions. Bind positions change; the captured bone pose does not.
use crate::{Affine, mesh_world_vertices, world_pose};
use peachy_document::{
    Attachment, AttachmentRef, BoneInfluence, Document, Mesh, MeshVertices, Transform,
    finite_transform,
};
use serde::{Deserialize, Serialize};

/// Bind/Remove-all are shape-preserving commands. Reject a recovered f32 local
/// position if rendering it would move either world coordinate by more than this
/// absolute amount. This command policy has no relative or large-pose exception.
const BIND_WORLD_COORDINATE_TOLERANCE: f64 = 0.001;

/// A reproducible local-pose snapshot, including any unkeyed Session pose edits.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeshBindPose {
    pub locals: Vec<Transform>,
}

fn bind_world(document: &Document, pose: &MeshBindPose) -> Result<Vec<Affine>, String> {
    if pose.locals.len() != document.bones.len()
        || pose.locals.iter().any(|local| !finite_transform(*local))
    {
        return Err("Bind pose needs one finite local transform per bone".into());
    }
    Ok(world_pose(document, &pose.locals))
}

fn mesh_target<'a>(
    document: &'a Document,
    attachment: &AttachmentRef,
) -> Result<(usize, &'a Mesh), String> {
    let slot = document
        .slots
        .iter()
        .find(|slot| slot.name == attachment.slot)
        .ok_or("Unknown attachment slot")?;
    let Some(Attachment::Mesh(mesh)) = document.attachment(attachment) else {
        return Err("Unknown mesh attachment".into());
    };
    Ok((slot.bone, mesh))
}

/// Invert only the matrix required by the operation. A small nonzero determinant
/// is valid; an absolute epsilon would reject ordinary small-scale bind poses.
struct Inverse {
    forward: Affine,
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    x: f64,
    y: f64,
}

impl Inverse {
    fn new(matrix: Affine) -> Result<Self, String> {
        if [matrix.a, matrix.b, matrix.c, matrix.d, matrix.x, matrix.y]
            .iter()
            .any(|value| !value.is_finite())
        {
            return Err("Binding requires a finite bone transform".into());
        }
        let [a, b, c, d] = [matrix.a, matrix.b, matrix.c, matrix.d].map(f64::from);
        let det = a * d - b * c;
        if det == 0.0 {
            return Err("Cannot bind through a singular bone transform".into());
        }
        Ok(Self {
            forward: matrix,
            a: d / det,
            b: -b / det,
            c: -c / det,
            d: a / det,
            x: f64::from(matrix.x),
            y: f64::from(matrix.y),
        })
    }

    fn point(&self, point: [f32; 2]) -> Result<[f32; 2], String> {
        if point.iter().any(|value| !value.is_finite()) {
            return Err("Binding requires finite world vertices".into());
        }
        let x = f64::from(point[0]) - self.x;
        let y = f64::from(point[1]) - self.y;
        let result = [
            (self.a * x + self.b * y) as f32,
            (self.c * x + self.d * y) as f32,
        ];
        if result.iter().any(|value| !value.is_finite()) {
            return Err("Binding positions exceed the finite coordinate range".into());
        }
        // Native positions can be finite while cancelling f32 matrix products
        // overflow during rendering. Reject that result before changing bindings.
        let recovered = self.forward.point(result[0], result[1]);
        if recovered.iter().any(|value| !value.is_finite()) {
            return Err("Binding would produce nonfinite world vertices".into());
        }
        if recovered.into_iter().zip(point).any(|(actual, original)| {
            (f64::from(actual) - f64::from(original)).abs() > BIND_WORLD_COORDINATE_TOLERANCE
        }) {
            return Err("Pose exceeds coordinate precision: binding would move vertices beyond the 0.001 world-coordinate tolerance".into());
        }
        Ok(result)
    }
}

pub(super) fn bind_mesh_to_bone(
    document: &mut Document,
    attachment: &AttachmentRef,
    bone: usize,
    pose: &MeshBindPose,
) -> Result<(), String> {
    let world = bind_world(document, pose)?;
    if bone >= document.bones.len() {
        return Err("Unknown bind bone".into());
    }
    let (slot_bone, mesh) = mesh_target(document, attachment)?;
    if mesh.is_weighted() {
        return Err("Initial binding requires an unweighted mesh".into());
    }
    let inverse = Inverse::new(world[bone])?;
    let influences = mesh_world_vertices(mesh, slot_bone, &world)
        .into_iter()
        .map(|point| {
            Ok(vec![BoneInfluence {
                bone,
                position: inverse.point(point)?,
                weight: 1.0,
            }])
        })
        .collect::<Result<Vec<_>, String>>()?;
    let Some(Attachment::Mesh(mesh)) = document.attachment_mut(attachment) else {
        unreachable!("Validated mesh target must remain present")
    };
    mesh.vertices = MeshVertices::Weighted { influences };
    Ok(())
}

pub(super) fn unbind_mesh(
    document: &mut Document,
    attachment: &AttachmentRef,
    pose: &MeshBindPose,
) -> Result<(), String> {
    let world = bind_world(document, pose)?;
    let (slot_bone, mesh) = mesh_target(document, attachment)?;
    if !mesh.is_weighted() {
        return Ok(());
    }
    let inverse = Inverse::new(world[slot_bone])?;
    let positions = mesh_world_vertices(mesh, slot_bone, &world)
        .into_iter()
        .map(|point| inverse.point(point))
        .collect::<Result<Vec<_>, _>>()?;
    let Some(Attachment::Mesh(mesh)) = document.attachment_mut(attachment) else {
        unreachable!("Validated mesh target must remain present")
    };
    mesh.vertices = MeshVertices::SlotLocal { positions };
    Ok(())
}
