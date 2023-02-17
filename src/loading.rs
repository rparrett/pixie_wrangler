use crate::{GameState, Handles, MainCamera};
use bevy::{asset::LoadState, prelude::*};

pub struct LoadingPlugin;

pub const NUM_LEVELS: u32 = 9;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_to_schedule(OnEnter(GameState::Loading), loading_setup);
        app.add_system(loading_update.in_set(OnUpdate(GameState::Loading)));
    }
}

fn loading_setup(
    mut commands: Commands,
    mut handles: ResMut<Handles>,
    asset_server: Res<AssetServer>,
) {
    let mut camera = Camera2dBundle::default();
    camera.transform.translation.y -= 10.0;

    commands.spawn((camera, MainCamera));

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
    mut next_state: ResMut<NextState<GameState>>,
) {
    if !matches!(
        asset_server.get_group_load_state(handles.levels.iter().cloned().map(|h| h.id())),
        LoadState::Loaded
    ) {
        return;
    }

    if !matches!(
        asset_server.get_group_load_state(handles.fonts.iter().cloned().map(|h| h.id())),
        LoadState::Loaded
    ) {
        return;
    }

    next_state.set(GameState::LevelSelect);
}
