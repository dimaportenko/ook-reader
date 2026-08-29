use crate::renderer::tao::window::Window;

/// Hands the window's frame to AppKit's autosave, which persists it to
/// `NSUserDefaults` on every move and resize and constrains it to a visible
/// screen on the way back in.
///
/// Called from `App`'s first render rather than from `Config::with_on_window`,
/// and the ordering is the whole reason this works. Dioxus builds the window
/// hidden, restores its own remembered geometry over it — debug builds only,
/// `app.rs:539` — and only shows the window once the first render has been
/// applied. Restoring at window-build time therefore lands *before* that
/// override and loses to it under `dx serve` while winning in a release build.
/// The first render is after the override and still before the window is shown,
/// so the frame is the last word and no one watches it move.
#[cfg(target_os = "macos")]
pub(crate) fn remember_frame(window: &Window) {
    use crate::renderer::tao::platform::macos::WindowExtMacOS;
    use objc2_app_kit::NSWindow;
    use objc2_foundation::NSString;

    const FRAME_AUTOSAVE_NAME: &str = "ook-reader-main";

    let ptr = window.ns_window().cast::<NSWindow>();
    let Some(ns_window) = (unsafe { ptr.as_ref() }) else {
        return;
    };

    let name = NSString::from_str(FRAME_AUTOSAVE_NAME);
    ns_window.setFrameUsingName(&name);
    ns_window.setFrameAutosaveName(&name);
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn remember_frame(_window: &Window) {}
