//! WebSocket IPC bridge between Bevy and the Svelte UI.
//!
//! Follows Pentimento's `OutboundUiMessages` pattern, but uses WebSocket
//! (tokio-tungstenite) instead of wry's native IPC for full bidirectional
//! streaming of scope data and parameter updates.

use std::path::Path;

use bevy::prelude::*;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use crate::config::AppConfig;
use crate::image_loader;
use crate::ipc::{BevyToUi, UiToBevy};
use crispen_bevy::events::{ColorGradingCommand, ImageLoadedEvent};
#[cfg(feature = "ocio")]
use crispen_bevy::resources::OcioColorManagement;
use crispen_bevy::resources::{GpuPipelineState, GradingState, ImageState};

/// Resource holding outbound messages to send to the UI.
///
/// Systems queue messages via `send()`, and the `flush_outbound_messages`
/// system drains them each frame and forwards over WebSocket.
#[derive(Resource, Default)]
pub struct OutboundUiMessages {
    messages: Vec<BevyToUi>,
}

impl OutboundUiMessages {
    /// Queue a message to send to the UI.
    pub fn send(&mut self, msg: BevyToUi) {
        self.messages.push(msg);
    }

    /// Drain all queued messages, returning them.
    pub fn drain(&mut self) -> Vec<BevyToUi> {
        std::mem::take(&mut self.messages)
    }
}

/// Resource holding the channel endpoints for WebSocket IPC.
#[derive(Resource)]
pub struct WsBridge {
    /// Send messages from Bevy to the WebSocket server (-> UI).
    pub outbound_tx: mpsc::UnboundedSender<String>,
    /// Receive messages from the WebSocket server (<- UI).
    pub inbound_rx: mpsc::UnboundedReceiver<String>,
}

/// Spawn the WebSocket server on a dedicated thread.
///
/// Returns channel endpoints for Bevy systems to communicate with.
/// The server listens on `ws://127.0.0.1:{port}` and handles one
/// client connection at a time (the Svelte UI).
pub fn spawn_ws_server(
    port: u16,
) -> (
    mpsc::UnboundedSender<String>,
    mpsc::UnboundedReceiver<String>,
) {
    let (bevy_to_ws_tx, mut bevy_to_ws_rx) = mpsc::unbounded_channel::<String>();
    let (ws_to_bevy_tx, ws_to_bevy_rx) = mpsc::unbounded_channel::<String>();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime for WS bridge");

        rt.block_on(async move {
            let addr = format!("127.0.0.1:{port}");
            let listener = tokio::net::TcpListener::bind(&addr)
                .await
                .expect("failed to bind WebSocket server");

            tracing::info!("WebSocket IPC server listening on ws://{addr}");

            // Accept connections in a loop, but only one client at a time.
            // When a client disconnects, we accept the next one.
            loop {
                let Ok((stream, peer)) = listener.accept().await else {
                    continue;
                };
                tracing::info!("WebSocket client connected: {peer}");

                let ws_stream = match tokio_tungstenite::accept_async(stream).await {
                    Ok(ws) => ws,
                    Err(e) => {
                        tracing::error!("WebSocket handshake failed: {e}");
                        continue;
                    }
                };

                let (mut ws_sink, mut ws_source) = ws_stream.split();
                let tx = ws_to_bevy_tx.clone();

                // Forward incoming WS messages to Bevy
                let recv_handle = tokio::spawn(async move {
                    while let Some(Ok(msg)) = ws_source.next().await {
                        if let tokio_tungstenite::tungstenite::Message::Text(text) = msg
                            && tx.send(text.to_string()).is_err()
                        {
                            break;
                        }
                    }
                });

                // Forward Bevy messages to WS client (inline, not spawned,
                // so bevy_to_ws_rx isn't moved into a closure).
                // Pin the JoinHandle so select! can poll it by &mut ref.
                tokio::pin!(recv_handle);
                loop {
                    tokio::select! {
                        result = &mut recv_handle => {
                            // Client disconnected (recv task ended)
                            let _ = result;
                            break;
                        }
                        msg = bevy_to_ws_rx.recv() => {
                            match msg {
                                Some(text) => {
                                    let ws_msg = tokio_tungstenite::tungstenite::Message::Text(text.into());
                                    if ws_sink.send(ws_msg).await.is_err() {
                                        break;
                                    }
                                }
                                None => break,
                            }
                        }
                    }
                }

                tracing::info!("WebSocket client disconnected");
            }
        });
    });

    (bevy_to_ws_tx, ws_to_bevy_rx)
}

/// Bevy system: sends queued outbound messages over the WebSocket bridge.
pub fn flush_outbound_messages(mut outbound: ResMut<OutboundUiMessages>, bridge: Res<WsBridge>) {
    for msg in outbound.drain() {
        match serde_json::to_string(&msg) {
            Ok(json) => {
                let _ = bridge.outbound_tx.send(json);
            }
            Err(e) => tracing::error!("Failed to serialize BevyToUi: {e}"),
        }
    }
}

