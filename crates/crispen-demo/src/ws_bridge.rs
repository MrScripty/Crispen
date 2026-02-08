//! WebSocket IPC bridge between Bevy and the Svelte UI.
//!
//! Follows Pentimento's `OutboundUiMessages` pattern, but uses WebSocket
//! (tokio-tungstenite) instead of wry's native IPC for full bidirectional
//! streaming of scope data and parameter updates.

use bevy::prelude::*;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use crate::ipc::{BevyToUi, UiToBevy};
use crispen_bevy::events::ColorGradingCommand;

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
pub fn flush_outbound_messages(
    mut outbound: ResMut<OutboundUiMessages>,
    bridge: Res<WsBridge>,
) {
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
pub fn poll_inbound_messages(
    mut bridge: ResMut<WsBridge>,
    mut commands: MessageWriter<ColorGradingCommand>,
) {
    while let Ok(json) = bridge.inbound_rx.try_recv() {
        match serde_json::from_str::<UiToBevy>(&json) {
            Ok(msg) => dispatch_ui_message(msg, &mut commands),
            Err(e) => tracing::warn!("Failed to parse UI message: {e}"),
        }
    }
}

/// Convert a `UiToBevy` message into the appropriate ECS command.
fn dispatch_ui_message(
    msg: UiToBevy,
    commands: &mut MessageWriter<ColorGradingCommand>,
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
            commands.write(ColorGradingCommand::LoadImage { path });
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
