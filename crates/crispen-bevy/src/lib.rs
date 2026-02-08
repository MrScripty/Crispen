//! Crispen Bevy Plugin — integrates the color grading pipeline into Bevy's ECS.
//!
//! Provides `CrispenPlugin` which registers all resources, events, and systems
//! needed to run the grading pipeline within a Bevy application.

pub mod events;
pub mod render_node;
pub mod resources;
pub mod scope_render;
pub mod systems;

use bevy::prelude::*;

use resources::{GradingState, ImageState, ScopeConfig, ScopeState};
use systems::{detect_param_changes, handle_grading_commands, rebake_lut_if_dirty, update_scopes};

/// Main Bevy plugin for the Crispen color grading pipeline.
///
/// Registers resources, events, and systems for:
/// - Managing `GradingParams` as a Bevy resource
/// - Triggering LUT re-bake on parameter changes
/// - Running scope computation when a graded image is available
/// - Integrating with Bevy's render graph (Phase 2)
pub struct CrispenPlugin;

impl Plugin for CrispenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GradingState>()
            .init_resource::<ImageState>()
            .init_resource::<ScopeState>()
            .init_resource::<ScopeConfig>()
            .add_systems(
                Update,
                (
                    handle_grading_commands,
                    rebake_lut_if_dirty.after(handle_grading_commands),
                    update_scopes.after(rebake_lut_if_dirty),
                    detect_param_changes,
                ),
            );
    }
}
