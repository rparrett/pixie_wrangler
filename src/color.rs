use bevy::prelude::*;

pub const FINISHED_ROAD: [Color; 3] = [
    Color::rgb(0.251, 0.435, 0.729),
    Color::rgb(0.247, 0.725, 0.314),
    Color::rgb(0.247, 0.725, 0.714),
];
pub const DRAWING_ROAD: [Color; 3] = [
    Color::rgb(0.102, 0.18, 0.298),
    Color::rgb(0.102, 0.298, 0.125),
    Color::rgb(0.102, 0.298, 0.298),
];
pub const PIXIE: [Color; 6] = [
    Color::AQUAMARINE,
    Color::PINK,
    Color::ORANGE,
    Color::PURPLE,
    Color::DARK_GREEN,
    Color::YELLOW,
];

pub const BACKGROUND: Color = Color::rgb(0.05, 0.066, 0.09);
pub const GRID: Color = Color::rgb(0.086, 0.105, 0.133);
pub const OBSTACLE: Color = Color::rgb(0.086, 0.105, 0.133);
pub const BOTTOM_BAR_BACKGROUND: Color = Color::rgb(0.09, 0.11, 0.13);
pub const DIALOG_BACKGROUND: Color = Color::rgb(0.2, 0.2, 0.2);
pub const OVERLAY: Color = Color::rgba(0.0, 0.0, 0.0, 0.7);

pub const UI_WHITE: Color = Color::rgb(0.788, 0.82, 0.851);
pub const UI_GREY_RED: Color = Color::rgb(1.0, 0.341, 0.341);
pub const UI_NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
pub const UI_HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
pub const UI_PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);
pub const UI_BUTTON_TEXT: Color = Color::rgb(0.9, 0.9, 0.9);
