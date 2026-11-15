//! Independent cubic subdivision for inserting color keys. Control geometry is
//! split in f64 before rebasing onto the byte endpoints stored by native v22.
use peachy_document::Curve;

#[derive(Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
}
fn mix(a: Point, b: Point, t: f64) -> Point {
    Point {
        x: a.x + (b.x - a.x) * t,
        y: a.y + (b.y - a.y) * t,
    }
}
fn subdivide(points: [Point; 4], t: f64) -> ([Point; 4], [Point; 4]) {
    let a = mix(points[0], points[1], t);
    let b = mix(points[1], points[2], t);
    let c = mix(points[2], points[3], t);
    let d = mix(a, b, t);
    let e = mix(b, c, t);
    let middle = mix(d, e, t);
    ([points[0], a, d, middle], [middle, e, c, points[3]])
}
fn x_at(x1: f64, x2: f64, t: f64) -> f64 {
    let a = x1 * t;
    let b = x1 + (x2 - x1) * t;
    let c = x2 + (1. - x2) * t;
    let d = a + (b - a) * t;
    let e = b + (c - b) * t;
    d + (e - d) * t
}

/// Independent calibration from original numeric fixtures in installed 4.3.26:
/// first crossing of 300 forward-difference steps in absolute f32 time. Using
/// normalized time or exact cubic inversion changes the observed key handles.
/// See slot_color_insert_tests for the measured midpoint and held-out cases.
fn scanned_parameter(times: [f32; 3], x1: f32, x2: f32) -> Option<f64> {
    let [start, time, end] = times;
    let span = end - start;
    let (x1, x2) = (start + span * x1, start + span * x2);
    let a = (end - 3. * x2) + (3. * x1 - start);
    let b = (3. * x2 - 6. * x1) + 3. * start;
    let c = 3. * (x1 - start);
    let h = 1_f32 / 300.;
    let h2 = h * h;
    let h3 = h2 * h;
    let mut dx = (c * h + b * h2) + a * h3;
    let mut ddx = (2. * b) * h2 + (6. * a) * h3;
    let dddx = (6. * a) * h3;
    if ![x1, x2, dx, ddx, dddx].into_iter().all(f32::is_finite) {
        return None;
    }
    let mut x = start;
    for index in 1..=300 {
        x += dx;
        dx += ddx;
        ddx += dddx;
        if ![x, dx, ddx].into_iter().all(f32::is_finite) {
            return None;
        }
        if x >= time {
            return (index < 300).then(|| f64::from(index as f32 / 300.));
        }
    }
    None
}

