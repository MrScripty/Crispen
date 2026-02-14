//! CEF browser creation, handler wiring, and subprocess management.

use cef::args::Args;
use cef::rc::Rc as _;
use cef::{
    api_hash, sys, wrap_app, wrap_client, wrap_display_handler, wrap_render_handler, App, Browser,
    BrowserSettings, CefString, CefStringUtf16, Client, CommandLine, DisplayHandler, ImplApp,
    ImplClient, ImplCommandLine, ImplDisplayHandler, ImplRenderHandler, LogSeverity,
    PaintElementType, Rect, RenderHandler, Settings, WindowInfo, WrapApp, WrapClient,
    WrapDisplayHandler, WrapRenderHandler,
};
use crispen_frontend_core::FrontendError;
use std::ffi::c_int;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::mpsc;

/// Global flag guarding one-time CEF initialisation.
static CEF_INITIALIZED: OnceLock<bool> = OnceLock::new();

/// Prefix for IPC messages sent via `console.log` from JavaScript.
pub(crate) const IPC_PREFIX: &str = "__CRISPEN_IPC__:";

// ── Shared state ─────────────────────────────────────────────────

/// Thread-safe state shared between the render/display handlers and
/// [`CefBackend`](super::CefBackend).
pub(crate) struct SharedState {
    /// BGRA pixel buffer wrapped in `Arc` for zero-copy sharing.
    pub framebuffer: Mutex<Option<Arc<Vec<u8>>>>,
    /// Dimensions of the current framebuffer.
    pub framebuffer_size: Mutex<(u32, u32)>,
    /// Set when the framebuffer has been updated.
    pub dirty: Arc<AtomicBool>,
    /// Current viewport size.
    pub size: Mutex<(u32, u32)>,
    /// Channel for forwarding IPC messages from JavaScript to Bevy.
    pub from_ui_tx: mpsc::UnboundedSender<String>,
}

// ── Render handler ───────────────────────────────────────────────

#[derive(Clone)]
pub(crate) struct OsrRenderHandler {
    pub shared: Arc<SharedState>,
}

wrap_render_handler! {
    pub(crate) struct RenderHandlerBuilder {
        handler: OsrRenderHandler,
    }

    impl RenderHandler {
        fn view_rect(&self, _browser: Option<&mut Browser>, rect: Option<&mut Rect>) {
            if let Some(rect) = rect {
                let size = self.handler.shared.size.lock().unwrap();
                rect.x = 0;
                rect.y = 0;
                rect.width = size.0 as c_int;
                rect.height = size.1 as c_int;
            }
        }

        fn on_paint(
            &self,
            _browser: Option<&mut Browser>,
            type_: PaintElementType,
            _dirty_rects: Option<&[Rect]>,
            buffer: *const u8,
            width: c_int,
            height: c_int,
        ) {
            if type_ != PaintElementType::VIEW {
                return;
            }
            if buffer.is_null() || width <= 0 || height <= 0 {
                return;
            }

            let width = width as u32;
            let height = height as u32;
            let len = (width * height * 4) as usize;

            // Safety: CEF guarantees the buffer is valid for the duration of `on_paint`.
            let bgra = unsafe { std::slice::from_raw_parts(buffer, len) };
            let buffer_copy = Arc::new(bgra.to_vec());

            *self.handler.shared.framebuffer.lock().unwrap() = Some(buffer_copy);
            *self.handler.shared.framebuffer_size.lock().unwrap() = (width, height);
            self.handler.shared.dirty.store(true, Ordering::SeqCst);
        }
    }
}

impl RenderHandlerBuilder {
    pub fn build(handler: OsrRenderHandler) -> RenderHandler {
        Self::new(handler)
    }
}

// ── Display handler (IPC interception) ───────────────────────────

#[derive(Clone)]
pub(crate) struct OsrDisplayHandler {
    pub shared: Arc<SharedState>,
}

