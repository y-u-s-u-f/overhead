//! Workaround for https://github.com/zed-industries/zed/issues/64284: on macOS 27, GPUI's
//! blurred window background uses a material that no longer blurs. Switch the blur view
//! to one that still does. Remove once GPUI ships the fix.

// The `objc` 0.2 macros trip this lint on current Rust.
#![allow(unexpected_cfgs)]

#[cfg(target_os = "macos")]
pub fn fix(window: &gpui::Window) {
    use objc::runtime::{BOOL, NO, Object};
    use objc::{class, msg_send, sel, sel_impl};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    const UNDER_WINDOW_BACKGROUND: isize = 21;

    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return;
    };

    unsafe {
        let view = handle.ns_view.as_ptr() as *mut Object;
        let ns_window: *mut Object = msg_send![view, window];
        let content: *mut Object = msg_send![ns_window, contentView];
        let subviews: *mut Object = msg_send![content, subviews];
        let count: usize = msg_send![subviews, count];
        for i in 0..count {
            let subview: *mut Object = msg_send![subviews, objectAtIndex: i];
            let is_blur: BOOL = msg_send![subview, isKindOfClass: class!(NSVisualEffectView)];
            if is_blur != NO {
                let _: () = msg_send![subview, setMaterial: UNDER_WINDOW_BACKGROUND];
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn fix(_window: &gpui::Window) {}
