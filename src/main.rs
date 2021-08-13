#![allow(clippy::too_many_arguments, clippy::type_complexity)]
use crate::collision::{point_segment_collision, segment_collision, SegmentCollision};
use crate::debug::DebugLinesPlugin;
use crate::level::Level;
use crate::level_select::LevelSelectPlugin;
use crate::lines::{possible_lines, Axis};
use crate::loading::LoadingPlugin;
use crate::pixie::{Pixie, PixieEmitter, PixieFlavor, PixiePlugin, PIXIE_COLORS};
use crate::radio_button::{
    RadioButton, RadioButtonGroup, RadioButtonGroupRelation, RadioButtonPlugin,
};
use crate::save::SavePlugin;

use bevy::utils::{Duration, HashMap};
//use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::{prelude::*, utils::HashSet, window::CursorMoved};

use bevy_asset_ron::*;
use bevy_prototype_lyon::prelude::*;
use itertools::Itertools;
use petgraph::algo::astar;
use petgraph::dot::{Config, Dot};
use petgraph::stable_graph::{NodeIndex, StableUnGraph};
use petgraph::visit::{DfsPostOrder, Walker};
use serde::{Deserialize, Serialize};

mod collision;
mod debug;
mod layer;
mod level;
mod level_select;
mod lines;
mod loading;
mod pixie;
mod radio_button;
mod save;

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
    app.add_plugin(RadioButtonPlugin);
    app.add_plugin(PixiePlugin);
    app.add_plugin(LoadingPlugin);
    app.add_plugin(LevelSelectPlugin);
    app.add_plugin(SavePlugin);
    app.add_plugin(DebugLinesPlugin);
    app.add_plugin(RonAssetPlugin::<Level>::new(&["level.ron"]));
    app.add_state(GameState::Loading);

    app.add_stage_after(CoreStage::Update, "after_update", SystemStage::parallel());
    app.add_state_to_stage("after_update", GameState::Loading);

    app.add_system_set(
        SystemSet::on_enter(GameState::Playing).with_system(playing_enter_system.system()),
    );
    app.add_system_set(
        SystemSet::on_exit(GameState::Playing).with_system(playing_exit_system.system()),
    );
    app.add_system_set(
        SystemSet::on_update(GameState::Playing)
            .label("drawing_input")
            .with_system(keyboard_system.system().before("mouse"))
            .with_system(mouse_movement_system.system().label("mouse")),
    );
    app.add_system_set(
        SystemSet::on_update(GameState::Playing)
            .after("drawing_input")
            .label("drawing_mouse_movement")
            .with_system(net_ripping_mouse_movement_system.system())
            .with_system(not_drawing_mouse_movement_system.system())
            .with_system(drawing_mouse_movement_system.system()),
    );
    app.add_system_set(
        SystemSet::on_update(GameState::Playing)
            .before("drawing_interaction")
            .before("radio_button_group_system")
            .with_system(tool_button_system.system())
            .with_system(tool_button_display_system.system())
            .with_system(drawing_mode_change_system.system()),
    );
    app.add_system_set(
        SystemSet::on_update(GameState::Playing)
            .after("drawing_mouse_movement")
            .label("drawing_interaction")
            .with_system(drawing_mouse_click_system.system())
            .with_system(net_ripping_mouse_click_system.system())
            .with_system(draw_mouse_system.system())
            .with_system(draw_net_ripping_system.system())
            .with_system(button_system_system.system()),
    );
    // whenever
    app.add_system_set(
        SystemSet::on_update(GameState::Playing)
            .label("test_buttons")
            .with_system(pixie_button_system.system())
            .with_system(reset_button_system.system())
            .with_system(back_button_system.system()),
    );
    app.add_system_set_to_stage(
        "after_update",
        SystemSet::on_update(GameState::Playing)
            .label("score_calc")
            .with_system(pathfinding_system.system())
            .with_system(update_cost_system.system())
            .with_system(update_test_state_system.system()),
    );
    app.add_system_set_to_stage(
        "after_update",
        SystemSet::on_update(GameState::Playing)
            .label("score_ui")
            .after("score_calc")
            .with_system(pixie_button_text_system.system())
            .with_system(update_pixie_count_text_system.system())
            .with_system(update_elapsed_text_system.system())
            .with_system(update_score_text_system.system()),
    );

    app.add_stage_after(
        bevy_prototype_lyon::plugin::Stage::Shape,
        "after_shape",
        SystemStage::parallel(),
    );
    app.add_system_to_stage("after_shape", shape_visibility_fix_system.system());

    app.init_resource::<Handles>();
    app.init_resource::<SelectedLevel>();
    app.init_resource::<DrawingState>();
    app.init_resource::<LineDrawingState>();
    app.init_resource::<NetRippingState>();
    app.init_resource::<TestingState>();
    app.init_resource::<PathfindingState>();
    app.init_resource::<MouseState>();
    app.init_resource::<RoadGraph>();
    app.init_resource::<ButtonMaterials>();
    app.init_resource::<PixieCount>();
    app.init_resource::<Cost>();
    app.init_resource::<BestScore>();
    app.init_resource::<BestScores>();
    //app.add_plugin(LogDiagnosticsPlugin::default());
    //app.add_plugin(FrameTimeDiagnosticsPlugin::default());
    app.run();
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum GameState {
    Loading,
    LevelSelect,
    Playing,
}

#[derive(Default)]
struct Handles {
    levels: Vec<Handle<Level>>,
    fonts: Vec<Handle<Font>>,
}
struct UiCamera;
struct MainCamera;
struct Cursor;
struct DrawingLine;
struct RippingLine;
struct GridPoint;
struct PixieCountText;
struct CostText;
struct ScoreText;
struct ElapsedText;

struct ToolButton;
struct LayerOneButton;
struct LayerTwoButton;
struct NetRippingButton;
struct PixieButton;
struct ResetButton;
struct BackButton;