wrap_display_handler! {
    pub(crate) struct DisplayHandlerBuilder {
        handler: OsrDisplayHandler,
    }

    impl DisplayHandler {
        fn on_console_message(
            &self,
            _browser: Option<&mut Browser>,
            _level: LogSeverity,
            message: Option<&CefString>,
            _source: Option<&CefString>,
            _line: c_int,
        ) -> c_int {
            if let Some(msg) = message {
                let msg_str = msg.to_string();
                if let Some(json_str) = msg_str.strip_prefix(IPC_PREFIX) {
                    // Mark dirty for UiDirty messages (fast path).
                    if json_str.contains("\"UiDirty\"") {
                        self.handler.shared.dirty.store(true, Ordering::SeqCst);
                    }
                    let _ = self.handler.shared.from_ui_tx.send(json_str.to_string());
                    return 1; // suppress from console
                }
            }
            0
        }
    }
}

impl DisplayHandlerBuilder {
    pub fn build(handler: OsrDisplayHandler) -> DisplayHandler {
        Self::new(handler)
    }
}

// ── Client (wires render + display handlers) ─────────────────────

wrap_client! {
    pub(crate) struct ClientBuilder {
        render_handler: RenderHandler,
        display_handler: DisplayHandler,
    }

    impl Client {
        fn render_handler(&self) -> Option<cef::RenderHandler> {
            Some(self.render_handler.clone())
        }

        fn display_handler(&self) -> Option<cef::DisplayHandler> {
            Some(self.display_handler.clone())
        }
    }
}

impl ClientBuilder {
    pub fn build(shared: Arc<SharedState>) -> Client {
        let rh = RenderHandlerBuilder::build(OsrRenderHandler { shared: Arc::clone(&shared) });
        let dh = DisplayHandlerBuilder::build(OsrDisplayHandler { shared });
        Self::new(rh, dh)
    }
}

// ── Minimal App ──────────────────────────────────────────────────

#[derive(Clone)]
pub(crate) struct OsrApp;

wrap_app! {
    pub(crate) struct AppBuilder {
        app: OsrApp,
    }

    impl App {
        fn on_before_command_line_processing(
            &self,
            _process_type: Option<&CefString>,
            command_line: Option<&mut CommandLine>,
        ) {
            if let Some(cmd) = command_line {
                // Allow file:// pages to load sibling CSS/JS assets.
                let switch: CefString = "allow-file-access-from-files".into();
                cmd.append_switch(Some(&switch));
            }
        }
    }
}

impl AppBuilder {
    pub fn build() -> App {
        Self::new(OsrApp)
    }
}

// ── Helper binary discovery ──────────────────────────────────────

fn find_helper_binary() -> Option<String> {
    if let Ok(path) = std::env::var("CEF_HELPER_PATH") {
        if std::path::Path::new(&path).exists() {
            return Some(path);
        }
        tracing::warn!("CEF_HELPER_PATH set but file not found: {path}");
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let helper = dir.join("crispen-cef-helper");
            if helper.exists() {
                return helper.to_str().map(|s| s.to_string());
            }
        }
    }

    None
}

// ── CEF directory discovery ──────────────────────────────────────

/// Find the CEF resource directory (containing locales/, *.pak, icudtl.dat).
fn find_cef_dir() -> Option<std::path::PathBuf> {
    // 1. Explicit env var (e.g. set by launcher script).
    if let Ok(dir) = std::env::var("CEF_DIR") {
        let p = std::path::PathBuf::from(&dir);
        if p.join("icudtl.dat").exists() {
            return Some(p);
        }
    }

    // 2. Resolve from LD_LIBRARY_PATH (where libcef.so lives).
    if let Ok(ld) = std::env::var("LD_LIBRARY_PATH") {
        for entry in ld.split(':') {
            let p = std::path::PathBuf::from(entry);
            // CEF requires absolute paths.
            let p = if p.is_relative() {
                std::env::current_dir()
                    .ok()
                    .map(|cwd| cwd.join(&p))
                    .unwrap_or(p)
            } else {
                p
            };
            if p.join("libcef.so").exists() && p.join("icudtl.dat").exists() {
                return Some(p);
            }
        }
    }

    // 3. Same directory as the running executable.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if dir.join("icudtl.dat").exists() {
                return Some(dir.to_path_buf());
            }
        }
    }

    None
}

