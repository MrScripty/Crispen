//! Collapsible right-side OFX plugin discovery panel.

use bevy::prelude::*;
use crispen_ofx::host::{OfxHost, OfxLoadFailure, OfxPluginDescriptor};

use super::theme;
use super::toolbar::ToolbarState;

/// Startup snapshot of discovered OFX plugins and non-fatal load failures.
#[derive(Resource, Default)]
pub struct OfxPluginRegistry {
    pub plugins: Vec<OfxPluginDescriptor>,
    pub failures: Vec<OfxLoadFailure>,
}

/// Marker for the OFX side panel root.
#[derive(Component)]
pub struct OfxPanelRoot;

/// Build the OFX discovery registry once at startup.
pub fn setup_ofx_registry(mut commands: Commands) {
    let host = OfxHost::new();
    commands.insert_resource(OfxPluginRegistry {
        plugins: host.plugins().to_vec(),
        failures: host.failures().to_vec(),
    });
}

/// Spawn a hidden OFX side panel on the right side of the viewer row.
pub fn spawn_ofx_panel(parent: &mut ChildSpawnerCommands, registry: &OfxPluginRegistry) {
    parent
        .spawn((
            OfxPanelRoot,
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                width: Val::Px(theme::OFX_PANEL_WIDTH),
                flex_shrink: 0.0,
                row_gap: Val::Px(8.0),
                padding: UiRect::all(Val::Px(8.0)),
                border: UiRect::left(Val::Px(1.0)),
                overflow: Overflow::clip_y(),
                ..default()
            },
            BackgroundColor(theme::BG_PANEL),
            BorderColor::all(theme::BORDER_SUBTLE),
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(format!("OFX Plugins ({})", registry.plugins.len())),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(theme::TEXT_PRIMARY),
            ));

            for plugin in &registry.plugins {
                panel
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(2.0),
                            padding: UiRect::all(Val::Px(6.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(theme::BG_CONTROL),
                        BorderColor::all(theme::BORDER_SUBTLE),
                    ))
                    .with_children(|card| {
                        card.spawn((
                            Text::new(plugin.plugin_identifier.clone()),
                            TextFont {
                                font_size: theme::FONT_SIZE_LABEL,
                                ..default()
                            },
                            TextColor(theme::TEXT_PRIMARY),
                        ));
                        card.spawn((
                            Text::new(format!(
                                "v{}.{}",
                                plugin.plugin_version_major, plugin.plugin_version_minor
                            )),
                            TextFont {
                                font_size: 10.0,
                                ..default()
                            },
                            TextColor(theme::TEXT_DIM),
                        ));
                        card.spawn((
                            Text::new(plugin.binary_path.display().to_string()),
                            TextFont {
                                font_size: 10.0,
                                ..default()
                            },
                            TextColor(theme::TEXT_DIM),
                        ));
                    });
            }

            if !registry.failures.is_empty() {
                panel.spawn((
                    Text::new("Load Failures"),
                    Node {
                        margin: UiRect::top(Val::Px(4.0)),
                        ..default()
                    },
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(theme::ACCENT),
                ));

                for failure in &registry.failures {
                    panel
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(2.0),
                                padding: UiRect::all(Val::Px(6.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(theme::BG_CONTROL),
                            BorderColor::all(theme::BORDER_SUBTLE),
                        ))
                        .with_children(|entry| {
                            entry.spawn((
                                Text::new(failure.binary_path.display().to_string()),
                                TextFont {
                                    font_size: 10.0,
                                    ..default()
                                },
                                TextColor(theme::TEXT_PRIMARY),
                            ));
                            entry.spawn((
                                Text::new(failure.message.clone()),
                                TextFont {
                                    font_size: 10.0,
                                    ..default()
                                },
                                TextColor(theme::TEXT_DIM),
                            ));
                        });
                }
            }
        });
}

/// Show or hide the OFX side panel based on toolbar toggle state.
pub fn toggle_ofx_panel(
    toolbar_state: Res<ToolbarState>,
    mut panels: Query<&mut Node, With<OfxPanelRoot>>,
) {
    if !toolbar_state.is_changed() {
        return;
    }

    for mut node in &mut panels {
        node.display = if toolbar_state.ofx_panel_visible {
            Display::Flex
        } else {
            Display::None
        };
    }
}
