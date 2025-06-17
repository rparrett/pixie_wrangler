// disable console on windows for release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::Duration;
#[cfg(feature = "debugdump")]
use std::{fs::File, io::Write};

use crate::{
    level::{Level, Obstacle, Terminus},
    loading::LoadingPlugin,
    net_ripping::NetRippingPlugin,
    pixie::{Pixie, PixieEmitter, PixieFlavor, PixiePlugin},
    road_drawing::{RoadDrawingPlugin, RoadDrawingState},
    save::{BestScores, MusicVolume, SavePlugin, Solution, Solutions},
    sim::{SimulationPlugin, SimulationSettings, SimulationState, SimulationSteps},
    ui::{
        radio_button::{RadioButton, RadioButtonGroup, RadioButtonGroupRelation, RadioButtonSet},
        UiPlugin,
    },
};

use bevy::{
    app::MainScheduleOrder, asset::AssetMetaCheck, ecs::schedule::ScheduleLabel,
    platform::collections::HashMap, prelude::*, sprite::Anchor, window::CursorMoved,
};

use bevy_common_assets::ron::RonAssetPlugin;
use bevy_easings::EasingsPlugin;
use bevy_prototype_lyon::prelude::*;
use itertools::Itertools;
use net_ripping::NetRippingState;
use petgraph::{
    algo::astar,
    dot::{Config, Dot},
    stable_graph::{NodeIndex, StableUnGraph},
};

mod collision;
mod layer;
mod level;
mod lines;
mod loading;
mod net_ripping;
mod pixie;
mod road_drawing;
mod save;
mod sim;
mod theme;
mod ui;

fn main() {
    let mut app = App::new();

    let mut order = app.world_mut().resource_mut::<MainScheduleOrder>();
    order.insert_after(Update, AfterUpdate);

    app.insert_resource(ClearColor(theme::BACKGROUND.into()));

    let default = DefaultPlugins
        .set(WindowPlugin {
            primary_window: Some(Window {
                title: String::from("Pixie Wrangler"),
                ..default()
            }),
            ..default()
        })
        .set(AssetPlugin {
            // Workaround for Bevy attempting to load .meta files in wasm builds. On itch,
            // the CDN serves HTTP 403 errors instead of 404 when files don't exist, which
            // causes Bevy to break.
            meta_check: AssetMetaCheck::Never,
            ..default()
        })
        .build();

    #[cfg(feature = "debugdump")]
    let default = default.disable::<bevy::log::LogPlugin>();

    // Bevy Plugins
    app.add_plugins(default);
    // Third Party
    app.add_plugins((
        EasingsPlugin::default(),
        RonAssetPlugin::<Level>::new(&["level.ron"]),
    ));
    // Our Plugins
    app.add_plugins((
        RoadDrawingPlugin,
        NetRippingPlugin,
        ShapePlugin,
        PixiePlugin,
        SimulationPlugin,
        LoadingPlugin,
        SavePlugin,
        UiPlugin,
    ));

    app.init_state::<GameState>();
    app.enable_state_scoped_entities::<GameState>();

    app.add_systems(
        OnEnter(GameState::Playing),
        (reset_game, spawn_level, spawn_game_ui).chain(),
    );
    app.add_systems(OnExit(GameState::Loading), spawn_music);

    app.configure_sets(Update, DrawingInput.run_if(in_state(GameState::Playing)));
    app.add_systems(
        Update,
        (
            keyboard_system.before(mouse_movement_system),
            mouse_movement_system,
        )
            .before(RadioButtonSet)
            .in_set(DrawingInput),
    );

    app.configure_sets(
        Update,
        DrawingMouseMovement
            .after(DrawingInput)
            .run_if(in_state(GameState::Playing)),
    );

    app.add_systems(
        Update,
        (
            tool_button_system,
            tool_button_display_system,
            drawing_mode_change_system,
        )
            .before(DrawingInteraction)
            .before(RadioButtonSet)
            .run_if(in_state(GameState::Playing)),
    );

    app.configure_sets(
        Update,
        DrawingInteraction
            .after(DrawingMouseMovement)
            .run_if(in_state(GameState::Playing)),
    );
    app.add_systems(Update, draw_cursor_system.in_set(DrawingInteraction));

    // whenever, when playing
    app.add_systems(
        Update,
        (
            pixie_button_system,
            reset_button_system,
            speed_button_system,
            back_button_system,
        )
            .run_if(in_state(GameState::Playing)),
    );
    // whenever
    app.add_systems(
        Update,
        set_music_volume_system
            .run_if(resource_changed::<MusicVolume>)
            .run_if(in_state(GameState::LevelSelect)),
    );

    app.configure_sets(AfterUpdate, ScoreCalc.run_if(in_state(GameState::Playing)));

    app.add_systems(
        AfterUpdate,
        (
            pathfinding_system,
            update_cost_system,
            save_solution_system,
            update_score_system.after(update_cost_system),
        )
            .in_set(ScoreCalc),
    );

    app.configure_sets(
        AfterUpdate,
        ScoreUi
            .after(ScoreCalc)
            .run_if(in_state(GameState::Playing)),
    );
    app.add_systems(
        AfterUpdate,
        (
            pixie_button_text_system,
            update_pixie_count_text_system,
            update_elapsed_text_system,
            update_score_text_system,
        )
            .in_set(ScoreUi),
    );

    app.init_resource::<SelectedLevel>();
    app.init_resource::<SelectedTool>();
    app.init_resource::<PathfindingState>();
    app.init_resource::<MousePos>();
    app.init_resource::<MouseSnappedPos>();
    app.init_resource::<RoadGraph>();
    app.init_resource::<PixieCount>();
    app.init_resource::<Cost>();

    #[cfg(feature = "debugdump")]
    {
        let settings = bevy_mod_debugdump::schedule_graph::Settings {
            ambiguity_enable: false,
            ambiguity_enable_on_world: false,
            ..Default::default()
        };

        let dot = bevy_mod_debugdump::schedule_graph_dot(&mut app, Update, &settings);
        let mut f = File::create("debugdump_update.dot").unwrap();
        f.write_all(dot.as_bytes()).unwrap();
    }

    #[cfg(not(feature = "debugdump"))]
    app.run();
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, ScheduleLabel)]
struct AfterUpdate;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct DrawingInput;
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct DrawingMouseMovement;
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct DrawingInteraction;
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct ScoreCalc;
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
struct ScoreUi;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    Loading,
    LevelSelect,
    Playing,
}

