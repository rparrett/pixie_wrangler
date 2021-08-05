use crate::collision::point_segment_collision;
use crate::layer;
use crate::lines::travel_on_segments;
use crate::{lines::corner_angle, GameState, RoadSegment, Score, TestingState, GRID_SIZE};

use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use rand::Rng;

pub const PIXIE_RADIUS: f32 = 6.0;

pub struct PixiePlugin;
impl Plugin for PixiePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_set(
            SystemSet::on_update(GameState::Playing)
                .label("pixies")
                .after("test_buttons")
                .with_system(
                    collide_pixies_system
                        .system()
                        .label("collide_pixies")
                        .before("move_pixies"),
                )
                .with_system(explode_pixies_system.system().after("collide_pixies"))
                .with_system(move_pixies_system.system().label("move_pixies"))
                .with_system(move_fragments_system.system())
                .with_system(emit_pixies_system.system()),
        );
    }
}

struct PixieFragment {
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

pub struct Pixie {
    pub flavor: u32,
    pub path: Vec<RoadSegment>,
    pub path_index: usize,
    pub next_corner_angle: Option<f32>,
    pub max_speed: f32,
    pub current_speed: f32,
    pub current_speed_limit: f32,
    pub acceleration: f32,
    pub deceleration: f32,
    pub attracted: bool,
    pub exploding: bool,
}
impl Default for Pixie {
    fn default() -> Self {
        Self {
            flavor: 0,
            path: vec![],
            path_index: 0,
            next_corner_angle: None,
            max_speed: 60.0,
            current_speed: 60.0,
            current_speed_limit: 60.0,
            acceleration: 10.0,
            deceleration: 50.0,
            attracted: false,
            exploding: false,
        }
    }
}

pub struct PixieEmitter {
    pub flavor: u32,
    pub path: Vec<RoadSegment>,
    pub remaining: u32,
    pub timer: Timer,
}

pub const PIXIE_COLORS: [Color; 6] = [
    Color::AQUAMARINE,
    Color::PINK,
    Color::ORANGE,
    Color::PURPLE,
    Color::DARK_GREEN,
    Color::YELLOW,
];

fn move_fragments_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut PixieFragment, &mut Transform)>,
) {
    let delta = time.delta_seconds();

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

fn explode_pixies_system(mut commands: Commands, query: Query<(Entity, &Pixie, &Transform)>) {
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
                    ShapeColors::new(PIXIE_COLORS[(pixie.flavor) as usize].as_rgba_linear()),
                    DrawMode::Fill(FillOptions::default()),
                    *transform,
                ))
                .insert(PixieFragment {
                    direction: Vec2::new(theta.cos(), theta.sin()),
                    ..Default::default()
                });
        }
    }
}

fn collide_pixies_system(
    mut queries: QuerySet<(Query<(Entity, &Pixie, &Transform)>, Query<&mut Pixie>)>,
) {
    let mut collisions = vec![];

    // TODO if this is set too high, there may be some mutual attraction and pixies will just
    // speed up and never actually annihilate. Maybe we should prevent attractors from getting
    // into the attracting state.
    let project_dist = PIXIE_RADIUS * 3.0;
    let explosion_dist = PIXIE_RADIUS * 0.5;

    for (e1, p1, t1) in queries
        .q0()
        .iter()
        .filter(|(_, p, _)| p.path_index < p.path.len())
    {
        // we are going to project forward along the pixie's travel path and check for collisions
        // with other pixies of different flavors.
        // if we find one, we'll put this pixie into an "attracted" state which should drive it
        // faster towards its ultimate annihilation.

        let layer = p1.path[p1.path_index].layer;

        // colliding from a slightly forward point to we don't get attracted to a pixie behind us

        let just_forward_segs =
            travel_on_segments(t1.translation.truncate(), 0.01, &p1.path[p1.path_index..]);
        let just_forward = if let Some(seg) = just_forward_segs.first() {
            seg.1
        } else {
            break;
        };

        let travel_segs = travel_on_segments(just_forward, project_dist, &p1.path[p1.path_index..]);

        for (e2, _, t2) in queries
            .q0()
            .iter()
            .filter(|(_, p2, _)| p2.path_index < p2.path.len())
            .filter(|(_, p2, _)| p2.path[p2.path_index].layer == layer)
            .filter(|(_, p2, _)| p2.flavor != p1.flavor)
            .filter(|(e2, _, _)| *e2 != e1)
        {
            for seg in travel_segs.iter() {
                let annihilating = t2
                    .translation
                    .truncate()
                    .distance(t1.translation.truncate())
                    < explosion_dist;

                if annihilating {
                    collisions.push((e1, e2, true));
                    break;
                }

                let col = point_segment_collision(t2.translation.truncate(), seg.0, seg.1);

                match col {
                    crate::collision::SegmentCollision::None => {}
                    _ => {
                        collisions.push((e1, e2, false));
                        break;
                    }
                }
            }
        }
    }

    for mut pixie in queries.q1_mut().iter_mut() {
        pixie.attracted = false;
    }

    for (e1, e2, explode) in collisions.iter() {
        if let Ok(mut pixie) = queries.q1_mut().get_mut(*e1) {
            pixie.attracted = true;
            if *explode {
                pixie.exploding = *explode;
            }
        }
        if let Ok(mut pixie) = queries.q1_mut().get_mut(*e2) {
            if *explode {
                pixie.exploding = *explode;
            }
        }
    }
}

