mod app;
mod blur;
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

actions!(overhead, [Quit, ChangeLocation]);

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.set_menus(vec![Menu {
            name: "Overhead".into(),
            items: vec![
                MenuItem::action("Change Home Location…", ChangeLocation),
                MenuItem::separator(),
                MenuItem::action("Quit Overhead", Quit),
            ],
        }]);
        cx.on_window_closed(|cx| cx.quit()).detach();
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
                |window, cx| cx.new(|cx| root::Root::new(window, cx)),
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
        cx.activate(true);
    });
}