#[derive(Resource, Default)]
struct Handles {
    levels: Vec<Handle<Level>>,
    fonts: Vec<Handle<Font>>,
    music: Handle<AudioSource>,
}
#[derive(Component)]
struct MainCamera;
#[derive(Component)]
struct Cursor;
#[derive(Component)]
struct GridPoint;
#[derive(Component)]
struct PixieCountText;
#[derive(Component)]
struct CostText;
#[derive(Component)]
struct ScoreText;
#[derive(Component)]
struct ElapsedText;

#[derive(Component)]
struct ToolButton;
#[derive(Component)]
struct LayerButton(u32);
#[derive(Component)]
struct NetRippingButton;
#[derive(Component)]
struct PixieButton;
#[derive(Component)]
struct ResetButton;
#[derive(Component)]
struct SpeedButton;
#[derive(Component)]
struct BackButton;
#[derive(Component)]
struct PlayAreaNode;
#[derive(Resource, Default)]
struct SelectedLevel(u32);
#[derive(Resource, Default)]
pub struct PixieCount(u32);
#[derive(Resource, Default)]
struct Cost(u32);
#[derive(Resource, Default)]
struct Score(Option<u32>);
#[derive(Debug, Clone, Component, Reflect)]
pub struct RoadSegment {
    points: (Vec2, Vec2),
    layer: u32,
}

#[derive(Component, Debug)]
struct PointGraphNode(NodeIndex);
#[derive(Component, Debug)]
struct SegmentGraphNodes(NodeIndex, NodeIndex);

#[derive(Default)]
enum Tool {
    #[default]
    LineDrawing,
    NetRipping,
}

#[derive(Resource, Default)]
struct SelectedTool(Tool);

#[derive(Resource, Default)]
struct PathfindingState {
    valid: bool,
    paths: Vec<(PixieFlavor, Entity, Vec<RoadSegment>)>,
    invalid_nodes: Vec<Entity>,
}

#[derive(Component)]
struct TerminusIssueIndicator;

#[derive(Resource, Default)]
struct RoadGraph {
    graph: StableUnGraph<Entity, f32>,
}

#[derive(Resource, Default, Debug)]
struct MousePos {
    world: Vec2,
    window: Vec2,
}
#[derive(Resource, Default, Debug)]
struct MouseSnappedPos(Vec2);

#[derive(Component)]
enum Collider {
    Point(Vec2),
    Segment((Vec2, Vec2)),
}
#[derive(Component)]
struct ColliderLayer(u32);
#[derive(Component)]
struct GameMusic;

const GRID_SIZE: f32 = 48.0;
pub const BOTTOM_BAR_HEIGHT: f32 = 70.0;
const LAYER_TWO_MULTIPLIER: f32 = 2.0;
const LAYER_THREE_MULTIPLIER: f32 = 4.0;

fn tool_button_display_system(
    mut q_text: Query<&mut TextColor>,
    q_button: Query<(&RadioButton, &Children), (Changed<RadioButton>, With<ToolButton>)>,
) {
    for (button, children) in q_button.iter() {
        let mut iter = q_text.iter_many_mut(children);
        while let Some(mut color) = iter.fetch_next() {
            color.0 = if button.selected {
                bevy::color::palettes::css::LIME.into()
            } else {
                theme::UI_LABEL.into()
            };
        }
    }
}

