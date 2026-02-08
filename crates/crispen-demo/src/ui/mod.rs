//! Native Bevy UI for the Crispen color grading demo.
//!
//! Replaces the wry/Svelte webview with Bevy's built-in UI widgets,
//! providing a DaVinci Resolve-style dark interface.

pub mod color_wheel;
pub mod components;
pub mod layout;
pub mod primaries;
pub mod systems;
pub mod theme;
pub mod viewer;

use bevy::prelude::*;

/// Top-level UI plugin. Registers layout, widget, and interaction systems.
pub struct CrispenUiPlugin;

impl Plugin for CrispenUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(color_wheel::ColorWheelPlugin)
            .add_systems(
                Startup,
                (viewer::setup_viewer, layout::spawn_root_layout).chain(),
            )
            .add_systems(
                Update,
                (
                    systems::sync_sliders_to_params,
                    systems::sync_params_to_sliders,
                    systems::sync_params_to_wheels,
                    components::update_param_slider_visuals,
                    viewer::update_viewer_texture,
                ),
            )
            .add_observer(systems::on_wheel_value_change);
    }
}
