use crate::{save::SaveFile, GameState, Handles, MainCamera};
use bevy::{asset::LoadState, prelude::*};
use bevy_pipelines_ready::{PipelinesReady, PipelinesReadyPlugin};
use bevy_prototype_lyon::prelude::*;
use bevy_simple_prefs::PrefsStatus;

pub struct LoadingPlugin;

const EXPECTED_PIPELINES: usize = 10;

pub const NUM_LEVELS: u32 = 12;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PipelinesReadyPlugin);
        app.init_resource::<Handles>();
        app.add_systems(OnEnter(GameState::Loading), loading_setup);
        app.add_systems(Update, loading_update.run_if(in_state(GameState::Loading)));
        app.add_systems(
            Update,
            print_pipelines.run_if(resource_changed::<PipelinesReady>),
        );
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

    commands.spawn((
        ShapeBuilder::with(&shapes::RegularPolygon::default())
            .fill(Color::BLACK)
            .build(),
        StateScoped(GameState::Loading),
    ));

    for i in 1..=NUM_LEVELS {
        handles
            .levels
            .push(asset_server.load(format!("levels/{i}.level.ron")));
    }

    handles
        .fonts
        .push(asset_server.load("fonts/ChakraPetch-Regular-PixieWrangler.ttf"));

    commands.spawn((Text::new("Loading..."), StateScoped(GameState::Loading)));

    handles.music = asset_server.load("music/galactic_odyssey_by_alkakrab.ogg");
}

fn loading_update(
    handles: Res<Handles>,
    asset_server: Res<AssetServer>,
    mut next_state: ResMut<NextState<GameState>>,
    prefs: Res<PrefsStatus<SaveFile>>,
    ready: Res<PipelinesReady>,
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

    if !matches!(
        asset_server.get_load_state(&handles.music),
        Some(LoadState::Loaded),
    ) {
        return;
    }

    if ready.get() < EXPECTED_PIPELINES {
        return;
    }

    if !prefs.loaded {
        return;
    }

    next_state.set(GameState::LevelSelect);
}

fn print_pipelines(ready: Res<PipelinesReady>) {
    info!("Pipelines Ready: {}/{}", ready.get(), EXPECTED_PIPELINES);
}
