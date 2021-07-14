use crate::collision::{point_segment_collision, segment_collision, SegmentCollision};

use bevy::ecs::schedule::GraphNode;
use bevy::{
    input::mouse::MouseButtonInput, input::ElementState::Released, prelude::*, utils::HashSet,
    window::CursorMoved,
};
use bevy_prototype_lyon::prelude::*;
use itertools::Itertools;
use petgraph::algo::{astar, dijkstra, min_spanning_tree};
use petgraph::dot::{Config, Dot};
use petgraph::graph::{NodeIndex, UnGraph};

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
    app.add_system(button_system.system());
    app.add_system(move_pixies.system());
    app.init_resource::<DrawingState>();
    app.init_resource::<MouseState>();
    app.init_resource::<RoadGraph>();
    app.init_resource::<ButtonMaterials>();
    app.run();
}

mod collision;

struct MainCamera;
struct Cursor;
struct DrawingLine;
struct GridPoint;
struct RoadChunk {
    points: Vec<Vec2>,
}

#[derive(Debug)]
struct PointGraphNode(NodeIndex);
#[derive(Debug)]
struct ChunkGraphNodes(NodeIndex, NodeIndex);

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
    start_nodes: Vec<NodeIndex>,
    end_nodes: Vec<NodeIndex>,
    axis_preference: Option<Axis>,
}

#[derive(Default, Debug)]
struct Terminus {
    point: Vec2,
    emits: HashSet<u32>,
    collects: HashSet<u32>,
}
struct Pixie {
    flavor: u32,
    path: Vec<Vec2>,
    path_index: usize,
}

#[derive(Default)]
struct RoadGraph {
    graph: UnGraph<Entity, i32>,
}

#[derive(Default, Debug)]
struct MouseState {
    position: Vec2,
}

enum Collider {
    Point(Vec2),
    Segment((Vec2, Vec2)),
}

struct ButtonMaterials {
    normal: Handle<ColorMaterial>,
    hovered: Handle<ColorMaterial>,
    pressed: Handle<ColorMaterial>,
}

