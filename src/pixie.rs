use crate::layer;
use crate::{lines::corner_angle, GameState, RoadSegment, Score, TestingState, GRID_SIZE};
use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

pub const PIXIE_RADIUS: f32 = 6.0;

pub struct PixiePlugin;
impl Plugin for PixiePlugin {
    // this is where we set up our plugin
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_set(
            SystemSet::on_update(GameState::Playing)
                .label("pixies")
                .after("test_buttons")
                .with_system(move_pixies_system.system())
                .with_system(emit_pixies_system.system()),
        );
    }
}

pub struct Pixie {
    pub path: Vec<RoadSegment>,
    pub path_index: usize,
    pub next_corner_angle: Option<f32>,
    pub max_speed: f32,
    pub current_speed: f32,
    pub current_speed_limit: f32,
    pub acceleration: f32,
    pub deceleration: f32,
}
impl Default for Pixie {
    fn default() -> Self {
        Self {
            path: vec![],
            path_index: 0,
            next_corner_angle: None,
            max_speed: 60.0,
            current_speed: 60.0,
            current_speed_limit: 60.0,
            acceleration: 10.0,
            deceleration: 50.0,
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

fn move_pixies_system(
    mut commands: Commands,
    time: Res<Time>,
    mut score: ResMut<Score>,
    mut query: Query<(Entity, &mut Pixie, &mut Transform)>,
) {
    for (entity, mut pixie, mut transform) in query.iter_mut() {
        if pixie.path_index > pixie.path.len() - 1 {
            commands.entity(entity).despawn_recursive();
            score.0 += 1;
            continue;
        }

        let delta = time.delta_seconds();

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

        // pixies must slow down as they approach sharp corners
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

        let speed_diff = pixie.current_speed_limit - pixie.current_speed;
        if speed_diff > f32::EPSILON {
            pixie.current_speed += pixie.acceleration * delta;
            if pixie.current_speed > pixie.current_speed_limit {
                pixie.current_speed = pixie.current_speed_limit;
            }
        } else if speed_diff < f32::EPSILON {
            pixie.current_speed -= pixie.deceleration * delta;
            if pixie.current_speed < pixie.current_speed_limit {
                pixie.current_speed = pixie.current_speed_limit;
            }
        }

        let step = pixie.current_speed * delta;

        // five radians per second, clockwise
        transform.rotate(Quat::from_rotation_z(-5.0 * delta));

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
            transform.translation.x = next_waypoint.x;
            transform.translation.y = next_waypoint.y;

            pixie.path_index += 1;
        }

        if !pixie.next_corner_angle.is_some() || step > dist {
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
                path: emitter.path.clone(),
                path_index: 0,
                ..Default::default()
            });

        emitter.remaining -= 1;
    }
}
