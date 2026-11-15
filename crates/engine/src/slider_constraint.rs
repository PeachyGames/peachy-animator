//! Manual Slider bone-channel application from original black-box observations.
//! The public 4.3.13 oracle supplied bone-time-01, domain-02 and 13 independent
//! zero/full-Mix holdouts. No runtime implementation was inspected.
//! Bone-driven source values are read from the current ordered state, while
//! calibrated referenced domains join that same state without another solve.
use crate::{Affine, ik::AppliedState, world_pose_with};
use peachy_document::{Document, Property, SliderConstraint, Track, Transform, TransformProperty};

// Independent source-angle observations select 180 / 3.1415927. This differs
// by one f64 ULP from inverting the separately observed FK degree coefficient.
const OBSERVED_RADIANS_TO_DEGREES: f64 = f64::from_bits(0x404c_a5dc_134a_154a);

fn source_value(slider: &SliderConstraint, local: Transform, frame: Affine) -> Result<f64, String> {
    let property = slider.property.ok_or("a source bone needs a property")?;
    let value = if slider.local {
        f64::from(match property {
            TransformProperty::Rotate => local.rotation,
            TransformProperty::X => local.x,
            TransformProperty::Y => local.y,
            TransformProperty::ScaleX => local.scale_x,
            TransformProperty::ScaleY => local.scale_y,
            TransformProperty::ShearY => local.shear_y,
        })
    } else {
        let [a, b, c, d, x, y] =
            [frame.a, frame.b, frame.c, frame.d, frame.x, frame.y].map(f64::from);
        let degrees = OBSERVED_RADIANS_TO_DEGREES;
        match property {
            TransformProperty::Rotate => {
                let angle = c.atan2(a) * degrees;
                if angle < 0. { angle + 360. } else { angle }
            }
            TransformProperty::X => x,
            TransformProperty::Y => y,
            TransformProperty::ScaleX => a.hypot(c),
            TransformProperty::ScaleY => b.hypot(d),
            // Axis angle separation has no wrapping or zero-axis fallback.
            // In particular, atan2's signed-zero branches remain observable.
            TransformProperty::ShearY => (d.atan2(b) - c.atan2(a)) * degrees - 90.,
        }
    };
    if !value.is_finite() {
        return Err("source property is nonfinite".into());
    }
    Ok(value)
}

fn derived_time(slider: &SliderConstraint, source: f64, duration: f64) -> Result<f64, String> {
    let mapped = (source - f64::from(slider.from)) * f64::from(slider.scale) + f64::from(slider.to);
    if !mapped.is_finite() {
        return Err("source mapping produced a nonfinite time".into());
    }
    if slider.looped {
        if duration == 0. {
            // The public oracle produces NaN here. Refuse that measured
            // hazard instead of publishing nonfinite state or JSON null.
            return Err("bone-driven Loop requires a nonempty referenced animation".into());
        }
        // Native fields are f32; JSON consumers parse their shortest decimal
        // spelling into f64. Original .01-scale probes at source 25/125/225
        // choose different Loop turns across those representations. Diagnose
        // that discontinuity without snapping time or changing published math.
        fn decimal(value: f32) -> f64 {
            value.to_string().parse().expect("finite validated number")
        }
        let exported_source = if slider.local {
            decimal(source as f32)
        } else {
            source
        };
        let exported_mapped =
            (exported_source - decimal(slider.from)) * decimal(slider.scale) + decimal(slider.to);
        let exported_duration = decimal(duration as f32);
        if (mapped / duration).floor() != (exported_mapped / exported_duration).floor() {
            return Err(
                "Loop native and JSON-consumer branches differ at a coordinate precision boundary; this pose is unsupported".into(),
            );
        }
        Ok(mapped % duration + duration)
    } else {
        // Max is retained editor range metadata, not an evaluation upper cap.
        Ok(mapped.max(0.))
    }
}

fn sample_offset(track: &Track, time: f64) -> Option<f64> {
    // Missing and before-first channels preserve incoming state. In particular,
    // a prior Slider may produce a time between adjacent stored f32 keys.
    crate::slider_sampling::scalar(&track.keys, time)
}

fn signed_unit(value: f64) -> f64 {
    if value == 0. {
        // f64::signum would turn +/-zero into +/-one. The observed zero target
        // instead has zero sign magnitude, including the sign bit of -zero.
        value
    } else if value < 0. {
        -1.
    } else {
        1.
    }
}

