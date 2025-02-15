use bevy::prelude::*;
use level_select::LevelSelectPlugin;
use radio_button::{RadioButton, RadioButtonPlugin};
use score_dialog::ScoreDialogPlugin;

use crate::color;

pub mod level_select;
pub mod radio_button;
pub mod score_dialog;

pub struct UiPlugin;
impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((RadioButtonPlugin, LevelSelectPlugin, ScoreDialogPlugin));
        app.add_systems(Update, button_system);
    }
}

fn button_system(
    mut q_interaction: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>, Without<RadioButton>),
    >,
) {
    for (interaction, mut color) in q_interaction.iter_mut() {
        match *interaction {
            Interaction::Pressed => *color = color::UI_PRESSED_BUTTON.into(),
            Interaction::Hovered => *color = color::UI_HOVERED_BUTTON.into(),
            Interaction::None => *color = color::UI_NORMAL_BUTTON.into(),
        }
    }
}
