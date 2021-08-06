use crate::{level::Level, BestScores, ButtonMaterials, GameState, Handles, UI_GREY_RED_COLOR};
use bevy::prelude::*;

pub struct LevelSelectPlugin;
pub struct LevelSelectScreen;
pub struct LevelSelectButton(u32);

impl Plugin for LevelSelectPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_set(
            SystemSet::on_enter(GameState::LevelSelect).with_system(level_select_enter.system()),
        );
        app.add_system_set(
            SystemSet::on_update(GameState::LevelSelect)
                .with_system(level_select_update.system())
                .with_system(crate::button_system_system.system())
                .with_system(level_select_button_system.system()),
        );
        app.add_system_set(
            SystemSet::on_exit(GameState::LevelSelect).with_system(level_select_exit.system()),
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
            .and_then(|h| levels.get(h.clone()))
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
    mut materials: ResMut<Assets<ColorMaterial>>,
    button_materials: Res<ButtonMaterials>,
    best_efficiencies: Res<BestScores>,
    handles: Res<Handles>,
    levels: Res<Assets<Level>>,
) {
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                flex_direction: FlexDirection::ColumnReverse,
                align_items: AlignItems::FlexStart,
                justify_content: JustifyContent::Center,
                ..Default::default()
            },
            material: materials.add(Color::NONE.into()),
            ..Default::default()
        })
        .insert(LevelSelectScreen)
        .with_children(|parent| {
            parent
                .spawn_bundle(NodeBundle {
                    style: Style {
                        size: Size::new(Val::Percent(100.0), Val::Px(100.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        position: Rect {
                            top: Val::Px(0.0),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    material: materials.add(Color::NONE.into()),
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent.spawn_bundle(TextBundle {
                        text: Text::with_section(
                            "Pixie Wrangler",
                            TextStyle {
                                font: handles.fonts[0].clone(),
                                font_size: 60.0,
                                color: Color::rgb(0.9, 0.9, 0.9),
                            },
                            TextAlignment {
                                vertical: VerticalAlign::Center,
                                horizontal: HorizontalAlign::Center,
                            },
                        ),
                        ..Default::default()
                    });
                });
            parent
                .spawn_bundle(NodeBundle {
                    style: Style {
                        size: Size::new(Val::Percent(100.0), Val::Auto),
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::ColumnReverse,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    },
                    material: materials.add(Color::NONE.into()),
                    ..Default::default()
                })
                .with_children(|parent| {
                    let rows = 3;
                    let cols = 3;

                    for row in 0..rows {
                        parent
                            .spawn_bundle(NodeBundle {
                                style: Style {
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    ..Default::default()
                                },
                                material: materials.add(Color::NONE.into()),
                                ..Default::default()
                            })
                            .with_children(|parent| {
                                for col in 0..cols {
                                    let i = row * cols + col + 1;
                                    parent
                                        .spawn_bundle(ButtonBundle {
                                            style: Style {
                                                size: Size::new(Val::Px(150.0), Val::Px(150.0)),
                                                flex_direction: FlexDirection::ColumnReverse,
                                                // horizontally center child text
                                                justify_content: JustifyContent::Center,
                                                // vertically center child text
                                                align_items: AlignItems::Center,
                                                margin: Rect {
                                                    left: Val::Px(10.0),
                                                    bottom: Val::Px(10.0),
                                                    ..Default::default()
                                                },
                                                ..Default::default()
                                            },
                                            material: button_materials.normal.clone(),
                                            ..Default::default()
                                        })
                                        .insert(LevelSelectButton(i))
                                        .with_children(|parent| {
                                            let level = handles
                                                .levels
                                                .get(i as usize - 1)
                                                .and_then(|h| levels.get(h.clone()));

                                            let level_color = match level {
                                                Some(_) => crate::UI_WHITE_COLOR,
                                                None => UI_GREY_RED_COLOR,
                                            };

                                            let (eff_text, star_text_one, star_text_two) =
                                                if let (Some(eff), Some(level)) =
                                                    (best_efficiencies.0.get(&i), level)
                                                {
                                                    let stars = level
                                                        .star_thresholds
                                                        .iter()
                                                        .filter(|t| **t < *eff)
                                                        .count();

                                                    (
                                                        format!("{}", eff),
                                                        "★".repeat(stars),
                                                        "★".repeat(3 - stars),
                                                    )
                                                } else {
                                                    ("".to_string(), "".to_string(), "".to_string())
                                                };

                                            parent.spawn_bundle(TextBundle {
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

                                            parent.spawn_bundle(TextBundle {
                                                text: Text::with_section(
                                                    format!("{}", i),
                                                    TextStyle {
                                                        font: handles.fonts[0].clone(),
                                                        font_size: 60.0,
                                                        color: level_color,
                                                    },
                                                    Default::default(),
                                                ),
                                                ..Default::default()
                                            });

                                            parent.spawn_bundle(TextBundle {
                                                text: Text::with_section(
                                                    eff_text,
                                                    TextStyle {
                                                        font: handles.fonts[0].clone(),
                                                        font_size: 30.0,
                                                        color: crate::FINISHED_ROAD_COLORS[1],
                                                    },
                                                    Default::default(),
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
    mouse.update();
    mouse.update();
}
