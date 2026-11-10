//! Bounded normal IK from elementary link geometry and original public pose
//! observations. No external implementation or runtime constant was inspected.
use crate::{
    Affine, IkPose, ik::check_world, pose::OBSERVED_DEGREES_TO_RADIANS, pose::inherit_linear,
    world_pose_with,
};
use peachy_document::{Document, IkConstraint, Inherit, Transform};

/// Applied f32 frames and bone endpoints must retain their solved f64 reference
/// within this absolute bound. There is no relative or large-coordinate exception.
pub(crate) const IK_WORLD_COORDINATE_TOLERANCE: f64 = 0.001;

/// What path constraints need beyond bone data: sampled position/spacing and
/// the attachment each slot shows under the evaluated skin.
pub(crate) struct PathContext<'a> {
    pub poses: &'a [crate::PathPose],
    pub slot_attachments: &'a [Option<String>],
    pub skin: &'a str,
    /// Keyed deform offsets (paths can be deformed).
    pub deforms: &'a [(peachy_document::AttachmentRef, Vec<f32>)],
}

pub(crate) fn apply(
    doc: &Document,
    poses: &[IkPose],
    locals: &mut [Transform],
    world: &mut Vec<Affine>,
    inherits: &[Inherit],
    enabled: &[bool],
    paths: &PathContext,
) -> Result<(), String> {
    // Skin-inactive constraints neither run nor take part in domain checks.
    let active: Vec<_> = doc
        .constraints
        .iter()
        .zip(poses)
        .zip(enabled)
        .filter_map(|((constraint, pose), &on)| {
            let ik = constraint.as_ik()?;
            (on && pose.mix != 0.0).then_some((ik, *pose))
        })
        .collect();
    for (index, (ik, _)) in active.iter().enumerate() {
        validate_domain(doc, locals, ik)?;
        // Spine: bones affected by IK cannot disable transform inheritance.
        if let Some(&bone) = ik
            .bones
            .iter()
            .find(|&&bone| inherits[bone] != Inherit::Normal)
        {
            return Err(format!(
                "IK {}: constrained bone {} must inherit normally",
                ik.name, doc.bones[bone].name
            ));
        }
        for (other, _) in &active[..index] {
            // Nested chains (a toe inside a leg) solve sequentially in constraint
            // order: each solve writes applied locals and rebuilds world frames
            // before the next. Shared bones and targets inside another chain's
            // subtree remain unsupported.
            if ik.bones.iter().any(|bone| other.bones.contains(bone))
                || within(doc, ik.target, other.bones[0])
                || within(doc, other.target, ik.bones[0])
            {
                return Err(format!(
                    "IK {} and {}: shared constrained bones or targets inside another chain are not supported",
                    other.name, ik.name
                ));
            }
        }
    }
    // All constraints run in list order: IK solves and transform constraints
    // interleave; physics is data only.
    for (index, constraint) in doc.constraints.iter().enumerate() {
        if !enabled[index] {
            continue;
        }
        match constraint {
            peachy_document::Constraint::Ik(ik) if poses[index].mix != 0.0 => {
                solve(doc, locals, world, ik, poses[index], inherits)
                    .map_err(|error| format!("IK {}: {error}", ik.name))?;
            }
            peachy_document::Constraint::Transform(transform) => {
                crate::transform_constraint::apply(doc, transform, locals, world, inherits)
                    .map_err(|error| format!("Transform {}: {error}", transform.name))?;
            }
            peachy_document::Constraint::Path(path) => {
                // Like the runtime, a slot not showing a path attachment leaves
                // the bones alone.
                let shown = paths.slot_attachments[path.slot]
                    .as_deref()
                    .and_then(|name| doc.resolve_attachment(paths.skin, path.slot, name));
                if let Some((key, peachy_document::Attachment::Path(curve))) = shown {
                    let offsets = paths
                        .deforms
                        .iter()
                        .find(|(target, _)| *target == key)
                        .map(|(_, offsets)| offsets.as_slice());
                    crate::path_constraint::apply(
                        doc,
                        path,
                        paths.poses[index],
                        curve,
                        offsets,
                        locals,
                        world,
                        inherits,
                    )
                    .map_err(|error| format!("Path {}: {error}", path.name))?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn within(doc: &Document, bone: usize, ancestor: usize) -> bool {
    let mut current = Some(bone);
    while let Some(index) = current {
        if index == ancestor {
            return true;
        }
        current = doc.bones[index].parent;
    }
    false
}

fn validate_domain(doc: &Document, locals: &[Transform], ik: &IkConstraint) -> Result<(), String> {
    if doc.bones[ik.bones[0]].parent.is_none() {
        return Err(format!(
            "IK {}: a constrained root bone is not supported",
            ik.name
        ));
    }
    // Constrained bones need equal-magnitude X/Y scale and zero shear; the
    // first bone may be mirrored, a child scales uniformly and positively.
    // Ancestors may have any scale, shear or reflection: the solve works in the
    // parent frame (docs/slices/ik-design.md addendum,
    // docs/slices/ik-scale-stretch.md).
    for (position, &bone) in ik.bones.iter().enumerate() {
        let local = locals[bone];
        if local.scale_x.abs() != local.scale_y.abs()
            || local.scale_x == 0.
            || (position == 1 && (local.scale_x < 0. || local.scale_y < 0.))
            || local.shear_x != 0.
            || local.shear_y != 0.
        {
            return Err(format!(
                "IK {}: bone {} requires equal-magnitude scale (a mirrored first bone is allowed) and zero shear",
                ik.name, doc.bones[bone].name
            ));
        }
    }
    if ik.bones.len() == 2 {
        let child = ik.bones[1];
        let setup = doc.bones[child].setup;
        if locals[child].x != setup.x
            || locals[child].y != setup.y
            || f64::from(setup.x).hypot(f64::from(setup.y)) == 0.
            || doc.bones[child].length <= 0.
        {
            return Err(format!(
                "IK {}: child requires a constant nonzero setup offset and positive length",
                ik.name
            ));
        }
    }
    Ok(())
}

fn solve(
    doc: &Document,
    locals: &mut [Transform],
    world: &mut Vec<Affine>,
    ik: &IkConstraint,
    pose: IkPose,
    inherits: &[Inherit],
) -> Result<(), String> {
    let first = ik.bones[0];
    let parent_index = doc.bones[first]
        .parent
        .ok_or("Constrained root is not supported")?;
    let incoming = wide_pose(doc, locals, inherits);
    check_input_precision(world, &incoming, parent_index, first, ik.target)?;
    if WideAffine::from(world[parent_index]).determinant() == 0. {
        return Err("target requires an invertible finite parent frame".into());
    }
    // Deriving an angle from a rounded target can turn a ten-unit offset at
    // origin1e8 into eight. Tiny lost offsets can also rotate a zero-length
    // bone incorrectly without violating an endpoint-only precision bound.
    let target = incoming[ik.target];
    let [x, y] = incoming[parent_index]
        .inverse_point([target.x, target.y])
        .ok_or("target requires an invertible finite parent frame")?;
    let dx = x - f64::from(locals[first].x);
    let dy = y - f64::from(locals[first].y);
    let distance = dx.hypot(dy);
    if !distance.is_finite() || distance == 0. {
        return Err("target must have a finite noncoincident direction".into());
    }
    let (s1x, s1y) = (
        f64::from(locals[first].scale_x),
        f64::from(locals[first].scale_y),
    );
    // A mirrored bone's X axis points the other way: turn 180° further.
    let mirror = if s1x < 0. { 180. } else { 0. };
    let mut desired = vec![(first, dy.atan2(dx) / OBSERVED_DEGREES_TO_RADIANS - mirror)];
    // Out-of-reach targets stretch the first bone along X (observed: factor
    // distance / reach, blended by mix like the rotation).
    let mut stretch = None;
    if ik.bones.len() == 1 {
        let reach = f64::from(doc.bones[first].length) * s1x.abs();
        if ik.stretch && reach > 0. && distance > reach {
            stretch = Some(distance / reach);
        }
    }
    if ik.bones.len() == 2 {
        let child = ik.bones[1];
        // The child origin sits at (x, y) in the first bone's frame, scaled by
        // that bone's scale: a link of length `a` at angle `offset` from its
        // X axis. The child's length scales by both bones' scales. Elementary
        // triangle geometry, verified against the external runtime for
        // uniform, mirrored and stretched chains.
        // Observed: with stretch on, the runtime solves as if the child had
        // no Y offset (results equal the on-axis case exactly).
        let child_y = if ik.stretch {
            0.
        } else {
            f64::from(locals[child].y)
        };
        let (cx, cy) = (f64::from(locals[child].x) * s1x, child_y * s1y);
        let a = cx.hypot(cy);
        let offset = cy.atan2(cx);
        let b = f64::from(doc.bones[child].length) * s1x.abs() * f64::from(locals[child].scale_x);
        if ik.stretch && distance > a + b && a + b > 0. {
            stretch = Some(distance / (a + b));
        }
        // Reach limiting is geometry, not normalization of authored angles.
        // Branch before division to avoid extreme but finite length ratios
        // overflowing a cosine that is already known to be outside reach.
        let cosine = if distance >= a + b {
            1.
        } else if distance <= (a - b).abs() {
            -1.
        } else {
            ((distance * distance - a * a - b * b) / (2. * a * b)).clamp(-1., 1.)
        };
        let bend = cosine.acos() * if pose.bend_positive { 1. } else { -1. };
        let parent_angle = dy.atan2(dx) - (b * bend.sin()).atan2(a + b * bend.cos()) - offset;
        desired[0].1 = parent_angle / OBSERVED_DEGREES_TO_RADIANS;
        // The child's world X angle relative to the first bone is bend + offset;
        // undo the first bone's (diagonal) scale to get its local rotation.
        let alpha = bend + offset;
        let child_angle = (alpha.sin() / s1y).atan2(alpha.cos() / s1x);
        desired.push((child, child_angle / OBSERVED_DEGREES_TO_RADIANS));
    }
    let mut solved = Vec::with_capacity(desired.len());
    for (bone, angle) in desired {
        let authored = f64::from(locals[bone].rotation);
        let mut delta = angle - authored;
        // Independently inferred from original ±181/359/720/1080 and exact
        // 180-degree tie observations. Exactly one correction, never modulo.
        if delta <= -180. {
            delta += 360.;
        } else if delta > 180. {
            delta -= 360.;
        }
        let rotation = authored + delta * pose.mix;
        if !rotation.is_finite() || !(rotation as f32).is_finite() {
            return Err("applied rotation is outside the finite coordinate range".into());
        }
        solved.push((bone, rotation));
    }
    let stretched_scale = stretch.map(|factor| (s1x * (1. + (factor - 1.) * pose.mix)) as f32);
    // Observed: an active two-bone stretch IK also moves the child onto the
    // first bone's X axis (applied Y = 0, not blended by mix).
    let flatten_child = (ik.stretch && ik.bones.len() == 2)
        .then_some(ik.bones.get(1).copied())
        .flatten()
        .filter(|&child| locals[child].y != 0.);
    if solved
        .iter()
        .all(|&(bone, rotation)| rotation == f64::from(locals[bone].rotation))
        && stretched_scale.is_none_or(|scale| scale == locals[first].scale_x)
        && flatten_child.is_none()
    {
        return Ok(());
    }
    if let Some(child) = flatten_child {
        locals[child].y = 0.;
    }
    for &(bone, rotation) in &solved {
        locals[bone].rotation = rotation as f32;
    }
    if let Some(scale) = stretched_scale {
        if !scale.is_finite() {
            return Err("stretched scale is outside the finite range".into());
        }
        locals[first].scale_x = scale;
    }
    let candidate = world_pose_with(doc, locals, inherits);
    check_world(doc, &candidate)?;
    check_reconstruction(doc, locals, &incoming, &candidate, first, &solved, inherits)?;
    *world = candidate;
    Ok(())
}

/// Narrow reference math for this operation. It does not replace the shared
/// f32 evaluator or decompose/rewrite an authored affine transform.
#[derive(Clone, Copy)]
struct WideAffine {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    x: f64,
    y: f64,
}

impl From<Affine> for WideAffine {
    fn from(m: Affine) -> Self {
        Self {
            a: f64::from(m.a),
            b: f64::from(m.b),
            c: f64::from(m.c),
            d: f64::from(m.d),
            x: f64::from(m.x),
            y: f64::from(m.y),
        }
    }
}
impl WideAffine {
    fn from_local(t: Transform, rotation: f64) -> Self {
        let x = (rotation + f64::from(t.shear_x)) * OBSERVED_DEGREES_TO_RADIANS;
        let y = (rotation + 90. + f64::from(t.shear_y)) * OBSERVED_DEGREES_TO_RADIANS;
        Self {
            a: x.cos() * f64::from(t.scale_x),
            b: y.cos() * f64::from(t.scale_y),
            c: x.sin() * f64::from(t.scale_x),
            d: y.sin() * f64::from(t.scale_y),
            x: f64::from(t.x),
            y: f64::from(t.y),
        }
    }
    fn then(self, q: Self) -> Self {
        Self {
            a: self.a * q.a + self.b * q.c,
            b: self.a * q.b + self.b * q.d,
            c: self.c * q.a + self.d * q.c,
            d: self.c * q.b + self.d * q.d,
            x: self.a * q.x + self.b * q.y + self.x,
            y: self.c * q.x + self.d * q.y + self.y,
        }
    }
    fn point(self, p: [f64; 2]) -> [f64; 2] {
        [
            self.a * p[0] + self.b * p[1] + self.x,
            self.c * p[0] + self.d * p[1] + self.y,
        ]
    }
    fn inverse_point(self, p: [f64; 2]) -> Option<[f64; 2]> {
        let det = self.determinant();
        if !det.is_finite() || det == 0. {
            return None;
        }
        let x = p[0] - self.x;
        let y = p[1] - self.y;
        let result = [
            (self.d * x - self.b * y) / det,
            (self.a * y - self.c * x) / det,
        ];
        result.into_iter().all(f64::is_finite).then_some(result)
    }
    fn determinant(self) -> f64 {
        self.a * self.d - self.b * self.c
    }
    fn components(self) -> [f64; 6] {
        [self.a, self.b, self.c, self.d, self.x, self.y]
    }
}

fn wide_pose(doc: &Document, locals: &[Transform], inherits: &[Inherit]) -> Vec<WideAffine> {
    let mut world: Vec<WideAffine> = Vec::with_capacity(locals.len());
    for (index, (bone, local)) in doc.bones.iter().zip(locals).enumerate() {
        let matrix = WideAffine::from_local(*local, f64::from(local.rotation));
        world.push(match bone.parent {
            None => matrix,
            Some(parent) if inherits[index] == Inherit::Normal => world[parent].then(matrix),
            Some(parent) => inherited_wide(world[parent], *local, inherits[index]),
        });
    }
    world
}

/// f64 reference frame for a non-normal child. Degenerate frames already failed
/// the checked f32 evaluation, so NaN here can only fail a precision check.
fn inherited_wide(parent: WideAffine, local: Transform, mode: Inherit) -> WideAffine {
    let [a, b, c, d] = inherit_linear(mode, [parent.a, parent.b, parent.c, parent.d], local)
        .unwrap_or([f64::NAN; 4]);
    let [x, y] = parent.point([f64::from(local.x), f64::from(local.y)]);
    WideAffine { a, b, c, d, x, y }
}

fn check_input_precision(
    native: &[Affine],
    wide: &[WideAffine],
    parent: usize,
    first: usize,
    target: usize,
) -> Result<(), String> {
    let mut pairs: Vec<_> = WideAffine::from(native[parent])
        .components()
        .into_iter()
        .zip(wide[parent].components())
        .collect();
    for index in [first, target] {
        pairs.extend(
            [f64::from(native[index].x), f64::from(native[index].y)]
                .into_iter()
                .zip([wide[index].x, wide[index].y]),
        );
    }
    if pairs.into_iter().any(|(a, b)| {
        !a.is_finite() || !b.is_finite() || (a - b).abs() > IK_WORLD_COORDINATE_TOLERANCE
    }) {
        return Err(
            "relevant parent or target exceeds coordinate precision before IK solving".into(),
        );
    }
    Ok(())
}

fn check_reconstruction(
    doc: &Document,
    locals: &[Transform],
    incoming: &[WideAffine],
    candidate: &[Affine],
    first: usize,
    solved: &[(usize, f64)],
    inherits: &[Inherit],
) -> Result<(), String> {
    let mut reference = vec![None; doc.bones.len()];
    for (index, bone) in doc.bones.iter().enumerate() {
        if !within(doc, index, first) {
            continue;
        }
        let rotation = solved
            .iter()
            .find(|&&(bone, _)| bone == index)
            .map_or(f64::from(locals[index].rotation), |&(_, r)| r);
        let local = WideAffine::from_local(locals[index], rotation);
        let parent = bone.parent.ok_or("Constrained root is not supported")?;
        let parent_world = reference[parent].unwrap_or(incoming[parent]);
        let expected = if inherits[index] == Inherit::Normal {
            parent_world.then(local)
        } else {
            let mut posed = locals[index];
            posed.rotation = rotation as f32;
            inherited_wide(parent_world, posed, inherits[index])
        };
        reference[index] = Some(expected);
        let actual = candidate[index];
        let mut pairs: Vec<_> = WideAffine::from(actual)
            .components()
            .into_iter()
            .zip(expected.components())
            .collect();
        pairs.extend(
            actual
                .point(bone.length, 0.)
                .map(f64::from)
                .into_iter()
                .zip(expected.point([f64::from(bone.length), 0.])),
        );
        if pairs.into_iter().any(|(a, b)| {
            !a.is_finite() || !b.is_finite() || (a - b).abs() > IK_WORLD_COORDINATE_TOLERANCE
        }) {
            return Err(format!(
                "bone {} exceeds coordinate precision: IK would move its frame or tip by more than 0.001",
                bone.name
            ));
        }
    }
    Ok(())
}
