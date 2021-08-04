use crate::{ButtonMaterials, GameState, Handles};
use bevy::prelude::*;

pub struct LevelSelectPlugin;
pub struct LevelSelectScreen;
pub struct LevelSelectButton(u32);

impl Plugin for LevelSelectPlugin {
    // this is where we set up our plugin
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
) {
    for (_, button) in query.iter().filter(|(i, _)| **i == Interaction::Clicked) {
        level.0 = button.0;
        state.replace(GameState::Playing).unwrap();
    }
}

fn level_select_enter(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    button_materials: Res<ButtonMaterials>,
    handles: Res<Handles>,
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
                                            parent.spawn_bundle(TextBundle {
                                                text: Text::with_section(
                                                    format!("{}", i),
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
