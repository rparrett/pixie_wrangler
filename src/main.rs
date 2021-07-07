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
    app.add_system(draw_current_shape.system().after("mouse")); // after mouse
    app.add_system(draw_current_line.system().after("mouse")); // after mouse
    app.init_resource::<PathDrawingState>();
    app.init_resource::<MouseState>();
    app.run();
}

struct MainCamera;

struct CurrentShape;
struct CurrentLine;
struct Road;

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
    Collinear,
    Parallel,
    Connecting,
    Touching,
    Intersecting,
    None,
}

// https://github.com/pgkelley4/line-segments-intersect/blob/master/js/line-segments-intersect.js
fn segment_collision(a1: Vec2, a2: Vec2, b1: Vec2, b2: Vec2) -> SegmentCollision {
    let da = a2 - a1;
    let db = b2 - b1;
    let dab = b1 - a1;

    let numerator = dab.perp_dot(da);
    let denominator = da.perp_dot(db);

    if numerator == 0.0 && denominator == 0.0 {
        // TODO are these collinear and overlapping?
        // collinear and merely touching endpoints?
        info!(
            "{} {} {} {} / {} {} {} {}",
            a1.x - b1.x,
            a1.x - b2.x,
            a2.x - b1.x,
            a2.x - b2.x,
            a1.y - b1.y,
            a1.y - b2.y,
            a2.y - b1.y,
            a2.y - b2.y,
        );
        let diffsX = vec![a1.x - b1.x, a1.x - b2.x, a2.x - b1.x, a2.x - b2.x];
        let diffsY = vec![a1.y - b1.y, a1.y - b2.y, a2.y - b1.y, a2.y - b2.y];

        let nonZeroesX = diffsX
            .iter()
            .cloned()
            .filter(|d| *d != 0.0)
            .map(|d| d < 0.0);
        let nonZeroesY = diffsY
            .iter()
            .cloned()
            .filter(|d| *d != 0.0)
            .map(|d| d < 0.0);

        let fuk = nonZeroesX.clone();
        let fuky = nonZeroesX.clone();

        info!("{:?}", fuk.collect::<Vec<_>>());
        info!("{:?}", fuky.collect::<Vec<_>>());

        let watX = nonZeroesX.clone();
        let watY = nonZeroesY.clone();

        if (nonZeroesX
            .tuple_windows()
            .inspect(|(a, b)| info!("{} {}", a, b))
            .any(|(a, b)| a != b))
        {
            return SegmentCollision::Collinear;
        }
        if (nonZeroesY.tuple_windows().any(|(a, b)| a != b)) {
            return SegmentCollision::Collinear;
        }

        let xc = watX.count();
        let yc = watY.count();

        if (xc == 3) {
            return SegmentCollision::Connecting;
        }

        if (yc == 3) {
            return SegmentCollision::Connecting;
        }

        return SegmentCollision::Collinear;
    }

    if denominator == 0.0 {
        return SegmentCollision::Parallel;
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

fn snap_to_angle(start: Vec2, end: Vec2, divisions: u32) -> Vec2 {
    let diff = end - start;

    let angle = diff.y.atan2(diff.x);

    let increment = std::f32::consts::TAU / divisions as f32;

    let snap_angle = (angle / increment).round() * increment;

    if snap_angle.to_degrees() == 90.0 || snap_angle.to_degrees() == -90.0 {
        return snap_to_grid(start + Vec2::new(0.0, diff.y), GRID_SIZE);
    }

    if snap_angle.to_degrees() == 0.0
        || snap_angle.to_degrees() == 180.0
        || snap_angle.to_degrees() == -180.0
    {
        return snap_to_grid(start + Vec2::new(diff.x, 0.0), GRID_SIZE);
    }

    let snapped_x = snap_to_grid_1d(diff.x, GRID_SIZE);
    let snapped_y = snap_to_grid_1d(diff.y, GRID_SIZE);

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
        let snapped = snap_to_angle(point.clone(), mouse.position, 8);
        if snapped == *point {
            return;
        }

        info!("");
        for (i, (a, b)) in path.points.iter().tuple_windows().enumerate() {
            info!("{}: {:?}", i, segment_collision(*a, *b, *point, snapped));
        }

        let points = vec![point.clone(), snapped];

        let shape = shapes::Polygon {
            points,
            closed: false,
        };
        commands
            .spawn_bundle(GeometryBuilder::build_as(
                &shape,
                ShapeColors::outlined(Color::NONE, Color::GREEN),
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
            if !path.drawing {
                path.drawing = true;
                path.points.clear();
            }

            // TODO check for collisions

            if let Some(last) = path.points.last() {
                let point = snap_to_angle(*last, mouse.position, 8);

                if *last == point {
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
                        .insert(Road);
                } else {
                    path.points.push(point);
                }
            } else {
                let point = snap_to_grid(mouse.position, GRID_SIZE);
                path.points.push(point);
            }
        }
    }
}

/// set up a simple 3D scene
fn setup(mut commands: Commands) {
    commands
        .spawn_bundle(OrthographicCameraBundle::new_2d())
        .insert(MainCamera);
}
