use bevy::prelude::*;

pub struct RadioButtonPlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct RadioButtonSet;

impl Plugin for RadioButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, interaction.in_set(RadioButtonSet));
        app.add_systems(
            Update,
            update_groups.after(interaction).in_set(RadioButtonSet),
        );
    }
}

/// Holds the state of the radio button and also controls the button group.
///
/// Setting this to `true` will cause the value to be set to `false` for every
/// other button in the group.
#[derive(Component)]
pub struct RadioButton {
    pub selected: bool,
}
#[derive(Component)]
pub struct RadioButtonGroupRelation(pub Entity);
#[derive(Component)]
pub struct RadioButtonGroup {
    pub entities: Vec<Entity>,
}

fn update_groups(
    mut button_set: ParamSet<(
        Query<(Entity, &RadioButtonGroupRelation), Changed<RadioButton>>,
        Query<&mut RadioButton>,
    )>,
    groups: Query<&RadioButtonGroup>,
) {
    // TODO this seems problematic if multiple buttons in the same group
    // get changed in a particular frame.

    let mut unselect: Vec<Entity> = vec![];

    for (entity, group_rel) in &button_set.p0() {
        let Ok(group) = groups.get(group_rel.0) else {
            warn!("Radio button without group relation.");
            continue;
        };

        unselect.extend(group.entities.iter().filter(|other| **other != entity));
    }

    let mut buttons = button_set.p1();
    let mut button_iter = buttons.iter_many_mut(unselect);
    while let Some(mut other_button) = button_iter.fetch_next() {
        other_button.selected = false;
    }
}

fn interaction(
    mut interactions: Query<
        (&mut RadioButton, &Interaction),
        (Changed<Interaction>, With<Button>, With<RadioButton>),
    >,
) {
    for (mut button, interaction) in &mut interactions {
        if *interaction == Interaction::Pressed {
            button.selected = true;
        }
    }
}
