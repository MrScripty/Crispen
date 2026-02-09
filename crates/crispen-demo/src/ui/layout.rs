//! Root layout: top toolbar, main viewer row, primaries panel at bottom.

use bevy::prelude::*;
use bevy::ui::UiTargetCamera;

use super::UiCameraEntity;
use super::ofx_panel::OfxPluginRegistry;
use super::split_viewer::SourceImageHandle;
use super::vectorscope::VectorscopeImageHandle;
use super::viewer::ViewerImageHandle;
use super::{ofx_panel, primaries, split_viewer, theme, toolbar};

/// Spawn the root layout with toolbar, main content row, and primaries panel.
pub fn spawn_root_layout(
    mut commands: Commands,
    viewer_handle: Res<ViewerImageHandle>,
    source_handle: Res<SourceImageHandle>,
    vectorscope_handle: Res<VectorscopeImageHandle>,
    ofx_registry: Res<OfxPluginRegistry>,
    ui_camera: Res<UiCameraEntity>,
) {
    commands
        .spawn((
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            UiTargetCamera(ui_camera.0),
            BackgroundColor(theme::BG_DARK),
        ))
        .with_children(|root| {
            toolbar::spawn_toolbar(root);

            root.spawn(Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                ..default()
            })
            .with_children(|main_row| {
                split_viewer::spawn_viewer_area(
                    main_row,
                    viewer_handle.handle.clone(),
                    source_handle.handle.clone(),
                );
                ofx_panel::spawn_ofx_panel(main_row, &ofx_registry);
            });

            primaries::spawn_primaries_panel(root, vectorscope_handle.handle.clone());
        });
}
