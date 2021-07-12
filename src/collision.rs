use bevy::prelude::*;

#[derive(Debug)]
pub enum SegmentCollision {
    Overlapping,
    Connecting,
    Touching,
    Intersecting,
    None,
}

pub fn point_segment_collision(p: Vec2, l1: Vec2, l2: Vec2) -> SegmentCollision {
    if p == l1 || p == l2 {
        return SegmentCollision::Connecting;
    }

    let d1 = l2 - l1;
    let d2 = p - l1;

    let cross = d1.perp_dot(d2);

    if cross.abs() > std::f32::EPSILON {
        return SegmentCollision::None;
    }

    let dot = d1.dot(d2);

    if dot < 0.0 {
        return SegmentCollision::None;
    }

    if dot > l1.distance_squared(l2) {
        return SegmentCollision::None;
    }

    SegmentCollision::Touching
}

// for reference, this is helpful
// https://github.com/pgkelley4/line-segments-intersect/blob/master/js/line-segments-intersect.js
// but we're differing pretty wildly in how we choose to deal with collinearities, and
// we threw epsilon out of the window because we're snapping to an integer grid
pub fn segment_collision(a1: Vec2, a2: Vec2, b1: Vec2, b2: Vec2) -> SegmentCollision {
    let da = a2 - a1;
    let db = b2 - b1;
    let dab = b1 - a1;

    let numerator = dab.perp_dot(da);
    let denominator = da.perp_dot(db);

    if numerator == 0.0 && denominator == 0.0 {
        // these are collinear
        // but are they overlapping? merely touching end to end?
        // or not touching at all?

        let dx = (a1.x - b1.x, a1.x - b2.x, a2.x - b1.x, a2.x - b2.x);
        let dy = (a1.y - b1.y, a1.y - b2.y, a2.y - b1.y, a2.y - b2.y);

        if !(((dx.0 == 0.0 || dx.0 < 0.0)
            && (dx.1 == 0.0 || dx.1 < 0.0)
            && (dx.2 == 0.0 || dx.2 < 0.0)
            && (dx.3 == 0.0 || dx.3 < 0.0))
            || ((dx.0 == 0.0 || dx.0 > 0.0)
                && (dx.1 == 0.0 || dx.1 > 0.0)
                && (dx.2 == 0.0 || dx.2 > 0.0)
                && (dx.3 == 0.0 || dx.3 > 0.0)))
        {
            return SegmentCollision::Overlapping;
        }

        if !(((dy.0 == 0.0 || dy.0 < 0.0)
            && (dy.1 == 0.0 || dy.1 < 0.0)
            && (dy.2 == 0.0 || dy.2 < 0.0)
            && (dy.3 == 0.0 || dy.3 < 0.0))
            || ((dy.0 == 0.0 || dy.0 > 0.0)
                && (dy.1 == 0.0 || dy.1 > 0.0)
                && (dy.2 == 0.0 || dy.2 > 0.0)
                && (dy.3 == 0.0 || dy.3 > 0.0)))
        {
            return SegmentCollision::Overlapping;
        }

        if dx.0 == 0.0 && dy.0 == 0.0
            || dx.1 == 0.0 && dy.1 == 0.0
            || dx.2 == 0.0 && dy.2 == 0.0
            || dx.3 == 0.0 && dy.3 == 0.0
        {
            return SegmentCollision::Connecting;
        }

        return SegmentCollision::None;
    }

    if denominator == 0.0 {
        // parallel, but we don't need to make that distinction
        return SegmentCollision::None;
    }

    let u = numerator / denominator;
    let t = dab.perp_dot(db) / denominator;

    if (t == 0.0 && u == 1.0)
        || (u == 0.0 && t == 1.0)
        || (u == 0.0 && t == 0.0)
        || (u == 1.0 && t == 1.0)
    {
        return SegmentCollision::Connecting;
    }

    let col = t >= 0.0 && t <= 1.0 && u >= 0.0 && u <= 1.0;

    if col && (t == 0.0 || u == 0.0 || t == 1.0 || u == 1.0) {
        return SegmentCollision::Touching;
    }

    if col {
        return SegmentCollision::Intersecting;
    }

    return SegmentCollision::None;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_point_segment_collision() {
        // -.-
        assert!(matches!(
            point_segment_collision(
                Vec2::new(0.0, 0.0),
                Vec2::new(-1.0, 0.0),
                Vec2::new(1.0, 0.0)
            ),
            SegmentCollision::Touching
        ));
        assert!(matches!(
            point_segment_collision(
                Vec2::new(0.0, 0.0),
                Vec2::new(0.0, -1.0),
                Vec2::new(0.0, 1.0)
            ),
            SegmentCollision::Touching
        ));
        // .--
        assert!(matches!(
            point_segment_collision(
                Vec2::new(0.0, 0.0),
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 2.0)
            ),
            SegmentCollision::Connecting
        ));
        // --.
        assert!(matches!(
            point_segment_collision(
                Vec2::new(1.0, 2.0),
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 2.0)
            ),
            SegmentCollision::Connecting
        ));
        assert!(matches!(
            point_segment_collision(
                Vec2::new(1.0, 1.0),
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0)
            ),
            SegmentCollision::None
        ));
    }

    #[test]
    fn test_segment_collision() {
        // collinear non-overlapping x axis
        assert!(matches!(
            segment_collision(
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(2.0, 0.0),
                Vec2::new(3.0, 0.0),
            ),
            SegmentCollision::None
        ));
        // collinear non-overlapping y axis
        assert!(matches!(
            segment_collision(
                Vec2::new(0.0, 0.0),
                Vec2::new(0.0, 1.0),
                Vec2::new(0.0, 2.0),
                Vec2::new(0.0, 3.0),
            ),
            SegmentCollision::None
        ));
        // x
        assert!(matches!(
            segment_collision(
                Vec2::new(-1.0, 1.0),
                Vec2::new(1.0, -1.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(-1.0, -1.0),
            ),
            SegmentCollision::Intersecting
        ));
        // 3-limbed x
        assert!(matches!(
            segment_collision(
                Vec2::new(-1.0, 1.0),
                Vec2::new(1.0, -1.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(0.0, 0.0),
            ),
            SegmentCollision::Touching
        ));
        // V
        assert!(matches!(
            segment_collision(
                Vec2::new(-2.0, 2.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(2.0, 2.0),
            ),
            SegmentCollision::Connecting
        ));
        assert!(matches!(
            segment_collision(
                Vec2::new(2.0, 2.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(-2.0, 2.0),
                Vec2::new(1.0, 1.0),
            ),
            SegmentCollision::Connecting
        ));
        // =
        assert!(matches!(
            segment_collision(
                Vec2::new(-2.0, -2.0),
                Vec2::new(2.0, -2.0),
                Vec2::new(-2.0, 2.0),
                Vec2::new(2.0, 2.0),
            ),
            SegmentCollision::None
        ));
        // -=-
        assert!(matches!(
            segment_collision(
                Vec2::new(10.0, 10.0),
                Vec2::new(20.0, 10.0),
                Vec2::new(13.0, 10.0),
                Vec2::new(17.0, 10.0),
            ),
            SegmentCollision::Overlapping
        ));
    }
}
