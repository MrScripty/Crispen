//! Native Bevy UI for the Crispen color grading demo.
//!
//! Replaces the wry/Svelte webview with Bevy's built-in UI widgets,
//! providing a DaVinci Resolve-style dark interface.

pub mod color_wheel;
pub mod components;
pub mod layout;
pub mod primaries;
pub mod systems;
pub mod theme;
pub mod viewer;

use bevy::prelude::*;

/// Top-level UI plugin. Registers layout, widget, and interaction systems.
pub struct CrispenUiPlugin;

impl Plugin for CrispenUiPlugin {
    fn build(&self, _app: &mut App) {
        // Phase 2: register UI systems, spawn root layout, wire widget events.
    }
}