fn tool_button_system(
    mut selected_tool: ResMut<SelectedTool>,
    mut road_state: ResMut<RoadDrawingState>,
    q_interaction_layer: Query<(&Interaction, &LayerButton), Changed<Interaction>>,
    q_interaction_rip: Query<&Interaction, (Changed<Interaction>, With<NetRippingButton>)>,
) {
    for (_, layer_button) in q_interaction_layer
        .iter()
        .filter(|(i, _)| **i == Interaction::Pressed)
    {
        road_state.layer = layer_button.0;
        if !matches!(selected_tool.0, Tool::LineDrawing) {
            selected_tool.0 = Tool::LineDrawing;
        }
    }

    for _ in q_interaction_rip
        .iter()
        .filter(|i| **i == Interaction::Pressed)
    {
        if !matches!(selected_tool.0, Tool::NetRipping) {
            selected_tool.0 = Tool::NetRipping;
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

    let mut ok = true;
    let mut paths = vec![];
    let mut not_ok = vec![];

    for (a_entity, a, a_node) in q_terminuses.iter() {
        for (_, b, b_node) in q_terminuses.iter() {
            for flavor in a.emits.intersection(&b.collects) {
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
    sim_state: Res<SimulationState>,
    mut q_text: Query<(&mut Text, &mut TextColor)>,
    q_pixie_button: Query<&Children, With<PixieButton>>,
) {
    if !pathfinding.is_changed() && !sim_state.is_changed() {
        return;
    }

    for children in q_pixie_button.iter() {
        let mut iter = q_text.iter_many_mut(children);
        while let Some((mut text, mut color)) = iter.fetch_next() {
            if *sim_state == SimulationState::Running {
                text.0 = "NO WAIT STOP".to_string();
            } else {
                text.0 = "RELEASE THE PIXIES".to_string();
                color.0 = if pathfinding.valid {
                    theme::UI_BUTTON_TEXT.into()
                } else {
                    theme::UI_LABEL_BAD.into()
                }
            }
        }
    }
}

fn back_button_system(
    q_interaction: Query<&Interaction, (Changed<Interaction>, With<Button>, With<BackButton>)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for _ in q_interaction.iter().filter(|i| **i == Interaction::Pressed) {
        next_state.set(GameState::LevelSelect);
    }
}

fn pixie_button_system(
    mut commands: Commands,
    mut pixie_count: ResMut<PixieCount>,
    mut sim_state: ResMut<SimulationState>,
    mut road_state: ResMut<RoadDrawingState>,
    pathfinding: Res<PathfindingState>,
    q_interaction: Query<&Interaction, (Changed<Interaction>, With<Button>, With<PixieButton>)>,
    q_emitters: Query<Entity, With<PixieEmitter>>,
    q_pixies: Query<Entity, With<Pixie>>,
    mut q_indicator: Query<(&mut Visibility, &ChildOf), With<TerminusIssueIndicator>>,
) {
    // do nothing while score dialog is shown
    if *sim_state == SimulationState::Finished {
        return;
    }

    for _ in q_interaction.iter().filter(|i| **i == Interaction::Pressed) {
        road_state.drawing = false;
        road_state.segments = vec![];

        if *sim_state == SimulationState::Running {
            // If the sim is ongoing, the button is a cancel button.
            for entity in q_emitters.iter().chain(q_pixies.iter()) {
                commands.entity(entity).despawn();
            }

            *sim_state = SimulationState::NotStarted;
        } else {
            if !pathfinding.valid {
                for (mut visibility, child_of) in q_indicator.iter_mut() {
                    *visibility = if pathfinding.invalid_nodes.contains(&child_of.parent()) {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    }
                }

                return;
            }

            for (mut visible, _) in q_indicator.iter_mut() {
                *visible = Visibility::Hidden;
            }

            let duration = 0.4;
            let total_pixies = 50;

            let mut counts = HashMap::new();
            for (_, start_entity, _) in pathfinding.paths.iter() {
                *counts.entry(start_entity).or_insert(0) += 1;
            }

            let mut is = HashMap::new();

            for (flavor, start_entity, world_path) in pathfinding.paths.iter() {
                let i = is.entry(start_entity).or_insert(0);

                // unwrap: we just inserted these above
                let count = counts.get(start_entity).unwrap();
                let pixies = total_pixies / *count;

                // if we have multiple pixies coming out of the same starting
                // point, stagger their emitters evenly. this prevents some
                // awkward bunching up at the start of the path.

                let mut timer = Timer::from_seconds(duration * *count as f32, TimerMode::Repeating);
                timer.set_elapsed(Duration::from_secs_f32((*i + 1) as f32 * duration));

                commands.spawn((
                    PixieEmitter {
                        flavor: *flavor,
                        path: world_path.clone(),
                        remaining: pixies,
                        timer,
                    },
                    StateScoped(GameState::Playing),
                ));

                *i += 1;
            }

            *sim_state = SimulationState::Running;
        }

        pixie_count.0 = 0;
    }
}

fn reset_button_system(
    mut commands: Commands,
    q_interaction: Query<&Interaction, (Changed<Interaction>, With<Button>, With<ResetButton>)>,
    mut graph: ResMut<RoadGraph>,
    mut pixie_count: ResMut<PixieCount>,
    mut sim_state: ResMut<SimulationState>,
    mut road_state: ResMut<RoadDrawingState>,
    q_road_chunks: Query<Entity, With<RoadSegment>>,
    q_pixies: Query<Entity, With<Pixie>>,
    q_emitters: Query<Entity, With<PixieEmitter>>,
    q_terminuses: Query<Entity, With<Terminus>>,
    mut q_indicator: Query<&mut Visibility, With<TerminusIssueIndicator>>,
) {
    // do nothing while score dialog is shown
    if *sim_state == SimulationState::Finished {
        return;
    }

    for _ in q_interaction.iter().filter(|i| **i == Interaction::Pressed) {
        for chunk in q_road_chunks
            .iter()
            .chain(q_pixies.iter())
            .chain(q_emitters.iter())
        {
            commands.entity(chunk).despawn();
        }

        for mut visibility in q_indicator.iter_mut() {
            *visibility = Visibility::Hidden;
        }

        graph.graph.clear();

        // we just nuked the graph, but left the start/end points
        // so we need to overwrite their old nodes with new ones.
        for entity in q_terminuses.iter() {
            let node = graph.graph.add_node(entity);
            commands.entity(entity).insert(PointGraphNode(node));
        }

        road_state.drawing = false;
        road_state.segments = vec![];

        *sim_state = SimulationState::default();

        pixie_count.0 = 0;
    }
}

fn speed_button_system(
    q_interaction: Query<
        (&Interaction, &Children),
        (Changed<Interaction>, With<Button>, With<SpeedButton>),
    >,
    mut q_text: Query<&mut Text>,
    mut simulation_settings: ResMut<SimulationSettings>,
) {
    for (_, children) in q_interaction
        .iter()
        .filter(|(i, _)| **i == Interaction::Pressed)
    {
        simulation_settings.speed = simulation_settings.speed.next();

        let mut iter = q_text.iter_many_mut(children);
        while let Some(mut text) = iter.fetch_next() {
            text.0 = simulation_settings.speed.label();
        }
    }
}

fn snap_to_grid(position: Vec2, grid_size: f32) -> Vec2 {
    (position / grid_size).round() * grid_size
}

fn draw_cursor_system(
    mut commands: Commands,
    line_drawing: Res<RoadDrawingState>,
    mouse_snapped: Res<MouseSnappedPos>,
    q_cursor: Query<Entity, With<Cursor>>,
) {
    if mouse_snapped.is_changed() || line_drawing.is_changed() {
        for entity in q_cursor.iter() {
            commands.entity(entity).despawn();
        }
        let shape = shapes::Circle {
            radius: 5.5,
            ..default()
        };
        let color = if line_drawing.drawing && line_drawing.valid {
            theme::DRAWING_ROAD[line_drawing.layer as usize - 1]
        } else if !line_drawing.drawing && line_drawing.valid {
            theme::UI_LABEL
        } else {
            bevy::color::palettes::css::RED
        };
        commands.spawn((
            ShapeBuilder::with(&shape).stroke((color, 2.0)).build(),
            Transform::from_translation(mouse_snapped.0.extend(layer::CURSOR)),
            Cursor,
            StateScoped(GameState::Playing),
        ));
    }
}

fn drawing_mode_change_system(
    selected_tool: Res<SelectedTool>,
    mut road_state: ResMut<RoadDrawingState>,
    mut ripping_state: ResMut<NetRippingState>,
) {
    if !selected_tool.is_changed() {
        return;
    }

    match selected_tool.0 {
        Tool::LineDrawing => {
            ripping_state.reset();
        }
        Tool::NetRipping => {
            road_state.drawing = false;
            road_state.segments = vec![];
        }
    }
}

fn keyboard_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut road_state: ResMut<RoadDrawingState>,
    mut selected_tool: ResMut<SelectedTool>,
    levels: Res<Assets<Level>>,
    selected_level: Res<SelectedLevel>,
    handles: Res<Handles>,
    mut q_radio_button: Query<&mut RadioButton>,
    q_layer_button: Query<(Entity, &LayerButton)>,
    q_net_ripping_button: Query<Entity, With<NetRippingButton>>,
) {
    if !keyboard_input.is_changed() {
        return;
    }

    if keyboard_input.pressed(KeyCode::Digit1)
        || keyboard_input.pressed(KeyCode::Digit2)
        || keyboard_input.pressed(KeyCode::Digit3)
    {
        let layer = if keyboard_input.pressed(KeyCode::Digit1) {
            1
        } else if keyboard_input.pressed(KeyCode::Digit2) {
            2
        } else {
            3
        };

        let level = levels
            .get(&handles.levels[selected_level.0 as usize - 1])
            .unwrap();

        if layer <= level.layers {
            if !matches!(selected_tool.0, Tool::LineDrawing) {
                selected_tool.0 = Tool::LineDrawing;
            }

            road_state.layer = layer;

            for (ent, _) in q_layer_button
                .iter()
                .filter(|(_, layer_button)| layer_button.0 == layer)
            {
                if let Ok(mut radio) = q_radio_button.get_mut(ent) {
                    radio.selected = true;
                }
            }
        }
    } else if keyboard_input.pressed(KeyCode::Escape) {
        if matches!(selected_tool.0, Tool::NetRipping) {
            selected_tool.0 = Tool::LineDrawing;
        } else {
            road_state.drawing = false;
            road_state.segments = vec![];
        }
    } else if keyboard_input.pressed(KeyCode::KeyR) {
        if !matches!(selected_tool.0, Tool::NetRipping) {
            selected_tool.0 = Tool::NetRipping;
        }

        if let Ok(ent) = q_net_ripping_button.single() {
            if let Ok(mut radio) = q_radio_button.get_mut(ent) {
                radio.selected = true;
            }
        }
    }
}

fn mouse_movement_system(
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut mouse: ResMut<MousePos>,
    mut mouse_snapped: ResMut<MouseSnappedPos>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    let Ok((camera, camera_transform)) = q_camera.single() else {
        return;
    };

    for event in cursor_moved_events.read() {
        if let Ok(pos) = camera.viewport_to_world_2d(camera_transform, event.position) {
            mouse.world = pos;

            let new_snapped = snap_to_grid(mouse.world, GRID_SIZE);
            if mouse_snapped.bypass_change_detection().0 != new_snapped {
                debug!("Cursor: {new_snapped}");
                mouse_snapped.0 = new_snapped;
            }

            mouse.window = event.position;
        }
    }
}

fn update_pixie_count_text_system(
    pixie_count: Res<PixieCount>,
    mut query: Query<&mut Text, With<PixieCountText>>,
) {
    if !pixie_count.is_changed() {
        return;
    }

    let Ok(mut text) = query.single_mut() else {
        return;
    };

    text.0 = format!("₽{}", pixie_count.0);
}

fn spawn_road_segment(
    commands: &mut Commands,
    graph: &mut RoadGraph,
    segment: RoadSegment,
) -> (Entity, NodeIndex, NodeIndex) {
    let color = theme::FINISHED_ROAD[segment.layer as usize - 1];
    let ent = commands
        .spawn((
            ShapeBuilder::with(&shapes::Line(segment.points.0, segment.points.1))
                .stroke((color, 2.0))
                .build(),
            Transform::from_xyz(0.0, 0.0, layer::ROAD - segment.layer as f32),
            segment.clone(),
            StateScoped(GameState::Playing),
        ))
        .with_children(|parent| {
            parent.spawn((
                Collider::Segment(segment.points),
                ColliderLayer(segment.layer),
            ));
        })
        .id();

    let start_node = graph.graph.add_node(ent);
    let end_node = graph.graph.add_node(ent);

    graph.graph.add_edge(
        start_node,
        end_node,
        (segment.points.0 - segment.points.1).length(),
    );
    commands
        .entity(ent)
        .insert(SegmentGraphNodes(start_node, end_node));

    (ent, start_node, end_node)
}

fn spawn_obstacle(commands: &mut Commands, obstacle: &Obstacle) {
    match obstacle {
        Obstacle::Rect(top_left, bottom_right) => {
            let diff = *bottom_right - *top_left;
            let origin = (*top_left + *bottom_right) / 2.0;

            commands
                .spawn((
                    ShapeBuilder::with(&shapes::Rectangle {
                        extents: Vec2::new(diff.x.abs(), diff.y.abs()),
                        ..default()
                    })
                    .fill(theme::OBSTACLE)
                    .build(),
                    Transform::from_translation(origin.extend(layer::OBSTACLE)),
                    StateScoped(GameState::Playing),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Collider::Segment((
                            Vec2::new(top_left.x, top_left.y),
                            Vec2::new(bottom_right.x, top_left.y),
                        )),
                        ColliderLayer(0),
                    ));
                    parent.spawn((
                        Collider::Segment((
                            Vec2::new(bottom_right.x, top_left.y),
                            Vec2::new(bottom_right.x, bottom_right.y),
                        )),
                        ColliderLayer(0),
                    ));
                    parent.spawn((
                        Collider::Segment((
                            Vec2::new(bottom_right.x, bottom_right.y),
                            Vec2::new(top_left.x, bottom_right.y),
                        )),
                        ColliderLayer(0),
                    ));
                    parent.spawn((
                        Collider::Segment((
                            Vec2::new(top_left.x, bottom_right.y),
                            Vec2::new(top_left.x, top_left.y),
                        )),
                        ColliderLayer(0),
                    ));
                });
        }
    }
}

fn spawn_name(
    commands: &mut Commands,
    number: u32,
    handles: &Res<Handles>,
    name: &String,
    name_position: &Vec2,
) {
    commands.spawn((
        Text2d::new(format!("/l{}/{}.pcb", number, name)),
        TextFont {
            font: handles.fonts[0].clone(),
            font_size: 25.0,
            ..default()
        },
        TextColor(theme::LEVEL_NAME.into()),
        Anchor::TopLeft,
        Transform::from_translation((name_position + Vec2::new(8., -8.)).extend(layer::GRID)),
        StateScoped(GameState::Playing),
    ));
}

fn spawn_terminus(
    commands: &mut Commands,
    graph: &mut ResMut<RoadGraph>,
    handles: &Res<Handles>,
    terminus: &Terminus,
) -> (Entity, NodeIndex) {
    let label_offset = 22.0;
    let label_spacing = 22.0;

    let ent = commands
        .spawn((
            ShapeBuilder::with(&shapes::Circle {
                radius: 5.5,
                ..default()
            })
            .fill(theme::BACKGROUND)
            .stroke((theme::FINISHED_ROAD[0], 2.0))
            .build(),
            Transform::from_translation(terminus.point.extend(layer::TERMINUS)),
            terminus.clone(),
            StateScoped(GameState::Playing),
        ))
        .with_children(|parent| {
            parent.spawn((Collider::Point(terminus.point), ColliderLayer(1)));

            let mut i = 0;

            for flavor in terminus.emits.iter() {
                let label_pos =
                    Vec2::new(0.0, -1.0 * label_offset + -1.0 * i as f32 * label_spacing);

                let label = if flavor.net > 0 {
                    format!("OUT.{}", flavor.net + 1)
                } else {
                    "OUT".to_string()
                };

                parent.spawn((
                    Text2d::new(label),
                    TextFont {
                        font: handles.fonts[0].clone(),
                        font_size: 25.0,
                        ..default()
                    },
                    TextColor(theme::PIXIE[flavor.color as usize].into()),
                    TextLayout::new_with_justify(JustifyText::Center),
                    Transform::from_translation(label_pos.extend(layer::TERMINUS)),
                ));

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

                parent.spawn((
                    Text2d::new(label),
                    TextFont {
                        font: handles.fonts[0].clone(),
                        font_size: 25.0,
                        ..default()
                    },
                    TextColor(theme::PIXIE[flavor.color as usize].into()),
                    TextLayout::new_with_justify(JustifyText::Center),
                    Transform::from_translation(label_pos.extend(layer::TERMINUS)),
                ));

                i += 1;
            }

            // TODO above code supports multiple emitters/collectors, but below
            // assumes a single emitter.

            parent.spawn((
                ShapeBuilder::with(&shapes::Circle {
                    radius: 5.5,
                    ..default()
                })
                .fill(bevy::color::palettes::css::RED)
                .build(),
                Transform::from_xyz(-30.0, -1.0 * label_offset, layer::TERMINUS),
                Visibility::Hidden,
                TerminusIssueIndicator,
            ));
        })
        .id();

    let node = graph.graph.add_node(ent);

    commands.entity(ent).insert(PointGraphNode(node));

    (ent, node)
}

fn update_cost_system(
    graph: Res<RoadGraph>,
    line_draw: Res<RoadDrawingState>,
    mut r_cost: ResMut<Cost>,
    q_segments: Query<(&RoadSegment, &Children)>,
    q_colliders: Query<&ColliderLayer>,
    mut q_cost: Query<Entity, With<CostText>>,
    mut writer: TextUiWriter,
) {
    if !graph.is_changed() && !line_draw.is_changed() {
        return;
    }

    let mut cost = 0.0;

    for (segment, children) in q_segments.iter() {
        let Some(child) = children.first() else {
            continue;
        };

        let Ok(layer) = q_colliders.get(*child) else {
            continue;
        };

        let multiplier = if layer.0 == 1 {
            LAYER_TWO_MULTIPLIER
        } else if layer.0 == 2 {
            LAYER_THREE_MULTIPLIER
        } else {
            1.0
        };

        cost += (segment.points.0 - segment.points.1).length() * multiplier;
    }

    cost /= GRID_SIZE;
    let cost_round = cost.ceil();

    r_cost.0 = cost as u32;

    let mut potential_cost = 0.0;
    if line_draw.valid {
        for segment in line_draw.segments.iter() {
            let multiplier = if line_draw.layer == 1 {
                LAYER_TWO_MULTIPLIER
            } else if line_draw.layer == 2 {
                LAYER_THREE_MULTIPLIER
            } else {
                1.0
            };
            potential_cost += (segment.0 - segment.1).length() * multiplier;
        }
    }

    potential_cost /= GRID_SIZE;
    let potential_cost_round = (cost + potential_cost).ceil() - cost_round;

    for entity in q_cost.iter_mut() {
        *writer.text(entity, 1) = format!("§{cost_round}");
        if potential_cost_round > 0.0 {
            *writer.text(entity, 2) = format!("+{potential_cost_round}");
        } else {
            *writer.text(entity, 2) = "".to_string();
        }
        *writer.color(entity, 2) = theme::FINISHED_ROAD[line_draw.layer as usize - 1].into();
    }
}

fn update_score_system(
    pixie_count: Res<PixieCount>,
    sim_state: Res<SimulationState>,
    sim_steps: Res<SimulationSteps>,
    mut score: ResMut<Score>,
    mut best_scores: ResMut<BestScores>,
    selected_level: Res<SelectedLevel>,
    cost: Res<Cost>,
) {
    if !sim_state.is_changed() {
        return;
    }

    if *sim_state != SimulationState::Finished {
        return;
    }

    let elapsed = sim_steps.get_elapsed_f32();

    let val = ((pixie_count.0 as f32 / cost.0 as f32 / elapsed) * 10000.0).ceil() as u32;

    score.0 = Some(val);

    if let Some(best) = best_scores.0.get_mut(&selected_level.0) {
        if *best < val {
            *best = val;
        }
    } else {
        best_scores.0.insert(selected_level.0, val);
    }
}

fn update_score_text_system(
    selected_level: Res<SelectedLevel>,
    best_scores: Res<BestScores>,
    mut q_score_text: Query<&mut Text, With<ScoreText>>,
) {
    if !best_scores.is_changed() && !selected_level.is_changed() {
        return;
    }

    if let Some(mut text) = q_score_text.iter_mut().next() {
        if let Some(best) = best_scores.0.get(&selected_level.0) {
            text.0 = format!("Æ{best}");
        } else {
            text.0 = "Æ?".to_string();
        }
    }
}

fn update_elapsed_text_system(
    sim_steps: Res<SimulationSteps>,
    mut q_text: Query<&mut Text, With<ElapsedText>>,
) {
    if !sim_steps.is_changed() {
        return;
    }

    for mut text in q_text.iter_mut() {
        text.0 = format!("ŧ{:.1}", sim_steps.get_elapsed_f32());
    }
}

fn save_solution_system(
    query: Query<&RoadSegment>,
    graph: Res<RoadGraph>,
    level: Res<SelectedLevel>,
    mut solutions: ResMut<Solutions>,
) {
    if !graph.is_changed() {
        return;
    }

    // TODO this saves the prefs unnecessarily when
    // the graph is modified after a particular level
    // is loaded.

    let segments = query.iter().cloned().collect();
    solutions.0.insert(level.0, Solution { segments });
}

fn reset_game(mut commands: Commands, mut graph: ResMut<RoadGraph>) {
    commands.insert_resource(Score::default());
    commands.insert_resource(PixieCount::default());
    commands.insert_resource(Cost::default());
    commands.insert_resource(SelectedTool::default());
    commands.insert_resource(RoadDrawingState::default());
    commands.insert_resource(NetRippingState::default());
    commands.insert_resource(SimulationState::default());
    commands.insert_resource(PathfindingState::default());
    graph.graph.clear();
}

fn spawn_level(
    mut commands: Commands,
    mut graph: ResMut<RoadGraph>,
    levels: Res<Assets<Level>>,
    selected_level: Res<SelectedLevel>,
    handles: Res<Handles>,
    solutions: Res<Solutions>,
) {
    // Build arena

    for x in ((-25 * (GRID_SIZE as i32))..=25 * (GRID_SIZE as i32)).step_by(GRID_SIZE as usize) {
        for y in (-15 * (GRID_SIZE as i32)..=15 * (GRID_SIZE as i32)).step_by(GRID_SIZE as usize) {
            commands.spawn((
                ShapeBuilder::with(&shapes::Circle {
                    radius: 2.5,
                    ..default()
                })
                .fill(theme::GRID)
                .build(),
                Transform::from_xyz(x as f32, y as f32, layer::GRID),
                GridPoint,
                StateScoped(GameState::Playing),
            ));
        }
    }

    // Build level

    let mut connections: Vec<(Vec2, NodeIndex)> = vec![];

    let level = levels
        .get(&handles.levels[selected_level.0 as usize - 1])
        .unwrap();

    for t in level.terminuses.iter() {
        let (_, node) = spawn_terminus(&mut commands, &mut graph, &handles, t);
        connections.push((t.point, node));
    }

    for o in level.obstacles.iter() {
        spawn_obstacle(&mut commands, o);
    }

    spawn_name(
        &mut commands,
        selected_level.0,
        &handles,
        &level.name,
        &level.name_position,
    );

    println!(
        "{:?}",
        Dot::with_config(&graph.graph, &[Config::EdgeNoLabel])
    );

    // Spawn previous solution to level

    if let Some(solution) = solutions.0.get(&selected_level.0) {
        for seg in solution.segments.iter() {
            let (_, node_a, node_b) = spawn_road_segment(&mut commands, &mut graph, seg.clone());

            for (point, node) in connections.iter() {
                if *point == seg.points.0 {
                    graph.graph.add_edge(*node, node_a, 0.0);
                }

                if *point == seg.points.1 {
                    graph.graph.add_edge(*node, node_b, 0.0);
                }
            }

            connections.push((seg.points.0, node_a));
            connections.push((seg.points.1, node_b));
        }
    }

    // Build UI
}

fn spawn_music(mut commands: Commands, handles: Res<Handles>, volume: Res<MusicVolume>) {
    if volume.is_muted() {
        return;
    }

    commands.spawn((
        AudioPlayer::new(handles.music.clone()),
        PlaybackSettings::LOOP.with_volume((*volume).into()),
        GameMusic,
    ));
}

fn spawn_game_ui(
    mut commands: Commands,
    simulation_settings: Res<SimulationSettings>,
    levels: Res<Assets<Level>>,
    selected_level: Res<SelectedLevel>,
    handles: Res<Handles>,
) {
    let level = levels
        .get(&handles.levels[selected_level.0 as usize - 1])
        .unwrap();

    let mut tool_button_ids = vec![];

    commands
        .spawn((
            Name::new("GameUiRoot"),
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                flex_direction: FlexDirection::ColumnReverse,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                ..default()
            },
            StateScoped(GameState::Playing),
        ))
        .with_children(|parent| {
            // bottom bar
            parent
                .spawn((
                    Node {
                        padding: UiRect::all(Val::Px(10.0)),
                        width: Val::Percent(100.),
                        height: Val::Px(BOTTOM_BAR_HEIGHT),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Stretch,
                        column_gap: Val::Px(10.),
                        ..default()
                    },
                    BackgroundColor(theme::UI_PANEL_BACKGROUND.into()),
                ))
                .with_children(|parent| {
                    // Container for left-aligned buttons
                    parent
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Stretch,
                            column_gap: Val::Px(10.),
                            ..default()
                        })
                        .with_children(|parent| {
                            // Back button
                            parent
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(50.),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        // extra padding to separate the back button from
                                        // the tools
                                        margin: UiRect {
                                            right: Val::Px(10.0),
                                            ..default()
                                        },
                                        ..default()
                                    },
                                    BackgroundColor(theme::UI_NORMAL_BUTTON.into()),
                                    BackButton,
                                ))
                                .with_children(|parent| {
                                    parent.spawn((
                                        Text::new("←"),
                                        TextFont {
                                            font: handles.fonts[0].clone(),
                                            font_size: 25.0,
                                            ..default()
                                        },
                                        TextColor(theme::UI_BUTTON_TEXT.into()),
                                    ));
                                });

                            // Tool Buttons

                            for layer in 1..=level.layers {
                                let id = parent
                                    .spawn((
                                        Button,
                                        Node {
                                            width: Val::Px(50.),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            ..default()
                                        },
                                        BackgroundColor(theme::UI_NORMAL_BUTTON.into()),
                                        LayerButton(layer),
                                        ToolButton,
                                        RadioButton {
                                            selected: layer == 1,
                                        },
                                    ))
                                    .with_children(|parent| {
                                        parent.spawn((
                                            Text::new(format!("{layer}")),
                                            TextFont {
                                                font: handles.fonts[0].clone(),
                                                font_size: 25.0,
                                                ..default()
                                            },
                                            TextColor(theme::UI_BUTTON_TEXT.into()),
                                        ));
                                    })
                                    .id();

                                tool_button_ids.push(id);
                            }

                            let net_ripping_id = parent
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(50.),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(theme::UI_NORMAL_BUTTON.into()),
                                    NetRippingButton,
                                    ToolButton,
                                    RadioButton { selected: false },
                                ))
                                .with_children(|parent| {
                                    parent.spawn((
                                        Text::new("R"),
                                        TextFont {
                                            font: handles.fonts[0].clone(),
                                            font_size: 25.0,
                                            ..default()
                                        },
                                        TextColor(theme::UI_BUTTON_TEXT.into()),
                                    ));
                                })
                                .id();

                            tool_button_ids.push(net_ripping_id);
                        });

                    // Container for score, etc

                    parent
                        .spawn(Node {
                            flex_grow: 1.,
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(10.),
                            ..default()
                        })
                        .with_children(|parent| {
                            parent
                                .spawn((
                                    Text::default(),
                                    // See Bevy#16521
                                    TextFont {
                                        font: handles.fonts[0].clone(),
                                        ..default()
                                    },
                                    Node {
                                        width: Val::Percent(25.),
                                        ..default()
                                    },
                                    CostText,
                                ))
                                .with_children(|parent| {
                                    parent.spawn((
                                        TextSpan::new("0".to_string()),
                                        TextFont {
                                            font: handles.fonts[0].clone(),
                                            font_size: 25.0,
                                            ..default()
                                        },
                                        TextColor(theme::UI_LABEL.into()),
                                    ));
                                    parent.spawn((
                                        TextSpan::default(),
                                        TextFont {
                                            font: handles.fonts[0].clone(),
                                            font_size: 25.0,
                                            ..default()
                                        },
                                        TextColor(theme::PIXIE[0].into()),
                                    ));
                                });

                            parent.spawn((
                                Text::new("0"),
                                TextFont {
                                    font: handles.fonts[0].clone(),
                                    font_size: 25.0,
                                    ..default()
                                },
                                TextColor(theme::PIXIE[1].into()),
                                Node {
                                    width: Val::Percent(25.),
                                    ..default()
                                },
                                PixieCountText,
                            ));

                            parent.spawn((
                                Text::new("ŧ0.0"),
                                TextFont {
                                    font: handles.fonts[0].clone(),
                                    font_size: 25.0,
                                    ..default()
                                },
                                TextColor(theme::PIXIE[2].into()),
                                Node {
                                    width: Val::Percent(25.),
                                    ..default()
                                },
                                ElapsedText,
                            ));

                            parent.spawn((
                                Text::new("Æ?"),
                                TextFont {
                                    font: handles.fonts[0].clone(),
                                    font_size: 25.0,
                                    ..default()
                                },
                                TextColor(theme::FINISHED_ROAD[1].into()),
                                Node {
                                    width: Val::Percent(25.),
                                    ..default()
                                },
                                ScoreText,
                            ));
                        });

                    // Container for right-aligned bar items

                    parent
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::FlexEnd,
                            align_items: AlignItems::Stretch,
                            column_gap: Val::Px(10.),
                            ..default()
                        })
                        .with_children(|parent| {
                            parent
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(110.),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(theme::UI_NORMAL_BUTTON.into()),
                                    ResetButton,
                                ))
                                .with_children(|parent| {
                                    parent.spawn((
                                        Text::new("RESET"),
                                        TextFont {
                                            font: handles.fonts[0].clone(),
                                            font_size: 25.0,
                                            ..default()
                                        },
                                        TextColor(theme::UI_BUTTON_TEXT.into()),
                                    ));
                                });
                            parent
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(50.),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(theme::UI_NORMAL_BUTTON.into()),
                                    SpeedButton,
                                ))
                                .with_children(|parent| {
                                    parent.spawn((
                                        Text::new(simulation_settings.speed.label()),
                                        TextFont {
                                            font: handles.fonts[0].clone(),
                                            font_size: 25.0,
                                            ..default()
                                        },
                                        TextColor(theme::UI_BUTTON_TEXT.into()),
                                    ));
                                });
                            parent
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(250.),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(theme::UI_NORMAL_BUTTON.into()),
                                    PixieButton,
                                ))
                                .with_children(|parent| {
                                    parent.spawn((
                                        Text::new("RELEASE THE PIXIES"),
                                        TextFont {
                                            font: handles.fonts[0].clone(),
                                            font_size: 25.0,
                                            ..default()
                                        },
                                        TextColor(theme::UI_BUTTON_TEXT.into()),
                                    ));
                                });
                        });
                });

            // the rest of the space over the play area
            parent.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                PlayAreaNode,
            ));
        });

    let tool_group_id = commands
        .spawn(RadioButtonGroup {
            entities: tool_button_ids.clone(),
        })
        .id();

    for id in tool_button_ids.iter() {
        commands
            .entity(*id)
            .insert(RadioButtonGroupRelation(tool_group_id));
    }
}

fn set_music_volume_system(
    volume: Res<MusicVolume>,
    sinks: Query<(&mut AudioSink, Entity), With<GameMusic>>,
    handles: Res<Handles>,
    mut commands: Commands,
) {
    match (volume.is_muted(), sinks.is_empty()) {
        (false, true) => {
            commands.spawn((
                AudioPlayer::new(handles.music.clone()),
                PlaybackSettings::LOOP.with_volume((*volume).into()),
                GameMusic,
            ));
        }
        (true, false) => {
            for (_, entity) in sinks {
                commands.entity(entity).despawn();
            }
        }
        (false, false) => {
            for (mut sink, _) in sinks {
                sink.set_volume((*volume).into());
            }
        }
        (true, true) => {}
    }
}
