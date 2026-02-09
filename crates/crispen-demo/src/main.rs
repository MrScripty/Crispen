//! Crispen Demo — standalone color grading application.
//!
//! Uses Bevy's native UI widgets for a DaVinci Resolve-style interface
//! with the Crispen grading pipeline and WebSocket IPC bridge.

mod config;
mod image_loader;
mod ipc;
mod ocio_support;
mod ui;
mod ws_bridge;

use bevy::input_focus::InputDispatchPlugin;
use bevy::prelude::*;
use bevy::window::WindowResolution;

use config::AppConfig;
use crispen_bevy::CrispenPlugin;
use crispen_bevy::events::{ParamsUpdatedEvent, ScopeDataReadyEvent};
#[cfg(feature = "ocio")]
use crispen_bevy::resources::OcioColorManagement;
use crispen_bevy::resources::{GradingState, ScopeState};
#[cfg(feature = "ocio")]
use crispen_ocio::OcioConfig;
use ws_bridge::{OutboundUiMessages, WsBridge};

fn main() {
    let config = AppConfig::default();
    tracing::info!(
        "starting demo: {}x{}, ws_port={}, dev_mode={}",
        config.width,
        config.height,
        config.ws_port,
        config.dev_mode
    );

    // Spawn WebSocket IPC server
    let (outbound_tx, inbound_rx) = ws_bridge::spawn_ws_server(config.ws_port);

    let window = Window {
        title: "Crispen".into(),
        resolution: WindowResolution::new(config.width as u32, config.height as u32)
            .with_scale_factor_override(1.0),
        present_mode: bevy::window::PresentMode::AutoVsync,
        ..default()
    };

    let mut app = App::new();
    app.insert_resource(config)
        .insert_resource(OutboundUiMessages::default())
        .insert_resource(WsBridge {
            outbound_tx,
            inbound_rx,
        })
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(window),
                    ..default()
                })
                .set(bevy::log::LogPlugin {
                    level: bevy::log::Level::INFO,
                    ..default()
                }),
        )
        .add_plugins(CrispenPlugin)
        .add_plugins(InputDispatchPlugin)
        .add_plugins(bevy::ui_widgets::UiWidgetsPlugins)
        .add_plugins(ui::CrispenUiPlugin)
        .add_systems(Startup, send_initial_state)
        .add_systems(
            Update,
            (
                ws_bridge::poll_inbound_messages,
                ws_bridge::flush_outbound_messages,
                forward_params_to_ui,
                forward_scopes_to_ui,
            ),
        );

    #[cfg(feature = "ocio")]
    try_insert_ocio_resource(&mut app);

    app.run();
}

#[cfg(feature = "ocio")]
fn try_insert_ocio_resource(app: &mut App) {
    let ocio_config = OcioConfig::from_env()
        .or_else(|_| OcioConfig::builtin("studio-config-v4.0.0_aces-v2.0_ocio-v2.5"))
        .or_else(|_| OcioConfig::builtin("studio-config-v2.2.0_aces-v1.3_ocio-v2.4"));

    let Ok(config) = ocio_config else {
        tracing::warn!("OCIO config unavailable; using native color management");
        return;
    };

    let default_display = config.default_display();
    let display = if default_display.is_empty() {
        config
            .displays()
            .into_iter()
            .next()
            .unwrap_or_else(|| "sRGB - Display".to_string())
    } else {
        default_display
    };

    let default_view = config.default_view(&display);
    let view = if default_view.is_empty() {
        config
            .views(&display)
            .into_iter()
            .next()
            .unwrap_or_else(|| "ACES 1.0 - SDR Video".to_string())
    } else {
        default_view
    };

    let working_space = config
        .role("scene_linear")
        .unwrap_or_else(|| "ACEScg".to_string());

    app.insert_resource(OcioColorManagement {
        config,
        input_space: "sRGB - Texture".to_string(),
        working_space,
        display,
        view,
        idt_lut: None,
        odt_lut: None,
        dirty: true,
    });
    tracing::info!("OCIO enabled");
}

/// Send initial state to the UI when the app starts.
fn send_initial_state(state: Res<GradingState>, mut outbound: ResMut<OutboundUiMessages>) {
    outbound.send(ipc::BevyToUi::Initialize {
        params: state.params.clone(),
    });
}

/// Forward `ParamsUpdatedEvent` to the UI via WebSocket.
fn forward_params_to_ui(
    mut events: MessageReader<ParamsUpdatedEvent>,
    mut outbound: ResMut<OutboundUiMessages>,
) {
    for event in events.read() {
        outbound.send(ipc::BevyToUi::ParamsUpdated {
            params: event.params.clone(),
        });
    }
}

/// Forward scope data to the UI when ready.
fn forward_scopes_to_ui(
    mut events: MessageReader<ScopeDataReadyEvent>,
    scope_state: Res<ScopeState>,
    mut outbound: ResMut<OutboundUiMessages>,
) {
    for _ in events.read() {
        if let (Some(h), Some(w), Some(v), Some(c)) = (
            scope_state.histogram.clone(),
            scope_state.waveform.clone(),
            scope_state.vectorscope.clone(),
            scope_state.cie.clone(),
        ) {
            outbound.send(ipc::BevyToUi::ScopeData {
                histogram: h,
                waveform: w,
                vectorscope: v,
                cie: c,
            });
        }
    }
}
