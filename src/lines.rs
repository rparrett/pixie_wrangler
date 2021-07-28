use bevy::prelude::*;

#[derive(Clone, Copy, Debug)]
pub enum Axis {
    X,
    Y,
}

/// Given three points forming two line segments, return the angle formed
/// at the middle. Returns values in the range of 0.0..=180.0
///
/// ```text
/// a - b
///      \
///       c
/// ```
pub fn corner_angle(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    let seg_a = a - b;
    let seg_b = c - b;

    seg_a.perp_dot(seg_b).abs().atan2(seg_a.dot(seg_b))
}

/// Given a start and endpoint, return up to two points that represent the
/// middle of possible 45-degree-only two segment polylines that connect them.
/// ```text
///   i
///  /|
/// o o
/// |/
/// i
/// ```
/// In the case where a straight line path is possible, returns that single
/// straight line.
///
/// * `axis_preference` - If this is Some(Axis), we will offer up the line that
///   "moves in the preferred axis first" as the first result.
pub fn possible_lines(
    from: Vec2,
    to: Vec2,
    axis_preference: Option<Axis>,
) -> Vec<Vec<(Vec2, Vec2)>> {
    let diff = to - from;

    // if a single 45 degree or 90 degree line does the job,
    // return that.
    if diff.x == 0.0 || diff.y == 0.0 || diff.x.abs() == diff.y.abs() {
        return vec![vec![(from, to)]];
    }

    let (a, b) = if diff.x.abs() < diff.y.abs() {
        (
            Vec2::new(from.x, to.y - diff.x.abs() * diff.y.signum()),
            Vec2::new(to.x, from.y + diff.x.abs() * diff.y.signum()),
        )
    } else {
        (
            Vec2::new(to.x - diff.y.abs() * diff.x.signum(), from.y),
            Vec2::new(from.x + diff.y.abs() * diff.x.signum(), to.y),
        )
    };

    if matches!(axis_preference, Some(Axis::X)) && a.y == from.y
        || matches!(axis_preference, Some(Axis::Y)) && a.x == from.x
    {
        return vec![vec![(from, a), (a, to)], vec![(from, b), (b, to)]];
    }

    return vec![vec![(from, b), (b, to)], vec![(from, a), (a, to)]];
}
