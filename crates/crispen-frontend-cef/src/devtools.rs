//! Chrome DevTools support (Ctrl+Shift+I).

use cef::{Browser, BrowserSettings, ImplBrowser, ImplBrowserHost, WindowInfo};

/// Open a DevTools window for the given browser.
pub fn show_dev_tools(browser: &Browser) {
    let Some(host) = browser.host() else { return };

    let window_info = WindowInfo::default();
    let settings = BrowserSettings::default();

    tracing::info!("opening CEF DevTools");
    host.show_dev_tools(Some(&window_info), None, Some(&settings), None);
}

/// Close DevTools if open.
pub fn close_dev_tools(browser: &Browser) {
    let Some(host) = browser.host() else { return };
    host.close_dev_tools();
}

/// Whether DevTools is currently open.
pub fn has_dev_tools(browser: &Browser) -> bool {
    browser.host().map(|h| h.has_dev_tools() != 0).unwrap_or(false)
}

/// Toggle DevTools visibility.
pub fn toggle_dev_tools(browser: &Browser) {
    if has_dev_tools(browser) {
        close_dev_tools(browser);
    } else {
        show_dev_tools(browser);
    }
}
