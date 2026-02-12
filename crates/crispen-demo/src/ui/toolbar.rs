//! Top toolbar containing color-management dropdowns and viewer toggles.

use bevy::picking::Pickable;
use bevy::picking::events::Click;
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use crispen_bevy::resources::GradingState;
#[cfg(feature = "ocio")]
use crispen_bevy::resources::OcioColorManagement;
use crispen_core::transform::params::ColorSpaceId;

use super::theme;

/// Runtime UI state for the top toolbar.
#[derive(Resource, Default)]
pub struct ToolbarState {
    pub active_dropdown: Option<ToolbarDropdownKind>,
    pub split_view_active: bool,
    pub ofx_panel_visible: bool,
}

/// Kinds of dropdowns supported by the toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(clippy::enum_variant_names)]
pub enum ToolbarDropdownKind {
    InputColorspace,
    WorkingColorspace,
    #[cfg(not(feature = "ocio"))]
    OutputColorspace,
    #[cfg(feature = "ocio")]
    OcioDisplay,
    #[cfg(feature = "ocio")]
    OcioView,
}

impl ToolbarDropdownKind {
    fn title(self) -> &'static str {
        match self {
            Self::InputColorspace => "Input",
            Self::WorkingColorspace => "Working",
            #[cfg(not(feature = "ocio"))]
            Self::OutputColorspace => "Output",
            #[cfg(feature = "ocio")]
            Self::OcioDisplay => "Display",
            #[cfg(feature = "ocio")]
            Self::OcioView => "View",
        }
    }
}

/// Marker for toolbar root node.
#[derive(Component)]
pub struct ToolbarRoot;

/// Marker for a dropdown button.
#[derive(Component)]
pub struct ToolbarDropdownButton(pub ToolbarDropdownKind);

/// Marker for a dropdown menu list.
#[derive(Component)]
pub struct ToolbarDropdownMenu(pub ToolbarDropdownKind);

/// Marker for dropdown selected-value label.
#[derive(Component)]
pub struct ToolbarDropdownLabel(pub ToolbarDropdownKind);

/// Marker for dropdown option entries.
#[derive(Component)]
pub struct ToolbarDropdownOption {
    pub kind: ToolbarDropdownKind,
    pub value: String,
}

/// Marker for split-view toggle button.
#[derive(Component)]
pub struct SplitViewToggleButton;

/// Marker for OFX-panel toggle button.
#[derive(Component)]
pub struct OfxPanelToggleButton;

/// Spawn the top toolbar row.
pub fn spawn_toolbar(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            ToolbarRoot,
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                width: Val::Percent(100.0),
                height: Val::Px(theme::TOOLBAR_HEIGHT),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                border: UiRect::bottom(Val::Px(1.0)),
                overflow: Overflow::visible(),
                ..default()
            },
            BackgroundColor(theme::BG_PANEL),
            BorderColor::all(theme::BORDER_SUBTLE),
            GlobalZIndex(500),
            ZIndex(50),
        ))
        .with_children(|toolbar| {
            toolbar
                .spawn(Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(8.0),
                    ..default()
                })
                .with_children(|left| {
                    spawn_dropdown(left, ToolbarDropdownKind::InputColorspace);
                    spawn_dropdown(left, ToolbarDropdownKind::WorkingColorspace);
                    #[cfg(not(feature = "ocio"))]
                    spawn_dropdown(left, ToolbarDropdownKind::OutputColorspace);
                    #[cfg(feature = "ocio")]
                    spawn_dropdown(left, ToolbarDropdownKind::OcioDisplay);
                    #[cfg(feature = "ocio")]
                    spawn_dropdown(left, ToolbarDropdownKind::OcioView);
                });

            toolbar
                .spawn(Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|right| {
                    spawn_toggle_button(right, SplitViewToggleButton, "Split", 56.0);
                    spawn_toggle_button(right, OfxPanelToggleButton, "OFX", 40.0);
                });
        });
}

