//! Hue-vs-curves panel section.
//!
//! This mirrors the Resolve-style "Hue vs Curves" area with mode tabs
//! and a curve plot preview.

use bevy::picking::Pickable;
use bevy::picking::events::{Cancel, Drag, DragEnd, DragStart, Pointer, Press};
use bevy::prelude::*;
use bevy::ui::{ComputedNode, ComputedUiRenderTargetInfo, UiGlobalTransform, UiScale};
use crispen_bevy::resources::GradingState;

use super::theme;

/// Hint text shown when the curve plot has no control points.
#[derive(Component)]
struct CurveHintText;

const CURVE_TRACE_SAMPLES: usize = 84;
const CURVE_THUMB_SIZE: f32 = 8.0;

#[derive(Debug, Clone, Copy)]
struct CurvePoint {
    id: u32,
    x: f32,
    y: f32,
}

/// Available curve modes in this panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HueCurveMode {
    HueVsHue,
    HueVsSat,
    LumVsSat,
}

impl HueCurveMode {
    fn label(self) -> &'static str {
        match self {
            Self::HueVsHue => "Hue vs Hue",
            Self::HueVsSat => "Hue vs Sat",
            Self::LumVsSat => "Lum vs Sat",
        }
    }
}

/// Runtime state for hue-vs-curves control points.
#[derive(Resource)]
pub struct HueCurvesState {
    mode: HueCurveMode,
    hue_vs_hue: Vec<CurvePoint>,
    hue_vs_sat: Vec<CurvePoint>,
    lum_vs_sat: Vec<CurvePoint>,
    next_point_id: u32,
}

impl Default for HueCurvesState {
    fn default() -> Self {
        Self {
            mode: HueCurveMode::HueVsHue,
            // Start with no nodes; empty vectors map to identity curves.
            hue_vs_hue: Vec::new(),
            hue_vs_sat: Vec::new(),
            lum_vs_sat: Vec::new(),
            next_point_id: 1,
        }
    }
}

impl HueCurvesState {
    fn points_for_mode(&self, mode: HueCurveMode) -> &[CurvePoint] {
        match mode {
            HueCurveMode::HueVsHue => &self.hue_vs_hue,
            HueCurveMode::HueVsSat => &self.hue_vs_sat,
            HueCurveMode::LumVsSat => &self.lum_vs_sat,
        }
    }

    fn points_for_mode_mut(&mut self, mode: HueCurveMode) -> &mut Vec<CurvePoint> {
        match mode {
            HueCurveMode::HueVsHue => &mut self.hue_vs_hue,
            HueCurveMode::HueVsSat => &mut self.hue_vs_sat,
            HueCurveMode::LumVsSat => &mut self.lum_vs_sat,
        }
    }

    fn active_points(&self) -> &[CurvePoint] {
        self.points_for_mode(self.mode)
    }

    fn active_points_mut(&mut self) -> &mut Vec<CurvePoint> {
        self.points_for_mode_mut(self.mode)
    }

    fn add_active_point(&mut self, x: f32, y: f32) {
        let point = CurvePoint {
            id: self.next_point_id,
            x: x.clamp(0.0, 1.0),
            y: y.clamp(0.0, 1.0),
        };
        self.next_point_id = self.next_point_id.wrapping_add(1);
        let points = self.active_points_mut();
        points.push(point);
        points.sort_by(|a, b| a.x.total_cmp(&b.x));
    }

    fn update_active_point(&mut self, point_id: u32, x: f32, y: f32) {
        let points = self.active_points_mut();
        if let Some(point) = points.iter_mut().find(|point| point.id == point_id) {
            point.x = x.clamp(0.0, 1.0);
            point.y = y.clamp(0.0, 1.0);
            points.sort_by(|a, b| a.x.total_cmp(&b.x));
        }
    }
}

/// Marker for the curve plot root node.
#[derive(Component)]
struct HueCurvePlot;

/// Marker on curve mode tab buttons.
#[derive(Component, Clone, Copy)]
struct HueCurveModeButton(HueCurveMode);

/// Marker on draggable control points.
#[derive(Component)]
struct HueCurveThumb {
    point_id: u32,
}

/// Marker on sample nodes used to draw the curve trace.
#[derive(Component)]
struct HueCurveTraceSample {
    t: f32,
}

/// Drag state for a control point thumb.
#[derive(Component, Default)]
struct HueCurveDragState {
    active: bool,
}

