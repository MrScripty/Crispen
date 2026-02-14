//! Image viewer panel (graded output display).
//!
//! Displays the graded image as a Bevy `ImageNode` that fills the top
//! portion of the window. The texture is updated each frame the
//! `ViewerData` resource changes.
//!
//! The GPU pipeline produces linear-light `Rgba16Float` (or `Rgba32Float`)
//! data.  Bevy's UI image pipeline does not reliably render HDR float
//! textures through the `ImageNode` compositing path, so we convert to
//! `Rgba8UnormSrgb` on the CPU before uploading.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use crispen_bevy::ViewerFormat;
use crispen_bevy::resources::ViewerData;

use super::split_viewer::GradedImageNode;
use super::theme;
use super::viewer_nav::{PICKABLE_IGNORE, ViewerFrame, ViewerImageWrapper, ViewerTransform};

/// Marker for the "Ctrl+O to load" hint text, hidden once an image is loaded.
#[derive(Component)]
pub struct LoadHint;

/// Handle to the dynamic Bevy `Image` asset used by the viewer.
#[derive(Resource)]
pub struct ViewerImageHandle {
    pub handle: Handle<Image>,
}

/// Create a 1x1 black placeholder image and store the handle.
pub fn setup_viewer(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let placeholder = Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255], // opaque black, Rgba8UnormSrgb
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let handle = images.add(placeholder);
    commands.insert_resource(ViewerImageHandle {
        handle: handle.clone(),
    });
}

// ── f16 / linear-to-sRGB helpers ──────────────────────────────────

/// Decode an IEEE 754 half-precision float from two little-endian bytes.
fn f16_to_f32(lo: u8, hi: u8) -> f32 {
    let bits = u16::from_le_bytes([lo, hi]);
    let sign = ((bits >> 15) & 1) as u32;
    let exp = ((bits >> 10) & 0x1F) as u32;
    let mant = (bits & 0x3FF) as u32;

    if exp == 0 {
        if mant == 0 {
            return f32::from_bits(sign << 31); // ±0
        }
        // Subnormal — normalise.
        let mut m = mant;
        let mut e: i32 = -14;
        while (m & 0x400) == 0 {
            m <<= 1;
            e -= 1;
        }
        m &= 0x3FF;
        let f32_exp = (e + 127) as u32;
        return f32::from_bits((sign << 31) | (f32_exp << 23) | (m << 13));
    }
    if exp == 31 {
        if mant == 0 {
            return if sign == 1 {
                f32::NEG_INFINITY
            } else {
                f32::INFINITY
            };
        }
        return f32::NAN;
    }

    let f32_exp = (exp as i32 - 15 + 127) as u32;
    f32::from_bits((sign << 31) | (f32_exp << 23) | (mant << 13))
}

/// Convert a single linear-light channel value to an sRGB-encoded `u8`.
#[inline]
fn linear_to_srgb_u8(v: f32) -> u8 {
    let c = v.clamp(0.0, 1.0);
    let s = if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0 + 0.5) as u8
}