fn blend(
    property: Property,
    current: f32,
    setup: f32,
    sample: f64,
    mix: f64,
    additive: bool,
) -> Result<f32, String> {
    if mix == 0. {
        return Ok(current);
    }
    let (current, setup) = (f64::from(current), f64::from(setup));
    let value = if property.is_scale() {
        if additive {
            current + (sample - 1.) * setup * mix
        } else {
            let target = setup * sample;
            if mix == 1. {
                // The independent large-current holdout proves scale's direct
                // full-Mix assignment; ordinary channels below keep their lerp.
                target
            } else {
                let base = current.abs() * signed_unit(target);
                // Preserve association: factoring the sign outside changes
                // target-zero signs at negative/overshoot Mix.
                base + (target - base) * mix
            }
        }
    } else if additive {
        current + sample * mix
    } else {
        // Rotation preserves authored winding; no +/-180 correction occurs.
        current + (setup + sample - current) * mix
    };
    let stored = value as f32;
    if !value.is_finite() || !stored.is_finite() {
        return Err("applied value is outside the finite f32 range".into());
    }
    Ok(stored)
}

pub(crate) fn apply(
    doc: &Document,
    slider: &SliderConstraint,
    index: usize,
    state: &mut AppliedState,
    world: &mut Vec<Affine>,
) -> Result<(), String> {
    if slider.bone.is_some_and(|bone| !state.active.bones[bone]) {
        return Ok(());
    }
    let reference = doc
        .animations
        .get(slider.animation)
        .ok_or("unknown referenced animation")?;
    crate::slider_domains::validate_reference(doc, reference)?;
    // Capture the current effective settings once at this ordered turn. A
    // referenced animation is never recursively evaluated as another skeleton.
    let mut pose = *state
        .sliders
        .get(index)
        .ok_or("missing effective Slider pose")?;
    if !pose.time.is_finite() || !pose.mix.is_finite() {
        return Err("effective settings must be finite".into());
    }
    if pose.mix == 0. {
        return Ok(());
    }
    let duration = f64::from(reference.duration());
    if let Some(bone) = slider.bone {
        let source = source_value(slider, state.locals[bone], world[bone])?;
        pose.time = derived_time(slider, source, duration)?;
        state.sliders[index].time = pose.time;
    }
    // Bone source effective time includes its measured pre-wrapping above.
    // Sampling/display performs the final modulo; manual effective time stays
    // raw. Neither path rewrites authored Time or infers source-clear behavior.
    let time = if slider.looped && duration > 0. {
        pose.time % duration
    } else {
        pose.time
    };
    let mut updates = vec![];
    for track in &reference.tracks {
        let Some(sample) = sample_offset(track, time) else {
            continue;
        };
        if !state.active.bones[track.bone] {
            // Referenced inactive targets skip their writes; other active
            // targets and global domains still apply at this same turn.
            continue;
        }
        let current = track.property.read(state.locals[track.bone]);
        let setup = track.property.read(doc.bones[track.bone].setup);
        let value = blend(
            track.property,
            current,
            setup,
            sample,
            pose.mix,
            slider.additive,
        )
        .map_err(|error| {
            format!(
                "bone {} {:?}: {error}",
                doc.bones[track.bone].name, track.property
            )
        })?;
        if value.to_bits() != current.to_bits() {
            updates.push((track.bone, track.property, value));
        }
    }
    // Stage bone changes until the other domains have validated their writes.
    // They read this turn's incoming state and never recursively run a solver.
    let changed_inherits =
        crate::slider_domains::apply(doc, reference, time, pose.mix, slider.additive, state)?;
    if updates.is_empty() && changed_inherits.is_empty() {
        return Ok(());
    }
    for (bone, property, value) in updates {
        property.write(&mut state.locals[bone], value);
    }
    let updated = world_pose_with(doc, &state.locals, &state.inherits);
    crate::ik::check_inherit_domain(doc, &updated, &state.locals, &state.inherits)?;
    crate::ik::check_world(doc, &updated)?;
    *world = updated;
    Ok(())
}

#[cfg(test)]
#[path = "slider_constraint_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "slider_source_tests.rs"]
mod source_tests;