fn move_pixies_system(
    mut commands: Commands,
    time: Res<Time>,
    mut score: ResMut<Score>,
    mut query: Query<(Entity, &mut Pixie, &mut Transform)>,
) {
    let delta = time.delta_seconds();

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
        let prev_layer = if let Some(seg) = pixie.path.get(pixie.path_index - 1) {
            seg.layer
        } else {
            current_layer
        };
        let dist = transform.translation.truncate().distance(next_waypoint);
        let last_dist = transform.translation.truncate().distance(prev_waypoint);

        if pixie.attracted {
            // pixies will drive very recklessly towards a pixie of another
            // flavor
            pixie.current_speed_limit = 100.0;
        } else {
            // pixies must otherwise slow down as they approach sharp corners
            if dist < GRID_SIZE {
                if let Some(angle) = pixie.next_corner_angle {
                    if angle <= 45.0 {
                        pixie.current_speed_limit = 10.0;
                    } else if angle <= 90.0 {
                        pixie.current_speed_limit = 30.0;
                    }
                } else {
                    pixie.current_speed_limit = pixie.max_speed;
                }
            } else {
                pixie.current_speed_limit = pixie.max_speed;
            }
        }

        let speed_diff = pixie.current_speed_limit - pixie.current_speed;
        if speed_diff > f32::EPSILON {
            pixie.current_speed += pixie.acceleration * delta;
            pixie.current_speed = pixie.current_speed.min(pixie.current_speed_limit);
        } else if speed_diff < f32::EPSILON {
            pixie.current_speed -= pixie.deceleration * delta;
            pixie.current_speed = pixie.current_speed.max(pixie.current_speed_limit);
        }

        let step = pixie.current_speed * delta;

        transform.rotate(Quat::from_rotation_z(pixie.current_speed * -0.08 * delta));

        if step < dist {
            transform.translation.x += step / dist * (next_waypoint.x - transform.translation.x);
            transform.translation.y += step / dist * (next_waypoint.y - transform.translation.y);

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
            // TODO we should really move past this next waypoint the remaining distance

            transform.translation.x = next_waypoint.x;
            transform.translation.y = next_waypoint.y;

            pixie.path_index += 1;
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
    }
}

fn emit_pixies_system(
    time: Res<Time>,
    testing_state: Res<TestingState>,
    mut q_emitters: Query<&mut PixieEmitter>,
    mut commands: Commands,
) {
    if testing_state.started.is_none() {
        return;
    }

    for mut emitter in q_emitters.iter_mut() {
        if emitter.remaining == 0 {
            continue;
        }

        emitter.timer.tick(time.delta());

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
                ShapeColors::new(PIXIE_COLORS[(emitter.flavor) as usize].as_rgba_linear()),
                DrawMode::Fill(FillOptions::default()),
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
