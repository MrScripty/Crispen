//! Scope mask tool — freehand draw on the viewer to restrict scope analysis to a region.
//!
//! The user draws a polygon on the image viewer; only pixels inside the polygon
//! are included in scope computations. The polygon auto-closes as a loop and the
//! scopes update in real-time during drawing.

use bevy::asset::RenderAssetUsages;
use bevy::picking::Pickable;
use bevy::picking::events::{Drag, DragEnd, DragStart, Pointer, Press};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::{ComputedNode, ComputedUiRenderTargetInfo, UiGlobalTransform, UiScale};

use crispen_bevy::resources::{GradingState, ImageState, ScopeMaskData};

use super::split_viewer::GradedImageNode;
use super::theme;

// ── Resources ───────────────────────────────────────────────────────

/// UI-side state for the scope mask drawing tool.
#[derive(Resource, Default)]
pub struct ScopeMaskState {
    /// Whether the mask drawing tool is active (user can draw).
    pub tool_active: bool,
    /// Whether the user is currently drawing (mouse held down).
    pub drawing: bool,
    /// Normalized polygon vertices (0..1 in image UV space).
    pub polygon: Vec<Vec2>,
    /// Whether the polygon has changed since last GPU upload / overlay render.
    pub mask_dirty: bool,
}

/// Handle to the overlay texture rendered on top of the viewer image.
#[derive(Resource)]
pub struct ScopeMaskOverlayHandle {
    pub handle: Handle<Image>,
}

// ── Marker components ───────────────────────────────────────────────

/// Mask toggle button in the scope header.
#[derive(Component)]
pub struct ScopeMaskButton;

/// Clear mask button in the scope header.
#[derive(Component)]
pub struct ScopeMaskClearButton;

/// Overlay node rendered on top of the viewer image.
#[derive(Component)]
pub struct ScopeMaskOverlayNode;

/// Text label on the mask toggle button (for highlight sync).
#[derive(Component)]
pub struct ScopeMaskButtonLabel;

// ── Plugin ──────────────────────────────────────────────────────────

pub struct ScopeMaskPlugin;

impl Plugin for ScopeMaskPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScopeMaskState>()
            .add_systems(Startup, setup_scope_mask)
            .add_systems(
                Update,
                (
                    ensure_overlay_spawned,
                    handle_mask_button_interactions,
                    handle_mask_shortcuts,
                    update_scope_mask,
                    update_mask_overlay_texture,
                    sync_mask_button_visuals,
                ),
            )
            .add_observer(on_viewer_press)
            .add_observer(on_viewer_drag_start)
            .add_observer(on_viewer_drag)
            .add_observer(on_viewer_drag_end);
    }
}

// ── Setup ───────────────────────────────────────────────────────────

fn setup_scope_mask(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let placeholder = Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let handle = images.add(placeholder);
    commands.insert_resource(ScopeMaskOverlayHandle {
        handle: handle.clone(),
    });
}

/// One-shot system: spawn the mask overlay as a child of `GradedImageNode`
/// once both the overlay handle and the image entity exist.
fn ensure_overlay_spawned(
    mut commands: Commands,
    overlay_handle: Option<Res<ScopeMaskOverlayHandle>>,
    existing: Query<(), With<ScopeMaskOverlayNode>>,
    graded_q: Query<Entity, (With<GradedImageNode>, With<ImageNode>)>,
) {
    if !existing.is_empty() {
        return;
    }
    let Some(handle) = overlay_handle else {
        return;
    };
    for entity in graded_q.iter() {
        commands.entity(entity).with_children(|parent| {
            spawn_mask_overlay(parent, handle.handle.clone());
        });
    }
}

// ── UI spawning helpers (called from vectorscope.rs and viewer.rs) ──

