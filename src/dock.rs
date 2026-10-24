//! Closing the window only hides it, so Overhead keeps tracking and notifying with no
//! window open. Clicking the Dock icon or a notification brings it back.

#[cfg(target_os = "macos")]
pub fn hide(window: &gpui::Window) {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return;
    };
    unsafe {
        let view = handle.ns_view.as_ptr() as *mut AnyObject;
        let ns_window: *mut AnyObject = msg_send![view, window];
        let _: () = msg_send![ns_window, orderOut: std::ptr::null_mut::<AnyObject>()];
    }
}

/// Shows the hidden window without going through GPUI, for callbacks from AppKit.
#[cfg(target_os = "macos")]
pub fn show() {
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject, Bool};

    let (Some(app_class), Some(window_class)) = (
        AnyClass::get(c"NSApplication"),
        AnyClass::get(c"GPUIWindow"),
    ) else {
        return;
    };
    unsafe {
        let app: *mut AnyObject = msg_send![app_class, sharedApplication];
        let windows: *mut AnyObject = msg_send![app, windows];
        let count: usize = msg_send![windows, count];
        for i in 0..count {
            let window: *mut AnyObject = msg_send![windows, objectAtIndex: i];
            let ours: Bool = msg_send![window, isKindOfClass: window_class];
            if ours.as_bool() {
                let _: () =
                    msg_send![window, makeKeyAndOrderFront: std::ptr::null_mut::<AnyObject>()];
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn hide(_window: &gpui::Window) {}

#[cfg(not(target_os = "macos"))]
pub fn show() {}
