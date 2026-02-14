//! CEF compositing bridge — replaces the WebSocket bridge.
//!
//! Sets up CEF offscreen rendering, creates a full-screen overlay texture,
//! and forwards IPC messages between Bevy ECS and the Svelte UI.

use std::path::Path;
use std::sync::Arc;

use bevy::asset::RenderAssetUsages;
use bevy::picking::prelude::Pickable;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};

use crispen_frontend_cef::CefBackend;
use crispen_frontend_core::{CaptureResult, CompositeBackend};

use crate::config::AppConfig;
use crate::image_loader;
use crate::ipc::{BevyToUi, UiToBevy};
use crate::layout_sync::PanelLayout;
use crispen_bevy::events::{ColorGradingCommand, ImageLoadedEvent};
#[cfg(feature = "ocio")]
use crispen_bevy::resources::OcioColorManagement;
use crispen_bevy::resources::{GpuPipelineState, GradingState, ImageState};

// ── Plugin ───────────────────────────────────────────────────────

/// Plugin that registers all CEF-related resources and systems.
pub struct CefBridgePlugin;

impl Plugin for CefBridgePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OutboundUiMessages>()
            .init_resource::<CefWebviewStatus>()
            .init_resource::<CefLastWindowSize>()
            .add_systems(Startup, setup_cef_frontend.pipe(handle_cef_error))
            .add_systems(
                Update,
                (
                    update_cef_texture,
                    handle_cef_resize,
                    // Run before handle_grading_commands so that any remaining
                    // ColorGradingCommand messages (AutoBalance, ToggleScope, etc.)
                    // are written before they are consumed.
                    handle_cef_ipc
                        .before(crispen_bevy::systems::handle_grading_commands),
                    flush_outbound_messages,
                ),
            );
    }
}

// ── Resources ────────────────────────────────────────────────────

/// Non-Send resource holding the CEF backend (CEF is single-threaded).
pub struct CefFrontendResource {
    pub backend: CefBackend,
}

/// Bevy `Handle<Image>` for the overlay texture.
#[derive(Resource)]
pub struct CefUiTextureHandle {
    pub handle: Handle<Image>,
}

/// Marker component for the full-screen overlay `ImageNode`.
#[derive(Component)]
pub struct CefUiOverlay;

/// CEF lifecycle tracking.
#[derive(Resource, Default)]
pub struct CefWebviewStatus {
    pub initialized: bool,
    pub first_capture_done: bool,
}

/// Last window size (to detect resize).
#[derive(Resource, Default)]
pub struct CefLastWindowSize {
    pub width: u32,
    pub height: u32,
}

/// Outbound message queue — systems call `send()`, flushed each frame.
#[derive(Resource, Default)]
pub struct OutboundUiMessages {
    messages: Vec<BevyToUi>,
}

impl OutboundUiMessages {
    pub fn send(&mut self, msg: BevyToUi) {
        self.messages.push(msg);
    }

    pub fn drain(&mut self) -> Vec<BevyToUi> {
        std::mem::take(&mut self.messages)
    }
}

// ── Startup ──────────────────────────────────────────────────────

