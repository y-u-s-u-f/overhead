//! Interface sounds from Google's Material sound resources (CC-BY 4.0), in assets/sounds.

#[derive(Clone, Copy)]
pub enum Sound {
    Next,
    Found,
    Done,
}

impl Sound {
    fn file(self) -> &'static str {
        match self {
            Sound::Next => "next.caf",
            Sound::Found => "found.caf",
            Sound::Done => "done.caf",
        }
    }
}

/// The app bundle's Resources, or the source tree when running with `cargo run`.
#[cfg(target_os = "macos")]
fn path(file: &str) -> Option<std::path::PathBuf> {
    let bundled = std::env::current_exe()
        .ok()?
        .parent()?
        .parent()?
        .join("Resources")
        .join(file);
    let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets/sounds")
        .join(file);
    [bundled, source].into_iter().find(|p| p.exists())
}

#[cfg(target_os = "macos")]
pub fn play(sound: Sound) {
    use objc2::msg_send;
    use objc2::rc::{Allocated, Retained};
    use objc2::runtime::{AnyClass, AnyObject};
    use objc2_foundation::NSString;
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        // NSSound stops when released, so loaded sounds live for the app's lifetime.
        static LOADED: RefCell<HashMap<&'static str, Retained<AnyObject>>> = RefCell::default();
    }

    let file = sound.file();
    LOADED.with_borrow_mut(|loaded| {
        if !loaded.contains_key(file) {
            let (Some(path), Some(class)) = (path(file), AnyClass::get(c"NSSound")) else {
                return;
            };
            let path = NSString::from_str(&path.to_string_lossy());
            let alloc: Allocated<AnyObject> = unsafe { msg_send![class, alloc] };
            let loaded_sound: Option<Retained<AnyObject>> =
                unsafe { msg_send![alloc, initWithContentsOfFile: &*path, byReference: true] };
            let Some(loaded_sound) = loaded_sound else {
                return;
            };
            loaded.insert(file, loaded_sound);
        }
        let sound = &loaded[file];
        unsafe {
            let _: bool = msg_send![&**sound, stop];
            let _: bool = msg_send![&**sound, play];
        }
    });
}

#[cfg(not(target_os = "macos"))]
pub fn play(_sound: Sound) {}