fn spawn_dropdown(parent: &mut ChildSpawnerCommands, kind: ToolbarDropdownKind) {
    parent
        .spawn(Node {
            position_type: PositionType::Relative,
            width: Val::Px(theme::TOOLBAR_DROPDOWN_WIDTH),
            ..default()
        })
        .with_children(|dropdown| {
            dropdown
                .spawn((
                    ToolbarDropdownButton(kind),
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
                        ToolbarDropdownLabel(kind),
                        Text::new(format!("{}: -", kind.title())),
                        TextFont {
                            font_size: theme::FONT_SIZE_LABEL,
                            ..default()
                        },
                        TextColor(theme::TEXT_PRIMARY),
                        Pickable::IGNORE,
                    ));
                    button.spawn((
                        Text::new("v"),
                        TextFont {
                            font_size: 10.0,
                            ..default()
                        },
                        TextColor(theme::TEXT_DIM),
                        Pickable::IGNORE,
                    ));
                });

            dropdown.spawn((
                ToolbarDropdownMenu(kind),
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(24.0),
                    left: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    display: Display::None,
                    flex_direction: FlexDirection::Column,
                    border: UiRect::all(Val::Px(1.0)),
                    max_height: Val::Px(300.0),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                BackgroundColor(theme::BG_CONTROL),
                BorderColor::all(theme::BORDER_SUBTLE),
                GlobalZIndex(600),
                ZIndex(60),
            ));
        });
}

fn spawn_toggle_button<T: Component>(
    parent: &mut ChildSpawnerCommands,
    marker: T,
    label: &str,
    width: f32,
) {
    parent
        .spawn((
            marker,
            Button,
            Node {
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                width: Val::Px(width),
                height: Val::Px(24.0),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(theme::BG_CONTROL),
            BorderColor::all(theme::BORDER_SUBTLE),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                TextFont {
                    font_size: theme::FONT_SIZE_LABEL,
                    ..default()
                },
                TextColor(theme::TEXT_PRIMARY),
                Pickable::IGNORE,
            ));
        });
}

/// Handle dropdown open/close and option-selection interactions.
#[allow(clippy::type_complexity)]
pub fn handle_toolbar_interactions(
    button_interactions: Query<
        (&Interaction, &ToolbarDropdownButton),
        (Changed<Interaction>, With<Button>),
    >,
    option_interactions: Query<
        (&Interaction, &ToolbarDropdownOption),
        (Changed<Interaction>, With<Button>),
    >,
    mut toolbar_state: ResMut<ToolbarState>,
    mut grading_state: ResMut<GradingState>,
    #[cfg(feature = "ocio")] mut ocio: Option<ResMut<OcioColorManagement>>,
) {
    for (interaction, button) in &button_interactions {
        if *interaction == Interaction::Pressed {
            if toolbar_state.active_dropdown == Some(button.0) {
                toolbar_state.active_dropdown = None;
            } else {
                toolbar_state.active_dropdown = Some(button.0);
            };
        }
    }

    for (interaction, option) in &option_interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        apply_dropdown_selection(
            option.kind,
            &option.value,
            &mut grading_state,
            #[cfg(feature = "ocio")]
            ocio.as_deref_mut(),
        );

        toolbar_state.active_dropdown = None;
    }
}