/// Spawn mask toggle and clear buttons. Called from `spawn_scope_header`.
pub fn spawn_mask_buttons(parent: &mut ChildSpawnerCommands) {
    // Mask toggle button
    parent
        .spawn((
            ScopeMaskButton,
            Button,
            Node {
                display: Display::Flex,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                height: Val::Px(24.0),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(theme::BG_CONTROL),
            BorderColor::all(theme::BORDER_SUBTLE),
        ))
        .with_children(|button| {
            button.spawn((
                ScopeMaskButtonLabel,
                Text::new("Mask"),
                TextFont {
                    font_size: theme::FONT_SIZE_LABEL,
                    ..default()
                },
                TextColor(theme::TEXT_DIM),
                Pickable::IGNORE,
            ));
        });

    // Clear button
    parent
        .spawn((
            ScopeMaskClearButton,
            Button,
            Node {
                display: Display::None,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                height: Val::Px(24.0),
                padding: UiRect::axes(Val::Px(6.0), Val::Px(0.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(theme::BG_CONTROL),
            BorderColor::all(theme::BORDER_SUBTLE),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new("Clear"),
                TextFont {
                    font_size: theme::FONT_SIZE_LABEL,
                    ..default()
                },
                TextColor(theme::TEXT_DIM),
                Pickable::IGNORE,
            ));
        });
}

/// Spawn the mask overlay node inside the viewer frame.
pub fn spawn_mask_overlay(parent: &mut ChildSpawnerCommands, handle: Handle<Image>) {
    parent.spawn((
        ScopeMaskOverlayNode,
        ImageNode::new(handle).with_mode(NodeImageMode::Stretch),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        // Start hidden; becomes visible once a mask polygon exists.
        Visibility::Hidden,
        Pickable::IGNORE,
    ));
}

// ── Pointer interaction ─────────────────────────────────────────────

/// Convert pointer screen position to normalized 0..1 UV within the image node.
fn pointer_to_image_uv(
    pointer_pos: Vec2,
    node: &ComputedNode,
    node_target: &ComputedUiRenderTargetInfo,
    transform: &UiGlobalTransform,
    ui_scale: f32,
) -> Vec2 {
    let local_pos = transform
        .try_inverse()
        .unwrap()
        .transform_point2(pointer_pos * node_target.scale_factor() / ui_scale);
    let pos = local_pos / node.size() + Vec2::splat(0.5);
    pos.clamp(Vec2::ZERO, Vec2::ONE)
}

/// Minimum squared distance between polygon points (in normalized space).
const MIN_POINT_DIST_SQ: f32 = 0.0001; // ~1% of image dimension

fn on_viewer_press(
    mut press: On<Pointer<Press>>,
    state: Res<ScopeMaskState>,
    q_graded: Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
        ),
        With<GradedImageNode>,
    >,
) {
    if !state.tool_active {
        return;
    }

    // Only handle if the press target is the graded image node.
    let target = press.event_target();
    if q_graded.get(target).is_ok() {
        press.propagate(false);
    }
}

fn on_viewer_drag_start(
    mut drag_start: On<Pointer<DragStart>>,
    mut state: ResMut<ScopeMaskState>,
    q_graded: Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
        ),
        With<GradedImageNode>,
    >,
    ui_scale: Res<UiScale>,
) {
    if !state.tool_active {
        return;
    }

    let target = drag_start.event_target();
    let Ok((node, node_target, transform)) = q_graded.get(target) else {
        return;
    };

    drag_start.propagate(false);

    let uv = pointer_to_image_uv(
        drag_start.pointer_location.position,
        node,
        node_target,
        transform,
        ui_scale.0,
    );

    state.polygon.clear();
    state.polygon.push(uv);
    state.drawing = true;
    state.mask_dirty = true;
}

fn on_viewer_drag(
    mut drag: On<Pointer<Drag>>,
    mut state: ResMut<ScopeMaskState>,
    q_graded: Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
        ),
        With<GradedImageNode>,
    >,
    ui_scale: Res<UiScale>,
) {
    if !state.drawing {
        return;
    }

    let target = drag.event_target();
    let Ok((node, node_target, transform)) = q_graded.get(target) else {
        return;
    };

    drag.propagate(false);

    let uv = pointer_to_image_uv(
        drag.pointer_location.position,
        node,
        node_target,
        transform,
        ui_scale.0,
    );

    // Only add point if far enough from the last one.
    if let Some(last) = state.polygon.last() {
        if (*last - uv).length_squared() < MIN_POINT_DIST_SQ {
            return;
        }
    }

    state.polygon.push(uv);
    state.mask_dirty = true;
}

fn on_viewer_drag_end(
    mut drag_end: On<Pointer<DragEnd>>,
    mut state: ResMut<ScopeMaskState>,
    q_graded: Query<Entity, With<GradedImageNode>>,
) {
    if !state.drawing {
        return;
    }

    let target = drag_end.event_target();
    if q_graded.get(target).is_ok() {
        drag_end.propagate(false);
    }

    state.drawing = false;
    // Final dirty to render the closed polygon.
    state.mask_dirty = true;
}

// ── Mask rasterization ──────────────────────────────────────────────

