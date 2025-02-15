use bevy::prelude::*;

pub const FINISHED_ROAD: [Srgba; 3] = [
    bevy::color::palettes::tailwind::CYAN_600,
    bevy::color::palettes::tailwind::GREEN_600,
    bevy::color::palettes::tailwind::INDIGO_600,
];
pub const DRAWING_ROAD: [Srgba; 3] = [
    bevy::color::palettes::tailwind::CYAN_700,
    bevy::color::palettes::tailwind::GREEN_700,
    bevy::color::palettes::tailwind::INDIGO_700,
];
pub const PIXIE: [Srgba; 6] = [
    bevy::color::palettes::tailwind::CYAN_500,
    bevy::color::palettes::tailwind::FUCHSIA_500,
    bevy::color::palettes::tailwind::ORANGE_500,
    bevy::color::palettes::tailwind::VIOLET_500,
    bevy::color::palettes::tailwind::LIME_500,
    bevy::color::palettes::tailwind::YELLOW_500,
];

pub const BACKGROUND: Srgba = bevy::color::palettes::tailwind::GRAY_950;
pub const GRID: Srgba = bevy::color::palettes::tailwind::GRAY_900;
pub const LEVEL_NAME: Srgba = bevy::color::palettes::tailwind::GRAY_700;
pub const OBSTACLE: Srgba = bevy::color::palettes::tailwind::GRAY_900;

pub const DARK_OVERLAY: Color = Color::srgba(0.0, 0.0, 0.0, 0.7);

pub const UI_LABEL: Srgba = bevy::color::palettes::tailwind::NEUTRAL_200;
pub const UI_LABEL_MUTED: Srgba = bevy::color::palettes::tailwind::NEUTRAL_600;
pub const UI_LABEL_BAD: Srgba = bevy::color::palettes::tailwind::RED_400;
pub const UI_NORMAL_BUTTON: Srgba = bevy::color::palettes::tailwind::NEUTRAL_800;
pub const UI_HOVERED_BUTTON: Srgba = bevy::color::palettes::tailwind::NEUTRAL_700;
pub const UI_PRESSED_BUTTON: Srgba = bevy::color::palettes::tailwind::LIME_700;
pub const UI_BUTTON_TEXT: Srgba = bevy::color::palettes::tailwind::NEUTRAL_100;
pub const UI_PANEL_BACKGROUND: Srgba = bevy::color::palettes::tailwind::NEUTRAL_900;
