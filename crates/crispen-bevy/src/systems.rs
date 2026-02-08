//! Bevy systems for the color grading pipeline.

use bevy::prelude::*;

use crate::events::ParamsUpdatedEvent;
use crate::resources::GradingState;

/// System that detects grading parameter changes and triggers LUT re-bake.
pub fn detect_param_changes(
    mut state: ResMut<GradingState>,
    mut events: MessageReader<ParamsUpdatedEvent>,
) {
    let _ = (&mut state, &mut events);
    todo!()
}

/// System that dispatches GPU scope computation after grading.
pub fn compute_scopes() {
    todo!()
}
