//! Bevy render graph node for the grading pipeline.
//!
//! Phase 2: will dispatch GPU pipeline via crispen-gpu, replacing the
//! CPU path in `systems::rebake_lut_if_dirty`.

/// Render graph node that executes the GPU grading pipeline.
pub struct GradingRenderNode {
    _private: (),
}

impl GradingRenderNode {
    /// Create a new grading render node.
    pub fn new() -> Self {
        Self { _private: () }
    }
}
