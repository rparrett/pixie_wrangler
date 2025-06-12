use bevy::prelude::*;

#[derive(Debug)]
pub enum SegmentCollision {
    /// Two segments share some portion of their length (collinear and overlapping)
    Overlapping,
    /// Two segments meet at exactly one endpoint (forming a corner/junction)
    Connecting(Vec2),
    /// Two collinear segments meet at exactly one endpoint (extending in the same line)
    ConnectingParallel(Vec2),
    /// One segment's endpoint lies somewhere along the other segment (T-junction)
    Touching(Vec2),
    /// Two segments cross each other at an interior point (X-intersection)
    Intersecting,
    /// No collision between the segments
    None,
}

pub enum PointCollision {
    End,
    Middle,
    None,
}

// we should possibly also be using tolerances on the first two comparisons, but
// things generally seem to be working as-is.
pub fn point_segment_collision(p: Vec2, a: Vec2, b: Vec2) -> PointCollision {
    if p == a || p == b {
        return PointCollision::End;
    }

    let diff = b - a;
    let len2 = diff.length_squared();
    if len2 == 0.0 {
        return PointCollision::Middle;
    }

    let d1 = p - a;
    let d2 = b - a;

    let t = (d1.dot(d2) / len2).clamp(0.0, 1.0);
    let proj = a + t * diff;
    let dist = p.distance(proj);

    if dist <= 0.0001 {
        return PointCollision::Middle;
    }

    PointCollision::None
}

