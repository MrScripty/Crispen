//! Root layout: viewer on top, primaries panel at bottom.
//!
//! Uses a vertical flex layout where the viewer expands to available
//! space and the primaries panel keeps a fixed control-surface height.

use bevy::prelude::*;
use bevy::ui::UiTargetCamera;

use super::UiCameraEntity;
use super::vectorscope::VectorscopeImageHandle;
use super::viewer::ViewerImageHandle;
use super::{primaries, theme, viewer};

/// Spawn the root layout with viewer (top) and primaries panel (bottom).
pub fn spawn_root_layout(
    mut commands: Commands,
    viewer_handle: Res<ViewerImageHandle>,
    vectorscope_handle: Res<VectorscopeImageHandle>,
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
            viewer::spawn_viewer_panel(root, viewer_handle.handle.clone());
            primaries::spawn_primaries_panel(root, vectorscope_handle.handle.clone());
        });
}
