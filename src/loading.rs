use crate::{GameState, Handles, UiCamera};
use bevy::{asset::LoadState, prelude::*};

pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_set(
            SystemSet::on_enter(GameState::Loading).with_system(loading_setup.system()),
        );
        app.add_system_set(
            SystemSet::on_update(GameState::Loading).with_system(loading_update.system()),
        );
    }
}

fn loading_setup(
    mut commands: Commands,
    mut handles: ResMut<Handles>,
    asset_server: Res<AssetServer>,
) {
    commands
        .spawn_bundle(UiCameraBundle::default())
        .insert(UiCamera);

    for i in 1..=8 {
        handles
            .levels
            .push(asset_server.load(format!("levels/{}.level.ron", i).as_str()));
    }

    handles
        .fonts
        .push(asset_server.load("fonts/CooperHewitt-Medium.ttf"));
}

fn loading_update(
    handles: Res<Handles>,
    asset_server: Res<AssetServer>,
    mut state: ResMut<State<GameState>>,
) {
    if !matches!(
        asset_server.get_group_load_state(handles.levels.iter().cloned().map(|h| h.id)),
        LoadState::Loaded
    ) {
        return;
    }

    if !matches!(
        asset_server.get_group_load_state(handles.fonts.iter().cloned().map(|h| h.id)),
        LoadState::Loaded
    ) {
        return;
    }

    state.replace(GameState::LevelSelect).unwrap();
}