/// Convert an `Rgba16Float` byte buffer to `Rgba8UnormSrgb`.
fn f16_linear_to_srgb8(src: &[u8], pixel_count: usize) -> Vec<u8> {
    let mut dst = vec![0u8; pixel_count * 4];
    for i in 0..pixel_count {
        let si = i * 8; // 4 channels × 2 bytes
        let di = i * 4;
        let r = f16_to_f32(src[si], src[si + 1]);
        let g = f16_to_f32(src[si + 2], src[si + 3]);
        let b = f16_to_f32(src[si + 4], src[si + 5]);
        let a = f16_to_f32(src[si + 6], src[si + 7]);
        dst[di] = linear_to_srgb_u8(r);
        dst[di + 1] = linear_to_srgb_u8(g);
        dst[di + 2] = linear_to_srgb_u8(b);
        dst[di + 3] = (a.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    }
    dst
}

/// Convert an `Rgba32Float` byte buffer to `Rgba8UnormSrgb`.
fn f32_linear_to_srgb8(src: &[u8], pixel_count: usize) -> Vec<u8> {
    let mut dst = vec![0u8; pixel_count * 4];
    for i in 0..pixel_count {
        let si = i * 16; // 4 channels × 4 bytes
        let di = i * 4;
        let r = f32::from_le_bytes([src[si], src[si + 1], src[si + 2], src[si + 3]]);
        let g = f32::from_le_bytes([src[si + 4], src[si + 5], src[si + 6], src[si + 7]]);
        let b = f32::from_le_bytes([src[si + 8], src[si + 9], src[si + 10], src[si + 11]]);
        let a = f32::from_le_bytes([src[si + 12], src[si + 13], src[si + 14], src[si + 15]]);
        dst[di] = linear_to_srgb_u8(r);
        dst[di + 1] = linear_to_srgb_u8(g);
        dst[di + 2] = linear_to_srgb_u8(b);
        dst[di + 3] = (a.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    }
    dst
}

/// Spawn the top viewer section inside the given parent.
///
/// The panel includes a framed viewport area with the dynamic image node.
pub fn spawn_viewer_panel(parent: &mut ChildSpawnerCommands, handle: Handle<Image>) {
    parent
        .spawn((
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                width: Val::Percent(100.0),
                min_height: Val::Px(200.0),
                padding: UiRect::all(Val::Px(12.0)),
                ..default()
            },
            BackgroundColor(theme::BG_DARK),
        ))
        .with_children(|viewer| {
            viewer
                .spawn((
                    ViewerFrame,
                    Node {
                        display: Display::Flex,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        overflow: Overflow::clip(),
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(theme::BG_VIEWER),
                    BorderColor::all(theme::BORDER_SUBTLE),
                ))
                .with_children(|frame| {
                    // Zoom/pan wrapper: absolutely positioned, sized by
                    // `apply_viewer_transform`.
                    frame
                        .spawn((
                            ViewerImageWrapper,
                            Node {
                                position_type: PositionType::Absolute,
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                        ))
                        .with_children(|wrapper| {
                            wrapper.spawn((
                                GradedImageNode,
                                ImageNode::new(handle).with_mode(NodeImageMode::Stretch),
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                            ));
                        });

                    frame.spawn((
                        Text::new("Viewer"),
                        PICKABLE_IGNORE,
                        Node {
                            position_type: PositionType::Absolute,
                            top: Val::Px(8.0),
                            left: Val::Px(10.0),
                            ..default()
                        },
                        TextFont {
                            font_size: theme::FONT_SIZE_LABEL,
                            ..default()
                        },
                        TextColor(theme::TEXT_DIM),
                    ));

                    frame.spawn((
                        LoadHint,
                        PICKABLE_IGNORE,
                        Text::new("Ctrl+O to load image"),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(theme::TEXT_DIM),
                    ));
                });
        });
}

/// Convert the GPU pipeline's linear-light pixel data to `Rgba8UnormSrgb`
/// and upload it to the Bevy `Image` asset referenced by the viewer
/// `ImageNode`.
pub fn update_viewer_texture(
    viewer_data: Res<ViewerData>,
    viewer: Option<Res<ViewerImageHandle>>,
    mut images: ResMut<Assets<Image>>,
    hints: Query<Entity, With<LoadHint>>,
    mut commands: Commands,
    mut transform: ResMut<ViewerTransform>,
) {
    if !viewer_data.is_changed() || viewer_data.width == 0 {
        return;
    }

    let t0 = std::time::Instant::now();

    let pixel_count = (viewer_data.width * viewer_data.height) as usize;

    // Keep the viewer transform's aspect ratio in sync with the loaded image.
    let ar = viewer_data.width as f32 / viewer_data.height as f32;
    if transform.image_aspect_ratio != Some(ar) {
        transform.image_aspect_ratio = Some(ar);
    }
    let Some(viewer) = viewer else {
        tracing::warn!("update_viewer_texture: ViewerImageHandle resource missing");
        return;
    };

    // Hide the load hint once we have an image.
    for entity in hints.iter() {
        commands.entity(entity).despawn();
    }

    let t_setup = t0.elapsed();

    // Convert to sRGB u8. Srgb8 is already GPU-converted — just copy bytes.
    let srgb_bytes = match viewer_data.format {
        ViewerFormat::Srgb8 => viewer_data.pixel_bytes.clone(),
        ViewerFormat::F16 => f16_linear_to_srgb8(&viewer_data.pixel_bytes, pixel_count),
        ViewerFormat::F32 => f32_linear_to_srgb8(&viewer_data.pixel_bytes, pixel_count),
    };

    let t_convert = t0.elapsed();

    if let Some(existing) = images.get_mut(&viewer.handle) {
        let new_size = Extent3d {
            width: viewer_data.width,
            height: viewer_data.height,
            depth_or_array_layers: 1,
        };

        if existing.texture_descriptor.size != new_size
            || existing.texture_descriptor.format != TextureFormat::Rgba8UnormSrgb
        {
            *existing = Image::new(
                new_size,
                TextureDimension::D2,
                srgb_bytes,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            );
        } else {
            existing.data = Some(srgb_bytes);
        }
    } else {
        tracing::warn!("viewer Image asset not found for handle");
    }

    let t_total = t0.elapsed();
    tracing::info!(
        "[PERF] update_viewer_texture: setup={:.2}ms convert={:.2}ms upload={:.2}ms total={:.2}ms ({}x{} {:?})",
        t_setup.as_secs_f64() * 1000.0,
        (t_convert - t_setup).as_secs_f64() * 1000.0,
        (t_total - t_convert).as_secs_f64() * 1000.0,
        t_total.as_secs_f64() * 1000.0,
        viewer_data.width,
        viewer_data.height,
        viewer_data.format,
    );
}
