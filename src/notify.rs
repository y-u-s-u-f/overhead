//! Native notifications when running as Overhead.app. Running unbundled (`cargo run`),
//! the notification center isn't available, so this falls back to `osascript`.

#[cfg(target_os = "macos")]
mod native {
    use block2::{DynBlock, RcBlock};
    use objc2::rc::Retained;
    use objc2::runtime::{Bool, NSObject, ProtocolObject};
    use objc2::{AllocAnyThread, define_class, msg_send};
    use objc2_foundation::{NSBundle, NSError, NSObjectProtocol, NSString};
    use objc2_user_notifications::{
        UNAuthorizationOptions, UNMutableNotificationContent, UNNotification,
        UNNotificationPresentationOptions, UNNotificationRequest, UNNotificationResponse,
        UNNotificationSound, UNUserNotificationCenter, UNUserNotificationCenterDelegate,
    };

    define_class!(
        #[unsafe(super(NSObject))]
        #[name = "OverheadNotificationDelegate"]
        struct Delegate;

        unsafe impl NSObjectProtocol for Delegate {}

        unsafe impl UNUserNotificationCenterDelegate for Delegate {
            // Show banners even while Overhead is the frontmost app.
            #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
            fn will_present(
                &self,
                _center: &UNUserNotificationCenter,
                _notification: &UNNotification,
                completion_handler: &DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
            ) {
                completion_handler.call((UNNotificationPresentationOptions::Banner
                    | UNNotificationPresentationOptions::List
                    | UNNotificationPresentationOptions::Sound,));
            }

            // Clicking a notification brings back the window if it was closed.
            #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
            fn did_receive(
                &self,
                _center: &UNUserNotificationCenter,
                _response: &UNNotificationResponse,
                completion_handler: &DynBlock<dyn Fn()>,
            ) {
                crate::dock::show();
                completion_handler.call(());
            }
        }
    );

    pub fn is_bundled() -> bool {
        NSBundle::mainBundle().bundleIdentifier().is_some()
    }

    pub fn init() {
        let center = UNUserNotificationCenter::currentNotificationCenter();
        let delegate: Retained<Delegate> = unsafe { msg_send![Delegate::alloc(), init] };
        center.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        // The center holds its delegate weakly; keep ours for the life of the app.
        std::mem::forget(delegate);
    }

    pub fn request_permission() {
        UNUserNotificationCenter::currentNotificationCenter()
            .requestAuthorizationWithOptions_completionHandler(
                UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound,
                &RcBlock::new(|_granted: Bool, _error: *mut NSError| {}),
            );
    }

    pub fn send(id: &str, title: &str, body: &str) {
        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(title));
        content.setBody(&NSString::from_str(body));
        let sound = UNNotificationSound::soundNamed(&NSString::from_str("arrival.caf"));
        content.setSound(Some(&sound));
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &NSString::from_str(id),
            &content,
            None,
        );
        UNUserNotificationCenter::currentNotificationCenter()
            .addNotificationRequest_withCompletionHandler(&request, None);
    }
}

pub fn init() {
    #[cfg(target_os = "macos")]
    if native::is_bundled() {
        native::init();
    }
}

/// Shows the system prompt the first time; does nothing once the user has answered.
pub fn request_permission() {
    #[cfg(target_os = "macos")]
    if native::is_bundled() {
        native::request_permission();
    }
}

pub fn send(id: &str, title: &str, body: &str) {
    #[cfg(target_os = "macos")]
    if native::is_bundled() {
        native::send(id, title, body);
        return;
    }

    // Passing the text as arguments avoids having to escape it for AppleScript.
    let result = std::process::Command::new("osascript")
        .args([
            "-e",
            "on run argv",
            "-e",
            "display notification (item 2 of argv) with title (item 1 of argv) sound name \"Glass\"",
            "-e",
            "end run",
            title,
            body,
        ])
        .spawn();
    if let Err(err) = result {
        eprintln!("notification failed: {err}");
    }
}
