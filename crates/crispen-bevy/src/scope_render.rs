//! Scope rendering integration with Bevy's render pipeline.
//!
//! Phase 2: will render scope data into Bevy `Image` assets for UI display.

/// Manages scope texture resources for UI display.
pub struct ScopeRenderer {
    _private: (),
}

impl ScopeRenderer {
    /// Create a new scope renderer.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for ScopeRenderer {
    fn default() -> Self {
        Self::new()
    }
}
