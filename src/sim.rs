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

// struct SimulationStage {
//     step: f64,
//     accumulator: f64,
//     stage: SystemStage,
// }

// impl Stage for SimulationStage {
//     fn run(&mut self, world: &mut World) {
//         if let Some(state) = world.get_resource::<SimulationState>() {
//             if !state.started || state.done {
//                 return;
//             }
//         }

//         let delta = match world.get_resource::<Time>() {
//             Some(time) => time.delta_seconds_f64(),
//             None => return,
//         };

//         let speed = match world.get_resource::<SimulationSettings>() {
//             Some(settings) => settings.speed,
//             None => return,
//         };

//         self.accumulator += delta * speed.scale();

//         while self.accumulator > self.step {
//             self.accumulator -= self.step;

//             self.stage.run(world);

//             match world.get_resource_mut::<SimulationState>() {
//                 Some(mut state) => state.tick += 1,
//                 None => return,
//             };

//             if let Some(state) = world.get_resource::<SimulationState>() {
//                 if !state.started || state.done {
//                     self.accumulator = 0.0;
//                     return;
//                 }
//             }
//         }
//     }
// }

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
        return;
    }

    let speed = world.resource::<SimulationSettings>().speed;
    let delta = world.resource::<Time>().delta_seconds_f64();

    let mut steps = world.resource_mut::<SimulationSteps>();
    steps.accumulator += delta * speed.scale();

    let mut check_again = true;
    while check_again {
        let mut steps = world.resource_mut::<SimulationSteps>();
        steps.accumulator -= steps.step;

        if steps.accumulator > steps.step {
            world.run_schedule(SimulationSchedule);
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
