use crate::{BestScores, GameState};

use bevy::prelude::*;
use ron::de::{from_reader, from_str};
use ron::ser::{to_string_pretty, to_writer_pretty, PrettyConfig};
use serde::{Deserialize, Serialize};
use std::fs::File;

const SAVE_FILE: &str = "save.ron";

#[derive(Deserialize, Serialize)]
struct SaveFile {
    scores: BestScores,
}

pub struct SavePlugin;
impl Plugin for SavePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(save_system.system());
        app.add_system_set(
            SystemSet::on_enter(GameState::Loading).with_system(load_system.system()),
        );
    }
}

pub fn load_system(mut commands: Commands) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let file = File::open(SAVE_FILE);
        if file.is_err() {
            return;
        }

        let save_file: SaveFile =
            from_reader(file.unwrap()).expect("Failed to deserialize save file");

        commands.insert_resource(save_file.scores.clone());
    };
    #[cfg(target_arch = "wasm32")]
    {
        let storage = web_sys::window()
            .expect("should have a Window")
            .local_storage()
            .expect("should have a Storage")
            .expect("should have a Storage");
        let maybe_item = storage.get_item(SAVE_FILE);
        if maybe_item.is_err() {
            return;
        }
        let maybe_item = maybe_item.unwrap();

        if maybe_item.is_none() {
            return;
        }

        let save_file: SaveFile =
            from_str(&maybe_item.unwrap()).expect("Failed to deserialize save file");

        commands.insert_resource(save_file.scores.clone());
    }
}

pub fn save_system(scores: Res<BestScores>) {
    if !scores.is_changed() || scores.is_added() {
        return;
    }

    let save_file = SaveFile {
        scores: (*scores).clone(),
    };

    let pretty = PrettyConfig::new();

    #[cfg(not(target_arch = "wasm32"))]
    {
        let file = File::create(SAVE_FILE).expect("couldn't create save file");
        let _ = to_writer_pretty(file, &save_file, pretty).expect("failed to serialize save file");
    }
    #[cfg(target_arch = "wasm32")]
    {
        let data = to_string_pretty(&save_file, pretty).expect("failed to serialize save file");

        let storage = web_sys::window()
            .expect("should have a Window")
            .local_storage()
            .expect("should have a Storage")
            .expect("should have a Storage");

        storage
            .set_item(SAVE_FILE, data.as_str())
            .expect("failed to store save file");
    }
}
