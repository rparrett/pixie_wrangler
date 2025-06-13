use crate::RoadSegment;

use bevy::{platform::collections::HashMap, prelude::*};
use bevy_simple_prefs::{Prefs, PrefsPlugin};

#[derive(Prefs, Reflect, Default)]
pub struct SaveFile {
    scores: BestScores,
    solutions: Solutions,
}
#[derive(Resource, Clone, Debug, Default, Reflect)]
pub struct BestScores(pub HashMap<u32, u32>);
#[derive(Resource, Clone, Debug, Default, Reflect)]
pub struct Solutions(pub HashMap<u32, Solution>);
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