/// (Re)build dropdown menu options.
pub fn rebuild_toolbar_menus(
    mut commands: Commands,
    #[cfg(feature = "ocio")] ocio: Option<Res<OcioColorManagement>>,
    menus: Query<(Entity, &ToolbarDropdownMenu, Option<&Children>)>,
) {
    let mut needs_initial_build = false;
    for (_, _, children) in &menus {
        if children.is_none_or(|c| c.is_empty()) {
            needs_initial_build = true;
            break;
        }
    }

    #[cfg(not(feature = "ocio"))]
    if !needs_initial_build {
        return;
    }

    #[cfg(feature = "ocio")]
    if !needs_initial_build && !ocio.as_ref().is_some_and(|state| state.is_changed()) {
        return;
    }

    for (menu_entity, menu_kind, children) in &menus {
        if let Some(children) = children {
            for child in children.iter() {
                commands.entity(child).despawn();
            }
        }

        #[cfg(not(feature = "ocio"))]
        let values = dropdown_values(menu_kind.0);

        #[cfg(feature = "ocio")]
        let values = dropdown_values(menu_kind.0, ocio.as_deref());

        commands.entity(menu_entity).with_children(|menu| {
            for value in values {
                menu.spawn((
                    ToolbarDropdownOption {
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
                ))
                .with_children(|entry| {
                    entry.spawn((
                        Text::new(value),
                        TextFont {
                            font_size: theme::FONT_SIZE_LABEL,
                            ..default()
                        },
                        TextColor(theme::TEXT_PRIMARY),
                        Pickable::IGNORE,
                    ));
                });
            }
        });
    }
}

/// Keep labels, menu visibility, highlights, and toggle-button visuals in sync.
#[allow(clippy::type_complexity)]
pub fn sync_toolbar_ui(
    toolbar_state: Res<ToolbarState>,
    grading_state: Res<GradingState>,
    #[cfg(feature = "ocio")] ocio: Option<Res<OcioColorManagement>>,
    mut ui_parts: ParamSet<(
        Query<(&ToolbarDropdownLabel, &mut Text)>,
        Query<(&ToolbarDropdownMenu, &mut Node)>,
        Query<(&ToolbarDropdownOption, &mut BackgroundColor)>,
        Query<&mut BackgroundColor, With<SplitViewToggleButton>>,
        Query<&mut BackgroundColor, With<OfxPanelToggleButton>>,
    )>,
) {
    #[cfg(not(feature = "ocio"))]
    if !(toolbar_state.is_changed() || grading_state.is_changed()) {
        return;
    }

    #[cfg(feature = "ocio")]
    if !(toolbar_state.is_changed()
        || grading_state.is_changed()
        || ocio.as_ref().is_some_and(|state| state.is_changed()))
    {
        return;
    }

    for (label, mut text) in &mut ui_parts.p0() {
        #[cfg(not(feature = "ocio"))]
        let value = selected_value_for_kind(label.0, &grading_state);
        #[cfg(feature = "ocio")]
        let value = selected_value_for_kind(label.0, &grading_state, ocio.as_deref());

        *text = Text::new(format!("{}: {}", label.0.title(), value));
    }

    for (menu, mut node) in &mut ui_parts.p1() {
        node.display = if toolbar_state.active_dropdown == Some(menu.0) {
            Display::Flex
        } else {
            Display::None
        };
    }

    for (option, mut bg) in &mut ui_parts.p2() {
        #[cfg(not(feature = "ocio"))]
        let selected_value = selected_value_for_kind(option.kind, &grading_state);
        #[cfg(feature = "ocio")]
        let selected_value = selected_value_for_kind(option.kind, &grading_state, ocio.as_deref());

        *bg = if option.value == selected_value {
            BackgroundColor(theme::BG_TOGGLE_ACTIVE)
        } else {
            BackgroundColor(theme::BG_CONTROL)
        };
    }

    for mut bg in &mut ui_parts.p3() {
        *bg = if toolbar_state.split_view_active {
            BackgroundColor(theme::BG_TOGGLE_ACTIVE)
        } else {
            BackgroundColor(theme::BG_CONTROL)
        };
    }

    for mut bg in &mut ui_parts.p4() {
        *bg = if toolbar_state.ofx_panel_visible {
            BackgroundColor(theme::BG_TOGGLE_ACTIVE)
        } else {
            BackgroundColor(theme::BG_CONTROL)
        };
    }
}

/// Handle toolbar toggle-button clicks.
pub fn handle_toolbar_toggles(
    split_toggles: Query<&Interaction, (Changed<Interaction>, With<SplitViewToggleButton>)>,
    ofx_toggles: Query<&Interaction, (Changed<Interaction>, With<OfxPanelToggleButton>)>,
    mut toolbar_state: ResMut<ToolbarState>,
) {
    for interaction in &split_toggles {
        if *interaction == Interaction::Pressed {
            toolbar_state.split_view_active = !toolbar_state.split_view_active;
        }
    }

    for interaction in &ofx_toggles {
        if *interaction == Interaction::Pressed {
            toolbar_state.ofx_panel_visible = !toolbar_state.ofx_panel_visible;
        }
    }
}

/// Observer: handle clicks on toolbar dropdown option entities via the picking
/// system as a backup when `Changed<Interaction>` doesn't fire inside scrollers.
pub fn on_toolbar_option_click(
    ev: On<Pointer<Click>>,
    options: Query<&ToolbarDropdownOption>,
    parents: Query<&ChildOf>,
    mut toolbar_state: ResMut<ToolbarState>,
    mut grading_state: ResMut<GradingState>,
    #[cfg(feature = "ocio")] mut ocio: Option<ResMut<OcioColorManagement>>,
) {
    if ev.button != PointerButton::Primary {
        return;
    }
    let Some(option_entity) = find_option_ancestor(ev.entity, &options, &parents) else {
        return;
    };
    let Ok(option) = options.get(option_entity) else {
        return;
    };
    apply_dropdown_selection(
        option.kind,
        &option.value,
        &mut grading_state,
        #[cfg(feature = "ocio")]
        ocio.as_deref_mut(),
    );
    toolbar_state.active_dropdown = None;
}

/// Observer: close the active dropdown when the user clicks outside of it.
pub fn on_toolbar_click_close_dropdown(
    ev: On<Pointer<Click>>,
    mut toolbar_state: ResMut<ToolbarState>,
    buttons: Query<&ToolbarDropdownButton>,
    menus: Query<&ToolbarDropdownMenu>,
    options: Query<&ToolbarDropdownOption>,
    parents: Query<&ChildOf>,
) {
    if ev.button != PointerButton::Primary {
        return;
    }
    let Some(active_kind) = toolbar_state.active_dropdown else {
        return;
    };
    let clicked_kind = find_dropdown_kind_ancestor(ev.entity, &buttons, &menus, &options, &parents);
    if clicked_kind != Some(active_kind) {
        toolbar_state.active_dropdown = None;
    }
}

fn find_option_ancestor(
    mut entity: Entity,
    options: &Query<&ToolbarDropdownOption>,
    parents: &Query<&ChildOf>,
) -> Option<Entity> {
    loop {
        if options.get(entity).is_ok() {
            return Some(entity);
        }
        let Ok(parent) = parents.get(entity) else {
            return None;
        };
        entity = parent.0;
    }
}

fn find_dropdown_kind_ancestor(
    mut entity: Entity,
    buttons: &Query<&ToolbarDropdownButton>,
    menus: &Query<&ToolbarDropdownMenu>,
    options: &Query<&ToolbarDropdownOption>,
    parents: &Query<&ChildOf>,
) -> Option<ToolbarDropdownKind> {
    loop {
        if let Ok(button) = buttons.get(entity) {
            return Some(button.0);
        }
        if let Ok(menu) = menus.get(entity) {
            return Some(menu.0);
        }
        if let Ok(option) = options.get(entity) {
            return Some(option.kind);
        }
        let Ok(parent) = parents.get(entity) else {
            return None;
        };
        entity = parent.0;
    }
}

/// Keyboard shortcuts for split view and OFX panel toggles.
pub fn handle_toolbar_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    mut toolbar_state: ResMut<ToolbarState>,
) {
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    if !ctrl {
        return;
    }

    if keys.just_pressed(KeyCode::Backslash) || keys.just_pressed(KeyCode::IntlBackslash) {
        toolbar_state.split_view_active = !toolbar_state.split_view_active;
    }

    if keys.just_pressed(KeyCode::KeyP) {
        toolbar_state.ofx_panel_visible = !toolbar_state.ofx_panel_visible;
    }
}

