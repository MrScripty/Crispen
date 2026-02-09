//! OpenFX host implementation for loading and running OFX plugins.

/// Manages OpenFX plugin discovery, loading, and execution.
pub struct OfxHost {
    _private: (),
}

impl OfxHost {
    /// Create a new OpenFX host and scan for available plugins.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for OfxHost {
    fn default() -> Self {
        Self::new()
    }
}
