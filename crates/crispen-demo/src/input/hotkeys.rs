//! Global hotkeys (e.g. Ctrl+Shift+I for DevTools).

use bevy::prelude::*;

use crate::cef_bridge::CefFrontendResource;

/// Toggle Chrome DevTools with Ctrl+Shift+I.
pub fn handle_devtools_hotkey(
    keys: Res<ButtonInput<KeyCode>>,
    webview: Option<NonSendMut<CefFrontendResource>>,
) {
    let Some(wv) = webview else { return };

    if keys.just_pressed(KeyCode::KeyI)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
    {
        wv.backend.show_dev_tools();
    }
}
