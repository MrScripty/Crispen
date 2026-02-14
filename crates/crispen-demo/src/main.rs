//! Crispen Demo — standalone color grading application.
//!
//! Uses CEF offscreen compositing for the Svelte UI with Bevy-rendered
//! GPU widgets (color wheels, scopes, curves, viewer).

#[cfg(feature = "cef")]
mod cef_bridge;
mod config;
mod embedded_ui;
mod image_loader;
#[cfg(feature = "cef")]
mod input;
mod ipc;
#[cfg(feature = "cef")]
mod layout_sync;
mod ocio_support;
mod ui;
mod ws_bridge;

use bevy::input_focus::InputDispatchPlugin;
use bevy::prelude::*;
use bevy::window::WindowResolution;

use config::{AppConfig, FrontendMode};
use crispen_bevy::CrispenPlugin;
use crispen_bevy::events::{ImageLoadedEvent, ParamsUpdatedEvent, ScopeDataReadyEvent};
#[cfg(feature = "ocio")]
use crispen_bevy::resources::OcioColorManagement;
use crispen_bevy::resources::{GradingState, ScopeState};
#[cfg(feature = "ocio")]
use crispen_ocio::OcioConfig;

fn main() {
    let config = AppConfig::default();
    tracing::info!(
        "starting demo: {}x{}, dev_mode={}, frontend={:?}",
        config.width,
        config.height,
        config.dev_mode,
        config.frontend_mode
    );

    let window = Window {
        title: "Crispen".into(),
        resolution: WindowResolution::new(config.width as u32, config.height as u32)
            .with_scale_factor_override(1.0),
        present_mode: bevy::window::PresentMode::AutoVsync,
        ..default()
    };

    let mut app = App::new();
    let frontend_mode = config.frontend_mode;

    app.insert_resource(config)
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
        .add_plugins(CrispenPlugin);

    // ── Frontend mode ────────────────────────────────────────────
    match frontend_mode {
        #[cfg(feature = "cef")]
        FrontendMode::Cef => {
            app.add_plugins(cef_bridge::CefBridgePlugin)
                .add_plugins(input::InputForwardingPlugin)
                .add_plugins(layout_sync::LayoutSyncPlugin)
                .add_plugins(InputDispatchPlugin)
                .init_resource::<ui::viewer_nav::ViewerTransform>()
                .add_systems(
                    Startup,
                    (
                        ui::setup_ui_camera,
                        ui::viewer::setup_viewer,
                        spawn_cef_viewer_panel,
                        send_initial_state,
                    )
                        .chain(),
                )
                .add_systems(
                    Update,
                    (
                        forward_params_to_ui,
                        forward_scopes_to_ui,
                        forward_image_loaded_to_ui,
                        ui::systems::handle_load_image_shortcut,
                        ui::viewer::update_viewer_texture
                            .after(crispen_bevy::systems::consume_gpu_results),
                    ),
                );
        }
        _ => {
            // Legacy WebSocket bridge fallback.
            let (outbound_tx, inbound_rx) = ws_bridge::spawn_ws_server(
                app.world().resource::<AppConfig>().ws_port,
            );
            app.insert_resource(ws_bridge::OutboundUiMessages::default())
                .insert_resource(ws_bridge::WsBridge { outbound_tx, inbound_rx })
                .add_systems(Startup, send_initial_state)
                .add_systems(
                    Update,
                    (
                        ws_bridge::poll_inbound_messages,
                        ws_bridge::flush_outbound_messages,
                        forward_params_to_ui,
                        forward_scopes_to_ui,
                        forward_image_loaded_to_ui,
                    ),
                );

            // Full native Bevy UI (layout, toolbar, widgets).
            app.add_plugins(InputDispatchPlugin)
                .add_plugins(bevy::ui_widgets::UiWidgetsPlugins)
                .add_plugins(ui::CrispenUiPlugin);
        }
    }

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

    let display_oetf = infer_display_oetf(&display);

    app.insert_resource(OcioColorManagement {
        config,
        input_space: "sRGB - Texture".to_string(),
        working_space,
        display,
        view,
        idt_lut: None,
        odt_lut: None,
        display_oetf,
        dirty: true,
    });
    tracing::info!("OCIO enabled");
}

#[cfg(feature = "ocio")]
fn infer_display_oetf(display_name: &str) -> crispen_core::transform::params::DisplayOetf {
    use crispen_core::transform::params::DisplayOetf;
    let lower = display_name.to_ascii_lowercase();
    if lower.contains("pq") || lower.contains("st.2084") || lower.contains("st2084") {
        DisplayOetf::Pq
    } else if lower.contains("hlg") {
        DisplayOetf::Hlg
    } else {
        DisplayOetf::Srgb
    }
}

