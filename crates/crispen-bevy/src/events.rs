//! Bevy messages for cross-system communication in the grading pipeline.

use bevy::prelude::*;
use crispen_core::transform::params::GradingParams;

// === Inbound Commands (UI -> ECS) ===

/// Commands received from the UI that the grading systems process.
/// Each variant maps 1:1 to a `UiToBevy` IPC message.
#[derive(Message)]
pub enum ColorGradingCommand {
    /// Apply new grading parameters.
    SetParams { params: GradingParams },
    /// Run automatic white balance on the current image.
    AutoBalance,
    /// Reset all grading to identity defaults.
    ResetGrade,
    /// Load a source image from disk.
    LoadImage { path: String },
    /// Load a 3D LUT file into a named slot.
    LoadLut { path: String, slot: String },
    /// Export the current grading as a .cube LUT file.
    ExportLut { path: String, size: u32 },
    /// Toggle visibility of a scope type.
    ToggleScope { scope_type: String, visible: bool },
}

// === Outbound Notifications (ECS -> UI) ===

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
