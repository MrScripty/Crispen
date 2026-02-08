//! Image viewer panel (graded output display).
//!
//! Displays the graded image as a Bevy `ImageNode` that fills the top
//! portion of the window. The texture is updated each frame the
//! `ImageState.graded` resource changes.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use crispen_bevy::resources::ImageState;

use super::theme;

/// Handle to the dynamic Bevy `Image` asset used by the viewer.
#[derive(Resource)]
pub struct ViewerImageHandle {
    pub handle: Handle<Image>,
}

/// Create a 1x1 transparent placeholder image and store the handle.
pub fn setup_viewer(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
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
    commands.insert_resource(ViewerImageHandle {
        handle: handle.clone(),
    });
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
                    Node {
                        display: Display::Flex,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
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
                    frame.spawn((
                        ImageNode::new(handle).with_mode(NodeImageMode::Auto),
                        Node {
                            max_width: Val::Percent(100.0),
                            max_height: Val::Percent(100.0),
                            ..default()
                        },
                    ));

                    frame.spawn((
                        Text::new("Viewer"),
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
                });
        });
}

/// When `ImageState.graded` changes, re-encode pixels as sRGB u8 and
/// replace the Bevy `Image` asset so the viewer updates on screen.
pub fn update_viewer_texture(
    image_state: Res<ImageState>,
    viewer: Option<Res<ViewerImageHandle>>,
    mut images: ResMut<Assets<Image>>,
) {
    if !image_state.is_changed() {
        return;
    }
    let Some(viewer) = viewer else { return };
    let Some(graded) = &image_state.graded else {
        return;
    };

    let pixel_count = graded.pixels.len();
    let mut data = Vec::with_capacity(pixel_count * 4);
    for px in &graded.pixels {
        data.push(linear_to_srgb(px[0]));
        data.push(linear_to_srgb(px[1]));
        data.push(linear_to_srgb(px[2]));
        data.push((px[3].clamp(0.0, 1.0) * 255.0 + 0.5) as u8);
    }

    let new_image = Image::new(
        Extent3d {
            width: graded.width,
            height: graded.height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );

    if let Some(existing) = images.get_mut(&viewer.handle) {
        *existing = new_image;
    }
}

/// Convert a single linear-light channel value to sRGB gamma-encoded u8.
fn linear_to_srgb(c: f32) -> u8 {
    let s = if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (s.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}
