use std::time::Duration;

use crate::{
    pixie::{
        collide_pixies_system, emit_pixies_system, explode_pixies_system, move_pixies_system,
        Pixie, PixieEmitter,
    },
    pixie_button_system,
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

        // TODO this must run after buffers from pixie_button_system are applied
        // so that emitters are created on time. It might be nice to move sim entity
        // initialization into the sim schedule.
        app.add_systems(
            (apply_system_buffers, run_simulation)
                .chain()
                .after(pixie_button_system),
        );
    }
}

pub const SIMULATION_TIMESTEP: f32 = 0.016_666_668;

#[derive(ScheduleLabel, Debug, PartialEq, Eq, Clone, Hash)]
pub struct SimulationSchedule;

#[derive(Resource, Default, PartialEq)]
pub enum SimulationState {
    #[default]
    NotStarted,
    Running,
    Finished,
}

#[derive(Resource)]
pub struct SimulationSteps {
    timestep: Duration,
    accumulator: Duration,
    step: u32,
}
impl Default for SimulationSteps {
    fn default() -> Self {
        Self {
            timestep: Duration::from_secs_f32(SIMULATION_TIMESTEP),
            accumulator: Duration::ZERO,
            step: 0,
        }
    }
}
impl SimulationSteps {
    fn expend(&mut self) -> bool {
        if let Some(new_value) = self.accumulator.checked_sub(self.timestep) {
            self.accumulator = new_value;
            self.step += 1;

            true
        } else {
            false
        }
    }

    fn tick(&mut self, delta: Duration) {
        self.accumulator += delta;
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn get_elapsed_f32(&self) -> f32 {
        self.step as f32 * SIMULATION_TIMESTEP
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
    fn scale(&self) -> u32 {
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
#[derive(Resource, Default)]
pub struct SimulationSettings {
    pub speed: SimulationSpeed,
}

fn run_simulation(world: &mut World) {
    let state = world.resource_mut::<SimulationState>();
    if *state != SimulationState::Running {
        return;
    }

    if state.is_changed() {
        world.resource_mut::<SimulationSteps>().reset();
    }

    let speed = world.resource::<SimulationSettings>().speed;
    let delta = world.resource::<Time>().delta();

    let mut steps = world.resource_mut::<SimulationSteps>();
    steps.tick(delta * speed.scale());

    let mut check_again = true;
    while check_again {
        let mut steps = world.resource_mut::<SimulationSteps>();

        if steps.expend() {
            world.run_schedule(SimulationSchedule);

            // If sim finished, don't run schedule again, even if there is
            // enough time in the accumulator.
            let state = world.resource::<SimulationState>();
            if *state != SimulationState::Running {
                check_again = false;
            }
        } else {
            check_again = false;
        }
    }
}

fn update_sim_state_system(
    mut sim_state: ResMut<SimulationState>,
    sim_steps: Res<SimulationSteps>,
    q_emitter: Query<&PixieEmitter>,
    q_pixie: Query<Entity, With<Pixie>>,
) {
    if *sim_state != SimulationState::Running {
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

    info!("Sim finished in {} ticks", sim_steps.step);

    *sim_state = SimulationState::Finished;
}
