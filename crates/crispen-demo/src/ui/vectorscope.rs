//! Scope panel renderer and selector UI.
//!
//! Supports vectorscope, waveform, RGB parade, and histogram display
//! modes in the bottom panel's Scopes section.

use bevy::asset::RenderAssetUsages;
use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use crispen_bevy::resources::ScopeState;
use crispen_core::scopes::{HistogramData, VectorscopeData, WaveformData};

use super::theme;

/// Handle to the dynamic Bevy image used for scope rendering.
#[derive(Resource)]
pub struct VectorscopeImageHandle {
    pub handle: Handle<Image>,
}

/// UI state for scope mode selection.
#[derive(Resource)]
pub struct ScopeViewState {
    pub mode: ScopeViewMode,
    pub dropdown_open: bool,
}

impl Default for ScopeViewState {
    fn default() -> Self {
        Self {
            mode: ScopeViewMode::Vectorscope,
            dropdown_open: false,
        }
    }
}

/// Supported scope display modes in the native UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeViewMode {
    Vectorscope,
    Waveform,
    RgbParade,
    Histogram,
}

impl ScopeViewMode {
    fn label(self) -> &'static str {
        match self {
            Self::Vectorscope => "Vectorscope",
            Self::Waveform => "Waveform",
            Self::RgbParade => "RGB Parade",
            Self::Histogram => "Histogram",
        }
    }

    fn missing_text(self) -> &'static str {
        match self {
            Self::Vectorscope => "No vectorscope data",
            Self::Waveform => "No waveform data",
            Self::RgbParade => "No waveform data for parade",
            Self::Histogram => "No histogram data",
        }
    }

    fn is_square(self) -> bool {
        matches!(self, Self::Vectorscope)
    }
}

/// Marker for scope-image frame node.
#[derive(Component)]
pub(crate) struct ScopeImageFrame;

/// Marker for frame layout container around scope image.
#[derive(Component)]
pub(crate) struct ScopePlotArea;

/// Marker for "waiting for scope data" hint text.
#[derive(Component)]
pub(crate) struct ScopeHint;

/// Dropdown toggle button.
#[derive(Component)]
pub(crate) struct ScopeDropdownButton;

/// Dropdown option list container.
#[derive(Component)]
pub(crate) struct ScopeDropdownMenu;

/// Text node that reflects the selected scope mode.
#[derive(Component)]
pub(crate) struct ScopeDropdownLabel;

/// Dropdown menu option for a specific scope mode.
#[derive(Component, Clone, Copy)]
pub(crate) struct ScopeDropdownOption(pub ScopeViewMode);

/// Allocate a placeholder scope image and store the handle.
pub fn setup_vectorscope(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let placeholder = Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[3, 3, 3, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let handle = images.add(placeholder);
    commands.insert_resource(VectorscopeImageHandle {
        handle: handle.clone(),
    });
}

/// Spawn the scopes header with a dropdown selector.
pub fn spawn_scope_header(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            width: Val::Percent(100.0),
            ..default()
        })
        .with_children(|row| {
            row.spawn((
                Text::new("Scopes"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(theme::TEXT_PRIMARY),
            ));

            row.spawn(Node {
                position_type: PositionType::Relative,
                width: Val::Px(136.0),
                ..default()
            })
            .with_children(|dropdown| {
                dropdown
                    .spawn((
                        ScopeDropdownButton,
                        Button,
                        Node {
                            display: Display::Flex,
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceBetween,
                            align_items: AlignItems::Center,
                            width: Val::Percent(100.0),
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
                            ScopeDropdownLabel,
                            Text::new(ScopeViewMode::Vectorscope.label()),
                            TextFont {
                                font_size: theme::FONT_SIZE_LABEL,
                                ..default()
                            },
                            TextColor(theme::TEXT_PRIMARY),
                        ));
                        button.spawn((
                            Text::new("v"),
                            TextFont {
                                font_size: 10.0,
                                ..default()
                            },
                            TextColor(theme::TEXT_DIM),
                        ));
                    });

                dropdown
                    .spawn((
                        ScopeDropdownMenu,
                        Node {
                            position_type: PositionType::Absolute,
                            top: Val::Px(24.0),
                            left: Val::Px(0.0),
                            width: Val::Percent(100.0),
                            display: Display::None,
                            flex_direction: FlexDirection::Column,
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(theme::BG_CONTROL),
                        BorderColor::all(theme::BORDER_SUBTLE),
                        GlobalZIndex(100),
                        ZIndex(10),
                    ))
                    .with_children(|menu| {
                        for mode in [
                            ScopeViewMode::Vectorscope,
                            ScopeViewMode::Waveform,
                            ScopeViewMode::RgbParade,
                            ScopeViewMode::Histogram,
                        ] {
                            menu.spawn((
                                ScopeDropdownOption(mode),
                                Button,
                                Node {
                                    width: Val::Percent(100.0),
                                    padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                                    border: UiRect::bottom(Val::Px(1.0)),
                                    ..default()
                                },
                                BackgroundColor(theme::BG_CONTROL),
                                BorderColor::all(theme::BORDER_SUBTLE),
                                children![(
                                    Text::new(mode.label()),
                                    TextFont {
                                        font_size: theme::FONT_SIZE_LABEL,
                                        ..default()
                                    },
                                    TextColor(theme::TEXT_PRIMARY),
                                )],
                            ));
                        }
                    });
            });
        });
}

