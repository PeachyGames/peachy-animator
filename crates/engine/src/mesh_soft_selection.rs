//! Pure unweighted-mesh translation snapshots. The caller owns gesture/history lifecycle.
use crate::{Affine, Command};
use peachy_document::{AttachmentRef, Mesh};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshSoftSelection {
    /// Outer radius in world units, independent of the viewport zoom.
    pub size: f32,
    /// Fraction of the radius over which weights fall linearly from one to zero.
    pub feather: f32,
    /// Include unselected hull vertices. The selected vertex is always included.
    pub hull_vertices: bool,
}

impl Default for MeshSoftSelection {
    fn default() -> Self {
        Self {
            size: 0.,
            feather: 0.5,
            hull_vertices: true,
        }
    }
}

/// Frozen slot-local positions, world-distance weights and the applied slot frame.
/// Repeated previews always derive from this snapshot, never from an earlier preview.
#[derive(Clone, Debug)]
pub struct MeshTranslation {
    positions: Vec<[f32; 2]>,
    weights: Vec<f32>,
    slot_world: Affine,
}

impl MeshTranslation {
    pub fn new(
        mesh: &Mesh,
        slot_world: Affine,
        selected: usize,
        settings: MeshSoftSelection,
    ) -> Result<Self, String> {
        if !settings.size.is_finite() || settings.size < 0. {
            return Err("Soft selection Size must be finite and nonnegative".into());
        }
        if !settings.feather.is_finite() || !(0.0..=1.0).contains(&settings.feather) {
            return Err("Soft selection Feather must be finite and in 0–1".into());
        }
        let positions = mesh
            .slot_local_positions()
            .ok_or("Soft selection translation is not supported for weighted meshes")?;
        mesh.validate()?;
        let center = *positions.get(selected).ok_or("Unknown mesh vertex")?;
        if ![
            slot_world.a,
            slot_world.b,
            slot_world.c,
            slot_world.d,
            slot_world.x,
            slot_world.y,
        ]
        .into_iter()
        .all(f32::is_finite)
        {
            return Err("Soft selection needs a finite slot world transform".into());
        }
        let radius = f64::from(settings.size);
        let inner = radius * (1. - f64::from(settings.feather));
        let mut weights = Vec::with_capacity(positions.len());
        for (index, &position) in positions.iter().enumerate() {
            check_projection(slot_world, position)?;
            let weight = if index == selected {
                1.
            } else if settings.size == 0. || (!settings.hull_vertices && index < mesh.hull) {
                0.
            } else {
                // Transform the difference directly. Subtracting projected positions
                // would lose a small distance at a very large translated origin.
                let dx = f64::from(position[0]) - f64::from(center[0]);
                let dy = f64::from(position[1]) - f64::from(center[1]);
                let world_x = f64::from(slot_world.a) * dx + f64::from(slot_world.b) * dy;
                let world_y = f64::from(slot_world.c) * dx + f64::from(slot_world.d) * dy;
                let distance = world_x.hypot(world_y);
                if distance <= inner {
                    // Feather zero is a hard radius, including its boundary.
                    1.
                } else if distance >= radius {
                    0.
                } else {
                    ((radius - distance) / (radius - inner)) as f32
                }
            };
            weights.push(weight);
        }
        Ok(Self {
            positions: positions.to_vec(),
            weights,
            slot_world,
        })
    }

    pub fn weights(&self) -> &[f32] {
        &self.weights
    }

    /// Numeric/nudge translation in slot-local coordinates. The returned batch
    /// contains every affected vertex and is validated/applied atomically as usual.
    /// Baseline-equal positions must be included to restore earlier live previews.
    pub fn command(&self, attachment: AttachmentRef, delta: [f32; 2]) -> Result<Command, String> {
        self.translate(attachment, delta.map(f64::from))
    }

    /// Viewport translation in world coordinates, using the frozen linear inverse.
    /// Keep inverse deltas wide until adding the baseline: an intermediate delta
    /// can exceed f32 while the resulting coordinate is still representable.
    pub fn world_command(
        &self,
        attachment: AttachmentRef,
        delta: [f32; 2],
    ) -> Result<Command, String> {
        let [x, y] = delta.map(f64::from);
        if ![x, y].into_iter().all(f64::is_finite) {
            return Err("Mesh translation delta must be finite".into());
        }
        let [a, b, c, d] = [
            self.slot_world.a,
            self.slot_world.b,
            self.slot_world.c,
            self.slot_world.d,
        ]
        .map(f64::from);
        let determinant = a * d - b * c;
        if !determinant.is_finite() || determinant == 0. {
            return Err("Mesh world translation needs an invertible slot transform".into());
        }
        self.translate(
            attachment,
            [(d * x - b * y) / determinant, (a * y - c * x) / determinant],
        )
    }

    fn translate(&self, attachment: AttachmentRef, delta: [f64; 2]) -> Result<Command, String> {
        if !delta.into_iter().all(f64::is_finite) {
            return Err("Mesh translation delta must be finite".into());
        }
        let mut commands = Vec::new();
        for (vertex, (&baseline, &weight)) in self.positions.iter().zip(&self.weights).enumerate() {
            if weight == 0. {
                continue;
            }
            let position = [0, 1].map(|axis| {
                if delta[axis] == 0. {
                    // Even -0 is unchanged on an axis that was not translated.
                    baseline[axis]
                } else {
                    (f64::from(baseline[axis]) + delta[axis] * f64::from(weight)) as f32
                }
            });
            if !position.into_iter().all(f32::is_finite) {
                return Err(
                    "Mesh translation produces a coordinate outside the finite f32 range".into(),
                );
            }
            check_projection(self.slot_world, position)?;
            commands.push(Command::SetMeshVertex {
                attachment: attachment.clone(),
                vertex,
                position,
            });
        }
        Ok(Command::Batch { commands })
    }
}

fn check_projection(frame: Affine, position: [f32; 2]) -> Result<(), String> {
    // Match the existing native scene projection, including its finite-output gate.
    if frame
        .point(position[0], position[1])
        .into_iter()
        .all(f32::is_finite)
    {
        Ok(())
    } else {
        Err("Mesh translation has a nonfinite world projection".into())
    }
}