/// Rasterize a normalized polygon into a per-pixel mask using scanline fill (even-odd rule).
fn rasterize_polygon(polygon: &[Vec2], width: u32, height: u32) -> Vec<u32> {
    let pixel_count = (width as usize) * (height as usize);
    let mut mask = vec![0u32; pixel_count];

    if polygon.len() < 3 {
        return mask;
    }

    for y in 0..height {
        let py = (y as f32 + 0.5) / height as f32;

        // Find x-intersections of the scanline with polygon edges.
        let mut intersections = Vec::new();
        let n = polygon.len();
        for i in 0..n {
            let j = (i + 1) % n;
            let (y0, y1) = (polygon[i].y, polygon[j].y);

            let (min_y, max_y) = if y0 < y1 { (y0, y1) } else { (y1, y0) };
            if py < min_y || py >= max_y {
                continue;
            }

            let t = (py - y0) / (y1 - y0);
            let x = polygon[i].x + t * (polygon[j].x - polygon[i].x);
            intersections.push(x);
        }

        intersections.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Fill between pairs of intersections (even-odd rule).
        for pair in intersections.chunks_exact(2) {
            let x_start = ((pair[0] * width as f32).max(0.0) as u32).min(width);
            let x_end = ((pair[1] * width as f32).ceil().max(0.0) as u32).min(width);
            let row_offset = (y * width) as usize;
            for x in x_start..x_end {
                mask[row_offset + x as usize] = 1;
            }
        }
    }

    mask
}

// ── Update systems ──────────────────────────────────────────────────

/// When the mask polygon changes, rasterize it and push to the GPU mask resource.
fn update_scope_mask(
    mut state: ResMut<ScopeMaskState>,
    image_state: Res<ImageState>,
    mut mask_data: ResMut<ScopeMaskData>,
    mut grading_state: ResMut<GradingState>,
) {
    if !state.mask_dirty {
        return;
    }
    state.mask_dirty = false;

    let Some(ref source) = image_state.source else {
        return;
    };

    let has_polygon = state.polygon.len() >= 3;

    if has_polygon {
        let mask = rasterize_polygon(&state.polygon, source.width, source.height);
        mask_data.mask = mask;
        mask_data.active = true;
    } else {
        mask_data.mask.clear();
        mask_data.active = false;
    }

    mask_data.dirty = true;
    grading_state.dirty = true;
}

/// Render the mask overlay texture and update the Bevy image asset.
#[allow(clippy::type_complexity)]
fn update_mask_overlay_texture(
    state: Res<ScopeMaskState>,
    image_state: Res<ImageState>,
    overlay_handle: Option<Res<ScopeMaskOverlayHandle>>,
    mut images: ResMut<Assets<Image>>,
    mut overlay_vis: Query<&mut Visibility, With<ScopeMaskOverlayNode>>,
) {
    if !state.is_changed() {
        return;
    }

    let Some(overlay_handle) = overlay_handle else {
        return;
    };

    let has_polygon = state.polygon.len() >= 3;
    let has_tool = state.tool_active;

    // Show/hide the overlay.
    for mut vis in &mut overlay_vis {
        *vis = if has_polygon || (has_tool && state.polygon.len() >= 2) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    if state.polygon.len() < 2 {
        return;
    }

    let Some(ref source) = image_state.source else {
        return;
    };

    // Cap overlay resolution for performance.
    let max_dim = 1024u32;
    let scale = if source.width > max_dim || source.height > max_dim {
        max_dim as f32 / source.width.max(source.height) as f32
    } else {
        1.0
    };
    let ov_w = ((source.width as f32 * scale) as u32).max(1);
    let ov_h = ((source.height as f32 * scale) as u32).max(1);

    let rgba = render_mask_overlay(&state.polygon, ov_w, ov_h);

    let new_size = Extent3d {
        width: ov_w,
        height: ov_h,
        depth_or_array_layers: 1,
    };

    if let Some(existing) = images.get_mut(&overlay_handle.handle) {
        if existing.texture_descriptor.size != new_size
            || existing.texture_descriptor.format != TextureFormat::Rgba8UnormSrgb
        {
            *existing = Image::new(
                new_size,
                TextureDimension::D2,
                rgba,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            );
        } else {
            existing.data = Some(rgba);
        }
    }
}

/// Render a mask overlay RGBA image: transparent inside, semi-dark outside, cyan outline.
fn render_mask_overlay(polygon: &[Vec2], width: u32, height: u32) -> Vec<u8> {
    let pixel_count = (width as usize) * (height as usize);
    let mut rgba = vec![0u8; pixel_count * 4];

    // Build the inside mask via scanline fill.
    let inside = if polygon.len() >= 3 {
        rasterize_polygon(polygon, width, height)
    } else {
        vec![0u32; pixel_count]
    };

    // Fill: outside gets a semi-transparent dark overlay, inside is transparent.
    for i in 0..pixel_count {
        let base = i * 4;
        if inside[i] == 0 {
            rgba[base] = 0;
            rgba[base + 1] = 0;
            rgba[base + 2] = 0;
            rgba[base + 3] = 100; // ~39% opacity
        }
        // else: fully transparent (rgba remains 0,0,0,0)
    }

    // Draw the polygon outline in cyan.
    let n = polygon.len();
    if n >= 2 {
        let segments = if n >= 3 { n } else { n - 1 };
        for i in 0..segments {
            let j = (i + 1) % n;
            draw_overlay_line(
                &mut rgba,
                width,
                height,
                polygon[i],
                polygon[j],
                [0, 200, 255, 220],
            );
        }
    }

    rgba
}

/// Draw an anti-aliased line segment onto an RGBA overlay.
fn draw_overlay_line(rgba: &mut [u8], width: u32, height: u32, a: Vec2, b: Vec2, color: [u8; 4]) {
    let ax = a.x * width as f32;
    let ay = a.y * height as f32;
    let bx = b.x * width as f32;
    let by = b.y * height as f32;

    let dx = (bx - ax).abs();
    let dy = (by - ay).abs();
    let step_count = (dx.max(dy) as u32).max(1);

    for i in 0..=step_count {
        let t = i as f32 / step_count as f32;
        let px = ax + (bx - ax) * t;
        let py = ay + (by - ay) * t;

        let ix = px.round() as i32;
        let iy = py.round() as i32;

        // 3px wide line for visibility.
        for oy in -1..=1i32 {
            for ox in -1..=1i32 {
                let x = ix + ox;
                let y = iy + oy;
                if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
                    let idx = (y as u32 * width + x as u32) as usize * 4;
                    // Alpha-blend the line color over the current pixel.
                    let ca = color[3] as f32 / 255.0;
                    let inv_a = 1.0 - ca;
                    rgba[idx] = (color[0] as f32 * ca + rgba[idx] as f32 * inv_a) as u8;
                    rgba[idx + 1] = (color[1] as f32 * ca + rgba[idx + 1] as f32 * inv_a) as u8;
                    rgba[idx + 2] = (color[2] as f32 * ca + rgba[idx + 2] as f32 * inv_a) as u8;
                    rgba[idx + 3] = rgba[idx + 3].max(color[3]);
                }
            }
        }
    }
}