/// Spawn a viewer `ImageNode` in CEF mode, positioned by `LayoutSyncPlugin`.
///
/// The Svelte dockview marks `"viewer"` as a Bevy panel (transparent in CEF),
/// so this entity shows through the overlay once layout_sync receives the
/// `LayoutUpdate` with the viewer region.
#[cfg(feature = "cef")]
fn spawn_cef_viewer_panel(
    mut commands: Commands,
    viewer_handle: Res<ui::viewer::ViewerImageHandle>,
) {
    commands.spawn((
        layout_sync::LayoutPanel {
            panel_id: "viewer".into(),
        },
        ImageNode::new(viewer_handle.handle.clone()).with_mode(NodeImageMode::Stretch),
        Node {
            position_type: PositionType::Absolute,
            ..default()
        },
        // Render above the CEF overlay so the Bevy texture is visible through
        // the transparent cutout in the dockview panel.
        GlobalZIndex(i32::MAX),
        Visibility::Hidden,
    ));
}

/// Send initial state to the UI when the app starts.
fn send_initial_state(
    state: Res<GradingState>,
    #[cfg(feature = "cef")] mut cef_outbound: Option<ResMut<cef_bridge::OutboundUiMessages>>,
    #[cfg(not(feature = "cef"))] mut ws_outbound: ResMut<ws_bridge::OutboundUiMessages>,
) {
    let msg = ipc::BevyToUi::Initialize {
        params: state.params.clone(),
    };

    #[cfg(feature = "cef")]
    if let Some(ref mut out) = cef_outbound {
        out.send(msg);
        return;
    }

    #[cfg(not(feature = "cef"))]
    ws_outbound.send(msg);
}

/// Forward `ParamsUpdatedEvent` to the UI.
fn forward_params_to_ui(
    mut events: MessageReader<ParamsUpdatedEvent>,
    #[cfg(feature = "cef")] mut cef_outbound: Option<ResMut<cef_bridge::OutboundUiMessages>>,
    #[cfg(not(feature = "cef"))] mut ws_outbound: ResMut<ws_bridge::OutboundUiMessages>,
) {
    for event in events.read() {
        let msg = ipc::BevyToUi::ParamsUpdated {
            params: event.params.clone(),
        };

        #[cfg(feature = "cef")]
        if let Some(ref mut out) = cef_outbound {
            out.send(msg);
            continue;
        }

        #[cfg(not(feature = "cef"))]
        ws_outbound.send(msg);
    }
}

/// Forward `ImageLoadedEvent` to the UI.
fn forward_image_loaded_to_ui(
    mut events: MessageReader<ImageLoadedEvent>,
    #[cfg(feature = "cef")] mut cef_outbound: Option<ResMut<cef_bridge::OutboundUiMessages>>,
    #[cfg(not(feature = "cef"))] mut ws_outbound: ResMut<ws_bridge::OutboundUiMessages>,
) {
    for event in events.read() {
        let msg = ipc::BevyToUi::ImageLoaded {
            path: event.path.clone(),
            width: event.width,
            height: event.height,
            bit_depth: event.bit_depth.clone(),
        };

        #[cfg(feature = "cef")]
        if let Some(ref mut out) = cef_outbound {
            out.send(msg);
            continue;
        }

        #[cfg(not(feature = "cef"))]
        ws_outbound.send(msg);
    }
}

/// Forward scope data to the UI when ready.
fn forward_scopes_to_ui(
    mut events: MessageReader<ScopeDataReadyEvent>,
    scope_state: Res<ScopeState>,
    #[cfg(feature = "cef")] mut cef_outbound: Option<ResMut<cef_bridge::OutboundUiMessages>>,
    #[cfg(not(feature = "cef"))] mut ws_outbound: ResMut<ws_bridge::OutboundUiMessages>,
) {
    for _ in events.read() {
        if let (Some(h), Some(w), Some(v), Some(c)) = (
            scope_state.histogram.clone(),
            scope_state.waveform.clone(),
            scope_state.vectorscope.clone(),
            scope_state.cie.clone(),
        ) {
            let msg = ipc::BevyToUi::ScopeData {
                histogram: h,
                waveform: w,
                vectorscope: v,
                cie: c,
            };

            #[cfg(feature = "cef")]
            if let Some(ref mut out) = cef_outbound {
                out.send(msg);
                continue;
            }

            #[cfg(not(feature = "cef"))]
            ws_outbound.send(msg);
        }
    }
}
