//! Keyboard event forwarding to CEF.

use bevy::prelude::*;
use crispen_frontend_core::{CompositeBackend, KeyboardEvent, Modifiers};

use crate::cef_bridge::CefFrontendResource;

/// Forward key press / release events to CEF.
pub fn forward_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    webview: Option<NonSendMut<CefFrontendResource>>,
) {
    let Some(mut wv) = webview else { return };

    let modifiers = Modifiers {
        shift: keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight),
        ctrl: keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight),
        alt: keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight),
        meta: keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight),
    };

    for code in keys.get_just_pressed() {
        if let Some(key) = keycode_to_string(*code) {
            wv.backend.send_keyboard_event(KeyboardEvent {
                key,
                pressed: true,
                modifiers: modifiers.clone(),
            });
        }
    }

    for code in keys.get_just_released() {
        if let Some(key) = keycode_to_string(*code) {
            wv.backend.send_keyboard_event(KeyboardEvent {
                key,
                pressed: false,
                modifiers: modifiers.clone(),
            });
        }
    }
}

/// Map Bevy `KeyCode` to a web-compatible key string.
fn keycode_to_string(code: KeyCode) -> Option<String> {
    let c = match code {
        KeyCode::KeyA => 'a',
        KeyCode::KeyB => 'b',
        KeyCode::KeyC => 'c',
        KeyCode::KeyD => 'd',
        KeyCode::KeyE => 'e',
        KeyCode::KeyF => 'f',
        KeyCode::KeyG => 'g',
        KeyCode::KeyH => 'h',
        KeyCode::KeyI => 'i',
        KeyCode::KeyJ => 'j',
        KeyCode::KeyK => 'k',
        KeyCode::KeyL => 'l',
        KeyCode::KeyM => 'm',
        KeyCode::KeyN => 'n',
        KeyCode::KeyO => 'o',
        KeyCode::KeyP => 'p',
        KeyCode::KeyQ => 'q',
        KeyCode::KeyR => 'r',
        KeyCode::KeyS => 's',
        KeyCode::KeyT => 't',
        KeyCode::KeyU => 'u',
        KeyCode::KeyV => 'v',
        KeyCode::KeyW => 'w',
        KeyCode::KeyX => 'x',
        KeyCode::KeyY => 'y',
        KeyCode::KeyZ => 'z',
        KeyCode::Digit0 => '0',
        KeyCode::Digit1 => '1',
        KeyCode::Digit2 => '2',
        KeyCode::Digit3 => '3',
        KeyCode::Digit4 => '4',
        KeyCode::Digit5 => '5',
        KeyCode::Digit6 => '6',
        KeyCode::Digit7 => '7',
        KeyCode::Digit8 => '8',
        KeyCode::Digit9 => '9',
        KeyCode::Space => ' ',
        KeyCode::Enter => return Some("Enter".into()),
        KeyCode::Escape => return Some("Escape".into()),
        KeyCode::Backspace => return Some("Backspace".into()),
        KeyCode::Tab => return Some("Tab".into()),
        KeyCode::ArrowUp => return Some("ArrowUp".into()),
        KeyCode::ArrowDown => return Some("ArrowDown".into()),
        KeyCode::ArrowLeft => return Some("ArrowLeft".into()),
        KeyCode::ArrowRight => return Some("ArrowRight".into()),
        KeyCode::Delete => return Some("Delete".into()),
        KeyCode::Home => return Some("Home".into()),
        KeyCode::End => return Some("End".into()),
        _ => return None,
    };
    Some(c.to_string())
}
