use bevy::{
    input::mouse::MouseButtonInput, input::ElementState::Released, prelude::*, window::CursorMoved,
};
use bevy_prototype_lyon::prelude::*;
use itertools::Itertools;

const GRID_SIZE: f32 = 16.0;
const HALF_GRID_SIZE: f32 = 8.0;

fn main() {
    let mut app = App::build();
    app.insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins);
    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);
    app.add_plugin(ShapePlugin);
    app.add_startup_system(setup.system());
    app.add_system(mouse_events_system.system().label("mouse"));
    app.add_system(draw_current_shape.system().after("mouse")); // after mouse
    app.add_system(draw_current_line.system().after("mouse")); // after mouse
    app.init_resource::<PathDrawingState>();
    app.init_resource::<MouseState>();
    app.run();
}

struct MainCamera;

struct CurrentShape;
struct CurrentLine;
struct Road {
    path: Vec<Vec2>,
}
struct Terminus {
    point: Vec2,
}

#[derive(Default, Debug)]
struct PathDrawingState {
    drawing: bool,
    points: Vec<Vec2>,
}

#[derive(Default, Debug)]
struct MouseState {
    position: Vec2,
}

#[derive(Debug)]
enum SegmentCollision {
    Overlapping,
    Connecting,
    Touching,
    Intersecting,
    None,
}

