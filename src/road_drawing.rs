use bevy::{prelude::*, utils::HashSet};

use crate::{
    collision::{point_segment_collision, segment_collision, PointCollision, SegmentCollision},
    lines::{possible_lines, Axis},
    sim::SimulationState,
    spawn_road_segment, Collider, ColliderLayer, DrawingInteraction, DrawingMouseMovement,
    MousePos, MouseSnappedPos, PointGraphNode, RoadGraph, RoadSegment, SegmentGraphNodes,
    SelectedTool, Tool, BOTTOM_BAR_HEIGHT,
};

use petgraph::{
    dot::{Config, Dot},
    stable_graph::NodeIndex,
};

pub struct RoadDrawingPlugin;
impl Plugin for RoadDrawingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RoadDrawingState>();
        app.add_systems(
            Update,
            (
                not_drawing_mouse_movement_system,
                drawing_mouse_movement_system,
            )
                .in_set(DrawingMouseMovement),
        );
        app.add_systems(
            Update,
            (drawing_mouse_click_system).in_set(DrawingInteraction),
        );
    }
}

#[derive(Resource)]
pub struct RoadDrawingState {
    pub drawing: bool,
    start: Vec2,
    end: Vec2,
    pub valid: bool,
    stop: bool,
    pub segments: Vec<(Vec2, Vec2)>,
    adds: Vec<AddSegment>,
    axis_preference: Option<Axis>,
    pub layer: u32,
    prev_layer: u32,
}
impl Default for RoadDrawingState {
    fn default() -> Self {
        Self {
            drawing: false,
            start: Vec2::ZERO,
            end: Vec2::ZERO,
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

fn drawing_mouse_click_system(
    mut commands: Commands,
    mouse_input: ResMut<ButtonInput<MouseButton>>,
    mouse: Res<MousePos>,
    mouse_snapped: Res<MouseSnappedPos>,
    selected_tool: ResMut<SelectedTool>,
    mut road_state: ResMut<RoadDrawingState>,
    sim_state: Res<SimulationState>,
    mut graph: ResMut<RoadGraph>,
    q_point_nodes: Query<&PointGraphNode>,
    q_segment_nodes: Query<&SegmentGraphNodes>,
    q_road_segments: Query<&RoadSegment>,
    q_window: Query<&Window>,
) {
    let Ok(window) = q_window.get_single() else {
        return;
    };

    if mouse.window.y > window.resolution.height() - BOTTOM_BAR_HEIGHT {
        return;
    }

    if !matches!(selected_tool.0, Tool::LineDrawing) {
        return;
    }

    if *sim_state != SimulationState::NotStarted {
        return;
    }

    if !mouse_input.just_pressed(MouseButton::Left) {
        return;
    }

    if !road_state.drawing {
        if road_state.valid {
            road_state.drawing = true;
            road_state.start = mouse_snapped.0;
            road_state.end = road_state.start;
        }
        return;
    }

    if road_state.end == road_state.start {
        road_state.drawing = false;
        return;
    }

    if !road_state.valid {
        return;
    }

    if road_state.adds.is_empty() {
        return;
    }

    let mut previous_end: Option<NodeIndex> = None;

    for add in road_state.adds.iter() {
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

        let mut points = add.points;

        if valid_extension_a {
            if let SegmentConnection::TryExtend(entity) = add.connections.0.first().unwrap() {
                let segment = q_road_segments.get(*entity).unwrap();

                if add.points.0 == segment.points.0 {
                    points.0 = segment.points.1;
                } else {
                    points.0 = segment.points.0;
                }
            }
        }
        if valid_extension_b {
            if let SegmentConnection::TryExtend(entity) = add.connections.1.first().unwrap() {
                let segment = q_road_segments.get(*entity).unwrap();

                if add.points.1 == segment.points.1 {
                    points.1 = segment.points.0;
                } else {
                    points.1 = segment.points.1;
                }
            }
        }

        let (_, start_node, end_node) = spawn_road_segment(
            &mut commands,
            &mut graph,
            RoadSegment {
                points,
                layer: road_state.layer,
            },
        );

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
                                warn!("Encountered a thing that should not happen while adding a connection.");
                            }
                        }
                    }
                    SegmentConnection::TryExtend(entity) => {
                        let t_segment = q_road_segments.get(*entity);
                        let t_nodes = q_segment_nodes.get(*entity);

                        if let (Ok(t_nodes), Ok(t_segment)) = (t_nodes, t_segment) {
                            if (*is_start && valid_extension_a) || (!is_start && valid_extension_b)
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
                        let start_neighbors = graph.graph.neighbors(s_nodes.0).collect::<Vec<_>>();

                        // get neighboring NodeIndex from split line's end node
                        let end_neighbors = graph.graph.neighbors(s_nodes.1).collect::<Vec<_>>();

                        // despawn split line
                        commands.entity(*entity).despawn_recursive();

                        // create a new segment on (entity start, this_point)
                        let (_, start_node_a, end_node_a) = spawn_road_segment(
                            &mut commands,
                            &mut graph,
                            RoadSegment {
                                points: (segment.points.0, *point),
                                layer: segment.layer,
                            },
                        );

                        // reconnect new segment to split line's old start node neighbors
                        for neighbor in start_neighbors {
                            graph.graph.add_edge(neighbor, start_node_a, 0.0);
                        }
                        graph.graph.add_edge(end_node_a, *node, 0.0);

                        // create a new segment on (entity end, this_point)
                        let (_, start_node_b, end_node_b) = spawn_road_segment(
                            &mut commands,
                            &mut graph,
                            RoadSegment {
                                points: (*point, segment.points.1),
                                layer: segment.layer,
                            },
                        );

                        // reconnect new segment to split line's old end node neighbors
                        for neighbor in end_neighbors {
                            graph.graph.add_edge(end_node_b, neighbor, 0.0);
                        }
                        graph.graph.add_edge(*node, start_node_b, 0.0);

                        // connect the two new segments together
                        graph.graph.add_edge(end_node_a, start_node_b, 0.0);

                        // remove all graph edges and nodes associated with the split line
                        graph.graph.remove_node(s_nodes.0);
                        graph.graph.remove_node(s_nodes.1);
                    }
                };
            }
        }

        previous_end = Some(end_node);
    }

    if road_state.stop {
        road_state.drawing = false;
        road_state.stop = false;
    }

    road_state.start = road_state.end;
    road_state.adds = vec![];
    road_state.segments = vec![];

    println!(
        "{:?}",
        Dot::with_config(&graph.graph, &[Config::EdgeNoLabel, Config::NodeIndexLabel])
    );
}

