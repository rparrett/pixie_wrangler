use std::time::Duration;

use crate::layer;
use crate::lines::{distance_on_path, travel, traveled_segments};
use crate::sim::SIMULATION_TIMESTEP;
use crate::{lines::corner_angle, GameState, PixieCount, RoadSegment, SimulationState, GRID_SIZE};

use bevy::prelude::*;
use bevy::utils::{HashMap, HashSet};
use bevy_prototype_lyon::prelude::*;
use rand::Rng;
use serde::Deserialize;

pub const PIXIE_RADIUS: f32 = 6.0;
pub const PIXIE_VISION_DISTANCE: f32 = PIXIE_RADIUS * 4.0;
pub const PIXIE_BRAKING_DISTANCE: f32 = PIXIE_RADIUS * 3.0;
pub const PIXIE_EXPLOSION_DISTANCE: f32 = PIXIE_RADIUS * 0.5;
pub const PIXIE_MIN_SPEED: f32 = 10.0;
pub const PIXIE_MAX_SPEED: f32 = 60.0;
pub const PIXIE_MAX_SPEED_45: f32 = 10.0;
pub const PIXIE_MAX_SPEED_90: f32 = 30.0;
pub const PIXIE_MAX_SPEED_ATTRACTED: f32 = 120.0;
pub const CORNER_DEBUFF_ACTIVATION_DISTANCE: f32 = GRID_SIZE;
pub const CORNER_DEBUFF_DISTANCE: f32 = 24.0;

pub struct PixiePlugin;
impl Plugin for PixiePlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_update(GameState::Playing).with_system(move_fragments_system.system()),
        );
    }
}

#[derive(Component)]
pub struct PixieFragment {
    direction: Vec2,
    life_remaining: f32,
}
impl Default for PixieFragment {
    fn default() -> Self {
        Self {
            direction: Vec2::splat(0.0),
            life_remaining: 5.0,
        }
    }
}

#[derive(Component)]
pub struct Pixie {
    pub flavor: PixieFlavor,
    pub path: Vec<RoadSegment>,
    pub path_index: usize,
    pub next_corner_angle: Option<f32>,
    pub current_speed: f32,
    pub acceleration: f32,
    pub deceleration: f32,
    pub exploding: bool,
    pub lead_pixie: Option<LeadPixie>,
    pub driving_state: DrivingState,
    pub corner_debuff_distance_remaining: f32,
    pub corner_debuff_acceleration: f32,
}
impl Default for Pixie {
    fn default() -> Self {
        Self {
            flavor: PixieFlavor::default(),
            path: vec![],
            path_index: 0,
            next_corner_angle: None,
            current_speed: PIXIE_MAX_SPEED,
            acceleration: 50.0,
            deceleration: 50.0,
            exploding: false,

            lead_pixie: None,
            driving_state: DrivingState::Cruising,
            corner_debuff_distance_remaining: 0.0,
            corner_debuff_acceleration: 0.0,
        }
    }
}

#[derive(Clone)]
pub struct LeadPixie {
    distance: f32,
    speed: f32,
    attractor: bool,
}

#[derive(Clone)]
pub enum DrivingState {
    Accelerating,
    Cruising,
    Braking,
}
#[derive(Component)]

pub struct PixieEmitter {
    pub flavor: PixieFlavor,
    pub path: Vec<RoadSegment>,
    pub remaining: u32,
    pub timer: Timer,
}

#[derive(Copy, Clone, Default, Debug, Deserialize, PartialEq, Eq, Hash)]
pub struct PixieFlavor {
    pub color: u32,
    pub net: u32,
}

pub const PIXIE_COLORS: [Color; 6] = [
    Color::AQUAMARINE,
    Color::PINK,
    Color::ORANGE,
    Color::PURPLE,
    Color::DARK_GREEN,
    Color::YELLOW,
];

