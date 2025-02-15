use std::time::Duration;

use bevy::prelude::*;
use bevy_easings::{Ease, EaseFunction, *};

use crate::{
    level::Level, pixie::PixieEmitter, sim::SimulationState, theme, AfterUpdate, BackButton,
    DrawingInteraction, GameState, Handles, PixieCount, PlayAreaNode, Score, ScoreUi,
    SelectedLevel,
};

pub struct ScoreDialogPlugin;

impl Plugin for ScoreDialogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            dismiss_score_dialog_button_system
                .after(DrawingInteraction)
                .run_if(in_state(GameState::Playing)),
        );

        app.add_systems(AfterUpdate, show_score_dialog_system.in_set(ScoreUi));
    }
}

#[derive(Component)]
struct DismissScoreDialogButton;
#[derive(Component)]
struct ScoreDialog;

fn show_score_dialog_system(
    mut commands: Commands,
    sim_state: Res<SimulationState>,
    handles: Res<Handles>,
    selected_level: Res<SelectedLevel>,
    levels: Res<Assets<Level>>,
    score: Res<Score>,
    mut q_node: Query<(Entity, &mut BackgroundColor), With<PlayAreaNode>>,
    q_dialog: Query<Entity, With<ScoreDialog>>,
) {
    if !sim_state.is_changed() && !score.is_changed() {
        return;
    }

    if *sim_state != SimulationState::Finished {
        return;
    }

    if q_dialog.get_single().is_ok() {
        return;
    }

    let Some(level) = handles
        .levels
        .get(selected_level.0 as usize - 1)
        .and_then(|h| levels.get(h))
    else {
        return;
    };

    let Some(score) = score.0 else { return };

    let num_stars = level
        .star_thresholds
        .iter()
        .filter(|t| **t <= score)
        .count();

    let dialog_node = Node {
        width: Val::Px(320.0),
        height: Val::Px(300.0),
        margin: UiRect {
            top: Val::Px(-1000.0),
            ..default()
        },
        padding: UiRect::all(Val::Px(20.0)),
        flex_direction: FlexDirection::Column,
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Center,
        ..default()
    };
    let mut dialog_node_to = dialog_node.clone();
    dialog_node_to.margin.top = Val::Px(0.0);

    let dialog_entity = commands
        .spawn((
            dialog_node.clone(),
            dialog_node.ease_to(
                dialog_node_to,
                EaseFunction::QuadraticInOut,
                EasingType::Once {
                    duration: Duration::from_secs_f32(0.7),
                },
            ),
            BackgroundColor(theme::UI_PANEL_BACKGROUND.into()),
            ScoreDialog,
        ))
        .with_children(|parent| {
            parent.spawn(Text::default()).with_children(|parent| {
                parent.spawn((
                    Text::new("★".repeat(num_stars)),
                    TextFont {
                        font: handles.fonts[0].clone(),
                        font_size: 83.0,
                        ..default()
                    },
                    TextColor(theme::UI_LABEL.into()),
                ));

                parent.spawn((
                    Text::new("★".repeat(3 - num_stars)),
                    TextFont {
                        font: handles.fonts[0].clone(),
                        font_size: 83.0,
                        ..default()
                    },
                    TextColor(Srgba::gray(0.25).into()),
                ));
            });

            parent.spawn((
                Text::new(format!("Æ{score}")),
                TextFont {
                    font: handles.fonts[0].clone(),
                    font_size: 83.0,
                    ..default()
                },
                TextColor(theme::FINISHED_ROAD[1].into()),
            ));

            // bottom buttons
            parent
                .spawn(Node {
                    width: Val::Percent(100.),
                    height: Val::Px(70.),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Stretch,
                    column_gap: Val::Px(10.),
                    ..default()
                })
                .with_children(|parent| {
                    parent
                        .spawn((
                            Button,
                            Node {
                                flex_grow: 1.,
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(theme::UI_NORMAL_BUTTON.into()),
                            DismissScoreDialogButton,
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("DISMISS"),
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
                                flex_grow: 1.,
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(theme::UI_NORMAL_BUTTON.into()),
                            BackButton,
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("ONWARD →"),
                                TextFont {
                                    font: handles.fonts[0].clone(),
                                    font_size: 25.0,
                                    ..default()
                                },
                                TextColor(theme::UI_BUTTON_TEXT.into()),
                            ));
                        });
                });
        })
        .id();
    if let Ok((entity, mut color)) = q_node.get_single_mut() {
        commands.entity(entity).add_children(&[dialog_entity]);
        *color = theme::DARK_OVERLAY.into();
    }
}

fn dismiss_score_dialog_button_system(
    mut commands: Commands,
    mut sim_state: ResMut<SimulationState>,
    mut pixie_count: ResMut<PixieCount>,
    q_interaction: Query<
        &Interaction,
        (
            Changed<Interaction>,
            With<Button>,
            With<DismissScoreDialogButton>,
        ),
    >,
    q_dialog: Query<Entity, With<ScoreDialog>>,
    q_emitters: Query<Entity, With<PixieEmitter>>,
    mut q_node: Query<&mut BackgroundColor, With<PlayAreaNode>>,
    mut score: ResMut<Score>,
) {
    for _ in q_interaction.iter().filter(|i| **i == Interaction::Pressed) {
        if let Ok(entity) = q_dialog.get_single() {
            commands.entity(entity).despawn_recursive();
            *sim_state = SimulationState::default();
            *pixie_count = PixieCount::default();
            *score = Score::default();
        }

        for entity in q_emitters.iter() {
            commands.entity(entity).despawn();
        }

        if let Ok(mut color) = q_node.get_single_mut() {
            *color = Color::NONE.into();
        }
    }
}
