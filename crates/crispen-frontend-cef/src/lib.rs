//! CEF (Chromium Embedded Framework) offscreen webview implementation.
//!
//! Renders the Svelte UI to an offscreen BGRA buffer that is uploaded as a
//! Bevy texture each frame.  IPC is handled via `console.log` interception
//! (Svelte → Bevy) and `eval` injection (Bevy → Svelte).
//!
//! # Setup
//!
//! CEF binaries must be downloaded before use:
//! ```bash
//! ./scripts/setup-cef.sh
//! ```
//!
//! # Architecture
//!
//! CEF uses a multi-process model (browser, render, GPU).  Offscreen
//! rendering (OSR) works by:
//! 1. Creating a browser with `windowless_rendering_enabled`.
//! 2. Implementing a `RenderHandler` that receives BGRA paint callbacks.
//! 3. Storing the pixel buffer in an `Arc<Vec<u8>>` for zero-copy sharing.

pub mod browser;
pub mod capture;
pub mod devtools;

use browser::{SharedState, IPC_PREFIX};
use cef::{
    Browser, CefStringUtf16, ImplBrowser, ImplBrowserHost, ImplFrame, KeyEvent, KeyEventType,
    MouseButtonType,
};
use crispen_frontend_core::{
    CaptureResult, CompositeBackend, FrontendError, KeyboardEvent, MouseButton, MouseEvent,
};
use std::ffi::c_int;
use std::mem::size_of;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

// ── State machine ────────────────────────────────────────────────

/// CEF webview lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CefState {
    /// Browser is loading content.
    Loading,
    /// Ready for capture.
    Ready,
}

// ── Backend ──────────────────────────────────────────────────────

/// CEF-based offscreen webview backend.
pub struct CefBackend {
    size: (u32, u32),
    state: CefState,
    shared: Arc<SharedState>,
    browser: Option<Browser>,
    from_ui_rx: mpsc::UnboundedReceiver<String>,
    to_ui_messages: Vec<String>,
}

impl CefBackend {
    /// Create a new CEF offscreen webview.
    ///
    /// `html_content` is loaded via a `data:` URL.
    pub fn new(html_content: &str, size: (u32, u32)) -> Result<Self, FrontendError> {
        browser::ensure_cef_initialized()?;

        let (from_ui_tx, from_ui_rx) = mpsc::unbounded_channel();

        let shared = Arc::new(SharedState {
            framebuffer: Mutex::new(None),
            framebuffer_size: Mutex::new((0, 0)),
            dirty: Arc::new(AtomicBool::new(false)),
            size: Mutex::new(size),
            from_ui_tx,
        });

        let browser_instance = browser::create_browser(html_content, size, &shared)?;

        Ok(Self {
            size,
            state: CefState::Loading,
            shared,
            browser: Some(browser_instance),
            from_ui_rx,
            to_ui_messages: Vec::new(),
        })
    }

    /// Create a CEF backend that navigates to a URL instead of loading HTML.
    pub fn from_url(url: &str, size: (u32, u32)) -> Result<Self, FrontendError> {
        browser::ensure_cef_initialized()?;

        let (from_ui_tx, from_ui_rx) = mpsc::unbounded_channel();

        let shared = Arc::new(SharedState {
            framebuffer: Mutex::new(None),
            framebuffer_size: Mutex::new((0, 0)),
            dirty: Arc::new(AtomicBool::new(false)),
            size: Mutex::new(size),
            from_ui_tx,
        });

        let browser_instance = browser::create_browser_from_url(url, size, &shared)?;

        Ok(Self {
            size,
            state: CefState::Loading,
            shared,
            browser: Some(browser_instance),
            from_ui_rx,
            to_ui_messages: Vec::new(),
        })
    }

    /// Current lifecycle state.
    pub fn state(&self) -> CefState {
        self.state
    }

    /// Inject the JavaScript IPC bridge (`window.ipc.postMessage`).
    ///
    /// Also sends `RequestState` to ensure the backend sends `Initialize`
    /// even if the Svelte app's initial request was lost (race condition:
    /// the app may start before this bridge is injected).  Dispatches a
    /// `crispen-ipc-ready` event so BevyPanel can re-report its layout.
    fn inject_ipc_bridge(&self) {
        let js = format!(
            r#"
            (function() {{
                if (window.ipc) return;
                window.ipc = {{
                    postMessage: function(message) {{
                        console.log('{}' + message);
                    }}
                }};
                window.__CRISPEN_IPC__ = window.ipc;
                window.ipc.postMessage(JSON.stringify({{ type: 'UiDirty' }}));
                window.ipc.postMessage(JSON.stringify({{ type: 'RequestState' }}));
                console.log('Crispen IPC bridge initialised');
                window.dispatchEvent(new Event('crispen-ipc-ready'));
            }})();
            "#,
            IPC_PREFIX
        );

        if let Err(e) = self.eval(&js) {
            tracing::error!("failed to inject IPC bridge: {e}");
        } else {
            tracing::info!("CEF IPC bridge injected");
        }
    }

    /// Evaluate JavaScript in the webview.
    pub fn eval(&self, js: &str) -> Result<(), FrontendError> {
        let Some(browser) = &self.browser else {
            return Err(FrontendError::NotReady);
        };

        if let Some(frame) = browser.main_frame() {
            let js_string: CefStringUtf16 = js.into();
            let empty_url: CefStringUtf16 = "".into();
            frame.execute_java_script(Some(&js_string), Some(&empty_url), 0);
            Ok(())
        } else {
            Err(FrontendError::NotReady)
        }
    }

    /// Open Chrome DevTools.
    pub fn show_dev_tools(&self) {
        if let Some(browser) = &self.browser {
            devtools::show_dev_tools(browser);
        }
    }

