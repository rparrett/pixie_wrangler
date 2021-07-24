use crate::collision::{point_segment_collision, segment_collision, SegmentCollision};

use bevy::{
    input::mouse::MouseButtonInput, input::ElementState::Released, prelude::*, utils::HashSet,
    window::CursorMoved,
};
use bevy_prototype_lyon::prelude::*;
use petgraph::algo::astar;
use petgraph::dot::{Config, Dot};
use petgraph::stable_graph::{NodeIndex, StableUnGraph};
use rand::seq::SliceRandom;

fn main() {
    let mut app = App::build();
    app.insert_resource(ClearColor(BACKGROUND_COLOR))
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(WindowDescriptor {
            title: String::from("Pixie Wrangler"),
            #[cfg(target_arch = "wasm32")]
            canvas: Some("#bevy".to_string()),
            ..Default::default()
        })
        .add_plugins(DefaultPlugins);
    #[cfg(target_arch = "wasm32")]
    app.add_plugin(bevy_webgl2::WebGL2Plugin);
    app.add_plugin(ShapePlugin);
    app.add_startup_system(setup.system());
    app.add_system(keyboard_system.system().before("mouse"));
    app.add_system(mouse_movement.system().label("mouse"));
    // TODO next two systems could be in a system set, maybe?
    app.add_system(
        not_drawing_mouse_movement
            .system()
            .label("not_drawing_mouse_movement")
            .after("mouse"),
    );
    app.add_system(
        drawing_mouse_movement
            .system()
            .label("drawing_mouse_movement")
            .after("not_drawing_mouse_movement"),
    );
    app.add_system(drawing_mouse_click.system().after("drawing_mouse_movement"));
    app.add_system(draw_mouse.system().after("drawing_mouse_movement"));
    app.add_system(button_system.system());
    app.add_system(pixie_button_system.system());
    app.add_system(reset_button_system.system().before("update_score"));

    app.add_system(move_pixies.system().label("pixies"));
    app.add_system(emit_pixies.system());
    app.add_system(update_score.system().label("update_score").after("pixies"));

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

struct PixieButton;
struct ResetButton;

#[derive(Default)]
struct Score(u32);
#[derive(Debug)]
struct RoadSegment {
    points: (Vec2, Vec2),
    layer: u32,
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
    stop: bool,
    segments: Vec<(Vec2, Vec2)>,
    adds: Vec<AddSegment>,
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
            stop: false,
            segments: vec![],
            adds: vec![],
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
    next_corner_angle: Option<f32>,
    max_speed: f32,
    current_speed: f32,
    current_speed_limit: f32,
    acceleration: f32,
    deceleration: f32,
}
impl Default for Pixie {
    fn default() -> Self {
        Self {
            path: vec![],
            path_index: 0,
            next_corner_angle: None,
            max_speed: 60.0,
            current_speed: 30.0,
            current_speed_limit: 60.0,
            acceleration: 25.0,
            deceleration: 50.0,
        }
    }
}

struct PixieEmitter {
    flavor: u32,
    path: Vec<Vec2>,
    remaining: u32,
    timer: Timer,
}

#[derive(Default)]
struct RoadGraph {
    graph: StableUnGraph<Entity, f32>,
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

#[derive(Clone, Debug)]
struct AddSegment {
    points: (Vec2, Vec2),
    connections: (Vec<SegmentConnection>, Vec<SegmentConnection>),
}
#[derive(Clone, Debug)]
enum SegmentConnection {
    Previous,
    Add(Entity),
    TryExtend(Entity),
    Split(Entity),
}

struct ButtonMaterials {
    normal: Handle<ColorMaterial>,
    hovered: Handle<ColorMaterial>,
    pressed: Handle<ColorMaterial>,
}
const GRID_SIZE: f32 = 48.0;
const BOTTOM_BAR_HEIGHT: f32 = 70.0;

const PIXIE_COLORS: [Color; 6] = [
    Color::AQUAMARINE,
    Color::PINK,
    Color::ORANGE,
    Color::PURPLE,
    Color::DARK_GREEN,
    Color::YELLOW,
];
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
) {
    for (interaction, mut material) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Clicked => {
                *material = button_materials.pressed.clone();
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

fn pixie_button_system(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<Button>, With<PixieButton>)>,
    graph: Res<RoadGraph>,
    q_terminuses: Query<(&Terminus, &PointGraphNode)>,
    q_road_chunks: Query<(&RoadSegment, &SegmentGraphNodes)>,
    mut commands: Commands,
) {
    for interaction in interaction_query.iter() {
        match *interaction {
            Interaction::Clicked => {
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

                                let with_ents = path.1.iter().filter_map(|node| {
                                    graph.graph.node_weight(*node).map(|ent| (node, ent))
                                });

                                for (node, ent) in with_ents {
                                    if let Ok((t, _)) = q_terminuses.get(*ent) {
                                        world_path.push(t.point);
                                    }
                                    if let Ok((s, n)) = q_road_chunks.get(*ent) {
                                        if n.0 == *node {
                                            world_path.push(s.points.0)
                                        } else if n.1 == *node {
                                            world_path.push(s.points.1);
                                        } else {
                                            info!(
                                                "pretty sure this shouldn't happen {:?}",
                                                s.points
                                            );
                                        }
                                    }
                                }

                                // would it be faster to avoid this duplication above?
                                world_path.dedup();

                                if world_path.is_empty() {
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

                if !ok || paths.is_empty() {
                    // TODO tell user we can't do that yet.
                    // or better yet, do this path calc upon connecting to a terminus
                    // and grey out the button if the requirements are not met.

                    continue;
                }

                for (flavor, world_path) in paths {
                    commands.spawn().insert(PixieEmitter {
                        flavor: *flavor,
                        path: world_path,
                        remaining: 50,
                        timer: Timer::from_seconds(0.4, true),
                    });
                }
            }
            _ => {}
        }
    }
}

fn reset_button_system(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<Button>, With<ResetButton>)>,
    mut graph: ResMut<RoadGraph>,
    mut score: ResMut<Score>,
    q_road_chunks: Query<Entity, With<RoadSegment>>,
    q_pixies: Query<Entity, With<Pixie>>,
    q_emitters: Query<Entity, With<PixieEmitter>>,
    q_terminuses: Query<Entity, With<Terminus>>,
    mut commands: Commands,
) {
    for interaction in interaction_query.iter() {
        match *interaction {
            Interaction::Clicked => {
                for chunk in q_road_chunks
                    .iter()
                    .chain(q_pixies.iter())
                    .chain(q_emitters.iter())
                {
                    commands.entity(chunk).despawn_recursive();
                }

                graph.graph.clear();

                // we just nuked the graph, but left the start/end points
                // so we need to overwrite their old nodes with new ones.
                for entity in q_terminuses.iter() {
                    let node = graph.graph.add_node(entity);
                    commands.entity(entity).insert(PointGraphNode(node));
                }

                score.0 = 0;
            }
            _ => {}
        }
    }
}

fn is_boring(in_flavors: &Vec<u32>, out_flavors: &Vec<u32>) -> bool {
    if in_flavors.windows(2).any(|v| v[0] == v[1]) {
        return true;
    }

    if out_flavors.windows(2).any(|v| v[0] == v[1]) {
        return true;
    }

    if in_flavors.first().unwrap() == out_flavors.first().unwrap() {
        return true;
    }

    if in_flavors.last().unwrap() == out_flavors.last().unwrap() {
        return true;
    }

    false
}

/// Given three points forming two line segments, return the angle formed
/// at the middle. Returns values in the range of 0.0..=180.0
///
/// ```text
/// a - b
///      \
///       c
/// ```
fn corner_angle(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    let seg_a = a - b;
    let seg_b = c - b;

    seg_a.perp_dot(seg_b).abs().atan2(seg_a.dot(seg_b))
}

fn snap_to_grid(position: Vec2, grid_size: f32) -> Vec2 {
    (position / grid_size).round() * grid_size
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
        let color = if draw.drawing && draw.valid {
            DRAWING_ROAD_COLORS[draw.layer as usize - 1]
        } else if !draw.drawing && draw.valid {
            UI_WHITE_COLOR
        } else {
            Color::RED
        };
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
                    &shapes::Line(*a, *b),
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
    } else if keyboard_input.pressed(KeyCode::Escape) {
        drawing_state.drawing = false;
        drawing_state.segments = vec![];
    }
}

#[allow(clippy::too_many_arguments)]
fn drawing_mouse_click(
    mut commands: Commands,
    mouse: Res<MouseState>,
    mut draw: ResMut<DrawingState>,
    mut graph: ResMut<RoadGraph>,
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    q_point_nodes: Query<&PointGraphNode>,
    q_segment_nodes: Query<&SegmentGraphNodes>,
    q_road_segments: Query<&RoadSegment>,
) {
    if mouse.window_position.y < BOTTOM_BAR_HEIGHT {
        return;
    }

    for event in mouse_button_input_events.iter() {
        if event.button == MouseButton::Left && event.state == Released {
            if !draw.drawing {
                if draw.valid {
                    draw.drawing = true;
                    draw.start = mouse.snapped;
                    draw.end = draw.start;
                }
            } else {
                if draw.end == draw.start {
                    draw.drawing = false;
                }

                if !draw.valid {
                    continue;
                }

                if draw.adds.is_empty() {
                    continue;
                }

                let mut previous_end: Option<NodeIndex> = None;

                for add in draw.adds.iter() {
                    info!("Add: {:?}", add);

                    // SegmentConnection::TryExtend is only valid if extending the
                    // target segment would not break any existing connections.

                    let valid_extension_a = add.connections.0.len() == 1
                        && add
                            .connections
                            .0
                            .iter()
                            .all(|c| matches!(c, SegmentConnection::TryExtend(_)));
                    let valid_extension_b = add.connections.1.len() == 1
                        && add
                            .connections
                            .1
                            .iter()
                            .all(|c| matches!(c, SegmentConnection::TryExtend(_)));

                    info!(
                        "valid_extension_a {:?} valid_extension_b {:?}",
                        valid_extension_a, valid_extension_b
                    );

                    let mut points = add.points;

                    info!("before: {:?}", points);

                    if valid_extension_a {
                        if let SegmentConnection::TryExtend(entity) =
                            add.connections.0.get(0).unwrap()
                        {
                            let segment = q_road_segments.get(*entity).unwrap();

                            if add.points.0 == segment.points.0 {
                                points.0 = segment.points.1;
                            } else {
                                points.0 = segment.points.0;
                            }
                        }
                    }
                    if valid_extension_b {
                        if let SegmentConnection::TryExtend(entity) =
                            add.connections.1.get(0).unwrap()
                        {
                            let segment = q_road_segments.get(*entity).unwrap();

                            if add.points.1 == segment.points.1 {
                                points.1 = segment.points.0;
                            } else {
                                points.1 = segment.points.1;
                            }
                        }
                    }

                    info!("after: {:?}", points);

                    let (start_node, end_node) =
                        spawn_road_segment(&mut commands, &mut graph, points, draw.layer);

                    for (node, is_start, connections, point) in [
                        (start_node, true, &add.connections.0, add.points.0),
                        (end_node, false, &add.connections.1, add.points.1),
                    ]
                    .iter()
                    {
                        for connection in connections.iter() {
                            match connection {
                                SegmentConnection::Add(entity) => {
                                    // seems like I should really just store whether the entity is a
                                    // segment or point in SegmentConnection::Add

                                    let s_nodes = q_segment_nodes.get(*entity);
                                    let segment = q_road_segments.get(*entity);
                                    let p_nodes = q_point_nodes.get(*entity);

                                    match (s_nodes, segment, p_nodes) {
                                        (Ok(segment_nodes), Ok(segment), Err(_)) => {
                                            if segment.points.0 == *point {
                                                graph.graph.add_edge(*node, segment_nodes.0, 0.0);
                                            }
                                            if segment.points.1 == *point {
                                                graph.graph.add_edge(*node, segment_nodes.1, 0.0);
                                            }
                                        }
                                        (Err(_), Err(_), Ok(p_nodes)) => {
                                            graph.graph.add_edge(*node, p_nodes.0, 0.0);
                                        }
                                        _ => {
                                            info!("encountered a thing that should not happen");
                                        }
                                    }
                                }
                                SegmentConnection::TryExtend(entity) => {
                                    let t_segment = q_road_segments.get(*entity);
                                    let t_nodes = q_segment_nodes.get(*entity);

                                    if let (Ok(t_nodes), Ok(t_segment)) = (t_nodes, t_segment) {
                                        if (*is_start && valid_extension_a)
                                            || (!is_start && valid_extension_b)
                                        {
                                            let neighbors = if t_segment.points.0 == *point {
                                                graph.graph.neighbors(t_nodes.1).collect::<Vec<_>>()
                                            } else {
                                                graph.graph.neighbors(t_nodes.0).collect::<Vec<_>>()
                                            };

                                            for neighbor in neighbors {
                                                graph.graph.add_edge(
                                                    neighbor,
                                                    if *is_start { start_node } else { end_node },
                                                    0.0,
                                                );
                                            }

                                            commands.entity(*entity).despawn_recursive();
                                            graph.graph.remove_node(t_nodes.0);
                                            graph.graph.remove_node(t_nodes.1);
                                        } else {
                                            // normal add
                                            if t_segment.points.0 == *point {
                                                graph.graph.add_edge(*node, t_nodes.0, 0.0);
                                            }
                                            if t_segment.points.1 == *point {
                                                graph.graph.add_edge(*node, t_nodes.1, 0.0);
                                            }
                                        }
                                    }
                                }
                                SegmentConnection::Previous => {
                                    if *is_start {
                                        if let Some(previous_end) = previous_end {
                                            graph.graph.add_edge(*node, previous_end, 0.0);
                                        }
                                    }
                                }
                                SegmentConnection::Split(entity) => {
                                    let s_nodes = q_segment_nodes.get(*entity).unwrap();
                                    let segment = q_road_segments.get(*entity).unwrap();

                                    // get neighboring NodeIndex from split line's start node
                                    let start_neighbors =
                                        graph.graph.neighbors(s_nodes.0).collect::<Vec<_>>();

                                    // get neighboring NodeIndex from split line's end node
                                    let end_neighbors =
                                        graph.graph.neighbors(s_nodes.1).collect::<Vec<_>>();

                                    // despawn split line
                                    commands.entity(*entity).despawn_recursive();

                                    // create a new segment on (entity start, this_point)
                                    let (start_node, end_node) = spawn_road_segment(
                                        &mut commands,
                                        &mut graph,
                                        (segment.points.0, *point),
                                        segment.layer,
                                    );

                                    // reconnect new segment to split line's old start node neighbors
                                    for neighbor in start_neighbors {
                                        graph.graph.add_edge(neighbor, start_node, 0.0);
                                    }
                                    graph.graph.add_edge(end_node, *node, 0.0);

                                    // create a new segment on (entity end, this_point)
                                    let (start_node, end_node) = spawn_road_segment(
                                        &mut commands,
                                        &mut graph,
                                        (*point, segment.points.1),
                                        segment.layer,
                                    );

                                    // reconnect new segment to split line's old end node neighbors
                                    for neighbor in end_neighbors {
                                        graph.graph.add_edge(end_node, neighbor, 0.0);
                                    }
                                    graph.graph.add_edge(*node, start_node, 0.0);

                                    // remove all graph edges and nodes associated with the split line
                                    graph.graph.remove_node(s_nodes.0);
                                    graph.graph.remove_node(s_nodes.1);
                                }
                            };
                        }
                    }

                    previous_end = Some(end_node);
                }

                if draw.stop {
                    draw.drawing = false;
                    draw.stop = false;
                }

                draw.start = draw.end;
                draw.adds = vec![];
                draw.segments = vec![];

                println!(
                    "{:?}",
                    Dot::with_config(&graph.graph, &[Config::EdgeNoLabel, Config::NodeIndexLabel])
                );
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

        mouse.window_position = event.position;
    }
}

fn not_drawing_mouse_movement(
    mut draw: ResMut<DrawingState>,
    mouse: Res<MouseState>,
    q_colliders: Query<(&Parent, &Collider, &ColliderLayer)>,
) {
    if draw.drawing {
        return;
    }

    let bad = q_colliders
        .iter()
        .any(|(_parent, collider, layer)| match collider {
            Collider::Segment(segment) => {
                match point_segment_collision(mouse.snapped, segment.0, segment.1) {
                    SegmentCollision::None => false,
                    _ => layer.0 == 0,
                }
            }
            _ => false,
        });

    if bad {
        draw.valid = false;
    } else {
        draw.valid = true;
    }
}

fn drawing_mouse_movement(
    mut draw: ResMut<DrawingState>,
    mouse: Res<MouseState>,
    q_colliders: Query<(&Parent, &Collider, &ColliderLayer)>,
) {
    if !draw.drawing {
        return;
    }

    if mouse.snapped == draw.end && draw.layer == draw.prev_layer {
        return;
    }

    info!("{:?}", mouse.snapped);

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
        draw.adds = vec![];
        draw.valid = true;
    }

    let possible = possible_lines(draw.start, mouse.snapped, draw.axis_preference);

    // groan
    let mut filtered_adds = vec![];
    let mut filtered_segments = vec![];
    let mut filtered_stops = vec![];

    for possibility in possible.iter() {
        let mut adds = vec![];
        let mut ok = true;
        let mut stop = false;

        for (segment_i, (a, b)) in possibility.iter().enumerate() {
            let mut connections = (vec![], vec![]);

            let mut split_layers: (HashSet<u32>, HashSet<u32>) =
                (HashSet::default(), HashSet::default());

            if segment_i == 1 {
                connections.0.push(SegmentConnection::Previous);
            }

            for (parent, collider, layer) in q_colliders.iter() {
                match collider {
                    Collider::Segment(s) => {
                        let collision = segment_collision(s.0, s.1, *a, *b);

                        match collision {
                            SegmentCollision::Intersecting => {
                                if layer.0 == draw.layer || layer.0 == 0 {
                                    ok = false;
                                    break;
                                }
                            }
                            SegmentCollision::Overlapping => {
                                ok = false;
                                break;
                            }
                            SegmentCollision::Touching => {
                                // "Touching" collisions are allowed only if they are the
                                // start or end of the line we are currently drawing.
                                //
                                // Ideally, segment_collision would return the intersection
                                // point(s) and we could just check that.

                                if layer.0 == 0 {
                                    ok = false;
                                    break;
                                }

                                let start_touching = matches!(
                                    point_segment_collision(draw.start, s.0, s.1),
                                    SegmentCollision::Touching
                                );
                                let end_touching = matches!(
                                    point_segment_collision(draw.end, s.0, s.1),
                                    SegmentCollision::Touching
                                );

                                if !start_touching && !end_touching {
                                    ok = false;
                                    break;
                                }

                                // account for the specific scenario where two lines on
                                // different layers are being "split" at the point where
                                // they would intersect. do this by keeping track of the
                                // layers that have been split so far, and calling foul
                                // if we're about to split another.

                                if start_touching
                                    && !split_layers.0.contains(&layer.0)
                                    && split_layers.0.len() >= 1
                                {
                                    ok = false;
                                    break;
                                }

                                if end_touching
                                    && !split_layers.1.contains(&layer.0)
                                    && split_layers.1.len() >= 1
                                {
                                    ok = false;
                                    break;
                                }

                                if start_touching {
                                    connections.0.push(SegmentConnection::Split(parent.0));
                                    split_layers.0.insert(layer.0);
                                }
                                if end_touching {
                                    connections.1.push(SegmentConnection::Split(parent.0));
                                    split_layers.1.insert(layer.0);
                                }
                            }
                            SegmentCollision::Connecting | SegmentCollision::ConnectingParallel => {
                                // "Connecting" collisions are allowed only if they are the
                                // start or end of the line we are currently drawing.
                                //
                                // Ideally, segment_collision would return the intersection
                                // point(s) and we could just check that.

                                if layer.0 == 0 {
                                    ok = false;
                                    break;
                                }

                                let start_touching = matches!(
                                    point_segment_collision(draw.start, s.0, s.1),
                                    SegmentCollision::Connecting
                                );
                                let end_touching = matches!(
                                    point_segment_collision(draw.end, s.0, s.1),
                                    SegmentCollision::Connecting
                                );

                                if !start_touching && !end_touching {
                                    ok = false;
                                    break;
                                }

                                if (draw.start == *a && start_touching)
                                    || (draw.end == *a && end_touching)
                                {
                                    if matches!(collision, SegmentCollision::ConnectingParallel)
                                        && layer.0 == draw.layer
                                    {
                                        connections.0.push(SegmentConnection::TryExtend(parent.0))
                                    } else {
                                        connections.0.push(SegmentConnection::Add(parent.0))
                                    }
                                }
                                if (draw.start == *b && start_touching)
                                    || (draw.end == *b && end_touching)
                                {
                                    if matches!(collision, SegmentCollision::ConnectingParallel)
                                        && layer.0 == draw.layer
                                    {
                                        connections.1.push(SegmentConnection::TryExtend(parent.0))
                                    } else {
                                        connections.1.push(SegmentConnection::Add(parent.0))
                                    }
                                }
                            }
                            SegmentCollision::None => {}
                        }
                    }
                    Collider::Point(p) => match point_segment_collision(*p, *a, *b) {
                        SegmentCollision::Connecting => {
                            // don't allow the midpoint of the line to connect to a
                            // terminus

                            if *p != draw.start && *p != draw.end {
                                ok = false;
                                break;
                            }

                            if *p == draw.end {
                                stop = true;
                            }

                            if *a == *p {
                                connections.0.push(SegmentConnection::Add(parent.0));
                            }
                            if *b == *p {
                                connections.1.push(SegmentConnection::Add(parent.0));
                            }
                        }
                        SegmentCollision::None => {}
                        _ => {
                            ok = false;
                            break;
                        }
                    },
                }
            }

            if !ok {
                break;
            }

            adds.push(AddSegment {
                points: (*a, *b),
                connections,
            })
        }

        if ok {
            filtered_adds.push(adds);
            filtered_segments.push(possibility.clone());
            filtered_stops.push(stop);
        }
    }

    if let Some(segments) = filtered_segments.get(0) {
        draw.segments = segments.clone();
        draw.adds = filtered_adds.first().cloned().unwrap();
        draw.stop = filtered_stops.first().cloned().unwrap();
        draw.valid = true;
    } else if let Some(segments) = possible.get(0) {
        draw.segments = segments.clone();
        draw.adds = vec![];
        draw.valid = false;
    } else {
        draw.segments = vec![];
        draw.adds = vec![];
        draw.valid = false;
    }
}

fn emit_pixies(time: Res<Time>, mut q_emitters: Query<&mut PixieEmitter>, mut commands: Commands) {
    for mut emitter in q_emitters.iter_mut() {
        if emitter.remaining == 0 {
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
                ..Default::default()
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

        if dist < GRID_SIZE {
            if let Some(angle) = pixie.next_corner_angle {
                if angle <= 45.0 {
                    pixie.current_speed_limit = 10.0;
                } else if angle <= 90.0 {
                    pixie.current_speed_limit = 30.0;
                }
            } else {
                pixie.current_speed_limit = pixie.max_speed;
            }
        } else {
            pixie.current_speed_limit = pixie.max_speed;
        }

        let delta = time.delta_seconds();

        let speed_diff = pixie.current_speed_limit - pixie.current_speed;
        if speed_diff > f32::EPSILON {
            pixie.current_speed += pixie.acceleration * delta;
            if pixie.current_speed > pixie.current_speed_limit {
                pixie.current_speed = pixie.current_speed_limit;
            }
        } else if speed_diff < f32::EPSILON {
            pixie.current_speed -= pixie.deceleration * delta;
            if pixie.current_speed < pixie.current_speed_limit {
                pixie.current_speed = pixie.current_speed_limit;
            }
        }

        let step = pixie.current_speed * delta;

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

        if !pixie.next_corner_angle.is_some() || step > dist {
            if let (Some(current_waypoint), Some(next_waypoint), Some(next_next_waypoint)) = (
                pixie.path.get(pixie.path_index),
                pixie.path.get(pixie.path_index + 1),
                pixie.path.get(pixie.path_index + 2),
            ) {
                pixie.next_corner_angle = Some(
                    corner_angle(*current_waypoint, *next_waypoint, *next_next_waypoint)
                        .to_degrees(),
                );
            } else {
                pixie.next_corner_angle = Some(180.0);
            }
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

fn spawn_road_segment(
    commands: &mut Commands,
    graph: &mut RoadGraph,
    points: (Vec2, Vec2),
    layer: u32,
) -> (NodeIndex, NodeIndex) {
    let color = FINISHED_ROAD_COLORS[layer as usize - 1];
    let ent = commands
        .spawn_bundle(GeometryBuilder::build_as(
            &shapes::Line(points.0, points.1),
            ShapeColors::new(color.as_rgba_linear()),
            DrawMode::Stroke(StrokeOptions::default().with_line_width(2.0)),
            Transform::default(),
        ))
        .insert(RoadSegment { points, layer })
        .with_children(|parent| {
            parent
                .spawn()
                .insert(Collider::Segment(points))
                .insert(ColliderLayer(layer));
        })
        .id();

    let start_node = graph.graph.add_node(ent);
    let end_node = graph.graph.add_node(ent);

    graph
        .graph
        .add_edge(start_node, end_node, (points.0 - points.1).length());
    commands
        .entity(ent)
        .insert(SegmentGraphNodes(start_node, end_node));

    (start_node, end_node)
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
                center: pos,
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
            point: pos,
            emits: emits.clone(),
            collects: collects.clone(),
        })
        .with_children(|parent| {
            parent
                .spawn()
                .insert(Collider::Point(pos))
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
        } else {
            text.sections[1].value = "".to_string();
        }
        text.sections[1].style.color = FINISHED_ROAD_COLORS[draw.layer as usize - 1]
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

    let mut points = [
        // left
        (
            snap_to_grid(Vec2::new(-576.0, -192.0), GRID_SIZE),
            vec![],
            vec![],
        ),
        (
            snap_to_grid(Vec2::new(-576.0, -96.0), GRID_SIZE),
            vec![],
            vec![],
        ),
        (
            snap_to_grid(Vec2::new(-576.0, 0.0), GRID_SIZE),
            vec![],
            vec![],
        ),
        (
            snap_to_grid(Vec2::new(-576.0, 96.0), GRID_SIZE),
            vec![],
            vec![],
        ),
        (
            snap_to_grid(Vec2::new(-576.0, 192.0), GRID_SIZE),
            vec![],
            vec![],
        ),
        (
            snap_to_grid(Vec2::new(-576.0, 288.0), GRID_SIZE),
            vec![],
            vec![],
        ),
        // top
        (
            snap_to_grid(Vec2::new(-192.0, 336.0), GRID_SIZE),
            vec![],
            vec![],
        ),
        (
            snap_to_grid(Vec2::new(0.0, 336.0), GRID_SIZE),
            vec![],
            vec![],
        ),
        (
            snap_to_grid(Vec2::new(192.0, 336.0), GRID_SIZE),
            vec![],
            vec![],
        ),
        // bottom
        (
            snap_to_grid(Vec2::new(-192.0, -240.0), GRID_SIZE),
            vec![],
            vec![],
        ),
        (
            snap_to_grid(Vec2::new(0.0, -240.0), GRID_SIZE),
            vec![],
            vec![],
        ),
        (
            snap_to_grid(Vec2::new(192.0, -240.0), GRID_SIZE),
            vec![],
            vec![],
        ),
        // right
        (
            snap_to_grid(Vec2::new(576.0, -192.0), GRID_SIZE),
            vec![],
            vec![],
        ),
        (
            snap_to_grid(Vec2::new(576.0, -96.0), GRID_SIZE),
            vec![],
            vec![],
        ),
        (
            snap_to_grid(Vec2::new(576.0, 0.0), GRID_SIZE),
            vec![],
            vec![],
        ),
        (
            snap_to_grid(Vec2::new(576.0, 96.0), GRID_SIZE),
            vec![],
            vec![],
        ),
        (
            snap_to_grid(Vec2::new(576.0, 192.0), GRID_SIZE),
            vec![],
            vec![],
        ),
        (
            snap_to_grid(Vec2::new(576.0, 288.0), GRID_SIZE),
            vec![],
            vec![],
        ),
    ];

    let mut rng = rand::thread_rng();
    let mut in_flavors = vec![0, 1, 2, 3, 4, 5];
    let mut out_flavors = vec![0, 1, 2, 3, 4, 5];

    let multiples: Vec<u32> = in_flavors.choose_multiple(&mut rng, 3).cloned().collect();
    in_flavors.extend(multiples.iter());
    out_flavors.extend(multiples);

    while is_boring(&in_flavors, &out_flavors) {
        info!("shuffling a boring level");
        in_flavors.shuffle(&mut rng);
        out_flavors.shuffle(&mut rng);
    }

    for (i, flavor) in in_flavors.iter().enumerate() {
        points[i].1 = vec![*flavor];
    }

    for (i, flavor) in out_flavors.iter().enumerate() {
        points[i + points.len() / 2].2 = vec![*flavor];
    }

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
        Vec2::new(336.0, -144.0),
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
                                        size: Size::new(Val::Px(150.0), Val::Percent(100.0)),
                                        // horizontally center child text
                                        justify_content: JustifyContent::Center,
                                        // vertically center child text
                                        align_items: AlignItems::Center,
                                        ..Default::default()
                                    },
                                    material: button_materials.normal.clone(),
                                    ..Default::default()
                                })
                                .insert(ResetButton)
                                .with_children(|parent| {
                                    parent.spawn_bundle(TextBundle {
                                        text: Text::with_section(
                                            "RESET",
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
                            parent
                                .spawn_bundle(ButtonBundle {
                                    style: Style {
                                        size: Size::new(Val::Px(250.0), Val::Percent(100.0)),
                                        // horizontally center child text
                                        justify_content: JustifyContent::Center,
                                        // vertically center child text
                                        align_items: AlignItems::Center,
                                        margin: Rect {
                                            left: Val::Px(10.0),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    },
                                    material: button_materials.normal.clone(),
                                    ..Default::default()
                                })
                                .insert(PixieButton)
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
