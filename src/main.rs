use crate::collision::{point_segment_collision, segment_collision, SegmentCollision};

use bevy::{
    input::mouse::MouseButtonInput, input::ElementState::Released, prelude::*, utils::HashSet,
    window::CursorMoved,
};
use bevy_prototype_lyon::prelude::*;
use petgraph::algo::astar;
use petgraph::dot::{Config, Dot};
use petgraph::graph::{NodeIndex, UnGraph};

fn main() {
    let mut app = App::build();
    app.insert_resource(ClearColor(BACKGROUND_COLOR))
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(WindowDescriptor {
            title: String::from("Pixie Wrangler"),
            ..Default::default()
        })
        .add_plugins(DefaultPlugins);
    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);
    app.add_plugin(ShapePlugin);
    app.add_startup_system(setup.system());
    app.add_system(keyboard_system.system().before("mouse"));
    app.add_system(mouse_movement.system().label("mouse"));
    app.add_system(
        drawing_mouse_movement
            .system()
            .label("drawing_mouse_movement")
            .after("mouse"),
    );
    app.add_system(drawing_mouse_click.system().after("drawing_mouse_movement"));
    app.add_system(draw_mouse.system().after("mouse"));
    app.add_system(button_system.system());
    app.add_system(move_pixies.system().label("pixies"));
    app.add_system(emit_pixies.system());
    app.add_system(update_score.system().after("pixies"));

    app.add_stage_after(CoreStage::Update, "after_update", SystemStage::parallel());
    app.add_system_to_stage("after_update", update_cost.system());
    app.init_resource::<DrawingState>();
    app.init_resource::<MouseState>();
    app.init_resource::<RoadGraph>();
    app.init_resource::<ButtonMaterials>();
    app.init_resource::<Score>();
    app.run();
}

mod collision;

struct MainCamera;
struct Cursor;
struct DrawingLine;
struct GridPoint;
struct ScoreText;
struct CostText;
#[derive(Default)]
struct Score(u32);
struct RoadSegment {
    points: (Vec2, Vec2),
}

#[derive(Debug)]
struct PointGraphNode(NodeIndex);
#[derive(Debug)]
struct SegmentGraphNodes(NodeIndex, NodeIndex);

#[derive(Clone, Copy, Debug)]
enum Axis {
    X,
    Y,
}

struct DrawingState {
    drawing: bool,
    start: Vec2,
    end: Vec2,
    valid: bool,
    segments: Vec<(Vec2, Vec2)>,
    start_nodes: Vec<NodeIndex>,
    end_nodes: Vec<NodeIndex>,
    axis_preference: Option<Axis>,
    layer: u32,
    prev_layer: u32,
}
impl Default for DrawingState {
    fn default() -> Self {
        Self {
            drawing: false,
            start: Vec2::new(0.0, 0.0),
            end: Vec2::new(0.0, 0.0),
            valid: false,
            segments: vec![],
            start_nodes: vec![],
            end_nodes: vec![],
            axis_preference: None,
            layer: 1,
            prev_layer: 1,
        }
    }
}

#[derive(Default, Debug)]
struct Terminus {
    point: Vec2,
    emits: HashSet<u32>,
    collects: HashSet<u32>,
}
struct Pixie {
    path: Vec<Vec2>,
    path_index: usize,
}

struct PixieEmitter {
    flavor: u32,
    path: Vec<Vec2>,
    remaining: u32,
    timer: Timer,
}

#[derive(Default)]
struct RoadGraph {
    graph: UnGraph<Entity, f32>,
}

#[derive(Default, Debug)]
struct MouseState {
    position: Vec2,
    snapped: Vec2,
    window_position: Vec2,
}

enum Collider {
    Point(Vec2),
    Segment((Vec2, Vec2)),
}
struct ColliderLayer(u32);

struct ButtonMaterials {
    normal: Handle<ColorMaterial>,
    hovered: Handle<ColorMaterial>,
    pressed: Handle<ColorMaterial>,
}
const GRID_SIZE: f32 = 48.0;
const BOTTOM_BAR_HEIGHT: f32 = 70.0;

