use crate::{corner_angle, GameState, Score, GRID_SIZE};
use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

pub struct PixiePlugin;
impl Plugin for PixiePlugin {
    // this is where we set up our plugin
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_set(
            SystemSet::on_update(GameState::Playing)
                .label("pixies")
                .with_system(move_pixies.system())
                .with_system(emit_pixies.system()),
        );
    }
}

pub struct Pixie {
    pub path: Vec<Vec2>,
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
    pub path: Vec<Vec2>,
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

fn move_pixies(
    mut commands: Commands,
    time: Res<Time>,
    mut score: ResMut<Score>,
    mut query: Query<(Entity, &mut Pixie, &mut Transform)>,
) {
    for (entity, mut pixie, mut transform) in query.iter_mut() {
        if pixie.path_index >= pixie.path.len() - 1 {
            commands.entity(entity).despawn_recursive();
            score.0 += 1;
            continue;
        }

        let next_waypoint = pixie.path[pixie.path_index + 1];
        let dist = transform.translation.truncate().distance(next_waypoint);

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

        let delta = time.delta_seconds();

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
        } else {
            transform.translation.x = next_waypoint.x;
            transform.translation.y = next_waypoint.y;
            pixie.path_index += 1;
        }

        if !pixie.next_corner_angle.is_some() || step > dist {
            if let (Some(current_waypoint), Some(next_waypoint), Some(next_next_waypoint)) = (
                pixie.path.get(pixie.path_index),
                pixie.path.get(pixie.path_index + 1),
                pixie.path.get(pixie.path_index + 2),
            ) {
                pixie.next_corner_angle = Some(
                    corner_angle(*current_waypoint, *next_waypoint, *next_next_waypoint)
                        .to_degrees(),
                );
            } else {
                pixie.next_corner_angle = Some(180.0);
            }
        }
    }
}

fn emit_pixies(time: Res<Time>, mut q_emitters: Query<&mut PixieEmitter>, mut commands: Commands) {
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
            feature: shapes::RegularPolygonFeature::Radius(6.0),
            ..shapes::RegularPolygon::default()
        };

        commands
            .spawn_bundle(GeometryBuilder::build_as(
                &shape,
                ShapeColors::new(PIXIE_COLORS[(emitter.flavor) as usize].as_rgba_linear()),
                DrawMode::Fill(FillOptions::default()),
                Transform::from_translation(emitter.path.first().unwrap().extend(1.0)),
            ))
            .insert(Pixie {
                path: emitter.path.clone(),
                path_index: 0,
                ..Default::default()
            });

        emitter.remaining -= 1;
    }
}
