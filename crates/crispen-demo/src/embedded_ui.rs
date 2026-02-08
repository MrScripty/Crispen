//! Embedded UI HTML content for the wry webview.
//!
//! In development mode (`CRISPEN_DEV=1`), loads from the Vite dev server.
//! In release mode, serves a placeholder until the UI is built and embedded.

/// Vite dev server port (must match `ui/vite.config.ts`).
const VITE_DEV_PORT: u16 = 5174;

/// Get HTML content for the webview.
pub fn get_html(dev_mode: bool, ws_port: u16) -> String {
    if dev_mode {
        dev_html(ws_port)
    } else {
        placeholder_html(ws_port)
    }
}

fn dev_html(ws_port: u16) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Crispen</title>
    <script>window.__CRISPEN_WS_PORT__ = {ws_port};</script>
    <script type="module" src="http://localhost:{VITE_DEV_PORT}/@vite/client"></script>
    <script type="module" src="http://localhost:{VITE_DEV_PORT}/src/main.ts"></script>
    <style>
        html, body {{ margin: 0; padding: 0; background: transparent; overflow: hidden; }}
    </style>
</head>
<body>
    <div id="app"></div>
</body>
</html>"#
    )
}

fn placeholder_html(ws_port: u16) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <style>
        html, body {{
            margin: 0; padding: 0; background: rgba(30, 30, 30, 0.95);
            font-family: system-ui, -apple-system, sans-serif; color: white;
        }}
        .center {{ display: flex; align-items: center; justify-content: center;
                   height: 100vh; flex-direction: column; }}
        h1 {{ font-size: 24px; font-weight: 300; margin: 0 0 8px 0; }}
        p {{ font-size: 14px; color: rgba(255,255,255,0.5); margin: 0; }}
    </style>
</head>
<body>
    <div class="center">
        <h1>Crispen</h1>
        <p>Build the UI: cd crates/crispen-demo/ui &amp;&amp; npm run build</p>
        <p>WebSocket port: {ws_port}</p>
    </div>
</body>
</html>"#
    )
}