// ── Initialisation ───────────────────────────────────────────────

/// Initialise CEF (idempotent — only runs once per process).
pub fn ensure_cef_initialized() -> Result<(), FrontendError> {
    CEF_INITIALIZED.get_or_init(|| {
        tracing::info!("initialising CEF…");

        let helper_path = match find_helper_binary() {
            Some(p) => {
                tracing::info!("using CEF helper: {p}");
                p
            }
            None => {
                tracing::error!(
                    "CEF helper binary not found.  Build crispen-cef-helper and place it \
                     next to the main executable, or set CEF_HELPER_PATH."
                );
                return false;
            }
        };

        let mut settings = Settings::default();
        settings.windowless_rendering_enabled = 1;
        settings.no_sandbox = 1;
        settings.external_message_pump = 1;
        settings.multi_threaded_message_loop = 0;
        settings.browser_subprocess_path = helper_path.as_str().into();

        // CEF resource paths — use CEF_DIR env (set by cef-dll-sys build.rs) or
        // fall back to the directory containing libcef.so.
        if let Some(cef_dir) = find_cef_dir() {
            tracing::info!("CEF resource dir: {}", cef_dir.display());
            let locales = cef_dir.join("locales");
            settings.resources_dir_path = cef_dir.to_str().unwrap_or_default().into();
            settings.locales_dir_path = locales.to_str().unwrap_or_default().into();
        }

        // Set root_cache_path to suppress singleton behavior warning.
        if let Ok(home) = std::env::var("HOME") {
            let cache = std::path::PathBuf::from(home).join(".cache/crispen/cef");
            let _ = std::fs::create_dir_all(&cache);
            settings.root_cache_path = cache.to_str().unwrap_or_default().into();
        }

        let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);

        let args = Args::new();
        let mut app = AppBuilder::build();

        let exec_result =
            cef::execute_process(Some(args.as_main_args()), Some(&mut app), std::ptr::null_mut());
        if exec_result >= 0 {
            tracing::warn!("execute_process returned {exec_result} — unexpected for browser process");
        }

        let ok =
            cef::initialize(Some(args.as_main_args()), Some(&settings), Some(&mut app), std::ptr::null_mut());

        if ok == 0 {
            tracing::error!("CEF initialisation failed");
            return false;
        }

        tracing::info!("CEF initialised");
        true
    });

    if *CEF_INITIALIZED.get().unwrap_or(&false) {
        Ok(())
    } else {
        Err(FrontendError::Backend("CEF initialisation failed".into()))
    }
}

// ── Browser creation ─────────────────────────────────────────────

/// Create a CEF browser that loads HTML content via a `data:` URL.
pub(crate) fn create_browser(
    html_content: &str,
    size: (u32, u32),
    shared: &Arc<SharedState>,
) -> Result<Browser, FrontendError> {
    let encoded = urlencoding::encode(html_content);
    let data_url = format!("data:text/html,{encoded}");
    create_browser_from_url(&data_url, size, shared)
}

/// Create a CEF browser that navigates to an arbitrary URL.
pub(crate) fn create_browser_from_url(
    url: &str,
    size: (u32, u32),
    shared: &Arc<SharedState>,
) -> Result<Browser, FrontendError> {
    let mut client = ClientBuilder::build(Arc::clone(shared));

    let mut window_info = WindowInfo::default();
    window_info.windowless_rendering_enabled = 1;
    window_info.bounds.width = size.0 as c_int;
    window_info.bounds.height = size.1 as c_int;

    let mut browser_settings = BrowserSettings::default();
    let mut cef_url: CefStringUtf16 = url.into();

    tracing::info!("creating CEF browser at {url} ({}x{})", size.0, size.1);

    cef::browser_host_create_browser_sync(
        Some(&mut window_info),
        Some(&mut client),
        Some(&mut cef_url),
        Some(&mut browser_settings),
        None,
        None,
    )
    .ok_or_else(|| FrontendError::Backend("failed to create CEF browser".into()))
}
