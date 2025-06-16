use bevy::{prelude::*, winit::WinitSettings};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_observer(add_items)
        .run();
}

#[derive(Component)]
struct ItemPanel;
#[derive(Component)]
struct PanelContent;

fn setup(mut commands: Commands) {
    // ui camera
    commands.spawn(Camera2d);

    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            padding: UiRect::all(Val::Px(20.0)),
            ..default()
        },
        children![(
            Node {
                display: Display::Grid,
                grid_template_columns: vec![GridTrack::flex(0.75), GridTrack::flex(0.25)],
                column_gap: Val::Px(20.),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            children![(ItemPanel, panel()), panel(),]
        )],
    ));
}

fn panel() -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        children![
            (
                Node {
                    width: Val::Px(100.),
                    padding: UiRect::all(Val::Px(5.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(1.0, 0.0, 0.0).into()),
                children![(Text::new("Panel"),)]
            ),
            (
                PanelContent,
                Node {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    padding: UiRect::all(Val::Px(5.0)),
                    flex_wrap: FlexWrap::Wrap,
                    overflow: Overflow::scroll_y(),
                    row_gap: Val::Px(5.0),
                    column_gap: Val::Px(5.0),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.0, 1.0, 0.0).into()),
                children![(Text::new("Panel Content"),)]
            )
        ],
    )
}

fn item(index: usize) -> impl Bundle {
    (
        Node {
            width: Val::Px(50.0),
            height: Val::Px(50.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::srgb(0.0, 0.0, 1.0).into()),
        children![Text::new(format!("{}", index)),],
    )
}

fn add_items(
    trigger: Trigger<OnAdd, PanelContent>,
    panel_content: Query<(), With<ItemPanel>>,
    children: Query<&ChildOf>,
    mut commands: Commands,
) {
    for entity in children.iter_ancestors(trigger.target()) {
        if panel_content.get(entity).is_ok() {
            commands
                .entity(trigger.target())
                .despawn_related::<Children>();
            for i in 0..50 {
                commands.entity(trigger.target()).with_child(item(i + 1));
            }
        }
    }
}
