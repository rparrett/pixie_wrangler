use crate::PixieFlavor;
use bevy::{
    prelude::*,
    reflect::{TypePath, TypeUuid},
    utils::HashSet,
};
use serde::Deserialize;

#[derive(Deserialize, Debug, TypeUuid, TypePath)]
#[uuid = "962DF4C2-C221-4364-A9F7-B7340FB60437"]
pub struct Level {
    pub layers: u32,
    pub terminuses: Vec<Terminus>,
    pub obstacles: Vec<Obstacle>,
    pub star_thresholds: Vec<u32>,
}

#[derive(Deserialize, Debug)]
pub enum Obstacle {
    Rect(Vec2, Vec2),
    Polygon(Vec<Vec2>),
}

#[derive(Default, Debug, Deserialize, Clone, Component)]
pub struct Terminus {
    pub point: Vec2,
    pub emits: HashSet<PixieFlavor>,
    pub collects: HashSet<PixieFlavor>,
}
