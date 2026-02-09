//! OCIO display/view/input controls for the native Bevy UI.

use bevy::prelude::*;
use crispen_bevy::resources::OcioColorManagement;

use super::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OcioDropdownKind {
    Display,
    View,
    Input,
}

impl OcioDropdownKind {
    fn title(self) -> &'static str {
        match self {
            Self::Display => "Display",
            Self::View => "View",
            Self::Input => "Input",
        }
    }
}

#[derive(Resource, Default)]
pub struct OcioDropdownUiState {
    pub open: Option<OcioDropdownKind>,
    pub menus_built: bool,
}

#[derive(Component)]
pub struct OcioDropdownButton(pub OcioDropdownKind);

#[derive(Component)]
pub struct OcioDropdownMenu(pub OcioDropdownKind);

#[derive(Component)]
pub struct OcioDropdownLabel(pub OcioDropdownKind);

#[derive(Component)]
pub struct OcioDropdownOption {
    pub kind: OcioDropdownKind,
    pub value: String,
}

pub fn spawn_ocio_controls(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            margin: UiRect::bottom(Val::Px(2.0)),
            ..default()
        })
        .with_children(|col| {
            col.spawn((
                Text::new("OCIO"),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
                TextColor(theme::TEXT_DIM),
            ));

            col.spawn(Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                width: Val::Px(theme::WHEEL_DIAL_ROW_WIDTH),
                ..default()
            })
            .with_children(|row| {
                spawn_dropdown(row, OcioDropdownKind::Display, 180.0);
                spawn_dropdown(row, OcioDropdownKind::View, 220.0);
                spawn_dropdown(row, OcioDropdownKind::Input, 220.0);
            });
        });
}

fn spawn_dropdown(parent: &mut ChildSpawnerCommands, kind: OcioDropdownKind, width_px: f32) {
    parent
        .spawn(Node {
            position_type: PositionType::Relative,
            width: Val::Px(width_px),
            ..default()
        })
        .with_children(|dropdown| {
            dropdown
                .spawn((
                    OcioDropdownButton(kind),
                    Button,
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        width: Val::Percent(100.0),
                        height: Val::Px(24.0),
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(theme::BG_CONTROL),
                    BorderColor::all(theme::BORDER_SUBTLE),
                ))
                .with_children(|button| {
                    button.spawn((
                        OcioDropdownLabel(kind),
                        Text::new(format!("{}: -", kind.title())),
                        TextFont {
                            font_size: theme::FONT_SIZE_LABEL,
                            ..default()
                        },
                        TextColor(theme::TEXT_PRIMARY),
                    ));
                    button.spawn((
                        Text::new("v"),
                        TextFont {
                            font_size: 10.0,
                            ..default()
                        },
                        TextColor(theme::TEXT_DIM),
                    ));
                });

            dropdown.spawn((
                OcioDropdownMenu(kind),
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(24.0),
                    left: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    display: Display::None,
                    flex_direction: FlexDirection::Column,
                    border: UiRect::all(Val::Px(1.0)),
                    max_height: Val::Px(220.0),
                    overflow: Overflow::clip_y(),
                    ..default()
                },
                BackgroundColor(theme::BG_CONTROL),
                BorderColor::all(theme::BORDER_SUBTLE),
                GlobalZIndex(100),
                ZIndex(10),
            ));
        });
}

