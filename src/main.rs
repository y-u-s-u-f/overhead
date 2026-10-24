mod app;
mod blur;
mod dock;
mod feed;
mod location;
mod notify;
mod root;
mod settings;
mod setup;
mod sound;
mod theme;
mod tracker;

use gpui::{
    App, Application, Bounds, KeyBinding, Menu, MenuItem, TitlebarOptions,
    WindowBackgroundAppearance, WindowBounds, WindowOptions, actions, point, prelude::*, px, size,
};

actions!(overhead, [Quit, ChangeLocation, CloseWindow]);

fn main() {
    let app = Application::new();
    app.on_reopen(|cx| {
        if let Some(window) = cx.windows().first() {
            window
                .update(cx, |_, window, _| window.activate_window())
                .ok();
        }
    });
    app.run(|cx: &mut App| {
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("cmd-w", CloseWindow, None),
        ]);
        cx.set_menus(vec![
            Menu {
                name: "Overhead".into(),
                items: vec![
                    MenuItem::action("Change Home Location…", ChangeLocation),
                    MenuItem::separator(),
                    MenuItem::action("Quit Overhead", Quit),
                ],
            },
            Menu {
                name: "Window".into(),
                items: vec![MenuItem::action("Close Window", CloseWindow)],
            },
        ]);
        notify::init();

        let bounds = Bounds::centered(None, size(px(420.), px(720.)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(360.), px(420.))),
                    window_background: WindowBackgroundAppearance::Blurred,
                    titlebar: Some(TitlebarOptions {
                        title: Some("Overhead".into()),
                        appears_transparent: true,
                        traffic_light_position: Some(point(px(16.), px(16.))),
                    }),
                    app_id: Some("overhead".into()),
                    ..Default::default()
                },
                |window, cx| {
                    window.on_window_should_close(cx, |window, _| {
                        dock::hide(window);
                        false
                    });
                    cx.new(|cx| root::Root::new(window, cx))
                },
            )
            .unwrap();
        // Deferred because the window is busy while it dispatches the action.
        cx.on_action(move |_: &ChangeLocation, cx| {
            cx.defer(move |cx| {
                window
                    .update(cx, |root, window, cx| root.change_location(window, cx))
                    .ok();
            })
        });
        cx.on_action(move |_: &CloseWindow, cx| {
            cx.defer(move |cx| {
                window.update(cx, |_, window, _| dock::hide(window)).ok();
            })
        });
        cx.activate(true);
    });
}
