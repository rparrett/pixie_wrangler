use crate::pixie::{
    collide_pixies_system, emit_pixies_system, explode_pixies_system, move_pixies_system, Pixie,
    PixieEmitter,
};
use bevy::prelude::*;

pub struct SimulationPlugin;
impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<SimulationSettings>();
        app.init_resource::<SimulationState>();
        app.add_stage_after(
            CoreStage::Update,
            "simulation",
            SimulationStage(
                SystemStage::parallel()
                    // need to run these in the same order every time for scores
                    // to be deterministic. probably.
                    .with_system(
                        collide_pixies_system
                            .system()
                            .label("collide_pixies")
                            .before("move_pixies"),
                    )
                    .with_system(move_pixies_system.system().label("move_pixies"))
                    .with_system(
                        emit_pixies_system
                            .system()
                            .label("emit_pixies")
                            .after("move_pixies"),
                    )
                    .with_system(explode_pixies_system.system().after("collide_pixies"))
                    .with_system(update_sim_state_system.system().after("emit_pixies")),
            ),
        );
    }
}

pub const SIMULATION_TIMESTEP: f32 = 0.016_666_668;

#[derive(Default)]
pub struct SimulationState {
    pub started: bool,
    pub step: u32,
    pub done: bool,
}

struct SimulationStage(SystemStage);
impl Stage for SimulationStage {
    fn run(&mut self, world: &mut World) {
        let speed = match world.get_resource::<SimulationSettings>() {
            Some(settings) => settings.speed,
            None => return,
        };

        for _ in 0..speed.steps_per_frame() {
            if let Some(state) = world.get_resource::<SimulationState>() {
                if !state.started {
                    return;
                }

                if state.done {
                    return;
                }
            }

            self.0.run(world);

            match world.get_resource_mut::<SimulationState>() {
                Some(mut state) => state.step += 1,
                None => return,
            };
        }
    }
}

#[derive(Clone, Copy)]
pub enum SimulationSpeed {
    Normal,
    Fast,
}
impl Default for SimulationSpeed {
    fn default() -> Self {
        SimulationSpeed::Normal
    }
}
impl SimulationSpeed {
    fn steps_per_frame(&self) -> u32 {
        match self {
            Self::Normal => 1,
            Self::Fast => 4,
        }
    }
    pub fn label(&self) -> String {
        match self {
            Self::Normal => "1X".to_string(),
            Self::Fast => "4X".to_string(),
        }
    }
}
#[derive(Default)]
pub struct SimulationSettings {
    pub speed: SimulationSpeed,
}

fn update_sim_state_system(
    mut sim_state: ResMut<SimulationState>,
    q_emitter: Query<&PixieEmitter>,
    q_pixie: Query<Entity, With<Pixie>>,
) {
    if sim_state.done {
        return;
    }

    if q_emitter.iter().count() < 1 {
        return;
    }

    for emitter in q_emitter.iter() {
        if emitter.remaining > 0 {
            return;
        }
    }

    if q_pixie.iter().count() > 0 {
        return;
    }

    sim_state.done = true;
}