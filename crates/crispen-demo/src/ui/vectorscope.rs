//! Scope panel renderer and selector UI.
//!
//! Supports vectorscope, waveform, RGB parade, and histogram display
//! modes in the bottom panel's Scopes section.

use bevy::asset::RenderAssetUsages;
use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use crispen_bevy::resources::{GradingState, ScopeState};
use crispen_core::color_management::{CieChromaticity, chromaticity};
use crispen_core::scopes::{CieData, HistogramData, VectorscopeData, WaveformData};

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
    CieDiagram,
}

impl ScopeViewMode {
    fn label(self) -> &'static str {
        match self {
            Self::Vectorscope => "Vectorscope",
            Self::Waveform => "Waveform",
            Self::RgbParade => "RGB Parade",
            Self::Histogram => "Histogram",
            Self::CieDiagram => "CIE Chromaticity",
        }
    }

    fn missing_text(self) -> &'static str {
        match self {
            Self::Vectorscope => "No vectorscope data",
            Self::Waveform => "No waveform data",
            Self::RgbParade => "No waveform data for parade",
            Self::Histogram => "No histogram data",
            Self::CieDiagram => "No CIE data",
        }
    }

    fn is_square(self) -> bool {
        matches!(self, Self::Vectorscope | Self::CieDiagram)
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
                            max_height: Val::Px(240.0),
                            overflow: Overflow::scroll_y(),
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
                            ScopeViewMode::CieDiagram,
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
#[allow(clippy::type_complexity)]
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
#[allow(clippy::type_complexity)]
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
#[allow(clippy::type_complexity)]
pub fn update_scope_texture(
    scope_state: Res<ScopeState>,
    view_state: Res<ScopeViewState>,
    grading_state: Res<GradingState>,
    scope_image: Option<Res<VectorscopeImageHandle>>,
    mut images: ResMut<Assets<Image>>,
    mut ui_parts: ParamSet<(
        Query<&mut Node, With<ScopePlotArea>>,
        Query<&mut Node, With<ScopeImageFrame>>,
        Query<(&mut Node, &mut Text), With<ScopeHint>>,
    )>,
) {
    if !(scope_state.is_changed() || view_state.is_changed() || grading_state.is_changed()) {
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

    let output_gamut = chromaticity(grading_state.params.color_management.output_space);

    let rendered = match view_state.mode {
        ScopeViewMode::Vectorscope => scope_state
            .vectorscope
            .as_ref()
            .and_then(render_vectorscope),
        ScopeViewMode::Waveform => scope_state.waveform.as_ref().and_then(render_waveform),
        ScopeViewMode::RgbParade => scope_state.waveform.as_ref().and_then(render_parade),
        ScopeViewMode::Histogram => scope_state.histogram.as_ref().and_then(render_histogram),
        ScopeViewMode::CieDiagram => scope_state
            .cie
            .as_ref()
            .and_then(|d| render_cie(d, output_gamut)),
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
    let skin_line_width = 2.5 / radius.max(1.0);
    // Skin tone indicator (I-line): 33° from the +Cr axis in the CbCr plane.
    let skin_angle = 33.0_f32.to_radians();
    let skin_dir = Vec2::new(-skin_angle.sin(), skin_angle.cos());

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

                let skin_dist = (nx * skin_dir.y - ny * skin_dir.x).abs();
                if skin_dist <= skin_line_width && dist <= 1.0 {
                    let blend = 1.0 - (skin_dist / skin_line_width);
                    r += blend * 0.16;
                    g += blend * 0.10;
                    b += blend * 0.03;
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
    for (idx, out_r) in accum[0].iter().enumerate().take(out_total) {
        let r = ((*out_r + 1.0).ln() / log_peak).clamp(0.0, 1.0);
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

    for (ch, channel) in accum.iter().enumerate() {
        let x_offset = ch as u32 * panel_width;
        for y in 0..height {
            for x in 0..panel_width {
                let dst_idx = (y * width + x + x_offset) as usize;
                let src_idx = (y * panel_width + x) as usize;
                let signal = ((channel[src_idx] + 1.0).ln() / log_peak).clamp(0.0, 1.0);

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

// ---------------------------------------------------------------------------
// CIE 1931 chromaticity diagram
// ---------------------------------------------------------------------------

/// CIE 1931 standard observer spectral locus boundary (xy coordinates).
///
/// 81 points from 380 nm to 780 nm at 5 nm intervals, derived from
/// the CIE 1931 2-degree standard observer color matching functions.
const SPECTRAL_LOCUS: [[f32; 2]; 81] = [
    [0.1741, 0.0050], // 380 nm
    [0.1740, 0.0050],
    [0.1738, 0.0049],
    [0.1736, 0.0049],
    [0.1733, 0.0048],
    [0.1730, 0.0048], // 405 nm
    [0.1726, 0.0048],
    [0.1721, 0.0048],
    [0.1714, 0.0051],
    [0.1703, 0.0058],
    [0.1689, 0.0069], // 430 nm
    [0.1669, 0.0086],
    [0.1644, 0.0109],
    [0.1611, 0.0138],
    [0.1566, 0.0177],
    [0.1510, 0.0227], // 455 nm
    [0.1440, 0.0297],
    [0.1355, 0.0399],
    [0.1241, 0.0578],
    [0.1096, 0.0868],
    [0.0913, 0.1327], // 480 nm
    [0.0687, 0.2007],
    [0.0454, 0.2950],
    [0.0235, 0.4127],
    [0.0082, 0.5384],
    [0.0039, 0.6548], // 505 nm
    [0.0139, 0.7502],
    [0.0389, 0.8120],
    [0.0743, 0.8338],
    [0.1142, 0.8262],
    [0.1547, 0.8059], // 530 nm
    [0.1929, 0.7816],
    [0.2296, 0.7543],
    [0.2658, 0.7243],
    [0.3016, 0.6923],
    [0.3373, 0.6589], // 555 nm
    [0.3731, 0.6245],
    [0.4087, 0.5896],
    [0.4441, 0.5547],
    [0.4788, 0.5202],
    [0.5125, 0.4866], // 580 nm
    [0.5448, 0.4544],
    [0.5752, 0.4242],
    [0.6029, 0.3965],
    [0.6270, 0.3725],
    [0.6482, 0.3514], // 605 nm
    [0.6658, 0.3340],
    [0.6801, 0.3197],
    [0.6915, 0.3083],
    [0.7006, 0.2993],
    [0.7079, 0.2920], // 630 nm
    [0.7140, 0.2859],
    [0.7190, 0.2809],
    [0.7230, 0.2770],
    [0.7260, 0.2740],
    [0.7283, 0.2717], // 655 nm
    [0.7300, 0.2700],
    [0.7311, 0.2689],
    [0.7320, 0.2680],
    [0.7327, 0.2673],
    [0.7334, 0.2666], // 680 nm
    [0.7340, 0.2660],
    [0.7344, 0.2656],
    [0.7346, 0.2654],
    [0.7347, 0.2653],
    [0.7347, 0.2653], // 705 nm
    [0.7347, 0.2653],
    [0.7347, 0.2653],
    [0.7347, 0.2653],
    [0.7347, 0.2653],
    [0.7347, 0.2653], // 730 nm
    [0.7347, 0.2653],
    [0.7347, 0.2653],
    [0.7347, 0.2653],
    [0.7347, 0.2653],
    [0.7347, 0.2653], // 755 nm
    [0.7347, 0.2653],
    [0.7347, 0.2653],
    [0.7347, 0.2653],
    [0.7347, 0.2653],
    [0.7347, 0.2653], // 780 nm
];

/// Draw an anti-aliased line segment onto an RGBA buffer.
///
/// Uses bilinear sub-pixel blending for smooth rendering.
fn draw_line(
    rgba: &mut [u8],
    resolution: u32,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    color: [u8; 3],
) {
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let step_count = (dx.max(dy) as u32).max(1);

    for i in 0..=step_count {
        let t = i as f32 / step_count as f32;
        let px = x0 + (x1 - x0) * t;
        let py = y0 + (y1 - y0) * t;

        let ix = px.floor() as i32;
        let iy = py.floor() as i32;
        let fx = px - ix as f32;
        let fy = py - iy as f32;

        let weights = [
            (ix, iy, (1.0 - fx) * (1.0 - fy)),
            (ix + 1, iy, fx * (1.0 - fy)),
            (ix, iy + 1, (1.0 - fx) * fy),
            (ix + 1, iy + 1, fx * fy),
        ];

        for (wx, wy, weight) in weights {
            if wx >= 0 && wx < resolution as i32 && wy >= 0 && wy < resolution as i32 {
                let idx = (wy as u32 * resolution + wx as u32) as usize * 4;
                let w = weight.clamp(0.0, 1.0);
                rgba[idx] = rgba[idx].saturating_add((color[0] as f32 * w) as u8);
                rgba[idx + 1] = rgba[idx + 1].saturating_add((color[1] as f32 * w) as u8);
                rgba[idx + 2] = rgba[idx + 2].saturating_add((color[2] as f32 * w) as u8);
            }
        }
    }
}

/// Map CIE xy coordinates to pixel coordinates.
///
/// Matches the mapping in `cie::compute()`:
/// - x range [0, 0.8] maps to [0, resolution-1]
/// - y range [0, 0.9] maps to [resolution-1, 0] (inverted)
fn cie_to_pixel(cx: f32, cy: f32, res: f32) -> (f32, f32) {
    let px = cx / 0.8 * res;
    let py = (1.0 - cy / 0.9) * res;
    (px, py)
}

fn render_cie(data: &CieData, gamut: &CieChromaticity) -> Option<(u32, u32, Vec<u8>)> {
    let resolution = data.resolution.max(1);
    let pixel_count = (resolution as usize).saturating_mul(resolution as usize);
    if data.density.len() < pixel_count {
        return None;
    }

    let res_f = (resolution - 1) as f32;
    let mut rgba = vec![0u8; pixel_count * 4];

    // Dark background fill
    for px in rgba.chunks_exact_mut(4) {
        px[0] = 5;
        px[1] = 5;
        px[2] = 6;
        px[3] = 255;
    }

    // --- Spectral locus outline ---
    let locus_color: [u8; 3] = [38, 38, 42];
    for i in 0..SPECTRAL_LOCUS.len() - 1 {
        let (x0, y0) = cie_to_pixel(SPECTRAL_LOCUS[i][0], SPECTRAL_LOCUS[i][1], res_f);
        let (x1, y1) = cie_to_pixel(SPECTRAL_LOCUS[i + 1][0], SPECTRAL_LOCUS[i + 1][1], res_f);
        draw_line(&mut rgba, resolution, x0, y0, x1, y1, locus_color);
    }
    // Purple line: connect 780 nm back to 380 nm
    let last = SPECTRAL_LOCUS.len() - 1;
    let (x0, y0) = cie_to_pixel(SPECTRAL_LOCUS[last][0], SPECTRAL_LOCUS[last][1], res_f);
    let (x1, y1) = cie_to_pixel(SPECTRAL_LOCUS[0][0], SPECTRAL_LOCUS[0][1], res_f);
    draw_line(&mut rgba, resolution, x0, y0, x1, y1, locus_color);

    // --- Output gamut triangle ---
    let triangle_color: [u8; 3] = [70, 75, 80];
    let primaries = [
        [gamut.r[0] as f32, gamut.r[1] as f32],
        [gamut.g[0] as f32, gamut.g[1] as f32],
        [gamut.b[0] as f32, gamut.b[1] as f32],
    ];
    for i in 0..3 {
        let j = (i + 1) % 3;
        let (x0, y0) = cie_to_pixel(primaries[i][0], primaries[i][1], res_f);
        let (x1, y1) = cie_to_pixel(primaries[j][0], primaries[j][1], res_f);
        draw_line(&mut rgba, resolution, x0, y0, x1, y1, triangle_color);
    }

    // --- White point cross ---
    let wp = [gamut.w[0] as f32, gamut.w[1] as f32];
    let (wpx, wpy) = cie_to_pixel(wp[0], wp[1], res_f);
    let cross_size = res_f * 0.015;
    let wp_color: [u8; 3] = [90, 90, 95];
    draw_line(
        &mut rgba,
        resolution,
        wpx - cross_size,
        wpy,
        wpx + cross_size,
        wpy,
        wp_color,
    );
    draw_line(
        &mut rgba,
        resolution,
        wpx,
        wpy - cross_size,
        wpx,
        wpy + cross_size,
        wp_color,
    );

    // --- Pixel density overlay ---
    let peak = data.density.iter().copied().max().unwrap_or(0) as f32;
    if peak > 0.0 {
        let log_peak = (peak + 1.0).ln().max(1.0);

        for y in 0..resolution {
            for x in 0..resolution {
                let idx = (y * resolution + x) as usize;
                let d = data.density[idx] as f32;
                if d <= 0.0 {
                    continue;
                }

                let signal = ((d + 1.0).ln() / log_peak).clamp(0.0, 1.0).powf(0.65);
                let base = idx * 4;
                rgba[base] = rgba[base].saturating_add((signal * 0.42 * 255.0) as u8);
                rgba[base + 1] = rgba[base + 1].saturating_add((signal * 0.90 * 255.0) as u8);
                rgba[base + 2] = rgba[base + 2].saturating_add((signal * 0.52 * 255.0) as u8);
            }
        }
    }

    Some((resolution, resolution, rgba))
}
