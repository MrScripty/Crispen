//! Webview compositing and UI overlay for the demo application.
//!
//! Sets up a transparent overlay texture for the wry-embedded Svelte UI.
//! Phase 2 will add actual wry WebView capture and compositing.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

use crate::config::AppConfig;

/// Resource holding the UI overlay texture handle.
#[derive(Resource)]
pub struct UiTextureHandle {
    pub handle: Handle<Image>,
}

/// Marker component for the UI overlay sprite.
#[derive(Component)]
pub struct UiOverlay;

/// Bevy plugin that registers webview compositing systems.
pub struct WebviewPlugin;

impl Plugin for WebviewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_webview);
    }
}

/// Startup system: create the UI overlay texture and fullscreen node.
fn setup_webview(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    config: Res<AppConfig>,
) {
    let width = config.width as u32;
    let height = config.height as u32;

    tracing::info!(
        "WebView setup: {}x{}, ws_port={}, dev={}",
        width,
        height,
        config.ws_port,
        config.dev_mode
    );

    // Create transparent overlay texture
    let mut image = Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;

    let handle = images.add(image);
    commands.insert_resource(UiTextureHandle {
        handle: handle.clone(),
    });

    // Spawn fullscreen overlay node
    commands.spawn((
        ImageNode {
            image: handle,
            ..default()
        },
        Node {
            width: Val::Vw(100.0),
            height: Val::Vh(100.0),
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            ..default()
        },
        ZIndex(i32::MAX),
        UiOverlay,
    ));

    // TODO Phase 2: Initialize wry::WebView here for offscreen capture.
    // For Phase 1 development, the Svelte UI runs in a browser tab at
    // http://localhost:5174 and connects via WebSocket.
}