impl FromWorld for ButtonMaterials {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.get_resource_mut::<Assets<ColorMaterial>>().unwrap();
        ButtonMaterials {
            normal: materials.add(Color::rgb(0.15, 0.15, 0.15).into()),
            hovered: materials.add(Color::rgb(0.25, 0.25, 0.25).into()),
            pressed: materials.add(Color::rgb(0.35, 0.75, 0.35).into()),
        }
    }
}
fn button_system(
    button_materials: Res<ButtonMaterials>,
    mut interaction_query: Query<
        (&Interaction, &mut Handle<ColorMaterial>, &Children),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
    mut graph: ResMut<RoadGraph>,
    q_terminuses: Query<(&Terminus, &PointGraphNode)>,
    q_road_chunks: Query<(&RoadChunk, &ChunkGraphNodes)>,
    mut commands: Commands,
) {
    for (interaction, mut material, children) in interaction_query.iter_mut() {
        let mut text = text_query.get_mut(children[0]).unwrap();
        match *interaction {
            Interaction::Clicked => {
                *material = button_materials.pressed.clone();

                for (a, a_node) in q_terminuses.iter() {
                    for (b, b_node) in q_terminuses.iter() {
                        for flavor in a.emits.intersection(&b.collects) {
                            info!(
                                "Pixie (flavor {}) wants to go from {:?} to {:?}",
                                flavor, a_node, b_node
                            );

                            let path = astar(
                                &graph.graph,
                                a_node.0,
                                |finish| finish == b_node.0,
                                |e| *e.weight(),
                                |_| 0,
                            );

                            let mut screen_path = vec![];
                            let mut last_ent = None;

                            if let Some(path) = path {
                                for node in path.1 {
                                    if let Some(ent) = graph.graph.node_weight(node) {
                                        if last_ent.is_some() && ent == last_ent.unwrap() {
                                            continue;
                                        }
                                        last_ent = Some(ent);
                                        info!("{:?} ({:?})", node, *ent);
                                        for (t, _) in q_terminuses.get(*ent) {
                                            screen_path.push(t.point);
                                            info!("-> {:?}", t.point);
                                        }
                                        for (s, _) in q_road_chunks.get(*ent) {
                                            if s.points.first().unwrap()
                                                == screen_path.last().unwrap()
                                            {
                                                for p in s.points.iter().skip(1) {
                                                    screen_path.push(*p);
                                                    info!("-> {:?}", *p);
                                                }
                                            } else if s.points.last().unwrap()
                                                == screen_path.last().unwrap()
                                            {
                                                for p in s.points.iter().rev().skip(1) {
                                                    screen_path.push(*p);
                                                    info!("-> {:?} (rev)", *p);
                                                }
                                            } else {
                                                info!("busted? {:?}", s.points);
                                            }
                                        }
                                    }
                                }
                            }

                            // blindly assume that all worked, create the pixie

                            if screen_path.len() < 1 {
                                // we should not be allowed to release the pixies
                                // if the required paths are not present.
                                continue;
                            }

                            let colors = [Color::PURPLE, Color::PINK];

                            let shape = shapes::RegularPolygon {
                                sides: 6,
                                feature: shapes::RegularPolygonFeature::Radius(6.0),
                                ..shapes::RegularPolygon::default()
                            };

                            commands
                                .spawn_bundle(GeometryBuilder::build_as(
                                    &shape,
                                    ShapeColors::new(colors[(flavor - 1) as usize]),
                                    DrawMode::Fill(FillOptions::default()),
                                    Transform::from_translation(
                                        screen_path.first().unwrap().extend(1.0),
                                    ),
                                ))
                                .insert(Pixie {
                                    flavor: *flavor,
                                    path: screen_path,
                                    path_index: 0,
                                });
                        }
                    }
                }
            }
            Interaction::Hovered => {
                *material = button_materials.hovered.clone();
            }
            Interaction::None => {
                *material = button_materials.normal.clone();
            }
        }
    }
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
    mut graph: ResMut<RoadGraph>,
    wnds: Res<Windows>,
    q_camera: Query<&Transform, With<MainCamera>>,
    q_terminuses: Query<&Terminus>,
    q_colliders: Query<(Entity, &Parent, &Collider)>,
    q_point_nodes: Query<&PointGraphNode>,
    q_chunk_nodes: Query<&ChunkGraphNodes>,
    q_road_chunks: Query<&RoadChunk>,
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
                        q_colliders.iter().any(|(_e, _p, c)| match c {
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
                            Collider::Point(p) => match point_segment_collision(*p, *a, *b) {
                                SegmentCollision::Connecting => *p != draw.start && *p != draw.end,
                                SegmentCollision::None => false,
                                _ => true,
                            },
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

                draw.start_nodes = vec![];
                draw.end_nodes = vec![];
                for (e, parent, c) in q_colliders.iter() {
                    match c {
                        Collider::Point(p) => {
                            if let Some(start) = draw.points.first() {
                                if *p == *start {
                                    if let Ok(node) = q_point_nodes.get(parent.0) {
                                        info!("start, so pushing a node");
                                        draw.start_nodes.push(node.0);
                                    }
                                }
                            }
                            if let Some(end) = draw.points.last() {
                                if *p == *end {
                                    if let Ok(node) = q_point_nodes.get(parent.0) {
                                        info!("end matched, so pushing a node");
                                        draw.end_nodes.push(node.0);
                                    }
                                }
                            }
                        }
                        Collider::Segment(_s) => {
                            if let Ok(chunk) = q_road_chunks.get(parent.0) {
                                if let Ok(nodes) = q_chunk_nodes.get(parent.0) {
                                    if let Some(start) = draw.points.first() {
                                        if let Some(chunk_start) = chunk.points.first() {
                                            if start == chunk_start {
                                                draw.start_nodes.push(nodes.0);
                                            }
                                        }
                                    }
                                    if let Some(start) = draw.points.first() {
                                        if let Some(chunk_end) = chunk.points.last() {
                                            if start == chunk_end {
                                                draw.start_nodes.push(nodes.1);
                                            }
                                        }
                                    }
                                    if let Some(end) = draw.points.last() {
                                        if let Some(chunk_start) = chunk.points.first() {
                                            if end == chunk_start {
                                                draw.end_nodes.push(nodes.0);
                                            }
                                        }
                                    }
                                    if let Some(end) = draw.points.last() {
                                        if let Some(chunk_end) = chunk.points.last() {
                                            if end == chunk_end {
                                                draw.end_nodes.push(nodes.1);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                draw.start_nodes.sort();
                draw.start_nodes.dedup();
                draw.end_nodes.sort();
                draw.end_nodes.dedup();
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
                    let ent = commands
                        .spawn_bundle(GeometryBuilder::build_as(
                            &shape,
                            ShapeColors::outlined(Color::NONE, Color::PINK),
                            DrawMode::Outlined {
                                fill_options: FillOptions::default(),
                                outline_options: StrokeOptions::default().with_line_width(2.0),
                            },
                            Transform::default(),
                        ))
                        .insert(RoadChunk {
                            points: draw.points.clone(),
                        })
                        .with_children(|parent| {
                            for (a, b) in draw.points.iter().tuple_windows() {
                                parent.spawn().insert(Collider::Segment((*a, *b)));
                            }
                        })
                        .id();

                    let start_node = graph.graph.add_node(ent);
                    let end_node = graph.graph.add_node(ent);

                    commands
                        .entity(ent)
                        .insert(ChunkGraphNodes(start_node, end_node));

                    info!(
                        "Adding road chunk with entity: {:?} and node indexes: {:?} {:?}",
                        ent, start_node, end_node
                    );

                    graph.graph.add_edge(start_node, end_node, 0);
                    for node in draw.start_nodes.iter() {
                        info!("Also attaching this chunk to {:?}", node);
                        graph.graph.add_edge(*node, start_node, 0);
                    }
                    for node in draw.end_nodes.iter() {
                        info!("Also attaching this chunk to {:?}", node);
                        graph.graph.add_edge(end_node, *node, 0);
                    }

                    println!(
                        "{:?}",
                        Dot::with_config(&graph.graph, &[Config::EdgeNoLabel])
                    );
                    draw.start = draw.end;
                    draw.points = vec![];
                    draw.start_nodes = vec![];
                    draw.end_nodes = vec![];
                }
            }
        }
    }
}

fn move_pixies(time: Res<Time>, mut query: Query<(&mut Pixie, &mut Transform)>) {
    for (mut pixie, mut transform) in query.iter_mut() {
        if pixie.path_index >= pixie.path.len() - 1 {
            continue;
        }

        let next_waypoint = pixie.path[pixie.path_index + 1];

        let dist = transform.translation.truncate().distance(next_waypoint);

        let delta = time.delta_seconds();

        let speed = 60.0;
        let step = speed * delta;

        // ten radians per second, clockwise
        transform.rotate(Quat::from_rotation_z(-10.0 * delta));

        if step < dist {
            transform.translation.x += step / dist * (next_waypoint.x - transform.translation.x);
            transform.translation.y += step / dist * (next_waypoint.y - transform.translation.y);
        } else {
            transform.translation.x = next_waypoint.x;
            transform.translation.y = next_waypoint.y;
            pixie.path_index += 1;
        }
    }
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut graph: ResMut<RoadGraph>,
    asset_server: Res<AssetServer>,
    button_materials: Res<ButtonMaterials>,
) {
    commands
        .spawn_bundle(OrthographicCameraBundle::new_2d())
        .insert(MainCamera);
    commands.spawn_bundle(UiCameraBundle::default());

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
        (
            snap_to_grid(Vec2::new(-500.0, -300.0), GRID_SIZE),
            vec![1],
            vec![],
        ),
        (
            snap_to_grid(Vec2::new(-500.0, 300.0), GRID_SIZE),
            vec![2],
            vec![],
        ),
        (
            snap_to_grid(Vec2::new(500.0, -300.0), GRID_SIZE),
            vec![],
            vec![2],
        ),
        (
            snap_to_grid(Vec2::new(500.0, 300.0), GRID_SIZE),
            vec![],
            vec![1],
        ),
    ];

    for (p, emits, collects) in points.iter() {
        let ent = commands
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
            .insert(Terminus {
                point: p.clone(),
                emits: emits.iter().cloned().collect(),
                collects: collects.iter().cloned().collect(),
            })
            .with_children(|parent| {
                parent.spawn().insert(Collider::Point(p.clone()));
            })
            .id();

        let node = graph.graph.add_node(ent);

        commands.entity(ent).insert(PointGraphNode(node));
    }

    println!(
        "{:?}",
        Dot::with_config(&graph.graph, &[Config::EdgeNoLabel])
    );

    commands
        .spawn_bundle(ButtonBundle {
            style: Style {
                size: Size::new(Val::Px(300.0), Val::Px(65.0)),
                // center button
                margin: Rect {
                    left: Val::Auto,
                    right: Val::Auto,
                    bottom: Val::Px(10.0),
                    ..Default::default()
                },
                // horizontally center child text
                justify_content: JustifyContent::Center,
                // vertically center child text
                align_items: AlignItems::Center,
                ..Default::default()
            },
            material: button_materials.normal.clone(),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(TextBundle {
                text: Text::with_section(
                    "Release The Pixies",
                    TextStyle {
                        font: asset_server.load("fonts/CooperHewitt-Medium.ttf"),
                        font_size: 40.0,
                        color: Color::rgb(0.9, 0.9, 0.9),
                    },
                    Default::default(),
                ),
                ..Default::default()
            });
        });
}
