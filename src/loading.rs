use crate::{GameState, Handles, UiCamera};
use bevy::{asset::LoadState, prelude::*};

pub struct LoadingPlugin;

pub const NUM_LEVELS: u32 = 9;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_enter(GameState::Loading).with_system(loading_setup));
        app.add_system_set(SystemSet::on_update(GameState::Loading).with_system(loading_update));
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

    for i in 1..=NUM_LEVELS {
        handles
            .levels
            .push(asset_server.load(format!("levels/{}.level.ron", i).as_str()));
    }

    handles
        .fonts
        .push(asset_server.load("fonts/ChakraPetch-Regular-PixieWrangler.ttf"));
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
