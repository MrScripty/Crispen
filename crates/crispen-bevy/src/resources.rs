//! Bevy resources for the color grading pipeline.

use bevy::prelude::*;
use crispen_core::transform::params::GradingParams;

/// Bevy resource holding the current grading parameters.
///
/// This is the single source of truth for grading state within the ECS.
/// Systems watch for changes via `Res<GradingState>` and trigger LUT re-bake.
#[derive(Resource)]
pub struct GradingState {
    /// The current grading parameters.
    pub params: GradingParams,
    /// Whether the params have changed since last LUT bake.
    pub dirty: bool,
}

impl Default for GradingState {
    fn default() -> Self {
        Self {
            params: GradingParams::default(),
            dirty: true,
        }
    }
}
