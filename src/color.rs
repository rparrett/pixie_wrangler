use bevy::prelude::*;

pub const FINISHED_ROAD: [Color; 3] = [
    Color::srgb(0.251, 0.435, 0.729),
    Color::srgb(0.247, 0.725, 0.314),
    Color::srgb(0.247, 0.725, 0.714),
];
pub const DRAWING_ROAD: [Color; 3] = [
    Color::srgb(0.102, 0.18, 0.298),
    Color::srgb(0.102, 0.298, 0.125),
    Color::srgb(0.102, 0.298, 0.298),
];
pub const PIXIE: [Srgba; 6] = [
    bevy::color::palettes::css::AQUAMARINE,
    bevy::color::palettes::css::DEEP_PINK,
    bevy::color::palettes::css::ORANGE,
    bevy::color::palettes::css::PURPLE,
    bevy::color::palettes::css::DARK_GREEN,
    bevy::color::palettes::css::YELLOW,
];

pub const BACKGROUND: Color = Color::srgb(0.05, 0.066, 0.09);
pub const GRID: Color = Color::srgb(0.086, 0.105, 0.133);
pub const OBSTACLE: Color = Color::srgb(0.086, 0.105, 0.133);
pub const BOTTOM_BAR_BACKGROUND: Color = Color::srgb(0.09, 0.11, 0.13);
pub const DIALOG_BACKGROUND: Color = Color::srgb(0.2, 0.2, 0.2);
pub const OVERLAY: Color = Color::srgba(0.0, 0.0, 0.0, 0.7);

pub const UI_WHITE: Color = Color::srgb(0.788, 0.82, 0.851);
pub const UI_GREY_RED: Color = Color::srgb(1.0, 0.341, 0.341);
pub const UI_NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
pub const UI_HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
pub const UI_PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);
pub const UI_BUTTON_TEXT: Color = Color::srgb(0.9, 0.9, 0.9);