/// Spawn the hue-vs-curves section in the bottom panel.
pub fn spawn_hue_curves_section(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                row_gap: Val::Px(8.0),
                width: Val::Px(theme::HUE_CURVES_SECTION_WIDTH),
                flex_shrink: 0.0,
                padding: UiRect::left(Val::Px(12.0)),
                border: UiRect::left(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(theme::BORDER_SUBTLE),
        ))
        .with_children(|section| {
            section.spawn((
                Text::new("Hue vs Curves"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(theme::TEXT_PRIMARY),
            ));
            spawn_curve_mode_tabs(section);
            spawn_curve_plot(section);
        });
}

fn spawn_curve_mode_tabs(section: &mut ChildSpawnerCommands) {
    section
        .spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(6.0),
            width: Val::Percent(100.0),
            ..default()
        })
        .with_children(|tabs| {
            for mode in [
                HueCurveMode::HueVsHue,
                HueCurveMode::HueVsSat,
                HueCurveMode::LumVsSat,
            ] {
                tabs.spawn((
                    Button,
                    HueCurveModeButton(mode),
                    Node {
                        flex_grow: 1.0,
                        height: Val::Px(24.0),
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
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
                        TextColor(theme::TEXT_DIM),
                        Pickable::IGNORE,
                    )],
                ));
            }
        });
}

fn spawn_curve_plot(section: &mut ChildSpawnerCommands) {
    section
        .spawn((
            HueCurvePlot,
            Node {
                position_type: PositionType::Relative,
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(theme::CURVE_PLOT_BG),
            BorderColor::all(theme::BORDER_SUBTLE),
        ))
        .with_children(|plot| {
            spawn_grid_lines(plot);
            spawn_neutral_line(plot);
            spawn_curve_trace(plot);
            spawn_hue_markers(plot);

            // Hint text shown until the user adds a control point.
            plot.spawn((
                CurveHintText,
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                Text::new("Ctrl+Click to add points"),
                TextFont {
                    font_size: theme::FONT_SIZE_LABEL,
                    ..default()
                },
                TextColor(theme::TEXT_DIM),
                Pickable::IGNORE,
            ));
        });
}

fn spawn_grid_lines(plot: &mut ChildSpawnerCommands) {
    for step in 0..=4 {
        let pct = step as f32 * 25.0;
        plot.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Percent(pct),
                height: Val::Px(1.0),
                ..default()
            },
            BackgroundColor(theme::CURVE_GRID_LINE),
            Pickable::IGNORE,
        ));
        plot.spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                left: Val::Percent(pct),
                width: Val::Px(1.0),
                ..default()
            },
            BackgroundColor(theme::CURVE_GRID_LINE),
            Pickable::IGNORE,
        ));
    }
}

fn spawn_neutral_line(plot: &mut ChildSpawnerCommands) {
    plot.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Percent(50.0),
            height: Val::Px(1.0),
            ..default()
        },
        BackgroundColor(theme::CURVE_NEUTRAL_LINE),
        Pickable::IGNORE,
    ));
}

fn spawn_curve_trace(plot: &mut ChildSpawnerCommands) {
    for idx in 0..=CURVE_TRACE_SAMPLES {
        let t = idx as f32 / CURVE_TRACE_SAMPLES as f32;
        plot.spawn((
            HueCurveTraceSample { t },
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(t * 100.0),
                top: Val::Percent(50.0),
                width: Val::Px(2.0),
                height: Val::Px(2.0),
                margin: UiRect::all(Val::Px(-1.0)),
                ..default()
            },
            BackgroundColor(theme::TEXT_PRIMARY),
            Pickable::IGNORE,
        ));
    }
}

