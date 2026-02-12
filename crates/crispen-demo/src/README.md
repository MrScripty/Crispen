# Demo Application

## Purpose

Standalone color grading demo that runs the Crispen Bevy backend with a Svelte UI connected over WebSocket IPC. Handles application-level concerns: window setup, image loading, WebSocket bridge, and frontend mode selection.

## Contents

| File | Description |
|------|-------------|
| `main.rs` | App setup, camera, initial state, forwarding systems for params and scopes |
| `config.rs` | `AppConfig` — WebSocket port, window size, dev mode, frontend mode (from env vars) |
| `ipc.rs` | `BevyToUi` / `UiToBevy` message enums with serde tag+content serialization |
| `ws_bridge.rs` | WebSocket server, `OutboundUiMessages`, `WsBridge`, inbound/outbound systems |
| `image_loader.rs` | `load_image()` — loads PNG/JPEG/TIFF/EXR via the `image` crate to `GradingImage` |
| `embedded_ui.rs` | HTML generation for wry webview (dev mode: Vite, release: placeholder) |

## Design Decisions

- **WebSocket IPC**: Chosen over wry's native IPC for full bidirectional streaming of scope data. Matches Pentimento's pattern.
- **Image loading in bridge**: `LoadImage` is handled directly in `poll_inbound_messages` rather than as a `ColorGradingCommand`, because it requires file I/O and GPU upload that only the demo crate owns.
- **Backend-owned state**: Bevy is the single source of truth for `GradingParams`. The UI sends actions and receives state updates — no optimistic updates.
- **Frontend mode split**: Svelte mode keeps the native in-window Bevy layout as a fallback while Svelte drives controls via WebSocket; legacy mode is native-only.

## Dependencies

- **Internal**: `crispen-bevy` (plugin), `crispen-core` (domain types)
- **External**: `bevy`, `serde_json`, `tokio`, `tokio-tungstenite`, `futures-util`, `wry`, `image`, `tracing`, `thiserror`

## Usage Examples

```bash
# Development (Svelte frontend + Bevy backend)
CRISPEN_DEV=1 cargo run -p crispen-demo

# Then in another terminal:
cd crates/crispen-demo/ui && npm run dev

# Optional: run legacy native Bevy UI
CRISPEN_FRONTEND=bevy cargo run -p crispen-demo
```