fn parse_color_space_option(value: &str) -> Option<ColorSpaceId> {
    ColorSpaceId::all()
        .iter()
        .copied()
        .find(|space| space.label() == value)
}

fn apply_dropdown_selection(
    kind: ToolbarDropdownKind,
    value: &str,
    grading_state: &mut GradingState,
    #[cfg(feature = "ocio")] ocio: Option<&mut OcioColorManagement>,
) {
    #[cfg(not(feature = "ocio"))]
    if let Some(selected) = parse_color_space_option(value) {
        match kind {
            ToolbarDropdownKind::InputColorspace => {
                if grading_state.params.color_management.input_space != selected {
                    grading_state.params.color_management.input_space = selected;
                    grading_state.dirty = true;
                }
            }
            ToolbarDropdownKind::WorkingColorspace => {
                if grading_state.params.color_management.working_space != selected {
                    grading_state.params.color_management.working_space = selected;
                    grading_state.dirty = true;
                }
            }
            ToolbarDropdownKind::OutputColorspace => {
                if grading_state.params.color_management.output_space != selected {
                    grading_state.params.color_management.output_space = selected;
                    grading_state.dirty = true;
                }
            }
        }
    }

    #[cfg(feature = "ocio")]
    if let Some(ocio_state) = ocio {
        match kind {
            ToolbarDropdownKind::InputColorspace => {
                if ocio_state.input_space != value {
                    ocio_state.input_space = value.to_string();
                    ocio_state.dirty = true;
                }
            }
            ToolbarDropdownKind::WorkingColorspace => {
                if ocio_state.working_space != value {
                    ocio_state.working_space = value.to_string();
                    ocio_state.dirty = true;
                }
            }
            ToolbarDropdownKind::OcioDisplay => {
                if ocio_state.display != value {
                    ocio_state.display = value.to_string();
                    let default_view = ocio_state.config.default_view(&ocio_state.display);
                    ocio_state.view = if default_view.is_empty() {
                        ocio_state
                            .config
                            .views(&ocio_state.display)
                            .into_iter()
                            .next()
                            .unwrap_or_default()
                    } else {
                        default_view
                    };
                    ocio_state.dirty = true;
                }
            }
            ToolbarDropdownKind::OcioView => {
                if ocio_state.view != value {
                    ocio_state.view = value.to_string();
                    ocio_state.dirty = true;
                }
            }
        }
    } else if let Some(selected) = parse_color_space_option(value) {
        match kind {
            ToolbarDropdownKind::InputColorspace => {
                if grading_state.params.color_management.input_space != selected {
                    grading_state.params.color_management.input_space = selected;
                    grading_state.dirty = true;
                }
            }
            ToolbarDropdownKind::WorkingColorspace => {
                if grading_state.params.color_management.working_space != selected {
                    grading_state.params.color_management.working_space = selected;
                    grading_state.dirty = true;
                }
            }
            ToolbarDropdownKind::OcioDisplay | ToolbarDropdownKind::OcioView => {}
        }
    }
}

