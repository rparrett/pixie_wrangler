use crate::{
    level::Level,
    loading::NUM_LEVELS,
    save::{BestScores, MusicVolume},
    theme,
    ui::button,
    GameState, Handles, BOTTOM_BAR_HEIGHT,
};

use bevy::prelude::*;

pub struct LevelSelectPlugin;
#[derive(Component)]
pub struct LevelSelectScreen;
#[derive(Component)]
pub struct LevelSelectButton(u32);
#[derive(Component)]
struct SettingsPanelBody;
#[derive(Component)]
struct LevelsPanelBody;
#[derive(Component)]
struct MusicVolumeDown;
#[derive(Component)]
struct MusicVolumeUp;
#[derive(Component)]
struct MusicVolumeLabel;

impl Plugin for LevelSelectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::LevelSelect), level_select_enter);

        app.add_systems(
            Update,
            (
                level_select_button_system,
                (
                    music_volume_button_system,
                    music_volume_text_system.run_if(resource_changed::<MusicVolume>),
                )
                    .chain(),
            )
                .run_if(in_state(GameState::LevelSelect)),
        );

        app.add_systems(OnExit(GameState::LevelSelect), level_select_exit);

        app.add_observer(populate_settings_panel_body);
        app.add_observer(populate_levels_panel_body);
    }
}

// TODO add "diagonal scrolling grid" background

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
    let num_stars = num_stars(&best_scores, &handles, &levels);

    let root = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                ..default()
            },
            LevelSelectScreen,
        ))
        .id();

    let bottom_bar = commands
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Px(BOTTOM_BAR_HEIGHT),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect {
                    left: Val::Px(20.),
                    right: Val::Px(20.),
                    top: Val::Px(10.),
                    bottom: Val::Px(10.),
                },
                ..default()
            },
            BackgroundColor(theme::UI_PANEL_BACKGROUND.into()),
        ))
        .with_children(|parent| {
            parent.spawn((
                Node {
                    align_self: AlignSelf::Center,
                    ..default()
                },
                Text::new("₽IXIE WRANGLER"),
                TextFont {
                    font: handles.fonts[0].clone(),
                    font_size: 25.0,
                    ..default()
                },
                TextColor(theme::PIXIE[1].into()),
            ));
            // Right side of top bar
            parent
                .spawn(Node {
                    align_items: AlignItems::FlexStart,
                    justify_content: JustifyContent::Center,
                    column_gap: Val::Px(10.),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        Node {
                            align_self: AlignSelf::Center,
                            ..default()
                        },
                        Text::new("★"),
                        TextFont {
                            font: handles.fonts[0].clone(),
                            font_size: 25.0,
                            ..default()
                        },
                        TextColor(theme::UI_LABEL.into()),
                        Children::spawn((
                            Spawn((
                                TextSpan::new(format!("{}/", num_stars.0)),
                                TextFont {
                                    font: handles.fonts[0].clone(),
                                    font_size: 25.0,
                                    ..default()
                                },
                                TextColor(if num_stars.0 == num_stars.1 {
                                    theme::UI_LABEL.into()
                                } else {
                                    theme::UI_LABEL_MUTED.into()
                                }),
                            )),
                            Spawn((
                                TextSpan::new(format!("{}", num_stars.1)),
                                TextFont {
                                    font: handles.fonts[0].clone(),
                                    font_size: 25.0,
                                    ..default()
                                },
                                TextColor(theme::UI_LABEL.into()),
                            )),
                        )),
                    ));
                    parent.spawn((
                        Node {
                            align_self: AlignSelf::Center,
                            ..default()
                        },
                        Text::new(format!("Æ{total_score}")),
                        TextFont {
                            font: handles.fonts[0].clone(),
                            font_size: 25.0,
                            ..default()
                        },
                        TextColor(theme::FINISHED_ROAD[1].into()),
                    ));
                    // TODO clock for flavor?
                });
        })
        .id();

    let main_content = commands
        .spawn((Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            padding: UiRect::all(Val::Px(20.)),
            column_gap: Val::Px(20.),
            display: Display::Grid,
            grid_template_columns: vec![GridTrack::flex(0.75), GridTrack::flex(0.25)],
            ..default()
        },))
        .id();

    let settings_panel = commands
        .spawn(panel(
            "\u{01a9} SETTINGS",
            &handles,
            Node {
                row_gap: Val::Px(10.),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            SettingsPanelBody,
        ))
        .id();

    let levels_panel = commands
        .spawn(panel(
            "\u{0393} /user/levels",
            &handles,
            Node {
                column_gap: Val::Px(10.),
                row_gap: Val::Px(10.),
                flex_wrap: FlexWrap::Wrap,
                align_content: AlignContent::FlexStart,
                ..default()
            },
            LevelsPanelBody,
        ))
        .id();

    commands
        .entity(main_content)
        .add_children(&[levels_panel, settings_panel]);

    commands
        .entity(root)
        .add_children(&[main_content, bottom_bar]);
}

fn panel<M: Component>(
    title: impl Into<String>,
    handles: &Handles,
    body_node: Node,
    body_marker: M,
) -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            overflow: Overflow::hidden(),
            ..default()
        },
        Name::new("Panel"),
        Children::spawn((
            Spawn((
                Name::new("PanelTitle"),
                Node {
                    padding: UiRect {
                        left: Val::Px(20.),
                        right: Val::Px(20.),
                        top: Val::Px(10.),
                        bottom: Val::Px(10.),
                    },
                    align_self: AlignSelf::FlexStart,
                    ..default()
                },
                BackgroundColor(theme::UI_NORMAL_BUTTON.into()),
                Children::spawn(Spawn((
                    Text::new(title),
                    TextFont {
                        font: handles.fonts[0].clone(),
                        font_size: 25.0,
                        ..default()
                    },
                    TextColor(theme::UI_LABEL.into()),
                ))),
            )),
            Spawn((
                Name::new("PanelBody"),
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    padding: UiRect::all(Val::Px(10.)),
                    overflow: Overflow::scroll_y(),
                    ..body_node
                },
                BackgroundColor(theme::UI_PANEL_BACKGROUND.into()),
                body_marker,
            )),
        )),
    )
}

