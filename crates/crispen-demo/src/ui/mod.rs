//! Native Bevy UI for the Crispen color grading demo.
//!
//! Replaces the wry/Svelte webview with Bevy's built-in UI widgets,
//! providing a DaVinci Resolve-style dark interface.

pub mod color_wheel;
pub mod components;
pub mod dial;
pub mod hue_curves;
pub mod layout;
pub mod primaries;
pub mod systems;
pub mod theme;
pub mod vectorscope;
pub mod viewer;

use bevy::prelude::*;
use bevy::ui::IsDefaultUiCamera;

/// Entity id of the camera used to render UI.
#[derive(Resource, Clone, Copy)]
pub struct UiCameraEntity(pub Entity);

/// Top-level UI plugin. Registers layout, widget, and interaction systems.
pub struct CrispenUiPlugin;

impl Plugin for CrispenUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            color_wheel::ColorWheelPlugin,
            dial::DialPlugin,
            hue_curves::HueCurvesPlugin,
        ))
        .init_resource::<vectorscope::ScopeViewState>()
        .add_systems(
            Startup,
            (
                setup_ui_camera,
                viewer::setup_viewer,
                vectorscope::setup_vectorscope,
                layout::spawn_root_layout,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                (
                    systems::sync_dials_to_params,
                    systems::sync_params_to_dials,
                    systems::sync_params_to_wheels,
                )
                    .chain(),
                dial::update_dial_visuals,
                viewer::update_viewer_texture,
                vectorscope::handle_scope_dropdown_interactions,
                vectorscope::sync_scope_dropdown_ui,
                vectorscope::update_scope_texture,
                systems::handle_load_image_shortcut,
            ),
        )
        .add_observer(systems::on_wheel_value_change);
    }
}

fn setup_ui_camera(mut commands: Commands) {
    let camera = commands.spawn((Camera2d, IsDefaultUiCamera)).id();
    commands.insert_resource(UiCameraEntity(camera));
}