// ── Button interaction ──────────────────────────────────────────────

#[allow(clippy::type_complexity)]
fn handle_mask_button_interactions(
    toggle_q: Query<&Interaction, (Changed<Interaction>, With<ScopeMaskButton>)>,
    clear_q: Query<&Interaction, (Changed<Interaction>, With<ScopeMaskClearButton>)>,
    mut state: ResMut<ScopeMaskState>,
    mut mask_data: ResMut<ScopeMaskData>,
    mut grading_state: ResMut<GradingState>,
) {
    for interaction in toggle_q.iter() {
        if *interaction == Interaction::Pressed {
            state.tool_active = !state.tool_active;
            if !state.tool_active && state.polygon.is_empty() {
                // Tool deactivated with no polygon — nothing to do.
            }
        }
    }

    for interaction in clear_q.iter() {
        if *interaction == Interaction::Pressed {
            clear_mask(&mut state, &mut mask_data, &mut grading_state);
        }
    }
}

fn clear_mask(
    state: &mut ScopeMaskState,
    mask_data: &mut ScopeMaskData,
    grading_state: &mut GradingState,
) {
    state.polygon.clear();
    state.drawing = false;
    state.mask_dirty = true;
    mask_data.mask.clear();
    mask_data.active = false;
    mask_data.dirty = true;
    grading_state.dirty = true;
}

fn handle_mask_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<ScopeMaskState>,
    mut mask_data: ResMut<ScopeMaskData>,
    mut grading_state: ResMut<GradingState>,
) {
    // Escape clears the mask when the tool is active.
    if keys.just_pressed(KeyCode::Escape) && state.tool_active && !state.polygon.is_empty() {
        clear_mask(&mut state, &mut mask_data, &mut grading_state);
    }
}

/// Keep the mask button appearance in sync with state.
#[allow(clippy::type_complexity)]
fn sync_mask_button_visuals(
    state: Res<ScopeMaskState>,
    mut toggle_q: Query<&mut BackgroundColor, With<ScopeMaskButton>>,
    mut label_q: Query<&mut TextColor, With<ScopeMaskButtonLabel>>,
    mut clear_q: Query<&mut Node, With<ScopeMaskClearButton>>,
) {
    if !state.is_changed() {
        return;
    }

    let active_bg = Color::srgb(0.20, 0.35, 0.50);
    let normal_bg = theme::BG_CONTROL;

    for mut bg in &mut toggle_q {
        *bg = if state.tool_active {
            BackgroundColor(active_bg)
        } else {
            BackgroundColor(normal_bg)
        };
    }

    for mut color in &mut label_q {
        *color = if state.tool_active {
            TextColor(theme::TEXT_PRIMARY)
        } else {
            TextColor(theme::TEXT_DIM)
        };
    }

    // Show/hide clear button based on whether a polygon exists.
    let show_clear = !state.polygon.is_empty();
    for mut node in &mut clear_q {
        node.display = if show_clear {
            Display::Flex
        } else {
            Display::None
        };
    }
}
