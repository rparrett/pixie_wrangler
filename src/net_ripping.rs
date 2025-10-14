use bevy::{ecs::entity::EntityHashSet, prelude::*};

use bevy_prototype_lyon::prelude::*;

use petgraph::{
    stable_graph::NodeIndex,
    visit::{DfsPostOrder, Walker},
};

use crate::{
    collision::{point_segment_collision, PointCollision},
    layer,
    sim::SimulationState,
    Collider, ColliderLayer, DrawingInteraction, DrawingMouseMovement, GameState, MouseSnappedPos,
    RoadGraph, RoadSegment, SegmentGraphNodes, SelectedTool, Tool,
};

pub struct NetRippingPlugin;
impl Plugin for NetRippingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NetRippingState>();
        app.add_systems(
            Update,
            net_ripping_mouse_movement_system.in_set(DrawingMouseMovement),
        );
        app.add_systems(
            Update,
            (net_ripping_mouse_click_system, draw_net_ripping_system).in_set(DrawingInteraction),
        );
    }
}

#[derive(Resource, Default)]
pub struct NetRippingState {
    pub entities: EntityHashSet,
    pub nodes: Vec<NodeIndex>,
    pub segments: Vec<(Vec2, Vec2)>,
}

impl NetRippingState {
    pub fn reset(&mut self) {
        self.entities.clear();
        self.nodes.clear();
        self.segments.clear();
    }
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

    if !mouse_snapped.is_changed() && !selected_tool.is_changed() && !ripping_state.is_changed() {
        return;
    }

    ripping_state.reset();

    // Find the top-most (lowest layer value) collision with a road segment
    let Some((entity, _layer)) = q_colliders
        .iter()
        .filter_map(|(child_of, collider, layer)| {
            let Collider::Segment(segment_points) = collider else {
                return None;
            };

            if q_road_segments.get(child_of.parent()).is_err() {
                return None;
            }

            if matches!(
                point_segment_collision(mouse_snapped.0, segment_points.0, segment_points.1),
                PointCollision::None
            ) {
                return None;
            };

            Some((child_of.parent(), layer.0))
        })
        .min_by_key(|(_, layer)| *layer)
    else {
        return;
    };

    let Ok(node) = q_segment_nodes.get(entity) else {
        warn!("Failed to look up SegmentNodes for {entity}");
        return;
    };

    let dfs = DfsPostOrder::new(&graph.graph, node.0);

    for index in dfs.iter(&graph.graph) {
        if let Some(net_entity) = graph.graph.node_weight(index) {
            if let Ok(seg) = q_road_segments.get(*net_entity) {
                if ripping_state.entities.insert(*net_entity) {
                    ripping_state.segments.push(seg.points);
                }

                ripping_state.nodes.push(index);
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
            commands.entity(*entity).despawn();
        }
        for node in ripping_state.nodes.iter() {
            graph.graph.remove_node(*node);
        }

        ripping_state.reset();
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
            DespawnOnExit(GameState::Playing),
        ));
    }
}
