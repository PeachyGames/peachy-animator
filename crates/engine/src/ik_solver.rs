//! Bounded normal IK from elementary link geometry and original public pose
//! observations. No external implementation or runtime constant was inspected.
use crate::{Affine, ik::check_world, pose::OBSERVED_DEGREES_TO_RADIANS, world_pose};
use peachy_document::{Constraint, Document, IkConstraint, Transform};

/// Applied f32 frames and bone endpoints must retain their solved f64 reference
/// within this absolute bound. There is no relative or large-coordinate exception.
const IK_WORLD_COORDINATE_TOLERANCE: f64 = 0.001;

pub(crate) fn apply(
    doc: &Document,
    locals: &mut [Transform],
    world: &mut Vec<Affine>,
) -> Result<(), String> {
    let active: Vec<_> = doc
        .constraints
        .iter()
        .filter_map(|constraint| {
            let Constraint::Ik(ik) = constraint;
            (ik.setup.mix > 0.0).then_some(ik)
        })
        .collect();
    for (index, ik) in active.iter().enumerate() {
        validate_domain(doc, locals, ik)?;
        for other in &active[..index] {
            let overlaps = (0..doc.bones.len())
                .any(|bone| within(doc, bone, ik.bones[0]) && within(doc, bone, other.bones[0]));
            if overlaps
                || within(doc, ik.target, other.bones[0])
                || within(doc, other.target, ik.bones[0])
            {
                return Err(format!(
                    "IK {} and {}: dependent constraint subtrees or targets are not supported",
                    other.name, ik.name
                ));
            }
        }
    }
    for ik in active {
        solve(doc, locals, world, ik).map_err(|error| format!("IK {}: {error}", ik.name))?;
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
    for &bone in &ik.bones {
        let mut current = Some(bone);
        while let Some(index) = current {
            let local = locals[index];
            if local.scale_x != 1.
                || local.scale_y != 1.
                || local.shear_x != 0.
                || local.shear_y != 0.
            {
                return Err(format!(
                    "IK {}: bone {} requires unit positive scales and zero shear",
                    ik.name, doc.bones[index].name
                ));
            }
            current = doc.bones[index].parent;
        }
    }
    if ik.bones.len() == 2 {
        let child = ik.bones[1];
        if locals[child].x <= 0.
            || locals[child].x != doc.bones[child].setup.x
            || locals[child].y != 0.
            || doc.bones[child].length <= 0.
        {
            return Err(format!(
                "IK {}: child requires a constant positive setup X, zero Y and positive length",
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
) -> Result<(), String> {
    let first = ik.bones[0];
    let parent_index = doc.bones[first]
        .parent
        .ok_or("Constrained root is not supported")?;
    let incoming = wide_pose(doc, locals);
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
    let mut desired = vec![(first, dy.atan2(dx) / OBSERVED_DEGREES_TO_RADIANS)];
    if ik.bones.len() == 2 {
        let child = ik.bones[1];
        let a = f64::from(locals[child].x);
        let b = f64::from(doc.bones[child].length);
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
        let child_angle = cosine.acos() * if ik.setup.bend_positive { 1. } else { -1. };
        let parent_angle = dy.atan2(dx) - (b * child_angle.sin()).atan2(a + b * child_angle.cos());
        desired[0].1 = parent_angle / OBSERVED_DEGREES_TO_RADIANS;
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
        let rotation = authored + delta * f64::from(ik.setup.mix);
        if !rotation.is_finite() || !(rotation as f32).is_finite() {
            return Err("applied rotation is outside the finite coordinate range".into());
        }
        solved.push((bone, rotation));
    }
    if solved
        .iter()
        .all(|&(bone, rotation)| rotation == f64::from(locals[bone].rotation))
    {
        return Ok(());
    }
    for &(bone, rotation) in &solved {
        locals[bone].rotation = rotation as f32;
    }
    let candidate = world_pose(doc, locals);
    check_world(doc, &candidate)?;
    check_reconstruction(doc, locals, &incoming, &candidate, first, &solved)?;
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

fn wide_pose(doc: &Document, locals: &[Transform]) -> Vec<WideAffine> {
    let mut world: Vec<WideAffine> = Vec::with_capacity(locals.len());
    for (bone, local) in doc.bones.iter().zip(locals) {
        let matrix = WideAffine::from_local(*local, f64::from(local.rotation));
        world.push(
            bone.parent
                .map_or(matrix, |parent| world[parent].then(matrix)),
        );
    }
    world
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
        let expected = parent_world.then(local);
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
