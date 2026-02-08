//! Root layout: viewer center, panels left/right/bottom.
//!
//! Uses `Display::Grid` with two rows:
//! - Row 0 (`fr(2.0)`): image viewer (≈2/3 of height)
//! - Row 1 (`auto`): primaries panel (sized to content)

use bevy::prelude::*;

use super::{primaries, theme, viewer};
use super::viewer::ViewerImageHandle;

/// Spawn the root grid layout with viewer (top) and primaries panel (bottom).
pub fn spawn_root_layout(
    mut commands: Commands,
    viewer_handle: Res<ViewerImageHandle>,
) {
    commands
        .spawn((
            Node {
                display: Display::Grid,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                grid_template_rows: vec![GridTrack::fr(2.0), GridTrack::auto()],
                grid_template_columns: vec![GridTrack::fr(1.0)],
                ..default()
            },
            BackgroundColor(theme::BG_DARK),
        ))
        .with_children(|root| {
            // Row 0: Image viewer
            viewer::spawn_viewer_node(root, viewer_handle.handle.clone());
            // Row 1: Primaries panel
            primaries::spawn_primaries_panel(root);
        });
}