fn spawn_hue_markers(plot: &mut ChildSpawnerCommands) {
    plot.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            right: Val::Px(10.0),
            bottom: Val::Px(10.0),
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        },
        Pickable::IGNORE,
    ))
    .with_children(|swatches| {
        for color in [
            Color::srgb(0.98, 0.20, 0.20),
            Color::srgb(0.95, 0.76, 0.18),
            Color::srgb(0.22, 0.86, 0.32),
            Color::srgb(0.20, 0.85, 0.90),
            Color::srgb(0.24, 0.45, 0.98),
            Color::srgb(0.86, 0.24, 0.88),
        ] {
            swatches.spawn((
                Node {
                    width: Val::Px(8.0),
                    height: Val::Px(8.0),
                    border_radius: BorderRadius::all(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(color),
                Pickable::IGNORE,
            ));
        }
    });
}

fn pointer_to_plot_normalized(
    pointer_pos: Vec2,
    node: &ComputedNode,
    node_target: &ComputedUiRenderTargetInfo,
    transform: &UiGlobalTransform,
    ui_scale: f32,
) -> Vec2 {
    let local_pos = transform
        .try_inverse()
        .expect("curve plot transform should be invertible")
        .transform_point2(pointer_pos * node_target.scale_factor() / ui_scale);
    let pos = local_pos / node.size() + Vec2::splat(0.5);
    pos.clamp(Vec2::ZERO, Vec2::ONE)
}

fn update_control_from_pointer(
    thumb_entity: Entity,
    pointer_pos: Vec2,
    q_thumb_parent: &Query<&ChildOf, With<HueCurveThumb>>,
    q_thumb: &Query<&HueCurveThumb>,
    q_plot: &Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
        ),
        With<HueCurvePlot>,
    >,
    ui_scale: f32,
    state: &mut HueCurvesState,
) {
    let Ok(thumb) = q_thumb.get(thumb_entity) else {
        return;
    };
    let Ok(parent) = q_thumb_parent.get(thumb_entity) else {
        return;
    };
    let Ok((node, node_target, transform)) = q_plot.get(parent.0) else {
        return;
    };

    let normalized =
        pointer_to_plot_normalized(pointer_pos, node, node_target, transform, ui_scale);
    // UI y is top-down; curve y is bottom-up.
    state.update_active_point(
        thumb.point_id,
        normalized.x.clamp(0.0, 1.0),
        (1.0 - normalized.y).clamp(0.0, 1.0),
    );
}

fn on_curve_plot_press(
    mut press: On<Pointer<Press>>,
    q_plot: Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
        ),
        With<HueCurvePlot>,
    >,
    ui_scale: Res<UiScale>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<HueCurvesState>,
) {
    let Ok((node, node_target, transform)) = q_plot.get(press.entity) else {
        return;
    };
    let ctrl_pressed = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    if !ctrl_pressed {
        return;
    }

    press.propagate(false);
    let normalized = pointer_to_plot_normalized(
        press.pointer_location.position,
        node,
        node_target,
        transform,
        ui_scale.0,
    );
    state.add_active_point(
        normalized.x.clamp(0.0, 1.0),
        (1.0 - normalized.y).clamp(0.0, 1.0),
    );
}

fn on_curve_thumb_press(
    mut press: On<Pointer<Press>>,
    q_thumb_parent: Query<&ChildOf, With<HueCurveThumb>>,
    q_thumb: Query<&HueCurveThumb>,
    q_plot: Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
        ),
        With<HueCurvePlot>,
    >,
    ui_scale: Res<UiScale>,
    mut state: ResMut<HueCurvesState>,
) {
    if q_thumb.get(press.entity).is_err() {
        return;
    }
    press.propagate(false);
    update_control_from_pointer(
        press.entity,
        press.pointer_location.position,
        &q_thumb_parent,
        &q_thumb,
        &q_plot,
        ui_scale.0,
        &mut state,
    );
}

fn on_curve_thumb_drag_start(
    mut drag_start: On<Pointer<DragStart>>,
    mut q_drag: Query<&mut HueCurveDragState, With<HueCurveThumb>>,
    q_thumb_parent: Query<&ChildOf, With<HueCurveThumb>>,
    q_thumb: Query<&HueCurveThumb>,
    q_plot: Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
        ),
        With<HueCurvePlot>,
    >,
    ui_scale: Res<UiScale>,
    mut state: ResMut<HueCurvesState>,
) {
    let Ok(mut drag_state) = q_drag.get_mut(drag_start.entity) else {
        return;
    };

    drag_start.propagate(false);
    drag_state.active = true;
    update_control_from_pointer(
        drag_start.entity,
        drag_start.pointer_location.position,
        &q_thumb_parent,
        &q_thumb,
        &q_plot,
        ui_scale.0,
        &mut state,
    );
}

fn on_curve_thumb_drag(
    mut drag: On<Pointer<Drag>>,
    q_drag: Query<&HueCurveDragState, With<HueCurveThumb>>,
    q_thumb_parent: Query<&ChildOf, With<HueCurveThumb>>,
    q_thumb: Query<&HueCurveThumb>,
    q_plot: Query<
        (
            &ComputedNode,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
        ),
        With<HueCurvePlot>,
    >,
    ui_scale: Res<UiScale>,
    mut state: ResMut<HueCurvesState>,
) {
    let Ok(drag_state) = q_drag.get(drag.entity) else {
        return;
    };
    drag.propagate(false);
    if !drag_state.active {
        return;
    }

    update_control_from_pointer(
        drag.entity,
        drag.pointer_location.position,
        &q_thumb_parent,
        &q_thumb,
        &q_plot,
        ui_scale.0,
        &mut state,
    );
}