pub fn move_fragments_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut PixieFragment, &mut Transform)>,
) {
    let delta = SIMULATION_TIMESTEP;

    for (entity, mut frag, mut transform) in query.iter_mut() {
        frag.life_remaining -= delta;
        if frag.life_remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        transform.rotate(Quat::from_rotation_z(5.0 * delta));

        transform.translation += Vec3::new(
            delta * 100.0 * frag.direction.x,
            delta * 100.0 * frag.direction.y,
            0.0,
        );
    }
}

pub fn explode_pixies_system(mut commands: Commands, query: Query<(Entity, &Pixie, &Transform)>) {
    let mut rng = rand::thread_rng();

    let shape = shapes::RegularPolygon {
        sides: 3,
        feature: shapes::RegularPolygonFeature::Radius(PIXIE_RADIUS / 2.0),
        ..shapes::RegularPolygon::default()
    };

    for (entity, pixie, transform) in query.iter().filter(|(_, p, _)| p.exploding) {
        commands.entity(entity).despawn();

        // ideally we would have just stored a list of annihilating pairs so we can fling
        // pixie fragments in opposite directions, and then we wouldn't have to iter
        // every pixie again

        // after doing some profiling though, compared to the effort of actually rendering
        // "lots of things," that's probably a micro-optimization.

        for _ in 0..2 {
            let theta = rng.gen_range(0.0..std::f32::consts::TAU);

            commands
                .spawn_bundle(GeometryBuilder::build_as(
                    &shape,
                    DrawMode::Fill(FillMode::color(PIXIE_COLORS[(pixie.flavor.color) as usize])),
                    *transform,
                ))
                .insert(PixieFragment {
                    direction: Vec2::new(theta.cos(), theta.sin()),
                    ..Default::default()
                });
        }
    }
}

pub fn collide_pixies_system(
    mut queries: QuerySet<(
        QueryState<(Entity, &Pixie, &Transform)>,
        QueryState<&mut Pixie>,
    )>,
) {
    let mut collisions = vec![];

    // prevent any pixie that's exploding from interacting in another
    // way
    let mut explosions = HashSet::default();
    // prevent any pixie that is attracting another from itself being
    // attracted
    let mut attractors = HashSet::default();
    // prevent pixies that are overlapping from mutually slowing down
    // for each other
    let mut followers = HashMap::default();

    let mut collision_groups = HashMap::default();

    for [(e1, p1, t1), (e2, p2, t2)] in queries.q0().iter_combinations() {
        if e2 == e1 {
            continue;
        }

        if p1.path_index >= p1.path.len() {
            continue;
        }

        let layer = p1.path[p1.path_index].layer;

        if p2.path_index >= p2.path.len() {
            continue;
        }

        if p2.path[p2.path_index].layer != layer {
            continue;
        }

        // this collision detection code is directional along the path,
        // so both orientations must be considered.
        for (e1, p1, t1, e2, p2, t2) in [(e1, p1, t1, e2, p2, t2), (e2, p2, t2, e1, p1, t1)] {
            let travel_segs = traveled_segments(
                t1.translation.truncate(),
                PIXIE_VISION_DISTANCE,
                &p1.path[p1.path_index..],
            );

            let dist = distance_on_path(
                t1.translation.truncate(),
                t2.translation.truncate(),
                &travel_segs,
            );

            if let Some(dist) = dist {
                collision_groups.entry(e1).or_insert(vec![]).push((
                    p1.flavor,
                    e2,
                    p2.flavor,
                    p2.current_speed,
                    dist,
                ));
            }
        }
    }

    for (e1, group) in collision_groups.iter_mut() {
        // we probably only need to care about the "lead pixie"
        group.sort_by(|a, b| a.4.partial_cmp(&b.4).unwrap());

        // TODO it would probably be proper to collect these, sort them by distance,
        // and then iterate them again so that pixies with the closest lead-pixie
        // get preferential treatment when deciding who can be attracted to whom.

        if let Some((p1_flavor, e2, flavor, current_speed, dist)) = group.first() {
            if explosions.contains(e1) || explosions.contains(e2) {
                continue;
            }

            if flavor.color != p1_flavor.color && *dist <= PIXIE_EXPLOSION_DISTANCE {
                explosions.insert(*e1);
                explosions.insert(*e2);
                continue;
            }

            // if we are already attracting a pixie, and our lead pixie is
            // dissimilar, then we can just carry on.
            if attractors.contains(e1) && flavor.color != p1_flavor.color {
                continue;
            }

            if flavor.color != p1_flavor.color {
                attractors.insert(*e2);
            }

            match followers.get(e2) {
                Some(follower) if *follower == *e1 => continue,
                _ => {}
            }

            collisions.push((
                *e1,
                *e2,
                LeadPixie {
                    speed: *current_speed,
                    distance: *dist,
                    attractor: flavor.color != p1_flavor.color,
                },
            ));

            followers.insert(*e1, *e2);
        }
    }

    for mut pixie in queries.q1().iter_mut() {
        pixie.lead_pixie = None;
    }

    for entity in explosions.iter() {
        if let Ok(mut pixie) = queries.q1().get_mut(*entity) {
            pixie.exploding = true;
        }
    }

    for (e1, _e2, lead_pixie) in collisions.iter() {
        if let Ok(mut pixie) = queries.q1().get_mut(*e1) {
            pixie.lead_pixie = Some(lead_pixie.clone());
        }
    }
}

