use crate::collision::{point_segment_collision, segment_collision, SegmentCollision};
use bevy::{
    input::mouse::MouseButtonInput, input::ElementState::Released, prelude::*, window::CursorMoved,
};
use bevy_prototype_lyon::prelude::*;
use itertools::Itertools;

const GRID_SIZE: f32 = 16.0;

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

#[derive(Default)]
struct DrawingState {
    drawing: bool,
    start: Vec2,
}

struct Terminus {
    point: Vec2,
}

#[derive(Default, Debug)]
struct MouseState {
    position: Vec2,
}

fn snap_to_grid(position: Vec2, grid_size: f32) -> Vec2 {
    let new = (position / grid_size).round() * grid_size;

    new
}

/// Given a start and endpoint, return up to two points that represent the
/// middle of possible 45-degree-only two segment polylines that connect them.
/// In the case where a straight line path is possible, returns None.
///   i
///  /|
/// o o
/// |/
/// i
fn possible_lines(from: Vec2, to: Vec2) -> Vec<Vec<Vec2>> {
    let diff = to - from;

    // if a single 45 degree or 90 degree line does the job,
    // return that.
    if diff.x == 0.0 || diff.y == 0.0 || diff.x.abs() == diff.y.abs() {
        return vec![vec![from, to]];
    }

    if diff.x.abs() < diff.y.abs() {
        vec![
            vec![
                from,
                Vec2::new(from.x, to.y - diff.x.abs() * diff.y.signum()),
                to,
            ],
            vec![
                from,
                Vec2::new(to.x, from.y + diff.x.abs() * diff.y.signum()),
                to,
            ],
        ]
    } else {
        vec![
            vec![
                from,
                Vec2::new(to.x - diff.y.abs() * diff.x.signum(), from.y),
                to,
            ],
            vec![
                from,
                Vec2::new(from.x + diff.y.abs() * diff.x.signum(), to.y),
                to,
            ],
        ]
    }
}

fn snap_to_angle(start: Vec2, end: Vec2, divisions: u32, offset: f32, grid_size: f32) -> Vec2 {
    let diff = end - start;

    let angle = diff.y.atan2(diff.x);

    let increment = std::f32::consts::TAU / divisions as f32;
    let offset = increment * offset;

    let snap_angle = ((angle - offset) / increment).round() * increment + offset;

    if snap_angle.to_degrees().abs() == 90.0 {
        return snap_to_grid(Vec2::new(start.x, end.y), grid_size);
    }

    if snap_angle.to_degrees() == 0.0 || snap_angle.to_degrees().abs() == 180.0 {
        return snap_to_grid(Vec2::new(end.x, start.y), grid_size);
    }

    let snapped_end = snap_to_grid(end, grid_size);
    let snapped_diff = snapped_end - start;

    if (end.x - snapped_diff.x).abs() > (end.y - snapped_diff.y).abs() {
        return start + Vec2::new(snapped_diff.x, snapped_diff.x * snap_angle.tan());
    } else {
        return start + Vec2::new(snapped_diff.y / snap_angle.tan(), snapped_diff.y);
    }
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
        let possible = possible_lines(draw.start, snapped);
        let colors = [Color::SEA_GREEN, Color::LIME_GREEN];

        // TODO filter presented options by whether or not they
        // collide with another line.
        // TODO if both options are available, somehow make it
        // possible for the user to favor one side or the other.
        // perhaps by preferring the option where the 90 degree
        // follows the "first next" grid point snapped to after
        // beginning drawing (and resetting this "first next"
        // state when moving the cursor over the origin)

        for (i, points) in possible.iter().enumerate() {
            let shape = shapes::Polygon {
                points: points.clone(),
                closed: false,
            };
            commands
                .spawn_bundle(GeometryBuilder::build_as(
                    &shape,
                    ShapeColors::outlined(Color::NONE, colors[i]),
                    DrawMode::Outlined {
                        fill_options: FillOptions::default(),
                        outline_options: StrokeOptions::default().with_line_width(2.0),
                    },
                    Transform::default(),
                ))
                .insert(DrawingLine);
        }
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
            if !draw.drawing {
                // TODO is it ok to start drawing here?
                draw.drawing = true;
                draw.start = snap_to_grid(mouse.position, GRID_SIZE);
            } else {
                // TODO is it ok to end drawing here?
                draw.drawing = false;
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