pub fn handle_ocio_dropdown_interactions(
    button_interactions: Query<
        (&Interaction, &OcioDropdownButton),
        (Changed<Interaction>, With<Button>),
    >,
    option_interactions: Query<
        (&Interaction, &OcioDropdownOption),
        (Changed<Interaction>, With<Button>),
    >,
    mut ui_state: ResMut<OcioDropdownUiState>,
    ocio: Option<ResMut<OcioColorManagement>>,
) {
    let Some(mut ocio) = ocio else {
        return;
    };

    for (interaction, button) in button_interactions.iter() {
        if *interaction == Interaction::Pressed {
            ui_state.open = if ui_state.open == Some(button.0) {
                None
            } else {
                Some(button.0)
            };
        }
    }

    for (interaction, option) in option_interactions.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match option.kind {
            OcioDropdownKind::Display => {
                if ocio.display != option.value {
                    ocio.display = option.value.clone();
                    let default_view = ocio.config.default_view(&ocio.display);
                    ocio.view = if default_view.is_empty() {
                        ocio.config
                            .views(&ocio.display)
                            .into_iter()
                            .next()
                            .unwrap_or_default()
                    } else {
                        default_view
                    };
                    ocio.dirty = true;
                }
            }
            OcioDropdownKind::View => {
                if ocio.view != option.value {
                    ocio.view = option.value.clone();
                    ocio.dirty = true;
                }
            }
            OcioDropdownKind::Input => {
                if ocio.input_space != option.value {
                    ocio.input_space = option.value.clone();
                    ocio.dirty = true;
                }
            }
        }

        ui_state.open = None;
    }
}

pub fn rebuild_ocio_dropdown_menus(
    mut commands: Commands,
    ocio: Option<Res<OcioColorManagement>>,
    mut ui_state: ResMut<OcioDropdownUiState>,
    menus: Query<(Entity, &OcioDropdownMenu, Option<&Children>)>,
) {
    let Some(ocio) = ocio else {
        return;
    };

    if !ocio.is_changed() && ui_state.menus_built {
        return;
    }

    for (menu_entity, menu_kind, children) in menus.iter() {
        if let Some(children) = children {
            for child in children.iter() {
                commands.entity(child).despawn();
            }
        }

        let values: Vec<String> = match menu_kind.0 {
            OcioDropdownKind::Display => ocio.config.displays(),
            OcioDropdownKind::View => ocio.config.views(&ocio.display),
            OcioDropdownKind::Input => ocio.config.color_space_names(),
        };

        commands.entity(menu_entity).with_children(|menu| {
            for value in values {
                menu.spawn((
                    OcioDropdownOption {
                        kind: menu_kind.0,
                        value: value.clone(),
                    },
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                        border: UiRect::bottom(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(theme::BG_CONTROL),
                    BorderColor::all(theme::BORDER_SUBTLE),
                    children![(
                        Text::new(value),
                        TextFont {
                            font_size: theme::FONT_SIZE_LABEL,
                            ..default()
                        },
                        TextColor(theme::TEXT_PRIMARY),
                    )],
                ));
            }
        });
    }

    ui_state.menus_built = true;
}

pub fn sync_ocio_dropdown_ui(
    ui_state: Res<OcioDropdownUiState>,
    ocio: Option<Res<OcioColorManagement>>,
    mut ui_parts: ParamSet<(
        Query<(&OcioDropdownLabel, &mut Text)>,
        Query<(&OcioDropdownMenu, &mut Node)>,
        Query<(&OcioDropdownOption, &mut BackgroundColor)>,
    )>,
) {
    let Some(ocio) = ocio else {
        return;
    };

    if !(ui_state.is_changed() || ocio.is_changed()) {
        return;
    }

    for (label, mut text) in ui_parts.p0().iter_mut() {
        let current = match label.0 {
            OcioDropdownKind::Display => &ocio.display,
            OcioDropdownKind::View => &ocio.view,
            OcioDropdownKind::Input => &ocio.input_space,
        };
        *text = Text::new(format!("{}: {}", label.0.title(), current));
    }

    for (menu, mut node) in ui_parts.p1().iter_mut() {
        node.display = if ui_state.open == Some(menu.0) {
            Display::Flex
        } else {
            Display::None
        };
    }

    for (option, mut bg) in ui_parts.p2().iter_mut() {
        let selected = match option.kind {
            OcioDropdownKind::Display => ocio.display == option.value,
            OcioDropdownKind::View => ocio.view == option.value,
            OcioDropdownKind::Input => ocio.input_space == option.value,
        };

        *bg = if selected {
            BackgroundColor(Color::srgb(0.29, 0.29, 0.29))
        } else {
            BackgroundColor(theme::BG_CONTROL)
        };
    }
}
