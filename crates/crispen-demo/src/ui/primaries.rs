//! Primaries panel (Lift / Gamma / Gain / Offset wheels + dial knobs).
//!
//! Layout matches DaVinci Resolve's Primaries panel:
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │  Temp  │  Tint  │  Contrast │  Pivot  │  Mid Detail            │
//! ├────────┼────────┼───────────┼─────────┼────────────────────────┤
//! │  LIFT  │  GAMMA │   GAIN    │  OFFSET │                        │
//! │(wheel) │(wheel) │ (wheel)   │(wheel)  │                        │
//! ├────────┼────────┼───────────┼─────────┼────────────────────────┤
//! │ Shadows│Highlights│Saturation│  Hue    │  Luma Mix              │
//! └──────────────────────────────────────────────────────────────────┘
//! ```

use bevy::prelude::*;

use super::color_wheel::{WheelType, color_wheel};
use super::components::{ParamId, param_default, param_label, param_range, param_step};
use super::dial::{DialLabelPosition, spawn_param_dial};
use super::theme;

/// Spawn the primaries panel as a child of the given parent.
pub fn spawn_primaries_panel(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                height: Val::Px(theme::PRIMARIES_PANEL_HEIGHT),
                padding: UiRect::all(Val::Px(theme::PANEL_PADDING)),
                row_gap: Val::Px(6.0),
                width: Val::Percent(100.0),
                border: UiRect::top(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(theme::BG_PANEL),
            BorderColor::all(theme::BORDER_SUBTLE),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("Primaries"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(theme::TEXT_PRIMARY),
            ));
            spawn_top_dials(panel);
            spawn_wheels_row(panel);
            spawn_bottom_dials(panel);
        });
}

/// Convenience wrapper that spawns a dial from a `ParamId`.
fn dial(parent: &mut ChildSpawnerCommands, id: ParamId, label_position: DialLabelPosition) {
    spawn_param_dial(
        parent,
        param_label(id),
        id,
        param_range(id),
        param_default(id),
        param_step(id),
        label_position,
    );
}

fn spawn_top_dials(panel: &mut ChildSpawnerCommands) {
    panel
        .spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            align_self: AlignSelf::Center,
            width: Val::Px(theme::WHEEL_DIAL_ROW_WIDTH),
            margin: UiRect::top(Val::Px(theme::TOP_DIAL_ROW_MARGIN_TOP)),
            ..default()
        })
        .with_children(|row| {
            dial(row, ParamId::Temperature, DialLabelPosition::Above);
            dial(row, ParamId::Tint, DialLabelPosition::Above);
            dial(row, ParamId::Contrast, DialLabelPosition::Above);
            dial(row, ParamId::Pivot, DialLabelPosition::Above);
            dial(row, ParamId::MidtoneDetail, DialLabelPosition::Above);
        });
}

fn spawn_wheels_row(panel: &mut ChildSpawnerCommands) {
    panel
        .spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            align_self: AlignSelf::Center,
            width: Val::Px(theme::WHEEL_GROUP_WIDTH),
            height: Val::Px(theme::WHEEL_SIZE + 20.0),
            min_height: Val::Px(theme::WHEEL_SIZE + 20.0),
            max_height: Val::Px(theme::WHEEL_SIZE + 20.0),
            ..default()
        })
        .with_children(|row| {
            for wheel_type in [
                WheelType::Lift,
                WheelType::Gamma,
                WheelType::Gain,
                WheelType::Offset,
            ] {
                row.spawn(color_wheel(wheel_type));
            }
        });
}

fn spawn_bottom_dials(panel: &mut ChildSpawnerCommands) {
    panel
        .spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            align_self: AlignSelf::Center,
            width: Val::Px(theme::WHEEL_DIAL_ROW_WIDTH),
            margin: UiRect::top(Val::Px(theme::BOTTOM_DIAL_ROW_MARGIN_TOP)),
            ..default()
        })
        .with_children(|row| {
            dial(row, ParamId::Shadows, DialLabelPosition::Below);
            dial(row, ParamId::Highlights, DialLabelPosition::Below);
            dial(row, ParamId::Saturation, DialLabelPosition::Below);
            dial(row, ParamId::Hue, DialLabelPosition::Below);
            dial(row, ParamId::LumaMix, DialLabelPosition::Below);
        });
}