fn level_item(
    level: &Level,
    level_index: u32,
    best_scores: &BestScores,
    font_handle: &Handle<Font>,
) -> impl Bundle {
    let (score_text, star_text_one, star_text_two) =
        if let Some(score) = best_scores.0.get(&level_index) {
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

    // TODO display level name

    (
        Button,
        Name::new("LevelItem"),
        Node {
            width: Val::Px(150.),
            height: Val::Px(150.),
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(theme::UI_NORMAL_BUTTON.into()),
        LevelSelectButton(level_index),
        Children::spawn((
            Spawn((
                Text::new(star_text_one),
                TextFont {
                    font: font_handle.clone(),
                    font_size: 25.0,
                    ..default()
                },
                TextColor(theme::UI_LABEL.into()),
                Children::spawn(Spawn((
                    TextSpan::new(star_text_two),
                    TextFont {
                        font: font_handle.clone(),
                        font_size: 25.0,
                        ..default()
                    },
                    TextColor(theme::UI_LABEL_MUTED.into()),
                ))),
            )),
            Spawn((
                Text::new(format!("{level_index}")),
                TextFont {
                    font: font_handle.clone(),
                    font_size: 50.0,
                    ..default()
                },
                TextColor(theme::UI_LABEL.into()),
            )),
            Spawn((
                Text::new(score_text),
                TextFont {
                    font: font_handle.clone(),
                    font_size: 25.0,
                    ..default()
                },
                TextColor(theme::FINISHED_ROAD[1].into()),
            )),
        )),
    )
}

fn level_select_exit(
    mut commands: Commands,
    query: Query<Entity, With<LevelSelectScreen>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }

    mouse.reset(MouseButton::Left);
    mouse.clear();
}

fn populate_settings_panel_body(
    trigger: Trigger<OnAdd, SettingsPanelBody>,
    mut commands: Commands,
    handles: Res<Handles>,
    music_volume: Res<MusicVolume>,
) {
    commands.entity(trigger.target()).with_child((
        Text::new("Music"),
        TextFont {
            font: handles.fonts[0].clone(),
            font_size: 25.0,
            ..default()
        },
    ));

    commands.entity(trigger.target()).with_child((
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Stretch,
            height: Val::Px(50.0),
            ..default()
        },
        Children::spawn((
            Spawn((MusicVolumeDown, button("<", handles.fonts[0].clone(), 50.0))),
            Spawn((
                Node {
                    flex_grow: 1.0,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                Children::spawn(Spawn((
                    MusicVolumeLabel,
                    Text::new(format!("{}%", music_volume.0)),
                    TextFont {
                        font: handles.fonts[0].clone(),
                        font_size: 25.0,
                        ..default()
                    },
                ))),
            )),
            Spawn((MusicVolumeUp, button(">", handles.fonts[0].clone(), 50.0))),
        )),
    ));
}

fn populate_levels_panel_body(
    trigger: Trigger<OnAdd, LevelsPanelBody>,
    mut commands: Commands,
    handles: Res<Handles>,
    best_scores: Res<BestScores>,
    levels: Res<Assets<Level>>,
) {
    for level_index in 1..=NUM_LEVELS {
        let Some(handle) = handles.levels.get(level_index as usize - 1) else {
            warn!("No level handle for level {level_index}");
            continue;
        };
        let Some(level) = levels.get(handle) else {
            warn!("No level asset for level {level_index}");
            continue;
        };

        commands.entity(trigger.target()).with_child(level_item(
            level,
            level_index,
            &best_scores,
            &handles.fonts[0],
        ));
    }
}

fn music_volume_button_system(
    up_buttons: Query<&Interaction, (Changed<Interaction>, With<MusicVolumeUp>)>,
    down_buttons: Query<&Interaction, (Changed<Interaction>, With<MusicVolumeDown>)>,
    mut volume: ResMut<MusicVolume>,
) {
    let current = volume.bypass_change_detection().0;

    for _ in up_buttons.iter().filter(|i| **i == Interaction::Pressed) {
        let new = (current + 10).min(100);
        volume.set_if_neq(MusicVolume(new));
    }
    for _ in down_buttons.iter().filter(|i| **i == Interaction::Pressed) {
        let new = current.saturating_sub(10);
        volume.set_if_neq(MusicVolume(new));
    }
}

fn music_volume_text_system(
    volume: Res<MusicVolume>,
    texts: Query<&mut Text, With<MusicVolumeLabel>>,
) {
    for mut text in texts {
        text.0 = format!("{}%", volume.0);
    }
}

/// Returns a tuple containing the number of stars the player has
/// earned and the total number of stars available to earn.
fn num_stars(
    best_scores: &BestScores,
    handles: &Handles,
    levels: &Assets<Level>,
) -> (usize, usize) {
    (1..=NUM_LEVELS)
        .flat_map(|i| {
            let handle = handles.levels.get(i as usize - 1)?;
            let level = levels.get(handle)?;
            let maybe_score = best_scores.0.get(&i);

            let stars = level
                .star_thresholds
                .iter()
                .filter(|t| maybe_score.is_some_and(|score| **t <= *score))
                .count();

            let total = level.star_thresholds.len();

            Some((stars, total))
        })
        .fold((0, 0), |acc, e| (acc.0 + e.0, acc.1 + e.1))
}
