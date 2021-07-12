use crate::collision::{point_segment_collision, segment_collision, SegmentCollision};
use bevy::{
    input::mouse::MouseButtonInput, input::ElementState::Released, prelude::*, window::CursorMoved,
};
use bevy_prototype_lyon::prelude::*;
use itertools::Itertools;

const GRID_SIZE: f32 = 25.0;

fn main() {
    let mut app = App::build();
    app.insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins);
    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);
    app.add_plugin(ShapePlugin);
    app.add_startup_system(setup.system());
    app.add_system(mouse_events_system.system().label("mouse"));
    app.add_system(draw_mouse.system().after("mouse")); // after mouse
    app.init_resource::<DrawingState>();
    app.init_resource::<MouseState>();
    app.run();
}

mod collision;

struct MainCamera;
struct Cursor;
struct DrawingLine;
struct GridPoint;
struct RoadChunk;

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
}
#[derive(Default)]
struct DrawingState {
    drawing: bool,
    start: Vec2,
    end: Vec2,
    valid: bool,
    points: Vec<Vec2>,
    axis_preference: Option<Axis>,
}

struct Terminus {
    point: Vec2,
}

#[derive(Default, Debug)]
struct MouseState {
    position: Vec2,
}

enum Collider {
    Point(Vec2),
    Segment((Vec2, Vec2)),
}

