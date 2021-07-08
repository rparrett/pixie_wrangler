use crate::collision::{point_segment_collision, segment_collision, SegmentCollision};
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

mod collision;

struct MainCamera;

struct GridPoint;
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

    // no line to draw
    let point = path.points.last();
    if point.is_none() {
        return;
    }
    let point = point.unwrap();

    let snapped = snap_to_angle(point.clone(), mouse.position, 8, GRID_SIZE);

    // no line to draw
    if snapped == *point {
        return;
    }

    // need to be able to snap to a half grid to properly connect to the middle of
    // some diagonal lines.

    let snapped_half = snap_to_angle(point.clone(), mouse.position, 8, HALF_GRID_SIZE);

    let mut invalid = false;
    let mut invalid_half = false;

    // no self-collisions at all
    for (a, b) in path.points.iter().tuple_windows() {
        match segment_collision(*a, *b, *point, snapped) {
            SegmentCollision::Intersecting
            | SegmentCollision::Overlapping
            | SegmentCollision::Touching => {
                invalid = true;
            }
            _ => {}
        };
        match point_segment_collision(snapped, *a, *b) {
            SegmentCollision::Connecting => invalid = true,
            _ => {}
        }
        match segment_collision(*a, *b, *point, snapped_half) {
            SegmentCollision::Intersecting
            | SegmentCollision::Overlapping
            | SegmentCollision::Touching => {
                invalid_half = true;
            }
            _ => {}
        };
    }

    let mut touching = false;
    let mut touching_half = false;
    let mut connecting = false;

    for (a, b) in q_roads.iter().flat_map(|r| r.path.iter().tuple_windows()) {
        match segment_collision(*a, *b, *point, snapped) {
            SegmentCollision::Intersecting | SegmentCollision::Overlapping => {
                invalid = true;
            }
            SegmentCollision::Touching => {
                touching = true;
            }
            _ => {}
        };
        match point_segment_collision(snapped, *a, *b) {
            SegmentCollision::Connecting => {
                connecting = true;
            }
            _ => {}
        }
        match segment_collision(*a, *b, *point, snapped_half) {
            SegmentCollision::Intersecting | SegmentCollision::Overlapping => {
                invalid_half = true;
            }
            _ => {}
        };
        match point_segment_collision(snapped_half, *a, *b) {
            SegmentCollision::Touching => {
                touching_half = true;
            }
            _ => {}
        }
    }

    let color = if invalid && invalid_half {
        Color::RED
    } else if touching || touching_half {
        Color::AQUAMARINE
    } else if connecting {
        Color::ALICE_BLUE
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

                // TODO when considering "touching" "connecting" collisions, it would probably
                // make more sense to do a separate point collision check just for the point
                // under the cursor

                // no self-collisions at all
                for (a, b) in path.points.iter().tuple_windows() {
                    match segment_collision(*a, *b, *last, snapped) {
                        SegmentCollision::Intersecting
                        | SegmentCollision::Overlapping
                        | SegmentCollision::Touching => {
                            invalid = true;
                        }
                        _ => {}
                    };
                    match point_segment_collision(snapped, *a, *b) {
                        SegmentCollision::Connecting => invalid = true,
                        _ => {}
                    }
                    match segment_collision(*a, *b, *last, snapped_half) {
                        SegmentCollision::Intersecting
                        | SegmentCollision::Overlapping
                        | SegmentCollision::Touching => {
                            invalid_half = true;
                        }
                        _ => {}
                    };
                }

                let mut touching = false;
                let mut touching_half = false;
                let mut connecting = false;

                for (a, b) in q_roads.iter().flat_map(|r| r.path.iter().tuple_windows()) {
                    match segment_collision(*a, *b, *last, snapped) {
                        SegmentCollision::Intersecting | SegmentCollision::Overlapping => {
                            invalid = true;
                        }

                        _ => {}
                    };
                    match point_segment_collision(snapped, *a, *b) {
                        SegmentCollision::Connecting => {
                            connecting = true;
                        }
                        SegmentCollision::Touching => {
                            touching = true;
                        }
                        _ => {}
                    }
                    match segment_collision(*a, *b, *last, snapped_half) {
                        SegmentCollision::Intersecting | SegmentCollision::Overlapping => {
                            invalid_half = true;
                        }
                        _ => {}
                    };
                    match point_segment_collision(snapped_half, *a, *b) {
                        SegmentCollision::Touching => {
                            touching_half = true;
                        }
                        _ => {}
                    }
                }

                if q_terminuses.iter().any(|t| t.point == snapped) {
                    connecting = true;
                }

                if invalid && invalid_half {
                    return;
                }

                if !touching
                    && !touching_half
                    && *last != snapped
                    && *last != snapped_half
                    && !connecting
                {
                    path.points.push(snapped);
                    return;
                }

                if touching || connecting {
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

    for x in ((-50 * (GRID_SIZE as i32))..50 * (GRID_SIZE as i32)).step_by(GRID_SIZE as usize) {
        for y in (-30 * (GRID_SIZE as i32)..30 * (GRID_SIZE as i32)).step_by(GRID_SIZE as usize) {
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