// for reference, this is helpful
// https://github.com/pgkelley4/line-segments-intersect/blob/master/js/line-segments-intersect.js
// but we're differing pretty wildly in how we choose to deal with collinearity, and
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

        if !(((dx.0 <= 0.0) && (dx.1 <= 0.0) && (dx.2 <= 0.0) && (dx.3 <= 0.0))
            || ((dx.0 >= 0.0) && (dx.1 >= 0.0) && (dx.2 >= 0.0) && (dx.3 >= 0.0)))
        {
            return SegmentCollision::Overlapping;
        }

        if !(((dy.0 <= 0.0) && (dy.1 <= 0.0) && (dy.2 <= 0.0) && (dy.3 <= 0.0))
            || ((dy.0 >= 0.0) && (dy.1 >= 0.0) && (dy.2 >= 0.0) && (dy.3 >= 0.0)))
        {
            return SegmentCollision::Overlapping;
        }

        // Check for end-to-end connection
        if (dx.0 == 0.0 && dy.0 == 0.0) || (dx.1 == 0.0 && dy.1 == 0.0) {
            return SegmentCollision::ConnectingParallel(a1);
        }
        if (dx.2 == 0.0 && dy.2 == 0.0) || (dx.3 == 0.0 && dy.3 == 0.0) {
            return SegmentCollision::ConnectingParallel(a2);
        }

        return SegmentCollision::None;
    }

    if denominator == 0.0 {
        // parallel, but we don't need to make that distinction
        return SegmentCollision::None;
    }

    let u = numerator / denominator;
    let t = dab.perp_dot(db) / denominator;
    let intersection = a1 + t * da;

    if (t == 1.0 || t == 0.0) && (u == 0.0 || u == 1.0) {
        return SegmentCollision::Connecting(intersection);
    }

    let col = (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u);

    if col && (t == 0.0 || u == 0.0 || t == 1.0 || u == 1.0) {
        return SegmentCollision::Touching(intersection);
    }

    if col {
        return SegmentCollision::Intersecting;
    }

    SegmentCollision::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_seg_connecting() {
        // start
        assert!(matches!(
            point_segment_collision(Vec2::ZERO, Vec2::ZERO, Vec2::new(1.0, 2.0),),
            PointCollision::End
        ));

        // end
        assert!(matches!(
            point_segment_collision(Vec2::new(1.0, 2.0), Vec2::ZERO, Vec2::new(1.0, 2.0),),
            PointCollision::End
        ));
    }

    #[test]
    fn point_seg_touching() {
        // -.-  (point in middle of horizontal segment)
        assert!(matches!(
            point_segment_collision(Vec2::ZERO, Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0),),
            PointCollision::Middle
        ));

        // Point in middle of vertical segment
        assert!(matches!(
            point_segment_collision(Vec2::ZERO, Vec2::new(0.0, -1.0), Vec2::new(0.0, 1.0),),
            PointCollision::Middle
        ));

        // Point in middle of diagonal segment
        assert!(matches!(
            point_segment_collision(Vec2::new(1.0, 1.0), Vec2::ZERO, Vec2::new(2.0, 2.0),),
            PointCollision::Middle
        ));
    }

    #[test]
    fn point_seg_none() {
        assert!(matches!(
            point_segment_collision(Vec2::new(1.0, 1.0), Vec2::ZERO, Vec2::new(1.0, 0.0)),
            PointCollision::None
        ));
    }

    #[test]
    fn point_seg_collinear() {
        // collinear horizontal
        assert!(matches!(
            point_segment_collision(Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(2.0, 0.0)),
            PointCollision::None
        ));

        // collinear vertical
        assert!(matches!(
            point_segment_collision(Vec2::ZERO, Vec2::new(0.0, 1.0), Vec2::new(0.0, 2.0)),
            PointCollision::None
        ));
    }

    #[test]
    fn seg_seg_collinear_none() {
        // collinear non-overlapping x axis
        assert!(matches!(
            segment_collision(
                Vec2::ZERO,
                Vec2::new(1.0, 0.0),
                Vec2::new(2.0, 0.0),
                Vec2::new(3.0, 0.0),
            ),
            SegmentCollision::None
        ));
        // collinear non-overlapping y axis
        assert!(matches!(
            segment_collision(
                Vec2::ZERO,
                Vec2::new(0.0, 1.0),
                Vec2::new(0.0, 2.0),
                Vec2::new(0.0, 3.0),
            ),
            SegmentCollision::None
        ));
    }

    #[test]
    fn seg_seg_intersecting() {
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
    }

    #[test]
    fn seg_seg_touching() {
        // One segment's endpoint touches the middle of another segment
        if let SegmentCollision::Touching(point) = segment_collision(
            Vec2::new(-1.0, 1.0),
            Vec2::new(1.0, -1.0),
            Vec2::new(1.0, 1.0),
            Vec2::ZERO,
        ) {
            assert_eq!(point, Vec2::ZERO);
        } else {
            panic!("Expected Touching collision");
        }

        // Horizontal T-junction: vertical segment touches middle of horizontal
        if let SegmentCollision::Touching(point) = segment_collision(
            Vec2::new(-2.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(0.0, -1.0),
            Vec2::ZERO,
        ) {
            assert_eq!(point, Vec2::ZERO);
        } else {
            panic!("Expected Touching collision");
        }

        // Vertical T-junction: horizontal segment touches middle of vertical
        if let SegmentCollision::Touching(point) = segment_collision(
            Vec2::new(0.0, -2.0),
            Vec2::new(0.0, 2.0),
            Vec2::new(-1.0, 0.0),
            Vec2::ZERO,
        ) {
            assert_eq!(point, Vec2::ZERO);
        } else {
            panic!("Expected Touching collision");
        }
    }

    #[test]
    fn seg_seg_connecting() {
        // V shape - two segments meeting at (0,0)
        if let SegmentCollision::Connecting(point) = segment_collision(
            Vec2::new(-2.0, 2.0),
            Vec2::ZERO,
            Vec2::ZERO,
            Vec2::new(2.0, 2.0),
        ) {
            assert_eq!(point, Vec2::ZERO);
        } else {
            panic!("Expected Connecting collision");
        }

        // Another V shape - segments meeting at (0,1.0)
        if let SegmentCollision::Connecting(point) = segment_collision(
            Vec2::new(2.0, 2.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(-2.0, 2.0),
            Vec2::new(0.0, 1.0),
        ) {
            assert_eq!(point, Vec2::new(0.0, 1.0));
        } else {
            panic!("Expected Connecting collision");
        }

        // L shape
        if let SegmentCollision::Connecting(point) = segment_collision(
            Vec2::ZERO,
            Vec2::new(2.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(2.0, 2.0),
        ) {
            assert_eq!(point, Vec2::new(2.0, 0.0));
        } else {
            panic!("Expected Connecting collision");
        }
    }

    #[test]
    fn seg_seg_connecting_parallel() {
        // Two collinear segments meeting end-to-end
        if let SegmentCollision::ConnectingParallel(point) = segment_collision(
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            Vec2::new(1.0, 0.0),
            Vec2::new(2.0, 0.0),
        ) {
            assert_eq!(point, Vec2::new(1.0, 0.0));
        } else {
            panic!("Expected ConnectingParallel collision");
        }

        // Vertical collinear segments
        if let SegmentCollision::ConnectingParallel(point) = segment_collision(
            Vec2::ZERO,
            Vec2::new(0.0, 1.0),
            Vec2::new(0.0, 1.0),
            Vec2::new(0.0, 2.0),
        ) {
            assert_eq!(point, Vec2::new(0.0, 1.0));
        } else {
            panic!("Expected ConnectingParallel collision");
        }

        // Diagonal collinear segments
        if let SegmentCollision::ConnectingParallel(point) = segment_collision(
            Vec2::ZERO,
            Vec2::new(1.0, 1.0),
            Vec2::new(-1.0, -1.0),
            Vec2::ZERO,
        ) {
            assert_eq!(point, Vec2::ZERO);
        } else {
            panic!("Expected ConnectingParallel collision");
        }
    }

    #[test]
    fn seg_seg_parallel() {
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
    }

    #[test]
    fn seg_seg_overlapping() {
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

    #[test]
    fn seg_seg_none() {
        // | -
        assert!(matches!(
            segment_collision(
                Vec2::new(-10.0, -10.0),
                Vec2::new(-10.0, 10.0),
                Vec2::ZERO,
                Vec2::new(10.0, 0.0),
            ),
            SegmentCollision::None
        ));
        // | /
        assert!(matches!(
            segment_collision(
                Vec2::new(-10.0, -10.0),
                Vec2::new(-10.0, 10.0),
                Vec2::new(0.0, -10.0),
                Vec2::new(10.0, 0.0),
            ),
            SegmentCollision::None
        ));
    }
}
