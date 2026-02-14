//! Native Bevy UI for the Crispen color grading demo.
//!
//! Replaces the wry/Svelte webview with Bevy's built-in UI widgets,
//! providing a DaVinci Resolve-style dark interface.

pub mod color_wheel;
pub mod components;
pub mod dial;
pub mod hue_curves;
pub mod layout;
pub mod master_slider;
pub mod ofx_panel;
pub mod primaries;
pub mod scope_mask;
pub mod split_viewer;
pub mod systems;
pub mod theme;
pub mod toolbar;
pub mod vectorscope;
pub mod viewer;
pub mod viewer_nav;

use bevy::prelude::*;
use bevy::ui::IsDefaultUiCamera;

/// Top-level UI plugin. Registers layout, widget, and interaction systems.
pub struct CrispenUiPlugin;

impl Plugin for CrispenUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            color_wheel::ColorWheelPlugin,
            dial::DialPlugin,
            master_slider::MasterSliderPlugin,
            hue_curves::HueCurvesPlugin,
            scope_mask::ScopeMaskPlugin,
        ))
        .init_resource::<toolbar::ToolbarState>()
        .init_resource::<vectorscope::ScopeViewState>()
        .init_resource::<viewer_nav::ViewerTransform>()
        .add_systems(
            Startup,
            (
                setup_ui_camera,
                viewer::setup_viewer,
                split_viewer::setup_source_image,
                ofx_panel::setup_ofx_registry,
                vectorscope::setup_vectorscope,
                layout::spawn_root_layout,
                log_ui_spawn_counts,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                (
                    systems::sync_dials_to_params,
                    systems::sync_master_sliders_to_params,
                    systems::sync_params_to_dials,
                    systems::sync_params_to_wheels,
                    systems::sync_params_to_master_sliders,
                )
                    .chain()
                    .before(crispen_bevy::systems::submit_gpu_work),
                dial::update_dial_visuals,
                viewer::update_viewer_texture
                    .after(crispen_bevy::systems::consume_gpu_results),
                split_viewer::update_source_texture,
                split_viewer::toggle_split_view,
                toolbar::handle_toolbar_interactions,
                toolbar::handle_toolbar_toggles,
                toolbar::handle_toolbar_shortcuts,
                toolbar::rebuild_toolbar_menus,
                toolbar::sync_toolbar_ui,
                ofx_panel::toggle_ofx_panel,
                vectorscope::handle_scope_dropdown_interactions,
                vectorscope::sync_scope_dropdown_ui,
                vectorscope::update_scope_texture
                    .after(crispen_bevy::systems::consume_gpu_results),
                systems::handle_load_image_shortcut,
                viewer_nav::handle_viewer_scroll,
                viewer_nav::reset_viewer_transform,
                viewer_nav::apply_viewer_transform,
            ),
        )
        .add_observer(systems::on_wheel_value_change)
        .add_observer(toolbar::on_toolbar_option_click)
        .add_observer(toolbar::on_toolbar_click_close_dropdown)
        .add_observer(vectorscope::on_scope_option_click)
        .add_observer(viewer_nav::on_viewer_drag_start)
        .add_observer(viewer_nav::on_viewer_drag)
        .add_observer(viewer_nav::on_viewer_drag_end)
        .add_observer(viewer_nav::on_viewer_drag_cancel)
        .add_observer(viewer_nav::on_viewer_click);
    }
}

pub fn setup_ui_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.102, 0.102, 0.102)),
            ..default()
        },
        IsDefaultUiCamera,
    ));
}

fn log_ui_spawn_counts(
    toolbar_roots: Query<Entity, With<toolbar::ToolbarRoot>>,
    viewer_containers: Query<Entity, With<split_viewer::ViewerContainer>>,
    source_nodes: Query<Entity, With<split_viewer::SourceImageNode>>,
    graded_nodes: Query<Entity, With<split_viewer::GradedImageNode>>,
    scope_frames: Query<Entity, With<vectorscope::ScopeImageFrame>>,
) {
    tracing::info!(
        "UI spawn counts: toolbar={}, viewer_container={}, source_nodes={}, graded_nodes={}, scope_frames={}",
        toolbar_roots.iter().count(),
        viewer_containers.iter().count(),
        source_nodes.iter().count(),
        graded_nodes.iter().count(),
        scope_frames.iter().count()
    );
}