#[derive(Default)]
struct SelectedLevel(u32);
#[derive(Default)]
struct PixieCount(u32);
#[derive(Default)]
struct Cost(u32);
#[derive(Default)]
struct Score(Option<u32>);
#[derive(Default)]
struct BestScore(Option<u32>);
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct BestScores(HashMap<u32, u32>);
#[derive(Debug, Clone)]
pub struct RoadSegment {
    points: (Vec2, Vec2),
    layer: u32,
}

#[derive(Debug)]
struct PointGraphNode(NodeIndex);
#[derive(Debug)]
struct SegmentGraphNodes(NodeIndex, NodeIndex);

enum DrawingMode {
    LineDrawing,
    NetRipping,
}
impl Default for DrawingMode {
    fn default() -> Self {
        DrawingMode::LineDrawing
    }
}
#[derive(Default)]
struct DrawingState {
    mode: DrawingMode,
}

struct LineDrawingState {
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
impl Default for LineDrawingState {
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
#[derive(Default)]
struct NetRippingState {
    entities: Vec<Entity>,
    nodes: Vec<NodeIndex>,
    segments: Vec<(Vec2, Vec2)>,
}

#[derive(Default)]
struct TestingState {
    started: Option<f64>,
    elapsed: f64,
    done: bool,
}

#[derive(Default)]
struct PathfindingState {
    valid: bool,
    paths: Vec<(PixieFlavor, Entity, Vec<RoadSegment>)>,
    invalid_nodes: Vec<Entity>,
}

#[derive(Default, Debug, Deserialize, Clone)]
pub struct Terminus {
    point: Vec2,
    emits: HashSet<PixieFlavor>,
    collects: HashSet<PixieFlavor>,
}

struct TerminusIssueIndicator;
struct ShapeStartsInvisible;

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

const GRID_SIZE: f32 = 48.0;
const BOTTOM_BAR_HEIGHT: f32 = 70.0;

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
const UI_GREY_RED_COLOR: Color = Color::rgb(1.0, 0.341, 0.341);

const TUNNEL_MULTIPLIER: f32 = 2.0;

fn tool_button_display_system(
    mut q_text: Query<&mut Text>,
    q_button: Query<(&RadioButton, &Children), (Changed<RadioButton>, With<ToolButton>)>,
) {
    for (button, children) in q_button.iter() {
        for child in children.iter() {
            if let Ok(mut text) = q_text.get_mut(*child) {
                text.sections[0].style.color = if button.selected {
                    Color::GREEN
                } else {
                    UI_WHITE_COLOR
                };
            }
        }
    }
}

fn tool_button_system(
    mut drawing_state: ResMut<DrawingState>,
    mut line_state: ResMut<LineDrawingState>,
    q_interaction_one: Query<&Interaction, (Changed<Interaction>, With<LayerOneButton>)>,
    q_interaction_two: Query<&Interaction, (Changed<Interaction>, With<LayerTwoButton>)>,
    q_interaction_rip: Query<&Interaction, (Changed<Interaction>, With<NetRippingButton>)>,
) {
    for _ in q_interaction_one
        .iter()
        .filter(|i| **i == Interaction::Clicked)
    {
        line_state.layer = 1;
        if !matches!(drawing_state.mode, DrawingMode::LineDrawing) {
            drawing_state.mode = DrawingMode::LineDrawing;
        }
    }
    for _ in q_interaction_two
        .iter()
        .filter(|i| **i == Interaction::Clicked)
    {
        line_state.layer = 2;
        if !matches!(drawing_state.mode, DrawingMode::LineDrawing) {
            drawing_state.mode = DrawingMode::LineDrawing;
        }
    }

    for _ in q_interaction_rip
        .iter()
        .filter(|i| **i == Interaction::Clicked)
    {
        if !matches!(drawing_state.mode, DrawingMode::NetRipping) {
            drawing_state.mode = DrawingMode::NetRipping;
        }
    }
}

fn button_system_system(
    button_materials: Res<ButtonMaterials>,
    mut q_interaction: Query<
        (&Interaction, &mut Handle<ColorMaterial>),
        (Changed<Interaction>, With<Button>, Without<RadioButton>),
    >,
) {
    for (interaction, mut material) in q_interaction.iter_mut() {
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

fn pathfinding_system(
    graph: Res<RoadGraph>,
    mut pathfinding: ResMut<PathfindingState>,
    q_terminuses: Query<(Entity, &Terminus, &PointGraphNode)>,
    q_road_chunks: Query<&RoadSegment>,
) {
    if !graph.is_changed() {
        return;
    }

    info!("doing pathfinding");

    let mut ok = true;
    let mut paths = vec![];
    let mut not_ok = vec![];

    for (a_entity, a, a_node) in q_terminuses.iter() {
        for (_, b, b_node) in q_terminuses.iter() {
            for flavor in a.emits.intersection(&b.collects) {
                info!(
                    "Pixie (flavor {:?}) wants to go from {:?} to {:?}",
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
                    let mut prev_end = graph
                        .graph
                        .node_weight(*path.1.first().unwrap())
                        .and_then(|ent| q_terminuses.get(*ent).ok())
                        .unwrap()
                        .1
                        .point;

                    let segments = path
                        .1
                        .iter()
                        .filter_map(|node| graph.graph.node_weight(*node))
                        .dedup()
                        .filter_map(|ent| q_road_chunks.get(*ent).ok());

                    let mut world_path = vec![];

                    for seg in segments {
                        let flipped_seg = if seg.points.0 != prev_end {
                            RoadSegment {
                                points: (seg.points.1, seg.points.0),
                                layer: seg.layer,
                            }
                        } else {
                            seg.clone()
                        };

                        prev_end = flipped_seg.points.1;

                        world_path.push(flipped_seg);
                    }

                    if world_path.is_empty() {
                        ok = false;
                        continue;
                    }

                    paths.push((*flavor, a_entity, world_path));
                } else {
                    ok = false;
                    not_ok.push(a_entity);
                }
            }
        }
    }

    if !ok || paths.is_empty() {
        pathfinding.valid = false;
        pathfinding.invalid_nodes = not_ok;
        return;
    }

    pathfinding.paths = paths;
    pathfinding.valid = true;
}

fn pixie_button_text_system(
    pathfinding: Res<PathfindingState>,
    testing_state: Res<TestingState>,
    mut q_text: Query<&mut Text>,
    q_pixie_button: Query<&Children, With<PixieButton>>,
) {
    if !pathfinding.is_changed() && !testing_state.is_changed() {
        return;
    }

    for children in q_pixie_button.iter() {
        for child in children.iter() {
            if let Ok(mut text) = q_text.get_mut(*child) {
                if testing_state.started.is_some() && !testing_state.done {
                    text.sections[0].value = "NO WAIT STOP".to_string();
                } else {
                    text.sections[0].value = "RELEASE THE PIXIES".to_string();
                    text.sections[0].style.color = if pathfinding.valid {
                        UI_WHITE_COLOR
                    } else {
                        UI_GREY_RED_COLOR
                    }
                }
            }
        }
    }
}

fn back_button_system(
    q_interaction: Query<&Interaction, (Changed<Interaction>, With<Button>, With<BackButton>)>,
    mut state: ResMut<State<GameState>>,
) {
    for _ in q_interaction.iter().filter(|i| **i == Interaction::Clicked) {
        state.replace(GameState::LevelSelect).unwrap();
    }
}

fn pixie_button_system(
    mut commands: Commands,
    time: Res<Time>,
    mut pixie_count: ResMut<PixieCount>,
    mut testing_state: ResMut<TestingState>,
    mut line_state: ResMut<LineDrawingState>,
    pathfinding: Res<PathfindingState>,
    q_interaction: Query<&Interaction, (Changed<Interaction>, With<Button>, With<PixieButton>)>,
    q_emitters: Query<Entity, With<PixieEmitter>>,
    q_pixies: Query<Entity, With<Pixie>>,
    mut q_indicator: Query<(&mut Visible, &Parent), With<TerminusIssueIndicator>>,
) {
    for _ in q_interaction.iter().filter(|i| **i == Interaction::Clicked) {
        line_state.drawing = false;
        line_state.segments = vec![];

        if testing_state.started.is_some() && !testing_state.done {
            for entity in q_emitters.iter().chain(q_pixies.iter()) {
                commands.entity(entity).despawn();
            }

            testing_state.started = None;
        } else {
            if !pathfinding.valid {
                for (mut visible, parent) in q_indicator.iter_mut() {
                    visible.is_visible = pathfinding.invalid_nodes.contains(&parent.0);
                }

                return;
            }

            for entity in q_emitters.iter() {
                commands.entity(entity).despawn();
            }

            for (mut visible, _) in q_indicator.iter_mut() {
                visible.is_visible = false;
            }

            let duration = 0.4;
            let total_pixies = 50;

            let mut counts = HashMap::default();
            for (_, start_entity, _) in pathfinding.paths.iter() {
                *counts.entry(start_entity).or_insert(0) += 1;
            }

            let mut is = HashMap::default();

            for (flavor, start_entity, world_path) in pathfinding.paths.iter() {
                let i = is.entry(start_entity).or_insert(0);

                // unwrap: we just inserted these above
                let count = counts.get(start_entity).unwrap();
                let pixies = total_pixies / *count;

                // if we have multiple pixies coming out of the same starting
                // point, stagger their emitters evenly. this prevents some
                // awkward bunching up at the start of the path.

                let mut timer = Timer::from_seconds(duration * *count as f32, true);
                timer.set_elapsed(Duration::from_secs_f32((*i + 1) as f32 * duration));

                commands.spawn().insert(PixieEmitter {
                    flavor: *flavor,
                    path: world_path.clone(),
                    remaining: pixies,
                    timer,
                });

                *i += 1;
            }

            testing_state.started = Some(time.seconds_since_startup());
        }

        testing_state.elapsed = 0.0;
        testing_state.done = false;
        pixie_count.0 = 0;
    }
}

fn reset_button_system(
    mut commands: Commands,
    q_interaction: Query<&Interaction, (Changed<Interaction>, With<Button>, With<ResetButton>)>,
    mut graph: ResMut<RoadGraph>,
    mut pixie_count: ResMut<PixieCount>,
    mut testing_state: ResMut<TestingState>,
    mut line_state: ResMut<LineDrawingState>,
    q_road_chunks: Query<Entity, With<RoadSegment>>,
    q_pixies: Query<Entity, With<Pixie>>,
    q_emitters: Query<Entity, With<PixieEmitter>>,
    q_terminuses: Query<Entity, With<Terminus>>,
    mut q_indicator: Query<&mut Visible, With<TerminusIssueIndicator>>,
) {
    for _ in q_interaction.iter().filter(|i| **i == Interaction::Clicked) {
        for chunk in q_road_chunks
            .iter()
            .chain(q_pixies.iter())
            .chain(q_emitters.iter())
        {
            commands.entity(chunk).despawn_recursive();
        }

        for mut visible in q_indicator.iter_mut() {
            visible.is_visible = false;
        }

        graph.graph.clear();

        // we just nuked the graph, but left the start/end points
        // so we need to overwrite their old nodes with new ones.
        for entity in q_terminuses.iter() {
            let node = graph.graph.add_node(entity);
            commands.entity(entity).insert(PointGraphNode(node));
        }

        line_state.drawing = false;
        line_state.segments = vec![];

        testing_state.started = None;
        testing_state.done = false;
        testing_state.elapsed = 0.0;

        pixie_count.0 = 0;
    }
}

fn snap_to_grid(position: Vec2, grid_size: f32) -> Vec2 {
    (position / grid_size).round() * grid_size
}

fn draw_mouse_system(
    mut commands: Commands,
    line_drawing: Res<LineDrawingState>,
    mouse: Res<MouseState>,
    q_cursor: Query<Entity, With<Cursor>>,
    q_drawing: Query<Entity, With<DrawingLine>>,
) {
    if mouse.is_changed() || line_drawing.is_changed() {
        let snapped = snap_to_grid(mouse.position, GRID_SIZE);

        for entity in q_cursor.iter() {
            commands.entity(entity).despawn();
        }
        let shape = shapes::Circle {
            radius: 5.5,
            ..Default::default()
        };
        let color = if line_drawing.drawing && line_drawing.valid {
            DRAWING_ROAD_COLORS[line_drawing.layer as usize - 1]
        } else if !line_drawing.drawing && line_drawing.valid {
            UI_WHITE_COLOR
        } else {
            Color::RED
        };
        commands
            .spawn_bundle(GeometryBuilder::build_as(
                &shape,
                ShapeColors::new(color.as_rgba_linear()),
                DrawMode::Stroke(StrokeOptions::default().with_line_width(2.0)),
                Transform::from_translation(snapped.extend(layer::CURSOR)),
            ))
            .insert(Cursor);
    }

    if !line_drawing.is_changed() {
        return;
    }

    for entity in q_drawing.iter() {
        commands.entity(entity).despawn();
    }

    if line_drawing.drawing {
        let color = if line_drawing.valid {
            DRAWING_ROAD_COLORS[line_drawing.layer as usize - 1]
        } else {
            Color::RED
        };

        for (a, b) in line_drawing.segments.iter() {
            commands
                .spawn_bundle(GeometryBuilder::build_as(
                    &shapes::Line(*a, *b),
                    ShapeColors::new(color.as_rgba_linear()),
                    DrawMode::Stroke(StrokeOptions::default().with_line_width(2.0)),
                    Transform::from_xyz(0.0, 0.0, layer::ROAD_OVERLAY),
                ))
                .insert(DrawingLine);
        }
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
        commands
            .spawn_bundle(GeometryBuilder::build_as(
                &shapes::Line(*a, *b),
                ShapeColors::new(Color::RED),
                DrawMode::Stroke(StrokeOptions::default().with_line_width(2.0)),
                Transform::from_xyz(0.0, 0.0, layer::ROAD_OVERLAY),
            ))
            .insert(RippingLine);
    }
}

fn drawing_mode_change_system(
    drawing_state: Res<DrawingState>,
    mut line_state: ResMut<LineDrawingState>,
    mut ripping_state: ResMut<NetRippingState>,
) {
    if !drawing_state.is_changed() {
        return;
    }

    match drawing_state.mode {
        DrawingMode::LineDrawing => {
            ripping_state.entities = vec![];
            ripping_state.nodes = vec![];
            ripping_state.segments = vec![];
        }
        DrawingMode::NetRipping => {
            line_state.drawing = false;
            line_state.segments = vec![];
        }
    }
}

fn keyboard_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut line_state: ResMut<LineDrawingState>,
    mut drawing_state: ResMut<DrawingState>,
    mut q_radio_button: Query<&mut RadioButton>,
    q_layer_one_button: Query<Entity, With<LayerOneButton>>,
    q_layer_two_button: Query<Entity, With<LayerTwoButton>>,
    q_net_ripping_button: Query<Entity, With<NetRippingButton>>,
) {
    if keyboard_input.pressed(KeyCode::Key1) {
        line_state.layer = 1;
        if !matches!(drawing_state.mode, DrawingMode::LineDrawing) {
            drawing_state.mode = DrawingMode::LineDrawing;
        }

        if let Ok(ent) = q_layer_one_button.single() {
            if let Ok(mut radio) = q_radio_button.get_mut(ent) {
                radio.selected = true;
            }
        }
    } else if keyboard_input.pressed(KeyCode::Key2) {
        line_state.layer = 2;
        if !matches!(drawing_state.mode, DrawingMode::LineDrawing) {
            drawing_state.mode = DrawingMode::LineDrawing;
        }

        if let Ok(ent) = q_layer_two_button.single() {
            if let Ok(mut radio) = q_radio_button.get_mut(ent) {
                radio.selected = true;
            }
        }
    } else if keyboard_input.pressed(KeyCode::Escape) {
        if matches!(drawing_state.mode, DrawingMode::NetRipping) {
            drawing_state.mode = DrawingMode::LineDrawing;
        } else {
            line_state.drawing = false;
            line_state.segments = vec![];
        }
    } else if keyboard_input.pressed(KeyCode::R) {
        if !matches!(drawing_state.mode, DrawingMode::NetRipping) {
            drawing_state.mode = DrawingMode::NetRipping;
        }

        if let Ok(ent) = q_net_ripping_button.single() {
            if let Ok(mut radio) = q_radio_button.get_mut(ent) {
                radio.selected = true;
            }
        }
    }
}

fn net_ripping_mouse_click_system(
    mut commands: Commands,
    mouse_input: ResMut<Input<MouseButton>>,
    mut ripping_state: ResMut<NetRippingState>,
    testing_state: Res<TestingState>,
    drawing_state: Res<DrawingState>,
    mut graph: ResMut<RoadGraph>,
) {
    if !matches!(drawing_state.mode, DrawingMode::NetRipping) {
        return;
    }

    if testing_state.started.is_some() && !testing_state.done {
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

#[allow(clippy::too_many_arguments)]
fn drawing_mouse_click_system(
    mut commands: Commands,
    mouse_input: ResMut<Input<MouseButton>>,
    mouse: Res<MouseState>,
    drawing_state: ResMut<DrawingState>,
    mut line_state: ResMut<LineDrawingState>,
    testing_state: Res<TestingState>,
    mut graph: ResMut<RoadGraph>,
    q_point_nodes: Query<&PointGraphNode>,
    q_segment_nodes: Query<&SegmentGraphNodes>,
    q_road_segments: Query<&RoadSegment>,
) {
    if mouse.window_position.y < BOTTOM_BAR_HEIGHT {
        return;
    }

    if !matches!(drawing_state.mode, DrawingMode::LineDrawing) {
        return;
    }

    if testing_state.started.is_some() && !testing_state.done {
        return;
    }

    if mouse_input.just_pressed(MouseButton::Left) {
        if !line_state.drawing {
            if line_state.valid {
                info!("{:?}", mouse.snapped);
                line_state.drawing = true;
                line_state.start = mouse.snapped;
                line_state.end = line_state.start;
            }
        } else {
            if line_state.end == line_state.start {
                line_state.drawing = false;
            }

            if !line_state.valid {
                return;
            }

            if line_state.adds.is_empty() {
                return;
            }

            let mut previous_end: Option<NodeIndex> = None;

            for add in line_state.adds.iter() {
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
                    if let SegmentConnection::TryExtend(entity) = add.connections.0.get(0).unwrap()
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
                    if let SegmentConnection::TryExtend(entity) = add.connections.1.get(0).unwrap()
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
                    spawn_road_segment(&mut commands, &mut graph, points, line_state.layer);

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
                                let (start_node_a, end_node_a) = spawn_road_segment(
                                    &mut commands,
                                    &mut graph,
                                    (segment.points.0, *point),
                                    segment.layer,
                                );

                                // reconnect new segment to split line's old start node neighbors
                                for neighbor in start_neighbors {
                                    graph.graph.add_edge(neighbor, start_node_a, 0.0);
                                }
                                graph.graph.add_edge(end_node_a, *node, 0.0);

                                // create a new segment on (entity end, this_point)
                                let (start_node_b, end_node_b) = spawn_road_segment(
                                    &mut commands,
                                    &mut graph,
                                    (*point, segment.points.1),
                                    segment.layer,
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

            if line_state.stop {
                line_state.drawing = false;
                line_state.stop = false;
            }

            line_state.start = line_state.end;
            line_state.adds = vec![];
            line_state.segments = vec![];

            println!(
                "{:?}",
                Dot::with_config(&graph.graph, &[Config::EdgeNoLabel, Config::NodeIndexLabel])
            );
        }
    }
}

fn mouse_movement_system(
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

fn net_ripping_mouse_movement_system(
    drawing_state: Res<DrawingState>,
    mouse: Res<MouseState>,
    mut ripping_state: ResMut<NetRippingState>,
    testing_state: Res<TestingState>,
    graph: Res<RoadGraph>,
    q_colliders: Query<(&Parent, &Collider, &ColliderLayer)>,
    q_road_segments: Query<&RoadSegment>,
    q_segment_nodes: Query<&SegmentGraphNodes>,
) {
    if !matches!(drawing_state.mode, DrawingMode::NetRipping) {
        return;
    }

    if testing_state.started.is_some() && !testing_state.done {
        return;
    }

    if !mouse.is_changed() && !drawing_state.is_changed() {
        return;
    }

    // TODO we don't need to do this work if mouse.snapped is not changed.
    // maybe we need a separate resource / change detection for MouseSnappingState
    // or something.

    ripping_state.entities = vec![];
    ripping_state.nodes = vec![];
    ripping_state.segments = vec![];

    let mut collisions: Vec<_> = q_colliders
        .iter()
        .filter_map(|(parent, collider, layer)| match collider {
            Collider::Segment(segment) => {
                match point_segment_collision(mouse.snapped, segment.0, segment.1) {
                    SegmentCollision::None => None,
                    _ => {
                        if layer.0 == 0 {
                            None
                        } else {
                            Some((parent.0, layer.0))
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

fn not_drawing_mouse_movement_system(
    mut line_state: ResMut<LineDrawingState>,
    drawing_state: Res<DrawingState>,
    mouse: Res<MouseState>,
    q_colliders: Query<(&Parent, &Collider, &ColliderLayer)>,
) {
    if !matches!(drawing_state.mode, DrawingMode::LineDrawing) {
        return;
    }

    if !mouse.is_changed() {
        return;
    }

    if line_state.drawing {
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

    if bad && line_state.valid {
        line_state.valid = false;
    } else if !bad && !line_state.valid {
        line_state.valid = true;
    }
}

fn drawing_mouse_movement_system(
    mut line_state: ResMut<LineDrawingState>,
    testing_state: Res<TestingState>,
    mouse: Res<MouseState>,
    q_colliders: Query<(&Parent, &Collider, &ColliderLayer)>,
) {
    if !line_state.drawing {
        return;
    }

    if testing_state.started.is_some() && !testing_state.done {
        return;
    }

    if mouse.snapped == line_state.end && line_state.layer == line_state.prev_layer {
        return;
    }

    info!("{:?}", mouse.snapped);

    line_state.end = mouse.snapped;
    line_state.prev_layer = line_state.layer;

    // line drawing can be coerced to follow one axis or another by moving the mouse to a
    // position that is a straight line from the starting point in that axis.

    if line_state.start.x == mouse.snapped.x {
        line_state.axis_preference = Some(Axis::Y);
    } else if line_state.start.y == mouse.snapped.y {
        line_state.axis_preference = Some(Axis::X);
    }

    if mouse.snapped == line_state.start {
        line_state.segments = vec![];
        line_state.adds = vec![];
        line_state.valid = true;
    }

    let possible = possible_lines(line_state.start, mouse.snapped, line_state.axis_preference);

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
                                if layer.0 == line_state.layer || layer.0 == 0 {
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
                                    point_segment_collision(line_state.start, s.0, s.1),
                                    SegmentCollision::Touching
                                );
                                let end_touching = matches!(
                                    point_segment_collision(line_state.end, s.0, s.1),
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
                                    point_segment_collision(line_state.start, s.0, s.1),
                                    SegmentCollision::Connecting
                                );
                                let end_touching = matches!(
                                    point_segment_collision(line_state.end, s.0, s.1),
                                    SegmentCollision::Connecting
                                );

                                if !start_touching && !end_touching {
                                    ok = false;
                                    break;
                                }

                                if (line_state.start == *a && start_touching)
                                    || (line_state.end == *a && end_touching)
                                {
                                    if matches!(collision, SegmentCollision::ConnectingParallel)
                                        && layer.0 == line_state.layer
                                    {
                                        connections.0.push(SegmentConnection::TryExtend(parent.0))
                                    } else {
                                        connections.0.push(SegmentConnection::Add(parent.0))
                                    }
                                }
                                if (line_state.start == *b && start_touching)
                                    || (line_state.end == *b && end_touching)
                                {
                                    if matches!(collision, SegmentCollision::ConnectingParallel)
                                        && layer.0 == line_state.layer
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

                            if *p != line_state.start && *p != line_state.end {
                                ok = false;
                                break;
                            }

                            if *p == line_state.end {
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
        line_state.segments = segments.clone();
        line_state.adds = filtered_adds.first().cloned().unwrap();
        line_state.stop = filtered_stops.first().cloned().unwrap();
        line_state.valid = true;
    } else if let Some(segments) = possible.get(0) {
        line_state.segments = segments.clone();
        line_state.adds = vec![];
        line_state.valid = false;
    } else {
        line_state.segments = vec![];
        line_state.adds = vec![];
        line_state.valid = false;
    }
}

fn update_pixie_count_text_system(
    pixie_count: Res<PixieCount>,
    mut query: Query<&mut Text, With<PixieCountText>>,
) {
    if !pixie_count.is_changed() {
        return;
    }

    let mut text = query.single_mut().unwrap();
    text.sections[0].value = format!("Þ{}", pixie_count.0);
}

/// Workaround for bevy_prototype_lyon always setting `is_visible = true` after it builds a mesh.
/// We'll just swoop in right afterwards and change it back.
fn shape_visibility_fix_system(
    mut invis: Query<&mut Visible, (Changed<Handle<Mesh>>, With<ShapeStartsInvisible>)>,
) {
    for mut visible in invis.iter_mut() {
        visible.is_visible = false;
    }
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
            Transform::from_xyz(0.0, 0.0, layer::ROAD - layer as f32),
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

fn spawn_obstacle(commands: &mut Commands, obstacle: &level::Obstacle) {
    match obstacle {
        level::Obstacle::Rect(top_left, bottom_right) => {
            let diff = *bottom_right - *top_left;
            let origin = (*top_left + *bottom_right) / 2.0;

            commands
                .spawn_bundle(GeometryBuilder::build_as(
                    &shapes::Rectangle {
                        width: diff.x.abs(),
                        height: diff.y.abs(),
                        ..Default::default()
                    },
                    ShapeColors::new(Color::GRAY),
                    DrawMode::Fill(FillOptions::default()),
                    Transform::from_translation(origin.extend(layer::OBSTACLE)),
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
        _ => {
            info!("{:?} not implemented", obstacle);
        }
    }
}

fn spawn_terminus(
    commands: &mut Commands,
    graph: &mut ResMut<RoadGraph>,
    handles: &Res<Handles>,
    terminus: &Terminus,
) {
    let label_offset = 22.0;
    let label_spacing = 22.0;

    let ent = commands
        .spawn_bundle(GeometryBuilder::build_as(
            &shapes::Circle {
                radius: 5.5,
                center: Vec2::splat(0.0),
            },
            ShapeColors::outlined(
                BACKGROUND_COLOR.as_rgba_linear(),
                FINISHED_ROAD_COLORS[0].as_rgba_linear(),
            ),
            DrawMode::Outlined {
                fill_options: FillOptions::default(),
                outline_options: StrokeOptions::default().with_line_width(2.0),
            },
            Transform::from_translation(terminus.point.extend(layer::TERMINUS)),
        ))
        .insert(terminus.clone())
        .with_children(|parent| {
            parent
                .spawn()
                .insert(Collider::Point(terminus.point))
                .insert(ColliderLayer(1));

            let mut i = 0;

            for flavor in terminus.emits.iter() {
                let label_pos =
                    Vec2::new(0.0, -1.0 * label_offset + -1.0 * i as f32 * label_spacing);

                let label = if flavor.net > 0 {
                    format!("OUT.{}", flavor.net + 1)
                } else {
                    "OUT".to_string()
                };

                parent.spawn_bundle(Text2dBundle {
                    text: Text::with_section(
                        label,
                        TextStyle {
                            font: handles.fonts[0].clone(),
                            font_size: 30.0,
                            color: PIXIE_COLORS[flavor.color as usize],
                        },
                        TextAlignment {
                            vertical: VerticalAlign::Center,
                            horizontal: HorizontalAlign::Center,
                        },
                    ),
                    transform: Transform::from_translation(label_pos.extend(layer::TERMINUS)),
                    ..Default::default()
                });

                i += 1;
            }

            for flavor in terminus.collects.iter() {
                let label_pos =
                    Vec2::new(0.0, -1.0 * label_offset + -1.0 * i as f32 * label_spacing);

                let label = if flavor.net > 0 {
                    format!("IN.{}", flavor.net + 1)
                } else {
                    "IN".to_string()
                };

                parent.spawn_bundle(Text2dBundle {
                    text: Text::with_section(
                        label,
                        TextStyle {
                            font: handles.fonts[0].clone(),
                            font_size: 30.0,
                            color: PIXIE_COLORS[flavor.color as usize],
                        },
                        TextAlignment {
                            vertical: VerticalAlign::Center,
                            horizontal: HorizontalAlign::Center,
                        },
                    ),
                    transform: Transform::from_translation(label_pos.extend(layer::TERMINUS)),
                    ..Default::default()
                });

                i += 1;
            }

            // TODO above code supports multiple emitters/collectors, but below
            // assumes a single emitter.

            parent
                .spawn_bundle(GeometryBuilder::build_as(
                    &shapes::Circle {
                        radius: 5.5,
                        center: Vec2::splat(0.0),
                    },
                    ShapeColors::new(Color::RED),
                    DrawMode::Fill(FillOptions::default()),
                    Transform::from_xyz(-30.0, -1.0 * label_offset, layer::TERMINUS),
                ))
                .insert(TerminusIssueIndicator)
                .insert(ShapeStartsInvisible);
        })
        .id();

    let node = graph.graph.add_node(ent);

    commands.entity(ent).insert(PointGraphNode(node));
}

fn update_cost_system(
    graph: Res<RoadGraph>,
    line_draw: Res<LineDrawingState>,
    mut r_cost: ResMut<Cost>,
    q_segments: Query<(&RoadSegment, &Children)>,
    q_colliders: Query<&ColliderLayer>,
    mut q_cost: Query<&mut Text, With<CostText>>,
) {
    if !graph.is_changed() && !line_draw.is_changed() {
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

    r_cost.0 = cost as u32;

    let mut potential_cost = 0.0;
    if line_draw.valid {
        for segment in line_draw.segments.iter() {
            let multiplier = if line_draw.layer > 1 {
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
        text.sections[1].style.color = FINISHED_ROAD_COLORS[line_draw.layer as usize - 1]
    }
}

fn update_test_state_system(
    mut testing_state: ResMut<TestingState>,
    time: Res<Time>,
    q_emitter: Query<&PixieEmitter>,
    q_pixie: Query<Entity, With<Pixie>>,
) {
    if testing_state.done {
        return;
    }

    if let Some(started) = testing_state.started {
        testing_state.elapsed = time.seconds_since_startup() - started;
    }

    if q_emitter.iter().count() < 1 {
        return;
    }

    for emitter in q_emitter.iter() {
        if emitter.remaining > 0 {
            return;
        }
    }

    if q_pixie.iter().count() > 0 {
        return;
    }

    testing_state.done = true;
}

fn update_score_text_system(
    testing_state: Res<TestingState>,
    pixie_count: Res<PixieCount>,
    cost: Res<Cost>,
    selected_level: Res<SelectedLevel>,
    mut best_score: ResMut<BestScore>,
    mut best_scores: ResMut<BestScores>,
    mut q_score_text: Query<&mut Text, With<ScoreText>>,
) {
    if !testing_state.is_changed() {
        return;
    }

    let eff_text = if testing_state.done {
        let val = ((pixie_count.0 as f32 / cost.0 as f32 / testing_state.elapsed as f32) * 10000.0)
            .ceil() as u32;

        if let Some(best) = best_scores.0.get_mut(&selected_level.0) {
            if *best < val {
                *best = val;
            }
        } else {
            best_scores.0.insert(selected_level.0, val);
        }

        match best_score.0 {
            Some(best) if best < val => {
                best_score.0 = Some(val);
                format!("Æ{}", val)
            }
            Some(best) => {
                format!("Æ{} ({})", val, best)
            }
            None => {
                best_score.0 = Some(val);
                format!("Æ{}", val)
            }
        }
    } else {
        match best_score.0 {
            Some(best) => format!("Æ? ({})", best),
            _ => "Æ?".to_string(),
        }
    };

    if let Some(mut text) = q_score_text.iter_mut().next() {
        text.sections[0].value = eff_text;
    }
}

fn update_elapsed_text_system(
    testing_state: Res<TestingState>,
    mut q_text: Query<&mut Text, With<ElapsedText>>,
) {
    if !testing_state.is_changed() {
        return;
    }

    for mut text in q_text.iter_mut() {
        text.sections[0].value = format!("ŧ{:.1}", testing_state.elapsed);
    }
}

fn playing_exit_system(mut commands: Commands, query: Query<Entity, Without<UiCamera>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn playing_enter_system(
    mut commands: Commands,
    mut more_commands: Commands,
    mut graph: ResMut<RoadGraph>,
    button_materials: Res<ButtonMaterials>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    levels: Res<Assets<Level>>,
    level: Res<SelectedLevel>,
    best_scores: Res<BestScores>,
    handles: Res<Handles>,
) {
    // Reset
    commands.insert_resource(Score::default());
    commands.insert_resource(PixieCount::default());
    commands.insert_resource(Cost::default());
    commands.insert_resource(DrawingState::default());
    commands.insert_resource(LineDrawingState::default());
    commands.insert_resource(NetRippingState::default());
    commands.insert_resource(TestingState::default());
    commands.insert_resource(PathfindingState::default());
    graph.graph.clear();

    if let Some(score) = best_scores.0.get(&level.0) {
        commands.insert_resource(BestScore(Some(*score)));
    } else {
        commands.insert_resource(BestScore::default());
    }

    // Build arena

    let mut camera = OrthographicCameraBundle::new_2d();
    camera.transform.translation.y -= 10.0;

    commands.spawn_bundle(camera).insert(MainCamera);

    for x in ((-25 * (GRID_SIZE as i32))..=25 * (GRID_SIZE as i32)).step_by(GRID_SIZE as usize) {
        for y in (-15 * (GRID_SIZE as i32)..=15 * (GRID_SIZE as i32)).step_by(GRID_SIZE as usize) {
            commands
                .spawn_bundle(GeometryBuilder::build_as(
                    &shapes::Circle {
                        radius: 2.5,
                        ..Default::default()
                    },
                    ShapeColors::new(GRID_COLOR.as_rgba_linear()),
                    DrawMode::Fill(FillOptions::default()),
                    Transform::from_xyz(x as f32, y as f32, layer::GRID),
                ))
                .insert(GridPoint);
        }
    }

    // Build level

    let level = levels
        .get(handles.levels[level.0 as usize - 1].clone())
        .unwrap();

    for t in level.terminuses.iter() {
        spawn_terminus(&mut commands, &mut graph, &handles, t);
    }

    for o in level.obstacles.iter() {
        spawn_obstacle(&mut commands, o);
    }

    println!(
        "{:?}",
        Dot::with_config(&graph.graph, &[Config::EdgeNoLabel])
    );

    // Build UI

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
                            // Back button
                            parent
                                .spawn_bundle(ButtonBundle {
                                    style: Style {
                                        size: Size::new(Val::Px(50.0), Val::Percent(100.0)),
                                        // horizontally center child text
                                        justify_content: JustifyContent::Center,
                                        // vertically center child text
                                        align_items: AlignItems::Center,
                                        ..Default::default()
                                    },
                                    material: button_materials.normal.clone(),
                                    ..Default::default()
                                })
                                .insert(BackButton)
                                .with_children(|parent| {
                                    parent.spawn_bundle(TextBundle {
                                        text: Text::with_section(
                                            "◄",
                                            TextStyle {
                                                font: handles.fonts[0].clone(),
                                                font_size: 30.0,
                                                color: Color::rgb(0.9, 0.9, 0.9),
                                            },
                                            Default::default(),
                                        ),
                                        ..Default::default()
                                    });
                                });
                            // Tool Buttons
                            let layer_one_id = parent
                                .spawn_bundle(ButtonBundle {
                                    style: Style {
                                        size: Size::new(Val::Px(50.0), Val::Percent(100.0)),
                                        // horizontally center child text
                                        justify_content: JustifyContent::Center,
                                        // vertically center child text
                                        align_items: AlignItems::Center,
                                        margin: Rect {
                                            left: Val::Px(30.0),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    },
                                    material: button_materials.normal.clone(),
                                    ..Default::default()
                                })
                                .insert(LayerOneButton)
                                .insert(ToolButton)
                                .insert(RadioButton { selected: true })
                                .with_children(|parent| {
                                    parent.spawn_bundle(TextBundle {
                                        text: Text::with_section(
                                            "1",
                                            TextStyle {
                                                font: handles.fonts[0].clone(),
                                                font_size: 30.0,
                                                color: Color::rgb(0.9, 0.9, 0.9),
                                            },
                                            Default::default(),
                                        ),
                                        ..Default::default()
                                    });
                                })
                                .id();

                            let layer_two_id = parent
                                .spawn_bundle(ButtonBundle {
                                    style: Style {
                                        size: Size::new(Val::Px(50.0), Val::Percent(100.0)),
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
                                .insert(LayerTwoButton)
                                .insert(ToolButton)
                                .insert(RadioButton { selected: false })
                                .with_children(|parent| {
                                    parent.spawn_bundle(TextBundle {
                                        text: Text::with_section(
                                            "2",
                                            TextStyle {
                                                font: handles.fonts[0].clone(),
                                                font_size: 30.0,
                                                color: Color::rgb(0.9, 0.9, 0.9),
                                            },
                                            Default::default(),
                                        ),
                                        ..Default::default()
                                    });
                                })
                                .id();

                            let net_ripping_id = parent
                                .spawn_bundle(ButtonBundle {
                                    style: Style {
                                        size: Size::new(Val::Px(50.0), Val::Percent(100.0)),
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
                                .insert(NetRippingButton)
                                .insert(ToolButton)
                                .insert(RadioButton { selected: false })
                                .with_children(|parent| {
                                    parent.spawn_bundle(TextBundle {
                                        text: Text::with_section(
                                            "R",
                                            TextStyle {
                                                font: handles.fonts[0].clone(),
                                                font_size: 30.0,
                                                color: Color::rgb(0.9, 0.9, 0.9),
                                            },
                                            Default::default(),
                                        ),
                                        ..Default::default()
                                    });
                                })
                                .id();

                            let tool_group_id = more_commands
                                .spawn()
                                .insert(RadioButtonGroup {
                                    entities: vec![layer_one_id, layer_two_id, net_ripping_id],
                                })
                                .id();
                            more_commands
                                .entity(layer_one_id)
                                .insert(RadioButtonGroupRelation(tool_group_id));
                            more_commands
                                .entity(layer_two_id)
                                .insert(RadioButtonGroupRelation(tool_group_id));
                            more_commands
                                .entity(net_ripping_id)
                                .insert(RadioButtonGroupRelation(tool_group_id));
                        });

                    // Score, etc
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
                            // Score etc
                            parent
                                .spawn_bundle(TextBundle {
                                    style: Style {
                                        size: Size::new(Val::Px(120.0), Val::Px(30.0)),
                                        ..Default::default()
                                    },

                                    text: Text {
                                        sections: vec![
                                            TextSection {
                                                value: "0".to_string(),
                                                style: TextStyle {
                                                    font: handles.fonts[0].clone(),
                                                    font_size: 30.0,
                                                    color: UI_WHITE_COLOR,
                                                },
                                            },
                                            TextSection {
                                                value: "".to_string(),
                                                style: TextStyle {
                                                    font: handles.fonts[0].clone(),
                                                    font_size: 30.0,
                                                    color: PIXIE_COLORS[0],
                                                },
                                            },
                                        ],
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                })
                                .insert(CostText);

                            parent
                                .spawn_bundle(TextBundle {
                                    style: Style {
                                        size: Size::new(Val::Px(100.0), Val::Px(30.0)),
                                        ..Default::default()
                                    },
                                    text: Text::with_section(
                                        "0",
                                        TextStyle {
                                            font: handles.fonts[0].clone(),
                                            font_size: 30.0,
                                            color: PIXIE_COLORS[1],
                                        },
                                        Default::default(),
                                    ),
                                    ..Default::default()
                                })
                                .insert(PixieCountText);

                            parent
                                .spawn_bundle(TextBundle {
                                    style: Style {
                                        size: Size::new(Val::Px(100.0), Val::Px(30.0)),
                                        ..Default::default()
                                    },
                                    text: Text {
                                        sections: vec![TextSection {
                                            value: "0".to_string(),
                                            style: TextStyle {
                                                font: handles.fonts[0].clone(),
                                                font_size: 30.0,
                                                color: PIXIE_COLORS[2],
                                            },
                                        }],
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                })
                                .insert(ElapsedText);

                            parent
                                .spawn_bundle(TextBundle {
                                    style: Style {
                                        size: Size::new(Val::Px(200.0), Val::Px(30.0)),
                                        ..Default::default()
                                    },
                                    text: Text {
                                        sections: vec![TextSection {
                                            value: "0".to_string(),
                                            style: TextStyle {
                                                font: handles.fonts[0].clone(),
                                                font_size: 30.0,
                                                color: FINISHED_ROAD_COLORS[1],
                                            },
                                        }],
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                })
                                .insert(ScoreText);
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
                                                font: handles.fonts[0].clone(),
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
                                                font: handles.fonts[0].clone(),
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
