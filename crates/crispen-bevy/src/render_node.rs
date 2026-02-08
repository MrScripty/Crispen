//! Bevy render graph node for the grading pipeline.

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
