use crate::PixieFlavor;
use bevy::{platform::collections::HashSet, prelude::*, reflect::TypePath};
use serde::Deserialize;

#[derive(Deserialize, Debug, Asset, TypePath)]
pub struct Level {
    pub name: String,
    pub name_position: Vec2,
    pub layers: u32,
    pub terminuses: Vec<Terminus>,
    pub obstacles: Vec<Obstacle>,
    pub star_thresholds: Vec<u32>,
}

#[derive(Deserialize, Debug, Clone, Component)]
pub enum Obstacle {
    Rect(Vec2, Vec2),
}

#[derive(Default, Debug, Deserialize, Clone, Component)]
pub struct Terminus {
    pub point: Vec2,
    pub emits: HashSet<PixieFlavor>,
    pub collects: HashSet<PixieFlavor>,
}
