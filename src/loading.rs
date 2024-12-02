use crate::{save::SaveFile, GameState, Handles, MainCamera};
use bevy::{asset::LoadState, prelude::*};
use bevy_simple_prefs::PrefsStatus;

pub struct LoadingPlugin;

pub const NUM_LEVELS: u32 = 9;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Handles>();
        app.add_systems(OnEnter(GameState::Loading), loading_setup);
        app.add_systems(Update, loading_update.run_if(in_state(GameState::Loading)));
    }
}

fn loading_setup(
    mut commands: Commands,
    mut handles: ResMut<Handles>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        Camera2d,
        Transform::from_translation(Vec3::new(0., -10., 0.)),
        Msaa::Sample4,
        MainCamera,
    ));

    for i in 1..=NUM_LEVELS {
        handles
            .levels
            .push(asset_server.load(format!("levels/{i}.level.ron")));
    }

    handles
        .fonts
        .push(asset_server.load("fonts/ChakraPetch-Regular-PixieWrangler.ttf"));
}

fn loading_update(
    handles: Res<Handles>,
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<GameState>>,
    prefs: Res<PrefsStatus<SaveFile>>,
) {
    if handles
        .fonts
        .iter()
        .any(|h| !matches!(asset_server.get_load_state(h), Some(LoadState::Loaded)))
    {
        return;
    }

    if handles
        .levels
        .iter()
        .any(|h| !matches!(asset_server.get_load_state(h), Some(LoadState::Loaded)))
    {
        return;
    }

    if !prefs.loaded {
        return;
    }

    next_state.set(GameState::LevelSelect);
}