pub fn move_pixies_system(
    mut commands: Commands,
    mut score: ResMut<PixieCount>,
    mut query: Query<(Entity, &mut Pixie, &mut Transform)>,
) {
    let delta = SIMULATION_TIMESTEP;

    for (entity, mut pixie, mut transform) in query.iter_mut() {
        if pixie.path_index > pixie.path.len() - 1 {
            commands.entity(entity).despawn_recursive();
            score.0 += 1;
            continue;
        }

        let next_waypoint = pixie.path[pixie.path_index].points.1;
        let prev_waypoint = pixie.path[pixie.path_index].points.0;
        let current_layer = pixie.path[pixie.path_index].layer;
        let next_layer = if let Some(seg) = pixie.path.get(pixie.path_index + 1) {
            seg.layer
        } else {
            current_layer
        };
        let prev_layer = if let Some(seg) = pixie
            .path_index
            .checked_sub(1)
            .map(|i| pixie.path.get(i))
            .flatten()
        {
            seg.layer
        } else {
            current_layer
        };
        let dist = transform.translation.truncate().distance(next_waypoint);
        let last_dist = transform.translation.truncate().distance(prev_waypoint);

        // determine speed limit and acceleration based on environmental factors

        let mut speed_limit = PIXIE_MAX_SPEED;

        if let Some(lead_pixie) = &pixie.lead_pixie {
            if !lead_pixie.attractor && lead_pixie.distance < PIXIE_BRAKING_DISTANCE {
                speed_limit = lead_pixie.speed - 10.0;
                speed_limit = speed_limit.max(PIXIE_MIN_SPEED);
            }
        }
        if dist < CORNER_DEBUFF_ACTIVATION_DISTANCE {
            // pixies must slow down as they approach sharp corners

            if let Some(angle) = pixie.next_corner_angle {
                if angle <= 45.0 {
                    speed_limit = speed_limit.min(PIXIE_MAX_SPEED_45);
                    pixie.corner_debuff_distance_remaining = CORNER_DEBUFF_DISTANCE;
                    pixie.corner_debuff_acceleration = pixie.acceleration / 8.0;
                } else if angle <= 90.0 {
                    speed_limit = speed_limit.min(PIXIE_MAX_SPEED_90);
                    pixie.corner_debuff_distance_remaining = CORNER_DEBUFF_DISTANCE;
                    pixie.corner_debuff_acceleration = pixie.acceleration / 6.0;
                }
            }
        }
        if let Some(lead_pixie) = &pixie.lead_pixie {
            // pixies will drive very recklessly towards a pixie of another
            // flavor. this overrides other cornering and braking behaviors.

            if lead_pixie.attractor {
                speed_limit = PIXIE_MAX_SPEED_ATTRACTED;
            }
        }

        let acceleration = if pixie.corner_debuff_distance_remaining > 0.0 {
            pixie.corner_debuff_acceleration
        } else {
            pixie.acceleration
        };

        pixie.driving_state = DrivingState::Cruising;

        // move towards speed limit

        let speed_diff = speed_limit - pixie.current_speed;

        if speed_diff < -1.0 * f32::EPSILON {
            pixie.current_speed -= pixie.deceleration * delta;
            pixie.current_speed = pixie.current_speed.max(speed_limit);
            pixie.driving_state = DrivingState::Braking;
        }

        if speed_diff > f32::EPSILON {
            pixie.current_speed += acceleration * delta;
            pixie.current_speed = pixie.current_speed.min(speed_limit);
            pixie.driving_state = DrivingState::Accelerating
        }

        // move the pixie

        let step = pixie.current_speed * delta;

        let (to, segments_traveled) = travel(
            transform.translation.truncate(),
            step,
            &pixie.path[pixie.path_index..],
        );

        transform.translation.x = to.x;
        transform.translation.y = to.y;

        if segments_traveled == 0 {
            // pixies travelling uphill should stay above the next road as they approach it.
            // pixies travelling downhill should stay above the previous road as they leave it.
            if next_layer < current_layer && dist < PIXIE_RADIUS {
                transform.translation.z = layer::PIXIE - next_layer as f32;
            } else if prev_layer < current_layer && last_dist < PIXIE_RADIUS {
                transform.translation.z = layer::PIXIE - prev_layer as f32;
            } else {
                transform.translation.z = layer::PIXIE - current_layer as f32
            }
        } else {
            pixie.path_index += segments_traveled;
        }

        if pixie.next_corner_angle.is_none() || step > dist {
            if let (Some(current_waypoint), Some(next_waypoint)) = (
                pixie.path.get(pixie.path_index),
                pixie.path.get(pixie.path_index + 1),
            ) {
                pixie.next_corner_angle = Some(
                    corner_angle(
                        current_waypoint.points.0,
                        next_waypoint.points.0,
                        next_waypoint.points.1,
                    )
                    .to_degrees(),
                );
            } else {
                pixie.next_corner_angle = Some(180.0);
            }
        }

        pixie.corner_debuff_distance_remaining =
            (pixie.corner_debuff_distance_remaining - step).max(0.0);

        transform.rotate(Quat::from_rotation_z(pixie.current_speed * -0.08 * delta));
    }
}

