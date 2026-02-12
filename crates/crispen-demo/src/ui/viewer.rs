//! Image viewer panel (graded output display).
//!
//! Displays the graded image as a Bevy `ImageNode` that fills the top
//! portion of the window. The texture is updated each frame the
//! `ViewerData` resource changes.

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

/// Create a 1x1 transparent placeholder image and store the handle.
pub fn setup_viewer(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let placeholder = Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0, 0, 0, 0, 0], // 8 bytes for Rgba16Float
        TextureFormat::Rgba16Float,
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

/// When `ViewerData` changes, write the raw pixel bytes directly into the
/// Bevy `Image` asset. No CPU conversion — data is already f16 or f32
/// linear-light, and Bevy's renderer handles gamma during compositing.
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

    // Keep the viewer transform's aspect ratio in sync with the loaded image.
    let ar = viewer_data.width as f32 / viewer_data.height as f32;
    if transform.image_aspect_ratio != Some(ar) {
        transform.image_aspect_ratio = Some(ar);
    }
    let Some(viewer) = viewer else { return };

    // Hide the load hint once we have an image.
    for entity in hints.iter() {
        commands.entity(entity).despawn();
    }

    let texture_format = match viewer_data.format {
        ViewerFormat::F16 => TextureFormat::Rgba16Float,
        ViewerFormat::F32 => TextureFormat::Rgba32Float,
    };

    if let Some(existing) = images.get_mut(&viewer.handle) {
        let new_size = Extent3d {
            width: viewer_data.width,
            height: viewer_data.height,
            depth_or_array_layers: 1,
        };

        if existing.texture_descriptor.size != new_size
            || existing.texture_descriptor.format != texture_format
        {
            *existing = Image::new(
                new_size,
                TextureDimension::D2,
                viewer_data.pixel_bytes.clone(),
                texture_format,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            );
        } else {
            existing.data = Some(viewer_data.pixel_bytes.clone());
        }
    } else {
        tracing::warn!("viewer Image asset not found for handle");
    }
}