fn not_drawing_mouse_movement_system(
    mut road_state: ResMut<RoadDrawingState>,
    selected_tool: Res<SelectedTool>,
    mouse_snapped: Res<MouseSnappedPos>,
    q_colliders: Query<(&Parent, &Collider, &ColliderLayer)>,
) {
    if !matches!(selected_tool.0, Tool::LineDrawing) {
        return;
    }

    if !mouse_snapped.is_changed() {
        return;
    }

    if road_state.drawing {
        return;
    }

    let bad = q_colliders
        .iter()
        .any(|(_parent, collider, layer)| match collider {
            Collider::Segment(segment) => {
                match point_segment_collision(mouse_snapped.0, segment.0, segment.1) {
                    PointCollision::None => false,
                    _ => layer.0 == 0,
                }
            }
            _ => false,
        });

    if bad && road_state.valid {
        road_state.valid = false;
    } else if !bad && !road_state.valid {
        road_state.valid = true;
    }
}

fn drawing_mouse_movement_system(
    mut road_state: ResMut<RoadDrawingState>,
    sim_state: Res<SimulationState>,
    mouse_snapped: Res<MouseSnappedPos>,
    q_colliders: Query<(&Parent, &Collider, &ColliderLayer)>,
) {
    if !road_state.drawing {
        return;
    }

    if *sim_state != SimulationState::NotStarted {
        return;
    }

    if !mouse_snapped.is_changed() && !road_state.is_changed() {
        return;
    }

    if mouse_snapped.0 == road_state.end && road_state.layer == road_state.prev_layer {
        return;
    }

    road_state.end = mouse_snapped.0;
    road_state.prev_layer = road_state.layer;

    // line drawing can be coerced to follow one axis or another by moving the mouse to a
    // position that is a straight line from the starting point in that axis.

    if road_state.start.x == mouse_snapped.0.x {
        road_state.axis_preference = Some(Axis::Y);
    } else if road_state.start.y == mouse_snapped.0.y {
        road_state.axis_preference = Some(Axis::X);
    }

    if mouse_snapped.0 == road_state.start {
        road_state.segments = vec![];
        road_state.adds = vec![];
        road_state.valid = true;
    }

    let possible = possible_lines(
        road_state.start,
        mouse_snapped.0,
        road_state.axis_preference,
    );

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
                                if layer.0 == road_state.layer || layer.0 == 0 {
                                    ok = false;
                                    break;
                                }
                            }
                            SegmentCollision::Overlapping => {
                                ok = false;
                                break;
                            }
                            SegmentCollision::Touching(intersection_point) => {
                                // "Touching" collisions are allowed only if they are the
                                // start or end of the line we are currently drawing.

                                if layer.0 == 0 {
                                    ok = false;
                                    break;
                                }

                                let start_touching = intersection_point == road_state.start;
                                let end_touching = intersection_point == road_state.end;

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
                                    && !split_layers.0.is_empty()
                                {
                                    ok = false;
                                    break;
                                }

                                if end_touching
                                    && !split_layers.1.contains(&layer.0)
                                    && !split_layers.1.is_empty()
                                {
                                    ok = false;
                                    break;
                                }

                                if start_touching {
                                    connections.0.push(SegmentConnection::Split(parent.get()));
                                    split_layers.0.insert(layer.0);
                                }
                                if end_touching {
                                    connections.1.push(SegmentConnection::Split(parent.get()));
                                    split_layers.1.insert(layer.0);
                                }
                            }
                            SegmentCollision::Connecting(intersection_point)
                            | SegmentCollision::ConnectingParallel(intersection_point) => {
                                // "Connecting" collisions are allowed only if they are the
                                // start or end of the line we are currently drawing.

                                if layer.0 == 0 {
                                    ok = false;
                                    break;
                                }

                                let start_touching = intersection_point == road_state.start;
                                let end_touching = intersection_point == road_state.end;

                                if !start_touching && !end_touching {
                                    ok = false;
                                    break;
                                }

                                if (road_state.start == *a && start_touching)
                                    || (road_state.end == *a && end_touching)
                                {
                                    if matches!(collision, SegmentCollision::ConnectingParallel(_))
                                        && layer.0 == road_state.layer
                                    {
                                        connections
                                            .0
                                            .push(SegmentConnection::TryExtend(parent.get()));
                                    } else {
                                        connections.0.push(SegmentConnection::Add(parent.get()));
                                    }
                                }
                                if (road_state.start == *b && start_touching)
                                    || (road_state.end == *b && end_touching)
                                {
                                    if matches!(collision, SegmentCollision::ConnectingParallel(_))
                                        && layer.0 == road_state.layer
                                    {
                                        connections
                                            .1
                                            .push(SegmentConnection::TryExtend(parent.get()));
                                    } else {
                                        connections.1.push(SegmentConnection::Add(parent.get()));
                                    }
                                }
                            }
                            SegmentCollision::None => {}
                        }
                    }
                    Collider::Point(p) => match point_segment_collision(*p, *a, *b) {
                        PointCollision::Middle => {
                            // don't allow the midpoint of the line to connect to a
                            // terminus
                            ok = false;
                            break;
                        }
                        PointCollision::End => {
                            if *p != road_state.start && *p != road_state.end {
                                ok = false;
                                break;
                            }

                            if *p == road_state.end {
                                stop = true;
                            }

                            if *a == *p {
                                connections.0.push(SegmentConnection::Add(parent.get()));
                            }
                            if *b == *p {
                                connections.1.push(SegmentConnection::Add(parent.get()));
                            }
                        }
                        PointCollision::None => {}
                    },
                }
            }

            if !ok {
                break;
            }

            adds.push(AddSegment {
                points: (*a, *b),
                connections,
            });
        }

        if ok {
            filtered_adds.push(adds);
            filtered_segments.push(possibility.clone());
            filtered_stops.push(stop);
        }
    }

    if let Some(segments) = filtered_segments.first() {
        road_state.segments.clone_from(segments);
        road_state.adds = filtered_adds.first().cloned().unwrap();
        road_state.stop = filtered_stops.first().cloned().unwrap();
        road_state.valid = true;
    } else if let Some(segments) = possible.first() {
        road_state.segments.clone_from(segments);
        road_state.adds = vec![];
        road_state.valid = false;
    } else {
        road_state.segments = vec![];
        road_state.adds = vec![];
        road_state.valid = false;
    }
}
