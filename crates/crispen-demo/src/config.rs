//! Application configuration for the demo.

/// Runtime configuration for the Crispen demo application.
pub struct AppConfig {
    _private: (),
}

impl AppConfig {
    /// Load configuration with defaults.
    pub fn load() -> Self {
        Self { _private: () }
    }
}