/// Bevy system: receives inbound messages from the WebSocket bridge
/// and dispatches them as `ColorGradingCommand` messages.
///
/// `LoadImage` is handled directly here (loading the file and uploading
/// to the GPU) rather than forwarded as a command, because the demo crate
/// owns the image loader and GPU resource access.
#[allow(clippy::too_many_arguments)]
pub fn poll_inbound_messages(
    mut bridge: ResMut<WsBridge>,
    config: Res<AppConfig>,
    mut commands: MessageWriter<ColorGradingCommand>,
    mut images: ResMut<ImageState>,
    mut gpu: Option<ResMut<GpuPipelineState>>,
    #[cfg(feature = "ocio")] mut ocio: Option<ResMut<OcioColorManagement>>,
    mut state: ResMut<GradingState>,
    mut outbound: ResMut<OutboundUiMessages>,
    mut image_loaded: MessageWriter<ImageLoadedEvent>,
) {
    while let Ok(json) = bridge.inbound_rx.try_recv() {
        let preview_size = preview_target_from_config(&config);
        match serde_json::from_str::<UiToBevy>(&json) {
            Ok(msg) => dispatch_ui_message(
                msg,
                preview_size,
                &mut commands,
                &mut images,
                gpu.as_deref_mut(),
                #[cfg(feature = "ocio")]
                ocio.as_deref_mut(),
                &mut state,
                &mut outbound,
                &mut image_loaded,
            ),
            Err(e) => tracing::warn!("Failed to parse UI message: {e}"),
        }
    }
}

/// Convert a `UiToBevy` message into the appropriate ECS action.
///
/// Most messages become `ColorGradingCommand` events. `LoadImage` is
/// handled directly since it requires file I/O and GPU upload.
#[allow(clippy::too_many_arguments)]
fn dispatch_ui_message(
    msg: UiToBevy,
    preview_size: Option<(u32, u32)>,
    commands: &mut MessageWriter<ColorGradingCommand>,
    images: &mut ResMut<ImageState>,
    gpu: Option<&mut GpuPipelineState>,
    #[cfg(feature = "ocio")] ocio_state: Option<&mut OcioColorManagement>,
    state: &mut ResMut<GradingState>,
    outbound: &mut ResMut<OutboundUiMessages>,
    image_loaded: &mut MessageWriter<ImageLoadedEvent>,
) {
    match msg {
        UiToBevy::SetParams { params } => {
            commands.write(ColorGradingCommand::SetParams { params });
        }
        UiToBevy::AutoBalance => {
            commands.write(ColorGradingCommand::AutoBalance);
        }
        UiToBevy::ResetGrade => {
            commands.write(ColorGradingCommand::ResetGrade);
        }
        UiToBevy::LoadImage { path } => {
            handle_load_image(
                &path,
                preview_size,
                images,
                gpu,
                #[cfg(feature = "ocio")]
                ocio_state,
                state,
                outbound,
                image_loaded,
            );
        }
        UiToBevy::LoadLut { path, slot } => {
            commands.write(ColorGradingCommand::LoadLut { path, slot });
        }
        UiToBevy::ExportLut { path, size } => {
            commands.write(ColorGradingCommand::ExportLut { path, size });
        }
        UiToBevy::ToggleScope {
            scope_type,
            visible,
        } => {
            commands.write(ColorGradingCommand::ToggleScope {
                scope_type,
                visible,
            });
        }
    }
}

/// Load an image from disk, upload to GPU, and update ECS state.
fn handle_load_image(
    path: &str,
    preview_size: Option<(u32, u32)>,
    images: &mut ResMut<ImageState>,
    gpu: Option<&mut GpuPipelineState>,
    #[cfg(feature = "ocio")] ocio_state: Option<&mut OcioColorManagement>,
    state: &mut ResMut<GradingState>,
    outbound: &mut ResMut<OutboundUiMessages>,
    image_loaded: &mut MessageWriter<ImageLoadedEvent>,
) {
    match image_loader::load_image_for_display(Path::new(path), preview_size) {
        Ok(img) => {
            let width = img.width;
            let height = img.height;
            let bit_depth = format!("{:?}", img.source_bit_depth);

            // Upload to GPU if pipeline is available.
            if let Some(gpu) = gpu {
                let handle = gpu.pipeline.upload_image(&img);
                gpu.source_handle = Some(handle);
            }

            images.source = Some(img);
            state.dirty = true;

            #[cfg(feature = "ocio")]
            if let Some(ocio) = ocio_state {
                let detected_space = state.params.color_management.input_space;
                ocio.input_space =
                    crate::ocio_support::map_detected_to_ocio_name(detected_space, &ocio.config);
                ocio.dirty = true;
            }

            image_loaded.write(ImageLoadedEvent {
                width,
                height,
                bit_depth: bit_depth.clone(),
            });
            outbound.send(BevyToUi::ImageLoaded {
                width,
                height,
                bit_depth,
            });

            tracing::info!("Image loaded: {path} ({width}x{height})");
        }
        Err(e) => {
            tracing::error!("Failed to load image {path}: {e}");
            outbound.send(BevyToUi::Error {
                message: format!("Failed to load image: {e}"),
            });
        }
    }
}

fn preview_target_from_config(config: &AppConfig) -> Option<(u32, u32)> {
    let width = config.width;
    let height = config.height;
    let target_width = (width - 24.0).max(128.0).round() as u32;
    let target_height = (height - crate::ui::theme::PRIMARIES_PANEL_HEIGHT - 32.0)
        .max(128.0)
        .round() as u32;
    Some((target_width, target_height))
}
