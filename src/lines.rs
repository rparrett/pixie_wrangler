use bevy::prelude::*;

use crate::{
    collision::{point_segment_collision, SegmentCollision},
    RoadSegment,
};

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

pub fn distance_on_path(start: Vec2, point: Vec2, segments: &[(Vec2, Vec2)]) -> Option<f32> {
    let mut total_dist = 0.0;
    let mut starting_point = start;

    for segment in segments.iter() {
        match point_segment_collision(point, segment.0, segment.1) {
            SegmentCollision::None => {
                total_dist += starting_point.distance(segment.1);
                starting_point = segment.0;
            }
            _ => {
                return Some(total_dist + starting_point.distance(point));
            }
        }
    }

    None
}

/// * `start` The starting point, which should be on the first segment
pub fn traveled_segments(
    start: Vec2,
    distance: f32,
    segments: &[RoadSegment],
) -> Vec<(Vec2, Vec2)> {
    let mut to_go = distance;
    let mut i = 0;
    let mut path = vec![];
    let mut current = start;

    while i < segments.len() {
        let next = segments[i].points.1;
        let to_next = current.distance(next);

        if to_next < to_go {
            path.push((current, next));
            current = next;
            to_go -= to_next;
            i += 1;
        } else {
            let diff = next - current;

            let theta = diff.y.atan2(diff.x);

            let projected = current + Vec2::new(to_go * theta.cos(), to_go * theta.sin());
            path.push((current, projected));

            return path;
        }
    }

    path
}

/// * `start` The starting point, which should be on the first segment
pub fn travel(start: Vec2, distance: f32, segments: &[RoadSegment]) -> (Vec2, usize) {
    let mut to_go = distance;
    let mut i = 0;
    let mut current = start;

    while i < segments.len() {
        let prev = segments[i].points.0;
        let next = segments[i].points.1;

        let to_next = current.distance(next);

        if to_next < to_go {
            current = next;
            to_go -= to_next;
            i += 1;
        } else {
            let segment_diff = next - prev;
            let segment_length = prev.distance(next);

            let projected = current + to_go / segment_length * segment_diff;

            return (projected, i);
        }
    }

    (segments.last().unwrap().points.1, i)
}