fn setup_cef_frontend(world: &mut World) -> Result<(), String> {
    let (width, height) = {
        let mut q = world.query::<&Window>();
        let window = q.iter(world).next().ok_or("no window found")?;
        (
            window.resolution.physical_width(),
            window.resolution.physical_height(),
        )
    };

    let config = world.resource::<AppConfig>();

    tracing::info!("setting up CEF UI composite ({width}x{height} physical)");

    let backend = if config.dev_mode {
        let url = format!("http://localhost:{}", crate::embedded_ui::VITE_DEV_PORT);
        tracing::info!("CEF dev mode: navigating to {url}");
        CefBackend::from_url(&url, (width, height))
    } else if let Some(index_path) = find_built_ui() {
        let url = format!("file://{}", index_path.display());
        tracing::info!("CEF production: loading {url}");
        CefBackend::from_url(&url, (width, height))
    } else {
        let html = crate::embedded_ui::get_html(config.dev_mode, config.ws_port);
        tracing::info!("CEF: built UI not found, using placeholder HTML");
        CefBackend::new(&html, (width, height))
    };

    let backend = backend.map_err(|e| format!("CEF creation failed: {e}"))?;
    world.insert_non_send_resource(CefFrontendResource { backend });

    // Create BGRA overlay texture.
    let mut image = Image::new_fill(
        Extent3d { width, height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;

    let handle = world.resource_mut::<Assets<Image>>().add(image);

    world.insert_resource(CefUiTextureHandle { handle: handle.clone() });
    world.insert_resource(CefLastWindowSize { width, height });

    // Full-screen overlay with pointer passthrough.
    world.spawn((
        ImageNode { image: handle, ..default() },
        Node {
            width: Val::Vw(100.0),
            height: Val::Vh(100.0),
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            ..default()
        },
        ZIndex(i32::MAX),
        CefUiOverlay,
        Pickable::IGNORE,
    ));

    tracing::info!("CEF UI overlay created");
    Ok(())
}

fn handle_cef_error(In(result): In<Result<(), String>>) {
    if let Err(e) = result {
        tracing::error!("{e}");
    }
}

// ── Update systems ───────────────────────────────────────────────

/// Poll CEF and upload dirty framebuffers to the overlay texture.
fn update_cef_texture(
    webview: Option<NonSendMut<CefFrontendResource>>,
    ui_tex: Option<Res<CefUiTextureHandle>>,
    mut images: ResMut<Assets<Image>>,
    mut status: ResMut<CefWebviewStatus>,
) {
    let Some(mut wv) = webview else { return };
    let Some(tex) = ui_tex else { return };

    wv.backend.poll();

    if !wv.backend.is_ready() {
        return;
    }
    if !status.initialized {
        tracing::info!("CEF webview ready");
        status.initialized = true;
    }

    if let Some(capture) = wv.backend.capture_if_dirty() {
        let CaptureResult::Bgra(bgra_arc, cap_w, cap_h) = capture else { return };

        if !status.first_capture_done {
            tracing::info!("first CEF capture: {cap_w}x{cap_h}");
            status.first_capture_done = true;
        }

        if let Some(image) = images.get_mut(&tex.handle) {
            if image.width() != cap_w || image.height() != cap_h {
                image.resize(Extent3d {
                    width: cap_w,
                    height: cap_h,
                    depth_or_array_layers: 1,
                });
            }
            let data = Arc::try_unwrap(bgra_arc).unwrap_or_else(|a| (*a).clone());
            image.data = Some(data);
        }
    }
}

/// Resize CEF backend when the Bevy window size changes.
///
/// Does NOT resize the Bevy texture here — `update_cef_texture` handles that
/// when the next CEF capture arrives with the new dimensions.  Calling
/// `image.resize()` eagerly would clear the framebuffer and blank the overlay
/// until CEF repaints (which is asynchronous).
fn handle_cef_resize(
    webview: Option<NonSendMut<CefFrontendResource>>,
    mut last_size: ResMut<CefLastWindowSize>,
    status: Res<CefWebviewStatus>,
    windows: Query<&Window>,
) {
    if !status.initialized {
        return;
    }
    let Some(mut wv) = webview else { return };
    let Ok(window) = windows.single() else { return };

    let w = window.resolution.physical_width();
    let h = window.resolution.physical_height();

    if (w, h) == (last_size.width, last_size.height) || w == 0 || h == 0 {
        return;
    }

    tracing::info!("window resized to {w}x{h}, updating CEF");
    last_size.width = w;
    last_size.height = h;
    wv.backend.resize(w, h);
}

/// Receive IPC messages from CEF and dispatch as ECS commands.
#[allow(clippy::too_many_arguments)]
fn handle_cef_ipc(
    webview: Option<NonSendMut<CefFrontendResource>>,
    config: Res<AppConfig>,
    mut commands: MessageWriter<ColorGradingCommand>,
    mut images: ResMut<ImageState>,
    mut gpu: Option<ResMut<GpuPipelineState>>,
    #[cfg(feature = "ocio")] mut ocio: Option<ResMut<OcioColorManagement>>,
    mut state: ResMut<GradingState>,
    mut outbound: ResMut<OutboundUiMessages>,
    mut image_loaded: MessageWriter<ImageLoadedEvent>,
    mut panel_layout: ResMut<PanelLayout>,
) {
    let Some(mut wv) = webview else { return };

    while let Some(json) = wv.backend.try_recv_from_ui() {
        let preview_size = preview_target_from_config(&config);
        match serde_json::from_str::<UiToBevy>(&json) {
            Ok(msg) => {
                tracing::debug!("CEF IPC received: {:?}", std::mem::discriminant(&msg));
                dispatch_ui_message(
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
                    &mut panel_layout,
                );
            }
            Err(e) => tracing::warn!("failed to parse UI message: {e}\n  json: {json}"),
        }
    }
}

/// Serialize and send queued outbound messages to CEF.
fn flush_outbound_messages(
    mut outbound: ResMut<OutboundUiMessages>,
    webview: Option<NonSendMut<CefFrontendResource>>,
) {
    let Some(mut wv) = webview else { return };

    for msg in outbound.drain() {
        match serde_json::to_string(&msg) {
            Ok(json) => {
                let _ = wv.backend.send_to_ui(json);
            }
            Err(e) => tracing::error!("failed to serialize BevyToUi: {e}"),
        }
    }
}

// ── Message dispatch ─────────────────────────────────────────────

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
    panel_layout: &mut ResMut<PanelLayout>,
) {
    match msg {
        UiToBevy::RequestState => {
            outbound.send(BevyToUi::Initialize {
                params: state.params.clone(),
            });
            if let Some(source) = images.source.as_ref() {
                outbound.send(BevyToUi::ImageLoaded {
                    path: images.source_path.clone().unwrap_or_default(),
                    width: source.width,
                    height: source.height,
                    bit_depth: format!("{:?}", source.source_bit_depth),
                });
            }
        }
        UiToBevy::SetParams { params } => {
            // Handle directly rather than routing through ColorGradingCommand
            // messages, which can be lost if handle_grading_commands runs before
            // handle_cef_ipc in the same frame.
            if state.params != params {
                tracing::info!(
                    "SetParams: updating grading state (bars: lift={:?}, gamma={:?}, gain={:?}, offset={:?}; wheels: lift={:?}, gamma={:?}, gain={:?}, offset={:?})",
                    params.lift, params.gamma, params.gain, params.offset,
                    params.lift_wheel, params.gamma_wheel, params.gain_wheel, params.offset_wheel
                );
                state.params = params.clone();
                state.dirty = true;
                outbound.send(BevyToUi::ParamsUpdated { params });
            }
        }
        UiToBevy::AutoBalance => {
            commands.write(ColorGradingCommand::AutoBalance);
        }
        UiToBevy::ResetGrade => {
            // Handle directly to avoid command ordering issues.
            let defaults = crispen_core::transform::params::GradingParams::default();
            if state.params != defaults {
                tracing::info!("ResetGrade: resetting to defaults");
                state.params = defaults.clone();
                state.dirty = true;
                outbound.send(BevyToUi::ParamsUpdated { params: defaults });
            }
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
        UiToBevy::ToggleScope { scope_type, visible } => {
            commands.write(ColorGradingCommand::ToggleScope { scope_type, visible });
        }
        UiToBevy::UiDirty => {
            // Handled internally by CEF dirty flag — nothing to do here.
        }
        UiToBevy::LayoutUpdate { regions } => {
            tracing::info!(
                "layout update: {} regions: {:?}",
                regions.len(),
                regions.iter().map(|r| (&r.id, r.x, r.y, r.width, r.height, r.visible)).collect::<Vec<_>>()
            );
            panel_layout.regions = regions;
        }
        UiToBevy::SaveLayout { layout_json } => {
            if let Some(dir) = config_dir() {
                let path = dir.join("layout.json");
                if let Err(e) = std::fs::create_dir_all(&dir) {
                    tracing::warn!("failed to create config dir: {e}");
                } else if let Err(e) = std::fs::write(&path, &layout_json) {
                    tracing::warn!("failed to save layout: {e}");
                } else {
                    tracing::debug!("layout saved ({} bytes)", layout_json.len());
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
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
    #[cfg(feature = "ocio")]
    let result = image_loader::load_image_oiio(Path::new(path), preview_size);
    #[cfg(not(feature = "ocio"))]
    let result = image_loader::load_image_for_display(Path::new(path), preview_size);

    match result {
        Ok(loaded) => {
            let img = loaded.image;
            let width = img.width;
            let height = img.height;
            let bit_depth = format!("{:?}", img.source_bit_depth);

            if let Some(gpu) = gpu {
                let handle = gpu.pipeline.upload_image(&img);
                gpu.source_handle = Some(handle);
            }

            images.source = Some(img);
            images.source_path = Some(path.to_string());
            state.dirty = true;

            #[cfg(feature = "ocio")]
            if let Some(ocio) = ocio_state {
                if let Some(ref cs) = loaded.detected_color_space {
                    ocio.input_space = cs.clone();
                } else {
                    let detected_space = state.params.color_management.input_space;
                    ocio.input_space = crate::ocio_support::map_detected_to_ocio_name(
                        detected_space,
                        &ocio.config,
                    );
                }
                ocio.dirty = true;
            }

            image_loaded.write(ImageLoadedEvent {
                path: path.to_string(),
                width,
                height,
                bit_depth: bit_depth.clone(),
            });
            outbound.send(BevyToUi::ImageLoaded {
                path: path.to_string(),
                width,
                height,
                bit_depth,
            });
            tracing::info!("image loaded: {path} ({width}x{height})");
        }
        Err(e) => {
            tracing::error!("failed to load image {path}: {e}");
            outbound.send(BevyToUi::Error {
                message: format!("Failed to load image: {e}"),
            });
        }
    }
}

fn preview_target_from_config(config: &AppConfig) -> Option<(u32, u32)> {
    let w = (config.width - 24.0).max(128.0).round() as u32;
    let h = (config.height - crate::ui::theme::PRIMARIES_PANEL_HEIGHT - 32.0)
        .max(128.0)
        .round() as u32;
    Some((w, h))
}

/// Locate the built Svelte UI (dist/ui/index.html) relative to the executable
/// or the project root.
fn find_built_ui() -> Option<std::path::PathBuf> {
    // 1. Next to the executable (for packaged builds).
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("ui/index.html");
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    // 2. Relative to the working directory (development layout).
    let candidates = [
        Path::new("crates/crispen-demo/dist/ui/index.html"),
        Path::new("dist/ui/index.html"),
    ];
    for candidate in &candidates {
        if candidate.exists() {
            return std::fs::canonicalize(candidate).ok();
        }
    }

    None
}

fn config_dir() -> Option<std::path::PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        Some(std::path::PathBuf::from(xdg).join("crispen"))
    } else if let Ok(home) = std::env::var("HOME") {
        Some(std::path::PathBuf::from(home).join(".config/crispen"))
    } else {
        None
    }
}