/// Spawn the scope panel in the Scopes section.
pub fn spawn_vectorscope_panel(parent: &mut ChildSpawnerCommands, handle: Handle<Image>) {
    parent
        .spawn((
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Pickable::IGNORE,
        ))
        .with_children(|scope| {
            scope
                .spawn((
                    ScopePlotArea,
                    Node {
                        display: Display::Flex,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        min_height: Val::Px(0.0),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|plot| {
                    plot.spawn((
                        ScopeImageFrame,
                        Node {
                            display: Display::Flex,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            width: Val::Auto,
                            height: Val::Percent(100.0),
                            max_width: Val::Percent(100.0),
                            aspect_ratio: Some(1.0),
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(4.0)),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(theme::BG_CONTROL),
                        BorderColor::all(theme::BORDER_SUBTLE),
                        Pickable::IGNORE,
                    ))
                    .with_children(|frame| {
                        frame.spawn((
                            ImageNode::new(handle).with_mode(NodeImageMode::Stretch),
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ));

                        frame.spawn((
                            ScopeHint,
                            Node {
                                position_type: PositionType::Absolute,
                                ..default()
                            },
                            Text::new("Waiting for image"),
                            TextFont {
                                font_size: theme::FONT_SIZE_LABEL,
                                ..default()
                            },
                            TextColor(theme::TEXT_DIM),
                            Pickable::IGNORE,
                        ));
                    });
                });
        });
}

/// Handle interactions for the scope dropdown toggle and options.
pub fn handle_scope_dropdown_interactions(
    button_interactions: Query<&Interaction, (Changed<Interaction>, With<ScopeDropdownButton>)>,
    option_interactions: Query<
        (&Interaction, &ScopeDropdownOption),
        (Changed<Interaction>, Without<ScopeDropdownButton>),
    >,
    mut state: ResMut<ScopeViewState>,
) {
    for interaction in button_interactions.iter() {
        if *interaction == Interaction::Pressed {
            state.dropdown_open = !state.dropdown_open;
        }
    }

    for (interaction, option) in option_interactions.iter() {
        if *interaction == Interaction::Pressed {
            state.mode = option.0;
            state.dropdown_open = false;
        }
    }
}

/// Keep dropdown label/menu visuals in sync with [`ScopeViewState`].
pub fn sync_scope_dropdown_ui(
    state: Res<ScopeViewState>,
    mut ui_parts: ParamSet<(
        Query<&mut Text, With<ScopeDropdownLabel>>,
        Query<&mut Node, With<ScopeDropdownMenu>>,
        Query<(&ScopeDropdownOption, &mut BackgroundColor)>,
    )>,
) {
    if !state.is_changed() {
        return;
    }

    for mut label in ui_parts.p0().iter_mut() {
        *label = Text::new(state.mode.label());
    }

    for mut menu in ui_parts.p1().iter_mut() {
        menu.display = if state.dropdown_open {
            Display::Flex
        } else {
            Display::None
        };
    }

    for (option, mut bg) in ui_parts.p2().iter_mut() {
        *bg = if option.0 == state.mode {
            BackgroundColor(Color::srgb(0.29, 0.29, 0.29))
        } else {
            BackgroundColor(theme::BG_CONTROL)
        };
    }
}

/// Render the selected scope mode into the shared scope image.
pub fn update_scope_texture(
    scope_state: Res<ScopeState>,
    view_state: Res<ScopeViewState>,
    scope_image: Option<Res<VectorscopeImageHandle>>,
    mut images: ResMut<Assets<Image>>,
    mut ui_parts: ParamSet<(
        Query<&mut Node, With<ScopePlotArea>>,
        Query<&mut Node, With<ScopeImageFrame>>,
        Query<(&mut Node, &mut Text), With<ScopeHint>>,
    )>,
) {
    if !(scope_state.is_changed() || view_state.is_changed()) {
        return;
    }

    for mut area in ui_parts.p0().iter_mut() {
        area.align_items = if view_state.mode.is_square() {
            AlignItems::Center
        } else {
            AlignItems::Stretch
        };
    }

    for mut frame in ui_parts.p1().iter_mut() {
        if view_state.mode.is_square() {
            frame.width = Val::Auto;
            frame.height = Val::Percent(100.0);
            frame.max_width = Val::Percent(100.0);
            frame.aspect_ratio = Some(1.0);
        } else {
            frame.width = Val::Percent(100.0);
            frame.height = Val::Percent(100.0);
            frame.max_width = Val::Auto;
            frame.aspect_ratio = None;
        }
    }

    let Some(scope_image) = scope_image else {
        return;
    };

    let rendered = match view_state.mode {
        ScopeViewMode::Vectorscope => scope_state
            .vectorscope
            .as_ref()
            .and_then(render_vectorscope),
        ScopeViewMode::Waveform => scope_state.waveform.as_ref().and_then(render_waveform),
        ScopeViewMode::RgbParade => scope_state.waveform.as_ref().and_then(render_parade),
        ScopeViewMode::Histogram => scope_state.histogram.as_ref().and_then(render_histogram),
    };

    if let Some((w, h, rgba)) = rendered {
        upload_scope_texture(scope_image.handle.clone(), &mut images, w, h, rgba);
        for (mut node, _) in ui_parts.p2().iter_mut() {
            node.display = Display::None;
        }
    } else {
        for (mut node, mut text) in ui_parts.p2().iter_mut() {
            node.display = Display::Flex;
            *text = Text::new(view_state.mode.missing_text());
        }
    }
}

fn upload_scope_texture(
    handle: Handle<Image>,
    images: &mut Assets<Image>,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
) {
    if let Some(existing) = images.get_mut(&handle) {
        let new_size = Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

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

fn render_vectorscope(data: &VectorscopeData) -> Option<(u32, u32, Vec<u8>)> {
    let resolution = data.resolution.max(1);
    let pixel_count = (resolution as usize).saturating_mul(resolution as usize);
    if data.density.len() < pixel_count {
        return None;
    }

    let peak = data.density.iter().copied().max().unwrap_or(0) as f32;
    let log_peak = (peak + 1.0).ln().max(1.0);

    let radius = resolution as f32 * 0.5;
    let center = (resolution as f32 - 1.0) * 0.5;
    let line = 1.5 / radius.max(1.0);
    let skin_angle = 33.0_f32.to_radians();
    let skin_dir = Vec2::new(skin_angle.cos(), -skin_angle.sin());

    let mut rgba = vec![0u8; pixel_count * 4];
    for y in 0..resolution {
        for x in 0..resolution {
            let idx = (y * resolution + x) as usize;
            let d = data.density[idx] as f32;

            let nx = (x as f32 - center) / radius;
            let ny = (y as f32 - center) / radius;
            let dist = (nx * nx + ny * ny).sqrt();
            let inside = dist <= 1.0;

            let mut r = 0.02;
            let mut g = 0.02;
            let mut b = 0.024;

            if inside {
                let falloff = (1.0 - dist).clamp(0.0, 1.0);
                r = 0.05 + falloff * 0.02;
                g = 0.05 + falloff * 0.02;
                b = 0.06 + falloff * 0.03;

                let rings = [0.25, 0.5, 0.75, 1.0];
                if rings.iter().any(|ring| (dist - ring).abs() <= line) {
                    r += 0.07;
                    g += 0.07;
                    b += 0.07;
                }

                if nx.abs() <= line || ny.abs() <= line {
                    r += 0.05;
                    g += 0.05;
                    b += 0.05;
                }

                let skin_line = (nx * skin_dir.y - ny * skin_dir.x).abs() <= line && dist <= 1.0;
                if skin_line {
                    r += 0.08;
                    g += 0.05;
                    b += 0.03;
                }

                if d > 0.0 {
                    let signal = ((d + 1.0).ln() / log_peak).clamp(0.0, 1.0).powf(0.65);
                    r += signal * 0.42;
                    g += signal * 0.90;
                    b += signal * 0.52;
                }
            }

            let base = idx * 4;
            rgba[base] = (r.clamp(0.0, 1.0) * 255.0) as u8;
            rgba[base + 1] = (g.clamp(0.0, 1.0) * 255.0) as u8;
            rgba[base + 2] = (b.clamp(0.0, 1.0) * 255.0) as u8;
            rgba[base + 3] = 255;
        }
    }

    Some((resolution, resolution, rgba))
}

fn render_waveform(data: &WaveformData) -> Option<(u32, u32, Vec<u8>)> {
    if data.width == 0 || data.height == 0 {
        return None;
    }
    let src_total = (data.width * data.height) as usize;
    if data.data.iter().any(|ch| ch.len() < src_total) {
        return None;
    }

    // Downsample horizontally for stable scope display and cleaner traces.
    let out_width = data.width.clamp(256, 768);
    let out_height = data.height;
    let out_total = (out_width * out_height) as usize;

    let mut accum = [
        vec![0.0f32; out_total],
        vec![0.0f32; out_total],
        vec![0.0f32; out_total],
    ];

    for y in 0..data.height {
        for x in 0..data.width {
            let dst_x = (x * out_width / data.width).min(out_width - 1);
            let src_idx = (y * data.width + x) as usize;
            let dst_idx = (y * out_width + dst_x) as usize;
            accum[0][dst_idx] += data.data[0][src_idx] as f32;
            accum[1][dst_idx] += data.data[1][src_idx] as f32;
            accum[2][dst_idx] += data.data[2][src_idx] as f32;
        }
    }

    let peak = accum
        .iter()
        .flat_map(|ch| ch.iter())
        .copied()
        .fold(0.0f32, f32::max);
    if peak <= 0.0 {
        return None;
    }

    let log_peak = (peak + 1.0).ln();
    let mut rgba = vec![0u8; out_total * 4];
    for idx in 0..out_total {
        let r = ((accum[0][idx] + 1.0).ln() / log_peak).clamp(0.0, 1.0);
        let g = ((accum[1][idx] + 1.0).ln() / log_peak).clamp(0.0, 1.0);
        let b = ((accum[2][idx] + 1.0).ln() / log_peak).clamp(0.0, 1.0);

        let base = idx * 4;
        rgba[base] = ((0.03 + r * 0.95).clamp(0.0, 1.0) * 255.0) as u8;
        rgba[base + 1] = ((0.03 + g * 0.95).clamp(0.0, 1.0) * 255.0) as u8;
        rgba[base + 2] = ((0.03 + b * 0.95).clamp(0.0, 1.0) * 255.0) as u8;
        rgba[base + 3] = 255;
    }

    Some((out_width, out_height, rgba))
}

fn render_parade(data: &WaveformData) -> Option<(u32, u32, Vec<u8>)> {
    if data.width == 0 || data.height == 0 {
        return None;
    }
    let src_total = (data.width * data.height) as usize;
    if data.data.iter().any(|ch| ch.len() < src_total) {
        return None;
    }

    let panel_width = data.width.clamp(192, 384);
    let height = data.height;
    let panel_total = (panel_width * height) as usize;

    let mut accum = [
        vec![0.0f32; panel_total],
        vec![0.0f32; panel_total],
        vec![0.0f32; panel_total],
    ];

    for y in 0..height {
        for x in 0..data.width {
            let dst_x = (x * panel_width / data.width).min(panel_width - 1);
            let src_idx = (y * data.width + x) as usize;
            let dst_idx = (y * panel_width + dst_x) as usize;
            accum[0][dst_idx] += data.data[0][src_idx] as f32;
            accum[1][dst_idx] += data.data[1][src_idx] as f32;
            accum[2][dst_idx] += data.data[2][src_idx] as f32;
        }
    }

    let peak = accum
        .iter()
        .flat_map(|ch| ch.iter())
        .copied()
        .fold(0.0f32, f32::max);
    if peak <= 0.0 {
        return None;
    }
    let log_peak = (peak + 1.0).ln();

    let width = panel_width * 3;
    let total = (width * height) as usize;
    let mut rgba = vec![0u8; total * 4];

    for ch in 0..3 {
        let x_offset = ch as u32 * panel_width;
        for y in 0..height {
            for x in 0..panel_width {
                let dst_idx = (y * width + x + x_offset) as usize;
                let src_idx = (y * panel_width + x) as usize;
                let signal = ((accum[ch][src_idx] + 1.0).ln() / log_peak).clamp(0.0, 1.0);

                let (r, g, b) = match ch {
                    0 => (signal, 0.0, 0.0),
                    1 => (0.0, signal, 0.0),
                    _ => (0.0, 0.0, signal),
                };

                let base = dst_idx * 4;
                rgba[base] = ((0.02 + r * 0.95).clamp(0.0, 1.0) * 255.0) as u8;
                rgba[base + 1] = ((0.02 + g * 0.95).clamp(0.0, 1.0) * 255.0) as u8;
                rgba[base + 2] = ((0.02 + b * 0.95).clamp(0.0, 1.0) * 255.0) as u8;
                rgba[base + 3] = 255;
            }
        }
    }

    for y in 0..height {
        for split in [panel_width, panel_width * 2] {
            let idx = (y * width + split.saturating_sub(1)) as usize;
            let base = idx * 4;
            rgba[base] = 50;
            rgba[base + 1] = 50;
            rgba[base + 2] = 50;
            rgba[base + 3] = 255;
        }
    }

    Some((width, height, rgba))
}

fn render_histogram(data: &HistogramData) -> Option<(u32, u32, Vec<u8>)> {
    let bins = data.bins[0].len().max(1);
    let width = 512u32;
    let height = 256u32;
    let total = (width * height) as usize;
    let mut rgba = vec![0u8; total * 4];

    for px in rgba.chunks_exact_mut(4) {
        px[0] = 8;
        px[1] = 8;
        px[2] = 10;
        px[3] = 255;
    }

    if data.peak == 0 {
        return Some((width, height, rgba));
    }

    let peak = data.peak as f32;
    for x in 0..width {
        let bin_idx = ((x as usize * bins) / width as usize).min(bins - 1);
        let r_h = ((data.bins[0][bin_idx] as f32 / peak) * (height as f32 - 1.0)) as u32;
        let g_h = ((data.bins[1][bin_idx] as f32 / peak) * (height as f32 - 1.0)) as u32;
        let b_h = ((data.bins[2][bin_idx] as f32 / peak) * (height as f32 - 1.0)) as u32;
        let l_h = ((data.bins[3][bin_idx] as f32 / peak) * (height as f32 - 1.0)) as u32;

        for y in 0..height {
            let from_bottom = height - 1 - y;
            let mut r = 0u8;
            let mut g = 0u8;
            let mut b = 0u8;
            if from_bottom <= r_h {
                r = 140;
            }
            if from_bottom <= g_h {
                g = 140;
            }
            if from_bottom <= b_h {
                b = 170;
            }
            if from_bottom == l_h {
                r = r.saturating_add(85);
                g = g.saturating_add(85);
                b = b.saturating_add(85);
            }

            let idx = (y * width + x) as usize * 4;
            rgba[idx] = rgba[idx].saturating_add(r);
            rgba[idx + 1] = rgba[idx + 1].saturating_add(g);
            rgba[idx + 2] = rgba[idx + 2].saturating_add(b);
        }
    }

    Some((width, height, rgba))
}
