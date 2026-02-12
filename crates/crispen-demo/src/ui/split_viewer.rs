//! Split-view viewer area for source vs graded comparison.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use crispen_bevy::resources::ImageState;

use super::theme;
use super::toolbar::ToolbarState;
use super::viewer;
use super::viewer_nav::{PICKABLE_IGNORE, ViewerFrame, ViewerImageWrapper};

/// Handle to the source-image texture used by the split viewer.
#[derive(Resource)]
pub struct SourceImageHandle {
    pub handle: Handle<Image>,
}

/// Root node for the viewer row (source + divider + graded).
#[derive(Component)]
pub struct ViewerContainer;

/// Source-half wrapper node.
#[derive(Component)]
pub struct SourceImageNode;

/// Graded-half wrapper node.
#[derive(Component)]
pub struct GradedImageNode;

/// Vertical divider between source and graded halves.
#[derive(Component)]
pub struct SplitDivider;

/// Allocate a 1x1 `Rgba32Float` placeholder texture for the source preview.
pub fn setup_source_image(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let placeholder = Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], // 16 bytes for Rgba32Float
        TextureFormat::Rgba32Float,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let handle = images.add(placeholder);
    commands.insert_resource(SourceImageHandle {
        handle: handle.clone(),
    });
}

/// Spawn viewer content area with hidden source panel and graded panel.
pub fn spawn_viewer_area(
    parent: &mut ChildSpawnerCommands,
    graded_handle: Handle<Image>,
    source_handle: Handle<Image>,
) {
    parent
        .spawn((
            ViewerContainer,
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                flex_grow: 1.0,
                width: Val::Percent(100.0),
                min_height: Val::Px(200.0),
                ..default()
            },
            BackgroundColor(theme::BG_DARK),
        ))
        .with_children(|row| {
            row.spawn((
                SourceImageNode,
                Node {
                    display: Display::None,
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(50.0),
                    flex_shrink: 0.0,
                    ..default()
                },
                BackgroundColor(theme::BG_DARK),
            ))
            .with_children(|source_panel| {
                source_panel
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
                    .with_children(|viewer_root| {
                        viewer_root
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
                                            ImageNode::new(source_handle)
                                                .with_mode(NodeImageMode::Stretch),
                                            Node {
                                                width: Val::Percent(100.0),
                                                height: Val::Percent(100.0),
                                                ..default()
                                            },
                                        ));
                                    });

                                frame.spawn((
                                    Text::new("Source"),
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
                            });
                    });
            });

            row.spawn((
                SplitDivider,
                Node {
                    display: Display::None,
                    width: Val::Px(2.0),
                    margin: UiRect::axes(Val::Px(0.0), Val::Px(12.0)),
                    ..default()
                },
                BackgroundColor(theme::BORDER_SUBTLE),
            ));

            row.spawn((
                GradedImageNode,
                Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    min_width: Val::Px(0.0),
                    ..default()
                },
            ))
            .with_children(|graded_panel| {
                viewer::spawn_viewer_panel(graded_panel, graded_handle.clone());
            });
        });
}

/// Toggle source/divider visibility and graded-half width for split mode.
#[allow(clippy::type_complexity)]
pub fn toggle_split_view(
    toolbar_state: Res<ToolbarState>,
    mut ui_parts: ParamSet<(
        Query<&mut Node, With<SourceImageNode>>,
        Query<&mut Node, With<SplitDivider>>,
        Query<&mut Node, (With<GradedImageNode>, Without<ImageNode>)>,
    )>,
) {
    if !toolbar_state.is_changed() {
        return;
    }

    for mut source in &mut ui_parts.p0() {
        source.display = if toolbar_state.split_view_active {
            Display::Flex
        } else {
            Display::None
        };
    }

    for mut divider in &mut ui_parts.p1() {
        divider.display = if toolbar_state.split_view_active {
            Display::Flex
        } else {
            Display::None
        };
    }

    for mut graded in &mut ui_parts.p2() {
        if toolbar_state.split_view_active {
            graded.width = Val::Percent(50.0);
            graded.flex_grow = 0.0;
        } else {
            graded.width = Val::Percent(100.0);
            graded.flex_grow = 1.0;
        }
    }
}

/// Upload the current source image into the split-view source texture.
pub fn update_source_texture(
    image_state: Res<ImageState>,
    source_image: Option<Res<SourceImageHandle>>,
    mut images: ResMut<Assets<Image>>,
) {
    if !image_state.is_changed() {
        return;
    }

    let Some(source) = image_state.source.as_ref() else {
        return;
    };
    let Some(source_image) = source_image else {
        return;
    };

    let new_size = Extent3d {
        width: source.width,
        height: source.height,
        depth_or_array_layers: 1,
    };
    let bytes = bytemuck::cast_slice(source.pixels.as_slice()).to_vec();

    if let Some(existing) = images.get_mut(&source_image.handle) {
        if existing.texture_descriptor.size != new_size
            || existing.texture_descriptor.format != TextureFormat::Rgba32Float
        {
            *existing = Image::new(
                new_size,
                TextureDimension::D2,
                bytes,
                TextureFormat::Rgba32Float,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            );
        } else {
            existing.data = Some(bytes);
        }
    }
}