#[cfg(not(feature = "ocio"))]
fn dropdown_values(kind: ToolbarDropdownKind) -> Vec<String> {
    match kind {
        ToolbarDropdownKind::InputColorspace
        | ToolbarDropdownKind::WorkingColorspace
        | ToolbarDropdownKind::OutputColorspace => ColorSpaceId::all()
            .iter()
            .map(|space| space.label().to_string())
            .collect(),
    }
}

#[cfg(feature = "ocio")]
fn dropdown_values(kind: ToolbarDropdownKind, ocio: Option<&OcioColorManagement>) -> Vec<String> {
    if let Some(ocio) = ocio {
        match kind {
            ToolbarDropdownKind::InputColorspace | ToolbarDropdownKind::WorkingColorspace => {
                ocio.config.color_space_names()
            }
            ToolbarDropdownKind::OcioDisplay => ocio.config.displays(),
            ToolbarDropdownKind::OcioView => ocio.config.views(&ocio.display),
        }
    } else {
        match kind {
            ToolbarDropdownKind::InputColorspace | ToolbarDropdownKind::WorkingColorspace => {
                ColorSpaceId::all()
                    .iter()
                    .map(|space| space.label().to_string())
                    .collect()
            }
            ToolbarDropdownKind::OcioDisplay | ToolbarDropdownKind::OcioView => Vec::new(),
        }
    }
}

#[cfg(not(feature = "ocio"))]
fn selected_value_for_kind(kind: ToolbarDropdownKind, grading: &GradingState) -> String {
    match kind {
        ToolbarDropdownKind::InputColorspace => grading
            .params
            .color_management
            .input_space
            .label()
            .to_string(),
        ToolbarDropdownKind::WorkingColorspace => grading
            .params
            .color_management
            .working_space
            .label()
            .to_string(),
        ToolbarDropdownKind::OutputColorspace => grading
            .params
            .color_management
            .output_space
            .label()
            .to_string(),
    }
}

#[cfg(feature = "ocio")]
fn selected_value_for_kind(
    kind: ToolbarDropdownKind,
    grading: &GradingState,
    ocio: Option<&OcioColorManagement>,
) -> String {
    if let Some(ocio) = ocio {
        match kind {
            ToolbarDropdownKind::InputColorspace => ocio.input_space.clone(),
            ToolbarDropdownKind::WorkingColorspace => ocio.working_space.clone(),
            ToolbarDropdownKind::OcioDisplay => ocio.display.clone(),
            ToolbarDropdownKind::OcioView => ocio.view.clone(),
        }
    } else {
        match kind {
            ToolbarDropdownKind::InputColorspace => grading
                .params
                .color_management
                .input_space
                .label()
                .to_string(),
            ToolbarDropdownKind::WorkingColorspace => grading
                .params
                .color_management
                .working_space
                .label()
                .to_string(),
            ToolbarDropdownKind::OcioDisplay | ToolbarDropdownKind::OcioView => "-".to_string(),
        }
    }
}
