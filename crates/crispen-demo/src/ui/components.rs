//! Reusable UI components (labeled sliders, value readouts, section headers).

use bevy::prelude::*;
use bevy::ui_widgets::{
    observe, slider_self_update, Slider, SliderRange, SliderStep, SliderThumb, SliderValue,
    TrackClick,
};

use super::theme;

/// Identifies which `GradingParams` field a slider controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub enum ParamId {
    Temperature,
    Tint,
    Contrast,
    Pivot,
    MidtoneDetail,
    Shadows,
    Highlights,
    Saturation,
    Hue,
    LumaMix,
}

/// Marker on the slider entity linking it to a `ParamId`.
#[derive(Component)]
pub struct ParamSlider(pub ParamId);

/// Marker on the text entity that displays the slider's numeric value.
/// Stores the entity ID of the associated slider.
#[derive(Component)]
pub struct ParamValueLabel(pub Entity);

/// Spawn a labeled slider with a numeric readout.
///
/// Layout (vertical):
/// ```text
/// ┌──────────────┐
/// │  Label        │  <- TEXT_DIM, FONT_SIZE_LABEL
/// │  [===o====]   │  <- Slider (SLIDER_HEIGHT tall)
/// │  0.44         │  <- TEXT_PRIMARY, FONT_SIZE_VALUE
/// └──────────────┘
/// ```
pub fn spawn_param_slider(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    param_id: ParamId,
    range: (f32, f32),
    default_val: f32,
    step: f32,
) {
    parent
        .spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(2.0),
            min_width: Val::Px(60.0),
            flex_grow: 1.0,
            ..default()
        })
        .with_children(|col| {
            // Label
            col.spawn((
                Text::new(label),
                TextFont {
                    font_size: theme::FONT_SIZE_LABEL,
                    ..default()
                },
                TextColor(theme::TEXT_DIM),
            ));

            // Value text — spawned before slider so we can capture its entity ID
            let value_id = col
                .spawn((
                    Text::new(format!("{default_val:.2}")),
                    TextFont {
                        font_size: theme::FONT_SIZE_VALUE,
                        ..default()
                    },
                    TextColor(theme::TEXT_PRIMARY),
                ))
                .id();

            // Slider
            let slider_id = col
                .spawn((
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Stretch,
                        width: Val::Percent(100.0),
                        height: Val::Px(theme::SLIDER_HEIGHT),
                        ..default()
                    },
                    Slider {
                        track_click: TrackClick::Snap,
                    },
                    SliderValue(default_val),
                    SliderRange::new(range.0, range.1),
                    SliderStep(step),
                    ParamSlider(param_id),
                    observe(slider_self_update),
                    // Track + thumb children
                    Children::spawn((
                        // Track bar
                        Spawn((
                            Node {
                                height: Val::Px(4.0),
                                border_radius: BorderRadius::all(Val::Px(2.0)),
                                ..default()
                            },
                            BackgroundColor(theme::SLIDER_TRACK),
                        )),
                        // Thumb travel container (absolute-positioned overlay)
                        Spawn((
                            Node {
                                display: Display::Flex,
                                position_type: PositionType::Absolute,
                                left: Val::Px(0.0),
                                right: Val::Px(8.0),
                                top: Val::Px(0.0),
                                bottom: Val::Px(0.0),
                                ..default()
                            },
                            children![(
                                SliderThumb,
                                Node {
                                    display: Display::Flex,
                                    width: Val::Px(8.0),
                                    height: Val::Px(14.0),
                                    position_type: PositionType::Absolute,
                                    left: Val::Percent(0.0),
                                    top: Val::Px(2.0),
                                    border_radius: BorderRadius::all(Val::Px(2.0)),
                                    ..default()
                                },
                                BackgroundColor(theme::SLIDER_THUMB),
                            )],
                        )),
                    )),
                ))
                .id();

            // Attach ParamValueLabel to the value text, pointing at the slider
            col.commands().entity(value_id).insert(ParamValueLabel(slider_id));
        });
}

/// Return the identity (no-op) default for a given param.
pub fn param_default(id: ParamId) -> f32 {
    match id {
        ParamId::Temperature
        | ParamId::Tint
        | ParamId::MidtoneDetail
        | ParamId::Shadows
        | ParamId::Highlights
        | ParamId::Hue
        | ParamId::LumaMix => 0.0,
        ParamId::Contrast | ParamId::Saturation => 1.0,
        ParamId::Pivot => 0.435,
    }
}

/// Return the (min, max) slider range for a given param.
pub fn param_range(id: ParamId) -> (f32, f32) {
    match id {
        ParamId::Temperature | ParamId::Tint => (-100.0, 100.0),
        ParamId::Contrast | ParamId::Saturation => (0.0, 4.0),
        ParamId::Pivot | ParamId::LumaMix => (0.0, 1.0),
        ParamId::MidtoneDetail => (-1.0, 1.0),
        ParamId::Shadows | ParamId::Highlights => (-1.0, 1.0),
        ParamId::Hue => (-180.0, 180.0),
    }
}

/// Return a reasonable step increment for a given param.
pub fn param_step(id: ParamId) -> f32 {
    match id {
        ParamId::Temperature | ParamId::Tint => 1.0,
        ParamId::Hue => 1.0,
        ParamId::Contrast | ParamId::Saturation => 0.01,
        ParamId::Pivot | ParamId::LumaMix => 0.005,
        ParamId::MidtoneDetail | ParamId::Shadows | ParamId::Highlights => 0.01,
    }
}

/// Update slider thumb position and value label text when `SliderValue` changes.
pub fn update_param_slider_visuals(
    sliders: Query<
        (Entity, &SliderValue, &SliderRange),
        (Changed<SliderValue>, With<ParamSlider>),
    >,
    children_q: Query<&Children>,
    mut thumbs: Query<&mut Node, With<SliderThumb>>,
    mut labels: Query<(&ParamValueLabel, &mut Text)>,
) {
    for (slider_ent, value, range) in sliders.iter() {
        // Update thumb position
        let position = range.thumb_position(value.0) * 100.0;
        for child in children_q.iter_descendants(slider_ent) {
            if let Ok(mut thumb_node) = thumbs.get_mut(child) {
                thumb_node.left = Val::Percent(position);
            }
        }
    }

    // Update value labels
    for (label, mut text) in labels.iter_mut() {
        if let Ok((_, value, _)) = sliders.get(label.0) {
            **text = format!("{:.2}", value.0);
        }
    }
}
