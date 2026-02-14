//! Panel layout synchronization between dockview (Svelte) and Bevy.
//!
//! When the Svelte dockview sends a `LayoutUpdate` IPC message, this module
//! repositions and resizes Bevy UI containers to match the panel regions.

use bevy::prelude::*;

use crate::ipc::LayoutRegion;

/// Resource holding the latest panel layout from dockview.
#[derive(Resource, Default)]
pub struct PanelLayout {
    pub regions: Vec<LayoutRegion>,
}

/// Marker component identifying a Bevy UI entity that should be positioned
/// by the layout sync system. The `panel_id` must match a dockview panel ID.
#[derive(Component)]
pub struct LayoutPanel {
    pub panel_id: String,
}

/// Plugin that registers layout sync resources and systems.
pub struct LayoutSyncPlugin;

impl Plugin for LayoutSyncPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PanelLayout>()
            .add_systems(Update, sync_panel_layout);
    }
}

/// Reposition `LayoutPanel`-tagged entities to match dockview panel regions.
///
/// CEF reports coordinates in its CSS pixel space, which equals physical pixels
/// (CEF is created at physical window dimensions with device_scale_factor=1).
/// Bevy's `Val::Px` uses logical pixels. On HiDPI displays we must divide by
/// the window scale factor to align the Bevy entity with the transparent
/// cutout in the CEF overlay.
fn sync_panel_layout(
    layout: Res<PanelLayout>,
    mut query: Query<(&LayoutPanel, &mut Node, &mut Visibility)>,
    windows: Query<&Window>,
) {
    if !layout.is_changed() {
        return;
    }

    let scale = windows
        .single()
        .map(|w| w.scale_factor() as f32)
        .unwrap_or(1.0);

    for (panel, mut node, mut vis) in &mut query {
        if let Some(region) = layout.regions.iter().find(|r| r.id == panel.panel_id) {
            let lx = region.x / scale;
            let ly = region.y / scale;
            let lw = region.width / scale;
            let lh = region.height / scale;
            tracing::info!(
                "sync_panel_layout: positioning '{}' at ({:.0}, {:.0}) {:.0}x{:.0} (scale={:.3}, css={}, {})",
                panel.panel_id, lx, ly, lw, lh, scale, region.x, region.y
            );
            node.position_type = PositionType::Absolute;
            node.left = Val::Px(lx);
            node.top = Val::Px(ly);
            node.width = Val::Px(lw);
            node.height = Val::Px(lh);

            *vis = if region.visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        } else {
            // Panel not in layout — hide it.
            *vis = Visibility::Hidden;
        }
    }
}
