use crate::RoadSegment;

use bevy::{audio::Volume, platform::collections::HashMap, prelude::*};
use bevy_simple_prefs::{Prefs, PrefsPlugin};

#[derive(Prefs, Reflect, Default)]
pub struct SaveFile {
    scores: BestScores,
    solutions: Solutions,
    music_volume: MusicVolume,
}
#[derive(Resource, Clone, Debug, Default, Reflect)]
pub struct BestScores(pub HashMap<u32, u32>);
#[derive(Resource, Clone, Debug, Default, Reflect)]
pub struct Solutions(pub HashMap<u32, Solution>);

#[derive(Resource, Reflect, Clone, Copy, Eq, PartialEq, Debug)]
pub struct MusicVolume(pub u8);
impl Default for MusicVolume {
    fn default() -> Self {
        Self(50)
    }
}
impl From<MusicVolume> for Volume {
    fn from(val: MusicVolume) -> Self {
        if val.0 == 0 {
            Volume::Linear(0.0)
        } else {
            let db = -30.0 * (1.0 - val.0 as f32 / 100.0);
            Volume::Decibels(db)
        }
    }
}

impl MusicVolume {
    pub fn is_muted(&self) -> bool {
        self.0 == 0
    }
}
#[derive(Clone, Debug, Default, Reflect)]
pub struct Solution {
    pub segments: Vec<RoadSegment>,
}

pub struct SavePlugin;
impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PrefsPlugin::<SaveFile>::default());
    }
}