/// First increasing-parameter intersection with this time. Derivative extrema
/// partition nonmonotone time curves, avoiding a monotonicity assumption.
fn parameter(x1: f64, x2: f64, time: f64) -> Result<f64, String> {
    let a = 3. * (1. + 3. * x1 - 3. * x2);
    let b = 6. * (x2 - 2. * x1);
    let c = 3. * x1;
    let mut stops = vec![0., 1.];
    if a == 0. {
        if b != 0. {
            let root = -c / b;
            if root > 0. && root < 1. {
                stops.push(root);
            }
        }
    } else {
        let discriminant = b * b - 4. * a * c;
        if discriminant >= 0. {
            let q = -0.5 * (b + discriminant.sqrt().copysign(b));
            let roots = if q == 0. {
                [-b / (2. * a); 2]
            } else {
                [q / a, c / q]
            };
            stops.extend(
                roots
                    .into_iter()
                    .filter(|root| root.is_finite() && *root > 0. && *root < 1.),
            );
        }
    }
    stops.sort_by(f64::total_cmp);
    stops.dedup();
    for interval in stops.windows(2) {
        let (mut low, mut high) = (interval[0], interval[1]);
        let (left, right) = (x_at(x1, x2, low), x_at(x1, x2, high));
        if time < left.min(right) || time > left.max(right) {
            continue;
        }
        if left == time {
            return Ok(low);
        }
        if right == time {
            return Ok(high);
        }
        let ascending = right >= left;
        // f32 times and handles can imply extremely small parameters. The
        // larger bound covers their entire exponent range without early
        // absolute-epsilon termination collapsing a near-endpoint key.
        for _ in 0..512 {
            let middle = low + (high - low) * 0.5;
            if middle == low || middle == high {
                break;
            }
            if (x_at(x1, x2, middle) < time) == ascending {
                low = middle;
            } else {
                high = middle;
            }
        }
        let result = if (x_at(x1, x2, low) - time).abs() <= (x_at(x1, x2, high) - time).abs() {
            low
        } else {
            high
        };
        if (x_at(x1, x2, result) - time).abs() > 1e-12 {
            return Err("Color curve time cannot be split at finite coordinate precision".into());
        }
        return Ok(result);
    }
    Err("Color curve has no representable split time".into())
}
fn finite(values: [f64; 4]) -> Option<[f32; 4]> {
    let converted = values.map(|value| value as f32);
    converted
        .iter()
        .all(|value| value.is_finite())
        .then_some(converted)
}
fn encode(points: [Point; 4], prefer_offset: bool) -> Result<Curve, String> {
    let duration = points[3].x - points[0].x;
    if !duration.is_finite() || duration <= 0. {
        return Err("Color curve split needs distinct ordered key times".into());
    }
    let x1 = (points[1].x - points[0].x) / duration;
    let x2 = (points[2].x - points[0].x) / duration;
    let h1 = points[1].y - points[0].y;
    let h2 = points[2].y - points[0].y;
    let span = points[3].y - points[0].y;
    if !prefer_offset
        && span != 0.
        && let Some(handles) = finite([x1, h1 / span, x2, h2 / span])
    {
        return Ok(Curve::Bezier(handles));
    }
    finite([x1, h1, x2, h2])
        .map(Curve::BezierOffset)
        .ok_or_else(|| "Color curve split exceeds finite handle coordinates".into())
}

pub(crate) fn split(
    curve: Curve,
    times: [f32; 3],
    values: [f32; 3],
    stored_new: f32,
) -> Result<(Curve, Curve), String> {
    let [start, time, end] = times.map(f64::from);
    let [left, new_value, right] = values.map(f64::from);
    if !(start < time && time < end)
        || ![left, new_value, right, f64::from(stored_new)]
            .iter()
            .all(|value| value.is_finite())
        || !curve.handles_valid()
    {
        return Err("Color curve split needs finite values and an interior exact time".into());
    }
    let (handles, offset) = match curve {
        Curve::Linear => return Ok((Curve::Linear, Curve::Linear)),
        Curve::Stepped => return Ok((Curve::Stepped, Curve::Stepped)),
        Curve::Bezier(handles) => (handles, false),
        Curve::BezierOffset(handles) => (handles, true),
    };
    let [x1, y1, x2, y2] = handles.map(f64::from);
    let fraction = (time - start) / (end - start);
    let points = [
        Point { x: 0., y: left },
        Point {
            x: x1,
            y: if offset {
                left + y1
            } else {
                left + (right - left) * y1
            },
        },
        Point {
            x: x2,
            y: if offset {
                left + y2
            } else {
                left + (right - left) * y2
            },
        },
        Point { x: 1., y: right },
    ];
    let u = match scanned_parameter(times, handles[0], handles[2]) {
        Some(u) => u,
        // Extreme/adjacent f32 times can overflow finite differences or make
        // every accumulated X identical. Stable subdivision remains available
        // there, outside the installed editor's calibrated numeric range.
        None => parameter(x1, x2, fraction)?,
    };
    let (mut before, mut after) = subdivide(points, u);
    let delta = new_value - before[3].y;
    before[2].y += delta;
    after[1].y += delta;
    // Keep precise split controls, but encode relative to actual stored endpoint
    // bytes; otherwise export would shift controls when the new key quantizes.
    before[3] = Point {
        x: fraction,
        y: f64::from(stored_new),
    };
    after[0] = before[3];
    Ok((encode(before, offset)?, encode(after, offset)?))
}