const PIXIE_COLORS: [Color; 4] = [Color::AQUAMARINE, Color::PINK, Color::ORANGE, Color::PURPLE];
const FINISHED_ROAD_COLORS: [Color; 2] = [
    Color::rgb(0.251, 0.435, 0.729),
    Color::rgb(0.247, 0.725, 0.314),
];
const DRAWING_ROAD_COLORS: [Color; 2] = [
    Color::rgb(0.102, 0.18, 0.298),
    Color::rgb(0.102, 0.298, 0.125),
];
const BACKGROUND_COLOR: Color = Color::rgb(0.05, 0.066, 0.09);
const GRID_COLOR: Color = Color::rgb(0.086, 0.105, 0.133);
const UI_WHITE_COLOR: Color = Color::rgb(0.788, 0.82, 0.851);

const TUNNEL_MULTIPLIER: f32 = 2.0;

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
        (&Interaction, &mut Handle<ColorMaterial>),
        (Changed<Interaction>, With<Button>),
    >,
    graph: Res<RoadGraph>,
    q_terminuses: Query<(&Terminus, &PointGraphNode)>,
    q_road_chunks: Query<(&RoadSegment, &SegmentGraphNodes)>,
    mut commands: Commands,
) {
    for (interaction, mut material) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Clicked => {
                *material = button_materials.pressed.clone();

                let mut ok = true;
                let mut paths = vec![];

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
                                |_| 0.0,
                            );

                            if let Some(path) = path {
                                let mut world_path = vec![];
                                let mut last_ent = None;

                                for node in path.1 {
                                    if let Some(ent) = graph.graph.node_weight(node) {
                                        if last_ent.is_some() && ent == last_ent.unwrap() {
                                            continue;
                                        }
                                        last_ent = Some(ent);
                                        for (t, _) in q_terminuses.get(*ent) {
                                            world_path.push(t.point);
                                        }
                                        for (s, _) in q_road_chunks.get(*ent) {
                                            if s.points.0 == *world_path.last().unwrap() {
                                                world_path.push(s.points.1);
                                            } else if s.points.1 == *world_path.last().unwrap() {
                                                world_path.push(s.points.0);
                                            } else {
                                                info!(
                                                    "pretty sure this shouldn't happen {:?}",
                                                    s.points
                                                );
                                            }
                                        }
                                    }
                                }

                                if world_path.len() < 1 {
                                    ok = false;
                                    continue;
                                }

                                paths.push((flavor, world_path));
                            } else {
                                ok = false
                            }
                        }
                    }
                }

                if !ok || paths.len() < 1 {
                    // TODO tell user we can't do that yet.
                    // or better yet, do this path calc upon connecting to a terminus
                    // and grey out the button if the requirements are not met.

                    continue;
                }

                for (flavor, world_path) in paths {
                    commands.spawn().insert(PixieEmitter {
                        flavor: *flavor,
                        path: world_path,
                        remaining: 70,
                        timer: Timer::from_seconds(0.4, true),
                    });
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
fn possible_lines(from: Vec2, to: Vec2, axis_preference: Option<Axis>) -> Vec<Vec<(Vec2, Vec2)>> {
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

fn draw_mouse(
    mut commands: Commands,
    draw: Res<DrawingState>,
    mouse: Res<MouseState>,
    q_cursor: Query<Entity, With<Cursor>>,
    q_drawing: Query<Entity, With<DrawingLine>>,
) {
    if mouse.is_changed() || draw.is_changed() {
        let snapped = snap_to_grid(mouse.position, GRID_SIZE);

        for entity in q_cursor.iter() {
            commands.entity(entity).despawn();
        }
        let shape = shapes::Circle {
            radius: 5.5,
            center: snapped,
        };
        let color = DRAWING_ROAD_COLORS[draw.layer as usize - 1];
        commands
            .spawn_bundle(GeometryBuilder::build_as(
                &shape,
                ShapeColors::new(color.as_rgba_linear()),
                DrawMode::Stroke(StrokeOptions::default().with_line_width(2.0)),
                Transform::default(),
            ))
            .insert(Cursor);
    }

    if !draw.is_changed() {
        return;
    }

    for entity in q_drawing.iter() {
        commands.entity(entity).despawn();
    }

    if draw.drawing {
        let color = if draw.valid {
            DRAWING_ROAD_COLORS[draw.layer as usize - 1]
        } else {
            Color::RED
        };

        for (a, b) in draw.segments.iter() {
            commands
                .spawn_bundle(GeometryBuilder::build_as(
                    &shapes::Line(a.clone(), b.clone()),
                    ShapeColors::new(color.as_rgba_linear()),
                    DrawMode::Stroke(StrokeOptions::default().with_line_width(2.0)),
                    Transform::from_xyz(0.0, 0.0, 2.0),
                ))
                .insert(DrawingLine);
        }
    }
}

fn keyboard_system(keyboard_input: Res<Input<KeyCode>>, mut drawing_state: ResMut<DrawingState>) {
    if keyboard_input.pressed(KeyCode::Key1) {
        drawing_state.layer = 1;
    } else if keyboard_input.pressed(KeyCode::Key2) {
        drawing_state.layer = 2;
    }
}

fn drawing_mouse_click(
    mut commands: Commands,
    mouse: Res<MouseState>,
    mut draw: ResMut<DrawingState>,
    mut graph: ResMut<RoadGraph>,
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    q_colliders: Query<(Entity, &Parent, &Collider, &ColliderLayer)>,
    q_point_nodes: Query<&PointGraphNode>,
    q_segment_nodes: Query<&SegmentGraphNodes>,
    q_road_segments: Query<&RoadSegment>,
) {
    if !draw.is_changed() && !mouse.is_changed() {
        return;
    }

    if mouse.window_position.y < BOTTOM_BAR_HEIGHT {
        return;
    }

    for event in mouse_button_input_events.iter() {
        if event.button == MouseButton::Left && event.state == Released {
            if !draw.drawing {
                // TODO is it ok to start drawing here?
                draw.drawing = true;
                draw.start = snap_to_grid(mouse.position, GRID_SIZE);
                draw.end = draw.start;
            } else {
                if draw.end == draw.start {
                    draw.drawing = false;
                }

                if !draw.valid {
                    continue;
                }

                if draw.segments.is_empty() {
                    continue;
                }

                let mut segments = vec![];

                for (a, b) in draw.segments.iter() {
                    let color = FINISHED_ROAD_COLORS[draw.layer as usize - 1];
                    let ent = commands
                        .spawn_bundle(GeometryBuilder::build_as(
                            &shapes::Line(a.clone(), b.clone()),
                            ShapeColors::new(color.as_rgba_linear()),
                            DrawMode::Stroke(StrokeOptions::default().with_line_width(2.0)),
                            Transform::default(),
                        ))
                        .insert(RoadSegment {
                            points: (a.clone(), b.clone()),
                        })
                        .with_children(|parent| {
                            parent
                                .spawn()
                                .insert(Collider::Segment((*a, *b)))
                                .insert(ColliderLayer(draw.layer));
                        })
                        .id();

                    let start_node = graph.graph.add_node(ent);
                    let end_node = graph.graph.add_node(ent);

                    graph
                        .graph
                        .add_edge(start_node, end_node, (*a - *b).length());
                    commands
                        .entity(ent)
                        .insert(SegmentGraphNodes(start_node, end_node));
                    info!(
                        "Adding road chunk with entity: {:?} and node indexes: {:?} {:?}",
                        ent, start_node, end_node
                    );
                    segments.push((ent, start_node, end_node))
                }

                draw.start_nodes = vec![];
                draw.end_nodes = vec![];
                for (_entity, parent, collider, _layer) in q_colliders.iter() {
                    match collider {
                        Collider::Point(p) => {
                            if *p == draw.start {
                                if let Ok(node) = q_point_nodes.get(parent.0) {
                                    draw.start_nodes.push(node.0);
                                }
                            }
                            if *p == draw.end {
                                if let Ok(node) = q_point_nodes.get(parent.0) {
                                    draw.end_nodes.push(node.0);
                                }
                            }
                        }
                        Collider::Segment(_s) => {
                            // These are basically "connecting" collision checks
                            if let Ok(chunk) = q_road_segments.get(parent.0) {
                                if let Ok(nodes) = q_segment_nodes.get(parent.0) {
                                    if draw.start == chunk.points.0 {
                                        draw.start_nodes.push(nodes.0);
                                    }
                                    if draw.start == chunk.points.1 {
                                        draw.start_nodes.push(nodes.1);
                                    }
                                    if draw.end == chunk.points.0 {
                                        draw.end_nodes.push(nodes.0);
                                    }
                                    if draw.end == chunk.points.1 {
                                        draw.end_nodes.push(nodes.1);
                                    }
                                }
                            }
                        }
                    }
                }

                for (i, segment) in segments.iter().enumerate() {
                    if i == 0 {
                        // TODO this edge weight should be based on angle/length
                        for node in draw.start_nodes.iter() {
                            graph.graph.add_edge(*node, segment.1, 0.0);
                        }
                    }
                    if i == segments.len() - 1 {
                        // TODO this edge weight should be based on angle/length
                        for node in draw.end_nodes.iter() {
                            graph.graph.add_edge(segment.2, *node, 0.0);
                        }
                    }
                    if i < segments.len() - 1 {
                        // TODO this edge weight should be based on angle/length
                        graph.graph.add_edge(segment.2, segments[i + 1].1, 0.0);
                    }
                }

                println!(
                    "{:?}",
                    Dot::with_config(&graph.graph, &[Config::EdgeNoLabel])
                );

                draw.start = draw.end;
                draw.segments = vec![];
                draw.start_nodes = vec![];
                draw.end_nodes = vec![];
            }
        }
    }
}

fn mouse_movement(
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut mouse: ResMut<MouseState>,
    windows: Res<Windows>,
    q_camera: Query<&Transform, With<MainCamera>>,
) {
    // assuming there is exactly one main camera entity, so this is OK
    let camera_transform = q_camera.iter().next().unwrap();

    for event in cursor_moved_events.iter() {
        let window = windows.get(event.id).unwrap();
        let size = Vec2::new(window.width() as f32, window.height() as f32);

        let p = event.position - size / 2.0;

        mouse.position = (camera_transform.compute_matrix() * p.extend(0.0).extend(1.0))
            .truncate()
            .truncate();

        mouse.snapped = snap_to_grid(mouse.position, GRID_SIZE);

        info!("{:?}", mouse.snapped);

        mouse.window_position = event.position;
    }
}

fn drawing_mouse_movement(
    mut draw: ResMut<DrawingState>,
    mouse: Res<MouseState>,
    q_colliders: Query<(Entity, &Parent, &Collider, &ColliderLayer)>,
) {
    if !draw.drawing {
        return;
    }

    if mouse.snapped == draw.end && draw.layer == draw.prev_layer {
        return;
    }

    draw.end = mouse.snapped;
    draw.prev_layer = draw.layer;

    // when we begin drawing, set the "axis preference" corresponding to the
    // direction the player initially moves the mouse.

    let diff = (mouse.snapped - draw.start).abs() / GRID_SIZE;
    if diff.x <= 1.0 && diff.y <= 1.0 && mouse.snapped != draw.start {
        if diff.x > diff.y {
            draw.axis_preference = Some(Axis::X);
        } else if diff.y > diff.x {
            draw.axis_preference = Some(Axis::Y);
        }
    }

    if mouse.snapped == draw.start {
        draw.segments = vec![];
        draw.valid = false;
    }

    // TODO we need to allow lines to both start and end with
    // SegmentCollision::Touching and split the RoadSegment(s) in that case.
    // TODO we need to handle SegmentCollision::Connecting and combine the
    // RoadSegment(s) in that case.

    let possible = possible_lines(draw.start, mouse.snapped, draw.axis_preference);
    let mut filtered = possible.iter().filter(|possibility| {
        !possibility.iter().any(|(a, b)| {
            q_colliders.iter().any(|(_e, _p, c, layer)| match c {
                Collider::Segment(s) => match segment_collision(s.0, s.1, *a, *b) {
                    SegmentCollision::Intersecting => layer.0 == draw.layer || layer.0 == 0,
                    SegmentCollision::Overlapping => layer.0 == draw.layer || layer.0 == 0,
                    SegmentCollision::Touching => {
                        // "Touching" collisions are allowed only if they are the
                        // start or end of the line we are currently drawing.
                        //
                        // Ideally, segment_collision would return the intersection
                        // point(s) and we could just check that.

                        let bad = layer.0 == 0
                            || (!matches!(
                                point_segment_collision(draw.start, s.0, s.1),
                                SegmentCollision::Touching
                            ) && !matches!(
                                point_segment_collision(draw.end, s.0, s.1),
                                SegmentCollision::Touching
                            ));

                        bad
                    }
                    SegmentCollision::Connecting => {
                        // "Connecting" collisions are allowed only if they are the
                        // start or end of the line we are currently drawing.
                        //
                        // Ideally, segment_collision would return the intersection
                        // point(s) and we could just check that.

                        let bad = layer.0 == 0
                            || (!matches!(
                                point_segment_collision(draw.start, s.0, s.1),
                                SegmentCollision::Connecting
                            ) && !matches!(
                                point_segment_collision(draw.end, s.0, s.1),
                                SegmentCollision::Connecting
                            ));
                        bad
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

    if let Some(segments) = filtered.next() {
        draw.segments = segments.clone();
        draw.valid = true;
    } else if let Some(segments) = possible.iter().next() {
        draw.segments = segments.clone();
        draw.valid = false;
    } else {
        draw.segments = vec![];
        draw.valid = false;
    }
}

fn emit_pixies(time: Res<Time>, mut q_emitters: Query<&mut PixieEmitter>, mut commands: Commands) {
    for mut emitter in q_emitters.iter_mut() {
        if emitter.remaining <= 0 {
            continue;
        }

        emitter.timer.tick(time.delta());

        if !emitter.timer.finished() {
            continue;
        }

        let shape = shapes::RegularPolygon {
            sides: 6,
            feature: shapes::RegularPolygonFeature::Radius(6.0),
            ..shapes::RegularPolygon::default()
        };

        commands
            .spawn_bundle(GeometryBuilder::build_as(
                &shape,
                ShapeColors::new(PIXIE_COLORS[(emitter.flavor) as usize].as_rgba_linear()),
                DrawMode::Fill(FillOptions::default()),
                Transform::from_translation(emitter.path.first().unwrap().extend(1.0)),
            ))
            .insert(Pixie {
                path: emitter.path.clone(),
                path_index: 0,
            });

        emitter.remaining -= 1;
    }
}

fn move_pixies(
    mut commands: Commands,
    time: Res<Time>,
    mut score: ResMut<Score>,
    mut query: Query<(Entity, &mut Pixie, &mut Transform)>,
) {
    for (entity, mut pixie, mut transform) in query.iter_mut() {
        if pixie.path_index >= pixie.path.len() - 1 {
            commands.entity(entity).despawn_recursive();
            score.0 += 1;
            continue;
        }

        let next_waypoint = pixie.path[pixie.path_index + 1];

        let dist = transform.translation.truncate().distance(next_waypoint);

        let delta = time.delta_seconds();

        let speed = 60.0;
        let step = speed * delta;

        // five radians per second, clockwise
        transform.rotate(Quat::from_rotation_z(-5.0 * delta));

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

fn update_score(score: Res<Score>, mut q_score: Query<&mut Text, With<ScoreText>>) {
    if !score.is_changed() {
        return;
    }

    let mut text = q_score.single_mut().unwrap();
    text.sections[0].value = format!("SCORE {}", score.0);
}

fn spawn_obstacle(commands: &mut Commands, top_left: Vec2, bottom_right: Vec2) {
    let diff = bottom_right - top_left;
    let origin = (top_left + bottom_right) / 2.0;

    commands
        .spawn_bundle(GeometryBuilder::build_as(
            &shapes::Rectangle {
                width: diff.x.abs(),
                height: diff.y.abs(),
                ..Default::default()
            },
            ShapeColors::new(Color::GRAY),
            DrawMode::Fill(FillOptions::default()),
            Transform::from_translation(origin.extend(0.0)),
        ))
        .with_children(|parent| {
            parent
                .spawn()
                .insert(Collider::Segment((
                    Vec2::new(top_left.x, top_left.y),
                    Vec2::new(bottom_right.x, top_left.y),
                )))
                .insert(ColliderLayer(0));
            parent
                .spawn()
                .insert(Collider::Segment((
                    Vec2::new(bottom_right.x, top_left.y),
                    Vec2::new(bottom_right.x, bottom_right.y),
                )))
                .insert(ColliderLayer(0));
            parent
                .spawn()
                .insert(Collider::Segment((
                    Vec2::new(bottom_right.x, bottom_right.y),
                    Vec2::new(top_left.x, bottom_right.y),
                )))
                .insert(ColliderLayer(0));
            parent
                .spawn()
                .insert(Collider::Segment((
                    Vec2::new(top_left.x, bottom_right.y),
                    Vec2::new(top_left.x, top_left.y),
                )))
                .insert(ColliderLayer(0));
        });
}

fn spawn_terminus(
    commands: &mut Commands,
    graph: &mut ResMut<RoadGraph>,
    asset_server: &Res<AssetServer>,
    pos: Vec2,
    emits: HashSet<u32>,
    collects: HashSet<u32>,
) {
    let label_offset = 22.0;
    let label_spacing = 22.0;

    let ent = commands
        .spawn_bundle(GeometryBuilder::build_as(
            &shapes::Circle {
                radius: 5.5,
                center: pos.clone(),
            },
            ShapeColors::outlined(
                BACKGROUND_COLOR.as_rgba_linear(),
                FINISHED_ROAD_COLORS[0].as_rgba_linear(),
            ),
            DrawMode::Outlined {
                fill_options: FillOptions::default(),
                outline_options: StrokeOptions::default().with_line_width(2.0),
            },
            Transform::default(),
        ))
        .insert(Terminus {
            point: pos.clone(),
            emits: emits.clone(),
            collects: collects.clone(),
        })
        .with_children(|parent| {
            parent
                .spawn()
                .insert(Collider::Point(pos.clone()))
                .insert(ColliderLayer(1));

            let mut i = 0;

            for flavor in emits {
                let label_pos =
                    pos + Vec2::new(0.0, -1.0 * label_offset + -1.0 * i as f32 * label_spacing);

                parent.spawn_bundle(Text2dBundle {
                    text: Text::with_section(
                        "OUT",
                        TextStyle {
                            font: asset_server.load("fonts/CooperHewitt-Medium.ttf"),
                            font_size: 30.0,
                            color: PIXIE_COLORS[flavor as usize],
                        },
                        TextAlignment {
                            vertical: VerticalAlign::Center,
                            horizontal: HorizontalAlign::Center,
                        },
                    ),
                    transform: Transform::from_translation(label_pos.extend(0.0)),
                    ..Default::default()
                });

                i += 1;
            }

            for flavor in collects {
                let label_pos =
                    pos + Vec2::new(0.0, -1.0 * label_offset + -1.0 * i as f32 * label_spacing);

                parent.spawn_bundle(Text2dBundle {
                    text: Text::with_section(
                        "IN",
                        TextStyle {
                            font: asset_server.load("fonts/CooperHewitt-Medium.ttf"),
                            font_size: 30.0,
                            color: PIXIE_COLORS[flavor as usize],
                        },
                        TextAlignment {
                            vertical: VerticalAlign::Center,
                            horizontal: HorizontalAlign::Center,
                        },
                    ),
                    transform: Transform::from_translation(label_pos.extend(0.0)),
                    ..Default::default()
                });

                i += 1;
            }
        })
        .id();

    let node = graph.graph.add_node(ent);

    commands.entity(ent).insert(PointGraphNode(node));
}

fn update_cost(
    graph: Res<RoadGraph>,
    draw: Res<DrawingState>,
    q_segments: Query<(&RoadSegment, &Children)>,
    q_colliders: Query<&ColliderLayer>,
    mut q_cost: Query<&mut Text, With<CostText>>,
) {
    if !graph.is_changed() && !draw.is_changed() {
        return;
    }

    let mut cost = 0.0;

    for (segment, children) in q_segments.iter() {
        let child = match children.first() {
            Some(child) => child,
            None => continue,
        };

        let layer = match q_colliders.get(*child) {
            Ok(layer) => layer,
            Err(_) => continue,
        };

        let multiplier = if layer.0 > 1 { TUNNEL_MULTIPLIER } else { 1.0 };

        cost += (segment.points.0 - segment.points.1).length() * multiplier;
    }

    cost /= GRID_SIZE;
    let cost_round = cost.ceil();

    let mut potential_cost = 0.0;
    if draw.valid {
        for segment in draw.segments.iter() {
            let multiplier = if draw.layer > 1 {
                TUNNEL_MULTIPLIER
            } else {
                1.0
            };
            potential_cost += (segment.0 - segment.1).length() * multiplier;
        }
    }

    potential_cost /= GRID_SIZE;
    let potential_cost_round = (cost + potential_cost).ceil() - cost_round;

    for mut text in q_cost.iter_mut() {
        text.sections[0].value = format!("§{}", cost_round);
        if potential_cost_round > 0.0 {
            text.sections[1].value = format!("+{}", potential_cost_round);
            text.sections[1].style.color = FINISHED_ROAD_COLORS[draw.layer as usize - 1]
        } else {
            text.sections[1].value = "".to_string();
            text.sections[1].style.color = FINISHED_ROAD_COLORS[draw.layer as usize - 1]
        }
    }
}

fn setup(
    mut commands: Commands,
    mut graph: ResMut<RoadGraph>,
    asset_server: Res<AssetServer>,
    button_materials: Res<ButtonMaterials>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut camera = OrthographicCameraBundle::new_2d();
    camera.transform.translation.y -= 10.0;

    commands.spawn_bundle(camera).insert(MainCamera);
    commands.spawn_bundle(UiCameraBundle::default());

    for x in ((-25 * (GRID_SIZE as i32))..=25 * (GRID_SIZE as i32)).step_by(GRID_SIZE as usize) {
        for y in (-15 * (GRID_SIZE as i32)..=15 * (GRID_SIZE as i32)).step_by(GRID_SIZE as usize) {
            commands
                .spawn_bundle(GeometryBuilder::build_as(
                    &shapes::Circle {
                        radius: 2.5,
                        center: Vec2::new(x as f32, y as f32),
                    },
                    ShapeColors::new(GRID_COLOR.as_rgba_linear()),
                    DrawMode::Fill(FillOptions::default()),
                    Transform::default(),
                ))
                .insert(GridPoint);
        }
    }

    let points = [
        (
            snap_to_grid(Vec2::new(-500.0, -192.0), GRID_SIZE),
            vec![0],
            vec![],
        ),
        (
            snap_to_grid(Vec2::new(-500.0, -48.0), GRID_SIZE),
            vec![3],
            vec![],
        ),
        (
            snap_to_grid(Vec2::new(-500.0, 144.0), GRID_SIZE),
            vec![2],
            vec![],
        ),
        (
            snap_to_grid(Vec2::new(-500.0, 288.0), GRID_SIZE),
            vec![1],
            vec![],
        ),
        (
            snap_to_grid(Vec2::new(500.0, -192.0), GRID_SIZE),
            vec![],
            vec![1],
        ),
        (
            snap_to_grid(Vec2::new(500.0, -48.0), GRID_SIZE),
            vec![],
            vec![2],
        ),
        (
            snap_to_grid(Vec2::new(500.0, 144.0), GRID_SIZE),
            vec![],
            vec![3],
        ),
        (
            snap_to_grid(Vec2::new(500.0, 288.0), GRID_SIZE),
            vec![],
            vec![0],
        ),
    ];

    for (p, emits, collects) in points.iter().cloned() {
        spawn_terminus(
            &mut commands,
            &mut graph,
            &asset_server,
            p,
            emits.iter().cloned().collect::<HashSet<_>>(),
            collects.iter().cloned().collect::<HashSet<_>>(),
        );
    }

    spawn_obstacle(
        &mut commands,
        Vec2::new(-336.0, 240.0),
        Vec2::new(-288.0, 96.0),
    );
    spawn_obstacle(
        &mut commands,
        Vec2::new(-288.0, 240.0),
        Vec2::new(-192.0, 192.0),
    );
    spawn_obstacle(
        &mut commands,
        Vec2::new(288.0, 0.0),
        Vec2::new(332.0, -144.0),
    );
    spawn_obstacle(
        &mut commands,
        Vec2::new(192.0, -96.0),
        Vec2::new(288.0, -144.0),
    );

    println!(
        "{:?}",
        Dot::with_config(&graph.graph, &[Config::EdgeNoLabel])
    );

    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            material: materials.add(Color::NONE.into()),
            ..Default::default()
        })
        .with_children(|parent| {
            // bottom bar
            parent
                .spawn_bundle(NodeBundle {
                    style: Style {
                        padding: Rect::all(Val::Px(10.0)),
                        size: Size::new(Val::Percent(100.0), Val::Px(BOTTOM_BAR_HEIGHT)),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    material: materials.add(Color::rgb(0.09, 0.11, 0.13).into()),
                    ..Default::default()
                })
                .with_children(|parent| {
                    // left-aligned bar items
                    parent
                        .spawn_bundle(NodeBundle {
                            style: Style {
                                size: Size::new(Val::Auto, Val::Percent(100.0)),
                                flex_direction: FlexDirection::Row,
                                justify_content: JustifyContent::FlexEnd,
                                align_items: AlignItems::Center,
                                ..Default::default()
                            },
                            material: materials.add(Color::NONE.into()),
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            parent
                                .spawn_bundle(TextBundle {
                                    style: Style {
                                        margin: Rect {
                                            right: Val::Px(10.0),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    },
                                    text: Text::with_section(
                                        "0",
                                        TextStyle {
                                            font: asset_server
                                                .load("fonts/CooperHewitt-Medium.ttf"),
                                            font_size: 30.0,
                                            color: FINISHED_ROAD_COLORS[0],
                                        },
                                        Default::default(),
                                    ),
                                    ..Default::default()
                                })
                                .insert(ScoreText);
                            parent
                                .spawn_bundle(TextBundle {
                                    style: Style {
                                        margin: Rect {
                                            right: Val::Px(10.0),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    },
                                    text: Text {
                                        sections: vec![
                                            TextSection {
                                                value: "0".to_string(),
                                                style: TextStyle {
                                                    font: asset_server
                                                        .load("fonts/CooperHewitt-Medium.ttf"),
                                                    font_size: 30.0,
                                                    color: UI_WHITE_COLOR,
                                                },
                                            },
                                            TextSection {
                                                value: "".to_string(),
                                                style: TextStyle {
                                                    font: asset_server
                                                        .load("fonts/CooperHewitt-Medium.ttf"),
                                                    font_size: 30.0,
                                                    color: FINISHED_ROAD_COLORS[0],
                                                },
                                            },
                                        ],
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                })
                                .insert(CostText);
                        });

                    // right-aligned bar items
                    parent
                        .spawn_bundle(NodeBundle {
                            style: Style {
                                size: Size::new(Val::Auto, Val::Percent(100.0)),
                                flex_direction: FlexDirection::Row,
                                justify_content: JustifyContent::FlexStart,
                                align_items: AlignItems::Center,
                                ..Default::default()
                            },
                            material: materials.add(Color::NONE.into()),
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            parent
                                .spawn_bundle(ButtonBundle {
                                    style: Style {
                                        size: Size::new(Val::Px(250.0), Val::Percent(100.0)),
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
                                            "RELEASE THE PIXIES",
                                            TextStyle {
                                                font: asset_server
                                                    .load("fonts/CooperHewitt-Medium.ttf"),
                                                font_size: 30.0,
                                                color: Color::rgb(0.9, 0.9, 0.9),
                                            },
                                            Default::default(),
                                        ),
                                        ..Default::default()
                                    });
                                });
                        });
                });
        });
}
