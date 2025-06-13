use bevy::prelude::*;

use bevy_prototype_lyon::prelude::*;

use petgraph::{
    stable_graph::NodeIndex,
    visit::{DfsPostOrder, Walker},
};

use crate::{
    collision::{point_segment_collision, PointCollision},
    layer,
    sim::SimulationState,
    Collider, ColliderLayer, DrawingInteraction, DrawingMouseMovement, MouseSnappedPos, RoadGraph,
    RoadSegment, SegmentGraphNodes, SelectedTool, Tool,
};

pub struct NetRippingPlugin;
impl Plugin for NetRippingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NetRippingState>();
        app.add_systems(
            Update,
            (net_ripping_mouse_movement_system,).in_set(DrawingMouseMovement),
        );
        app.add_systems(
            Update,
            (net_ripping_mouse_click_system, draw_net_ripping_system).in_set(DrawingInteraction),
        );
    }
}

#[derive(Resource, Default)]
pub struct NetRippingState {
    pub entities: Vec<Entity>,
    pub nodes: Vec<NodeIndex>,
    pub segments: Vec<(Vec2, Vec2)>,
}

#[derive(Component)]
struct RippingLine;

fn net_ripping_mouse_movement_system(
    selected_tool: Res<SelectedTool>,
    mouse_snapped: Res<MouseSnappedPos>,
    mut ripping_state: ResMut<NetRippingState>,
    sim_state: Res<SimulationState>,
    graph: Res<RoadGraph>,
    q_colliders: Query<(&ChildOf, &Collider, &ColliderLayer)>,
    q_road_segments: Query<&RoadSegment>,
    q_segment_nodes: Query<&SegmentGraphNodes>,
) {
    if !matches!(selected_tool.0, Tool::NetRipping) {
        return;
    }

    if *sim_state != SimulationState::NotStarted {
        return;
    }

    if !mouse_snapped.is_changed() && !selected_tool.is_changed() {
        return;
    }

    ripping_state.entities = vec![];
    ripping_state.nodes = vec![];
    ripping_state.segments = vec![];

    let mut collisions: Vec<_> = q_colliders
        .iter()
        .filter_map(|(child_of, collider, layer)| match collider {
            Collider::Segment(segment) => {
                match point_segment_collision(mouse_snapped.0, segment.0, segment.1) {
                    PointCollision::None => None,
                    _ => {
                        if layer.0 == 0 {
                            None
                        } else {
                            Some((child_of.parent(), layer.0))
                        }
                    }
                }
            }
            _ => None,
        })
        .collect();

    if collisions.is_empty() {
        return;
    }

    // if there are multiple collisions, choose one on the top-most layer

    collisions.sort_by(|a, b| a.1.cmp(&b.1));

    if let Some((entity, _layer)) = collisions.first() {
        if let Ok(node) = q_segment_nodes.get(*entity) {
            let dfs = DfsPostOrder::new(&graph.graph, node.0);
            for index in dfs.iter(&graph.graph) {
                if let Some(net_entity) = graph.graph.node_weight(index) {
                    if let Ok(seg) = q_road_segments.get(*net_entity) {
                        ripping_state.entities.push(*net_entity);
                        ripping_state.nodes.push(index);
                        ripping_state.segments.push(seg.points);
                    }
                }
            }
        }
    }
}

fn net_ripping_mouse_click_system(
    mut commands: Commands,
    mouse_input: ResMut<ButtonInput<MouseButton>>,
    mut ripping_state: ResMut<NetRippingState>,
    sim_state: Res<SimulationState>,
    selected_tool: Res<SelectedTool>,
    mut graph: ResMut<RoadGraph>,
) {
    if !matches!(selected_tool.0, Tool::NetRipping) {
        return;
    }

    if *sim_state != SimulationState::NotStarted {
        return;
    }

    if mouse_input.just_pressed(MouseButton::Left) {
        for entity in ripping_state.entities.iter() {
            commands.entity(*entity).despawn_recursive();
        }
        for node in ripping_state.nodes.iter() {
            graph.graph.remove_node(*node);
        }

        ripping_state.entities = vec![];
        ripping_state.nodes = vec![];
        ripping_state.segments = vec![];
    }
}

fn draw_net_ripping_system(
    mut commands: Commands,
    ripping_state: Res<NetRippingState>,
    q_ripping: Query<Entity, With<RippingLine>>,
) {
    if !ripping_state.is_changed() {
        return;
    }

    for ent in q_ripping.iter() {
        commands.entity(ent).despawn();
    }

    for (a, b) in ripping_state.segments.iter() {
        commands.spawn((
            ShapeBuilder::with(&shapes::Line(*a, *b))
                .stroke((bevy::color::palettes::css::RED, 2.0))
                .build(),
            Transform::from_xyz(0.0, 0.0, layer::ROAD_OVERLAY),
            RippingLine,
        ));
    }
}