fn point_segment_collision(p: Vec2, l1: Vec2, l2: Vec2) -> SegmentCollision {
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
fn segment_collision(a1: Vec2, a2: Vec2, b1: Vec2, b2: Vec2) -> SegmentCollision {
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

    if (t == 0.0 && u == 1.0) || (u == 0.0 && t == 1.0) {
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

fn snap_to_grid_1d(position: f32, grid_size: f32) -> f32 {
    let new = (position / grid_size).round() * grid_size;

    new
}

fn snap_to_grid(position: Vec2, grid_size: f32) -> Vec2 {
    let new = (position / grid_size).round() * grid_size;

    new
}

fn snap_to_angle(start: Vec2, end: Vec2, divisions: u32, grid_size: f32) -> Vec2 {
    let diff = end - start;

    let angle = diff.y.atan2(diff.x);

    let increment = std::f32::consts::TAU / divisions as f32;

    let snap_angle = (angle / increment).round() * increment;

    if snap_angle.to_degrees() == 90.0 || snap_angle.to_degrees() == -90.0 {
        return snap_to_grid(start + Vec2::new(0.0, diff.y), grid_size);
    }

    if snap_angle.to_degrees() == 0.0
        || snap_angle.to_degrees() == 180.0
        || snap_angle.to_degrees() == -180.0
    {
        return snap_to_grid(start + Vec2::new(diff.x, 0.0), grid_size);
    }

    let snapped_x = snap_to_grid_1d(diff.x, grid_size);
    let snapped_y = snap_to_grid_1d(diff.y, grid_size);

    if (end.x - snapped_x).abs() > (end.y - snapped_y).abs() {
        return start + Vec2::new(snapped_x, snapped_x * snap_angle.tan());
    } else {
        return start + Vec2::new(snapped_y / snap_angle.tan(), snapped_y);
    }
}

fn draw_current_shape(
    mut commands: Commands,
    path: Res<PathDrawingState>,
    query_shape: Query<Entity, With<CurrentShape>>,
) {
    if !path.is_changed() {
        return;
    }

    for ent in query_shape.iter() {
        commands.entity(ent).despawn();
    }

    let shape = shapes::Polygon {
        points: path.points.clone(),
        closed: false,
    };
    commands
        .spawn_bundle(GeometryBuilder::build_as(
            &shape,
            ShapeColors::outlined(Color::NONE, Color::BLACK),
            DrawMode::Outlined {
                fill_options: FillOptions::default(),
                outline_options: StrokeOptions::default().with_line_width(2.0),
            },
            Transform::default(),
        ))
        .insert(CurrentShape);
}

fn draw_current_line(
    mut commands: Commands,
    path: Res<PathDrawingState>,
    mouse: Res<MouseState>,
    query_line: Query<Entity, With<CurrentLine>>,
    q_roads: Query<&Road>,
) {
    if !mouse.is_changed() {
        return;
    }

    for ent in query_line.iter() {
        commands.entity(ent).despawn();
    }

    if !path.drawing {
        return;
    }

    if let Some(point) = path.points.last() {
        let snapped = snap_to_angle(point.clone(), mouse.position, 8, GRID_SIZE);
        if snapped == *point {
            return;
        }

        // need to be able to snap to a half grid to properly connect to the middle of
        // some diagonal lines.

        let snapped_half = snap_to_angle(point.clone(), mouse.position, 8, HALF_GRID_SIZE);

        let mut invalid = false;
        let mut invalid_half = false;
        let mut touching = false;
        let mut touching_half = false;
        let mut connecting = 0;

        for (a, b) in path.points.iter().tuple_windows() {
            match segment_collision(*a, *b, *point, snapped) {
                SegmentCollision::Intersecting | SegmentCollision::Overlapping => {
                    invalid = true;
                }
                SegmentCollision::Touching => {
                    touching = true;
                }
                SegmentCollision::Connecting => {
                    connecting += 1;
                }
                _ => {}
            };
            match segment_collision(*a, *b, *point, snapped_half) {
                SegmentCollision::Intersecting | SegmentCollision::Overlapping => {
                    invalid_half = true;
                }
                SegmentCollision::Touching => {
                    touching_half = true;
                }
                _ => {}
            };
        }
        for r in q_roads.iter() {
            for (a, b) in r.path.iter().tuple_windows() {
                match segment_collision(*a, *b, *point, snapped) {
                    SegmentCollision::Intersecting | SegmentCollision::Overlapping => {
                        invalid = true;
                    }
                    SegmentCollision::Touching => {
                        touching = true;
                    }
                    SegmentCollision::Connecting => {
                        connecting += 1;
                    }
                    _ => {}
                };
                match segment_collision(*a, *b, *point, snapped_half) {
                    SegmentCollision::Intersecting | SegmentCollision::Overlapping => {
                        invalid_half = true;
                    }
                    SegmentCollision::Touching => {
                        touching_half = true;
                    }
                    _ => {}
                };
            }
        }

        let color = if invalid && invalid_half {
            Color::RED
        } else if touching || touching_half || connecting > 1 {
            Color::BISQUE
        } else {
            Color::BLUE
        };

        let snapped_point = if invalid && invalid_half {
            snapped
        } else if touching_half {
            snapped_half
        } else {
            snapped
        };

        let points = vec![point.clone(), snapped_point];

        let shape = shapes::Polygon {
            points,
            closed: false,
        };
        commands
            .spawn_bundle(GeometryBuilder::build_as(
                &shape,
                ShapeColors::outlined(Color::NONE, color),
                DrawMode::Outlined {
                    fill_options: FillOptions::default(),
                    outline_options: StrokeOptions::default().with_line_width(2.0),
                },
                Transform::default(),
            ))
            .insert(CurrentLine);
    }
}

/// This system prints out all mouse events as they come in
fn mouse_events_system(
    mut commands: Commands,
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut path: ResMut<PathDrawingState>,
    mut mouse: ResMut<MouseState>,
    wnds: Res<Windows>,
    q_camera: Query<&Transform, With<MainCamera>>,
    q_terminuses: Query<&Terminus>,
    q_roads: Query<&Road>,
) {
    // assuming there is exactly one main camera entity, so this is OK
    let camera_transform = q_camera.iter().next().unwrap();

    for event in cursor_moved_events.iter() {
        let wnd = wnds.get(event.id).unwrap();
        let size = Vec2::new(wnd.width() as f32, wnd.height() as f32);

        let p = event.position - size / 2.0;

        mouse.position = (camera_transform.compute_matrix() * p.extend(0.0).extend(1.0))
            .truncate()
            .truncate();
    }

    for event in mouse_button_input_events.iter() {
        if event.button == MouseButton::Left && event.state == Released {
            if let Some(last) = path.points.last() {
                if !path.drawing {
                    return;
                }

                info!("continuing path");
                let snapped = snap_to_angle(*last, mouse.position, 8, GRID_SIZE);
                let snapped_half = snap_to_angle(*last, mouse.position, 8, HALF_GRID_SIZE);

                let mut invalid = false;
                let mut invalid_half = false;
                let mut touching = false;
                let mut touching_half = false;
                let mut connecting = 0;

                // TODO when considering "touching" "connecting" collisions, it would probably
                // make more sense to do a separate point collision check just for the point
                // under the cursor

                for (a, b) in path.points.iter().tuple_windows() {
                    match segment_collision(*a, *b, *last, snapped) {
                        SegmentCollision::Intersecting | SegmentCollision::Overlapping => {
                            invalid = true;
                        }
                        SegmentCollision::Touching => {
                            touching = true;
                        }
                        SegmentCollision::Connecting => {
                            connecting += 1;
                        }
                        _ => {}
                    }
                    match segment_collision(*a, *b, *last, snapped_half) {
                        SegmentCollision::Intersecting | SegmentCollision::Overlapping => {
                            invalid_half = true;
                        }
                        SegmentCollision::Touching => {
                            touching_half = true;
                        }
                        _ => {}
                    }
                }
                for r in q_roads.iter() {
                    for (a, b) in r.path.iter().tuple_windows() {
                        match segment_collision(*a, *b, *last, snapped) {
                            SegmentCollision::Intersecting | SegmentCollision::Overlapping => {
                                invalid = true;
                            }
                            SegmentCollision::Touching => {
                                touching = true;
                            }
                            SegmentCollision::Connecting => {
                                connecting += 1;
                            }
                            _ => {}
                        }
                        match segment_collision(*a, *b, *last, snapped_half) {
                            SegmentCollision::Intersecting | SegmentCollision::Overlapping => {
                                invalid_half = true;
                            }
                            SegmentCollision::Touching => {
                                touching_half = true;
                            }
                            _ => {}
                        }
                    }
                }

                if invalid && invalid_half {
                    return;
                }

                if !touching
                    && !touching_half
                    && *last != snapped
                    && *last != snapped_half
                    && connecting < 2
                {
                    path.points.push(snapped);
                    return;
                }

                if touching || connecting >= 2 {
                    path.points.push(snapped);
                } else if touching_half {
                    path.points.push(snapped_half);
                }

                info!("finishing path");
                path.drawing = false;

                let shape = shapes::Polygon {
                    points: path.points.clone(),
                    closed: false,
                };
                commands
                    .spawn_bundle(GeometryBuilder::build_as(
                        &shape,
                        ShapeColors::outlined(Color::NONE, Color::BLUE),
                        DrawMode::Outlined {
                            fill_options: FillOptions::default(),
                            outline_options: StrokeOptions::default().with_line_width(2.0),
                        },
                        Transform::default(),
                    ))
                    .insert(Road {
                        path: path.points.clone(),
                    });

                path.points.clear();
            } else {
                if path.drawing {
                    return;
                }

                // TODO point-segment collision check for first point of
                // new line.
                // TODO er, pretty sure we didn't need that.

                let snapped = snap_to_grid(mouse.position, GRID_SIZE);
                let snapped_half = snap_to_grid(mouse.position, HALF_GRID_SIZE);

                let mut ok_half = false;
                let mut ok = q_terminuses
                    .iter()
                    .inspect(|t| info!("{:?}:", t.point))
                    .any(|t| t.point == snapped);

                if !ok {
                    for r in q_roads.iter() {
                        for (a, b) in r.path.iter().tuple_windows() {
                            match point_segment_collision(snapped, *a, *b) {
                                SegmentCollision::Connecting => ok = true,
                                SegmentCollision::Touching => ok = true,
                                _ => {}
                            };
                            match point_segment_collision(snapped_half, *a, *b) {
                                SegmentCollision::Connecting => ok_half = true,
                                SegmentCollision::Touching => ok_half = true,
                                _ => {}
                            };
                        }
                    }
                }

                if ok || ok_half {
                    path.drawing = true;
                    path.points.clear();

                    if ok {
                        path.points.push(snapped);
                    } else {
                        path.points.push(snapped_half)
                    }
                }
            }
        }
    }
}

/// set up a simple 3D scene
fn setup(mut commands: Commands) {
    commands
        .spawn_bundle(OrthographicCameraBundle::new_2d())
        .insert(MainCamera);

    let points = [
        snap_to_grid(Vec2::new(-500.0, -300.0), GRID_SIZE),
        snap_to_grid(Vec2::new(-500.0, 300.0), GRID_SIZE),
        snap_to_grid(Vec2::new(500.0, -300.0), GRID_SIZE),
        snap_to_grid(Vec2::new(500.0, 300.0), GRID_SIZE),
    ];

    for p in points.iter() {
        commands
            .spawn_bundle(GeometryBuilder::build_as(
                &shapes::Circle {
                    radius: 5.0,
                    center: p.clone(),
                },
                ShapeColors::outlined(Color::NONE, Color::BLUE),
                DrawMode::Outlined {
                    fill_options: FillOptions::default(),
                    outline_options: StrokeOptions::default().with_line_width(2.0),
                },
                Transform::default(),
            ))
            .insert(Terminus { point: p.clone() });
    }
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
