use crate::pixie::{
    collide_pixies_system, emit_pixies_system, explode_pixies_system, move_pixies_system, Pixie,
    PixieEmitter,
};
use bevy::{ecs::schedule::ScheduleLabel, prelude::*};

pub struct SimulationPlugin;
impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        let mut schedule = Schedule::new();

        // explicit ordering for determinism
        schedule.add_systems(
            (
                collide_pixies_system,
                move_pixies_system,
                emit_pixies_system,
                explode_pixies_system,
                update_sim_state_system,
            )
                .chain(),
        );

        app.add_schedule(SimulationSchedule, schedule);
        app.init_resource::<SimulationSettings>();
        app.init_resource::<SimulationState>();
        app.init_resource::<SimulationSteps>();

        app.add_system(run_simulation);
    }
}

pub const SIMULATION_TIMESTEP: f32 = 0.016_666_668;

#[derive(ScheduleLabel, Debug, PartialEq, Eq, Clone, Hash)]
pub struct SimulationSchedule;

#[derive(Resource, Default)]
pub struct SimulationState {
    pub started: bool,
    pub tick: u32,
    pub done: bool,
}

#[derive(Resource)]
struct SimulationSteps {
    step: f64,
    accumulator: f64,
}
impl Default for SimulationSteps {
    fn default() -> Self {
        Self {
            step: SIMULATION_TIMESTEP as f64,
            accumulator: 0.,
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
    fn scale(&self) -> f64 {
        match self {
            Self::Normal => 1.0,
            Self::Fast => 4.0,
        }
    }
    pub fn label(&self) -> String {
        match self {
            Self::Normal => "1X".to_string(),
            Self::Fast => "4X".to_string(),
        }
    }
}
#[derive(Resource, Default)]
pub struct SimulationSettings {
    pub speed: SimulationSpeed,
}

fn run_simulation(world: &mut World) {
    let state = world.resource::<SimulationState>();
    if !state.started || state.done {
        // If the sim just ended or was just stopped, reset the
        // accumulator.
        if world.is_resource_changed::<SimulationState>() {
            let mut steps = world.resource_mut::<SimulationSteps>();
            steps.accumulator = 0.;
        }

        return;
    }

    let speed = world.resource::<SimulationSettings>().speed;
    let delta = world.resource::<Time>().delta_seconds_f64();

    let mut steps = world.resource_mut::<SimulationSteps>();
    steps.accumulator += delta * speed.scale();

    let mut check_again = true;
    while check_again {
        let mut steps = world.resource_mut::<SimulationSteps>();

        if steps.accumulator > steps.step {
            steps.accumulator -= steps.step;
            world.run_schedule(SimulationSchedule);

            {
                let mut state = world.resource_mut::<SimulationState>();
                state.tick += 1;
            }

            // If sim finished, don't run schedule again.
            let state = world.resource::<SimulationState>();
            if state.done || !state.started {
                check_again = false;
            }
        } else {
            check_again = false;
        }
    }
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
