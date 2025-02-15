use bevy::prelude::*;

pub struct RadioButtonPlugin;
#[derive(Component)]
pub struct RadioButtonGroup {
    pub entities: Vec<Entity>,
}
#[derive(Component)]
pub struct RadioButton {
    pub selected: bool,
}
#[derive(Component)]
pub struct RadioButtonGroupRelation(pub Entity);

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub struct RadioButtonSet;

impl Plugin for RadioButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, radio_button_system.in_set(RadioButtonSet));
        app.add_systems(
            Update,
            radio_button_group_system
                .after(radio_button_system)
                .in_set(RadioButtonSet),
        );
    }
}

fn radio_button_group_system(
    mut q: ParamSet<(
        Query<(Entity, &RadioButton, &RadioButtonGroupRelation), Changed<RadioButton>>,
        Query<&mut RadioButton>,
    )>,
    q_radio_group: Query<&RadioButtonGroup>,
) {
    let mut unselect = vec![];
    for (entity, radio, group_rel) in q.p0().iter() {
        if let Ok(radio_group) = q_radio_group.get(group_rel.0) {
            if radio.selected {
                for other_entity in radio_group.entities.iter() {
                    if *other_entity != entity {
                        unselect.push(*other_entity);
                    }
                }
            }
        }
    }

    for entity in unselect.iter() {
        if let Ok(mut other_radio) = q.p1().get_mut(*entity) {
            other_radio.selected = false;
        }
    }
}

fn radio_button_system(
    mut interaction_query: Query<
        (&mut RadioButton, &Interaction),
        (Changed<Interaction>, With<Button>, With<RadioButton>),
    >,
) {
    for (mut radio, interaction) in interaction_query.iter_mut() {
        if *interaction == Interaction::Pressed {
            radio.selected = true;
        }
    }
}
