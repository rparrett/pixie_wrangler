use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

pub struct DebugLinesPlugin;
#[derive(Resource, Default)]
pub struct DebugLines(pub Vec<((Vec2, Vec2), Color, f32)>);
#[derive(Component)]
struct DebugLine;

impl Plugin for DebugLinesPlugin {
    // this is where we set up our plugin
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugLines>();
        // run despawn before spawn, ensuring that lines stick around for one frame
        app.add_systems(Update, debug_lines_spawn_system);
        app.add_systems(
            Update,
            debug_lines_despawn_system.before(debug_lines_spawn_system),
        );
    }
}

fn debug_lines_despawn_system(mut commands: Commands, query: Query<Entity, With<DebugLine>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn debug_lines_spawn_system(mut commands: Commands, mut debug_lines: ResMut<DebugLines>) {
    for (line, color, width) in debug_lines.0.drain(..) {
        commands.spawn((
            ShapeBundle {
                path: GeometryBuilder::build_as(&shapes::Line(line.0, line.1)),
                transform: Transform::from_xyz(0.0, 0.0, 999.0),
                ..default()
            },
            Stroke::new(color, width),
            DebugLine,
        ));
    }
}