fn on_curve_thumb_drag_end(
    mut drag_end: On<Pointer<DragEnd>>,
    mut q_drag: Query<&mut HueCurveDragState, With<HueCurveThumb>>,
) {
    let Ok(mut drag_state) = q_drag.get_mut(drag_end.entity) else {
        return;
    };
    drag_end.propagate(false);
    drag_state.active = false;
}

fn on_curve_thumb_drag_cancel(
    mut drag_cancel: On<Pointer<Cancel>>,
    mut q_drag: Query<&mut HueCurveDragState, With<HueCurveThumb>>,
) {
    let Ok(mut drag_state) = q_drag.get_mut(drag_cancel.entity) else {
        return;
    };
    drag_cancel.propagate(false);
    drag_state.active = false;
}

#[allow(clippy::type_complexity)]
fn handle_curve_mode_buttons(
    interactions: Query<(&Interaction, &HueCurveModeButton), (Changed<Interaction>, With<Button>)>,
    mut state: ResMut<HueCurvesState>,
) {
    for (interaction, button) in interactions.iter() {
        if *interaction == Interaction::Pressed && state.mode != button.0 {
            state.mode = button.0;
        }
    }
}

fn sample_curve(points: &[CurvePoint], t: f32) -> f32 {
    match points.len() {
        0 => return 0.5,
        1 => return points[0].y,
        _ => {}
    }

    let mut wrapped_t = t.rem_euclid(1.0);
    if (t - 1.0).abs() <= f32::EPSILON {
        wrapped_t = 0.0;
    }

    for idx in 0..points.len() {
        let a = points[idx];
        let b = points[(idx + 1) % points.len()];
        let x0 = a.x;
        let mut x1 = b.x;
        if idx == points.len() - 1 {
            x1 += 1.0;
        }

        let mut segment_t = wrapped_t;
        if segment_t < x0 {
            segment_t += 1.0;
        }
        if segment_t >= x0 && segment_t <= x1 {
            if (x1 - x0).abs() <= f32::EPSILON {
                return b.y;
            }
            let local = ((segment_t - x0) / (x1 - x0)).clamp(0.0, 1.0);
            return a.y + (b.y - a.y) * local;
        }
    }

    points[0].y
}

