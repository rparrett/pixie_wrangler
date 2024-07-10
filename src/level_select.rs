use crate::{color, level::Level, loading::NUM_LEVELS, save::BestScores, GameState, Handles};
use bevy::prelude::*;

pub struct LevelSelectPlugin;
#[derive(Component)]
pub struct LevelSelectScreen;
#[derive(Component)]
pub struct LevelSelectButton(u32);

impl Plugin for LevelSelectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::LevelSelect), level_select_enter);

        app.add_systems(
            Update,
            (
                level_select_update,
                crate::button_system,
                level_select_button_system,
            )
                .run_if(in_state(GameState::LevelSelect)),
        );

        app.add_systems(OnExit(GameState::LevelSelect), level_select_exit);
    }
}

fn level_select_button_system(
    query: Query<(&Interaction, &LevelSelectButton), Changed<Interaction>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut level: ResMut<crate::SelectedLevel>,
    handles: Res<Handles>,
    levels: Res<Assets<Level>>,
) {
    for (_, button) in query.iter().filter(|(i, _)| **i == Interaction::Pressed) {
        if handles
            .levels
            .get(button.0 as usize - 1)
            .and_then(|h| levels.get(h))
            .is_none()
        {
            continue;
        };

        level.0 = button.0;
        next_state.set(GameState::Playing);
    }
}

fn level_select_enter(
    mut commands: Commands,
    best_scores: Res<BestScores>,
    handles: Res<Handles>,
    levels: Res<Assets<Level>>,
) {
    let total_score: u32 = best_scores.0.iter().map(|(_, v)| v).sum();

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceEvenly,
                    ..default()
                },
                ..default()
            },
            LevelSelectScreen,
        ))
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(TextBundle {
                        style: Style {
                            align_self: AlignSelf::Center,
                            ..default()
                        },
                        text: Text::from_section(
                            "₽IXIE WRANGLER",
                            TextStyle {
                                font: handles.fonts[0].clone(),
                                font_size: 60.0,
                                color: color::PIXIE[1].into(),
                            },
                        ),
                        ..default()
                    });
                    parent.spawn(TextBundle {
                        style: Style {
                            align_self: AlignSelf::Center,
                            ..default()
                        },
                        text: Text::from_section(
                            format!("Æ{total_score}"),
                            TextStyle {
                                font: handles.fonts[0].clone(),
                                font_size: 30.0,
                                color: color::FINISHED_ROAD[1],
                            },
                        ),
                        ..default()
                    });
                });

            let cols = (NUM_LEVELS as f32 / 3.).ceil() as u16;

            parent
                .spawn(NodeBundle {
                    style: Style {
                        display: Display::Grid,
                        grid_template_rows: RepeatedGridTrack::auto(3),
                        grid_template_columns: RepeatedGridTrack::auto(cols),
                        row_gap: Val::Px(10.),
                        column_gap: Val::Px(10.),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    for i in 1..=NUM_LEVELS {
                        parent
                            .spawn((
                                ButtonBundle {
                                    style: Style {
                                        width: Val::Px(150.),
                                        height: Val::Px(150.),
                                        flex_direction: FlexDirection::Column,
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    background_color: color::UI_NORMAL_BUTTON.into(),
                                    ..default()
                                },
                                LevelSelectButton(i),
                            ))
                            .with_children(|parent| {
                                let level = handles
                                    .levels
                                    .get(i as usize - 1)
                                    .and_then(|h| levels.get(h));

                                let level_color = match level {
                                    Some(_) => color::UI_WHITE,
                                    None => color::UI_GREY_RED,
                                };

                                let (score_text, star_text_one, star_text_two) =
                                    if let (Some(score), Some(level)) =
                                        (best_scores.0.get(&i), level)
                                    {
                                        let stars = level
                                            .star_thresholds
                                            .iter()
                                            .filter(|t| **t <= *score)
                                            .count();

                                        (
                                            format!("Æ{score}"),
                                            "★".repeat(stars),
                                            "★".repeat(3 - stars),
                                        )
                                    } else {
                                        ("".to_string(), "".to_string(), "".to_string())
                                    };

                                parent.spawn(TextBundle {
                                    text: Text {
                                        sections: vec![
                                            TextSection {
                                                value: star_text_one,
                                                style: TextStyle {
                                                    font: handles.fonts[0].clone(),
                                                    font_size: 30.0,
                                                    color: color::UI_WHITE,
                                                },
                                            },
                                            TextSection {
                                                value: star_text_two,
                                                style: TextStyle {
                                                    font: handles.fonts[0].clone(),
                                                    font_size: 30.0,
                                                    color: Srgba::gray(0.25).into(),
                                                },
                                            },
                                        ],
                                        ..default()
                                    },
                                    ..default()
                                });

                                parent.spawn(TextBundle {
                                    text: Text::from_section(
                                        format!("{i}"),
                                        TextStyle {
                                            font: handles.fonts[0].clone(),
                                            font_size: 60.0,
                                            color: level_color,
                                        },
                                    ),
                                    ..default()
                                });

                                parent.spawn(TextBundle {
                                    text: Text::from_section(
                                        score_text,
                                        TextStyle {
                                            font: handles.fonts[0].clone(),
                                            font_size: 30.0,
                                            color: color::FINISHED_ROAD[1],
                                        },
                                    ),
                                    ..default()
                                });
                            });
                    }
                });
        });
}

fn level_select_update() {}

fn level_select_exit(
    mut commands: Commands,
    query: Query<Entity, With<LevelSelectScreen>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }

    mouse.reset(MouseButton::Left);
    mouse.clear();
}
