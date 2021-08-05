use crate::ButtonMaterials;
use bevy::prelude::*;

pub struct RadioButtonPlugin;
pub struct RadioButtonGroup {
    pub entities: Vec<Entity>,
}
pub struct RadioButton {
    pub selected: bool,
}
pub struct RadioButtonGroupRelation(pub Entity);

impl Plugin for RadioButtonPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(radio_button_system.system().label("radio_button_system"));
        app.add_system(
            radio_button_group_system
                .system()
                .label("radio_button_group_system")
                .after("radio_button_system"),
        );
    }
}

fn radio_button_group_system(
    mut q: QuerySet<(
        Query<(Entity, &RadioButton, &RadioButtonGroupRelation), Changed<RadioButton>>,
        Query<&mut RadioButton>,
    )>,
    q_radio_group: Query<&RadioButtonGroup>,
) {
    let mut unselect = vec![];
    for (entity, radio, group_rel) in q.q0().iter() {
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
        if let Ok(mut other_radio) = q.q1_mut().get_mut(*entity) {
            other_radio.selected = false;
        }
    }
}

fn radio_button_system(
    button_materials: Res<ButtonMaterials>,
    mut interaction_query: Query<
        (&mut RadioButton, &Interaction, &mut Handle<ColorMaterial>),
        (Changed<Interaction>, With<Button>, With<RadioButton>),
    >,
) {
    for (mut radio, interaction, mut material) in interaction_query.iter_mut() {
        match *interaction {
            Interaction::Clicked => {
                *material = button_materials.pressed.clone();

                radio.selected = true;
            }
            Interaction::Hovered => {
                *material = button_materials.hovered.clone();
            }
            Interaction::None => {
                *material = button_materials.normal.clone();
            }
        }
    }
}