    /// Toggle DevTools visibility.
    pub fn toggle_dev_tools(&self) {
        if let Some(browser) = &self.browser {
            devtools::toggle_dev_tools(browser);
        }
    }

    /// Flush queued messages to the Svelte UI via `eval`.
    fn flush_to_ui_messages(&mut self) {
        if self.to_ui_messages.is_empty() || self.state != CefState::Ready {
            return;
        }

        let messages = std::mem::take(&mut self.to_ui_messages);
        for json in messages {
            let js = format!(
                r#"if (window.__CRISPEN_RECEIVE__) {{ window.__CRISPEN_RECEIVE__('{}'); }}"#,
                json.replace('\\', "\\\\").replace('\'', "\\'")
            );
            if let Err(e) = self.eval(&js) {
                tracing::warn!("failed to send message to UI: {e}");
            }
        }
    }
}

// ── CompositeBackend impl ────────────────────────────────────────

impl CompositeBackend for CefBackend {
    fn poll(&mut self) {
        cef::do_message_loop_work();

        if self.state == CefState::Loading && capture::has_framebuffer(&self.shared) {
            self.state = CefState::Ready;
            tracing::info!("CEF webview ready");
            self.inject_ipc_bridge();
        }

        self.flush_to_ui_messages();
    }

    fn is_ready(&self) -> bool {
        self.state == CefState::Ready
    }

    fn capture_if_dirty(&mut self) -> Option<CaptureResult> {
        capture::capture_if_dirty(&self.shared)
    }

    fn size(&self) -> (u32, u32) {
        self.size
    }

    fn resize(&mut self, width: u32, height: u32) {
        if self.size == (width, height) {
            return;
        }
        self.size = (width, height);
        *self.shared.size.lock().unwrap() = (width, height);
        tracing::info!("CEF webview resized to {width}x{height}");

        if let Some(browser) = &self.browser {
            if let Some(host) = browser.host() {
                host.was_resized();
            }
        }
    }

    fn send_mouse_event(&mut self, event: MouseEvent) {
        let Some(browser) = &self.browser else { return };
        let Some(host) = browser.host() else { return };

        match event {
            MouseEvent::Move { x, y } => {
                let me = cef::MouseEvent { x: x as c_int, y: y as c_int, modifiers: 0 };
                host.send_mouse_move_event(Some(&me), 0);
            }
            MouseEvent::ButtonDown { button, x, y } => {
                let me = cef::MouseEvent { x: x as c_int, y: y as c_int, modifiers: 0 };
                host.send_mouse_click_event(Some(&me), to_cef_button(button), 0, 1);
            }
            MouseEvent::ButtonUp { button, x, y } => {
                let me = cef::MouseEvent { x: x as c_int, y: y as c_int, modifiers: 0 };
                host.send_mouse_click_event(Some(&me), to_cef_button(button), 1, 1);
            }
            MouseEvent::Scroll { x, y, delta_x, delta_y } => {
                let me = cef::MouseEvent { x: x as c_int, y: y as c_int, modifiers: 0 };
                host.send_mouse_wheel_event(Some(&me), delta_x as c_int, delta_y as c_int);
            }
        }
    }

    fn send_keyboard_event(&mut self, event: KeyboardEvent) {
        let Some(browser) = &self.browser else { return };
        let Some(host) = browser.host() else { return };

        let char_code = event.key.chars().next().unwrap_or('\0');
        let vk_code = if char_code.is_ascii_lowercase() {
            char_code.to_ascii_uppercase() as c_int
        } else {
            char_code as c_int
        };
        let typed_char = if event.modifiers.shift && char_code.is_ascii_lowercase() {
            char_code.to_ascii_uppercase()
        } else {
            char_code
        };

        let mut modifiers: u32 = 0;
        if event.modifiers.shift { modifiers |= 1 << 1; }
        if event.modifiers.ctrl { modifiers |= 1 << 2; }
        if event.modifiers.alt { modifiers |= 1 << 3; }

        let key_event = KeyEvent {
            size: size_of::<KeyEvent>(),
            type_: if event.pressed { KeyEventType::RAWKEYDOWN } else { KeyEventType::KEYUP },
            modifiers,
            windows_key_code: vk_code,
            native_key_code: 0,
            is_system_key: 0,
            character: typed_char as u16,
            unmodified_character: char_code as u16,
            focus_on_editable_field: 0,
        };
        host.send_key_event(Some(&key_event));

        if event.pressed && char_code != '\0' {
            let char_event = KeyEvent {
                size: size_of::<KeyEvent>(),
                type_: KeyEventType::CHAR,
                modifiers,
                windows_key_code: vk_code,
                native_key_code: 0,
                is_system_key: 0,
                character: typed_char as u16,
                unmodified_character: char_code as u16,
                focus_on_editable_field: 0,
            };
            host.send_key_event(Some(&char_event));
        }
    }

    fn send_to_ui(&mut self, json: String) -> Result<(), FrontendError> {
        self.to_ui_messages.push(json);
        Ok(())
    }

    fn try_recv_from_ui(&mut self) -> Option<String> {
        self.from_ui_rx.try_recv().ok()
    }

    fn show_dev_tools(&self) {
        self.show_dev_tools();
    }
}

impl Drop for CefBackend {
    fn drop(&mut self) {
        tracing::info!("dropping CEF webview");
        if let Some(browser) = self.browser.take() {
            if let Some(host) = browser.host() {
                host.close_browser(1);
            }
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────

fn to_cef_button(button: MouseButton) -> MouseButtonType {
    match button {
        MouseButton::Left => MouseButtonType::LEFT,
        MouseButton::Middle => MouseButtonType::MIDDLE,
        MouseButton::Right => MouseButtonType::RIGHT,
    }
}
