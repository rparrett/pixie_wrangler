use crate::{BestScores, GameState};

use bevy::prelude::*;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

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
        let file = match std::fs::File::open(SAVE_FILE) {
            Ok(f) => f,
            Err(_) => return,
        };

        let save_file: SaveFile = match ron::de::from_reader(file) {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to deserialize save file: {:?}", e);
                return;
            }
        };

        commands.insert_resource(save_file.scores.clone());
    }
    #[cfg(target_arch = "wasm32")]
    {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };

        let storage = match window.local_storage() {
            Ok(Some(s)) => s,
            _ => return,
        };

        let item = match storage.get_item(SAVE_FILE) {
            Ok(Some(i)) => i,
            _ => return,
        };

        let save_file: SaveFile = match ron::de::from_str(&item) {
            Ok(s) => s,
            Err(e) => {
                warn!("Failed to serialize save file: {:?}", e);
                return;
            }
        };

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
        let file = match std::fs::File::create(SAVE_FILE) {
            Ok(f) => f,
            Err(e) => {
                warn!("Failed to create save file: {:?}", e);
                return;
            }
        };

        match ron::ser::to_writer_pretty(file, &save_file, pretty) {
            Err(e) => {
                warn!("Failed to serialize save data: {:?}", e);
                return;
            }
            _ => {}
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let data = match ron::ser::to_string_pretty(&save_file, pretty) {
            Ok(d) => d,
            Err(e) => {
                warn!("Failed to serialize save data: {:?}", e);
                return;
            }
        };

        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };

        let storage = match window.local_storage() {
            Ok(Some(s)) => s,
            _ => return,
        };

        match storage.set_item(SAVE_FILE, data.as_str()) {
            Err(e) => {
                warn!("Failed to store save file: {:?}", e);
            }
            _ => {}
        }
    }
}
