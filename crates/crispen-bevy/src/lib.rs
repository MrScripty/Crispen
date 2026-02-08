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

/// Main Bevy plugin for the Crispen color grading pipeline.
///
/// Registers resources, events, and systems for:
/// - Managing `GradingParams` as a Bevy resource
/// - Triggering LUT re-bake on parameter changes
/// - Running scope computation each frame
/// - Integrating with Bevy's render graph
pub struct CrispenPlugin;

impl Plugin for CrispenPlugin {
    fn build(&self, app: &mut App) {
        let _ = app;
        todo!()
    }
}