fn snap_to_grid(position: Vec2, grid_size: f32) -> Vec2 {
    let new = (position / grid_size).round() * grid_size;

    new
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
fn possible_lines(from: Vec2, to: Vec2, axis_preference: Option<Axis>) -> Vec<Vec<Vec2>> {
    let diff = to - from;

    // if a single 45 degree or 90 degree line does the job,
    // return that.
    if diff.x == 0.0 || diff.y == 0.0 || diff.x.abs() == diff.y.abs() {
        return vec![vec![from, to]];
    }

    let (a, b) = if diff.x.abs() < diff.y.abs() {
        (
            Vec2::new(to.x, from.y + diff.x.abs() * diff.y.signum()),
            Vec2::new(from.x, to.y - diff.x.abs() * diff.y.signum()),
        )
    } else {
        (
            Vec2::new(to.x - diff.y.abs() * diff.x.signum(), from.y),
            Vec2::new(from.x + diff.y.abs() * diff.x.signum(), to.y),
        )
    };

    if matches!(axis_preference, Some(Axis::X)) {
        return vec![vec![from, a, to], vec![from, b, to]];
    }

    vec![vec![from, b, to], vec![from, a, to]]
}

fn draw_mouse(
    mut commands: Commands,
    draw: Res<DrawingState>,
    mouse: Res<MouseState>,
    q_cursor: Query<Entity, With<Cursor>>,
    q_drawing: Query<Entity, With<DrawingLine>>,
) {
    if !mouse.is_changed() {
        return;
    }

    for cursor in q_cursor.iter().chain(q_drawing.iter()) {
        commands.entity(cursor).despawn();
    }

    let snapped = snap_to_grid(mouse.position, GRID_SIZE);

    let shape = shapes::Circle {
        radius: 5.5,
        center: snapped,
    };
    let color = Color::WHITE;
    commands
        .spawn_bundle(GeometryBuilder::build_as(
            &shape,
            ShapeColors::new(color),
            DrawMode::Stroke(StrokeOptions::default().with_line_width(2.0)),
            Transform::default(),
        ))
        .insert(Cursor);

    if draw.drawing {
        // TODO filter presented options by whether or not they
        // collide with another line.

        let color = if draw.valid {
            Color::SEA_GREEN
        } else {
            Color::RED
        };

        let shape = shapes::Polygon {
            points: draw.points.clone(),
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
            .insert(DrawingLine);
    }
}

/// This system prints out all mouse events as they come in
fn mouse_events_system(
    mut commands: Commands,
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut draw: ResMut<DrawingState>,
    mut mouse: ResMut<MouseState>,
    wnds: Res<Windows>,
    q_camera: Query<&Transform, With<MainCamera>>,
    q_terminuses: Query<&Terminus>,
    q_colliders: Query<&Collider>,
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

    if draw.drawing {
        let snapped = snap_to_grid(mouse.position, GRID_SIZE);

        if snapped != draw.end {
            draw.end = snapped;

            // when we begin drawing, set the "axis preference" corresponding to the
            // direction the player initially moves the mouse.
            if !draw.axis_preference.is_some() && snapped != draw.start {
                let diff = (snapped - draw.start).abs();
                if diff.x > diff.y {
                    draw.axis_preference = Some(Axis::X);
                } else {
                    draw.axis_preference = Some(Axis::Y);
                }
            } else if draw.axis_preference.is_some() && snapped == draw.start {
                draw.axis_preference = None;
            }

            // TODO we need to allow lines to both start and end with
            // SegmentCollision::Touching and split the RoadChunk(s) in that case.
            // TODO we need to handle SegmentCollision::Connecting and combine the
            // RoadChunk(s) in that case.

            if snapped != draw.start {
                let possible = possible_lines(draw.start, snapped, draw.axis_preference);
                let mut filtered = possible.iter().filter(|possibility| {
                    !possibility.iter().tuple_windows().any(|(a, b)| {
                        q_colliders.iter().any(|c| match c {
                            Collider::Segment(s) => match segment_collision(s.0, s.1, *a, *b) {
                                SegmentCollision::Intersecting => true,
                                SegmentCollision::Overlapping => true,
                                SegmentCollision::Touching => {
                                    // "Touching" collisions are allowed only if they are the
                                    // start or end of the line we are currently drawing.
                                    //
                                    // Ideally, segment_collision would return the intersection
                                    // point(s) and we could just check that.

                                    !matches!(
                                        point_segment_collision(draw.start, s.0, s.1),
                                        SegmentCollision::Touching
                                    ) && !matches!(
                                        point_segment_collision(draw.end, s.0, s.1),
                                        SegmentCollision::Touching
                                    )
                                }
                                SegmentCollision::Connecting => {
                                    // "Connecting" collisions are allowed only if they are the
                                    // start or end of the line we are currently drawing.
                                    //
                                    // Ideally, segment_collision would return the intersection
                                    // point(s) and we could just check that.

                                    !matches!(
                                        point_segment_collision(draw.start, s.0, s.1),
                                        SegmentCollision::Connecting
                                    ) && !matches!(
                                        point_segment_collision(draw.end, s.0, s.1),
                                        SegmentCollision::Connecting
                                    )
                                }
                                _ => false,
                            },
                            _ => false,
                        })
                    })
                });

                if let Some(points) = filtered.next() {
                    draw.points = points.clone();
                    draw.valid = true;
                } else if let Some(points) = possible.iter().next() {
                    draw.points = points.clone();
                    draw.valid = false;
                } else {
                    draw.points = vec![];
                    draw.valid = false;
                }
            } else {
                draw.points = vec![];
                draw.valid = false;
            }
        }
    }

    for event in mouse_button_input_events.iter() {
        if event.button == MouseButton::Left && event.state == Released {
            if !draw.drawing {
                // TODO is it ok to start drawing here?
                draw.drawing = true;
                draw.start = snap_to_grid(mouse.position, GRID_SIZE);
                draw.end = draw.start;
            } else {
                // TODO is it ok to end drawing here?
                if draw.end == draw.start {
                    draw.drawing = false;
                }

                if !draw.points.is_empty() {
                    let shape = shapes::Polygon {
                        points: draw.points.clone(),
                        closed: false,
                    };
                    commands
                        .spawn_bundle(GeometryBuilder::build_as(
                            &shape,
                            ShapeColors::outlined(Color::NONE, Color::PINK),
                            DrawMode::Outlined {
                                fill_options: FillOptions::default(),
                                outline_options: StrokeOptions::default().with_line_width(2.0),
                            },
                            Transform::default(),
                        ))
                        .insert(RoadChunk)
                        .with_children(|parent| {
                            for (a, b) in draw.points.iter().tuple_windows() {
                                parent.spawn().insert(Collider::Segment((*a, *b)));
                            }
                        });

                    draw.start = draw.end;
                    draw.points = vec![];
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

    for x in ((-25 * (GRID_SIZE as i32))..=25 * (GRID_SIZE as i32)).step_by(GRID_SIZE as usize) {
        for y in (-15 * (GRID_SIZE as i32)..=15 * (GRID_SIZE as i32)).step_by(GRID_SIZE as usize) {
            commands
                .spawn_bundle(GeometryBuilder::build_as(
                    &shapes::Circle {
                        radius: 2.5,
                        center: Vec2::new(x as f32, y as f32),
                    },
                    ShapeColors::new(Color::DARK_GRAY),
                    DrawMode::Fill(FillOptions::default()),
                    Transform::default(),
                ))
                .insert(GridPoint);
        }
    }

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
                    radius: 5.5,
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
