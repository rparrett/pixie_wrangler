use crate::pixie::{
    collide_pixies_system, emit_pixies_system, explode_pixies_system, move_pixies_system, Pixie,
    PixieEmitter,
};
use bevy::prelude::*;

pub struct SimulationPlugin;
impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        // app.init_resource::<SimulationSettings>();
        // app.init_resource::<SimulationState>();
        // app.add_stage_after(
        //     CoreStage::Update,
        //     "simulation",
        //     SimulationStage {
        //         step: SIMULATION_TIMESTEP as f64,
        //         accumulator: 0.0,
        //         stage: SystemStage::parallel()
        //             // need to run these in the same order every time for scores
        //             // to be deterministic. probably.
        //             .with_system(
        //                 collide_pixies_system
        //                     .label("collide_pixies")
        //                     .before("move_pixies"),
        //             )
        //             .with_system(move_pixies_system.label("move_pixies"))
        //             .with_system(emit_pixies_system.label("emit_pixies").after("move_pixies"))
        //             .with_system(explode_pixies_system.after("collide_pixies"))
        //             .with_system(update_sim_state_system.after("emit_pixies")),
        //     },
        // );
    }
}

pub const SIMULATION_TIMESTEP: f32 = 0.016_666_668;

#[derive(Resource, Default)]
pub struct SimulationState {
    pub started: bool,
    pub tick: u32,
    pub done: bool,
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
