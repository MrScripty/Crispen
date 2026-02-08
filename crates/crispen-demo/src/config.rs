//! Application configuration for the demo.

use bevy::prelude::*;

/// Default WebSocket port for the IPC bridge.
const DEFAULT_WS_PORT: u16 = 9400;
/// Default window width.
const DEFAULT_WIDTH: f32 = 1920.0;
/// Default window height.
const DEFAULT_HEIGHT: f32 = 1080.0;

/// Runtime configuration for the Crispen demo application.
#[derive(Resource, Clone)]
pub struct AppConfig {
    /// WebSocket port for Bevy <-> Svelte IPC.
    pub ws_port: u16,
    /// Window width in logical pixels.
    pub width: f32,
    /// Window height in logical pixels.
    pub height: f32,
    /// Whether to use the Vite dev server for the UI.
    pub dev_mode: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ws_port: std::env::var("CRISPEN_WS_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(DEFAULT_WS_PORT),
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            dev_mode: std::env::var("CRISPEN_DEV").is_ok(),
        }
    }
}