fn sync_curve_tab_visuals(
    state: Res<HueCurvesState>,
    mut tabs: Query<
        (
            &HueCurveModeButton,
            &Children,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<Button>,
    >,
    mut text_colors: Query<&mut TextColor>,
) {
    for (tab, children, mut bg, mut border) in tabs.iter_mut() {
        let active = tab.0 == state.mode;
        bg.0 = if active {
            Color::srgb(0.18, 0.18, 0.18)
        } else {
            theme::BG_CONTROL
        };
        *border = if active {
            BorderColor::all(theme::ACCENT)
        } else {
            BorderColor::all(theme::BORDER_SUBTLE)
        };

        let Some(&label_entity) = children.first() else {
            continue;
        };
        let Ok(mut color) = text_colors.get_mut(label_entity) else {
            continue;
        };
        color.0 = if active {
            theme::TEXT_PRIMARY
        } else {
            theme::TEXT_DIM
        };
    }
}

fn map_ui_to_hue_offset(y: f32) -> f32 {
    (y - 0.5).clamp(-0.5, 0.5)
}

fn map_ui_to_sat_factor(y: f32) -> f32 {
    (y * 2.0).clamp(0.0, 2.0)
}

fn points_to_grading_curve(mode: HueCurveMode, points: &[CurvePoint]) -> Vec<[f32; 2]> {
    if points.is_empty() {
        return Vec::new();
    }

    let map_y = |y: f32| match mode {
        HueCurveMode::HueVsHue => map_ui_to_hue_offset(y),
        HueCurveMode::HueVsSat | HueCurveMode::LumVsSat => map_ui_to_sat_factor(y),
    };

    // Bake seam anchors from wrapped interpolation so x=0 and x=1 always match.
    let seam_y = sample_curve(points, 0.0);
    let mut out = Vec::with_capacity(points.len() + 2);
    out.push([0.0, map_y(seam_y)]);
    for point in points {
        if point.x > 0.0 && point.x < 1.0 {
            out.push([point.x, map_y(point.y)]);
        }
    }
    out.push([1.0, map_y(seam_y)]);
    out.sort_by(|a, b| a[0].total_cmp(&b[0]));
    out
}

fn sync_hue_curves_to_grading_params(
    curves: Res<HueCurvesState>,
    mut grading: ResMut<GradingState>,
) {
    if !curves.is_changed() {
        return;
    }

    let hue_vs_hue = points_to_grading_curve(HueCurveMode::HueVsHue, &curves.hue_vs_hue);
    let hue_vs_sat = points_to_grading_curve(HueCurveMode::HueVsSat, &curves.hue_vs_sat);
    let lum_vs_sat = points_to_grading_curve(HueCurveMode::LumVsSat, &curves.lum_vs_sat);

    let mut changed = false;
    if grading.params.hue_vs_hue != hue_vs_hue {
        grading.params.hue_vs_hue = hue_vs_hue;
        changed = true;
    }
    if grading.params.hue_vs_sat != hue_vs_sat {
        grading.params.hue_vs_sat = hue_vs_sat;
        changed = true;
    }
    if grading.params.lum_vs_sat != lum_vs_sat {
        grading.params.lum_vs_sat = lum_vs_sat;
        changed = true;
    }

    if changed {
        tracing::info!(
            "sync_hue_curves_to_grading_params: curves changed (hvh={}, hvs={}, lvs={})",
            curves.hue_vs_hue.len(),
            curves.hue_vs_sat.len(),
            curves.lum_vs_sat.len(),
        );
        grading.dirty = true;
    }
}

#[allow(clippy::type_complexity)]
fn sync_curve_visuals(
    state: Res<HueCurvesState>,
    q_plot: Query<Entity, With<HueCurvePlot>>,
    mut commands: Commands,
    mut thumbs: Query<
        (
            Entity,
            &HueCurveThumb,
            &HueCurveDragState,
            &mut Node,
            &mut BackgroundColor,
        ),
        (With<HueCurveThumb>, Without<HueCurveTraceSample>),
    >,
    mut samples: Query<
        (&HueCurveTraceSample, &mut Node),
        (With<HueCurveTraceSample>, Without<HueCurveThumb>),
    >,
    mut hints: Query<&mut Visibility, With<CurveHintText>>,
) {
    let Some(plot_entity) = q_plot.iter().next() else {
        return;
    };
    let active_points = state.active_points();

    // Show/hide the "Ctrl+Click" hint depending on whether points exist.
    for mut vis in hints.iter_mut() {
        *vis = if active_points.is_empty() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    let mut present_ids = Vec::with_capacity(active_points.len());

    for (entity, thumb, drag_state, mut node, mut bg) in thumbs.iter_mut() {
        let Some(point) = active_points
            .iter()
            .find(|point| point.id == thumb.point_id)
            .copied()
        else {
            commands.entity(entity).despawn();
            continue;
        };

        present_ids.push(point.id);
        node.left = Val::Percent(point.x * 100.0);
        node.top = Val::Percent((1.0 - point.y) * 100.0);
        bg.0 = if drag_state.active {
            Color::WHITE
        } else {
            theme::ACCENT
        };
    }

    if !active_points.is_empty() {
        commands.entity(plot_entity).with_children(|plot| {
            for point in active_points {
                if present_ids.contains(&point.id) {
                    continue;
                }
                plot.spawn((
                    HueCurveThumb { point_id: point.id },
                    HueCurveDragState::default(),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(point.x * 100.0),
                        top: Val::Percent((1.0 - point.y) * 100.0),
                        width: Val::Px(CURVE_THUMB_SIZE),
                        height: Val::Px(CURVE_THUMB_SIZE),
                        margin: UiRect::all(Val::Px(-CURVE_THUMB_SIZE * 0.5)),
                        border: UiRect::all(Val::Px(1.0)),
                        border_radius: BorderRadius::all(Val::Px(CURVE_THUMB_SIZE * 0.5)),
                        ..default()
                    },
                    BackgroundColor(theme::ACCENT),
                    BorderColor::all(Color::BLACK),
                ));
            }
        });
    }

    for (sample, mut node) in samples.iter_mut() {
        let y = sample_curve(active_points, sample.t);
        node.left = Val::Percent(sample.t * 100.0);
        node.top = Val::Percent((1.0 - y) * 100.0);
    }
}

/// Registers Hue-vs-Curves state, observers, and sync systems.
pub struct HueCurvesPlugin;

impl Plugin for HueCurvesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HueCurvesState>();
        app.add_systems(
            Update,
            (
                handle_curve_mode_buttons,
                sync_curve_tab_visuals,
                sync_hue_curves_to_grading_params,
            ),
        );
        app.add_systems(PostUpdate, sync_curve_visuals);
        app.add_observer(on_curve_plot_press)
            .add_observer(on_curve_thumb_press)
            .add_observer(on_curve_thumb_drag_start)
            .add_observer(on_curve_thumb_drag)
            .add_observer(on_curve_thumb_drag_end)
            .add_observer(on_curve_thumb_drag_cancel);
    }
}
