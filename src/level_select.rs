use crate::{
    level::Level, pixie::PIXIE_COLORS, save::BestScores, GameState, Handles, UI_GREY_RED_COLOR,
};
use bevy::prelude::*;

pub struct LevelSelectPlugin;
#[derive(Component)]
pub struct LevelSelectScreen;
#[derive(Component)]
pub struct LevelSelectButton(u32);

impl Plugin for LevelSelectPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_enter(GameState::LevelSelect).with_system(level_select_enter),
        );
        app.add_system_set(
            SystemSet::on_update(GameState::LevelSelect)
                .with_system(level_select_update)
                .with_system(crate::button_system)
                .with_system(level_select_button_system),
        );
        app.add_system_set(
            SystemSet::on_exit(GameState::LevelSelect).with_system(level_select_exit),
        );
    }
}

fn level_select_button_system(
    query: Query<(&Interaction, &LevelSelectButton), Changed<Interaction>>,
    mut state: ResMut<State<GameState>>,
    mut level: ResMut<crate::SelectedLevel>,
    handles: Res<Handles>,
    levels: Res<Assets<Level>>,
) {
    for (_, button) in query.iter().filter(|(i, _)| **i == Interaction::Clicked) {
        if handles
            .levels
            .get(button.0 as usize - 1)
            .and_then(|h| levels.get(h))
            .is_none()
        {
            continue;
        };

        level.0 = button.0;
        state.replace(GameState::Playing).unwrap();
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
                    size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceEvenly,
                    ..Default::default()
                },
                ..Default::default()
            },
            LevelSelectScreen,
        ))
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent.spawn(TextBundle {
                        style: Style {
                            align_self: AlignSelf::Center,
                            ..Default::default()
                        },
                        text: Text::from_section(
                            "₽IXIE WRANGLER",
                            TextStyle {
                                font: handles.fonts[0].clone(),
                                font_size: 60.0,
                                color: PIXIE_COLORS[1],
                            },
                        ),
                        ..Default::default()
                    });
                    parent.spawn(TextBundle {
                        style: Style {
                            align_self: AlignSelf::Center,
                            ..Default::default()
                        },
                        text: Text::from_section(
                            format!("Æ{}", total_score),
                            TextStyle {
                                font: handles.fonts[0].clone(),
                                font_size: 30.0,
                                color: crate::FINISHED_ROAD_COLORS[1],
                            },
                        ),
                        ..Default::default()
                    });
                });

            parent
                .spawn(NodeBundle {
                    style: Style {
                        size: Size::new(Val::Percent(100.0), Val::Auto),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with_children(|parent| {
                    let rows = 3;
                    let cols = 3;

                    for row in 0..rows {
                        parent
                            .spawn(NodeBundle {
                                style: Style {
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    ..Default::default()
                                },
                                ..Default::default()
                            })
                            .with_children(|parent| {
                                for col in 0..cols {
                                    let i = row * cols + col + 1;
                                    parent
                                        .spawn((
                                            ButtonBundle {
                                                style: Style {
                                                    size: Size::new(Val::Px(150.0), Val::Px(150.0)),
                                                    flex_direction: FlexDirection::Column,
                                                    // horizontally center child text
                                                    justify_content: JustifyContent::Center,
                                                    // vertically center child text
                                                    align_items: AlignItems::Center,
                                                    margin: UiRect {
                                                        left: Val::Px(10.0),
                                                        bottom: Val::Px(10.0),
                                                        ..Default::default()
                                                    },
                                                    ..Default::default()
                                                },
                                                background_color: crate::NORMAL_BUTTON.into(),
                                                ..Default::default()
                                            },
                                            LevelSelectButton(i),
                                        ))
                                        .with_children(|parent| {
                                            let level = handles
                                                .levels
                                                .get(i as usize - 1)
                                                .and_then(|h| levels.get(h));

                                            let level_color = match level {
                                                Some(_) => crate::UI_WHITE_COLOR,
                                                None => UI_GREY_RED_COLOR,
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
                                                        format!("Æ{}", score),
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
                                                                color: crate::UI_WHITE_COLOR,
                                                            },
                                                        },
                                                        TextSection {
                                                            value: star_text_two,
                                                            style: TextStyle {
                                                                font: handles.fonts[0].clone(),
                                                                font_size: 30.0,
                                                                color: Color::DARK_GRAY,
                                                            },
                                                        },
                                                    ],
                                                    ..Default::default()
                                                },
                                                ..Default::default()
                                            });

                                            parent.spawn(TextBundle {
                                                text: Text::from_section(
                                                    format!("{}", i),
                                                    TextStyle {
                                                        font: handles.fonts[0].clone(),
                                                        font_size: 60.0,
                                                        color: level_color,
                                                    },
                                                ),
                                                ..Default::default()
                                            });

                                            parent.spawn(TextBundle {
                                                text: Text::from_section(
                                                    score_text,
                                                    TextStyle {
                                                        font: handles.fonts[0].clone(),
                                                        font_size: 30.0,
                                                        color: crate::FINISHED_ROAD_COLORS[1],
                                                    },
                                                ),
                                                ..Default::default()
                                            });
                                        });
                                }
                            });
                    }
                });
        });
}

fn level_select_update() {}

fn level_select_exit(
    mut commands: Commands,
    query: Query<Entity, With<LevelSelectScreen>>,
    mut mouse: ResMut<Input<MouseButton>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }

    mouse.reset(MouseButton::Left);
    mouse.clear();
}
