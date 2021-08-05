use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use serde::Deserialize;

use crate::Terminus;

#[derive(Deserialize, Debug, TypeUuid)]
#[uuid = "962DF4C2-C221-4364-A9F7-B7340FB60437"]
pub struct Level {
    pub terminuses: Vec<Terminus>,
    pub obstacles: Vec<Obstacle>,
    pub star_thresholds: Vec<u32>,
}

#[derive(Deserialize, Debug)]
pub enum Obstacle {
    Rect(Vec2, Vec2),
    Polygon(Vec<Vec2>),
}
