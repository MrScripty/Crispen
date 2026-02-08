//! Bevy messages for cross-system communication in the grading pipeline.

use bevy::prelude::*;
use crispen_core::transform::params::GradingParams;

/// Fired when grading parameters are updated (e.g., from UI input).
#[derive(Message)]
pub struct ParamsUpdatedEvent {
    /// The new grading parameters.
    pub params: GradingParams,
}

/// Fired when a new image is loaded into the grading pipeline.
#[derive(Message)]
pub struct ImageLoadedEvent {
    /// Width of the loaded image.
    pub width: u32,
    /// Height of the loaded image.
    pub height: u32,
    /// Bit depth description of the source.
    pub bit_depth: String,
}

/// Fired when scope data has been computed and is ready for display.
#[derive(Message)]
pub struct ScopeDataReadyEvent;