pub fn emit_pixies_system(
    testing_state: Res<SimulationState>,
    mut q_emitters: Query<&mut PixieEmitter>,
    mut commands: Commands,
) {
    if !testing_state.started {
        return;
    }

    for mut emitter in q_emitters.iter_mut() {
        if emitter.remaining == 0 {
            continue;
        }

        emitter
            .timer
            .tick(Duration::from_secs_f32(SIMULATION_TIMESTEP));

        if !emitter.timer.finished() {
            continue;
        }

        let shape = shapes::RegularPolygon {
            sides: 6,
            feature: shapes::RegularPolygonFeature::Radius(PIXIE_RADIUS),
            ..shapes::RegularPolygon::default()
        };

        let first_segment = emitter.path.first().unwrap();

        commands
            .spawn_bundle(GeometryBuilder::build_as(
                &shape,
                DrawMode::Fill(FillMode::color(
                    PIXIE_COLORS[(emitter.flavor.color) as usize],
                )),
                Transform::from_translation(
                    first_segment
                        .points
                        .0
                        .extend(layer::PIXIE - first_segment.layer as f32),
                ),
            ))
            .insert(Pixie {
                flavor: emitter.flavor,
                path: emitter.path.clone(),
                path_index: 0,
                ..Default::default()
            });

        emitter.remaining -= 1;
    }
}
